struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] tangent: vec3<f32>;
    [[location(3)]] bitangent: vec3<f32>;
    [[location(4)]] color: vec4<f32>;
    [[location(5)]] tex_coords: vec2<f32>;
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
var t_normal: texture_2d<f32>;
[[group(2), binding(1)]]
var s_normal: sampler;

[[block]]
struct Light {
    eye_position: vec3<f32>;
    // Directional light
    direction: vec3<f32>;
    color: vec3<f32>;
    // Ambient light
    ambient: vec3<f32>;
};

[[group(3), binding(0)]]
var<uniform> light: Light;

[[stage(vertex)]]
fn vs_main([[location(0)]] position: vec4<f32>, [[location(1)]] normal: vec3<f32>, [[location(2)]] tangent: vec3<f32>, [[location(3)]] bitangent: vec3<f32>, [[location(4)]] color: vec4<f32>, [[location(5)]] tex_coords: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = global.mat * local.mat * position;
    out.world_position = global.mat * local.mat * position;
    out.normal = normalize((global.mat * local.mat * vec4<f32>(normal, 0.0)).xyz);
    out.tangent = normalize((global.mat * local.mat * vec4<f32>(tangent, 0.0)).xyz);
    out.bitangent = normalize((global.mat * local.mat * vec4<f32>(bitangent, 0.0)).xyz);
    out.color = color;
    out.tex_coords = tex_coords;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let local_normal = textureSample(t_normal, s_normal, in.tex_coords).xyz;
    let local_normal = normalize(local_normal * 2.0 - 1.0);
    let normal = in.tangent * local_normal.x + in.bitangent * local_normal.y + in.normal * local_normal.z;

    let ref = reflect(light.direction, normal);
    let to_eye = normalize(light.eye_position - in.world_position.xyz);
    let specular = max(0.0, dot(ref, to_eye));
    let specular = pow(specular, 5.0);

    let diffuse: f32 = max(0.0, -1.0 * dot(normal, light.direction));

    return vec4<f32>((specular + diffuse) * light.color + light.ambient, 1.0);
}
