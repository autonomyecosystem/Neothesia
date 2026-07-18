use wgpu_jumpstart::{Gpu, Shape, wgpu};

pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

pub(super) struct CompositeRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl CompositeRenderer {
    pub(super) fn new(gpu: &Gpu) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("CymaComposite::shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("composite.wgsl").into()),
            });
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CymaComposite::bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CymaComposite::pipeline_layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let target = wgpu_jumpstart::default_color_target_state(gpu.texture_format);
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("CymaComposite::pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Shape::layout())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(target)],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("CymaComposite::sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    pub(super) fn create_bind_group(
        &self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CymaComposite::bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(view),
                },
            ],
        })
    }

    pub(super) fn render<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        target: &'pass RenderTarget,
        quad: &'pass Shape,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &target.composite_bind_group, &[]);
        pass.set_vertex_buffer(0, quad.vertex_buffer.slice(..));
        pass.set_index_buffer(quad.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..quad.indices_len, 0, 0..1);
    }
}

pub(super) struct RenderTarget {
    _color_texture: wgpu::Texture,
    _depth_texture: Option<wgpu::Texture>,
    pub(super) color_view: wgpu::TextureView,
    pub(super) depth_view: Option<wgpu::TextureView>,
    composite_bind_group: wgpu::BindGroup,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl RenderTarget {
    pub(super) fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        composite: &CompositeRenderer,
        width: u32,
        height: u32,
        create_depth: bool,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("CymaRenderTarget::color"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_texture = create_depth.then(|| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("CymaRenderTarget::depth"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        });
        let depth_view = depth_texture
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let composite_bind_group = composite.create_bind_group(device, &color_view);

        Self {
            _color_texture: color_texture,
            _depth_texture: depth_texture,
            color_view,
            depth_view,
            composite_bind_group,
            width: size.width,
            height: size.height,
        }
    }

    pub(super) fn matches(&self, width: u32, height: u32, needs_depth: bool) -> bool {
        self.width == width.max(1)
            && self.height == height.max(1)
            && self.depth_view.is_some() == needs_depth
    }
}
