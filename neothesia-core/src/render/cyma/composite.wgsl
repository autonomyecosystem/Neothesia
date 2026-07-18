@group(0) @binding(0)
var cyma_sampler: sampler;

@group(0) @binding(1)
var cyma_texture: texture_2d<f32>;

struct Vertex {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(vertex.position, 0.0, 1.0);
    out.uv = vec2<f32>(
        vertex.position.x * 0.5 + 0.5,
        0.5 - vertex.position.y * 0.5
    );
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(cyma_texture, cyma_sampler, in.uv);
}
