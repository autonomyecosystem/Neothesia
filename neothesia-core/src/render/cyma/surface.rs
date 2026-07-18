use bytemuck::{Pod, Zeroable};
use cyma_core::CymaQuality;
use wgpu::util::DeviceExt;
use wgpu_jumpstart::{Gpu, wgpu};

use super::target::DEPTH_FORMAT;

pub(super) struct SurfaceRenderer {
    pipeline: wgpu::RenderPipeline,
    meshes: [SurfaceMesh; 3],
}

impl SurfaceRenderer {
    pub(super) fn new(gpu: &Gpu, uniform_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("CymaSurface::shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("surface.wgsl").into()),
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CymaSurface::pipeline_layout"),
                bind_group_layouts: &[Some(uniform_layout)],
                immediate_size: 0,
            });
        let target = wgpu::ColorTargetState {
            format: gpu.texture_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        };
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("CymaSurface::pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(SurfaceVertex::layout())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(target)],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        Self {
            pipeline,
            meshes: [
                SurfaceMesh::new(&gpu.device, 32, 24),
                SurfaceMesh::new(&gpu.device, 64, 40),
                SurfaceMesh::new(&gpu.device, 96, 64),
            ],
        }
    }

    pub(super) fn render<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        uniform_bind_group: &'pass wgpu::BindGroup,
        quality: CymaQuality,
    ) {
        let mesh = &self.meshes[quality_index(quality)];
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

fn quality_index(quality: CymaQuality) -> usize {
    match quality {
        CymaQuality::Low => 0,
        CymaQuality::Medium => 1,
        CymaQuality::High => 2,
    }
}

struct SurfaceMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl SurfaceMesh {
    fn new(device: &wgpu::Device, columns: u16, rows: u16) -> Self {
        let mut vertices = Vec::with_capacity(usize::from(columns) * usize::from(rows));
        for row in 0..rows {
            for column in 0..columns {
                vertices.push(SurfaceVertex {
                    uv: [
                        f32::from(column) / f32::from(columns - 1),
                        f32::from(row) / f32::from(rows - 1),
                    ],
                });
            }
        }

        let mut indices = Vec::with_capacity(usize::from(columns - 1) * usize::from(rows - 1) * 6);
        for row in 0..(rows - 1) {
            for column in 0..(columns - 1) {
                let top_left = row * columns + column;
                let top_right = top_left + 1;
                let bottom_left = top_left + columns;
                let bottom_right = bottom_left + 1;
                indices.extend_from_slice(&[
                    top_left,
                    bottom_left,
                    top_right,
                    top_right,
                    bottom_left,
                    bottom_right,
                ]);
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("CymaSurface::vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("CymaSurface::indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SurfaceVertex {
    uv: [f32; 2],
}

impl SurfaceVertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn high_quality_mesh_stays_within_u16_indices() {
        let vertices = 96_u32 * 64;
        assert!(vertices <= u32::from(u16::MAX));
    }
}
