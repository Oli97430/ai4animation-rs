//! Actor: skeletal character component (mirrors Python Actor.py).
//!
//! An Actor owns a skeleton hierarchy, current-frame transforms, velocities,
//! bone lengths, and provides FK propagation + bone lookup + alignment utilities.

use glam::{Mat4, Vec3, Quat};
use anim_math::transform::Transform;
use crate::hierarchy::Hierarchy;

/// A single bone in the skeleton.
#[derive(Debug, Clone)]
pub struct Bone {
    pub index: usize,
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// All descendant bone indices (for FK propagation).
    pub successors: Vec<usize>,
    /// Rest-pose (bind-pose) transform.
    pub zero_transform: Mat4,
    /// Default bone length (distance to parent in rest pose).
    pub default_length: f32,
}

/// Skeletal character: hierarchy + per-bone state.
///
/// Mirrors Python's `Actor` class. Holds the current pose (transforms, velocities)
/// and provides FK, alignment, and bone-length preservation utilities.
pub struct Actor {
    pub hierarchy: Hierarchy,
    pub bones: Vec<Bone>,
    /// Current global transforms (one per joint).
    pub transforms: Vec<Mat4>,
    /// Current joint velocities (world space).
    pub velocities: Vec<Vec3>,
    /// Root transform (separate from skeleton).
    pub root: Mat4,
}

impl Actor {
    /// Create an Actor from a hierarchy and rest-pose transforms.
    pub fn new(hierarchy: Hierarchy, rest_pose: &[Mat4]) -> Self {
        let n = hierarchy.num_joints();
        assert_eq!(rest_pose.len(), n, "rest_pose length must match hierarchy joint count");

        let positions: Vec<Vec3> = rest_pose.iter().map(|t| t.get_position()).collect();

        // Build bones with hierarchy relationships
        let mut bones: Vec<Bone> = Vec::with_capacity(n);
        for i in 0..n {
            let parent_idx = hierarchy.get_parent_index(i);
            let parent = if parent_idx >= 0 { Some(parent_idx as usize) } else { None };

            let default_length = if let Some(p) = parent {
                (positions[i] - positions[p]).length()
            } else {
                0.0
            };

            bones.push(Bone {
                index: i,
                name: hierarchy.bone_names[i].clone(),
                parent,
                children: Vec::new(),
                successors: Vec::new(),
                zero_transform: rest_pose[i],
                default_length,
            });
        }

        // Fill children
        for i in 0..n {
            if let Some(p) = bones[i].parent {
                // Clone-free: we know p < i for valid hierarchies
                let child_idx = i;
                bones[p].children.push(child_idx);
            }
        }

        // Compute successors (all descendants) via DFS
        for i in (0..n).rev() {
            // Each bone's successors = its children + their successors
            let mut succs = Vec::new();
            for &c in &bones[i].children {
                succs.push(c);
                succs.extend_from_slice(&bones[c].successors);
            }
            bones[i].successors = succs;
        }

        Self {
            hierarchy,
            bones,
            transforms: rest_pose.to_vec(),
            velocities: vec![Vec3::ZERO; n],
            root: Mat4::IDENTITY,
        }
    }

    /// Create from an imported model (uses first frame as rest pose).
    pub fn from_imported(model: &anim_import::ImportedModel) -> Self {
        let hierarchy = Hierarchy::new(
            model.joint_names.clone(),
            model.parent_indices.clone(),
        );

        let rest_pose = if let Some(ref anim) = model.animation_frames {
            if !anim.frames.is_empty() {
                anim.frames[0].clone()
            } else {
                vec![Mat4::IDENTITY; model.joint_names.len()]
            }
        } else {
            vec![Mat4::IDENTITY; model.joint_names.len()]
        };

        Self::new(hierarchy, &rest_pose)
    }

    // ── Queries ────────────────────────────────────────────

    pub fn num_bones(&self) -> usize {
        self.bones.len()
    }

    /// Get bone by index.
    pub fn get_bone(&self, index: usize) -> Option<&Bone> {
        self.bones.get(index)
    }

    /// Get bone by name.
    pub fn get_bone_by_name(&self, name: &str) -> Option<&Bone> {
        self.hierarchy.get_bone_index(name).map(|i| &self.bones[i])
    }

    /// Get bone index by name.
    pub fn bone_index(&self, name: &str) -> Option<usize> {
        self.hierarchy.get_bone_index(name)
    }

    /// Find bones matching a pattern (substring search).
    pub fn find_bones(&self, pattern: &str) -> Vec<usize> {
        let pattern_lower = pattern.to_lowercase();
        self.bones.iter()
            .filter(|b| b.name.to_lowercase().contains(&pattern_lower))
            .map(|b| b.index)
            .collect()
    }

    // ── Transform access ──────────────────────────────────

    /// Get current global transform of a bone.
    pub fn get_transform(&self, bone_idx: usize) -> Mat4 {
        self.transforms[bone_idx]
    }

    /// Get current position of a bone.
    pub fn get_position(&self, bone_idx: usize) -> Vec3 {
        self.transforms[bone_idx].get_position()
    }

    /// Get current rotation of a bone as a quaternion.
    pub fn get_rotation(&self, bone_idx: usize) -> Quat {
        let mat3 = self.transforms[bone_idx].get_rotation();
        Quat::from_mat3(&mat3)
    }

    /// Get velocity of a bone.
    pub fn get_velocity(&self, bone_idx: usize) -> Vec3 {
        self.velocities[bone_idx]
    }

    /// Get all positions as a vec.
    pub fn positions(&self) -> Vec<Vec3> {
        self.transforms.iter().map(|t| t.get_position()).collect()
    }

    // ── Transform modification ────────────────────────────

    /// Set the global transform of a bone, optionally propagating FK to successors.
    pub fn set_transform(&mut self, bone_idx: usize, transform: Mat4, fk: bool) {
        let old = self.transforms[bone_idx];
        self.transforms[bone_idx] = transform;

        if fk {
            let delta = transform * old.inverse();
            // Apply delta to all successors
            for &succ in &self.bones[bone_idx].successors {
                self.transforms[succ] = delta * self.transforms[succ];
            }
        }
    }

    /// Set position of a bone with FK propagation.
    pub fn set_position(&mut self, bone_idx: usize, pos: Vec3, fk: bool) {
        let mut t = self.transforms[bone_idx];
        t.set_position(pos);
        self.set_transform(bone_idx, t, fk);
    }

    /// Set rotation of a bone with FK propagation.
    pub fn set_rotation(&mut self, bone_idx: usize, rotation: Quat, fk: bool) {
        let pos = self.get_position(bone_idx);
        let new_t = Mat4::from_rotation_translation(rotation, pos);
        self.set_transform(bone_idx, new_t, fk);
    }

    /// Set all transforms from a frame of animation data.
    pub fn set_pose(&mut self, transforms: &[Mat4]) {
        let n = transforms.len().min(self.transforms.len());
        self.transforms[..n].copy_from_slice(&transforms[..n]);
    }

    /// Set all transforms and compute velocities from previous pose.
    pub fn set_pose_with_velocities(&mut self, transforms: &[Mat4], dt: f32) {
        let old_positions = self.positions();
        self.set_pose(transforms);
        let new_positions = self.positions();

        let inv_dt = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        for i in 0..self.velocities.len().min(new_positions.len()) {
            self.velocities[i] = (new_positions[i] - old_positions[i]) * inv_dt;
        }
    }

    // ── Bone length preservation ──────────────────────────

    /// Restore default bone lengths after IK or manual editing.
    /// Adjusts positions to maintain original distances between parent-child pairs.
    pub fn restore_bone_lengths(&mut self) {
        for i in 0..self.bones.len() {
            if let Some(parent) = self.bones[i].parent {
                let parent_pos = self.get_position(parent);
                let child_pos = self.get_position(i);
                let dir = (child_pos - parent_pos).normalize_or_zero();
                let target_len = self.bones[i].default_length;

                if dir.length_squared() > 0.0001 && target_len > 0.0001 {
                    let corrected_pos = parent_pos + dir * target_len;
                    self.set_position(i, corrected_pos, false);
                }
            }
        }
    }

    /// Get the length of bone i (distance to parent in current pose).
    pub fn bone_length(&self, bone_idx: usize) -> f32 {
        if let Some(parent) = self.bones[bone_idx].parent {
            (self.get_position(bone_idx) - self.get_position(parent)).length()
        } else {
            0.0
        }
    }

    // ── Alignment utilities ───────────────────────────────

    /// Align the actor's root to face a target direction on the ground plane (XZ).
    pub fn align_root_to_direction(&mut self, forward: Vec3) {
        let flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        if flat.length_squared() > 0.0001 {
            let angle = flat.z.atan2(flat.x);
            self.root = Mat4::from_rotation_y(-angle);
        }
    }

    /// Get the forward direction of the root (local Z axis projected onto ground).
    pub fn root_forward(&self) -> Vec3 {
        let z = self.root.z_axis.truncate().normalize_or_zero();
        Vec3::new(z.x, 0.0, z.z).normalize_or_zero()
    }

    /// Get the root position.
    pub fn root_position(&self) -> Vec3 {
        self.root.get_position()
    }

    // ── IK chain extraction ───────────────────────────────

    /// Get the chain of bone indices from `root_bone` up to (and including) `tip_bone`.
    /// Returns empty if tip is not a descendant of root.
    pub fn get_chain(&self, root_bone: usize, tip_bone: usize) -> Vec<usize> {
        let mut chain = vec![tip_bone];
        let mut current = tip_bone;
        while current != root_bone {
            if let Some(parent) = self.bones[current].parent {
                chain.push(parent);
                current = parent;
            } else {
                return Vec::new(); // tip is not a descendant of root
            }
        }
        chain.reverse();
        chain
    }

    /// Extract transforms for a bone chain (for IK solver input).
    pub fn chain_transforms(&self, chain: &[usize]) -> Vec<Mat4> {
        chain.iter().map(|&i| self.transforms[i]).collect()
    }

    /// Apply IK-solved positions back to the actor's bone chain.
    pub fn apply_ik_positions(&mut self, chain: &[usize], positions: &[Vec3]) {
        for (ci, &bone_idx) in chain.iter().enumerate() {
            if ci < positions.len() {
                let mut t = self.transforms[bone_idx];
                t.set_position(positions[ci]);
                self.transforms[bone_idx] = t;
            }
        }
    }

    /// Apply IK-solved rotations to the actor's bone chain.
    pub fn apply_ik_rotations(&mut self, chain: &[usize], rotations: &[Quat]) {
        for (ci, &bone_idx) in chain.iter().enumerate() {
            if ci < rotations.len() {
                let current_rot = self.get_rotation(bone_idx);
                let new_rot = rotations[ci] * current_rot;
                self.set_rotation(bone_idx, new_rot, false);
            }
        }
    }

    // ── Reset ─────────────────────────────────────────────

    /// Reset to rest (bind) pose.
    pub fn reset_to_rest_pose(&mut self) {
        for i in 0..self.bones.len() {
            self.transforms[i] = self.bones[i].zero_transform;
        }
        self.velocities.fill(Vec3::ZERO);
        self.root = Mat4::IDENTITY;
    }

    /// Get rest-pose transforms.
    pub fn rest_pose(&self) -> Vec<Mat4> {
        self.bones.iter().map(|b| b.zero_transform).collect()
    }
}
