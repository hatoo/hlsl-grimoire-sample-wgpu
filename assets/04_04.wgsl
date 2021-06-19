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
struct DirectionLight {
    direction: vec3<f32>;
    color: vec3<f32>;
};

[[group(3), binding(0)]]
var<uniform> directional_light: DirectionLight;

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
    let eye_pos = vec3<f32>(0.0, 0.0, 0.0);
    
    let ref = reflect(directional_light.direction, in.normal);
    let to_eye = normalize(eye_pos - in.world_position.xyz);
    let specular = max(0.0, dot(ref, to_eye));
    let specular = pow(specular, 5.0);

    let diffuse: f32 = max(0.0, -1.0 * dot(in.normal, directional_light.direction));

    let ambient = vec3<f32>(0.3, 0.3, 0.3);

    return vec4<f32>((specular + diffuse) * directional_light.color + ambient, 1.0);
}
