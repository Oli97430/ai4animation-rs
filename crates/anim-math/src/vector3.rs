//! Vector3 utilities. Mirrors ai4animation/Math/Vector3.py

use glam::Vec3;

pub trait Vec3Ext {
    /// Signed angle between two vectors around an axis (degrees).
    fn signed_angle(from: Vec3, to: Vec3, axis: Vec3) -> f32;

    /// Exponential lerp with delta time.
    fn lerp_dt(a: Vec3, b: Vec3, dt: f32, rate: f32) -> Vec3;

    /// Spherical lerp between two vectors.
    fn slerp_vec(a: Vec3, b: Vec3, t: f32) -> Vec3;

    /// Exponential slerp with delta time.
    fn slerp_dt(a: Vec3, b: Vec3, dt: f32, rate: f32) -> Vec3;

    /// Clamp vector magnitude.
    fn clamp_magnitude(v: Vec3, max: f32) -> Vec3;
}

impl Vec3Ext for Vec3 {
    fn signed_angle(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
        let f = from.normalize();
        let t = to.normalize();
        let dot = f.dot(t).clamp(-1.0, 1.0);
        let angle = dot.acos();
        let cross = f.cross(t);
        let sign = if cross.dot(axis) >= 0.0 { 1.0 } else { -1.0 };
        (angle * sign).to_degrees()
    }

    fn lerp_dt(a: Vec3, b: Vec3, dt: f32, rate: f32) -> Vec3 {
        let t = 1.0 - (-dt * rate).exp();
        a.lerp(b, t)
    }

    fn slerp_vec(a: Vec3, b: Vec3, t: f32) -> Vec3 {
        let len_a = a.length();
        let len_b = b.length();
        if len_a < 1e-6 || len_b < 1e-6 {
            return a.lerp(b, t);
        }
        let na = a / len_a;
        let nb = b / len_b;
        let dot = na.dot(nb).clamp(-1.0, 1.0);
        let theta = dot.acos();
        let len = len_a + (len_b - len_a) * t;
        if theta.abs() < 1e-6 {
            return a.lerp(b, t);
        }
        let sin_theta = theta.sin();
        let wa = ((1.0 - t) * theta).sin() / sin_theta;
        let wb = (t * theta).sin() / sin_theta;
        (na * wa + nb * wb) * len
    }

    fn slerp_dt(a: Vec3, b: Vec3, dt: f32, rate: f32) -> Vec3 {
        let t = 1.0 - (-dt * rate).exp();
        Self::slerp_vec(a, b, t)
    }

    fn clamp_magnitude(v: Vec3, max: f32) -> Vec3 {
        let len = v.length();
        if len > max && len > 0.0 {
            v * (max / len)
        } else {
            v
        }
    }
}
