//! 3x3 rotation matrix utilities. Mirrors ai4animation/Math/Rotation.py

use glam::{Mat3, Vec3, Quat};

pub trait Rotation {
    /// Create from Euler angles in degrees (Y * X * Z).
    fn from_euler_degrees(x: f32, y: f32, z: f32) -> Mat3;

    /// Rotation around X axis (degrees).
    fn rotation_x(angle: f32) -> Mat3;
    /// Rotation around Y axis (degrees).
    fn rotation_y(angle: f32) -> Mat3;
    /// Rotation around Z axis (degrees).
    fn rotation_z(angle: f32) -> Mat3;

    /// Build rotation looking along Z with Y up.
    fn look(forward: Vec3, up: Vec3) -> Mat3;

    /// Build rotation looking along Z projected to XZ plane.
    fn look_planar(forward: Vec3) -> Mat3;

    /// Angle-axis rotation (degrees).
    fn from_angle_axis(angle: f32, axis: Vec3) -> Mat3;

    /// Interpolate between two rotations (via quaternion slerp).
    fn interpolate(&self, other: &Mat3, t: f32) -> Mat3;

    /// Rotate vector by this rotation.
    fn rotate_vector(&self, v: Vec3) -> Vec3;

    /// Transform rotation from local to world space.
    fn rotation_from(&self, space: &Mat3) -> Mat3;

    /// Transform rotation from world to local space.
    fn rotation_to(&self, space: &Mat3) -> Mat3;

    /// Normalize rotation matrix (Gram-Schmidt orthogonalization).
    fn orthonormalize(&self) -> Mat3;
}

impl Rotation for Mat3 {
    fn from_euler_degrees(x: f32, y: f32, z: f32) -> Mat3 {
        let q = Quat::from_rotation_y(y.to_radians())
            * Quat::from_rotation_x(x.to_radians())
            * Quat::from_rotation_z(z.to_radians());
        Mat3::from_quat(q)
    }

    fn rotation_x(angle: f32) -> Mat3 {
        Mat3::from_quat(Quat::from_rotation_x(angle.to_radians()))
    }

    fn rotation_y(angle: f32) -> Mat3 {
        Mat3::from_quat(Quat::from_rotation_y(angle.to_radians()))
    }

    fn rotation_z(angle: f32) -> Mat3 {
        Mat3::from_quat(Quat::from_rotation_z(angle.to_radians()))
    }

    fn look(forward: Vec3, up: Vec3) -> Mat3 {
        let z = forward.normalize();
        let x = up.cross(z).normalize();
        let y = z.cross(x).normalize();
        Mat3::from_cols(x, y, z)
    }

    fn look_planar(forward: Vec3) -> Mat3 {
        let z = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let y = Vec3::Y;
        let x = y.cross(z).normalize();
        Mat3::from_cols(x, y, z)
    }

    fn from_angle_axis(angle: f32, axis: Vec3) -> Mat3 {
        Mat3::from_quat(Quat::from_axis_angle(axis.normalize(), angle.to_radians()))
    }

    fn interpolate(&self, other: &Mat3, t: f32) -> Mat3 {
        let qa = Quat::from_mat3(self);
        let qb = Quat::from_mat3(other);
        Mat3::from_quat(qa.slerp(qb, t))
    }

    fn rotate_vector(&self, v: Vec3) -> Vec3 {
        *self * v
    }

    fn rotation_from(&self, space: &Mat3) -> Mat3 {
        *space * *self
    }

    fn rotation_to(&self, space: &Mat3) -> Mat3 {
        space.transpose() * *self
    }

    fn orthonormalize(&self) -> Mat3 {
        let z = self.z_axis.normalize();
        let x = self.y_axis.cross(z).normalize();
        let y = z.cross(x).normalize();
        Mat3::from_cols(x, y, z)
    }
}
