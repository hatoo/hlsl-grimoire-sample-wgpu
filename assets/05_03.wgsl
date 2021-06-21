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
    // Rim light
    direction: vec3<f32>;
    color: vec3<f32>;
    // Ambient light
    ambient: vec3<f32>;
};

[[group(3), binding(0)]]
var<uniform> light: Light;

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
    let eye_direction = normalize(in.world_position.xyz - light.eye_position);
    let rim1 = 1.0 - max(0.0, dot(light.direction, in.normal));
    let rim2 = 1.0 - max(0.0, dot(-eye_direction, in.normal));

    return vec4<f32>(pow(rim1 * rim2, 1.3) * light.color, 1.0);
}
