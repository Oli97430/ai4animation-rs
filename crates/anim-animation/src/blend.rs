//! Animation blending: crossfade, layers, and transition management.
//!
//! Provides tools for smooth interpolation between motion clips,
//! additive layering, and managed animation state transitions.

use glam::{Mat4, Vec3, Quat};
use anim_math::transform::Transform;

/// Blend two poses (arrays of Mat4) using spherical interpolation.
///
/// `weight` in [0.0, 1.0]: 0.0 = fully pose_a, 1.0 = fully pose_b.
pub fn blend_poses(pose_a: &[Mat4], pose_b: &[Mat4], weight: f32) -> Vec<Mat4> {
    let w = weight.clamp(0.0, 1.0);
    let len = pose_a.len().min(pose_b.len());
    let mut result = Vec::with_capacity(len);

    for i in 0..len {
        result.push(pose_a[i].interpolate(&pose_b[i], w));
    }
    result
}

/// Blend two position arrays using linear interpolation.
pub fn blend_positions(a: &[Vec3], b: &[Vec3], weight: f32) -> Vec<Vec3> {
    let w = weight.clamp(0.0, 1.0);
    a.iter().zip(b.iter())
        .map(|(&va, &vb)| va.lerp(vb, w))
        .collect()
}

/// Blend two velocity arrays using linear interpolation.
pub fn blend_velocities(a: &[Vec3], b: &[Vec3], weight: f32) -> Vec<Vec3> {
    blend_positions(a, b, weight)
}

/// Blend mode for an animation layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    /// Override: replaces the lower layer by `weight`.
    Override,
    /// Additive: adds the delta from rest pose, scaled by `weight`.
    Additive,
}

/// A single layer in an animation blend stack.
#[derive(Clone)]
pub struct AnimationLayer {
    /// Display name.
    pub name: String,
    /// Blend mode.
    pub mode: BlendMode,
    /// Blend weight [0.0, 1.0].
    pub weight: f32,
    /// Which joints this layer affects (empty = all joints).
    pub mask: Vec<usize>,
    /// Current pose in this layer.
    pub pose: Vec<Mat4>,
}

impl AnimationLayer {
    pub fn new(name: impl Into<String>, mode: BlendMode) -> Self {
        Self {
            name: name.into(),
            mode,
            weight: 1.0,
            mask: Vec::new(),
            pose: Vec::new(),
        }
    }

    /// Set a joint mask: only these joint indices will be affected.
    pub fn with_mask(mut self, joints: Vec<usize>) -> Self {
        self.mask = joints;
        self
    }
}

/// Apply a stack of animation layers onto a base pose.
///
/// Layers are applied in order (bottom to top). Each layer either overrides
/// or additively modifies the accumulated pose.
pub fn apply_layers(base_pose: &[Mat4], layers: &[AnimationLayer], rest_pose: &[Mat4]) -> Vec<Mat4> {
    let num_joints = base_pose.len();
    let mut result = base_pose.to_vec();

    for layer in layers {
        if layer.weight < 0.001 || layer.pose.is_empty() {
            continue;
        }

        let w = layer.weight.clamp(0.0, 1.0);

        match layer.mode {
            BlendMode::Override => {
                if layer.mask.is_empty() {
                    // Affect all joints
                    for i in 0..num_joints.min(layer.pose.len()) {
                        result[i] = result[i].interpolate(&layer.pose[i], w);
                    }
                } else {
                    // Only affect masked joints
                    for &j in &layer.mask {
                        if j < num_joints && j < layer.pose.len() {
                            result[j] = result[j].interpolate(&layer.pose[j], w);
                        }
                    }
                }
            }
            BlendMode::Additive => {
                // Additive: compute delta from rest, apply scaled
                if layer.mask.is_empty() {
                    for i in 0..num_joints.min(layer.pose.len()).min(rest_pose.len()) {
                        let delta = compute_additive_delta(rest_pose[i], layer.pose[i]);
                        result[i] = apply_additive_delta(result[i], delta, w);
                    }
                } else {
                    for &j in &layer.mask {
                        if j < num_joints && j < layer.pose.len() && j < rest_pose.len() {
                            let delta = compute_additive_delta(rest_pose[j], layer.pose[j]);
                            result[j] = apply_additive_delta(result[j], delta, w);
                        }
                    }
                }
            }
        }
    }

    result
}

/// Compute the additive delta between a rest pose and an animated pose.
fn compute_additive_delta(rest: Mat4, animated: Mat4) -> (Vec3, Quat) {
    let rest_pos = rest.get_position();
    let anim_pos = animated.get_position();
    let pos_delta = anim_pos - rest_pos;

    let rest_rot = Quat::from_mat4(&rest);
    let anim_rot = Quat::from_mat4(&animated);
    let rot_delta = rest_rot.conjugate() * anim_rot;

    (pos_delta, rot_delta)
}

/// Apply an additive delta (position offset + rotation delta) scaled by weight.
fn apply_additive_delta(base: Mat4, delta: (Vec3, Quat), weight: f32) -> Mat4 {
    let base_pos = base.get_position();
    let base_rot = Quat::from_mat4(&base);

    let new_pos = base_pos + delta.0 * weight;
    let new_rot = base_rot * Quat::IDENTITY.slerp(delta.1, weight);

    Mat4::from_rotation_translation(new_rot, new_pos)
}

/// Crossfade transition between two animation clips.
///
/// Manages the timing and blend weight for a smooth transition
/// from one clip to another over a configurable duration.
#[derive(Clone)]
pub struct AnimationTransition {
    /// Duration of the crossfade in seconds.
    pub duration: f32,
    /// Elapsed time since transition started.
    pub elapsed: f32,
    /// Is the transition currently active?
    pub active: bool,
    /// Easing curve type.
    pub curve: EasingCurve,
    /// Source clip timestamp at transition start.
    pub from_timestamp: f32,
    /// Target clip timestamp at transition start.
    pub to_timestamp: f32,
}

/// Easing curve for transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingCurve {
    /// Linear interpolation.
    Linear,
    /// Smooth start and end (cubic Hermite).
    SmoothStep,
    /// Smooth start, fast end.
    EaseIn,
    /// Fast start, smooth end.
    EaseOut,
}

impl EasingCurve {
    /// Apply the easing function to a linear parameter t in [0,1].
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::SmoothStep => t * t * (3.0 - 2.0 * t),
            Self::EaseIn => t * t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
        }
    }
}

impl AnimationTransition {
    /// Create a new transition with given crossfade duration.
    pub fn new(duration: f32) -> Self {
        Self {
            duration: duration.max(0.01),
            elapsed: 0.0,
            active: false,
            curve: EasingCurve::SmoothStep,
            from_timestamp: 0.0,
            to_timestamp: 0.0,
        }
    }

    /// Start a transition from the current playback position.
    pub fn start(&mut self, from_time: f32, to_time: f32) {
        self.from_timestamp = from_time;
        self.to_timestamp = to_time;
        self.elapsed = 0.0;
        self.active = true;
    }

    /// Advance the transition by dt seconds. Returns the blend weight [0,1].
    /// 0 = fully source, 1 = fully target.
    pub fn update(&mut self, dt: f32) -> f32 {
        if !self.active {
            return 1.0; // fully on target when not transitioning
        }

        self.elapsed += dt;
        let linear_t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let weight = self.curve.apply(linear_t);

        if self.elapsed >= self.duration {
            self.active = false;
        }

        weight
    }

    /// Whether the transition is still blending.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Current blend weight without advancing time.
    pub fn weight(&self) -> f32 {
        if !self.active { return 1.0; }
        let linear_t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        self.curve.apply(linear_t)
    }
}

/// Animation blend tree node: a simple two-input blend controller.
///
/// Useful for locomotion (blend between walk and run based on speed),
/// or directional blending (blend between forward and strafe).
#[derive(Clone)]
pub struct BlendNode {
    /// Parameter that drives the blend (e.g., speed, direction angle).
    pub parameter: f32,
    /// Threshold at which clip A is fully weighted.
    pub threshold_a: f32,
    /// Threshold at which clip B is fully weighted.
    pub threshold_b: f32,
}

impl BlendNode {
    pub fn new(threshold_a: f32, threshold_b: f32) -> Self {
        Self {
            parameter: 0.0,
            threshold_a,
            threshold_b,
        }
    }

    /// Set the blend parameter.
    pub fn set_parameter(&mut self, value: f32) {
        self.parameter = value;
    }

    /// Get the blend weight for clip B: 0.0 = fully A, 1.0 = fully B.
    pub fn weight(&self) -> f32 {
        let range = self.threshold_b - self.threshold_a;
        if range.abs() < 1e-6 { return 0.0; }
        ((self.parameter - self.threshold_a) / range).clamp(0.0, 1.0)
    }

    /// Blend two poses using the current parameter value.
    pub fn blend(&self, pose_a: &[Mat4], pose_b: &[Mat4]) -> Vec<Mat4> {
        blend_poses(pose_a, pose_b, self.weight())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_poses_extremes() {
        let a = vec![Mat4::IDENTITY; 3];
        let b = vec![Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)); 3];

        let result_a = blend_poses(&a, &b, 0.0);
        let result_b = blend_poses(&a, &b, 1.0);

        for m in &result_a {
            let pos = m.get_position();
            assert!(pos.length() < 0.01, "At weight 0, should be close to pose A");
        }
        for m in &result_b {
            let pos = m.get_position();
            assert!((pos - Vec3::new(1.0, 2.0, 3.0)).length() < 0.01);
        }
    }

    #[test]
    fn test_blend_midpoint() {
        let a = vec![Mat4::from_translation(Vec3::ZERO)];
        let b = vec![Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0))];

        let mid = blend_poses(&a, &b, 0.5);
        let pos = mid[0].get_position();
        assert!((pos.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_easing_curves() {
        assert!((EasingCurve::Linear.apply(0.5) - 0.5).abs() < 0.01);
        assert!((EasingCurve::SmoothStep.apply(0.0)).abs() < 0.01);
        assert!((EasingCurve::SmoothStep.apply(1.0) - 1.0).abs() < 0.01);
        assert!((EasingCurve::EaseIn.apply(0.0)).abs() < 0.01);
        assert!((EasingCurve::EaseOut.apply(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_transition_lifecycle() {
        let mut trans = AnimationTransition::new(1.0);
        assert!(!trans.is_active());

        trans.start(0.5, 0.0);
        assert!(trans.is_active());

        // Halfway through
        let w = trans.update(0.5);
        assert!(w > 0.3 && w < 0.7);
        assert!(trans.is_active());

        // Complete
        let w = trans.update(0.6);
        assert!((w - 1.0).abs() < 0.01);
        assert!(!trans.is_active());
    }

    #[test]
    fn test_blend_node() {
        let mut node = BlendNode::new(0.0, 1.0);

        node.set_parameter(0.0);
        assert!((node.weight() - 0.0).abs() < 0.01);

        node.set_parameter(0.5);
        assert!((node.weight() - 0.5).abs() < 0.01);

        node.set_parameter(1.0);
        assert!((node.weight() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_apply_layers_override() {
        let base = vec![Mat4::IDENTITY; 2];
        let layer_pose = vec![Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)); 2];

        let layer = AnimationLayer {
            name: "test".into(),
            mode: BlendMode::Override,
            weight: 0.5,
            mask: Vec::new(),
            pose: layer_pose,
        };

        let result = apply_layers(&base, &[layer], &base);
        let pos = result[0].get_position();
        assert!((pos.x - 0.5).abs() < 0.05);
    }
}
