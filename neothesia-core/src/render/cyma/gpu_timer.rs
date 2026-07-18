use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use wgpu_jumpstart::wgpu;

const QUERY_COUNT: u32 = 4;
const QUERY_BUFFER_SIZE: u64 = QUERY_COUNT as u64 * std::mem::size_of::<u64>() as u64;
const SLOT_IDLE: u8 = 0;
const SLOT_RESERVED: u8 = 1;
const SLOT_MAPPING: u8 = 2;
const SLOT_READY: u8 = 3;
const SLOT_FAILED: u8 = 4;

pub(super) struct GpuTimer {
    resources: Option<GpuTimerResources>,
    last_gpu_ms: Option<f32>,
    failed: bool,
}

impl GpuTimer {
    pub(super) fn unsupported() -> Self {
        Self {
            resources: None,
            last_gpu_ms: None,
            failed: false,
        }
    }

    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Self::unsupported();
        }

        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("CymaGpuTimer::queries"),
            ty: wgpu::QueryType::Timestamp,
            count: QUERY_COUNT,
        });
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CymaGpuTimer::resolve"),
            size: QUERY_BUFFER_SIZE,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = [
            ReadbackSlot::new(device, "CymaGpuTimer::readback_0"),
            ReadbackSlot::new(device, "CymaGpuTimer::readback_1"),
        ];
        let timestamp_period_ns = queue.get_timestamp_period();

        if !timestamp_period_ns.is_finite() || timestamp_period_ns <= 0.0 {
            log::warn!(
                "Cyma GPU timing disabled because the timestamp period is invalid: {timestamp_period_ns}"
            );
            return Self::unsupported();
        }

        Self {
            resources: Some(GpuTimerResources {
                query_set,
                resolve_buffer,
                slots,
                next_slot: 0,
                frame_slot: None,
                timestamp_period_ns,
            }),
            last_gpu_ms: None,
            failed: false,
        }
    }

    pub(super) fn is_supported(&self) -> bool {
        self.resources.is_some() && !self.failed
    }

    pub(super) fn last_gpu_ms(&self) -> Option<f32> {
        self.last_gpu_ms.filter(|value| value.is_finite())
    }

    pub(super) fn begin_frame(&mut self) {
        let Some(resources) = self.resources.as_mut().filter(|_| !self.failed) else {
            return;
        };

        resources.frame_slot = None;
        for offset in 0..resources.slots.len() {
            let index = (resources.next_slot + offset) % resources.slots.len();
            let state = &resources.slots[index].state;
            if state
                .compare_exchange(
                    SLOT_IDLE,
                    SLOT_RESERVED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                resources.frame_slot = Some(index);
                resources.next_slot = (index + 1) % resources.slots.len();
                break;
            }
        }
    }

    pub(super) fn pass_writes(
        &self,
        beginning_index: u32,
        end_index: u32,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let resources = self.resources.as_ref().filter(|_| !self.failed)?;
        resources.frame_slot?;

        Some(wgpu::RenderPassTimestampWrites {
            query_set: &resources.query_set,
            beginning_of_pass_write_index: Some(beginning_index),
            end_of_pass_write_index: Some(end_index),
        })
    }

    pub(super) fn resolve(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(resources) = self.resources.as_mut().filter(|_| !self.failed) else {
            return;
        };
        let Some(slot_index) = resources.frame_slot else {
            return;
        };

        encoder.resolve_query_set(
            &resources.query_set,
            0..QUERY_COUNT,
            &resources.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &resources.resolve_buffer,
            0,
            &resources.slots[slot_index].buffer,
            0,
            QUERY_BUFFER_SIZE,
        );
    }

    pub(super) fn after_submit(&mut self, device: &wgpu::Device) {
        let Some(resources) = self.resources.as_mut().filter(|_| !self.failed) else {
            return;
        };

        if let Some(slot_index) = resources.frame_slot.take() {
            let slot = &resources.slots[slot_index];
            slot.state.store(SLOT_MAPPING, Ordering::Release);
            let completion_state = Arc::clone(&slot.state);
            slot.buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    completion_state.store(
                        if result.is_ok() {
                            SLOT_READY
                        } else {
                            SLOT_FAILED
                        },
                        Ordering::Release,
                    );
                });
        }

        if let Err(err) = device.poll(wgpu::PollType::Poll) {
            log::warn!("Cyma GPU timing disabled after device polling failed: {err}");
            self.failed = true;
            self.last_gpu_ms = None;
            return;
        }

        for slot in &resources.slots {
            match slot.state.load(Ordering::Acquire) {
                SLOT_READY => {
                    let mapped = match slot.buffer.slice(..).get_mapped_range() {
                        Ok(mapped) => mapped,
                        Err(err) => {
                            log::warn!(
                                "Cyma GPU timing disabled after timestamp readback failed: {err}"
                            );
                            self.failed = true;
                            self.last_gpu_ms = None;
                            return;
                        }
                    };
                    let timestamps = decode_timestamps(&mapped);
                    drop(mapped);
                    slot.buffer.unmap();
                    slot.state.store(SLOT_IDLE, Ordering::Release);

                    if let Some(timestamps) = timestamps {
                        let offscreen_ticks = timestamps[1].wrapping_sub(timestamps[0]);
                        let composite_ticks = timestamps[3].wrapping_sub(timestamps[2]);
                        if let Some(total_ticks) = offscreen_ticks.checked_add(composite_ticks) {
                            let gpu_ms = total_ticks as f64
                                * f64::from(resources.timestamp_period_ns)
                                / 1_000_000.0;
                            if gpu_ms.is_finite() && (0.0..=10_000.0).contains(&gpu_ms) {
                                self.last_gpu_ms = Some(gpu_ms as f32);
                            }
                        }
                    }
                }
                SLOT_FAILED => {
                    log::warn!("Cyma GPU timing disabled because timestamp mapping failed");
                    self.failed = true;
                    self.last_gpu_ms = None;
                    return;
                }
                _ => {}
            }
        }
    }
}

struct GpuTimerResources {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    slots: [ReadbackSlot; 2],
    next_slot: usize,
    frame_slot: Option<usize>,
    timestamp_period_ns: f32,
}

struct ReadbackSlot {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
}

impl ReadbackSlot {
    fn new(device: &wgpu::Device, label: &'static str) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: QUERY_BUFFER_SIZE,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            state: Arc::new(AtomicU8::new(SLOT_IDLE)),
        }
    }
}

fn decode_timestamps(bytes: &[u8]) -> Option<[u64; QUERY_COUNT as usize]> {
    if bytes.len() < QUERY_BUFFER_SIZE as usize {
        return None;
    }

    let mut timestamps = [0_u64; QUERY_COUNT as usize];
    for (index, timestamp) in timestamps.iter_mut().enumerate() {
        let start = index * std::mem::size_of::<u64>();
        let end = start + std::mem::size_of::<u64>();
        let mut encoded = [0_u8; std::mem::size_of::<u64>()];
        encoded.copy_from_slice(&bytes[start..end]);
        *timestamp = u64::from_ne_bytes(encoded);
    }
    Some(timestamps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_decode_is_fixed_size_and_deterministic() {
        let expected = [1_u64, 2, 5, 9];
        let mut bytes = [0_u8; QUERY_BUFFER_SIZE as usize];
        for (index, value) in expected.iter().enumerate() {
            let start = index * std::mem::size_of::<u64>();
            let end = start + std::mem::size_of::<u64>();
            bytes[start..end].copy_from_slice(&value.to_ne_bytes());
        }

        assert_eq!(decode_timestamps(&bytes), Some(expected));
        assert_eq!(decode_timestamps(&bytes[..bytes.len() - 1]), None);
    }
}
