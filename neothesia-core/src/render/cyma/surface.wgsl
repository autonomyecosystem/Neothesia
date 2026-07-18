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

fn rotated_surface_position(uv: vec2<f32>, value: f32) -> vec3<f32> {
    let activity = clamp(cyma.metrics.x, 0.0, 1.0);
    let world = vec3<f32>(
        (uv.x - 0.5) * 2.35,
        (uv.y - 0.5) * 1.55,
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
        rotated.x / 1.42,
        rotated.y / 1.08 - 0.04,
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
    return out;
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
    let surface_color = cyma.color.rgb * diffuse;
    let node_color = mix(cyma.color.rgb, vec3<f32>(1.0), 0.3 + consonance * 0.35);
    let color = mix(surface_color, node_color, node);
    let alpha = activity * (0.28 + diffuse * 0.42 + node * 0.25);
    return vec4<f32>(color, alpha);
}
