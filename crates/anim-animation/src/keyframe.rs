//! Flash-inspired keyframe animation system.
//!
//! Provides sparse keyframe storage with tweened interpolation,
//! supporting multiple easing types including cubic bezier curves.
//! Keyframe tracks can be converted to/from dense frame-based [`Motion`] data.

use glam::{Mat4, Vec3, Quat, EulerRot};
use anim_math::transform::Transform;
use crate::motion::Motion;

// ---------------------------------------------------------------------------
// Interpolatable trait
// ---------------------------------------------------------------------------

/// Types that can be linearly interpolated between two values.
pub trait Interpolatable {
    fn interpolate_value(a: &Self, b: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    fn interpolate_value(a: &f32, b: &f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}

// ---------------------------------------------------------------------------
// TweenType
// ---------------------------------------------------------------------------

/// Easing / tweening function applied between two keyframes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TweenType {
    /// No interpolation -- hold the left keyframe value until the next keyframe.
    None,
    /// Linear interpolation.
    Linear,
    /// Quadratic ease-in (slow start).
    EaseIn,
    /// Quadratic ease-out (slow end).
    EaseOut,
    /// Quadratic ease-in-out (slow start and end).
    EaseInOut,
    /// Cubic bezier curve with two control points.
    Bezier {
        cx1: f32,
        cy1: f32,
        cx2: f32,
        cy2: f32,
    },
}

impl Default for TweenType {
    fn default() -> Self {
        TweenType::Linear
    }
}

impl TweenType {
    /// Evaluate the easing function for a normalized parameter `t` in [0, 1].
    /// Returns the eased parameter in [0, 1] (may overshoot for some bezier configs).
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            TweenType::None => 0.0,
            TweenType::Linear => t,
            TweenType::EaseIn => t * t,
            TweenType::EaseOut => t * (2.0 - t),
            TweenType::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            TweenType::Bezier { cx1, cy1, cx2, cy2 } => {
                cubic_bezier_y(t, *cx1, *cy1, *cx2, *cy2)
            }
        }
    }
}

/// Approximate the Y-value of a cubic bezier curve B(t) = (1-t)^3*P0 + 3(1-t)^2*t*P1 + 3(1-t)*t^2*P2 + t^3*P3
/// where P0=(0,0), P1=(cx1,cy1), P2=(cx2,cy2), P3=(1,1).
///
/// We first solve for the bezier parameter `u` such that X(u) = t using Newton's method,
/// then evaluate Y(u).
fn cubic_bezier_y(t: f32, cx1: f32, cy1: f32, cx2: f32, cy2: f32) -> f32 {
    // Find u such that bezier_x(u) == t
    let mut u = t; // initial guess
    for _ in 0..8 {
        let x = bezier_component(u, cx1, cx2);
        let dx = bezier_component_derivative(u, cx1, cx2);
        if dx.abs() < 1e-7 {
            break;
        }
        u -= (x - t) / dx;
        u = u.clamp(0.0, 1.0);
    }
    bezier_component(u, cy1, cy2)
}

/// Evaluate one component of a cubic bezier:
/// B(u) = 3(1-u)^2*u*c1 + 3(1-u)*u^2*c2 + u^3
fn bezier_component(u: f32, c1: f32, c2: f32) -> f32 {
    let u2 = u * u;
    let u3 = u2 * u;
    let inv = 1.0 - u;
    let inv2 = inv * inv;
    3.0 * inv2 * u * c1 + 3.0 * inv * u2 * c2 + u3
}

/// Derivative of bezier_component with respect to u.
fn bezier_component_derivative(u: f32, c1: f32, c2: f32) -> f32 {
    let inv = 1.0 - u;
    3.0 * inv * inv * c1 + 6.0 * inv * u * (c2 - c1) + 3.0 * u * u * (1.0 - c2)
}

// ---------------------------------------------------------------------------
// Keyframe
// ---------------------------------------------------------------------------

/// A single keyframe: a value at a specific frame with an outgoing tween type.
#[derive(Debug, Clone)]
pub struct Keyframe<T: Clone> {
    /// Frame index (0-based).
    pub frame: usize,
    /// Value at this keyframe.
    pub value: T,
    /// Tween type used when interpolating *from* this keyframe to the next.
    pub tween: TweenType,
}

impl<T: Clone> Keyframe<T> {
    pub fn new(frame: usize, value: T, tween: TweenType) -> Self {
        Self { frame, value, tween }
    }
}

// ---------------------------------------------------------------------------
// KeyframeTrack
// ---------------------------------------------------------------------------

/// A track of keyframes for a single animatable property.
///
/// Keyframes are kept sorted by frame index. Interpolation between two
/// keyframes uses the *outgoing* tween of the earlier keyframe.
#[derive(Debug, Clone)]
pub struct KeyframeTrack<T: Clone + Default + Interpolatable> {
    keyframes: Vec<Keyframe<T>>,
}

impl<T: Clone + Default + Interpolatable> Default for KeyframeTrack<T> {
    fn default() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }
}

impl<T: Clone + Default + Interpolatable> KeyframeTrack<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a keyframe at the given frame.
    /// Uses `TweenType::Linear` as the default tween.
    pub fn insert_keyframe(&mut self, frame: usize, value: T) {
        self.insert_keyframe_with_tween(frame, value, TweenType::Linear);
    }

    /// Insert or replace a keyframe with a specific tween type.
    pub fn insert_keyframe_with_tween(&mut self, frame: usize, value: T, tween: TweenType) {
        match self.keyframes.binary_search_by_key(&frame, |kf| kf.frame) {
            Ok(idx) => {
                self.keyframes[idx].value = value;
                self.keyframes[idx].tween = tween;
            }
            Err(idx) => {
                self.keyframes.insert(idx, Keyframe::new(frame, value, tween));
            }
        }
    }

    /// Remove the keyframe at the given frame, if it exists.
    pub fn remove_keyframe(&mut self, frame: usize) -> bool {
        if let Ok(idx) = self.keyframes.binary_search_by_key(&frame, |kf| kf.frame) {
            self.keyframes.remove(idx);
            true
        } else {
            false
        }
    }

    /// Check whether a keyframe exists at the given frame.
    pub fn has_keyframe(&self, frame: usize) -> bool {
        self.keyframes.binary_search_by_key(&frame, |kf| kf.frame).is_ok()
    }

    /// Return the sorted list of frame indices that have keyframes.
    pub fn keyframe_frames(&self) -> Vec<usize> {
        self.keyframes.iter().map(|kf| kf.frame).collect()
    }

    /// Get the number of keyframes in this track.
    pub fn len(&self) -> usize {
        self.keyframes.len()
    }

    /// Check if the track has no keyframes.
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }

    /// Get the interpolated value at a (possibly fractional) frame.
    ///
    /// - Before the first keyframe: returns the first keyframe's value.
    /// - After the last keyframe: returns the last keyframe's value.
    /// - Between two keyframes: interpolates using the left keyframe's tween.
    /// - On a keyframe exactly: returns that keyframe's value.
    pub fn get_value(&self, frame: usize) -> T {
        if self.keyframes.is_empty() {
            return T::default();
        }
        if self.keyframes.len() == 1 {
            return self.keyframes[0].value.clone();
        }

        let first = &self.keyframes[0];
        let last = &self.keyframes[self.keyframes.len() - 1];

        if frame <= first.frame {
            return first.value.clone();
        }
        if frame >= last.frame {
            return last.value.clone();
        }

        // Find the pair of keyframes surrounding this frame.
        match self.keyframes.binary_search_by_key(&frame, |kf| kf.frame) {
            Ok(idx) => self.keyframes[idx].value.clone(),
            Err(idx) => {
                // idx is the insertion point; the keyframe before is at idx-1.
                let kf_a = &self.keyframes[idx - 1];
                let kf_b = &self.keyframes[idx];
                let span = (kf_b.frame - kf_a.frame) as f32;
                let local_t = (frame - kf_a.frame) as f32 / span;
                let eased_t = kf_a.tween.evaluate(local_t);
                T::interpolate_value(&kf_a.value, &kf_b.value, eased_t)
            }
        }
    }

    /// Get a reference to the underlying keyframes slice.
    pub fn keyframes(&self) -> &[Keyframe<T>] {
        &self.keyframes
    }

    /// Set the tween type on a keyframe at the given frame.
    pub fn set_tween(&mut self, frame: usize, tween: TweenType) -> bool {
        if let Ok(idx) = self.keyframes.binary_search_by_key(&frame, |kf| kf.frame) {
            self.keyframes[idx].tween = tween;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// KeyframeLayer -- per-joint keyframe tracks
// ---------------------------------------------------------------------------

/// A set of keyframe tracks controlling the transform of a single joint / bone.
///
/// Stores position (x, y, z) and rotation as Euler angles (x, y, z) in radians
/// using the YXZ convention (matching glam's `EulerRot::YXZ`).
#[derive(Debug, Clone)]
pub struct KeyframeLayer {
    pub name: String,
    pub joint_index: usize,

    pub position_x: KeyframeTrack<f32>,
    pub position_y: KeyframeTrack<f32>,
    pub position_z: KeyframeTrack<f32>,

    pub rotation_x: KeyframeTrack<f32>,
    pub rotation_y: KeyframeTrack<f32>,
    pub rotation_z: KeyframeTrack<f32>,
}

impl KeyframeLayer {
    pub fn new(name: impl Into<String>, joint_index: usize) -> Self {
        Self {
            name: name.into(),
            joint_index,
            position_x: KeyframeTrack::new(),
            position_y: KeyframeTrack::new(),
            position_z: KeyframeTrack::new(),
            rotation_x: KeyframeTrack::new(),
            rotation_y: KeyframeTrack::new(),
            rotation_z: KeyframeTrack::new(),
        }
    }

    /// Insert keyframes for all six channels at the given frame from a Mat4.
    pub fn insert_transform_keyframe(&mut self, frame: usize, transform: &Mat4) {
        let pos = transform.get_position();
        self.position_x.insert_keyframe(frame, pos.x);
        self.position_y.insert_keyframe(frame, pos.y);
        self.position_z.insert_keyframe(frame, pos.z);

        let quat = Quat::from_mat4(transform);
        let (ry, rx, rz) = quat.to_euler(EulerRot::YXZ);
        self.rotation_x.insert_keyframe(frame, rx);
        self.rotation_y.insert_keyframe(frame, ry);
        self.rotation_z.insert_keyframe(frame, rz);
    }

    /// Remove keyframes at the given frame from all channels.
    pub fn remove_all_keyframes(&mut self, frame: usize) {
        self.position_x.remove_keyframe(frame);
        self.position_y.remove_keyframe(frame);
        self.position_z.remove_keyframe(frame);
        self.rotation_x.remove_keyframe(frame);
        self.rotation_y.remove_keyframe(frame);
        self.rotation_z.remove_keyframe(frame);
    }

    /// Reconstruct a 4x4 transform matrix at the given frame by evaluating
    /// all six tracks with their tweens.
    pub fn get_transform(&self, frame: usize) -> Mat4 {
        let px = self.position_x.get_value(frame);
        let py = self.position_y.get_value(frame);
        let pz = self.position_z.get_value(frame);

        let rx = self.rotation_x.get_value(frame);
        let ry = self.rotation_y.get_value(frame);
        let rz = self.rotation_z.get_value(frame);

        let quat = Quat::from_euler(EulerRot::YXZ, ry, rx, rz);
        Mat4::from_rotation_translation(quat, Vec3::new(px, py, pz))
    }

    /// Check whether any track has a keyframe at the given frame.
    pub fn has_keyframe(&self, frame: usize) -> bool {
        self.position_x.has_keyframe(frame)
            || self.position_y.has_keyframe(frame)
            || self.position_z.has_keyframe(frame)
            || self.rotation_x.has_keyframe(frame)
            || self.rotation_y.has_keyframe(frame)
            || self.rotation_z.has_keyframe(frame)
    }
}

// ---------------------------------------------------------------------------
// KeyframeAnimation
// ---------------------------------------------------------------------------

/// A Flash-inspired keyframe animation that stores sparse keyframes with tweens
/// rather than per-frame dense data.
///
/// Each layer controls one joint/bone.  The animation can be converted to and
/// from the dense [`Motion`] format used by the rest of the engine.
#[derive(Debug, Clone)]
pub struct KeyframeAnimation {
    pub name: String,
    pub layers: Vec<KeyframeLayer>,
    pub total_frames: usize,
    pub framerate: f32,
    pub current_frame: usize,
}

impl KeyframeAnimation {
    pub fn new(name: impl Into<String>, total_frames: usize, framerate: f32) -> Self {
        Self {
            name: name.into(),
            layers: Vec::new(),
            total_frames,
            framerate,
            current_frame: 0,
        }
    }

    /// Add a new layer (joint track) to the animation.
    pub fn add_layer(&mut self, name: impl Into<String>, joint_index: usize) -> usize {
        let idx = self.layers.len();
        self.layers.push(KeyframeLayer::new(name, joint_index));
        idx
    }

    /// Insert a keyframe on a specific layer at a given frame,
    /// capturing the supplied transform.
    pub fn insert_keyframe(&mut self, layer_index: usize, frame: usize, transform: &Mat4) {
        if let Some(layer) = self.layers.get_mut(layer_index) {
            layer.insert_transform_keyframe(frame, transform);
        }
    }

    /// Remove all keyframes on a layer at the given frame.
    pub fn remove_keyframe(&mut self, layer_index: usize, frame: usize) {
        if let Some(layer) = self.layers.get_mut(layer_index) {
            layer.remove_all_keyframes(frame);
        }
    }

    /// Get the interpolated transform for a layer at a given frame.
    pub fn get_layer_transform(&self, layer_index: usize, frame: usize) -> Mat4 {
        self.layers
            .get(layer_index)
            .map(|l| l.get_transform(frame))
            .unwrap_or(Mat4::IDENTITY)
    }

    /// Convert an existing dense [`Motion`] clip into a [`KeyframeAnimation`].
    ///
    /// Only the first and last frames are stored as keyframes (with linear tween),
    /// giving a minimal representation that can be refined by adding intermediate
    /// keyframes in the editor.
    pub fn from_motion(motion: &Motion) -> Self {
        let num_frames = motion.num_frames();
        let num_joints = motion.num_joints();
        let framerate = if motion.framerate > 0.0 { motion.framerate } else { 30.0 };

        let mut anim = KeyframeAnimation::new(motion.name.clone(), num_frames, framerate);

        for joint_idx in 0..num_joints {
            let joint_name = if joint_idx < motion.hierarchy.bone_names.len() {
                motion.hierarchy.bone_names[joint_idx].clone()
            } else {
                format!("Joint_{}", joint_idx)
            };
            let layer_idx = anim.add_layer(joint_name, joint_idx);

            if num_frames > 0 {
                // First frame keyframe
                let first_transform = &motion.frames[0][joint_idx];
                anim.layers[layer_idx].insert_transform_keyframe(0, first_transform);

                // Last frame keyframe (if more than one frame)
                if num_frames > 1 {
                    let last_transform = &motion.frames[num_frames - 1][joint_idx];
                    anim.layers[layer_idx].insert_transform_keyframe(num_frames - 1, last_transform);
                }
            }
        }

        anim
    }

    /// Bake the keyframe animation back into a dense [`Motion`] by evaluating
    /// every frame of every layer.
    ///
    /// The resulting Motion has no hierarchy information beyond joint names and
    /// a flat parent structure (all joints parented to root). The caller should
    /// supply a proper hierarchy if needed.
    pub fn to_motion(&self) -> Motion {
        let num_joints = self.layers.len();
        let mut frames: Vec<Vec<Mat4>> = Vec::with_capacity(self.total_frames);

        for frame_idx in 0..self.total_frames {
            let mut joints = Vec::with_capacity(num_joints);
            for layer in &self.layers {
                joints.push(layer.get_transform(frame_idx));
            }
            frames.push(joints);
        }

        // Build joint names and a flat parent structure.
        let joint_names: Vec<String> = self.layers.iter().map(|l| l.name.clone()).collect();
        let parent_indices: Vec<i32> = (0..num_joints).map(|i| if i == 0 { -1 } else { 0 }).collect();

        Motion::from_animation_data(&joint_names, &parent_indices, &frames, self.framerate)
    }

    /// Get the total duration in seconds.
    pub fn duration(&self) -> f32 {
        let fps = if self.framerate > 0.0 { self.framerate } else { 30.0 };
        if self.total_frames == 0 {
            0.0
        } else {
            (self.total_frames - 1) as f32 / fps
        }
    }

    /// Set the tween type on all six channels of a layer at a given frame.
    pub fn set_tween(&mut self, layer_index: usize, frame: usize, tween: TweenType) {
        if let Some(layer) = self.layers.get_mut(layer_index) {
            layer.position_x.set_tween(frame, tween);
            layer.position_y.set_tween(frame, tween);
            layer.position_z.set_tween(frame, tween);
            layer.rotation_x.set_tween(frame, tween);
            layer.rotation_y.set_tween(frame, tween);
            layer.rotation_z.set_tween(frame, tween);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    // -- TweenType evaluation --

    #[test]
    fn test_tween_none() {
        assert_eq!(TweenType::None.evaluate(0.0), 0.0);
        assert_eq!(TweenType::None.evaluate(0.5), 0.0);
        assert_eq!(TweenType::None.evaluate(1.0), 0.0);
    }

    #[test]
    fn test_tween_linear() {
        assert!((TweenType::Linear.evaluate(0.0)).abs() < 1e-6);
        assert!((TweenType::Linear.evaluate(0.5) - 0.5).abs() < 1e-6);
        assert!((TweenType::Linear.evaluate(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_tween_ease_in() {
        // EaseIn: t^2 -- slower at the start.
        assert!((TweenType::EaseIn.evaluate(0.0)).abs() < 1e-6);
        assert!((TweenType::EaseIn.evaluate(0.5) - 0.25).abs() < 1e-6);
        assert!((TweenType::EaseIn.evaluate(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_tween_ease_out() {
        // EaseOut: t*(2-t) -- slower at the end.
        assert!((TweenType::EaseOut.evaluate(0.0)).abs() < 1e-6);
        assert!((TweenType::EaseOut.evaluate(0.5) - 0.75).abs() < 1e-6);
        assert!((TweenType::EaseOut.evaluate(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_tween_ease_in_out() {
        let tw = TweenType::EaseInOut;
        assert!((tw.evaluate(0.0)).abs() < 1e-6);
        assert!((tw.evaluate(1.0) - 1.0).abs() < 1e-6);
        // Midpoint should be 0.5
        assert!((tw.evaluate(0.5) - 0.5).abs() < 1e-6);
        // First half is slower than linear
        assert!(tw.evaluate(0.25) < 0.25);
        // Second half is faster than linear
        assert!(tw.evaluate(0.75) > 0.75);
    }

    #[test]
    fn test_tween_bezier_linear_equivalent() {
        // Bezier with control points that produce a linear curve.
        let tw = TweenType::Bezier {
            cx1: 0.33,
            cy1: 0.33,
            cx2: 0.67,
            cy2: 0.67,
        };
        // Should be approximately linear.
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let val = tw.evaluate(t);
            assert!(
                (val - t).abs() < 0.05,
                "Bezier linear equivalent: t={}, val={}, diff={}",
                t,
                val,
                (val - t).abs()
            );
        }
    }

    #[test]
    fn test_tween_bezier_endpoints() {
        let tw = TweenType::Bezier {
            cx1: 0.42,
            cy1: 0.0,
            cx2: 0.58,
            cy2: 1.0,
        };
        assert!((tw.evaluate(0.0)).abs() < 1e-4);
        assert!((tw.evaluate(1.0) - 1.0).abs() < 1e-4);
    }

    // -- KeyframeTrack --

    #[test]
    fn test_track_linear_interpolation() {
        let mut track = KeyframeTrack::<f32>::new();
        track.insert_keyframe(0, 0.0);
        track.insert_keyframe(10, 100.0);

        assert!((track.get_value(0) - 0.0).abs() < 1e-6);
        assert!((track.get_value(5) - 50.0).abs() < 1e-6);
        assert!((track.get_value(10) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_track_ease_in_interpolation() {
        let mut track = KeyframeTrack::<f32>::new();
        track.insert_keyframe_with_tween(0, 0.0, TweenType::EaseIn);
        track.insert_keyframe(10, 100.0);

        // At midpoint, ease-in should yield 25.0 (t^2 = 0.25 * 100)
        let mid = track.get_value(5);
        assert!(
            (mid - 25.0).abs() < 1e-4,
            "EaseIn midpoint: expected ~25.0, got {}",
            mid
        );
    }

    #[test]
    fn test_track_none_tween() {
        let mut track = KeyframeTrack::<f32>::new();
        track.insert_keyframe_with_tween(0, 10.0, TweenType::None);
        track.insert_keyframe(10, 50.0);

        // "None" holds the left value.
        assert!((track.get_value(3) - 10.0).abs() < 1e-6);
        assert!((track.get_value(9) - 10.0).abs() < 1e-6);
        assert!((track.get_value(10) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_track_clamp_before_and_after() {
        let mut track = KeyframeTrack::<f32>::new();
        track.insert_keyframe(5, 42.0);
        track.insert_keyframe(15, 84.0);

        // Before first keyframe -- clamp to first value.
        assert!((track.get_value(0) - 42.0).abs() < 1e-6);
        assert!((track.get_value(3) - 42.0).abs() < 1e-6);
        // After last keyframe -- clamp to last value.
        assert!((track.get_value(20) - 84.0).abs() < 1e-6);
    }

    #[test]
    fn test_track_empty_returns_default() {
        let track = KeyframeTrack::<f32>::new();
        assert!((track.get_value(0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_track_insert_and_remove() {
        let mut track = KeyframeTrack::<f32>::new();
        track.insert_keyframe(0, 0.0);
        track.insert_keyframe(5, 50.0);
        track.insert_keyframe(10, 100.0);

        assert_eq!(track.len(), 3);
        assert!(track.has_keyframe(5));
        assert_eq!(track.keyframe_frames(), vec![0, 5, 10]);

        // Remove middle keyframe.
        assert!(track.remove_keyframe(5));
        assert!(!track.has_keyframe(5));
        assert_eq!(track.len(), 2);
        assert_eq!(track.keyframe_frames(), vec![0, 10]);

        // Value at frame 5 should now interpolate between 0 and 10.
        assert!((track.get_value(5) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_track_replace_keyframe() {
        let mut track = KeyframeTrack::<f32>::new();
        track.insert_keyframe(0, 10.0);
        assert!((track.get_value(0) - 10.0).abs() < 1e-6);

        // Replace value.
        track.insert_keyframe(0, 20.0);
        assert!((track.get_value(0) - 20.0).abs() < 1e-6);
        assert_eq!(track.len(), 1);
    }

    // -- KeyframeLayer transform --

    #[test]
    fn test_layer_identity_transform() {
        let layer = KeyframeLayer::new("test", 0);
        let mat = layer.get_transform(0);
        // With no keyframes, all tracks return 0.0, so we get identity rotation + zero translation.
        let expected = Mat4::from_rotation_translation(Quat::IDENTITY, Vec3::ZERO);
        assert_mat4_approx_eq(&mat, &expected, 1e-6);
    }

    #[test]
    fn test_layer_roundtrip() {
        let original = Mat4::from_rotation_translation(
            Quat::from_euler(EulerRot::YXZ, 0.3, 0.1, -0.2),
            Vec3::new(1.0, 2.0, 3.0),
        );

        let mut layer = KeyframeLayer::new("test", 0);
        layer.insert_transform_keyframe(0, &original);
        let recovered = layer.get_transform(0);

        assert_mat4_approx_eq(&original, &recovered, 1e-5);
    }

    // -- KeyframeAnimation from_motion roundtrip --

    #[test]
    fn test_from_motion_roundtrip() {
        // Build a simple 2-joint, 2-frame motion.
        let joint_names = vec!["Root".to_string(), "Child".to_string()];
        let parent_indices = vec![-1_i32, 0];

        let frame0 = vec![
            Mat4::from_rotation_translation(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.0)),
            Mat4::from_rotation_translation(Quat::IDENTITY, Vec3::new(1.0, 0.0, 0.0)),
        ];
        let frame1 = vec![
            Mat4::from_rotation_translation(Quat::IDENTITY, Vec3::new(0.0, 1.0, 0.0)),
            Mat4::from_rotation_translation(Quat::IDENTITY, Vec3::new(1.0, 1.0, 0.0)),
        ];
        let frames = vec![frame0.clone(), frame1.clone()];

        let motion = Motion::from_animation_data(&joint_names, &parent_indices, &frames, 30.0);

        // Convert to keyframe animation.
        let kf_anim = KeyframeAnimation::from_motion(&motion);
        assert_eq!(kf_anim.total_frames, 2);
        assert_eq!(kf_anim.layers.len(), 2);
        assert_eq!(kf_anim.framerate, 30.0);

        // Both first and last frame keyframes should be present.
        assert!(kf_anim.layers[0].has_keyframe(0));
        assert!(kf_anim.layers[0].has_keyframe(1));

        // Bake back to motion.
        let baked = kf_anim.to_motion();
        assert_eq!(baked.num_frames(), 2);
        assert_eq!(baked.num_joints(), 2);

        // Verify frame 0.
        for j in 0..2 {
            assert_mat4_approx_eq(&baked.frames[0][j], &frames[0][j], 1e-5);
        }
        // Verify frame 1.
        for j in 0..2 {
            assert_mat4_approx_eq(&baked.frames[1][j], &frames[1][j], 1e-5);
        }
    }

    #[test]
    fn test_from_motion_multi_frame_interpolation() {
        // Build a 5-frame, 1-joint motion with linear Y translation.
        let joint_names = vec!["Root".to_string()];
        let parent_indices = vec![-1_i32];

        let mut frames = Vec::new();
        for i in 0..5 {
            let y = i as f32 * 10.0;
            frames.push(vec![
                Mat4::from_rotation_translation(Quat::IDENTITY, Vec3::new(0.0, y, 0.0)),
            ]);
        }

        let motion = Motion::from_animation_data(&joint_names, &parent_indices, &frames, 30.0);
        let kf_anim = KeyframeAnimation::from_motion(&motion);

        // Only first (frame 0) and last (frame 4) should be keyframed.
        assert!(kf_anim.layers[0].has_keyframe(0));
        assert!(!kf_anim.layers[0].has_keyframe(2));
        assert!(kf_anim.layers[0].has_keyframe(4));

        // Because we use linear interpolation between frame 0 (y=0) and frame 4 (y=40),
        // frame 2 should interpolate to y=20.
        let mid_transform = kf_anim.layers[0].get_transform(2);
        let mid_pos = mid_transform.get_position();
        assert!(
            (mid_pos.y - 20.0).abs() < 1e-4,
            "Expected y=20.0 at frame 2, got {}",
            mid_pos.y
        );
    }

    #[test]
    fn test_add_layer_and_insert_keyframe() {
        let mut anim = KeyframeAnimation::new("test", 60, 30.0);
        let layer_idx = anim.add_layer("Hip", 0);

        let transform = Mat4::from_rotation_translation(
            Quat::from_rotation_y(PI / 4.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        anim.insert_keyframe(layer_idx, 0, &transform);
        anim.insert_keyframe(layer_idx, 30, &Mat4::IDENTITY);

        // Verify keyframes.
        assert!(anim.layers[layer_idx].has_keyframe(0));
        assert!(anim.layers[layer_idx].has_keyframe(30));

        // Midpoint should be interpolated.
        let mid = anim.get_layer_transform(layer_idx, 15);
        let mid_pos = mid.get_position();
        assert!(
            (mid_pos.y - 0.5).abs() < 1e-4,
            "Expected y=0.5 at midpoint, got {}",
            mid_pos.y
        );
    }

    #[test]
    fn test_set_tween_on_layer() {
        let mut anim = KeyframeAnimation::new("test", 100, 30.0);
        let idx = anim.add_layer("Root", 0);

        anim.insert_keyframe(idx, 0, &Mat4::from_translation(Vec3::ZERO));
        anim.insert_keyframe(idx, 100, &Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0)));

        // Change tween to ease-in.
        anim.set_tween(idx, 0, TweenType::EaseIn);

        // At frame 50, ease-in should give 25% of 100 = 25.
        let t50 = anim.get_layer_transform(idx, 50);
        let x50 = t50.get_position().x;
        assert!(
            (x50 - 25.0).abs() < 1.0,
            "EaseIn at midpoint: expected ~25.0, got {}",
            x50
        );
    }

    // -- Helpers --

    fn assert_mat4_approx_eq(a: &Mat4, b: &Mat4, eps: f32) {
        let cols_a = a.to_cols_array();
        let cols_b = b.to_cols_array();
        for (i, (&va, &vb)) in cols_a.iter().zip(cols_b.iter()).enumerate() {
            assert!(
                (va - vb).abs() < eps,
                "Mat4 mismatch at element {}: {} vs {} (diff={})",
                i,
                va,
                vb,
                (va - vb).abs()
            );
        }
    }
}
