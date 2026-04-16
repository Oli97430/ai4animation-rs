// FXAA (Fast Approximate Anti-Aliasing)

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

struct FxaaParams {
    texel_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(2) var<uniform> params: FxaaParams;

const SPAN_MAX: f32 = 4.0;
const REDUCE_AMOUNT: f32 = 0.25;  // 1/4
const REDUCE_MIN: f32 = 0.015625; // 1/64
const LUMA: vec3<f32> = vec3<f32>(0.299, 0.587, 0.114);

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
    let ts = params.texel_size;

    // Sample center and 4 corners
    let rgbM = textureSample(input_tex, tex_sampler, in.uv).rgb;
    let rgbNW = textureSample(input_tex, tex_sampler, in.uv + vec2<f32>(-1.0, -1.0) * ts).rgb;
    let rgbNE = textureSample(input_tex, tex_sampler, in.uv + vec2<f32>(1.0, -1.0) * ts).rgb;
    let rgbSW = textureSample(input_tex, tex_sampler, in.uv + vec2<f32>(-1.0, 1.0) * ts).rgb;
    let rgbSE = textureSample(input_tex, tex_sampler, in.uv + vec2<f32>(1.0, 1.0) * ts).rgb;

    // Compute luma for each sample
    let lumaNW = dot(rgbNW, LUMA);
    let lumaNE = dot(rgbNE, LUMA);
    let lumaSW = dot(rgbSW, LUMA);
    let lumaSE = dot(rgbSE, LUMA);
    let lumaM = dot(rgbM, LUMA);

    let lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    let lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));

    // Edge direction
    var dir: vec2<f32>;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y = ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    let dir_reduce = max((lumaNW + lumaNE + lumaSW + lumaSE) * (REDUCE_AMOUNT * 0.25), REDUCE_MIN);
    let rcp_dir_min = 1.0 / (min(abs(dir.x), abs(dir.y)) + dir_reduce);

    dir = clamp(dir * rcp_dir_min, vec2<f32>(-SPAN_MAX), vec2<f32>(SPAN_MAX)) * ts;

    // Sample along edge
    let rgbA = 0.5 * (
        textureSample(input_tex, tex_sampler, in.uv + dir * (1.0 / 3.0 - 0.5)).rgb +
        textureSample(input_tex, tex_sampler, in.uv + dir * (2.0 / 3.0 - 0.5)).rgb
    );
    let rgbB = rgbA * 0.5 + 0.25 * (
        textureSample(input_tex, tex_sampler, in.uv + dir * -0.5).rgb +
        textureSample(input_tex, tex_sampler, in.uv + dir * 0.5).rgb
    );

    let lumaB = dot(rgbB, LUMA);

    // Choose based on luma range
    var result: vec3<f32>;
    if lumaB < lumaMin || lumaB > lumaMax {
        result = rgbA;
    } else {
        result = rgbB;
    }

    return vec4<f32>(result, 1.0);
}
