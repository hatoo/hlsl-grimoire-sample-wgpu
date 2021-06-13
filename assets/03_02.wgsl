struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
    [[location(1)]] uv: vec2<f32>;
};

[[block]]
struct Uniforms {
    mat: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(vertex)]]
fn vs_main([[location(0)]] position: vec4<f32>, [[location(1)]] color: vec4<f32>, [[location(2)]] uv: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = uniforms.mat * position;
    out.color = color;
    out.uv = uv;
    return out;
}

[[group(1), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(1), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
