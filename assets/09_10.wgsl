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

fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

fn simplex_noise(x: vec3<f32>) -> f32 {
    let p = floor(x);
    let f = fract(x);

    let f = f * f * (3.0 - 2.0 * f);
    let n = p.x + p.y * 57.0 + 113.0 * p.z;

    return mix(mix(mix(hash(n + 0.0), hash(n + 1.0), f.x),
                     mix(hash(n + 57.0), hash(n + 58.0), f.x), f.y),
                mix(mix(hash(n + 113.0), hash(n + 114.0), f.x),
                     mix(hash(n + 170.0), hash(n + 171.0), f.x), f.y), f.z);
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let t = simplex_noise(in.position.xyz);
    let t = (t - 0.5) * 2.0;

    let uv = in.tex_coords + t * 0.01;

    return textureSample(t_diffuse, s_diffuse, uv);
}
