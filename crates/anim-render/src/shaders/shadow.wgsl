// Shadow map pass: depth-only rendering from light's perspective

struct LightUniforms {
    light_vp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> light: LightUniforms;
@group(1) @binding(0) var<storage, read> bone_matrices: array<mat4x4<f32>>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) bone_indices: vec4<u32>,
    @location(4) bone_weights: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    // GPU skinning
    var skin_matrix = mat4x4<f32>(
        vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0)
    );
    skin_matrix += bone_matrices[in.bone_indices.x] * in.bone_weights.x;
    skin_matrix += bone_matrices[in.bone_indices.y] * in.bone_weights.y;
    skin_matrix += bone_matrices[in.bone_indices.z] * in.bone_weights.z;
    skin_matrix += bone_matrices[in.bone_indices.w] * in.bone_weights.w;

    let world_pos = skin_matrix * vec4<f32>(in.position, 1.0);
    return light.light_vp * world_pos;
}

@fragment
fn fs_main() {
    // Depth-only pass, no color output needed
}
