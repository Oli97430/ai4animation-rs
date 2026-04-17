//! Animation constraints -- parent, aim, path follow, copy transforms.
//!
//! Constraints modify joint transforms after animation evaluation,
//! enabling procedural behaviors like eyes tracking a target,
//! objects following a spline path, or limbs pinned to world positions.

use glam::{Mat4, Quat, Vec3};

// ---------------------------------------------------------------------------
// Constraint enum
// ---------------------------------------------------------------------------

/// Constraint type.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Parent constraint: follow another joint's transform with offset.
    Parent {
        /// Index of the parent joint to follow.
        parent_joint: usize,
        /// Local offset from parent.
        offset: Mat4,
        /// Influence weight \[0..1\].
        weight: f32,
    },

    /// Aim/Look-at constraint: rotate to face a target.
    Aim {
        /// World-space target position.
        target: Vec3,
        /// Up vector for orientation.
        up: Vec3,
        /// Which local axis points at target: 0=X, 1=Y, 2=Z
        aim_axis: u8,
        /// Influence weight \[0..1\].
        weight: f32,
    },

    /// Copy position from another joint.
    CopyPosition {
        source_joint: usize,
        /// Which axes to copy: \[x, y, z\]
        axes: [bool; 3],
        weight: f32,
    },

    /// Copy rotation from another joint.
    CopyRotation {
        source_joint: usize,
        axes: [bool; 3],
        weight: f32,
    },

    /// Pin to world position (IK-like but simpler).
    PinToWorld {
        target: Vec3,
        weight: f32,
    },

    /// Follow a path (spline).
    FollowPath {
        /// Path to follow.
        path: SplinePath,
        /// Progress along path \[0..1\].
        progress: f32,
        /// Whether to orient along path tangent.
        orient_to_path: bool,
        /// Up vector for path orientation.
        up: Vec3,
        weight: f32,
    },
}

// ---------------------------------------------------------------------------
// SplinePath
// ---------------------------------------------------------------------------

/// A 3D spline path defined by control points.
#[derive(Debug, Clone)]
pub struct SplinePath {
    pub name: String,
    pub points: Vec<Vec3>,
    pub closed: bool,
}

impl SplinePath {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            points: Vec::new(),
            closed: false,
        }
    }

    pub fn add_point(&mut self, p: Vec3) {
        self.points.push(p);
    }

    /// Evaluate position on the path at parameter `t` \[0..1\].
    /// Uses Catmull-Rom interpolation between control points.
    pub fn evaluate(&self, t: f32) -> Vec3 {
        let n = self.points.len();
        match n {
            0 => Vec3::ZERO,
            1 => self.points[0],
            2 => self.points[0].lerp(self.points[1], t.clamp(0.0, 1.0)),
            _ => {
                let t = t.clamp(0.0, 1.0);
                let segments = if self.closed { n } else { n - 1 };
                let raw = t * segments as f32;
                let seg = (raw.floor() as usize).min(segments - 1);
                let local_t = raw - seg as f32;

                // Catmull-Rom needs four points: p0, p1, p2, p3
                // The curve goes from p1 to p2.
                let idx = |i: isize| -> Vec3 {
                    if self.closed {
                        self.points[((i % n as isize) + n as isize) as usize % n]
                    } else {
                        self.points[(i.max(0) as usize).min(n - 1)]
                    }
                };

                let s = seg as isize;
                let p0 = idx(s - 1);
                let p1 = idx(s);
                let p2 = idx(s + 1);
                let p3 = idx(s + 2);

                catmull_rom(p0, p1, p2, p3, local_t)
            }
        }
    }

    /// Evaluate tangent (derivative) at parameter `t`.
    pub fn tangent(&self, t: f32) -> Vec3 {
        let dt = 0.001_f32;
        let a = self.evaluate((t - dt).max(0.0));
        let b = self.evaluate((t + dt).min(1.0));
        let diff = b - a;
        if diff.length_squared() < 1e-12 {
            Vec3::X
        } else {
            diff.normalize()
        }
    }

    /// Total approximate length of the path by sampling.
    pub fn length(&self) -> f32 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let steps = (self.points.len() * 16).max(64);
        let mut total = 0.0_f32;
        let mut prev = self.evaluate(0.0);
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let cur = self.evaluate(t);
            total += (cur - prev).length();
            prev = cur;
        }
        total
    }
}

/// Catmull-Rom spline interpolation between p1 and p2,
/// using p0 and p3 as neighbouring control points. `t` in \[0..1\].
fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

// ---------------------------------------------------------------------------
// JointConstraint / ConstraintStack
// ---------------------------------------------------------------------------

/// A constraint applied to a specific joint.
#[derive(Debug, Clone)]
pub struct JointConstraint {
    pub name: String,
    pub joint_index: usize,
    pub constraint: Constraint,
    pub enabled: bool,
    /// Evaluation order (lower = first).
    pub order: i32,
}

/// Collection of constraints that can be applied to a pose.
#[derive(Debug, Clone)]
pub struct ConstraintStack {
    pub constraints: Vec<JointConstraint>,
}

impl ConstraintStack {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    /// Add a constraint and return its index.
    pub fn add(&mut self, c: JointConstraint) -> usize {
        let idx = self.constraints.len();
        self.constraints.push(c);
        idx
    }

    /// Remove a constraint by index.
    pub fn remove(&mut self, index: usize) {
        if index < self.constraints.len() {
            self.constraints.remove(index);
        }
    }

    /// Enable or disable a constraint by index.
    pub fn set_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(c) = self.constraints.get_mut(index) {
            c.enabled = enabled;
        }
    }

    /// Apply all enabled constraints to a pose (array of global joint transforms).
    /// Constraints are applied in order (sorted by `order` field).
    pub fn apply(&self, transforms: &mut [Mat4]) {
        // Collect enabled constraints and sort by order.
        let mut sorted: Vec<&JointConstraint> =
            self.constraints.iter().filter(|c| c.enabled).collect();
        sorted.sort_by_key(|c| c.order);

        for jc in sorted {
            let idx = jc.joint_index;
            if idx >= transforms.len() {
                continue;
            }
            match &jc.constraint {
                Constraint::Parent {
                    parent_joint,
                    offset,
                    weight,
                } => {
                    if *parent_joint < transforms.len() {
                        let target = transforms[*parent_joint] * *offset;
                        transforms[idx] = lerp_mat4(&transforms[idx], &target, *weight);
                    }
                }
                Constraint::Aim {
                    target,
                    up,
                    aim_axis,
                    weight,
                } => {
                    let pos = translation_of(&transforms[idx]);
                    let dir = *target - pos;
                    if dir.length_squared() > 1e-12 {
                        let forward = dir.normalize();
                        let right;
                        let actual_up;
                        match aim_axis {
                            0 => {
                                // X aims at target
                                right = forward;
                                actual_up = up.normalize();
                                let back = right.cross(actual_up).normalize();
                                let corrected_up = back.cross(right).normalize();
                                let aim_mat = Mat4::from_cols(
                                    right.extend(0.0),
                                    corrected_up.extend(0.0),
                                    back.extend(0.0),
                                    pos.extend(1.0),
                                );
                                transforms[idx] =
                                    lerp_mat4(&transforms[idx], &aim_mat, *weight);
                            }
                            1 => {
                                // Y aims at target
                                actual_up = forward;
                                right = actual_up.cross(up.normalize()).normalize();
                                let back = right.cross(actual_up).normalize();
                                let aim_mat = Mat4::from_cols(
                                    right.extend(0.0),
                                    actual_up.extend(0.0),
                                    back.extend(0.0),
                                    pos.extend(1.0),
                                );
                                transforms[idx] =
                                    lerp_mat4(&transforms[idx], &aim_mat, *weight);
                            }
                            _ => {
                                // Z aims at target (most common: look-at)
                                let back = -forward;
                                actual_up = up.normalize();
                                right = actual_up.cross(back).normalize();
                                let corrected_up = back.cross(right).normalize();
                                let aim_mat = Mat4::from_cols(
                                    right.extend(0.0),
                                    corrected_up.extend(0.0),
                                    back.extend(0.0),
                                    pos.extend(1.0),
                                );
                                transforms[idx] =
                                    lerp_mat4(&transforms[idx], &aim_mat, *weight);
                            }
                        }
                    }
                }
                Constraint::CopyPosition {
                    source_joint,
                    axes,
                    weight,
                } => {
                    if *source_joint < transforms.len() {
                        let src_pos = translation_of(&transforms[*source_joint]);
                        let cur_pos = translation_of(&transforms[idx]);
                        let new_pos = Vec3::new(
                            if axes[0] {
                                lerp_f32(cur_pos.x, src_pos.x, *weight)
                            } else {
                                cur_pos.x
                            },
                            if axes[1] {
                                lerp_f32(cur_pos.y, src_pos.y, *weight)
                            } else {
                                cur_pos.y
                            },
                            if axes[2] {
                                lerp_f32(cur_pos.z, src_pos.z, *weight)
                            } else {
                                cur_pos.z
                            },
                        );
                        set_translation(&mut transforms[idx], new_pos);
                    }
                }
                Constraint::CopyRotation {
                    source_joint,
                    axes,
                    weight,
                } => {
                    if *source_joint < transforms.len() {
                        let src_rot = quat_of(&transforms[*source_joint]);
                        let cur_rot = quat_of(&transforms[idx]);
                        let (src_x, src_y, src_z) = euler_from_quat(src_rot);
                        let (cur_x, cur_y, cur_z) = euler_from_quat(cur_rot);
                        let rx = if axes[0] {
                            lerp_f32(cur_x, src_x, *weight)
                        } else {
                            cur_x
                        };
                        let ry = if axes[1] {
                            lerp_f32(cur_y, src_y, *weight)
                        } else {
                            cur_y
                        };
                        let rz = if axes[2] {
                            lerp_f32(cur_z, src_z, *weight)
                        } else {
                            cur_z
                        };
                        let new_rot = Quat::from_euler(glam::EulerRot::XYZ, rx, ry, rz);
                        let pos = translation_of(&transforms[idx]);
                        let (scale, _, _) = transforms[idx].to_scale_rotation_translation();
                        transforms[idx] = Mat4::from_scale_rotation_translation(
                            scale, new_rot, pos,
                        );
                    }
                }
                Constraint::PinToWorld { target, weight } => {
                    let cur_pos = translation_of(&transforms[idx]);
                    let new_pos = cur_pos.lerp(*target, *weight);
                    set_translation(&mut transforms[idx], new_pos);
                }
                Constraint::FollowPath {
                    path,
                    progress,
                    orient_to_path,
                    up,
                    weight,
                } => {
                    let pos = path.evaluate(*progress);
                    let cur_pos = translation_of(&transforms[idx]);
                    let new_pos = cur_pos.lerp(pos, *weight);

                    if *orient_to_path {
                        let tangent = path.tangent(*progress);
                        let forward = tangent;
                        let right = up.normalize().cross(forward).normalize();
                        let corrected_up = forward.cross(right).normalize();
                        let rot = Mat4::from_cols(
                            right.extend(0.0),
                            corrected_up.extend(0.0),
                            forward.extend(0.0),
                            Vec3::ZERO.extend(1.0),
                        );
                        let target_mat =
                            Mat4::from_translation(new_pos) * rot;
                        transforms[idx] =
                            lerp_mat4(&transforms[idx], &target_mat, *weight);
                    } else {
                        set_translation(&mut transforms[idx], new_pos);
                    }
                }
            }
        }
    }
}

impl Default for ConstraintStack {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Preset path generators
// ---------------------------------------------------------------------------

/// Create a circular path on the XZ plane.
pub fn circle_path(name: &str, center: Vec3, radius: f32, segments: usize) -> SplinePath {
    let mut path = SplinePath::new(name);
    path.closed = true;
    for i in 0..segments {
        let angle = std::f32::consts::TAU * (i as f32 / segments as f32);
        path.add_point(Vec3::new(
            center.x + radius * angle.cos(),
            center.y,
            center.z + radius * angle.sin(),
        ));
    }
    path
}

/// Create a figure-eight (lemniscate) path on the XZ plane.
pub fn figure_eight_path(name: &str, center: Vec3, size: f32, segments: usize) -> SplinePath {
    let mut path = SplinePath::new(name);
    path.closed = true;
    for i in 0..segments {
        let angle = std::f32::consts::TAU * (i as f32 / segments as f32);
        // Lemniscate of Bernoulli (parametric approximation)
        path.add_point(Vec3::new(
            center.x + size * angle.sin(),
            center.y,
            center.z + size * angle.sin() * angle.cos(),
        ));
    }
    path
}

/// Create a straight-line path from `start` to `end`.
pub fn linear_path(name: &str, start: Vec3, end: Vec3) -> SplinePath {
    let mut path = SplinePath::new(name);
    path.add_point(start);
    path.add_point(end);
    path
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn translation_of(m: &Mat4) -> Vec3 {
    Vec3::new(m.w_axis.x, m.w_axis.y, m.w_axis.z)
}

fn set_translation(m: &mut Mat4, pos: Vec3) {
    m.w_axis.x = pos.x;
    m.w_axis.y = pos.y;
    m.w_axis.z = pos.z;
}

fn quat_of(m: &Mat4) -> Quat {
    let (_, rot, _) = m.to_scale_rotation_translation();
    rot
}

fn euler_from_quat(q: Quat) -> (f32, f32, f32) {
    q.to_euler(glam::EulerRot::XYZ)
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Linearly interpolate two Mat4 by decomposing into translation + rotation + scale.
fn lerp_mat4(a: &Mat4, b: &Mat4, t: f32) -> Mat4 {
    let t = t.clamp(0.0, 1.0);
    let (sa, ra, ta) = a.to_scale_rotation_translation();
    let (sb, rb, tb) = b.to_scale_rotation_translation();
    let pos = ta.lerp(tb, t);
    let rot = ra.slerp(rb, t);
    let scl = sa.lerp(sb, t);
    Mat4::from_scale_rotation_translation(scl, rot, pos)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Vec3, b: Vec3, eps: f32) -> bool {
        (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
    }

    #[test]
    fn test_spline_linear() {
        // Two points should produce a simple lerp.
        let mut path = SplinePath::new("linear");
        path.add_point(Vec3::ZERO);
        path.add_point(Vec3::new(10.0, 0.0, 0.0));

        let mid = path.evaluate(0.5);
        assert!(approx(mid, Vec3::new(5.0, 0.0, 0.0), 0.01));

        let start = path.evaluate(0.0);
        assert!(approx(start, Vec3::ZERO, 0.01));

        let end = path.evaluate(1.0);
        assert!(approx(end, Vec3::new(10.0, 0.0, 0.0), 0.01));
    }

    #[test]
    fn test_spline_evaluate_endpoints() {
        let mut path = SplinePath::new("quad");
        path.add_point(Vec3::new(0.0, 0.0, 0.0));
        path.add_point(Vec3::new(5.0, 10.0, 0.0));
        path.add_point(Vec3::new(10.0, 0.0, 0.0));
        path.add_point(Vec3::new(15.0, 5.0, 0.0));

        let start = path.evaluate(0.0);
        let end = path.evaluate(1.0);
        assert!(approx(start, Vec3::new(0.0, 0.0, 0.0), 0.01));
        assert!(approx(end, Vec3::new(15.0, 5.0, 0.0), 0.01));
    }

    #[test]
    fn test_circle_path() {
        let path = circle_path("circle", Vec3::ZERO, 5.0, 32);
        assert_eq!(path.points.len(), 32);
        assert!(path.closed);

        // First point should be at (5, 0, 0)
        assert!(approx(path.points[0], Vec3::new(5.0, 0.0, 0.0), 0.01));

        // Length should be approximately 2*pi*r = ~31.4
        let len = path.length();
        assert!((len - 31.416).abs() < 1.0, "circle length {} far from 31.4", len);
    }

    #[test]
    fn test_aim_constraint() {
        // Place a joint at origin, aim Z-axis at a target on +Z.
        let mut transforms = vec![Mat4::IDENTITY];
        let target = Vec3::new(0.0, 0.0, 10.0);

        let mut stack = ConstraintStack::new();
        stack.add(JointConstraint {
            name: "aim".to_string(),
            joint_index: 0,
            constraint: Constraint::Aim {
                target,
                up: Vec3::Y,
                aim_axis: 2,
                weight: 1.0,
            },
            enabled: true,
            order: 0,
        });
        stack.apply(&mut transforms);

        // After aiming Z at +Z, the transform should still be close to identity-like
        // (joint already faces +Z). Position should remain at origin.
        let pos = translation_of(&transforms[0]);
        assert!(approx(pos, Vec3::ZERO, 0.01));
    }

    #[test]
    fn test_parent_constraint() {
        // Joint 0 is the "parent" at (10, 0, 0). Joint 1 follows joint 0 with offset.
        let mut transforms = vec![
            Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            Mat4::IDENTITY,
        ];

        let offset = Mat4::from_translation(Vec3::new(0.0, 5.0, 0.0));
        let mut stack = ConstraintStack::new();
        stack.add(JointConstraint {
            name: "parent".to_string(),
            joint_index: 1,
            constraint: Constraint::Parent {
                parent_joint: 0,
                offset,
                weight: 1.0,
            },
            enabled: true,
            order: 0,
        });
        stack.apply(&mut transforms);

        let pos = translation_of(&transforms[1]);
        assert!(
            approx(pos, Vec3::new(10.0, 5.0, 0.0), 0.01),
            "expected (10,5,0) got {:?}",
            pos
        );
    }

    #[test]
    fn test_follow_path() {
        let path = linear_path("line", Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0));
        let mut transforms = vec![Mat4::IDENTITY];

        let mut stack = ConstraintStack::new();
        stack.add(JointConstraint {
            name: "follow".to_string(),
            joint_index: 0,
            constraint: Constraint::FollowPath {
                path,
                progress: 0.5,
                orient_to_path: false,
                up: Vec3::Y,
                weight: 1.0,
            },
            enabled: true,
            order: 0,
        });
        stack.apply(&mut transforms);

        let pos = translation_of(&transforms[0]);
        assert!(
            approx(pos, Vec3::new(10.0, 0.0, 0.0), 0.01),
            "expected (10,0,0) got {:?}",
            pos
        );
    }

    #[test]
    fn test_constraint_stack_order() {
        // Two constraints on the same joint: the one with lower order runs first.
        // PinToWorld at (5,0,0) order=1, then PinToWorld at (0,10,0) order=2.
        // The second one should win (applied last).
        let mut transforms = vec![Mat4::IDENTITY];

        let mut stack = ConstraintStack::new();
        stack.add(JointConstraint {
            name: "pin_a".to_string(),
            joint_index: 0,
            constraint: Constraint::PinToWorld {
                target: Vec3::new(5.0, 0.0, 0.0),
                weight: 1.0,
            },
            enabled: true,
            order: 1,
        });
        stack.add(JointConstraint {
            name: "pin_b".to_string(),
            joint_index: 0,
            constraint: Constraint::PinToWorld {
                target: Vec3::new(0.0, 10.0, 0.0),
                weight: 1.0,
            },
            enabled: true,
            order: 2,
        });
        stack.apply(&mut transforms);

        let pos = translation_of(&transforms[0]);
        assert!(
            approx(pos, Vec3::new(0.0, 10.0, 0.0), 0.01),
            "expected (0,10,0) got {:?}",
            pos
        );
    }

    #[test]
    fn test_pin_to_world() {
        let mut transforms = vec![Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))];
        let target = Vec3::new(10.0, 20.0, 30.0);

        let mut stack = ConstraintStack::new();
        stack.add(JointConstraint {
            name: "pin".to_string(),
            joint_index: 0,
            constraint: Constraint::PinToWorld {
                target,
                weight: 0.5,
            },
            enabled: true,
            order: 0,
        });
        stack.apply(&mut transforms);

        // With weight 0.5, position should be midway between (1,2,3) and (10,20,30).
        let pos = translation_of(&transforms[0]);
        assert!(
            approx(pos, Vec3::new(5.5, 11.0, 16.5), 0.01),
            "expected (5.5,11,16.5) got {:?}",
            pos
        );
    }
}
