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

// Physically motivated basis: standing-wave eigenfunctions of an ideal,
// simply supported square thin plate. Pitch-to-mode mapping, gains and color
// are artistic; material, thickness and excitation are not calibrated.
fn modal_field(uv: vec2<f32>, component_count: u32) -> f32 {
    var value = 0.0;
    let plate_size_m = max(cyma.viewport.zw, vec2<f32>(0.0001));
    let position_m = uv * plate_size_m;

    for (var index = 0u; index < MAX_MODES; index += 1u) {
        if index >= component_count {
            break;
        }

        let mode = cyma.modes[index];
        let mode_x = mode.x;
        let mode_y = mode.y;
        let amplitude = mode.z;
        let phase_gain = mode.w;
        let spatial = sin(PI * mode_x * position_m.x / plate_size_m.x)
            * sin(PI * mode_y * position_m.y / plate_size_m.y);
        value += amplitude * spatial * phase_gain;
    }

    return value;
}

fn hash_pixel(pixel: vec2<f32>) -> f32 {
    return fract(sin(dot(pixel, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let width = max(cyma.viewport.x, 1.0);
    let height = max(cyma.viewport.y, 1.0);
    let activity = clamp(cyma.metrics.x, 0.0, 1.0);
    let consonance = clamp(cyma.metrics.y, 0.0, 1.0);
    let tension = clamp(cyma.metrics.z, 0.0, 1.0);
    let component_count = min(u32(cyma.metrics.w + 0.5), MAX_MODES);

    let viewport = vec2<f32>(width, height);
    let plate_side = max(min(width, height), 1.0);
    let plate_origin = (viewport - vec2<f32>(plate_side)) * 0.5;
    let plate_pixel = in.uv * viewport - plate_origin;
    let plate_uv = plate_pixel / plate_side;
    if any(plate_uv < vec2<f32>(0.0)) || any(plate_uv > vec2<f32>(1.0)) {
        return vec4<f32>(0.0);
    }

    let value = modal_field(plate_uv, component_count) / max(activity, 0.0001);
    let distance_to_node = abs(value);
    let node_width = mix(0.018, 0.045, tension);
    let node = 1.0 - smoothstep(node_width, node_width + 0.028, distance_to_node);
    let contours = 0.5 + 0.5 * cos(value * (18.0 + tension * 10.0));

    let grain = hash_pixel(floor(plate_pixel));
    let sand = node * mix(0.58, 1.0, grain);
    let edge_distance = min(
        min(plate_uv.x, 1.0 - plate_uv.x),
        min(plate_uv.y, 1.0 - plate_uv.y)
    );
    let border = 1.0 - smoothstep(0.0, 0.008, edge_distance);

    let plate_color = mix(vec3<f32>(0.025, 0.028, 0.035), cyma.color.rgb * 0.24, 0.62);
    let sand_color = mix(
        vec3<f32>(0.78, 0.68, 0.43),
        cyma.color.rgb,
        0.42 + consonance * 0.24
    );
    let base_color = plate_color * (0.72 + contours * 0.18);
    let color = mix(base_color, sand_color, clamp(sand + border * 0.42, 0.0, 1.0));
    let alpha = activity * (0.18 + contours * 0.06 + node * 0.68 + border * 0.22);

    return vec4<f32>(color, alpha);
}
