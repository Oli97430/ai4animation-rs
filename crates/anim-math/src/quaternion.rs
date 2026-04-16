//! Quaternion utilities. Mirrors ai4animation/Math/Quaternion.py

use glam::{Quat, Vec3, Mat3};

pub trait QuatExt {
    /// Create from Euler angles in degrees (Y * X * Z order).
    fn from_euler_degrees(x: f32, y: f32, z: f32) -> Quat;

    /// Create rotation from one vector to another.
    fn from_to(from: Vec3, to: Vec3) -> Quat;

    /// Rotate a vector by this quaternion.
    fn rotate_vector(&self, v: Vec3) -> Vec3;

    /// Convert to angle (degrees) and axis.
    fn to_angle_axis(&self) -> (f32, Vec3);

    /// Create from angle (degrees) and axis.
    fn from_angle_axis(angle_deg: f32, axis: Vec3) -> Quat;
}

impl QuatExt for Quat {
    fn from_euler_degrees(x: f32, y: f32, z: f32) -> Quat {
        let rx = x.to_radians();
        let ry = y.to_radians();
        let rz = z.to_radians();
        // Y * X * Z order (matching Python: Ry @ Rx @ Rz)
        Quat::from_rotation_y(ry) * Quat::from_rotation_x(rx) * Quat::from_rotation_z(rz)
    }

    fn from_to(from: Vec3, to: Vec3) -> Quat {
        let f = from.normalize();
        let t = to.normalize();
        let dot = f.dot(t);

        if dot > 0.999999 {
            return Quat::IDENTITY;
        }
        if dot < -0.999999 {
            // 180 degree rotation - pick arbitrary perpendicular axis
            let mut axis = Vec3::X.cross(f);
            if axis.length_squared() < 0.0001 {
                axis = Vec3::Y.cross(f);
            }
            return Quat::from_axis_angle(axis.normalize(), std::f32::consts::PI);
        }

        let axis = f.cross(t);
        let w = 1.0 + dot;
        Quat::from_xyzw(axis.x, axis.y, axis.z, w).normalize()
    }

    fn rotate_vector(&self, v: Vec3) -> Vec3 {
        *self * v
    }

    fn to_angle_axis(&self) -> (f32, Vec3) {
        let (axis, angle) = self.to_axis_angle();
        (angle.to_degrees(), axis)
    }

    fn from_angle_axis(angle_deg: f32, axis: Vec3) -> Quat {
        Quat::from_axis_angle(axis.normalize(), angle_deg.to_radians())
    }
}

/// Convert 3x3 rotation matrix to quaternion.
pub fn mat3_to_quat(m: Mat3) -> Quat {
    Quat::from_mat3(&m)
}

/// Convert quaternion to 3x3 rotation matrix.
pub fn quat_to_mat3(q: Quat) -> Mat3 {
    Mat3::from_quat(q)
}
