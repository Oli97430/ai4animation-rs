//! Tracking module — track specific joints (head, wrists) with temporal smoothing.
//!
//! Extracts bone transforms/velocities at specified timestamps with
//! optional Gaussian smoothing, and visualizes trajectories.

use glam::Vec3;
use crate::motion::Motion;

/// Configuration for a single tracked joint.
#[derive(Clone, Debug)]
pub struct TrackedJoint {
    pub bone_name: String,
    pub bone_index: usize,
}

/// Tracking module for specific joints.
pub struct TrackingModule {
    pub joints: Vec<TrackedJoint>,
    pub smoothing_window: f32,
    pub sample_count: usize,
    pub visible: bool,
}

/// A single tracking sample (one timestamp, one joint).
#[derive(Clone, Debug)]
pub struct TrackingSample {
    pub position: Vec3,
    pub velocity: Vec3,
    pub timestamp: f32,
}

/// Computed tracking data for all joints across a time window.
pub struct TrackingFrame {
    /// [joint_idx][sample_idx]
    pub trajectories: Vec<Vec<TrackingSample>>,
    pub joint_names: Vec<String>,
}

impl TrackingModule {
    /// Create from explicit joint names.
    pub fn new(motion: &Motion, bone_names: &[&str]) -> Self {
        let joints = bone_names.iter().filter_map(|&name| {
            motion.hierarchy.get_bone_index(name).map(|idx| TrackedJoint {
                bone_name: name.to_string(),
                bone_index: idx,
            })
        }).collect();

        Self {
            joints,
            smoothing_window: 0.5,
            sample_count: 15,
            visible: true,
        }
    }

    /// Auto-detect: head + wrists.
    pub fn auto_detect(motion: &Motion) -> Self {
        let patterns = [
            &["Head", "head", "HEAD"][..],
            &["LeftHand", "lHand", "L_Hand", "Left_Hand", "left_hand"],
            &["RightHand", "rHand", "R_Hand", "Right_Hand", "right_hand"],
        ];

        let mut joints = Vec::new();
        for names in &patterns {
            for name in *names {
                if let Some(idx) = motion.hierarchy.get_bone_index(name) {
                    joints.push(TrackedJoint {
                        bone_name: name.to_string(),
                        bone_index: idx,
                    });
                    break;
                }
            }
        }

        // Fallback case-insensitive
        if joints.is_empty() {
            for target in &["head", "hand"] {
                for (i, name) in motion.hierarchy.bone_names.iter().enumerate() {
                    if name.to_lowercase().contains(target) {
                        joints.push(TrackedJoint {
                            bone_name: name.clone(),
                            bone_index: i,
                        });
                    }
                }
            }
        }

        Self {
            joints,
            smoothing_window: 0.5,
            sample_count: 15,
            visible: true,
        }
    }

    /// Compute tracking trajectories over a time window centered on `timestamp`.
    pub fn compute(
        &self,
        motion: &Motion,
        timestamp: f32,
        mirrored: bool,
    ) -> TrackingFrame {
        let half_window = self.smoothing_window;
        let n = self.sample_count.max(2);
        let total_time = motion.total_time();
        let dt_sample = (2.0 * half_window) / (n - 1) as f32;

        let mut trajectories = Vec::with_capacity(self.joints.len());
        let joint_names: Vec<String> = self.joints.iter()
            .map(|j| j.bone_name.clone())
            .collect();

        for joint in &self.joints {
            let mut samples = Vec::with_capacity(n);

            for i in 0..n {
                let t = (timestamp - half_window + i as f32 * dt_sample)
                    .clamp(0.0, total_time);

                let positions = motion.get_positions(t, mirrored);
                let velocities = motion.get_velocities(t, mirrored);

                let pos = positions.get(joint.bone_index).copied().unwrap_or(Vec3::ZERO);
                let vel = velocities.get(joint.bone_index).copied().unwrap_or(Vec3::ZERO);

                samples.push(TrackingSample {
                    position: pos,
                    velocity: vel,
                    timestamp: t,
                });
            }

            trajectories.push(samples);
        }

        TrackingFrame { trajectories, joint_names }
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }
}
