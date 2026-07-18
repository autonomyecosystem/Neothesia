use bytemuck::{Pod, Zeroable};
use cyma_core::{MAX_MODAL_COMPONENTS, ModalField};
use wgpu_jumpstart::{Color, Gpu, Shape, Uniform, wgpu};

pub struct CymaRenderer {
    render_pipeline: wgpu::RenderPipeline,
    fullscreen_quad: Shape,
    uniform: Uniform<CymaUniform>,
    queue: wgpu::Queue,
    visible: bool,
}

impl CymaRenderer {
    pub fn new(gpu: &Gpu) -> Result<Self, wgpu::Error> {
        let out_of_memory_scope = gpu.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let internal_scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation_scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);

        let renderer = Self::create(gpu);

        let validation_error = pollster::block_on(validation_scope.pop());
        let internal_error = pollster::block_on(internal_scope.pop());
        let out_of_memory_error = pollster::block_on(out_of_memory_scope.pop());

        if let Some(error) = validation_error.or(internal_error).or(out_of_memory_error) {
            Err(error)
        } else {
            Ok(renderer)
        }
    }

    fn create(gpu: &Gpu) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("CymaRenderer::shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let uniform = Uniform::new(
            &gpu.device,
            CymaUniform::default(),
            wgpu::ShaderStages::FRAGMENT,
        );

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CymaRenderer::pipeline_layout"),
                bind_group_layouts: &[Some(&uniform.bind_group_layout)],
                immediate_size: 0,
            });

        let target = wgpu_jumpstart::default_color_target_state(gpu.texture_format);
        let render_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("CymaRenderer::pipeline"),
                layout: Some(&pipeline_layout),
                fragment: Some(wgpu_jumpstart::default_fragment(&shader, &[Some(target)])),
                ..wgpu_jumpstart::default_render_pipeline(wgpu_jumpstart::default_vertex(
                    &shader,
                    &[Some(Shape::layout())],
                ))
            });

        Self {
            render_pipeline,
            fullscreen_quad: Shape::new_fullscreen_quad(&gpu.device),
            uniform,
            queue: gpu.queue.clone(),
            visible: false,
        }
    }

    pub fn update(&mut self, field: &ModalField, width: u32, height: u32) {
        self.visible = field.is_active() && width > 0 && height > 0;
        if !self.visible {
            return;
        }

        self.uniform.data = CymaUniform::from_modal_field(field, width, height);
        self.uniform.update(&self.queue);
    }

    pub fn render<'pass>(&'pass self, render_pass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        if !self.visible {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.fullscreen_quad.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.fullscreen_quad.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.draw_indexed(0..self.fullscreen_quad.indices_len, 0, 0..1);
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CymaUniform {
    viewport: [f32; 4],
    color: [f32; 4],
    metrics: [f32; 4],
    modes: [[f32; 4]; MAX_MODAL_COMPONENTS],
}

impl Default for CymaUniform {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl CymaUniform {
    fn from_modal_field(field: &ModalField, width: u32, height: u32) -> Self {
        let mut modes = [[0.0; 4]; MAX_MODAL_COMPONENTS];
        for (target, component) in modes.iter_mut().zip(field.components()) {
            *target = [
                f32::from(component.mode_x),
                f32::from(component.mode_y),
                component.amplitude,
                component.phase,
            ];
        }

        let color = Color::new(field.color.red, field.color.green, field.color.blue, 1.0)
            .into_linear_rgba();

        Self {
            viewport: [width as f32, height as f32, 0.0, 0.0],
            color,
            metrics: [
                field.activity,
                field.consonance,
                field.tension,
                f32::from(field.component_count()),
            ],
            modes,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, offset_of, size_of};
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use cyma_core::{HarmonicState, MidiEvent, MidiState};

    #[test]
    fn uniform_matches_wgsl_alignment_and_padding() {
        assert_eq!(align_of::<CymaUniform>(), 16);
        assert_eq!(size_of::<CymaUniform>(), 240);
        assert_eq!(offset_of!(CymaUniform, viewport), 0);
        assert_eq!(offset_of!(CymaUniform, color), 16);
        assert_eq!(offset_of!(CymaUniform, metrics), 32);
        assert_eq!(offset_of!(CymaUniform, modes), 48);
    }

    #[test]
    fn wgsl_shader_parses_and_validates() {
        let module = wgpu::naga::front::wgsl::parse_str(include_str!("shader.wgsl"))
            .expect("Cyma WGSL must parse");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("Cyma WGSL must validate");
    }

    #[test]
    fn renderer_draws_a_non_empty_field_when_a_gpu_adapter_is_available() {
        let instance = wgpu::Instance::default();
        let mut gpu = match pollster::block_on(Gpu::new(instance, None)) {
            Ok(gpu) => gpu,
            Err(err) => {
                eprintln!("Skipping Cyma GPU rendering test: {err}");
                return;
            }
        };

        let mut midi = MidiState::default();
        for note in [60, 64, 67] {
            midi.apply(MidiEvent::NoteOn {
                channel: 0,
                note,
                velocity: 100,
            })
            .unwrap();
        }
        let field = ModalField::from_harmonic(&HarmonicState::from_midi(&midi));
        let mut renderer =
            CymaRenderer::new(&gpu).expect("Cyma renderer must initialize on the selected adapter");

        const WIDTH: u32 = 64;
        const HEIGHT: u32 = 64;
        const BYTES_PER_ROW: u32 = WIDTH * 4;
        let extent = wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("CymaRenderer::test_texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CymaRenderer::test_readback"),
            size: u64::from(BYTES_PER_ROW * HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        renderer.update(&field, WIDTH, HEIGHT);
        {
            let render_pass = gpu.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CymaRenderer::test_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut render_pass = wgpu_jumpstart::RenderPass::new(render_pass, extent);
            renderer.render(&mut render_pass);
        }

        gpu.encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(BYTES_PER_ROW),
                    rows_per_image: Some(HEIGHT),
                },
            },
            extent,
        );
        gpu.submit();

        let slice = readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("GPU readback polling must succeed");
        receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("GPU readback callback must run")
            .expect("GPU readback mapping must succeed");

        let pixels = slice
            .get_mapped_range()
            .expect("GPU readback range must be available");
        let has_visible_pixel = pixels
            .chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0);
        drop(pixels);
        readback.unmap();

        assert!(has_visible_pixel, "Cyma renderer must draw visible pixels");
    }
}
