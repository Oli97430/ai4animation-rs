//! Guidance module — interactive guide skeleton overlay.
//!
//! Creates a visual reference skeleton showing target positions,
//! used for motion editing and control point visualization.

use glam::Vec3;
use crate::motion::Motion;

/// A single guidance control point.
#[derive(Clone, Debug)]
pub struct GuidancePoint {
    pub bone_name: String,
    pub bone_index: usize,
    pub target_position: Vec3,
    pub locked: bool,
}

/// Guidance module for interactive motion control.
pub struct GuidanceModule {
    pub points: Vec<GuidancePoint>,
    pub smoothing_window: f32,
    pub visible: bool,
}

/// Computed guidance frame for visualization.
pub struct GuidanceFrame {
    pub positions: Vec<Vec3>,
    pub bone_indices: Vec<usize>,
}

impl GuidanceModule {
    /// Create from a list of bone names to guide.
    pub fn new(motion: &Motion, bone_names: &[&str]) -> Self {
        let points = bone_names.iter().filter_map(|&name| {
            motion.hierarchy.get_bone_index(name).map(|idx| GuidancePoint {
                bone_name: name.to_string(),
                bone_index: idx,
                target_position: Vec3::ZERO,
                locked: false,
            })
        }).collect();

        Self {
            points,
            smoothing_window: 0.5,
            visible: true,
        }
    }

    /// Auto-create guidance for key bones (hips, hands, feet, head).
    pub fn auto_detect(motion: &Motion) -> Self {
        let key_patterns = [
            &["Hips", "pelvis", "hip", "Root"][..],
            &["Head", "head"],
            &["LeftHand", "lHand", "L_Hand", "Left_Hand"],
            &["RightHand", "rHand", "R_Hand", "Right_Hand"],
            &["LeftFoot", "lFoot", "L_Foot", "Left_Foot"],
            &["RightFoot", "rFoot", "R_Foot", "Right_Foot"],
        ];

        let mut points = Vec::new();
        for names in &key_patterns {
            for name in *names {
                if let Some(idx) = motion.hierarchy.get_bone_index(name) {
                    points.push(GuidancePoint {
                        bone_name: name.to_string(),
                        bone_index: idx,
                        target_position: Vec3::ZERO,
                        locked: false,
                    });
                    break;
                }
            }
        }

        // Fallback: case-insensitive
        if points.is_empty() {
            let targets = ["hip", "head", "hand", "foot"];
            for target in &targets {
                for (i, name) in motion.hierarchy.bone_names.iter().enumerate() {
                    if name.to_lowercase().contains(target) {
                        points.push(GuidancePoint {
                            bone_name: name.clone(),
                            bone_index: i,
                            target_position: Vec3::ZERO,
                            locked: false,
                        });
                        break;
                    }
                }
            }
        }

        Self {
            points,
            smoothing_window: 0.5,
            visible: true,
        }
    }

    /// Compute guidance positions at current timestamp.
    pub fn compute(&self, motion: &Motion, timestamp: f32, mirrored: bool) -> GuidanceFrame {
        let positions_all = motion.get_positions(timestamp, mirrored);

        // Apply optional smoothing by averaging nearby frames
        let smoothed = if self.smoothing_window > 0.01 {
            let dt = motion.delta_time();
            let half_window = self.smoothing_window / 2.0;
            let steps = (half_window / dt).ceil() as i32;
            let total = motion.total_time();

            let mut averaged = positions_all.clone();
            if steps > 0 {
                for pt in &self.points {
                    let idx = pt.bone_index;
                    if idx >= averaged.len() { continue; }
                    let mut sum = Vec3::ZERO;
                    let mut count = 0;
                    for s in -steps..=steps {
                        let t = (timestamp + s as f32 * dt).clamp(0.0, total);
                        let p = motion.get_positions(t, mirrored);
                        if idx < p.len() {
                            sum += p[idx];
                            count += 1;
                        }
                    }
                    if count > 0 {
                        averaged[idx] = sum / count as f32;
                    }
                }
            }
            averaged
        } else {
            positions_all
        };

        let mut positions = Vec::with_capacity(self.points.len());
        let mut bone_indices = Vec::with_capacity(self.points.len());

        for pt in &self.points {
            let pos = if pt.locked {
                pt.target_position
            } else if pt.bone_index < smoothed.len() {
                smoothed[pt.bone_index]
            } else {
                Vec3::ZERO
            };
            positions.push(pos);
            bone_indices.push(pt.bone_index);
        }

        GuidanceFrame { positions, bone_indices }
    }

    /// Lock a point to its current position (for manual editing).
    pub fn lock_point(&mut self, index: usize, position: Vec3) {
        if let Some(pt) = self.points.get_mut(index) {
            pt.target_position = position;
            pt.locked = true;
        }
    }

    /// Unlock a point (return to following the motion).
    pub fn unlock_point(&mut self, index: usize) {
        if let Some(pt) = self.points.get_mut(index) {
            pt.locked = false;
        }
    }

    pub fn point_count(&self) -> usize {
        self.points.len()
    }
}
