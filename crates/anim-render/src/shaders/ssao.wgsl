// Screen-Space Ambient Occlusion (SSAO) with shadow lookup
// Uses Fibonacci spiral sampling pattern (9 samples, 7 turns)

struct SsaoParams {
    inv_view_proj: mat4x4<f32>,
    view_matrix: mat4x4<f32>,
    light_vp: mat4x4<f32>,
    screen_size: vec2<f32>,
    near: f32,
    far: f32,
    radius: f32,
    bias: f32,
    intensity: f32,
    shadow_bias: f32,
};

@group(0) @binding(0) var<uniform> params: SsaoParams;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_2d<f32>;
@group(0) @binding(3) var shadow_map: texture_depth_2d;
@group(0) @binding(4) var shadow_sampler: sampler_comparison;
@group(0) @binding(5) var tex_sampler: sampler;

const SSAO_SAMPLES: i32 = 9;
const SSAO_TURNS: f32 = 7.0;
const PI: f32 = 3.14159265359;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Full-screen triangle
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vi) / 2) * 4.0 - 1.0;
    let y = f32(i32(vi) % 2) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

fn reconstruct_position(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let world = params.inv_view_proj * ndc;
    return world.xyz / world.w;
}

fn hash(p: vec2<f32>) -> f32 {
    var h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth_val = textureSample(depth_tex, tex_sampler, in.uv).r;
    if depth_val >= 0.999 {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0); // No geometry: no occlusion, no shadow
    }

    // Decode normal
    let normal_raw = textureSample(normal_tex, tex_sampler, in.uv).rgb;
    let normal = normalize(normal_raw * 2.0 - 1.0);

    // Reconstruct world position from depth
    // Convert linear depth back to NDC depth
    let lin_depth = depth_val;
    let ndc_depth = (((2.0 * params.near) / lin_depth) - params.far - params.near) / (params.near - params.far);
    let world_pos = reconstruct_position(in.uv, ndc_depth);

    // Camera-space position for SSAO
    let cam_pos = (params.view_matrix * vec4<f32>(world_pos, 1.0)).xyz;
    let cam_normal = normalize((params.view_matrix * vec4<f32>(normal, 0.0)).xyz);

    // === SSAO ===
    let seed = hash(in.uv * params.screen_size);
    var occlusion = 0.0;

    for (var i = 0; i < SSAO_SAMPLES; i++) {
        let alpha = (f32(i) + 0.5) / f32(SSAO_SAMPLES);
        let angle = alpha * SSAO_TURNS * 2.0 * PI + 2.0 * PI * seed;
        let sample_radius = params.radius * alpha;

        let offset = vec2<f32>(cos(angle), sin(angle)) * sample_radius;
        let sample_uv = in.uv + offset / params.screen_size;

        let sample_depth = textureSample(depth_tex, tex_sampler, sample_uv).r;
        let sample_ndc = (((2.0 * params.near) / max(sample_depth, 0.001)) - params.far - params.near) / (params.near - params.far);
        let sample_world = reconstruct_position(sample_uv, sample_ndc);
        let sample_cam = (params.view_matrix * vec4<f32>(sample_world, 1.0)).xyz;

        let diff = sample_cam - cam_pos;
        let vv = dot(diff, diff);
        let vn = dot(diff, cam_normal) - params.bias;
        let f = max(params.radius * params.radius - vv, 0.0);
        occlusion += f * f * f * max(vn / (0.001 + vv), 0.0);
    }

    let ao = max(0.0, 1.0 - occlusion * params.intensity * (5.0 / f32(SSAO_SAMPLES)));

    // === Shadow ===
    let light_pos = params.light_vp * vec4<f32>(world_pos + normal * 0.01, 1.0);
    let light_ndc = light_pos.xyz / light_pos.w;
    let shadow_uv = vec2<f32>(light_ndc.x * 0.5 + 0.5, -light_ndc.y * 0.5 + 0.5);
    let shadow_depth = light_ndc.z;

    var shadow = 1.0;
    if shadow_uv.x >= 0.0 && shadow_uv.x <= 1.0 && shadow_uv.y >= 0.0 && shadow_uv.y <= 1.0 {
        shadow = textureSampleCompare(shadow_map, shadow_sampler, shadow_uv, shadow_depth - params.shadow_bias);
    }

    return vec4<f32>(ao, shadow, 0.0, 1.0);
}
