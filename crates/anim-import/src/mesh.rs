//! Mesh and skin data structures for imported models.

use glam::{Mat4, Vec3, Vec2};

/// A single mesh with vertex data ready for GPU upload.
#[derive(Clone)]
pub struct ImportedMesh {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texcoords: Vec<Vec2>,
    pub indices: Vec<u32>,
    /// Per-vertex: up to 4 bone indices.
    pub bone_indices: Vec<[u32; 4]>,
    /// Per-vertex: up to 4 bone weights.
    pub bone_weights: Vec<[f32; 4]>,
    /// Texture image (RGBA pixels).
    pub texture: Option<TextureData>,
    /// Normal map texture (RGB → tangent-space normals).
    pub normal_map: Option<TextureData>,
    /// Metallic-roughness map (G=roughness, B=metallic, like glTF).
    pub metallic_roughness_map: Option<TextureData>,
    /// Emission map (RGB).
    pub emission_map: Option<TextureData>,
    /// Material index for multi-material support.
    pub material_index: u32,
}

#[derive(Clone)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Skin binding data.
#[derive(Clone)]
pub struct ImportedSkin {
    /// Inverse bind matrices per joint.
    pub inverse_bind_matrices: Vec<Mat4>,
    /// Joint names in order.
    pub joint_names: Vec<String>,
    /// Joint indices (into the node array).
    pub joint_indices: Vec<usize>,
}

/// Complete imported model with mesh, skeleton, and animations.
pub struct ImportedModel {
    pub name: String,
    pub meshes: Vec<ImportedMesh>,
    pub skin: Option<ImportedSkin>,
    /// Joint hierarchy: (name, parent_index) pairs. parent = -1 for roots.
    pub joint_names: Vec<String>,
    pub parent_indices: Vec<i32>,
    /// Animation frames: [num_frames, num_joints] of Mat4 (global space).
    pub animation_frames: Option<AnimationData>,
}

pub struct AnimationData {
    pub frames: Vec<Vec<Mat4>>,  // [num_frames][num_joints]
    pub framerate: f32,
}

impl ImportedModel {
    pub fn num_joints(&self) -> usize {
        self.joint_names.len()
    }

    pub fn num_frames(&self) -> usize {
        self.animation_frames.as_ref().map_or(0, |a| a.frames.len())
    }
}
