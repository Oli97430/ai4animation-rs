//! Motion data: frames of skeletal transformations.

use glam::{Mat4, Vec3};
use anim_math::transform::{Transform, MirrorAxis};
use crate::hierarchy::Hierarchy;
use anim_import::mesh::ImportedModel;

/// A motion clip holding per-frame, per-joint 4x4 global transforms.
#[derive(Clone)]
pub struct Motion {
    pub name: String,
    pub hierarchy: Hierarchy,
    /// [num_frames][num_joints] of Mat4 in global space.
    pub frames: Vec<Vec<Mat4>>,
    pub framerate: f32,
    pub mirror_axis: MirrorAxis,
    pub symmetry: Vec<usize>,
}

impl Motion {
    /// Create from an imported model.
    pub fn from_imported(model: &ImportedModel) -> Option<Self> {
        let anim = model.animation_frames.as_ref()?;
        let hierarchy = Hierarchy::new(
            model.joint_names.clone(),
            model.parent_indices.clone(),
        );
        let symmetry = hierarchy.detect_symmetry();

        Some(Self {
            name: model.name.clone(),
            hierarchy,
            frames: anim.frames.clone(),
            framerate: anim.framerate,
            mirror_axis: MirrorAxis::Z,
            symmetry,
        })
    }

    /// Create from raw animation data (for procedural generation).
    pub fn from_animation_data(
        joint_names: &[String],
        parent_indices: &[i32],
        frames: &[Vec<Mat4>],
        framerate: f32,
    ) -> Self {
        let hierarchy = Hierarchy::new(
            joint_names.to_vec(),
            parent_indices.to_vec(),
        );
        let symmetry = hierarchy.detect_symmetry();
        Self {
            name: "Procedural".into(),
            hierarchy,
            frames: frames.to_vec(),
            framerate,
            mirror_axis: MirrorAxis::Z,
            symmetry,
        }
    }

    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }

    pub fn num_joints(&self) -> usize {
        self.hierarchy.num_joints()
    }

    /// Effective framerate, defaulting to 30.0 if the stored value is invalid.
    fn effective_framerate(&self) -> f32 {
        if self.framerate > 0.0 { self.framerate } else { 30.0 }
    }

    pub fn delta_time(&self) -> f32 {
        1.0 / self.effective_framerate()
    }

    pub fn total_time(&self) -> f32 {
        if self.frames.is_empty() { 0.0 }
        else { (self.frames.len() - 1) as f32 / self.effective_framerate() }
    }

    /// Get the frame index for a given timestamp (clamped).
    pub fn frame_index(&self, timestamp: f32) -> usize {
        let fps = self.effective_framerate();
        let idx = (timestamp * fps).round() as i64;
        idx.clamp(0, (self.frames.len() as i64) - 1) as usize
    }

    /// Get bone transforms at a given timestamp.
    pub fn get_transforms(&self, timestamp: f32, mirrored: bool) -> Vec<Mat4> {
        let idx = self.frame_index(timestamp);
        let frame = &self.frames[idx];

        if mirrored {
            let mut mirrored_frame = vec![Mat4::IDENTITY; frame.len()];
            for (i, &sym_idx) in self.symmetry.iter().enumerate() {
                mirrored_frame[i] = frame[sym_idx].get_mirror(self.mirror_axis);
            }
            mirrored_frame
        } else {
            frame.clone()
        }
    }

    /// Get positions at a given timestamp.
    pub fn get_positions(&self, timestamp: f32, mirrored: bool) -> Vec<Vec3> {
        self.get_transforms(timestamp, mirrored)
            .iter()
            .map(|t| t.get_position())
            .collect()
    }

    /// Get velocities (numerical differentiation).
    pub fn get_velocities(&self, timestamp: f32, mirrored: bool) -> Vec<Vec3> {
        let dt = self.delta_time();
        let curr = self.get_positions(timestamp, mirrored);
        let prev = self.get_positions((timestamp - dt).max(0.0), mirrored);
        let inv_dt = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        curr.iter().zip(prev.iter())
            .map(|(c, p)| (*c - *p) * inv_dt)
            .collect()
    }

    /// Set a single joint's transform at a specific frame (for auto-key).
    pub fn set_joint_transform(&mut self, frame_idx: usize, joint_idx: usize, transform: Mat4) {
        if frame_idx < self.frames.len() && joint_idx < self.frames[frame_idx].len() {
            self.frames[frame_idx][joint_idx] = transform;
        }
    }

    /// Set all joint transforms at a specific frame.
    pub fn set_frame(&mut self, frame_idx: usize, transforms: &[Mat4]) {
        if frame_idx < self.frames.len() {
            let n = self.frames[frame_idx].len().min(transforms.len());
            self.frames[frame_idx][..n].copy_from_slice(&transforms[..n]);
        }
    }

    /// Get interpolated transforms between two frames.
    pub fn get_transforms_interpolated(&self, timestamp: f32, mirrored: bool) -> Vec<Mat4> {
        let continuous = timestamp * self.effective_framerate();
        let idx_a = (continuous.floor() as usize).min(self.frames.len().saturating_sub(1));
        let idx_b = (idx_a + 1).min(self.frames.len().saturating_sub(1));
        let t = continuous - continuous.floor();

        if idx_a == idx_b || t < 0.001 {
            return self.get_transforms(timestamp, mirrored);
        }

        let frame_a = &self.frames[idx_a];
        let frame_b = &self.frames[idx_b];
        let mut result = vec![Mat4::IDENTITY; frame_a.len()];

        for i in 0..frame_a.len() {
            result[i] = frame_a[i].interpolate(&frame_b[i], t);
        }

        if mirrored {
            let mut mirrored = vec![Mat4::IDENTITY; result.len()];
            for (i, &sym_idx) in self.symmetry.iter().enumerate() {
                mirrored[i] = result[sym_idx].get_mirror(self.mirror_axis);
            }
            mirrored
        } else {
            result
        }
    }
}
