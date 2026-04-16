//! Animation Recorder — capture live transforms into keyframe clips.
//!
//! Records per-joint transforms each frame from ragdoll simulation,
//! IK adjustments, or manual pose editing. Produces a Motion clip
//! that can be played back or exported.

use glam::Mat4;

/// Recording state.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
}

/// Configuration for the animation recorder.
#[derive(Clone)]
pub struct RecorderConfig {
    /// Target framerate for the recording. Default: 30.0.
    pub framerate: f32,
    /// Maximum recording duration in seconds (0 = unlimited). Default: 60.0.
    pub max_duration: f32,
    /// Whether to record every frame or sub-sample. Default: true.
    pub record_every_frame: bool,
    /// Sub-sample interval in seconds (when record_every_frame is false). Default: 1/30.
    pub sample_interval: f32,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            framerate: 30.0,
            max_duration: 60.0,
            record_every_frame: true,
            sample_interval: 1.0 / 30.0,
        }
    }
}

/// A recorded animation clip (raw captured data).
pub struct RecordedClip {
    /// Recorded frames: [frame_index][joint_index] = Mat4 (global space).
    pub frames: Vec<Vec<Mat4>>,
    /// Framerate of the recording.
    pub framerate: f32,
    /// Name of the clip.
    pub name: String,
    /// Number of joints per frame.
    pub num_joints: usize,
    /// Total duration in seconds.
    pub duration: f32,
}

impl RecordedClip {
    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// The animation recorder.
pub struct AnimRecorder {
    pub state: RecordingState,
    pub config: RecorderConfig,
    /// Accumulated frames during recording.
    captured_frames: Vec<Vec<Mat4>>,
    /// Number of joints (set when recording starts).
    num_joints: usize,
    /// Time accumulator for sub-sampling.
    accumulator: f32,
    /// Total elapsed recording time.
    elapsed: f32,
    /// Number of frames captured so far.
    frame_count: usize,
}

impl Default for AnimRecorder {
    fn default() -> Self {
        Self {
            state: RecordingState::Idle,
            config: RecorderConfig::default(),
            captured_frames: Vec::new(),
            num_joints: 0,
            accumulator: 0.0,
            elapsed: 0.0,
            frame_count: 0,
        }
    }
}

impl AnimRecorder {
    pub fn new(config: RecorderConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Start recording. `num_joints` is the number of joints to capture per frame.
    pub fn start(&mut self, num_joints: usize) {
        self.captured_frames.clear();
        self.num_joints = num_joints;
        self.accumulator = 0.0;
        self.elapsed = 0.0;
        self.frame_count = 0;
        self.state = RecordingState::Recording;
    }

    /// Pause the recording (can resume later).
    pub fn pause(&mut self) {
        if self.state == RecordingState::Recording {
            self.state = RecordingState::Paused;
        }
    }

    /// Resume a paused recording.
    pub fn resume(&mut self) {
        if self.state == RecordingState::Paused {
            self.state = RecordingState::Recording;
        }
    }

    /// Stop recording and return the captured clip. Returns None if nothing was recorded.
    pub fn stop(&mut self) -> Option<RecordedClip> {
        if self.captured_frames.is_empty() {
            self.state = RecordingState::Idle;
            return None;
        }

        let clip = RecordedClip {
            frames: std::mem::take(&mut self.captured_frames),
            framerate: self.config.framerate,
            name: format!("Recording_{}", self.frame_count),
            num_joints: self.num_joints,
            duration: self.elapsed,
        };

        self.state = RecordingState::Idle;
        self.num_joints = 0;
        self.accumulator = 0.0;
        self.elapsed = 0.0;
        self.frame_count = 0;

        Some(clip)
    }

    /// Cancel recording and discard all captured data.
    pub fn cancel(&mut self) {
        self.captured_frames.clear();
        self.state = RecordingState::Idle;
        self.num_joints = 0;
        self.accumulator = 0.0;
        self.elapsed = 0.0;
        self.frame_count = 0;
    }

    /// Feed a frame of transforms to the recorder.
    /// `transforms` should contain `num_joints` Mat4 transforms (global space).
    /// `dt` is the delta time since the last call.
    /// Returns true if a frame was actually captured (sub-sampling may skip frames).
    pub fn capture_frame(&mut self, transforms: &[Mat4], dt: f32) -> bool {
        if self.state != RecordingState::Recording {
            return false;
        }

        self.elapsed += dt;

        // Check max duration
        if self.config.max_duration > 0.0 && self.elapsed >= self.config.max_duration {
            return false;
        }

        if self.config.record_every_frame {
            self.store_frame(transforms);
            return true;
        }

        // Sub-sampling mode
        self.accumulator += dt;
        if self.accumulator >= self.config.sample_interval {
            self.accumulator -= self.config.sample_interval;
            self.store_frame(transforms);
            return true;
        }

        false
    }

    fn store_frame(&mut self, transforms: &[Mat4]) {
        let frame: Vec<Mat4> = transforms.iter()
            .take(self.num_joints)
            .copied()
            .collect();
        self.captured_frames.push(frame);
        self.frame_count += 1;
    }

    /// Current recording time in seconds.
    pub fn elapsed_time(&self) -> f32 {
        self.elapsed
    }

    /// Number of frames captured so far.
    pub fn captured_frame_count(&self) -> usize {
        self.frame_count
    }

    /// Whether the recorder is actively recording.
    pub fn is_recording(&self) -> bool {
        self.state == RecordingState::Recording
    }

    /// Whether the recorder is paused.
    pub fn is_paused(&self) -> bool {
        self.state == RecordingState::Paused
    }

    /// Whether the recorder is idle (not recording).
    pub fn is_idle(&self) -> bool {
        self.state == RecordingState::Idle
    }
}

/// Convert a RecordedClip into a Motion (for playback/export).
/// This is a convenience function that bridges the recorder output
/// to the animation system.
pub fn clip_to_motion_data(
    clip: &RecordedClip,
    _joint_names: &[String],
    _parent_indices: &[i32],
) -> (Vec<Vec<Mat4>>, f32) {
    // The clip frames are already in global space, same as Motion expects
    (clip.frames.clone(), clip.framerate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_lifecycle() {
        let mut rec = AnimRecorder::default();
        assert!(rec.is_idle());

        rec.start(3);
        assert!(rec.is_recording());
        assert_eq!(rec.captured_frame_count(), 0);

        // Capture 5 frames
        let frame = vec![Mat4::IDENTITY; 3];
        for _ in 0..5 {
            rec.capture_frame(&frame, 1.0 / 30.0);
        }
        assert_eq!(rec.captured_frame_count(), 5);

        // Pause
        rec.pause();
        assert!(rec.is_paused());
        rec.capture_frame(&frame, 1.0 / 30.0);
        assert_eq!(rec.captured_frame_count(), 5); // No capture while paused

        // Resume
        rec.resume();
        rec.capture_frame(&frame, 1.0 / 30.0);
        assert_eq!(rec.captured_frame_count(), 6);

        // Stop
        let clip = rec.stop().unwrap();
        assert_eq!(clip.num_frames(), 6);
        assert_eq!(clip.num_joints, 3);
        assert!(rec.is_idle());
    }

    #[test]
    fn test_recorder_cancel() {
        let mut rec = AnimRecorder::default();
        rec.start(2);

        let frame = vec![Mat4::IDENTITY; 2];
        rec.capture_frame(&frame, 1.0 / 30.0);
        assert_eq!(rec.captured_frame_count(), 1);

        rec.cancel();
        assert!(rec.is_idle());
        assert_eq!(rec.captured_frame_count(), 0);
    }

    #[test]
    fn test_recorder_subsampling() {
        let config = RecorderConfig {
            framerate: 30.0,
            record_every_frame: false,
            sample_interval: 0.1, // capture every 100ms
            ..Default::default()
        };
        let mut rec = AnimRecorder::new(config);
        rec.start(1);

        let frame = vec![Mat4::IDENTITY];

        // Feed frames at 60fps (dt = 16.6ms) for 0.5 seconds (30 frames)
        let dt = 1.0 / 60.0;
        let mut captured = 0;
        for _ in 0..30 {
            if rec.capture_frame(&frame, dt) {
                captured += 1;
            }
        }

        // At 100ms intervals over 0.5s, we expect ~5 captures
        assert!(captured >= 4 && captured <= 6, "captured {} frames", captured);
    }

    #[test]
    fn test_recorder_max_duration() {
        let config = RecorderConfig {
            max_duration: 0.1, // 100ms max
            ..Default::default()
        };
        let mut rec = AnimRecorder::new(config);
        rec.start(1);

        let frame = vec![Mat4::IDENTITY];
        let dt = 1.0 / 30.0;

        // Try to record 1 second (30 frames at 30fps)
        let mut count = 0;
        for _ in 0..30 {
            if rec.capture_frame(&frame, dt) {
                count += 1;
            }
        }

        // Should have stopped after ~3 frames (100ms at 33ms/frame)
        assert!(count <= 4, "captured {} frames (expected <=4)", count);
    }

    #[test]
    fn test_empty_stop() {
        let mut rec = AnimRecorder::default();
        rec.start(1);
        let clip = rec.stop();
        assert!(clip.is_none());
    }
}
