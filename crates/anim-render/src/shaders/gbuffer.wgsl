// G-Buffer pass: outputs position, normal+glossiness, albedo+specularity
// Supports per-mesh material color and optional albedo texture

struct ViewUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    near: f32,
    light_dir: vec3<f32>,
    far: f32,
};

struct MaterialUniforms {
    base_color: vec4<f32>,     // RGB=albedo, A=specularity
    properties: vec4<f32>,     // x=glossiness, y=has_texture(0/1), z=metallic, w=roughness
};

@group(0) @binding(0) var<uniform> view: ViewUniforms;
@group(1) @binding(0) var<storage, read> bone_matrices: array<mat4x4<f32>>;
@group(2) @binding(0) var<uniform> material: MaterialUniforms;
@group(2) @binding(1) var t_albedo: texture_2d<f32>;
@group(2) @binding(2) var s_albedo: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) bone_indices: vec4<u32>,
    @location(4) bone_weights: vec4<f32>,
};

struct GBufferOutput {
    @location(0) albedo_spec: vec4<f32>,    // RGB=albedo, A=specularity
    @location(1) normal_gloss: vec4<f32>,   // RGB=encoded normal, A=glossiness/100
    @location(2) depth: vec4<f32>,          // R=linear depth
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // GPU skinning (4 bones per vertex)
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

fn linear_depth(depth: f32, near: f32, far: f32) -> f32 {
    return (2.0 * near) / (far + near - depth * (far - near));
}

@fragment
fn fs_main(in: VertexOutput) -> GBufferOutput {
    let normal = normalize(in.world_normal);

    // Material color from uniform
    var albedo = material.base_color.rgb;
    let specularity = material.base_color.a;
    let glossiness = material.properties.x;
    let has_texture = material.properties.y;

    // Sample albedo texture if available
    if has_texture > 0.5 {
        let tex_color = textureSample(t_albedo, s_albedo, in.texcoord);
        albedo = tex_color.rgb * albedo;
    }

    // Encode normal to [0,1] range
    let encoded_normal = normal * 0.5 + 0.5;

    // Linear depth
    let depth = linear_depth(in.clip_position.z, view.near, view.far);

    var out: GBufferOutput;
    out.albedo_spec = vec4<f32>(albedo, specularity);
    out.normal_gloss = vec4<f32>(encoded_normal, glossiness / 100.0);
    out.depth = vec4<f32>(depth, 0.0, 0.0, 1.0);
    return out;
}
