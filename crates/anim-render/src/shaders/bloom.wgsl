// Bloom post-process: box blur blended with original via intensity

struct BloomParams {
    texel_size: vec2<f32>,
    spread: f32,
    intensity: f32,
};

@group(0) @binding(0) var<uniform> params: BloomParams;
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
    let source = textureSample(input_tex, tex_sampler, in.uv);

    // Early out when bloom is disabled
    if params.intensity <= 0.001 {
        return source;
    }

    // 5x5 box blur for bloom glow
    var bloom = vec4<f32>(0.0);
    let range = 2;
    let samples = f32((range * 2 + 1) * (range * 2 + 1));

    for (var x = -range; x <= range; x++) {
        for (var y = -range; y <= range; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * params.texel_size * params.spread;
            bloom += textureSample(input_tex, tex_sampler, in.uv + offset);
        }
    }
    bloom /= samples;

    // Blend: 0 = source only, 1 = fully blurred
    return mix(source, bloom, params.intensity);
}
