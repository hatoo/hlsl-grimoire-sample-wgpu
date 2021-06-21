struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] color: vec4<f32>;
    [[location(3)]] tex_coords: vec2<f32>;
};

[[block]]
struct Uniforms {
    mat: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> global: Uniforms;

[[group(1), binding(0)]]
var<uniform> local: Uniforms;

[[group(2), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(2), binding(1)]]
var s_diffuse: sampler;

[[block]]
struct Light {
    eye_position: vec3<f32>;
    // Directional light
    direction: vec3<f32>;
    color: vec3<f32>;
    // Ambient light
    ambient: vec3<f32>;
    // Point light
    point: vec3<f32>;
    point_color: vec3<f32>;
    point_range: f32;
};

[[group(3), binding(0)]]
var<uniform> light: Light;

fn lambert_diffuse(light_direction: vec3<f32>, normal: vec3<f32>) -> f32 {
    return max(0.0, -1.0 * dot(normal, light_direction));
}

fn phong_speclar(light_direction: vec3<f32>, normal: vec3<f32>, world_position: vec3<f32>, eye_position: vec3<f32>) -> f32 {
    let ref = reflect(light_direction, normal);
    let to_eye = normalize(eye_position - world_position);
    return pow(max(0.0, dot(ref, to_eye)), 5.0);
}

[[stage(vertex)]]
fn vs_main([[location(0)]] position: vec4<f32>, [[location(1)]] normal: vec3<f32>, [[location(2)]] color: vec4<f32>, [[location(3)]] tex_coords: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = global.mat * local.mat * position;
    out.world_position = global.mat * local.mat * position;
    out.normal = normalize((global.mat * local.mat * vec4<f32>(normal, 0.0)).xyz);
    out.color = color;
    out.tex_coords = tex_coords;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let directional_diffuse = lambert_diffuse(light.direction, in.normal);
    let directional_specular = phong_speclar(light.direction, in.normal, in.world_position.xyz, light.eye_position);

    let point_direction = normalize(in.world_position.xyz - light.point);
    let d = distance(in.world_position.xyz, light.point);
    let affect = pow(max(0.0, 1.0 - 1.0 / light.point_range * d), 3.0);
    let point_diffuse = affect * lambert_diffuse(point_direction, in.normal);
    let point_specular = affect * phong_speclar(point_direction, in.normal, in.world_position.xyz, light.eye_position);

    return vec4<f32>((directional_specular + directional_diffuse) * light.color + (point_diffuse + point_specular) * light.point_color + light.ambient, 1.0);
}
