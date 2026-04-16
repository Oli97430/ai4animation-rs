//! Root motion extraction: planar root position, rotation, velocity, and deltas.
//!
//! Mirrors the Python `RootModule` — extracts the locomotion-relevant root
//! transform by projecting onto the XZ ground plane (Y-up).

use glam::{Mat4, Quat, Vec3};
use crate::motion::Motion;

/// Topology hint for root orientation detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Topology {
    Biped,
    Quadruped,
}

/// Configuration for root motion extraction.
#[derive(Debug, Clone)]
pub struct RootConfig {
    pub topology: Topology,
    /// Index of the root joint (usually 0 = Hips).
    pub root_joint: usize,
    /// Smoothing window in seconds (0 = no smoothing).
    pub smoothing: f32,
    /// Up axis (default Y).
    pub up: Vec3,
}

impl Default for RootConfig {
    fn default() -> Self {
        Self {
            topology: Topology::Biped,
            root_joint: 0,
            smoothing: 0.0,
            up: Vec3::Y,
        }
    }
}

/// Extracted root data for a single frame.
#[derive(Debug, Clone, Copy)]
pub struct RootSample {
    /// Root position projected onto the ground plane (Y=0).
    pub position: Vec3,
    /// Root forward direction on the ground plane (normalized).
    pub forward: Vec3,
    /// Root rotation (around Y axis only).
    pub rotation: Quat,
    /// Root velocity (position delta / dt).
    pub velocity: Vec3,
    /// Angular velocity around up axis (rad/s).
    pub angular_velocity: f32,
}

/// Pre-computed root motion for an entire motion clip.
pub struct RootMotion {
    pub samples: Vec<RootSample>,
    pub framerate: f32,
}

impl RootMotion {
    /// Extract root motion from a motion clip.
    pub fn compute(motion: &Motion, config: &RootConfig) -> Self {
        let n = motion.num_frames();
        if n == 0 {
            return Self { samples: Vec::new(), framerate: motion.framerate };
        }

        let dt = motion.delta_time();

        // Phase 1: extract raw planar positions and forward directions
        let mut raw_positions = Vec::with_capacity(n);
        let mut raw_forwards = Vec::with_capacity(n);

        for f in 0..n {
            let transforms = &motion.frames[f];
            let root_pos = extract_position(transforms, config.root_joint);
            // Project position onto ground plane
            let planar_pos = project_to_plane(root_pos, config.up);
            raw_positions.push(planar_pos);

            let fwd = compute_forward(motion, f, config);
            raw_forwards.push(fwd);
        }

        // Phase 2: optional Gaussian smoothing
        let positions = if config.smoothing > 0.0 {
            gaussian_smooth_vec3(&raw_positions, config.smoothing, motion.framerate)
        } else {
            raw_positions
        };

        let forwards = if config.smoothing > 0.0 {
            let smoothed = gaussian_smooth_vec3(&raw_forwards, config.smoothing, motion.framerate);
            // Re-normalize after smoothing
            smoothed.iter().map(|f| {
                let projected = project_to_plane(*f, config.up);
                if projected.length_squared() > 1e-8 { projected.normalize() } else { Vec3::Z }
            }).collect()
        } else {
            raw_forwards
        };

        // Phase 3: build samples with velocities
        let mut samples = Vec::with_capacity(n);
        for i in 0..n {
            let position = positions[i];
            let forward = forwards[i];
            let rotation = quat_from_forward(forward, config.up);

            // Velocity: finite difference
            let velocity = if i > 0 {
                (positions[i] - positions[i - 1]) / dt
            } else if n > 1 {
                (positions[1] - positions[0]) / dt
            } else {
                Vec3::ZERO
            };

            // Angular velocity
            let angular_velocity = if i > 0 {
                let prev_fwd = forwards[i - 1];
                signed_angle_y(prev_fwd, forward, config.up) / dt
            } else if n > 1 {
                signed_angle_y(forwards[0], forwards[1], config.up) / dt
            } else {
                0.0
            };

            samples.push(RootSample {
                position,
                forward,
                rotation,
                velocity,
                angular_velocity,
            });
        }

        Self { samples, framerate: motion.framerate }
    }

    /// Get root sample at a given timestamp (interpolated).
    pub fn sample_at(&self, timestamp: f32) -> RootSample {
        if self.samples.is_empty() {
            return RootSample {
                position: Vec3::ZERO,
                forward: Vec3::Z,
                rotation: Quat::IDENTITY,
                velocity: Vec3::ZERO,
                angular_velocity: 0.0,
            };
        }

        let t = timestamp * self.framerate;
        let idx = t.floor() as usize;
        let frac = t - t.floor();

        if idx >= self.samples.len() - 1 {
            return *self.samples.last().unwrap();
        }

        let a = &self.samples[idx];
        let b = &self.samples[idx + 1];

        RootSample {
            position: a.position.lerp(b.position, frac),
            forward: a.forward.lerp(b.forward, frac).normalize_or_zero(),
            rotation: a.rotation.slerp(b.rotation, frac),
            velocity: a.velocity.lerp(b.velocity, frac),
            angular_velocity: a.angular_velocity + (b.angular_velocity - a.angular_velocity) * frac,
        }
    }

    /// Get delta transform between current and previous frame (in local root space).
    pub fn get_delta(&self, frame: usize) -> (Vec3, f32) {
        if frame == 0 || self.samples.is_empty() {
            return (Vec3::ZERO, 0.0);
        }
        let idx = frame.min(self.samples.len() - 1);
        let prev = &self.samples[idx - 1];
        let curr = &self.samples[idx];

        // Position delta in previous root's local space
        let inv_rot = prev.rotation.inverse();
        let world_delta = curr.position - prev.position;
        let local_delta = inv_rot * world_delta;

        // Rotation delta (angle around up axis)
        let angle_delta = signed_angle_y(prev.forward, curr.forward, Vec3::Y);

        (local_delta, angle_delta)
    }

    /// Get the delta vector as [dx, dangle, dz] (Python-compatible format).
    pub fn get_delta_vector(&self, frame: usize) -> Vec3 {
        let (local_delta, angle) = self.get_delta(frame);
        Vec3::new(local_delta.x, angle, local_delta.z)
    }

    pub fn num_frames(&self) -> usize {
        self.samples.len()
    }
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

fn extract_position(transforms: &[Mat4], joint: usize) -> Vec3 {
    if joint < transforms.len() {
        transforms[joint].col(3).truncate()
    } else {
        Vec3::ZERO
    }
}

fn project_to_plane(v: Vec3, up: Vec3) -> Vec3 {
    v - up * v.dot(up)
}

/// Compute the forward direction from the skeleton at a given frame.
fn compute_forward(motion: &Motion, frame: usize, config: &RootConfig) -> Vec3 {
    let transforms = &motion.frames[frame];
    let up = config.up;

    match config.topology {
        Topology::Biped => {
            // Try to find left/right hip and shoulder pairs to compute forward
            let names = &motion.hierarchy.bone_names;
            let hip_pair = find_pair(names, &["LeftUpLeg", "LeftHip", "L_Hip", "lThigh"],
                                              &["RightUpLeg", "RightHip", "R_Hip", "rThigh"]);
            let shoulder_pair = find_pair(names, &["LeftArm", "LeftShoulder", "L_Shoulder", "lShldr"],
                                                  &["RightArm", "RightShoulder", "R_Shoulder", "rShldr"]);

            let mut forward = Vec3::ZERO;
            let mut count = 0;

            if let Some((li, ri)) = hip_pair {
                let left = extract_position(transforms, li);
                let right = extract_position(transforms, ri);
                let across = (right - left).normalize_or_zero();
                let fwd = project_to_plane(across.cross(up), up);
                if fwd.length_squared() > 1e-8 {
                    forward += fwd.normalize();
                    count += 1;
                }
            }

            if let Some((li, ri)) = shoulder_pair {
                let left = extract_position(transforms, li);
                let right = extract_position(transforms, ri);
                let across = (right - left).normalize_or_zero();
                let fwd = project_to_plane(across.cross(up), up);
                if fwd.length_squared() > 1e-8 {
                    forward += fwd.normalize();
                    count += 1;
                }
            }

            if count > 0 {
                let avg = forward / count as f32;
                let projected = project_to_plane(avg, up);
                if projected.length_squared() > 1e-8 {
                    return projected.normalize();
                }
            }

            // Fallback: use root joint's local Z axis projected
            let root_z = transforms[config.root_joint].col(2).truncate();
            let projected = project_to_plane(root_z, up);
            if projected.length_squared() > 1e-8 { projected.normalize() } else { Vec3::Z }
        }
        Topology::Quadruped => {
            // Use neck/head direction from root
            let names = &motion.hierarchy.bone_names;
            let head_idx = find_bone(names, &["Head", "Neck", "Neck1"]);

            if let Some(hi) = head_idx {
                let root_pos = extract_position(transforms, config.root_joint);
                let head_pos = extract_position(transforms, hi);
                let dir = head_pos - root_pos;
                let projected = project_to_plane(dir, up);
                if projected.length_squared() > 1e-8 {
                    return projected.normalize();
                }
            }

            let root_z = transforms[config.root_joint].col(2).truncate();
            let projected = project_to_plane(root_z, up);
            if projected.length_squared() > 1e-8 { projected.normalize() } else { Vec3::Z }
        }
    }
}

fn find_bone(names: &[String], candidates: &[&str]) -> Option<usize> {
    for candidate in candidates {
        let lower = candidate.to_lowercase();
        for (i, name) in names.iter().enumerate() {
            if name.to_lowercase() == lower || name.to_lowercase().contains(&lower) {
                return Some(i);
            }
        }
    }
    None
}

fn find_pair(names: &[String], left_candidates: &[&str], right_candidates: &[&str]) -> Option<(usize, usize)> {
    let left = find_bone(names, left_candidates)?;
    let right = find_bone(names, right_candidates)?;
    Some((left, right))
}

/// Quaternion that rotates Vec3::Z to the given forward direction on the XZ plane.
fn quat_from_forward(forward: Vec3, _up: Vec3) -> Quat {
    if forward.length_squared() < 1e-8 {
        return Quat::IDENTITY;
    }
    let angle = forward.z.atan2(forward.x) - std::f32::consts::FRAC_PI_2;
    Quat::from_rotation_y(-angle)
}

/// Signed angle between two vectors around the Y axis.
fn signed_angle_y(from: Vec3, to: Vec3, up: Vec3) -> f32 {
    let cross = from.cross(to);
    let dot = from.dot(to).clamp(-1.0, 1.0);
    let angle = dot.acos();
    if cross.dot(up) < 0.0 { -angle } else { angle }
}

/// Delegate to shared signal processing utility.
fn gaussian_smooth_vec3(data: &[Vec3], window_sec: f32, fps: f32) -> Vec<Vec3> {
    anim_math::signal::gaussian_smooth_vec3(data, window_sec, fps)
}
