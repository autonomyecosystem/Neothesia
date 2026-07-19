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
    @location(0) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) field_value: f32,
    @location(2) uv: vec2<f32>,
}

fn modal_field(uv: vec2<f32>, component_count: u32) -> f32 {
    var value = 0.0;
    let plate_size_m = max(cyma.viewport.zw, vec2<f32>(0.0001));
    let position_m = uv * plate_size_m;
    for (var index = 0u; index < MAX_MODES; index += 1u) {
        if index >= component_count {
            break;
        }
        let mode = cyma.modes[index];
        let spatial = sin(PI * mode.x * position_m.x / plate_size_m.x)
            * sin(PI * mode.y * position_m.y / plate_size_m.y);
        value += mode.z * spatial * mode.w;
    }
    return value / max(cyma.metrics.x, 0.0001);
}

fn rotated_surface_position(uv: vec2<f32>, value: f32) -> vec3<f32> {
    let activity = clamp(cyma.metrics.x, 0.0, 1.0);
    let world = vec3<f32>(
        (uv.x - 0.5) * 1.82,
        (uv.y - 0.5) * 1.82,
        value * mix(0.12, 0.48, activity)
    );
    let angle = 0.88;
    let cosine = cos(angle);
    let sine = sin(angle);
    return vec3<f32>(
        world.x,
        world.y * cosine - world.z * sine,
        world.y * sine + world.z * cosine
    );
}

fn clip_position(uv: vec2<f32>, value: f32) -> vec4<f32> {
    let rotated = rotated_surface_position(uv, value);
    return vec4<f32>(
        rotated.x / 1.16,
        rotated.y / 1.12 - 0.03,
        clamp(0.55 - rotated.z * 0.24, 0.02, 0.98),
        1.0
    );
}

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    let component_count = min(u32(cyma.metrics.w + 0.5), MAX_MODES);
    let value = modal_field(vertex.uv, component_count);
    let epsilon = 0.004;
    let value_x = modal_field(vertex.uv + vec2<f32>(epsilon, 0.0), component_count);
    let value_y = modal_field(vertex.uv + vec2<f32>(0.0, epsilon), component_count);
    let tangent_x = vec3<f32>(1.0, 0.0, (value_x - value) * 90.0);
    let tangent_y = vec3<f32>(0.0, 1.0, (value_y - value) * 90.0);
    var normal = normalize(cross(tangent_x, tangent_y));
    let angle = 0.88;
    normal = normalize(vec3<f32>(
        normal.x,
        normal.y * cos(angle) - normal.z * sin(angle),
        normal.y * sin(angle) + normal.z * cos(angle)
    ));

    var out: VertexOutput;
    out.position = clip_position(vertex.uv, value);
    out.normal = normal;
    out.field_value = value;
    out.uv = vertex.uv;
    return out;
}

fn hash_grain(uv: vec2<f32>) -> f32 {
    return fract(sin(dot(floor(uv * 1024.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let activity = clamp(cyma.metrics.x, 0.0, 1.0);
    let consonance = clamp(cyma.metrics.y, 0.0, 1.0);
    let tension = clamp(cyma.metrics.z, 0.0, 1.0);
    let light = normalize(vec3<f32>(-0.35, -0.45, 0.82));
    let diffuse = 0.28 + max(dot(normalize(in.normal), light), 0.0) * 0.72;
    let node_width = mix(0.022, 0.055, tension);
    let node = 1.0 - smoothstep(node_width, node_width + 0.03, abs(in.field_value));
    let grain = hash_grain(in.uv);
    let sand = node * mix(0.62, 1.0, grain);
    let edge_distance = min(
        min(in.uv.x, 1.0 - in.uv.x),
        min(in.uv.y, 1.0 - in.uv.y)
    );
    let border = 1.0 - smoothstep(0.0, 0.012, edge_distance);
    let surface_color = mix(vec3<f32>(0.035, 0.038, 0.045), cyma.color.rgb * 0.48, 0.72)
        * (0.65 + diffuse * 0.55);
    let sand_color = mix(
        vec3<f32>(0.78, 0.68, 0.43),
        cyma.color.rgb,
        0.4 + consonance * 0.25
    );
    let color = mix(surface_color, sand_color, clamp(sand + border * 0.38, 0.0, 1.0));
    let alpha = activity * (0.34 + diffuse * 0.36 + node * 0.26 + border * 0.12);
    return vec4<f32>(color, alpha);
}
