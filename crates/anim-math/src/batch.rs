//! Batch operations on arrays of transforms.
//! Replaces numpy broadcasting on [N, 4, 4] arrays.

use glam::{Mat4, Vec3};
use crate::transform::Transform;

/// Forward kinematics: compute global transforms from local transforms and parent indices.
/// parent_indices[i] = -1 means root (no parent).
pub fn forward_kinematics(local_transforms: &[Mat4], parent_indices: &[i32]) -> Vec<Mat4> {
    let n = local_transforms.len();
    let mut global = vec![Mat4::IDENTITY; n];
    for i in 0..n {
        let parent = parent_indices[i];
        if parent < 0 {
            global[i] = local_transforms[i];
        } else {
            global[i] = global[parent as usize] * local_transforms[i];
        }
    }
    global
}

/// Extract positions from an array of 4x4 transforms.
pub fn extract_positions(transforms: &[Mat4]) -> Vec<Vec3> {
    transforms.iter().map(|t| t.get_position()).collect()
}

/// Compute velocities from position arrays (numerical differentiation).
pub fn compute_velocities(positions_curr: &[Vec3], positions_prev: &[Vec3], dt: f32) -> Vec<Vec3> {
    let inv_dt = if dt > 0.0 { 1.0 / dt } else { 0.0 };
    positions_curr
        .iter()
        .zip(positions_prev.iter())
        .map(|(c, p)| (*c - *p) * inv_dt)
        .collect()
}

/// Interpolate between two transform arrays.
pub fn interpolate_transforms(a: &[Mat4], b: &[Mat4], t: f32) -> Vec<Mat4> {
    a.iter().zip(b.iter()).map(|(ma, mb)| ma.interpolate(mb, t)).collect()
}

/// Compute bone lengths from transforms and parent indices.
pub fn compute_bone_lengths(transforms: &[Mat4], parent_indices: &[i32]) -> Vec<f32> {
    let positions = extract_positions(transforms);
    let mut lengths = vec![0.0f32; transforms.len()];
    for i in 0..transforms.len() {
        let parent = parent_indices[i];
        if parent >= 0 {
            lengths[i] = (positions[i] - positions[parent as usize]).length();
        }
    }
    lengths
}
