//! Trajectory module — root joint path visualization.
//!
//! Computes and stores the root joint's trajectory (past + future)
//! as a series of positions, forward directions, and velocities.

use glam::Vec3;
use anim_math::transform::Transform;
use crate::motion::Motion;

/// Trajectory sample at a single timestamp.
#[derive(Clone, Debug)]
pub struct TrajectorySample {
    pub position: Vec3,
    pub direction: Vec3, // forward (Z axis of root)
    pub velocity: Vec3,
    pub timestamp: f32,
}

/// A trajectory over a temporal window.
pub struct Trajectory {
    pub samples: Vec<TrajectorySample>,
    /// Time window start (negative = past).
    pub window_start: f32,
    /// Time window end (positive = future).
    pub window_end: f32,
}

/// Configuration for trajectory computation.
#[derive(Clone, Debug)]
pub struct TrajectoryConfig {
    /// Seconds into the past to show.
    pub past_window: f32,
    /// Seconds into the future to show.
    pub future_window: f32,
    /// Number of samples (total, uniformly distributed).
    pub sample_count: usize,
    /// Which joint to track (index). 0 = root.
    pub root_joint: usize,
}

impl Default for TrajectoryConfig {
    fn default() -> Self {
        Self {
            past_window: 0.5,
            future_window: 0.5,
            sample_count: 31,
            root_joint: 0,
        }
    }
}

impl Trajectory {
    /// Compute trajectory from a motion at the given timestamp.
    pub fn compute(
        motion: &Motion,
        timestamp: f32,
        mirrored: bool,
        config: &TrajectoryConfig,
    ) -> Self {
        let start = -config.past_window;
        let end = config.future_window;
        let n = config.sample_count.max(2);
        let dt_sample = (end - start) / (n - 1) as f32;
        let joint = config.root_joint;

        let total_time = motion.total_time();

        let mut samples = Vec::with_capacity(n);

        for i in 0..n {
            let t_offset = start + i as f32 * dt_sample;
            let t = (timestamp + t_offset).clamp(0.0, total_time);

            let transforms = motion.get_transforms_interpolated(t, mirrored);
            if joint >= transforms.len() {
                continue;
            }

            let root_mat = transforms[joint];
            let position = root_mat.get_position();
            let direction = root_mat.get_axis_z().normalize_or_zero();

            // Velocity via finite difference
            let t_prev = (t - motion.delta_time()).max(0.0);
            let prev_transforms = motion.get_transforms_interpolated(t_prev, mirrored);
            let prev_pos = if joint < prev_transforms.len() {
                prev_transforms[joint].get_position()
            } else {
                position
            };
            let inv_dt = if motion.delta_time() > 0.0 { 1.0 / motion.delta_time() } else { 0.0 };
            let velocity = (position - prev_pos) * inv_dt;

            samples.push(TrajectorySample {
                position,
                direction,
                velocity,
                timestamp: t,
            });
        }

        Self {
            samples,
            window_start: start,
            window_end: end,
        }
    }

    /// Get the "current" sample (closest to t=0, i.e., the present).
    pub fn current_sample(&self) -> Option<&TrajectorySample> {
        if self.samples.is_empty() {
            return None;
        }
        // The midpoint sample is the present
        Some(&self.samples[self.samples.len() / 2])
    }

    /// Get positions as a flat array (for line strip rendering).
    pub fn positions(&self) -> Vec<Vec3> {
        self.samples.iter().map(|s| s.position).collect()
    }

    /// Get past samples (timestamps <= current).
    pub fn past_samples(&self) -> &[TrajectorySample] {
        let mid = self.samples.len() / 2;
        &self.samples[..=mid.min(self.samples.len().saturating_sub(1))]
    }

    /// Get future samples (timestamps > current).
    pub fn future_samples(&self) -> &[TrajectorySample] {
        let mid = self.samples.len() / 2 + 1;
        if mid < self.samples.len() {
            &self.samples[mid..]
        } else {
            &[]
        }
    }
}
