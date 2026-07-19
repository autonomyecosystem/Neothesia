mod gpu_timer;
mod particles;
mod surface;
mod target;

use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use cyma_core::{
    CymaQuality, CymaVisualization, MAX_MODAL_COMPONENTS, ModalField, PLATE_HEIGHT_METERS,
    PLATE_WIDTH_METERS,
};
use gpu_timer::GpuTimer;
use particles::ParticleRenderer;
use surface::SurfaceRenderer;
use target::{CompositeRenderer, DEPTH_FORMAT, RenderTarget};
use wgpu_jumpstart::{Color, Gpu, Shape, Uniform, wgpu};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CymaRenderOptions {
    pub visualization: CymaVisualization,
    pub quality: CymaQuality,
    pub particles_enabled: bool,
}

impl Default for CymaRenderOptions {
    fn default() -> Self {
        Self {
            visualization: CymaVisualization::Field2d,
            quality: CymaQuality::Medium,
            particles_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CymaRendererCapabilities {
    pub surface_3d: bool,
    pub particles: bool,
    pub gpu_timing: bool,
}

pub struct CymaRenderer {
    device: wgpu::Device,
    texture_format: wgpu::TextureFormat,
    field_pipeline: wgpu::RenderPipeline,
    fullscreen_quad: Shape,
    uniform: Uniform<CymaUniform>,
    queue: wgpu::Queue,
    composite: CompositeRenderer,
    target: Option<RenderTarget>,
    surface: Option<SurfaceRenderer>,
    particles: Option<ParticleRenderer>,
    gpu_timer: GpuTimer,
    options: CymaRenderOptions,
    active_visualization: CymaVisualization,
    visible: bool,
}

impl CymaRenderer {
    pub fn new(gpu: &Gpu) -> Result<Self, wgpu::Error> {
        let out_of_memory_scope = gpu.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let internal_scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation_scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);

        let mut renderer = Self::create_base(gpu);

        if let Some(error) = pop_scoped_error(validation_scope, internal_scope, out_of_memory_scope)
        {
            return Err(error);
        }

        let depth_features = gpu.adapter.get_texture_format_features(DEPTH_FORMAT);
        let depth_supported = depth_features
            .allowed_usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT);

        renderer.surface = if depth_supported {
            create_optional(&gpu.device, "3D surface", || {
                SurfaceRenderer::new(gpu, &renderer.uniform.bind_group_layout)
            })
        } else {
            log::warn!(
                "Cyma 3D surface disabled because {:?} is not renderable",
                DEPTH_FORMAT
            );
            None
        };

        renderer.particles = create_optional(&gpu.device, "particles", || {
            ParticleRenderer::new(
                &gpu.device,
                gpu.texture_format,
                &renderer.uniform.bind_group_layout,
                depth_supported,
            )
        });

        renderer.gpu_timer = if gpu
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            create_optional(&gpu.device, "GPU timing", || {
                GpuTimer::new(&gpu.device, &gpu.queue)
            })
            .unwrap_or_else(GpuTimer::unsupported)
        } else {
            GpuTimer::unsupported()
        };

        Ok(renderer)
    }

    fn create_base(gpu: &Gpu) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("CymaRenderer::shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let uniform = Uniform::new(
            &gpu.device,
            CymaUniform::default(),
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        );
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CymaRenderer::pipeline_layout"),
                bind_group_layouts: &[Some(&uniform.bind_group_layout)],
                immediate_size: 0,
            });
        let target = wgpu::ColorTargetState {
            format: gpu.texture_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        };
        let field_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("CymaRenderer::field_pipeline"),
                layout: Some(&pipeline_layout),
                fragment: Some(wgpu_jumpstart::default_fragment(&shader, &[Some(target)])),
                ..wgpu_jumpstart::default_render_pipeline(wgpu_jumpstart::default_vertex(
                    &shader,
                    &[Some(Shape::layout())],
                ))
            });

        Self {
            device: gpu.device.clone(),
            texture_format: gpu.texture_format,
            field_pipeline,
            fullscreen_quad: Shape::new_fullscreen_quad(&gpu.device),
            uniform,
            queue: gpu.queue.clone(),
            composite: CompositeRenderer::new(gpu),
            target: None,
            surface: None,
            particles: None,
            gpu_timer: GpuTimer::unsupported(),
            options: CymaRenderOptions::default(),
            active_visualization: CymaVisualization::Field2d,
            visible: false,
        }
    }

    pub fn capabilities(&self) -> CymaRendererCapabilities {
        CymaRendererCapabilities {
            surface_3d: self.surface.is_some(),
            particles: self.particles.is_some(),
            gpu_timing: self.gpu_timer.is_supported(),
        }
    }

    pub fn last_gpu_ms(&self) -> Option<f32> {
        self.gpu_timer.last_gpu_ms()
    }

    pub fn update(
        &mut self,
        field: &ModalField,
        options: CymaRenderOptions,
        delta: Duration,
        width: u32,
        height: u32,
    ) {
        self.options = options;
        self.visible = field.is_active() && width > 0 && height > 0;
        if !self.visible {
            if let Some(particles) = &mut self.particles {
                particles.deactivate();
            }
            return;
        }

        let target_width = scaled_dimension(width, options.quality);
        let target_height = scaled_dimension(height, options.quality);
        let needs_depth =
            options.visualization == CymaVisualization::Surface3d && self.surface.is_some();
        let target_matches = self
            .target
            .as_ref()
            .is_some_and(|target| target.matches(target_width, target_height, needs_depth));
        if !target_matches {
            self.target = Some(RenderTarget::new(
                &self.device,
                self.texture_format,
                &self.composite,
                target_width,
                target_height,
                needs_depth,
            ));
        }

        self.active_visualization = if needs_depth
            && self
                .target
                .as_ref()
                .is_some_and(|target| target.depth_view.is_some())
        {
            CymaVisualization::Surface3d
        } else {
            CymaVisualization::Field2d
        };

        self.uniform.data = CymaUniform::from_modal_field(
            field,
            target_width,
            target_height,
            max_modes(options.quality),
        );
        self.uniform.update(&self.queue);

        if let Some(particles) = &mut self.particles {
            if options.particles_enabled {
                particles.update(field, options.quality, delta, &self.queue);
            } else {
                particles.deactivate();
            }
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        clear_color: wgpu::Color,
        surface_extent: wgpu::Extent3d,
    ) -> bool {
        if !self.visible || surface_extent.width == 0 || surface_extent.height == 0 {
            return false;
        }
        let Some(target) = self.target.as_ref() else {
            return false;
        };

        self.gpu_timer.begin_frame();
        let offscreen_timestamps = self.gpu_timer.pass_writes(0, 1);
        let use_surface_3d = self.active_visualization == CymaVisualization::Surface3d;
        let depth_stencil_attachment = if use_surface_3d {
            target
                .depth_view
                .as_ref()
                .map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
        } else {
            None
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Cyma Off-screen Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment,
                timestamp_writes: offscreen_timestamps,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if use_surface_3d {
                if let Some(surface) = &self.surface {
                    surface.render(&mut pass, &self.uniform.bind_group, self.options.quality);
                }
            } else {
                self.render_field(&mut pass);
            }

            if self.options.particles_enabled
                && let Some(particles) = &self.particles
            {
                particles.render(&mut pass, &self.uniform.bind_group, use_surface_3d);
            }
        }

        let composite_timestamps = self.gpu_timer.pass_writes(2, 3);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Cyma Composite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: composite_timestamps,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.composite
                .render(&mut pass, target, &self.fullscreen_quad);
        }

        self.gpu_timer.resolve(encoder);
        true
    }

    pub fn after_submit(&mut self, device: &wgpu::Device) {
        self.gpu_timer.after_submit(device);
    }

    fn render_field<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        pass.set_pipeline(&self.field_pipeline);
        pass.set_bind_group(0, &self.uniform.bind_group, &[]);
        pass.set_vertex_buffer(0, self.fullscreen_quad.vertex_buffer.slice(..));
        pass.set_index_buffer(
            self.fullscreen_quad.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        pass.draw_indexed(0..self.fullscreen_quad.indices_len, 0, 0..1);
    }
}

fn create_optional<T>(
    device: &wgpu::Device,
    capability: &'static str,
    create: impl FnOnce() -> T,
) -> Option<T> {
    let out_of_memory_scope = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let internal_scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = create();

    if let Some(error) = pop_scoped_error(validation_scope, internal_scope, out_of_memory_scope) {
        log::warn!("Cyma {capability} capability is unavailable: {error}");
        None
    } else {
        Some(value)
    }
}

fn pop_scoped_error(
    validation_scope: wgpu::ErrorScopeGuard,
    internal_scope: wgpu::ErrorScopeGuard,
    out_of_memory_scope: wgpu::ErrorScopeGuard,
) -> Option<wgpu::Error> {
    let validation_error = pollster::block_on(validation_scope.pop());
    let internal_error = pollster::block_on(internal_scope.pop());
    let out_of_memory_error = pollster::block_on(out_of_memory_scope.pop());
    validation_error.or(internal_error).or(out_of_memory_error)
}

fn max_modes(quality: CymaQuality) -> usize {
    match quality {
        CymaQuality::Low => 4,
        CymaQuality::Medium => 8,
        CymaQuality::High => MAX_MODAL_COMPONENTS,
    }
}

fn scaled_dimension(dimension: u32, quality: CymaQuality) -> u32 {
    let (numerator, denominator) = match quality {
        CymaQuality::Low => (2_u64, 5_u64),
        CymaQuality::Medium => (13_u64, 20_u64),
        CymaQuality::High => (1_u64, 1_u64),
    };
    let scaled = (u64::from(dimension) * numerator).div_ceil(denominator);
    scaled.clamp(1, u64::from(u32::MAX)) as u32
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
    fn from_modal_field(field: &ModalField, width: u32, height: u32, mode_limit: usize) -> Self {
        let mut modes = [[0.0; 4]; MAX_MODAL_COMPONENTS];
        let component_count = usize::from(field.component_count()).min(mode_limit);
        for (target, component) in modes
            .iter_mut()
            .zip(field.components().iter().take(component_count))
        {
            *target = [
                f32::from(component.mode_x),
                f32::from(component.mode_y),
                component.amplitude,
                component.phase_gain,
            ];
        }

        let color = Color::new(field.color.red, field.color.green, field.color.blue, 1.0)
            .into_linear_rgba();

        Self {
            viewport: [
                width as f32,
                height as f32,
                PLATE_WIDTH_METERS,
                PLATE_HEIGHT_METERS,
            ],
            color,
            metrics: [
                field.activity,
                field.consonance,
                field.tension,
                component_count as f32,
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
    fn quality_presets_bound_resolution_and_modes() {
        assert_eq!(scaled_dimension(1_000, CymaQuality::Low), 400);
        assert_eq!(scaled_dimension(1_000, CymaQuality::Medium), 650);
        assert_eq!(scaled_dimension(1_000, CymaQuality::High), 1_000);
        assert_eq!(max_modes(CymaQuality::Low), 4);
        assert_eq!(max_modes(CymaQuality::Medium), 8);
        assert_eq!(max_modes(CymaQuality::High), MAX_MODAL_COMPONENTS);
    }

    #[test]
    fn uniform_carries_the_one_meter_square_plate() {
        let uniform = CymaUniform::from_modal_field(&ModalField::default(), 800, 600, 12);
        assert_eq!(uniform.viewport, [800.0, 600.0, 1.0, 1.0]);
    }

    #[test]
    fn all_cyma_wgsl_shaders_parse_and_validate() {
        for source in [
            include_str!("shader.wgsl"),
            include_str!("surface.wgsl"),
            include_str!("particles.wgsl"),
            include_str!("composite.wgsl"),
        ] {
            let module = wgpu::naga::front::wgsl::parse_str(source).expect("Cyma WGSL must parse");
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .expect("Cyma WGSL must validate");
        }
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

        renderer.update(
            &field,
            CymaRenderOptions::default(),
            Duration::from_millis(16),
            WIDTH,
            HEIGHT,
        );
        assert!(renderer.render(&mut gpu.encoder, &view, wgpu::Color::BLACK, extent,));

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
        renderer.after_submit(&gpu.device);

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
