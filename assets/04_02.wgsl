struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] normal: vec3<f32>;
    [[location(1)]] color: vec4<f32>;
    [[location(2)]] tex_coords: vec2<f32>;
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
struct V3 {
    v: vec3<f32>;
};

[[group(3), binding(0)]]
var<uniform> directional_light_direction: V3;
[[group(3), binding(1)]]
var<uniform> directional_light_color: V3;

[[stage(vertex)]]
fn vs_main([[location(0)]] position: vec4<f32>, [[location(1)]] normal: vec3<f32>, [[location(2)]] color: vec4<f32>, [[location(3)]] tex_coords: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = global.mat * local.mat * position;
    out.normal = normalize((global.mat * local.mat * vec4<f32>(normal, 0.0)).xyz);
    out.color = color;
    out.tex_coords = tex_coords;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let t: f32 = max(0.0, -1.0 * dot(in.normal, directional_light_direction.v));
    return vec4<f32>(t * directional_light_color.v, 1.0);
}
