const PI: f32 = 3.141592653589793;
const MAX_MODES: u32 = 12u;

struct CymaUniform {
    viewport: vec4<f32>,
    color: vec4<f32>,
    metrics: vec4<f32>,
    modes: array<vec4<f32>, 12>,
}

@group(0) @binding(0)
var<uniform> cyma: CymaUniform;

struct Vertex {
    @location(0) corner: vec2<f32>,
}

struct Particle {
    @location(1) position: vec2<f32>,
    @location(2) opacity: f32,
    @location(3) size: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) opacity: f32,
}

fn modal_field(uv: vec2<f32>, component_count: u32) -> f32 {
    var value = 0.0;
    for (var index = 0u; index < MAX_MODES; index += 1u) {
        if index >= component_count {
            break;
        }
        let mode = cyma.modes[index];
        let spatial = sin(PI * mode.x * uv.x) * sin(PI * mode.y * uv.y);
        value += mode.z * spatial * mode.w;
    }
    return value / max(cyma.metrics.x, 0.0001);
}

fn surface_clip_position(uv: vec2<f32>) -> vec4<f32> {
    let component_count = min(u32(cyma.metrics.w + 0.5), MAX_MODES);
    let value = modal_field(uv, component_count);
    let activity = clamp(cyma.metrics.x, 0.0, 1.0);
    let world = vec3<f32>(
        (uv.x - 0.5) * 2.35,
        (uv.y - 0.5) * 1.55,
        value * mix(0.12, 0.48, activity)
    );
    let angle = 0.88;
    let rotated = vec3<f32>(
        world.x,
        world.y * cos(angle) - world.z * sin(angle),
        world.y * sin(angle) + world.z * cos(angle)
    );
    return vec4<f32>(
        rotated.x / 1.42,
        rotated.y / 1.08 - 0.04,
        clamp(0.548 - rotated.z * 0.24, 0.015, 0.975),
        1.0
    );
}

@vertex
fn vs_2d(vertex: Vertex, particle: Particle) -> VertexOutput {
    let viewport = max(cyma.viewport.xy, vec2<f32>(1.0));
    let center = particle.position * 2.0 - vec2<f32>(1.0);
    let offset = vertex.corner * particle.size * 2.0 / viewport;
    var out: VertexOutput;
    out.position = vec4<f32>(center + offset, 0.0, 1.0);
    out.local = vertex.corner;
    out.opacity = particle.opacity;
    return out;
}

@vertex
fn vs_3d(vertex: Vertex, particle: Particle) -> VertexOutput {
    let viewport = max(cyma.viewport.xy, vec2<f32>(1.0));
    let center = surface_clip_position(particle.position);
    let offset = vertex.corner * particle.size * 2.0 / viewport;
    var out: VertexOutput;
    out.position = center + vec4<f32>(offset, -0.002, 0.0);
    out.local = vertex.corner;
    out.opacity = particle.opacity;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radial = length(in.local) * 2.0;
    let alpha = in.opacity * (1.0 - smoothstep(0.35, 1.0, radial));
    let color = mix(cyma.color.rgb, vec3<f32>(1.0), 0.58);
    return vec4<f32>(color, alpha);
}
