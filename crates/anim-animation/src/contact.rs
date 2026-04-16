//! Contact detection module.
//!
//! Detects foot/hand contacts using a dual-criterion approach:
//! - Height threshold: joint Y position < threshold
//! - Velocity threshold: joint velocity magnitude < threshold

use glam::Vec3;
use crate::motion::Motion;

/// Configuration for a single contact sensor (e.g., one foot).
#[derive(Clone, Debug)]
pub struct ContactSensor {
    /// Joint name (e.g., "LeftFoot", "RightAnkle").
    pub bone_name: String,
    /// Resolved bone index (set at init from hierarchy).
    pub bone_index: usize,
    /// Height below which contact is possible (meters).
    pub height_threshold: f32,
    /// Velocity below which contact is detected (m/s).
    pub velocity_threshold: f32,
}

/// Contact detection module for a motion clip.
pub struct ContactModule {
    pub sensors: Vec<ContactSensor>,
}

/// Per-frame contact state for all sensors.
#[derive(Clone, Debug)]
pub struct ContactFrame {
    /// Position of each sensor joint.
    pub positions: Vec<Vec3>,
    /// Whether each sensor is in contact.
    pub contacts: Vec<bool>,
}

impl ContactModule {
    /// Create a contact module from a list of (bone_name, height_threshold, velocity_threshold).
    pub fn new(motion: &Motion, configs: &[(&str, f32, f32)]) -> Self {
        let sensors = configs.iter().filter_map(|&(name, h, v)| {
            motion.hierarchy.get_bone_index(name).map(|idx| ContactSensor {
                bone_name: name.to_string(),
                bone_index: idx,
                height_threshold: h,
                velocity_threshold: v,
            })
        }).collect();
        Self { sensors }
    }

    /// Auto-detect contact sensors from common bone naming conventions.
    pub fn auto_detect(motion: &Motion) -> Self {
        let patterns = [
            // (search patterns, height, velocity)
            (&["LeftFoot", "LeftAnkle", "lFoot", "L_Foot", "L_Ankle", "Left_Foot", "left_foot"][..], 0.10, 0.25),
            (&["LeftToeBase", "LeftBall", "lToe", "L_Toe", "Left_Toe", "left_toe"][..], 0.05, 0.25),
            (&["RightFoot", "RightAnkle", "rFoot", "R_Foot", "R_Ankle", "Right_Foot", "right_foot"][..], 0.10, 0.25),
            (&["RightToeBase", "RightBall", "rToe", "R_Toe", "Right_Toe", "right_toe"][..], 0.05, 0.25),
        ];

        let mut sensors = Vec::new();
        for (names, height, velocity) in &patterns {
            for name in *names {
                if let Some(idx) = motion.hierarchy.get_bone_index(name) {
                    sensors.push(ContactSensor {
                        bone_name: name.to_string(),
                        bone_index: idx,
                        height_threshold: *height,
                        velocity_threshold: *velocity,
                    });
                    break; // found one for this pattern, move to next
                }
            }
        }

        // Fallback: try case-insensitive partial match
        if sensors.is_empty() {
            for (i, name) in motion.hierarchy.bone_names.iter().enumerate() {
                let lower = name.to_lowercase();
                if lower.contains("foot") || lower.contains("ankle") || lower.contains("toe") {
                    sensors.push(ContactSensor {
                        bone_name: name.clone(),
                        bone_index: i,
                        height_threshold: 0.08,
                        velocity_threshold: 0.3,
                    });
                }
            }
        }

        Self { sensors }
    }

    /// Compute contacts at a given timestamp.
    pub fn get_contacts(&self, motion: &Motion, timestamp: f32, mirrored: bool) -> ContactFrame {
        let positions_all = motion.get_positions(timestamp, mirrored);
        let velocities_all = motion.get_velocities(timestamp, mirrored);

        let mut positions = Vec::with_capacity(self.sensors.len());
        let mut contacts = Vec::with_capacity(self.sensors.len());

        for sensor in &self.sensors {
            let idx = sensor.bone_index;
            if idx < positions_all.len() {
                let pos = positions_all[idx];
                let vel = velocities_all[idx];

                let height_ok = pos.y < sensor.height_threshold;
                let velocity_ok = vel.length() < sensor.velocity_threshold;

                positions.push(pos);
                contacts.push(height_ok && velocity_ok);
            } else {
                positions.push(Vec3::ZERO);
                contacts.push(false);
            }
        }

        ContactFrame { positions, contacts }
    }

    /// Compute contacts for multiple timestamps (for trajectory visualization).
    pub fn get_contacts_range(
        &self,
        motion: &Motion,
        timestamps: &[f32],
        mirrored: bool,
    ) -> Vec<ContactFrame> {
        timestamps.iter()
            .map(|&t| self.get_contacts(motion, t, mirrored))
            .collect()
    }

    pub fn sensor_count(&self) -> usize {
        self.sensors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }
}
