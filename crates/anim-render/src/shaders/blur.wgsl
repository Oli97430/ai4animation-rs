// Gaussian blur pass (runs twice: horizontal then vertical)

struct BlurParams {
    direction: vec2<f32>,  // (1/w, 0) for horizontal, (0, 1/h) for vertical
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: BlurParams;
@group(0) @binding(1) var input_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vi) / 2) * 4.0 - 1.0;
    let y = f32(i32(vi) % 2) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 7-tap Gaussian blur: weights [0.006, 0.061, 0.242, 0.383, 0.242, 0.061, 0.006]
    let w0 = 0.383;
    let w1 = 0.242;
    let w2 = 0.061;
    let w3 = 0.006;

    var result = textureSample(input_tex, tex_sampler, in.uv) * w0;
    result += textureSample(input_tex, tex_sampler, in.uv + params.direction * 1.0) * w1;
    result += textureSample(input_tex, tex_sampler, in.uv - params.direction * 1.0) * w1;
    result += textureSample(input_tex, tex_sampler, in.uv + params.direction * 2.0) * w2;
    result += textureSample(input_tex, tex_sampler, in.uv - params.direction * 2.0) * w2;
    result += textureSample(input_tex, tex_sampler, in.uv + params.direction * 3.0) * w3;
    result += textureSample(input_tex, tex_sampler, in.uv - params.direction * 3.0) * w3;

    return result;
}
