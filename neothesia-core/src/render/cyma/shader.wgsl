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
    out.uv = vertex.position * 0.5 + vec2<f32>(0.5);
    return out;
}

// Physically motivated basis: standing-wave eigenfunctions of an ideal
// rectangular membrane. Pitch-to-mode mapping, phases and color are artistic.
fn modal_field(uv: vec2<f32>, component_count: u32) -> f32 {
    var value = 0.0;

    for (var index = 0u; index < MAX_MODES; index += 1u) {
        if index >= component_count {
            break;
        }

        let mode = cyma.modes[index];
        let mode_x = mode.x;
        let mode_y = mode.y;
        let amplitude = mode.z;
        let phase = mode.w;
        let spatial = sin(PI * mode_x * uv.x) * sin(PI * mode_y * uv.y);
        let modal_frequency = sqrt(mode_x * mode_x + mode_y * mode_y);
        let deterministic_phase_gain = 0.75 + 0.25 * cos(modal_frequency * 0.25 + phase);
        value += amplitude * spatial * deterministic_phase_gain;
    }

    return value;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let width = max(cyma.viewport.x, 1.0);
    let height = max(cyma.viewport.y, 1.0);
    let activity = clamp(cyma.metrics.x, 0.0, 1.0);
    let consonance = clamp(cyma.metrics.y, 0.0, 1.0);
    let tension = clamp(cyma.metrics.z, 0.0, 1.0);
    let component_count = min(u32(cyma.metrics.w + 0.5), MAX_MODES);

    let value = modal_field(in.uv, component_count) / max(activity, 0.0001);
    let distance_to_node = abs(value);
    let node_width = mix(0.025, 0.06, tension);
    let node = 1.0 - smoothstep(node_width, node_width + 0.035, distance_to_node);
    let contours = 0.5 + 0.5 * cos(value * (18.0 + tension * 10.0));

    let aspect = width / height;
    let centered = (in.uv * 2.0 - vec2<f32>(1.0)) * vec2<f32>(aspect, 1.0);
    let vignette = 1.0 - smoothstep(0.45, 1.35, length(centered));

    let base_color = cyma.color.rgb * (0.22 + contours * 0.08);
    let node_color = mix(cyma.color.rgb, vec3<f32>(1.0), 0.25 + consonance * 0.35);
    let color = mix(base_color, node_color, node);
    let alpha = activity * vignette * (0.025 + contours * 0.025 + node * 0.32);

    return vec4<f32>(color, alpha);
}
