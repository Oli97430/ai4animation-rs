//! Camera animation — keyframe-based camera paths with position, target, FOV.
//!
//! Supports smooth camera fly-throughs, dolly shots, and scripted camera moves
//! using the same tween system as the Flash timeline.

use glam::Vec3;

// ---------------------------------------------------------------------------
// Camera keyframe
// ---------------------------------------------------------------------------

/// Interpolation type for camera keyframes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraEasing {
    /// Hard cut — instant transition.
    Cut,
    /// Linear interpolation.
    Linear,
    /// Smooth ease-in-out (hermite).
    Smooth,
    /// Slow start.
    EaseIn,
    /// Slow end.
    EaseOut,
}

impl Default for CameraEasing {
    fn default() -> Self {
        CameraEasing::Smooth
    }
}

impl CameraEasing {
    /// Evaluate the easing curve for parameter `t` in [0, 1].
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            CameraEasing::Cut => 0.0, // always return start value, snap at end
            CameraEasing::Linear => t,
            CameraEasing::Smooth => t * t * (3.0 - 2.0 * t), // smoothstep
            CameraEasing::EaseIn => t * t,
            CameraEasing::EaseOut => t * (2.0 - t),
        }
    }
}

/// A single camera keyframe.
#[derive(Debug, Clone)]
pub struct CameraKeyframe {
    /// Time in seconds from start of camera animation.
    pub time: f32,
    /// Camera eye position.
    pub position: Vec3,
    /// Camera look-at target.
    pub target: Vec3,
    /// Field of view in degrees.
    pub fov: f32,
    /// Easing to use when interpolating TO the next keyframe.
    pub easing: CameraEasing,
}

impl CameraKeyframe {
    pub fn new(time: f32, position: Vec3, target: Vec3, fov: f32) -> Self {
        Self {
            time,
            position,
            target,
            fov,
            easing: CameraEasing::Smooth,
        }
    }
}

// ---------------------------------------------------------------------------
// Camera animation track
// ---------------------------------------------------------------------------

/// A camera animation track containing multiple keyframes.
#[derive(Debug, Clone)]
pub struct CameraAnimation {
    /// Name of this camera animation (e.g., "Intro fly-through").
    pub name: String,
    /// Ordered list of camera keyframes (sorted by time).
    pub keyframes: Vec<CameraKeyframe>,
    /// Whether to loop the animation.
    pub looping: bool,
    /// Total duration. If None, uses last keyframe time.
    pub duration: Option<f32>,
}

/// Result of evaluating a camera animation at a given time.
#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub position: Vec3,
    pub target: Vec3,
    pub fov: f32,
}

impl CameraAnimation {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            keyframes: Vec::new(),
            looping: false,
            duration: None,
        }
    }

    /// Total duration of the animation.
    pub fn total_duration(&self) -> f32 {
        self.duration.unwrap_or_else(|| {
            self.keyframes.last().map_or(0.0, |kf| kf.time)
        })
    }

    /// Add a keyframe. Keyframes are kept sorted by time.
    pub fn add_keyframe(&mut self, kf: CameraKeyframe) {
        let time = kf.time;
        let pos = self.keyframes.partition_point(|k| k.time < time);
        self.keyframes.insert(pos, kf);
    }

    /// Remove the keyframe closest to the given time (within threshold).
    pub fn remove_keyframe_at(&mut self, time: f32, threshold: f32) -> bool {
        if let Some(idx) = self.keyframes.iter().position(|k| (k.time - time).abs() < threshold) {
            self.keyframes.remove(idx);
            true
        } else {
            false
        }
    }

    /// Evaluate the camera state at a given time.
    pub fn evaluate(&self, time: f32) -> Option<CameraState> {
        if self.keyframes.is_empty() {
            return None;
        }

        let total = self.total_duration();
        let t = if self.looping && total > 0.0 {
            time % total
        } else {
            time.clamp(0.0, total)
        };

        // Find the two keyframes to interpolate between
        if self.keyframes.len() == 1 {
            let kf = &self.keyframes[0];
            return Some(CameraState {
                position: kf.position,
                target: kf.target,
                fov: kf.fov,
            });
        }

        // Find the segment
        let mut left_idx = 0;
        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.time <= t {
                left_idx = i;
            }
        }

        let right_idx = (left_idx + 1).min(self.keyframes.len() - 1);

        if left_idx == right_idx {
            let kf = &self.keyframes[left_idx];
            return Some(CameraState {
                position: kf.position,
                target: kf.target,
                fov: kf.fov,
            });
        }

        let left = &self.keyframes[left_idx];
        let right = &self.keyframes[right_idx];

        // Compute local parameter
        let segment_duration = right.time - left.time;
        let local_t = if segment_duration > 1e-6 {
            (t - left.time) / segment_duration
        } else {
            1.0
        };

        // Apply easing
        let eased = left.easing.evaluate(local_t);

        // Handle cut — instant transition
        if left.easing == CameraEasing::Cut {
            return Some(CameraState {
                position: left.position,
                target: left.target,
                fov: left.fov,
            });
        }

        // Interpolate
        Some(CameraState {
            position: left.position.lerp(right.position, eased),
            target: left.target.lerp(right.target, eased),
            fov: left.fov + (right.fov - left.fov) * eased,
        })
    }

    /// Number of keyframes.
    pub fn len(&self) -> usize {
        self.keyframes.len()
    }

    /// Is empty.
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Camera animation player
// ---------------------------------------------------------------------------

/// Plays back a camera animation, updating camera state each frame.
#[derive(Debug, Clone)]
pub struct CameraAnimPlayer {
    /// The animation being played.
    pub animation: CameraAnimation,
    /// Current playback time.
    pub time: f32,
    /// Whether playback is active.
    pub playing: bool,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Whether the animation has finished (non-looping only).
    pub finished: bool,
}

impl CameraAnimPlayer {
    pub fn new(animation: CameraAnimation) -> Self {
        Self {
            animation,
            time: 0.0,
            playing: false,
            speed: 1.0,
            finished: false,
        }
    }

    /// Start playback from the beginning.
    pub fn play(&mut self) {
        self.time = 0.0;
        self.playing = true;
        self.finished = false;
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resume playback.
    pub fn resume(&mut self) {
        self.playing = true;
    }

    /// Stop playback and reset to start.
    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
        self.finished = false;
    }

    /// Update playback. Returns the current camera state.
    pub fn update(&mut self, dt: f32) -> Option<CameraState> {
        if self.playing && !self.finished {
            self.time += dt * self.speed;

            let total = self.animation.total_duration();
            if !self.animation.looping && self.time >= total {
                self.time = total;
                self.finished = true;
                self.playing = false;
            }
        }

        self.animation.evaluate(self.time)
    }

    /// Scrub to a specific time without advancing playback.
    pub fn seek(&mut self, time: f32) {
        self.time = time.max(0.0);
        self.finished = false;
    }
}

// ---------------------------------------------------------------------------
// Preset camera animations
// ---------------------------------------------------------------------------

/// Create a simple orbit animation around a point.
pub fn orbit_animation(
    name: &str,
    center: Vec3,
    radius: f32,
    height: f32,
    duration: f32,
    steps: usize,
    fov: f32,
) -> CameraAnimation {
    let mut anim = CameraAnimation::new(name);
    anim.looping = true;
    anim.duration = Some(duration);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = t * std::f32::consts::TAU;
        let pos = Vec3::new(
            center.x + radius * angle.cos(),
            center.y + height,
            center.z + radius * angle.sin(),
        );
        anim.add_keyframe(CameraKeyframe::new(
            t * duration,
            pos,
            center,
            fov,
        ));
    }

    anim
}

/// Create a dolly shot (linear movement between two points).
pub fn dolly_animation(
    name: &str,
    start_pos: Vec3,
    end_pos: Vec3,
    target: Vec3,
    duration: f32,
    fov: f32,
) -> CameraAnimation {
    let mut anim = CameraAnimation::new(name);
    anim.add_keyframe(CameraKeyframe::new(0.0, start_pos, target, fov));
    anim.add_keyframe(CameraKeyframe::new(duration, end_pos, target, fov));
    anim
}

/// Create a zoom shot (FOV change at a fixed position).
pub fn zoom_animation(
    name: &str,
    position: Vec3,
    target: Vec3,
    start_fov: f32,
    end_fov: f32,
    duration: f32,
) -> CameraAnimation {
    let mut anim = CameraAnimation::new(name);
    anim.add_keyframe(CameraKeyframe::new(0.0, position, target, start_fov));
    anim.add_keyframe(CameraKeyframe::new(duration, position, target, end_fov));
    anim
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_keyframe() {
        let mut anim = CameraAnimation::new("test");
        anim.add_keyframe(CameraKeyframe::new(0.0, Vec3::ZERO, Vec3::Z, 45.0));

        let state = anim.evaluate(0.0).unwrap();
        assert!((state.position - Vec3::ZERO).length() < 1e-5);
        assert!((state.fov - 45.0).abs() < 1e-5);
    }

    #[test]
    fn test_linear_interpolation() {
        let mut anim = CameraAnimation::new("test");
        let mut kf1 = CameraKeyframe::new(0.0, Vec3::ZERO, Vec3::Z, 45.0);
        kf1.easing = CameraEasing::Linear;
        anim.add_keyframe(kf1);
        anim.add_keyframe(CameraKeyframe::new(1.0, Vec3::new(10.0, 0.0, 0.0), Vec3::Z, 60.0));

        let state = anim.evaluate(0.5).unwrap();
        assert!((state.position.x - 5.0).abs() < 1e-4, "pos.x = {}", state.position.x);
        assert!((state.fov - 52.5).abs() < 1e-4);
    }

    #[test]
    fn test_smooth_interpolation() {
        let mut anim = CameraAnimation::new("test");
        let mut kf1 = CameraKeyframe::new(0.0, Vec3::ZERO, Vec3::Z, 45.0);
        kf1.easing = CameraEasing::Smooth;
        anim.add_keyframe(kf1);
        anim.add_keyframe(CameraKeyframe::new(1.0, Vec3::X * 10.0, Vec3::Z, 45.0));

        let state = anim.evaluate(0.5).unwrap();
        // Smoothstep at 0.5 = 0.5 (symmetric)
        assert!((state.position.x - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_cut_easing() {
        let mut anim = CameraAnimation::new("test");
        let mut kf1 = CameraKeyframe::new(0.0, Vec3::ZERO, Vec3::Z, 45.0);
        kf1.easing = CameraEasing::Cut;
        anim.add_keyframe(kf1);
        anim.add_keyframe(CameraKeyframe::new(1.0, Vec3::X * 10.0, Vec3::Z, 60.0));

        // During cut, we stay at the left keyframe value
        let state = anim.evaluate(0.5).unwrap();
        assert!((state.position - Vec3::ZERO).length() < 1e-5);
    }

    #[test]
    fn test_looping() {
        let mut anim = CameraAnimation::new("test");
        anim.looping = true;
        let mut kf1 = CameraKeyframe::new(0.0, Vec3::ZERO, Vec3::Z, 45.0);
        kf1.easing = CameraEasing::Linear;
        anim.add_keyframe(kf1);
        anim.add_keyframe(CameraKeyframe::new(2.0, Vec3::X * 10.0, Vec3::Z, 45.0));

        // At t=3.0 with looping, effectively at t=1.0
        let state = anim.evaluate(3.0).unwrap();
        assert!((state.position.x - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_orbit_preset() {
        let anim = orbit_animation("orbit", Vec3::ZERO, 5.0, 2.0, 4.0, 16, 45.0);
        assert_eq!(anim.keyframes.len(), 17); // 0..=16
        assert!(anim.looping);

        let state = anim.evaluate(0.0).unwrap();
        assert!((state.position.y - 2.0).abs() < 1e-4);
    }

    #[test]
    fn test_dolly_preset() {
        let anim = dolly_animation(
            "dolly",
            Vec3::new(0.0, 1.0, -5.0),
            Vec3::new(0.0, 1.0, 5.0),
            Vec3::ZERO,
            3.0,
            45.0,
        );
        assert_eq!(anim.keyframes.len(), 2);
        assert!((anim.total_duration() - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_player_lifecycle() {
        let anim = dolly_animation(
            "test",
            Vec3::ZERO,
            Vec3::X * 10.0,
            Vec3::Z,
            2.0,
            45.0,
        );
        let mut player = CameraAnimPlayer::new(anim);

        assert!(!player.playing);
        player.play();
        assert!(player.playing);

        let state = player.update(1.0).unwrap();
        assert!(state.position.x > 0.0);

        player.pause();
        assert!(!player.playing);

        player.stop();
        assert!((player.time - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_player_finishes() {
        let anim = dolly_animation(
            "test", Vec3::ZERO, Vec3::X, Vec3::Z, 1.0, 45.0,
        );
        let mut player = CameraAnimPlayer::new(anim);
        player.play();

        // Advance past the end
        player.update(2.0);
        assert!(player.finished);
        assert!(!player.playing);
    }

    #[test]
    fn test_add_keyframe_sorted() {
        let mut anim = CameraAnimation::new("test");
        anim.add_keyframe(CameraKeyframe::new(2.0, Vec3::ZERO, Vec3::Z, 45.0));
        anim.add_keyframe(CameraKeyframe::new(0.0, Vec3::X, Vec3::Z, 45.0));
        anim.add_keyframe(CameraKeyframe::new(1.0, Vec3::Y, Vec3::Z, 45.0));

        assert!((anim.keyframes[0].time - 0.0).abs() < 1e-5);
        assert!((anim.keyframes[1].time - 1.0).abs() < 1e-5);
        assert!((anim.keyframes[2].time - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_remove_keyframe() {
        let mut anim = CameraAnimation::new("test");
        anim.add_keyframe(CameraKeyframe::new(0.0, Vec3::ZERO, Vec3::Z, 45.0));
        anim.add_keyframe(CameraKeyframe::new(1.0, Vec3::X, Vec3::Z, 45.0));

        assert!(anim.remove_keyframe_at(0.5, 0.6)); // matches t=1.0
        assert_eq!(anim.keyframes.len(), 1);
    }

    #[test]
    fn test_zoom_preset() {
        let anim = zoom_animation(
            "zoom",
            Vec3::new(0.0, 1.0, 5.0),
            Vec3::ZERO,
            45.0,
            15.0,
            2.0,
        );
        assert_eq!(anim.len(), 2);
    }

    #[test]
    fn test_easing_curves() {
        // All should return 0 at t=0 (except Cut which always returns 0)
        assert!((CameraEasing::Linear.evaluate(0.0) - 0.0).abs() < 1e-5);
        assert!((CameraEasing::Smooth.evaluate(0.0) - 0.0).abs() < 1e-5);
        assert!((CameraEasing::EaseIn.evaluate(0.0) - 0.0).abs() < 1e-5);
        assert!((CameraEasing::EaseOut.evaluate(0.0) - 0.0).abs() < 1e-5);

        // All should return 1 at t=1 (except Cut)
        assert!((CameraEasing::Linear.evaluate(1.0) - 1.0).abs() < 1e-5);
        assert!((CameraEasing::Smooth.evaluate(1.0) - 1.0).abs() < 1e-5);
        assert!((CameraEasing::EaseIn.evaluate(1.0) - 1.0).abs() < 1e-5);
        assert!((CameraEasing::EaseOut.evaluate(1.0) - 1.0).abs() < 1e-5);
    }
}
