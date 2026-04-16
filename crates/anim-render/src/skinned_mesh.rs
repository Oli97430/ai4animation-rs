//! GPU skinned mesh rendering data.

use glam::Mat4;
use crate::vertex::SkinnedVertex;
use anim_import::mesh::{ImportedMesh, ImportedSkin};

/// Max bones per skinned mesh (GPU limit for uniform buffer).
pub const MAX_BONES: usize = 254;

/// Material properties for GPU upload.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniforms {
    pub base_color: [f32; 4],   // RGB=albedo, A=specularity
    pub properties: [f32; 4],   // x=glossiness, y=has_texture(0/1), z=metallic, w=roughness
}

/// GPU-ready skinned mesh data.
pub struct SkinnedMeshData {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    pub inverse_bind_matrices: Vec<Mat4>,
    pub bone_matrices: Vec<Mat4>,
    pub color: [f32; 4],
    pub specularity: f32,
    pub glossiness: f32,
    pub metallic: f32,
    pub roughness: f32,
    pub has_texture: bool,
    /// Raw RGBA texture data (if any).
    pub texture_data: Option<TextureRgba>,
}

/// Raw texture image for GPU upload.
pub struct TextureRgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl SkinnedMeshData {
    pub fn from_imported(meshes: &[ImportedMesh], skin: &ImportedSkin) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut base_vertex = 0u32;
        let mut texture_data = None;

        for mesh in meshes {
            for i in 0..mesh.vertices.len() {
                let v = mesh.vertices[i];
                let n = if i < mesh.normals.len() { mesh.normals[i] } else { glam::Vec3::Y };
                let uv = if i < mesh.texcoords.len() { mesh.texcoords[i] } else { glam::Vec2::ZERO };
                let bi = if i < mesh.bone_indices.len() { mesh.bone_indices[i] } else { [0; 4] };
                let bw = if i < mesh.bone_weights.len() { mesh.bone_weights[i] } else { [1.0, 0.0, 0.0, 0.0] };

                vertices.push(SkinnedVertex {
                    position: v.into(),
                    normal: n.into(),
                    texcoord: uv.into(),
                    bone_indices: bi,
                    bone_weights: bw,
                });
            }

            // Grab first texture found
            if texture_data.is_none() {
                if let Some(ref tex) = mesh.texture {
                    texture_data = Some(TextureRgba {
                        width: tex.width,
                        height: tex.height,
                        pixels: tex.pixels.clone(),
                    });
                }
            }

            for &idx in &mesh.indices {
                indices.push(idx + base_vertex);
            }
            base_vertex += mesh.vertices.len() as u32;
        }

        let num_joints = skin.inverse_bind_matrices.len().min(MAX_BONES);
        let mut ibm = vec![Mat4::IDENTITY; num_joints];
        ibm[..num_joints].copy_from_slice(&skin.inverse_bind_matrices[..num_joints]);

        let has_texture = texture_data.is_some();

        Self {
            vertices,
            indices,
            inverse_bind_matrices: ibm,
            bone_matrices: vec![Mat4::IDENTITY; num_joints],
            color: [0.85, 0.82, 0.78, 1.0],
            specularity: 0.5,
            glossiness: 10.0,
            metallic: 0.0,
            roughness: 0.5,
            has_texture,
            texture_data,
        }
    }

    /// Get material uniforms for GPU upload.
    pub fn material_uniforms(&self) -> MaterialUniforms {
        MaterialUniforms {
            base_color: [self.color[0], self.color[1], self.color[2], self.specularity],
            properties: [
                self.glossiness,
                if self.has_texture { 1.0 } else { 0.0 },
                self.metallic,
                self.roughness,
            ],
        }
    }

    /// Update bone matrices from current global transforms.
    pub fn update_bones(&mut self, global_transforms: &[Mat4]) {
        let n = self.bone_matrices.len().min(global_transforms.len());
        for i in 0..n {
            self.bone_matrices[i] = global_transforms[i] * self.inverse_bind_matrices[i];
        }
    }

    pub fn num_bones(&self) -> usize {
        self.bone_matrices.len()
    }

    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    pub fn num_indices(&self) -> usize {
        self.indices.len()
    }
}
