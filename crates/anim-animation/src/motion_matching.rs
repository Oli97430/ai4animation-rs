//! Motion matching: real-time animation search from a motion database.
//!
//! Builds a database of per-frame feature vectors from loaded motion clips,
//! then finds the best matching frame given a query (current pose + desired trajectory).

use glam::{Vec3, Mat4};
use anim_math::transform::Transform;
use crate::motion::Motion;
use crate::blend::AnimationTransition;

/// Feature weights for controlling which aspects matter most in matching.
#[derive(Debug, Clone)]
pub struct MatchWeights {
    /// Weight for trajectory positions (future path).
    pub trajectory_position: f32,
    /// Weight for trajectory directions (facing).
    pub trajectory_direction: f32,
    /// Weight for joint positions (current pose).
    pub pose_position: f32,
    /// Weight for joint velocities (momentum).
    pub pose_velocity: f32,
    /// Weight for foot contact state.
    pub contact: f32,
}

impl Default for MatchWeights {
    fn default() -> Self {
        Self {
            trajectory_position: 1.0,
            trajectory_direction: 1.5,
            pose_position: 1.0,
            pose_velocity: 0.8,
            contact: 0.5,
        }
    }
}

/// A single frame entry in the motion database.
#[derive(Clone)]
struct FrameEntry {
    /// Index of the motion clip in the database.
    clip_index: usize,
    /// Frame index within the clip.
    frame_index: usize,
    /// Pre-computed feature vector (flattened, weighted).
    features: Vec<f32>,
}

/// Configuration for trajectory sampling in the feature vector.
#[derive(Debug, Clone)]
pub struct TrajectoryFeatureConfig {
    /// Number of future trajectory samples.
    pub num_samples: usize,
    /// Time step between samples (seconds).
    pub sample_interval: f32,
}

impl Default for TrajectoryFeatureConfig {
    fn default() -> Self {
        Self {
            num_samples: 6,
            sample_interval: 0.1, // 6 samples * 0.1s = 0.6s lookahead
        }
    }
}

/// A loaded motion clip with metadata.
pub struct MotionClip {
    pub name: String,
    pub motion: Motion,
    /// Per-frame root positions (precomputed).
    root_positions: Vec<Vec3>,
    /// Per-frame root forward directions (precomputed).
    root_directions: Vec<Vec3>,
    /// Per-frame joint positions relative to root (precomputed).
    local_positions: Vec<Vec<Vec3>>,
    /// Per-frame joint velocities (precomputed).
    velocities: Vec<Vec<Vec3>>,
    /// Per-frame foot contact state [left, right] (precomputed).
    contacts: Vec<[bool; 2]>,
}

/// The motion matching database: holds all clips and their per-frame features.
pub struct MotionDatabase {
    /// All loaded motion clips.
    pub clips: Vec<MotionClip>,
    /// Flattened frame entries with pre-computed features.
    entries: Vec<FrameEntry>,
    /// Feature weights.
    pub weights: MatchWeights,
    /// Trajectory feature config.
    pub trajectory_config: TrajectoryFeatureConfig,
    /// Joints to include in pose matching (indices). Empty = all.
    pub pose_joints: Vec<usize>,
    /// Mean of each feature dimension (for normalization).
    feature_mean: Vec<f32>,
    /// Std deviation of each feature dimension (for normalization).
    feature_std: Vec<f32>,
    /// Whether the database has been built.
    pub built: bool,
}

impl MotionDatabase {
    pub fn new() -> Self {
        Self {
            clips: Vec::new(),
            entries: Vec::new(),
            weights: MatchWeights::default(),
            trajectory_config: TrajectoryFeatureConfig::default(),
            pose_joints: Vec::new(),
            feature_mean: Vec::new(),
            feature_std: Vec::new(),
            built: false,
        }
    }

    /// Add a motion clip to the database. Call `build()` after adding all clips.
    pub fn add_clip(&mut self, name: String, motion: Motion) {
        let num_frames = motion.num_frames();
        let num_joints = motion.num_joints();

        // Precompute per-frame data
        let mut root_positions = Vec::with_capacity(num_frames);
        let mut root_directions = Vec::with_capacity(num_frames);
        let mut local_positions = Vec::with_capacity(num_frames);
        let mut velocities = Vec::with_capacity(num_frames);
        let mut contacts = Vec::with_capacity(num_frames);

        let dt = motion.delta_time();

        for f in 0..num_frames {
            let t = f as f32 * dt;
            let transforms = motion.get_transforms(t, false);

            // Root position (joint 0)
            let root_pos = transforms[0].get_position();
            root_positions.push(root_pos);

            // Root forward direction (Z column of root rotation)
            let root_fwd = Vec3::new(
                transforms[0].z_axis.x,
                0.0,
                transforms[0].z_axis.z,
            ).normalize_or_zero();
            root_directions.push(root_fwd);

            // Joint positions relative to root
            let root_inv = Mat4::from_translation(-root_pos);
            let local_pos: Vec<Vec3> = transforms.iter()
                .map(|t| (root_inv * *t).get_position())
                .collect();
            local_positions.push(local_pos);

            // Velocities (finite difference)
            if f > 0 {
                let prev_t = (f as f32 - 1.0) * dt;
                let prev_transforms = motion.get_transforms(prev_t, false);
                let inv_dt = if dt > 0.0 { 1.0 / dt } else { 0.0 };
                let vel: Vec<Vec3> = (0..num_joints).map(|j| {
                    (transforms[j].get_position() - prev_transforms[j].get_position()) * inv_dt
                }).collect();
                velocities.push(vel);
            } else {
                velocities.push(vec![Vec3::ZERO; num_joints]);
            }

            // Simple foot contact detection (heel + toe if they exist, else last 2 joints)
            let left_foot = find_foot_joint(&motion, "left");
            let right_foot = find_foot_joint(&motion, "right");
            let lc = left_foot.map(|j| is_contact(&transforms, j, 0.05)).unwrap_or(false);
            let rc = right_foot.map(|j| is_contact(&transforms, j, 0.05)).unwrap_or(false);
            contacts.push([lc, rc]);
        }

        self.clips.push(MotionClip {
            name,
            motion,
            root_positions,
            root_directions,
            local_positions,
            velocities,
            contacts,
        });
        self.built = false;
    }

    /// Build the feature database. Must be called after all clips are added.
    pub fn build(&mut self) {
        self.entries.clear();

        let traj = &self.trajectory_config;
        let _weights = &self.weights;

        // First pass: compute raw features for all frames
        let mut raw_features: Vec<Vec<f32>> = Vec::new();
        let mut entry_metadata: Vec<(usize, usize)> = Vec::new();

        for (clip_idx, clip) in self.clips.iter().enumerate() {
            let num_frames = clip.motion.num_frames();
            let dt = clip.motion.delta_time();
            // Skip first/last frames to allow trajectory sampling
            let margin = (traj.num_samples as f32 * traj.sample_interval / dt).ceil() as usize + 1;
            let start = margin.min(num_frames);
            let end = num_frames.saturating_sub(margin);

            for f in start..end {
                let features = self.extract_features_for_frame(clip_idx, f);
                raw_features.push(features);
                entry_metadata.push((clip_idx, f));
            }
        }

        if raw_features.is_empty() {
            self.built = true;
            return;
        }

        // Compute feature normalization (mean + std)
        let dim = raw_features[0].len();
        let n = raw_features.len() as f32;
        let mut mean = vec![0.0f32; dim];
        let mut var = vec![0.0f32; dim];

        for feat in &raw_features {
            for (i, &v) in feat.iter().enumerate() {
                mean[i] += v;
            }
        }
        for m in &mut mean {
            *m /= n;
        }

        for feat in &raw_features {
            for (i, &v) in feat.iter().enumerate() {
                let d = v - mean[i];
                var[i] += d * d;
            }
        }
        let std: Vec<f32> = var.iter().map(|v| (v / n).sqrt().max(1e-6)).collect();

        // Normalize and apply weights, build entries
        for (idx, feat) in raw_features.iter().enumerate() {
            let normalized: Vec<f32> = feat.iter().enumerate()
                .map(|(i, &v)| (v - mean[i]) / std[i])
                .collect();

            let (clip_index, frame_index) = entry_metadata[idx];
            self.entries.push(FrameEntry {
                clip_index,
                frame_index,
                features: normalized,
            });
        }

        self.feature_mean = mean;
        self.feature_std = std;
        self.built = true;
    }

    /// Extract the raw feature vector for a specific frame.
    fn extract_features_for_frame(&self, clip_idx: usize, frame: usize) -> Vec<f32> {
        let clip = &self.clips[clip_idx];
        let dt = clip.motion.delta_time();
        let num_frames = clip.motion.num_frames();
        let weights = &self.weights;
        let traj = &self.trajectory_config;

        let mut features = Vec::with_capacity(128);

        // Current root (for relative computations)
        let root_pos = clip.root_positions[frame];
        let root_dir = clip.root_directions[frame];
        let root_right = Vec3::new(-root_dir.z, 0.0, root_dir.x);

        // ── Trajectory features (future path) ──────────────
        for s in 0..traj.num_samples {
            let future_t = (s + 1) as f32 * traj.sample_interval;
            let future_frame = (frame as f32 + future_t / dt).min((num_frames - 1) as f32) as usize;

            let fp = clip.root_positions[future_frame] - root_pos;
            // Project into root-relative space
            let local_x = fp.dot(root_right);
            let local_z = fp.dot(root_dir);
            features.push(local_x * weights.trajectory_position);
            features.push(local_z * weights.trajectory_position);

            let fd = clip.root_directions[future_frame];
            let dir_x = fd.dot(root_right);
            let dir_z = fd.dot(root_dir);
            features.push(dir_x * weights.trajectory_direction);
            features.push(dir_z * weights.trajectory_direction);
        }

        // ── Pose features (current joint positions relative to root) ──
        let pose_joints: Vec<usize> = if self.pose_joints.is_empty() {
            // Use a subset of important joints for efficiency
            let n = clip.motion.num_joints();
            if n <= 10 {
                (0..n).collect()
            } else {
                // Sample evenly + always include root
                let mut joints = vec![0];
                let step = n / 8;
                for i in (step..n).step_by(step) {
                    joints.push(i);
                }
                joints
            }
        } else {
            self.pose_joints.clone()
        };

        for &j in &pose_joints {
            if j < clip.local_positions[frame].len() {
                let p = clip.local_positions[frame][j];
                features.push(p.x * weights.pose_position);
                features.push(p.y * weights.pose_position);
                features.push(p.z * weights.pose_position);
            }
        }

        // ── Velocity features ──────────────────────────────
        for &j in &pose_joints {
            if j < clip.velocities[frame].len() {
                let v = clip.velocities[frame][j];
                features.push(v.x * weights.pose_velocity);
                features.push(v.y * weights.pose_velocity);
                features.push(v.z * weights.pose_velocity);
            }
        }

        // ── Contact features ───────────────────────────────
        let [lc, rc] = clip.contacts[frame];
        features.push(if lc { 1.0 } else { 0.0 } * weights.contact);
        features.push(if rc { 1.0 } else { 0.0 } * weights.contact);

        features
    }

    /// Query the database for the best matching frame.
    /// Returns (clip_index, frame_index, cost).
    pub fn query(&self, query_features: &[f32]) -> Option<(usize, usize, f32)> {
        if !self.built || self.entries.is_empty() {
            return None;
        }

        // Normalize the query
        let normalized: Vec<f32> = query_features.iter().enumerate()
            .map(|(i, &v)| {
                if i < self.feature_mean.len() {
                    (v - self.feature_mean[i]) / self.feature_std[i]
                } else {
                    v
                }
            })
            .collect();

        // Brute-force nearest neighbor (fast enough for <100k frames)
        let mut best_cost = f32::MAX;
        let mut best_entry: Option<&FrameEntry> = None;

        for entry in &self.entries {
            let cost = squared_distance(&normalized, &entry.features);
            if cost < best_cost {
                best_cost = cost;
                best_entry = Some(entry);
            }
        }

        best_entry.map(|e| (e.clip_index, e.frame_index, best_cost))
    }

    /// Query the database, excluding a window around the current playback.
    /// This prevents matching to frames very close to where we already are.
    pub fn query_excluding(
        &self,
        query_features: &[f32],
        exclude_clip: usize,
        exclude_frame: usize,
        exclude_window: usize,
    ) -> Option<(usize, usize, f32)> {
        if !self.built || self.entries.is_empty() {
            return None;
        }

        let normalized: Vec<f32> = query_features.iter().enumerate()
            .map(|(i, &v)| {
                if i < self.feature_mean.len() {
                    (v - self.feature_mean[i]) / self.feature_std[i]
                } else {
                    v
                }
            })
            .collect();

        let mut best_cost = f32::MAX;
        let mut best_entry: Option<&FrameEntry> = None;

        for entry in &self.entries {
            // Skip frames near current playback
            if entry.clip_index == exclude_clip {
                let diff = (entry.frame_index as i64 - exclude_frame as i64).unsigned_abs() as usize;
                if diff < exclude_window {
                    continue;
                }
            }
            let cost = squared_distance(&normalized, &entry.features);
            if cost < best_cost {
                best_cost = cost;
                best_entry = Some(entry);
            }
        }

        best_entry.map(|e| (e.clip_index, e.frame_index, best_cost))
    }

    /// Build a query feature vector from the current game state.
    pub fn build_query(
        &self,
        current_positions: &[Vec3],  // Current joint positions (world space)
        current_velocities: &[Vec3], // Current joint velocities
        root_position: Vec3,
        root_direction: Vec3,
        desired_trajectory: &[(Vec3, Vec3)], // (future_pos, future_dir) samples
        left_contact: bool,
        right_contact: bool,
    ) -> Vec<f32> {
        let weights = &self.weights;
        let mut features = Vec::with_capacity(128);

        let root_right = Vec3::new(-root_direction.z, 0.0, root_direction.x);

        // ── Trajectory ─────────────────────────────────────
        for (fp, fd) in desired_trajectory {
            let rel = *fp - root_position;
            features.push(rel.dot(root_right) * weights.trajectory_position);
            features.push(rel.dot(root_direction) * weights.trajectory_position);
            features.push(fd.dot(root_right) * weights.trajectory_direction);
            features.push(fd.dot(root_direction) * weights.trajectory_direction);
        }

        // Pad if fewer trajectory samples than expected
        let expected_traj = self.trajectory_config.num_samples * 4;
        while features.len() < expected_traj {
            features.push(0.0);
        }

        // ── Pose (root-relative) ───────────────────────────
        let pose_joints: Vec<usize> = if self.pose_joints.is_empty() {
            let n = current_positions.len();
            if n <= 10 {
                (0..n).collect()
            } else {
                let mut joints = vec![0];
                let step = n / 8;
                for i in (step..n).step_by(step) {
                    joints.push(i);
                }
                joints
            }
        } else {
            self.pose_joints.clone()
        };

        for &j in &pose_joints {
            if j < current_positions.len() {
                let p = current_positions[j] - root_position;
                features.push(p.x * weights.pose_position);
                features.push(p.y * weights.pose_position);
                features.push(p.z * weights.pose_position);
            }
        }

        // ── Velocities ─────────────────────────────────────
        for &j in &pose_joints {
            if j < current_velocities.len() {
                let v = current_velocities[j];
                features.push(v.x * weights.pose_velocity);
                features.push(v.y * weights.pose_velocity);
                features.push(v.z * weights.pose_velocity);
            }
        }

        // ── Contacts ───────────────────────────────────────
        features.push(if left_contact { 1.0 } else { 0.0 } * weights.contact);
        features.push(if right_contact { 1.0 } else { 0.0 } * weights.contact);

        features
    }

    /// Number of frames in the database.
    pub fn num_entries(&self) -> usize {
        self.entries.len()
    }

    /// Number of loaded clips.
    pub fn num_clips(&self) -> usize {
        self.clips.len()
    }

    /// Total frames across all clips.
    pub fn total_frames(&self) -> usize {
        self.clips.iter().map(|c| c.motion.num_frames()).sum()
    }
}

/// Motion matching controller: manages real-time playback from the database.
pub struct MotionMatchingController {
    /// Reference to the motion database.
    pub db_built: bool,
    /// Currently playing clip index.
    pub current_clip: usize,
    /// Current frame in the clip.
    pub current_frame: usize,
    /// Current timestamp within the clip.
    pub current_time: f32,
    /// How often to re-query the database (seconds).
    pub query_interval: f32,
    /// Time since last query.
    query_elapsed: f32,
    /// Minimum cost improvement to trigger a transition.
    pub transition_threshold: f32,
    /// Current cost of the active match.
    current_cost: f32,
    /// Crossfade transition for smooth blending.
    pub transition: AnimationTransition,
    /// Source pose for blending during transition.
    transition_source: Vec<Mat4>,
    /// Whether the controller is active.
    pub active: bool,
    /// Exclusion window (frames) around current position.
    pub exclusion_window: usize,
}

impl MotionMatchingController {
    pub fn new() -> Self {
        Self {
            db_built: false,
            current_clip: 0,
            current_frame: 0,
            current_time: 0.0,
            query_interval: 0.1, // Re-query every 100ms
            query_elapsed: 0.0,
            transition_threshold: 0.5,
            current_cost: f32::MAX,
            transition: AnimationTransition::new(0.2),
            transition_source: Vec::new(),
            active: false,
            exclusion_window: 15, // ~0.5s at 30fps
        }
    }

    /// Update the controller each frame.
    /// Returns the current blended pose if active, or None.
    pub fn update(
        &mut self,
        db: &MotionDatabase,
        dt: f32,
        query_features: &[f32],
    ) -> Option<Vec<Mat4>> {
        if !self.active || !db.built || db.clips.is_empty() {
            return None;
        }

        // Advance time
        self.current_time += dt;
        self.query_elapsed += dt;

        let clip = &db.clips[self.current_clip];
        let clip_dt = clip.motion.delta_time();
        self.current_frame = (self.current_time / clip_dt).round() as usize;

        // Wrap or clamp
        let total = clip.motion.num_frames();
        if self.current_frame >= total {
            self.current_frame = 0;
            self.current_time = 0.0;
        }

        // Re-query the database periodically
        if self.query_elapsed >= self.query_interval {
            self.query_elapsed = 0.0;

            if let Some((best_clip, best_frame, cost)) = db.query_excluding(
                query_features,
                self.current_clip,
                self.current_frame,
                self.exclusion_window,
            ) {
                // Only transition if significantly better
                let improvement = self.current_cost - cost;
                if improvement > self.transition_threshold || self.current_cost == f32::MAX {
                    // Save current pose for crossfade
                    let current_t = self.current_frame as f32 * clip_dt;
                    self.transition_source = clip.motion.get_transforms(current_t, false);
                    self.transition.start(0.0, 0.0);

                    self.current_clip = best_clip;
                    self.current_frame = best_frame;
                    self.current_time = best_frame as f32 * db.clips[best_clip].motion.delta_time();
                    self.current_cost = cost;
                }
            }
        }

        // Update transition
        self.transition.update(dt);

        // Get current pose
        let clip = &db.clips[self.current_clip];
        let t = self.current_frame as f32 * clip.motion.delta_time();
        let current_pose = clip.motion.get_transforms_interpolated(t, false);

        // Blend with transition source if crossfading
        if self.transition.is_active() && !self.transition_source.is_empty() {
            let w = self.transition.weight();
            Some(crate::blend::blend_poses(&self.transition_source, &current_pose, w))
        } else {
            Some(current_pose)
        }
    }

    /// Force a transition to a specific clip and frame.
    pub fn goto(&mut self, db: &MotionDatabase, clip_index: usize, frame: usize) {
        if clip_index < db.clips.len() {
            let clip = &db.clips[self.current_clip];
            let current_t = self.current_frame as f32 * clip.motion.delta_time();
            self.transition_source = clip.motion.get_transforms(current_t, false);
            self.transition.start(0.0, 0.0);

            self.current_clip = clip_index;
            self.current_frame = frame;
            self.current_time = frame as f32 * db.clips[clip_index].motion.delta_time();
        }
    }

    /// Get info about current playback state.
    pub fn status(&self) -> (usize, usize, f32) {
        (self.current_clip, self.current_frame, self.current_cost)
    }
}

// ── Helpers ────────────────────────────────────────────────

/// Squared Euclidean distance between two feature vectors.
fn squared_distance(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let mut sum = 0.0f32;
    for i in 0..n {
        let d = a[i] - b[i];
        sum += d * d;
    }
    sum
}

/// Find a foot joint by looking for common naming patterns.
fn find_foot_joint(motion: &Motion, side: &str) -> Option<usize> {
    let names = &motion.hierarchy.bone_names;
    let side_lower = side.to_lowercase();

    // Try common patterns
    for (i, name) in names.iter().enumerate() {
        let lower: String = name.to_lowercase();
        if lower.contains(&side_lower) && (lower.contains("foot") || lower.contains("ankle") || lower.contains("toe")) {
            return Some(i);
        }
    }
    None
}

/// Simple contact detection: joint Y position below threshold.
fn is_contact(transforms: &[Mat4], joint_index: usize, height_threshold: f32) -> bool {
    if joint_index < transforms.len() {
        let pos = transforms[joint_index].get_position();
        pos.y < height_threshold
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_squared_distance() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((squared_distance(&a, &b)).abs() < 1e-6);

        let c = vec![2.0, 3.0, 4.0];
        assert!((squared_distance(&a, &c) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_match_weights_default() {
        let w = MatchWeights::default();
        assert!(w.trajectory_position > 0.0);
        assert!(w.pose_position > 0.0);
    }

    #[test]
    fn test_empty_database() {
        let db = MotionDatabase::new();
        assert!(db.query(&[0.0; 10]).is_none());
    }

    #[test]
    fn test_controller_inactive() {
        let mut ctrl = MotionMatchingController::new();
        let db = MotionDatabase::new();
        assert!(ctrl.update(&db, 0.016, &[]).is_none());
    }
}
