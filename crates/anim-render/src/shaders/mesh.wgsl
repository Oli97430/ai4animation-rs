// Skinned mesh shader with basic lighting

struct ViewUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
    light_dir: vec3<f32>,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> view: ViewUniforms;
@group(1) @binding(0) var<storage, read> bone_matrices: array<mat4x4<f32>>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) bone_indices: vec4<u32>,
    @location(4) bone_weights: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // GPU skinning: weighted sum of bone matrices
    var skin_matrix = mat4x4<f32>(
        vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0)
    );
    skin_matrix += bone_matrices[in.bone_indices.x] * in.bone_weights.x;
    skin_matrix += bone_matrices[in.bone_indices.y] * in.bone_weights.y;
    skin_matrix += bone_matrices[in.bone_indices.z] * in.bone_weights.z;
    skin_matrix += bone_matrices[in.bone_indices.w] * in.bone_weights.w;

    let world_pos = skin_matrix * vec4<f32>(in.position, 1.0);
    let world_normal = normalize((skin_matrix * vec4<f32>(in.normal, 0.0)).xyz);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_pos;
    out.world_normal = world_normal;
    out.world_pos = world_pos.xyz;
    out.texcoord = in.texcoord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let light_dir = normalize(-view.light_dir);
    let view_dir = normalize(view.camera_pos - in.world_pos);
    let half_dir = normalize(light_dir + view_dir);

    // Base color
    let base_color = vec3<f32>(0.85, 0.82, 0.78);

    // Ambient
    let sky_color = vec3<f32>(0.4, 0.45, 0.55);
    let ground_color = vec3<f32>(0.15, 0.12, 0.1);
    let sky_factor = normal.y * 0.5 + 0.5;
    let ambient = mix(ground_color, sky_color, sky_factor) * 0.3;

    // Diffuse (sun)
    let sun_color = vec3<f32>(1.0, 0.95, 0.9);
    let ndotl = max(dot(normal, light_dir), 0.0);
    let diffuse = sun_color * ndotl * 0.7;

    // Specular
    let ndoth = max(dot(normal, half_dir), 0.0);
    let specular = sun_color * pow(ndoth, 32.0) * 0.3;

    let color = base_color * (ambient + diffuse) + specular;

    // Gamma correction
    let gamma = pow(color, vec3<f32>(1.0 / 2.2));
    return vec4<f32>(gamma, 1.0);
}
