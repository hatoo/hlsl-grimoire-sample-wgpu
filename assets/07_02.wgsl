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
var t_ambient_occlusion: texture_2d<f32>;
[[group(2), binding(1)]]
var s_ambient_occlusion: sampler;

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

let PI: f32 = 3.14159265359;

fn lambert_diffuse(light_direction: vec3<f32>, normal: vec3<f32>) -> f32 {
    return max(0.0, -1.0 * dot(normal, light_direction)) / PI;
}

fn fresnel_diffuse(light_direction: vec3<f32>, normal: vec3<f32>, world_position: vec3<f32>, eye_position: vec3<f32>) -> f32 {
    let to_eye = eye_position - world_position;
    let nl = max(0.0, dot(normal, -light_direction));
    let nv = max(0.0, dot(normal, to_eye));

    return nl * nv;
}

fn phong_speclar(light_direction: vec3<f32>, normal: vec3<f32>, world_position: vec3<f32>, eye_position: vec3<f32>) -> f32 {
    let ref = reflect(light_direction, normal);
    let to_eye = normalize(eye_position - world_position);
    return pow(max(0.0, dot(ref, to_eye)), 5.0);
}

fn beckmann(m: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t4 = t * t * t * t;
    let m2 = m * m;
    let d = 1.0 / (4.0 * m2 * t4);
    return d * exp((-1.0 / m2) * (1.0 - t2) / t2);
}

fn spc_fresnel(f0: f32, u: f32) -> f32 {
    return f0 + (1.0 - f0) * pow(1.0 - u, 5.0);
}

fn cook_torrance_specular(light_direction: vec3<f32>, normal: vec3<f32>, world_position: vec3<f32>, eye_position: vec3<f32>, metalic: f32) -> f32 {
    let to_eye = eye_position - world_position;

    let micro_facet: f32 = 0.76;

    let f0 = metalic;
    
    let h = normalize(-light_direction + to_eye);

    let nh = max(0.0, dot(normal, h));
    let vh = max(0.0, dot(to_eye, h));
    let nl = max(0.0, dot(normal, -light_direction));
    let nv = max(0.0, dot(normal, to_eye));

    let d = beckmann(micro_facet, nh);

    let f = spc_fresnel(f0, vh);

    let g = min(1.0, min(2.0 * nh * nv / vh, 2.0 * nh * nl / vh));

    let m = PI * nv * nh;

    return max(g * d * g / m, 0.0);
}

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
    let specular = cook_torrance_specular(light.direction, in.normal, in.world_position.xyz, light.eye_position, 0.5);
    let diffuse = lambert_diffuse(light.direction, in.normal) * fresnel_diffuse(light.direction, in.normal, in.world_position.xyz, light.eye_position);;

    return vec4<f32>((specular + diffuse) * light.color + light.ambient, 1.0);
}
