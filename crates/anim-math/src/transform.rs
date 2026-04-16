//! 4x4 homogeneous transformation matrices.
//! Mirrors ai4animation/Math/Transform.py

use glam::{Mat4, Mat3, Vec3, Vec4, Quat};

/// Extension trait for Mat4 with animation-specific operations.
pub trait Transform {
    /// Create from translation + 3x3 rotation.
    fn from_tr(translation: Vec3, rotation: Mat3) -> Mat4;

    /// Create from translation + rotation + scale.
    fn from_trs(translation: Vec3, rotation: Mat3, scale: Vec3) -> Mat4;

    /// Extract position (column 3, top 3 elements).
    fn get_position(&self) -> Vec3;

    /// Set position in-place.
    fn set_position(&mut self, pos: Vec3);

    /// Extract 3x3 rotation.
    fn get_rotation(&self) -> Mat3;

    /// Set 3x3 rotation in-place.
    fn set_rotation(&mut self, rot: Mat3);

    /// Extract X axis (column 0).
    fn get_axis_x(&self) -> Vec3;

    /// Extract Y axis (column 1).
    fn get_axis_y(&self) -> Vec3;

    /// Extract Z axis (column 2).
    fn get_axis_z(&self) -> Vec3;

    /// Transform position from local space to space defined by this matrix.
    fn transform_position_from(&self, pos: Vec3) -> Vec3;

    /// Transform position from world to local space of this matrix.
    fn transform_position_to(&self, pos: Vec3) -> Vec3;

    /// Transform direction from local to world space.
    fn transform_direction_from(&self, dir: Vec3) -> Vec3;

    /// Transform direction from world to local space.
    fn transform_direction_to(&self, dir: Vec3) -> Vec3;

    /// Get transform relative to a space: space * self
    fn transformation_from(&self, space: &Mat4) -> Mat4;

    /// Get transform in local space: space^-1 * self
    fn transformation_to(&self, space: &Mat4) -> Mat4;

    /// Interpolate between two transforms.
    fn interpolate(&self, other: &Mat4, t: f32) -> Mat4;

    /// Mirror transform across an axis.
    fn get_mirror(&self, axis: MirrorAxis) -> Mat4;

    /// Build delta transform projected to XZ plane.
    fn delta_xz(delta: Vec3) -> Mat4;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MirrorAxis {
    X,
    Y,
    Z,
}

impl Transform for Mat4 {
    fn from_tr(translation: Vec3, rotation: Mat3) -> Mat4 {
        let cols = rotation.to_cols_array_2d();
        Mat4::from_cols(
            Vec4::new(cols[0][0], cols[0][1], cols[0][2], 0.0),
            Vec4::new(cols[1][0], cols[1][1], cols[1][2], 0.0),
            Vec4::new(cols[2][0], cols[2][1], cols[2][2], 0.0),
            Vec4::new(translation.x, translation.y, translation.z, 1.0),
        )
    }

    fn from_trs(translation: Vec3, rotation: Mat3, scale: Vec3) -> Mat4 {
        let cols = rotation.to_cols_array_2d();
        Mat4::from_cols(
            Vec4::new(cols[0][0] * scale.x, cols[0][1] * scale.x, cols[0][2] * scale.x, 0.0),
            Vec4::new(cols[1][0] * scale.y, cols[1][1] * scale.y, cols[1][2] * scale.y, 0.0),
            Vec4::new(cols[2][0] * scale.z, cols[2][1] * scale.z, cols[2][2] * scale.z, 0.0),
            Vec4::new(translation.x, translation.y, translation.z, 1.0),
        )
    }

    fn get_position(&self) -> Vec3 {
        Vec3::new(self.w_axis.x, self.w_axis.y, self.w_axis.z)
    }

    fn set_position(&mut self, pos: Vec3) {
        self.w_axis.x = pos.x;
        self.w_axis.y = pos.y;
        self.w_axis.z = pos.z;
    }

    fn get_rotation(&self) -> Mat3 {
        Mat3::from_cols(
            Vec3::new(self.x_axis.x, self.x_axis.y, self.x_axis.z),
            Vec3::new(self.y_axis.x, self.y_axis.y, self.y_axis.z),
            Vec3::new(self.z_axis.x, self.z_axis.y, self.z_axis.z),
        )
    }

    fn set_rotation(&mut self, rot: Mat3) {
        let cols = rot.to_cols_array_2d();
        self.x_axis.x = cols[0][0]; self.x_axis.y = cols[0][1]; self.x_axis.z = cols[0][2];
        self.y_axis.x = cols[1][0]; self.y_axis.y = cols[1][1]; self.y_axis.z = cols[1][2];
        self.z_axis.x = cols[2][0]; self.z_axis.y = cols[2][1]; self.z_axis.z = cols[2][2];
    }

    fn get_axis_x(&self) -> Vec3 {
        Vec3::new(self.x_axis.x, self.x_axis.y, self.x_axis.z)
    }

    fn get_axis_y(&self) -> Vec3 {
        Vec3::new(self.y_axis.x, self.y_axis.y, self.y_axis.z)
    }

    fn get_axis_z(&self) -> Vec3 {
        Vec3::new(self.z_axis.x, self.z_axis.y, self.z_axis.z)
    }

    fn transform_position_from(&self, pos: Vec3) -> Vec3 {
        self.get_position() + self.get_rotation() * pos
    }

    fn transform_position_to(&self, pos: Vec3) -> Vec3 {
        self.get_rotation().transpose() * (pos - self.get_position())
    }

    fn transform_direction_from(&self, dir: Vec3) -> Vec3 {
        self.get_rotation() * dir
    }

    fn transform_direction_to(&self, dir: Vec3) -> Vec3 {
        self.get_rotation().transpose() * dir
    }

    fn transformation_from(&self, space: &Mat4) -> Mat4 {
        *space * *self
    }

    fn transformation_to(&self, space: &Mat4) -> Mat4 {
        space.inverse() * *self
    }

    fn interpolate(&self, other: &Mat4, t: f32) -> Mat4 {
        let pos = self.get_position().lerp(other.get_position(), t);
        let q_a = Quat::from_mat3(&self.get_rotation());
        let q_b = Quat::from_mat3(&other.get_rotation());
        let rot = q_a.slerp(q_b, t);
        Mat4::from_rotation_translation(rot, pos)
    }

    fn get_mirror(&self, axis: MirrorAxis) -> Mat4 {
        let mut m = *self;
        match axis {
            MirrorAxis::X => {
                m.w_axis.x = -m.w_axis.x;
                m.x_axis.y = -m.x_axis.y; m.x_axis.z = -m.x_axis.z;
                m.y_axis.x = -m.y_axis.x;
                m.z_axis.x = -m.z_axis.x;
            }
            MirrorAxis::Y => {
                m.w_axis.y = -m.w_axis.y;
                m.x_axis.y = -m.x_axis.y;
                m.y_axis.x = -m.y_axis.x; m.y_axis.z = -m.y_axis.z;
                m.z_axis.y = -m.z_axis.y;
            }
            MirrorAxis::Z => {
                m.w_axis.z = -m.w_axis.z;
                m.x_axis.z = -m.x_axis.z;
                m.y_axis.z = -m.y_axis.z;
                m.z_axis.x = -m.z_axis.x; m.z_axis.y = -m.z_axis.y;
            }
        }
        m
    }

    fn delta_xz(delta: Vec3) -> Mat4 {
        let pos = Vec3::new(delta.x, 0.0, delta.z);
        let rot = Mat4::from_rotation_y(delta.y);
        let mut m = rot;
        m.set_position(pos);
        m
    }
}
