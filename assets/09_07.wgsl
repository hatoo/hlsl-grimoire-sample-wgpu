struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
};

[[block]]
struct Uniforms {
    mat: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> global: Uniforms;

[[stage(vertex)]]
fn vs_main([[location(0)]] position: vec4<f32>, [[location(1)]] tex_coords: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = global.mat * position;
    out.tex_coords = tex_coords;
    return out;
}

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[block]]
struct Effect {
    rate: f32;
};

[[group(2), binding(0)]]
var<uniform> effect: Effect;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    let y = 0.299 * color.r + 0.587 * color.g * 0.114 * color.b;

    let monochrome = vec3<f32>(y, y, y);

    return vec4<f32>(mix(color.xyz, monochrome, vec3<f32>(effect.rate)), color.a);
}
