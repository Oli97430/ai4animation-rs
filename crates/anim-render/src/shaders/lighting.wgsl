// Deferred lighting pass: combines G-buffer with SSAO and shadows

struct LightParams {
    camera_pos: vec3<f32>,
    exposure: f32,
    light_dir: vec3<f32>,
    sun_strength: f32,
    sun_color: vec3<f32>,
    sky_strength: f32,
    sky_color: vec3<f32>,
    ground_strength: f32,
    ambient_strength: f32,
    _pad: vec3<f32>,
};

@group(0) @binding(0) var<uniform> params: LightParams;
@group(0) @binding(1) var albedo_tex: texture_2d<f32>;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_2d<f32>;
@group(0) @binding(4) var ssao_tex: texture_2d<f32>;
@group(0) @binding(5) var tex_sampler: sampler;

const PI: f32 = 3.14159265359;

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

fn to_gamma(col: vec3<f32>) -> vec3<f32> {
    return pow(col, vec3<f32>(1.0 / 2.2));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth_val = textureSample(depth_tex, tex_sampler, in.uv).r;
    if depth_val >= 0.999 {
        // Sky/background: dark gradient
        let sky = mix(vec3<f32>(0.08, 0.08, 0.12), vec3<f32>(0.15, 0.15, 0.2), in.uv.y);
        return vec4<f32>(sky, 1.0);
    }

    // Sample G-buffer
    let albedo_spec = textureSample(albedo_tex, tex_sampler, in.uv);
    let normal_gloss = textureSample(normal_tex, tex_sampler, in.uv);
    let ssao_shadow = textureSample(ssao_tex, tex_sampler, in.uv);

    let albedo = albedo_spec.rgb;
    let specularity = albedo_spec.a;
    let normal = normalize(normal_gloss.rgb * 2.0 - 1.0);
    let glossiness = normal_gloss.a * 100.0;
    let ao = ssao_shadow.r;
    let shadow = ssao_shadow.g;

    // Reconstruct world position (approximation using camera for eye direction)
    let eye_dir = normalize(in.uv * 2.0 - 1.0);
    let light_dir = normalize(params.light_dir);
    let sky_dir = vec3<f32>(0.0, -1.0, 0.0);

    // === Diffuse ===
    // Sun diffuse
    let ndotl_sun = max(dot(normal, -light_dir), 0.0);
    let sun_diffuse = shadow * params.sun_strength * params.sun_color * albedo * ndotl_sun;

    // Sky diffuse
    let ndotl_sky = max(dot(normal, -sky_dir), 0.0);
    let sky_diffuse = params.sky_strength * params.sky_color * albedo * ndotl_sky;

    // Ground diffuse
    let ndotl_ground = max(dot(normal, sky_dir), 0.0);
    let ground_diffuse = params.ground_strength * params.sky_color * albedo * ndotl_ground;

    // === Specular (Blinn-Phong) ===
    let view_dir = normalize(params.camera_pos);
    let half_sun = normalize(-light_dir + view_dir);
    let spec_sun = specularity * ((glossiness + 2.0) / (8.0 * PI)) * pow(max(dot(normal, half_sun), 0.0), glossiness);
    let sun_specular = shadow * params.sun_strength * spec_sun;

    let half_sky = normalize(-sky_dir + view_dir);
    let spec_sky = specularity * ((glossiness + 2.0) / (8.0 * PI)) * pow(max(dot(normal, half_sky), 0.0), glossiness);
    let sky_specular = params.sky_strength * spec_sky;

    // === Ambient ===
    let ambient = ao * params.ambient_strength * params.sky_color * albedo;

    // === Combine ===
    let diffuse = sun_diffuse + sky_diffuse + ground_diffuse;
    let specular = sun_specular + sky_specular;
    let final_color = diffuse + ambient + specular;

    // Exposure + gamma correction
    let exposed = params.exposure * final_color;
    let gamma = to_gamma(exposed);

    return vec4<f32>(gamma, 1.0);
}
