//! MeshRenderer component — syncs an entity's transform to its rendered mesh.
//!
//! Mirrors Python Components/MeshRenderer.py.
//! In the Rust version, this is a lightweight tag that connects an entity
//! to a skinned mesh or static mesh in the render system.

use glam::{Vec3, Mat4, Quat};
use anim_math::transform::Transform;

/// Index into the renderer's mesh registry.
pub type MeshHandle = usize;

/// MeshRenderer component: bridges an entity's transform to the render system.
pub struct MeshRenderer {
    /// Handle to the registered mesh in the render system.
    pub mesh_handle: MeshHandle,
    /// Cached position for the renderer.
    pub position: Vec3,
    /// Cached rotation (angle-axis) for the renderer.
    pub rotation_axis: Vec3,
    pub rotation_angle: f32,
    /// Cached scale for the renderer.
    pub scale: Vec3,
    /// Whether this renderer is visible.
    pub visible: bool,
}

impl MeshRenderer {
    pub fn new(mesh_handle: MeshHandle) -> Self {
        Self {
            mesh_handle,
            position: Vec3::ZERO,
            rotation_axis: Vec3::Y,
            rotation_angle: 0.0,
            scale: Vec3::ONE,
            visible: true,
        }
    }

    /// Sync renderer state from an entity's 4x4 transform matrix.
    pub fn update_from_transform(&mut self, transform: &Mat4) {
        self.position = transform.get_position();

        // Decompose rotation to angle-axis
        let rot_mat = transform.get_rotation();
        let quat = Quat::from_mat3(&rot_mat);
        let (axis, angle) = quat.to_axis_angle();
        self.rotation_axis = axis;
        self.rotation_angle = angle.to_degrees();

        // Extract scale from column lengths
        let col_x = transform.x_axis.truncate();
        let col_y = transform.y_axis.truncate();
        let col_z = transform.z_axis.truncate();
        self.scale = Vec3::new(col_x.length(), col_y.length(), col_z.length());
    }

    /// Sync from an entity's transform + explicit scale.
    pub fn update_from_transform_and_scale(&mut self, transform: &Mat4, scale: Vec3) {
        self.position = transform.get_position();
        let rot_mat = transform.get_rotation();
        let quat = Quat::from_mat3(&rot_mat);
        let (axis, angle) = quat.to_axis_angle();
        self.rotation_axis = axis;
        self.rotation_angle = angle.to_degrees();
        self.scale = scale;
    }
}
