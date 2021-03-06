struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};


[[stage(vertex)]]
fn vs_main([[location(0)]] position: vec4<f32>, [[location(1)]] color: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = position;
    out.color = color;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
