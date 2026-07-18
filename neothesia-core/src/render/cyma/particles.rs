use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use cyma_core::{CymaQuality, ModalField};
use wgpu_jumpstart::{Shape, wgpu};

use super::target::DEPTH_FORMAT;

const MAX_PARTICLES: usize = 768;

pub(super) struct ParticleRenderer {
    pipeline_2d: wgpu::RenderPipeline,
    pipeline_3d: Option<wgpu::RenderPipeline>,
    quad: Shape,
    states: Vec<ParticleState>,
    instances: Vec<ParticleInstance>,
    instance_buffer: wgpu::Buffer,
    active_count: u32,
    needs_reset: bool,
}

impl ParticleRenderer {
    pub(super) fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        uniform_layout: &wgpu::BindGroupLayout,
        depth_supported: bool,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("CymaParticles::shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("particles.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("CymaParticles::pipeline_layout"),
            bind_group_layouts: &[Some(uniform_layout)],
            immediate_size: 0,
        });
        let pipeline_2d = create_pipeline(
            device,
            &pipeline_layout,
            &shader,
            texture_format,
            "vs_2d",
            None,
            "CymaParticles::pipeline_2d",
        );
        let pipeline_3d = depth_supported.then(|| {
            create_pipeline(
                device,
                &pipeline_layout,
                &shader,
                texture_format,
                "vs_3d",
                Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                "CymaParticles::pipeline_3d",
            )
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CymaParticles::instances"),
            size: (std::mem::size_of::<ParticleInstance>() * MAX_PARTICLES) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut renderer = Self {
            pipeline_2d,
            pipeline_3d,
            quad: Shape::new_centered_quad(device),
            states: vec![ParticleState::default(); MAX_PARTICLES],
            instances: vec![ParticleInstance::default(); MAX_PARTICLES],
            instance_buffer,
            active_count: 0,
            needs_reset: true,
        };
        renderer.reset();
        renderer.needs_reset = true;
        renderer
    }

    pub(super) fn update(
        &mut self,
        field: &ModalField,
        quality: CymaQuality,
        delta: Duration,
        queue: &wgpu::Queue,
    ) {
        if self.needs_reset {
            self.reset();
            self.needs_reset = false;
        }

        let count =
            simulate_particles(field, quality, delta, &mut self.states, &mut self.instances);

        self.active_count = count as u32;
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances[..count]),
        );
    }

    pub(super) fn deactivate(&mut self) {
        self.active_count = 0;
        self.needs_reset = true;
    }

    pub(super) fn render<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        uniform_bind_group: &'pass wgpu::BindGroup,
        surface_3d: bool,
    ) {
        if self.active_count == 0 {
            return;
        }

        let pipeline = if surface_3d {
            let Some(pipeline) = self.pipeline_3d.as_ref() else {
                return;
            };
            pipeline
        } else {
            &self.pipeline_2d
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.quad.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.quad.indices_len, 0, 0..self.active_count);
    }

    fn reset(&mut self) {
        initialize_particle_states(&mut self.states);
    }
}

fn simulate_particles(
    field: &ModalField,
    quality: CymaQuality,
    delta: Duration,
    states: &mut [ParticleState],
    instances: &mut [ParticleInstance],
) -> usize {
    let count = particle_count(quality)
        .min(states.len())
        .min(instances.len());
    let delta_seconds = delta.as_secs_f32().clamp(0.0, 0.05);
    let attraction = 0.35 + field.tension * 0.9;
    let damping = (1.0 - delta_seconds * 3.2).clamp(0.0, 1.0);

    for index in 0..count {
        let state = &mut states[index];
        let sample = field.sample_with_node_gradient(state.position[0], state.position[1]);
        state.velocity[0] =
            (state.velocity[0] - sample.node_gradient[0] * attraction * delta_seconds) * damping;
        state.velocity[1] =
            (state.velocity[1] - sample.node_gradient[1] * attraction * delta_seconds) * damping;

        let speed_squared =
            state.velocity[0] * state.velocity[0] + state.velocity[1] * state.velocity[1];
        if speed_squared > 0.032 * 0.032 {
            let scale = 0.032 / speed_squared.sqrt();
            state.velocity[0] *= scale;
            state.velocity[1] *= scale;
        }

        state.position[0] += state.velocity[0] * delta_seconds;
        state.position[1] += state.velocity[1] * delta_seconds;
        reflect_axis(&mut state.position[0], &mut state.velocity[0]);
        reflect_axis(&mut state.position[1], &mut state.velocity[1]);

        instances[index] = ParticleInstance {
            position: state.position,
            opacity: field.activity * (1.0 / (1.0 + sample.value.abs() * 22.0)),
            size: particle_size(quality),
        };
    }

    count
}

fn initialize_particle_states(states: &mut [ParticleState]) {
    for (index, state) in states.iter_mut().enumerate() {
        let seed = index as f32 + 1.0;
        let angle = fract(seed * 0.618_034) * std::f32::consts::TAU;
        *state = ParticleState {
            position: [fract(seed * 0.754_877_7), fract(seed * 0.569_840_3)],
            velocity: [angle.cos() * 0.006, angle.sin() * 0.006],
        };
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    texture_format: wgpu::TextureFormat,
    vertex_entry: &str,
    depth_stencil: Option<wgpu::DepthStencilState>,
    label: &str,
) -> wgpu::RenderPipeline {
    let target = wgpu_jumpstart::default_color_target_state(texture_format);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex_entry),
            buffers: &[Some(Shape::layout()), Some(ParticleInstance::layout())],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(target)],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn particle_count(quality: CymaQuality) -> usize {
    match quality {
        CymaQuality::Low => 128,
        CymaQuality::Medium => 384,
        CymaQuality::High => MAX_PARTICLES,
    }
}

fn particle_size(quality: CymaQuality) -> f32 {
    match quality {
        CymaQuality::Low => 2.6,
        CymaQuality::Medium => 2.2,
        CymaQuality::High => 1.8,
    }
}

fn reflect_axis(position: &mut f32, velocity: &mut f32) {
    if *position < 0.0 {
        *position = 0.0;
        *velocity = velocity.abs();
    } else if *position > 1.0 {
        *position = 1.0;
        *velocity = -velocity.abs();
    }
}

fn fract(value: f32) -> f32 {
    value - value.floor()
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct ParticleState {
    position: [f32; 2],
    velocity: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
struct ParticleInstance {
    position: [f32; 2],
    opacity: f32,
    size: f32,
}

impl ParticleInstance {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32,
            },
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBUTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use cyma_core::{HarmonicState, MidiEvent, MidiState, ModalField};

    use super::*;

    fn c_major_field() -> ModalField {
        let mut midi = MidiState::default();
        for note in [60, 64, 67] {
            midi.apply(MidiEvent::NoteOn {
                channel: 0,
                note,
                velocity: 100,
            })
            .unwrap();
        }
        ModalField::from_harmonic(&HarmonicState::from_midi(&midi))
    }

    #[test]
    fn particle_reset_is_deterministic() {
        let mut first = vec![ParticleState::default(); MAX_PARTICLES];
        let mut second = vec![ParticleState::default(); MAX_PARTICLES];
        initialize_particle_states(&mut first);
        initialize_particle_states(&mut second);
        assert_eq!(first, second);
    }

    #[test]
    fn field_gradient_used_by_particles_is_finite() {
        let field = c_major_field();
        for index in 0..32 {
            let seed = index as f32 + 1.0;
            let position = [fract(seed * 0.754_877_7), fract(seed * 0.569_840_3)];
            let gradient = field.node_gradient(position[0], position[1]);
            assert!(gradient.iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn optimized_particle_step_is_deterministic_and_finite() {
        let field = c_major_field();
        let mut first_states = vec![ParticleState::default(); MAX_PARTICLES];
        let mut second_states = vec![ParticleState::default(); MAX_PARTICLES];
        let mut first_instances = vec![ParticleInstance::default(); MAX_PARTICLES];
        let mut second_instances = vec![ParticleInstance::default(); MAX_PARTICLES];
        initialize_particle_states(&mut first_states);
        initialize_particle_states(&mut second_states);

        let first_count = simulate_particles(
            &field,
            CymaQuality::High,
            Duration::from_millis(16),
            &mut first_states,
            &mut first_instances,
        );
        let second_count = simulate_particles(
            &field,
            CymaQuality::High,
            Duration::from_millis(16),
            &mut second_states,
            &mut second_instances,
        );

        assert_eq!(first_count, MAX_PARTICLES);
        assert_eq!(second_count, MAX_PARTICLES);
        assert_eq!(first_states, second_states);
        assert_eq!(first_instances, second_instances);
        assert!(first_instances.iter().all(|instance| {
            instance.position.iter().all(|value| value.is_finite())
                && instance.opacity.is_finite()
                && instance.size.is_finite()
        }));
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "manual Windows release performance probe"]
    fn high_quality_particle_update_stays_within_cpu_budget() {
        use std::{hint::black_box, time::Instant};

        assert!(
            is_release_build(),
            "run this probe with cargo test --release"
        );

        const WARMUP_FRAMES: usize = 60;
        const MEASURED_FRAMES: usize = 600;
        const HIGH_QUALITY_BUDGET_MS: f64 = 4.0;

        let field = c_major_field();
        let mut states = vec![ParticleState::default(); MAX_PARTICLES];
        let mut instances = vec![ParticleInstance::default(); MAX_PARTICLES];
        initialize_particle_states(&mut states);

        for _ in 0..WARMUP_FRAMES {
            simulate_particles(
                black_box(&field),
                CymaQuality::High,
                Duration::from_secs_f64(1.0 / 60.0),
                &mut states,
                &mut instances,
            );
        }

        let started = Instant::now();
        for _ in 0..MEASURED_FRAMES {
            simulate_particles(
                black_box(&field),
                CymaQuality::High,
                Duration::from_secs_f64(1.0 / 60.0),
                &mut states,
                &mut instances,
            );
            black_box(instances[0]);
        }
        let average_ms = started.elapsed().as_secs_f64() * 1_000.0 / MEASURED_FRAMES as f64;
        eprintln!("Cyma high-quality particle CPU average: {average_ms:.3} ms/frame");

        assert!(
            average_ms < HIGH_QUALITY_BUDGET_MS,
            "particle CPU average {average_ms:.3} ms exceeds {HIGH_QUALITY_BUDGET_MS:.1} ms"
        );
    }

    #[cfg(target_os = "windows")]
    fn is_release_build() -> bool {
        !cfg!(debug_assertions)
    }
}
