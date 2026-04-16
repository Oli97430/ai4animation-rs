//! MotionModule — computes motion features (transforms, velocities) over time series.
//!
//! Extracts per-bone transforms and velocities at multiple timestamps,
//! with optional Gaussian smoothing.

use glam::{Mat4, Vec3};
use crate::motion::Motion;
use crate::time_series::TimeSeries;

/// Extracted motion features at multiple time samples.
#[derive(Debug, Clone)]
pub struct MotionFeatures {
    /// Transforms: [sample_count][joint_count] Mat4.
    pub transforms: Vec<Vec<Mat4>>,
    /// Velocities: [sample_count][joint_count] Vec3.
    pub velocities: Vec<Vec<Vec3>>,
    /// Timestamps used for each sample.
    pub timestamps: Vec<f32>,
    /// Joint names included (all or filtered).
    pub joint_names: Vec<String>,
}

/// Compute motion features over a time series at a given center timestamp.
pub fn compute_features(
    motion: &Motion,
    center_timestamp: f32,
    mirrored: bool,
    time_series: &TimeSeries,
    joint_filter: Option<&[&str]>,
    smoothing: f32,
) -> MotionFeatures {
    let timestamps = time_series.simulate_timestamps(center_timestamp);
    let num_joints = motion.num_joints();

    // Determine which joints to include
    let joint_indices: Vec<usize> = match joint_filter {
        Some(names) => {
            names.iter()
                .filter_map(|name| motion.hierarchy.get_bone_index(name))
                .collect()
        }
        None => (0..num_joints).collect(),
    };

    let joint_names: Vec<String> = joint_indices.iter()
        .filter_map(|&i| motion.hierarchy.get_bone_name(i).map(|s| s.to_string()))
        .collect();

    let mut all_transforms = Vec::with_capacity(timestamps.len());
    let mut all_velocities = Vec::with_capacity(timestamps.len());

    for &t in &timestamps {
        let clamped_t = t.clamp(0.0, motion.total_time());

        // Get transforms (with optional smoothing)
        let full_transforms = if smoothing > 0.0 {
            get_smoothed_transforms(motion, clamped_t, mirrored, smoothing)
        } else {
            motion.get_transforms_interpolated(clamped_t, mirrored)
        };

        // Get velocities
        let full_velocities = if smoothing > 0.0 {
            get_smoothed_velocities(motion, clamped_t, mirrored, smoothing)
        } else {
            motion.get_velocities(clamped_t, mirrored)
        };

        // Filter to selected joints
        let transforms: Vec<Mat4> = joint_indices.iter()
            .map(|&i| *full_transforms.get(i).unwrap_or(&Mat4::IDENTITY))
            .collect();
        let velocities: Vec<Vec3> = joint_indices.iter()
            .map(|&i| *full_velocities.get(i).unwrap_or(&Vec3::ZERO))
            .collect();

        all_transforms.push(transforms);
        all_velocities.push(velocities);
    }

    MotionFeatures {
        transforms: all_transforms,
        velocities: all_velocities,
        timestamps,
        joint_names,
    }
}

/// Gaussian-smoothed transforms.
fn get_smoothed_transforms(
    motion: &Motion,
    timestamp: f32,
    mirrored: bool,
    window: f32,
) -> Vec<Mat4> {
    let dt = motion.delta_time();
    let half_window = window * 0.5;
    let radius = (half_window / dt).ceil() as i32;
    let sigma = window / 6.0; // 3-sigma ≈ half window

    let num_joints = motion.num_joints();
    let mut accum_pos = vec![Vec3::ZERO; num_joints];
    let mut weight_sum = 0.0f32;

    for k in -radius..=radius {
        let t = timestamp + k as f32 * dt;
        let t_clamped = t.clamp(0.0, motion.total_time());
        let d = k as f32 * dt / sigma;
        let w = (-0.5 * d * d).exp();

        let transforms = motion.get_transforms_interpolated(t_clamped, mirrored);
        for (j, mat) in transforms.iter().enumerate() {
            accum_pos[j] += mat.col(3).truncate() * w;
        }
        weight_sum += w;
    }

    // For smoothing, we only smooth positions; rotations stay from the center frame
    let center_transforms = motion.get_transforms_interpolated(timestamp, mirrored);
    let mut result = center_transforms;
    for (j, mat) in result.iter_mut().enumerate() {
        let smoothed_pos = accum_pos[j] / weight_sum;
        // Replace position column
        *mat = Mat4::from_cols(
            mat.col(0),
            mat.col(1),
            mat.col(2),
            smoothed_pos.extend(1.0),
        );
    }

    result
}

/// Gaussian-smoothed velocities.
fn get_smoothed_velocities(
    motion: &Motion,
    timestamp: f32,
    mirrored: bool,
    window: f32,
) -> Vec<Vec3> {
    let dt = motion.delta_time();
    let half_window = window * 0.5;
    let radius = (half_window / dt).ceil() as i32;
    let sigma = window / 6.0;

    let num_joints = motion.num_joints();
    let mut accum = vec![Vec3::ZERO; num_joints];
    let mut weight_sum = 0.0f32;

    for k in -radius..=radius {
        let t = timestamp + k as f32 * dt;
        let t_clamped = t.clamp(0.0, motion.total_time());
        let d = k as f32 * dt / sigma;
        let w = (-0.5 * d * d).exp();

        let velocities = motion.get_velocities(t_clamped, mirrored);
        for (j, vel) in velocities.iter().enumerate() {
            accum[j] += *vel * w;
        }
        weight_sum += w;
    }

    accum.iter().map(|v| *v / weight_sum).collect()
}
