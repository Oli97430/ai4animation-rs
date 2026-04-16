//! FABRIK (Forward And Backward Reaching Inverse Kinematics) solver
//! with joint angle limits and pole target support.

pub mod leg_ik;

use glam::{Mat4, Vec3, Quat};
use anim_math::transform::Transform;

pub use leg_ik::LegIk;

/// Per-joint angle constraint.
#[derive(Debug, Clone, Copy)]
pub struct JointConstraint {
    /// Maximum bend angle in radians from the reference direction.
    pub max_angle: f32,
    /// Minimum bend angle in radians (0 = fully straight allowed).
    pub min_angle: f32,
    /// Whether this constraint is active.
    pub enabled: bool,
}

impl JointConstraint {
    pub fn new(min_angle: f32, max_angle: f32) -> Self {
        Self { max_angle, min_angle, enabled: true }
    }

    /// Typical human knee: bends 0..~150 degrees backward.
    pub fn knee() -> Self {
        Self::new(0.0, std::f32::consts::FRAC_PI_2 * 1.67) // ~150 deg
    }

    /// Typical human elbow: bends 0..~145 degrees.
    pub fn elbow() -> Self {
        Self::new(0.0, std::f32::consts::FRAC_PI_2 * 1.6) // ~145 deg
    }

    /// Shoulder: wide range of motion.
    pub fn shoulder() -> Self {
        Self::new(0.0, std::f32::consts::PI * 0.95)
    }

    /// No constraint (full freedom).
    pub fn free() -> Self {
        Self { max_angle: std::f32::consts::PI, min_angle: 0.0, enabled: false }
    }
}

impl Default for JointConstraint {
    fn default() -> Self {
        Self::free()
    }
}

/// Pole target to control the plane of a joint chain (e.g., knee/elbow direction).
#[derive(Debug, Clone, Copy)]
pub struct PoleTarget {
    /// World-space position of the pole target.
    pub position: Vec3,
    /// How strongly the pole target influences the chain [0..1].
    pub weight: f32,
}

impl PoleTarget {
    pub fn new(position: Vec3, weight: f32) -> Self {
        Self { position, weight: weight.clamp(0.0, 1.0) }
    }
}

/// FABRIK solver for a chain of bones with optional constraints.
pub struct FabrikSolver {
    /// Bone positions in the chain (from root to tip).
    pub positions: Vec<Vec3>,
    /// Segment lengths between consecutive bones.
    pub lengths: Vec<f32>,
    /// Maximum solver iterations.
    pub max_iterations: usize,
    /// Convergence threshold (distance squared).
    pub threshold: f32,
    /// Per-joint angle constraints (same length as positions, or empty = unconstrained).
    pub constraints: Vec<JointConstraint>,
    /// Optional pole target for controlling the bend plane.
    pub pole_target: Option<PoleTarget>,
}

impl FabrikSolver {
    /// Create a new solver from a chain of positions.
    pub fn new(positions: Vec<Vec3>) -> Self {
        let lengths: Vec<f32> = positions.windows(2)
            .map(|w| (w[1] - w[0]).length())
            .collect();

        Self {
            positions,
            lengths,
            max_iterations: 10,
            threshold: 0.001,
            constraints: Vec::new(),
            pole_target: None,
        }
    }

    /// Create from bone transforms (extracts positions).
    pub fn from_transforms(transforms: &[Mat4]) -> Self {
        let positions: Vec<Vec3> = transforms.iter().map(|t| t.get_position()).collect();
        Self::new(positions)
    }

    /// Set joint constraints for the chain.
    pub fn with_constraints(mut self, constraints: Vec<JointConstraint>) -> Self {
        self.constraints = constraints;
        self
    }

    /// Set a pole target for the chain.
    pub fn with_pole_target(mut self, pole: PoleTarget) -> Self {
        self.pole_target = Some(pole);
        self
    }

    /// Solve IK to reach the target position.
    /// Returns true if converged within threshold.
    pub fn solve(&mut self, target: Vec3) -> bool {
        let root = self.positions[0];
        let n = self.positions.len();

        for _ in 0..self.max_iterations {
            // Check convergence
            let dist_sq = (self.positions[n - 1] - target).length_squared();
            if dist_sq < self.threshold * self.threshold {
                return true;
            }

            // Backward pass: move end effector to target, pull chain backward
            self.positions[n - 1] = target;
            for i in (0..n - 1).rev() {
                let dir = (self.positions[i] - self.positions[i + 1]).normalize_or_zero();
                self.positions[i] = self.positions[i + 1] + dir * self.lengths[i];
            }

            // Forward pass: restore root position, push chain forward
            self.positions[0] = root;
            for i in 1..n {
                let dir = (self.positions[i] - self.positions[i - 1]).normalize_or_zero();
                self.positions[i] = self.positions[i - 1] + dir * self.lengths[i - 1];

                // Apply angle constraints
                self.apply_constraint(i);
            }

            // Apply pole target after each iteration
            self.apply_pole_target();
        }

        let dist_sq = (self.positions[n - 1] - target).length_squared();
        dist_sq < self.threshold * self.threshold
    }

    /// Apply angle constraint to joint `i` (between parent, joint, child directions).
    fn apply_constraint(&mut self, i: usize) {
        if i == 0 || i >= self.positions.len() - 1 { return; }
        if i >= self.constraints.len() { return; }
        let constraint = &self.constraints[i];
        if !constraint.enabled { return; }

        let parent_pos = self.positions[i - 1];
        let joint_pos = self.positions[i];
        let child_pos = self.positions[i + 1];

        let to_parent = (parent_pos - joint_pos).normalize_or_zero();
        let to_child = (child_pos - joint_pos).normalize_or_zero();

        if to_parent.length_squared() < 0.0001 || to_child.length_squared() < 0.0001 {
            return;
        }

        let angle = to_parent.dot(to_child).clamp(-1.0, 1.0).acos();

        // Clamp angle to constraint range
        let clamped = angle.clamp(constraint.min_angle, constraint.max_angle);
        if (clamped - angle).abs() < 0.001 { return; }

        // Rotate the child position around the joint to satisfy the constraint
        let axis = to_parent.cross(to_child);
        if axis.length_squared() < 0.00001 { return; }
        let axis = axis.normalize();

        let delta_angle = clamped - angle;
        let rotation = Quat::from_axis_angle(axis, delta_angle);
        let rotated_dir = rotation * to_child;
        let length = self.lengths.get(i).copied().unwrap_or(
            (child_pos - joint_pos).length()
        );
        self.positions[i + 1] = joint_pos + rotated_dir * length;
    }

    /// Apply pole target: rotate the chain around the root-tip axis
    /// so that the middle joints move toward the pole target.
    fn apply_pole_target(&mut self) {
        let pole = match self.pole_target {
            Some(ref p) if p.weight > 0.001 => p,
            _ => return,
        };

        let n = self.positions.len();
        if n < 3 { return; }

        let root = self.positions[0];
        let tip = self.positions[n - 1];
        let chain_axis = (tip - root).normalize_or_zero();
        if chain_axis.length_squared() < 0.0001 { return; }

        // For each interior joint, project onto the plane perpendicular to chain_axis,
        // then rotate toward the pole target projection.
        for i in 1..n - 1 {
            let joint = self.positions[i];

            // Project joint and pole onto the plane at root, normal = chain_axis
            let joint_rel = joint - root;
            let pole_rel = pole.position - root;

            let joint_proj = joint_rel - chain_axis * joint_rel.dot(chain_axis);
            let pole_proj = pole_rel - chain_axis * pole_rel.dot(chain_axis);

            if joint_proj.length_squared() < 0.0001 || pole_proj.length_squared() < 0.0001 {
                continue;
            }

            let joint_dir = joint_proj.normalize();
            let pole_dir = pole_proj.normalize();

            let dot = joint_dir.dot(pole_dir).clamp(-1.0, 1.0);
            let angle = dot.acos() * pole.weight;

            if angle.abs() < 0.001 { continue; }

            // Determine rotation sign via cross product
            let cross = joint_dir.cross(pole_dir);
            let sign = if cross.dot(chain_axis) >= 0.0 { 1.0 } else { -1.0 };

            let rotation = Quat::from_axis_angle(chain_axis, angle * sign);
            let new_rel = rotation * joint_rel;
            self.positions[i] = root + new_rel;
        }
    }

    /// Get solved positions.
    pub fn get_positions(&self) -> &[Vec3] {
        &self.positions
    }

    /// Compute rotations that align bones to solved positions.
    pub fn compute_rotations(&self, original_transforms: &[Mat4]) -> Vec<Quat> {
        let n = self.positions.len();
        let mut rotations = vec![Quat::IDENTITY; n];

        for i in 0..n - 1 {
            let original_dir = (original_transforms[i + 1].get_position()
                - original_transforms[i].get_position()).normalize_or_zero();
            let solved_dir = (self.positions[i + 1] - self.positions[i]).normalize_or_zero();

            if original_dir.length_squared() > 0.0001 && solved_dir.length_squared() > 0.0001 {
                rotations[i] = Quat::from_rotation_arc(original_dir, solved_dir);
            }
        }

        rotations
    }
}
