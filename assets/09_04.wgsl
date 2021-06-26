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
struct Wipe {
    size: f32;
};

[[group(2), binding(0)]]
var<uniform> wipe: Wipe;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return clamp((in.position.x % 64.0) - wipe.size, 0.0, 1.0) * textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
