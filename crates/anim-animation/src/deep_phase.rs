//! DeepPhase — neural phase manifold for motion transitions.
//!
//! Implements a multi-channel phase representation inspired by the DeepPhase paper.
//! Each bone contributes an oscillating signal decomposed into amplitude, frequency,
//! and phase offset. The phase state is represented as 2D manifold points
//! (A·cos(φ), A·sin(φ)) enabling smooth distance computation for transitions.

use crate::motion::Motion;
use anim_math::transform::Transform;

// ── Configuration ─────────────────────────────────────────────

/// Configuration for DeepPhase extraction.
#[derive(Clone)]
pub struct DeepPhaseConfig {
    /// Number of phase channels (bone groups). Default: 5.
    pub num_channels: usize,
    /// Analysis window size in frames (for local frequency estimation). Default: 60.
    pub window_size: usize,
    /// Minimum detectable frequency in Hz. Default: 0.5.
    pub min_frequency: f32,
    /// Maximum detectable frequency in Hz. Default: 4.0.
    pub max_frequency: f32,
    /// Smoothing factor for amplitude envelope (0..1). Default: 0.1.
    pub amplitude_smoothing: f32,
    /// Number of frequency bins for analysis. Default: 32.
    pub num_freq_bins: usize,
}

impl Default for DeepPhaseConfig {
    fn default() -> Self {
        Self {
            num_channels: 5,
            window_size: 60,
            min_frequency: 0.5,
            max_frequency: 4.0,
            amplitude_smoothing: 0.1,
            num_freq_bins: 32,
        }
    }
}

// ── Channel assignment ─────────────────────────────────────────

/// Standard bone-group channels for phase decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelGroup {
    /// Hips / spine / root motion
    Core,
    /// Left leg chain
    LeftLeg,
    /// Right leg chain
    RightLeg,
    /// Left arm chain
    LeftArm,
    /// Right arm chain
    RightArm,
}

impl ChannelGroup {
    pub fn all() -> &'static [ChannelGroup] {
        &[
            Self::Core,
            Self::LeftLeg,
            Self::RightLeg,
            Self::LeftArm,
            Self::RightArm,
        ]
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Core => 0,
            Self::LeftLeg => 1,
            Self::RightLeg => 2,
            Self::LeftArm => 3,
            Self::RightArm => 4,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Core => "Tronc",
            Self::LeftLeg => "Jambe G",
            Self::RightLeg => "Jambe D",
            Self::LeftArm => "Bras G",
            Self::RightArm => "Bras D",
        }
    }
}

/// Assign joints to channel groups based on name heuristics.
pub fn assign_channels(joint_names: &[String]) -> Vec<usize> {
    joint_names.iter().map(|name| {
        let lower = name.to_lowercase();
        if lower.contains("leg") || lower.contains("foot") || lower.contains("toe")
            || lower.contains("knee") || lower.contains("ankle")
            || lower.contains("jambe") || lower.contains("pied")
            || lower.contains("upleg") || lower.contains("lowleg")
        {
            if lower.contains("left") || lower.contains("_l")
                || lower.ends_with(".l") || lower.contains("gauche")
            {
                ChannelGroup::LeftLeg.index()
            } else if lower.contains("right") || lower.contains("_r")
                || lower.ends_with(".r") || lower.contains("droit")
            {
                ChannelGroup::RightLeg.index()
            } else {
                // Ambiguous leg bone — assign to left by default
                ChannelGroup::LeftLeg.index()
            }
        } else if lower.contains("arm") || lower.contains("hand") || lower.contains("finger")
            || lower.contains("elbow") || lower.contains("wrist") || lower.contains("shoulder")
            || lower.contains("bras") || lower.contains("main") || lower.contains("doigt")
            || lower.contains("forearm") || lower.contains("upperarm")
        {
            if lower.contains("left") || lower.contains("_l")
                || lower.ends_with(".l") || lower.contains("gauche")
            {
                ChannelGroup::LeftArm.index()
            } else if lower.contains("right") || lower.contains("_r")
                || lower.ends_with(".r") || lower.contains("droit")
            {
                ChannelGroup::RightArm.index()
            } else {
                ChannelGroup::LeftArm.index()
            }
        } else {
            // Hips, Spine, Head, Neck, Root — all core
            ChannelGroup::Core.index()
        }
    }).collect()
}

// ── Phase state representation ────────────────────────────────

/// A single frame's phase state in the manifold.
/// Each channel stores a 2D manifold point (A·cos(φ), A·sin(φ)).
#[derive(Clone, Debug)]
pub struct PhaseState {
    /// Manifold points per channel: (x, y) where x = A·cos(φ), y = A·sin(φ).
    pub manifold: Vec<[f32; 2]>,
    /// Raw amplitude per channel.
    pub amplitudes: Vec<f32>,
    /// Raw frequency per channel (Hz).
    pub frequencies: Vec<f32>,
    /// Raw phase offset per channel [0, 2π).
    pub phases: Vec<f32>,
}

impl PhaseState {
    pub fn num_channels(&self) -> usize {
        self.manifold.len()
    }

    /// Euclidean distance between two phase states in the manifold.
    pub fn distance(&self, other: &PhaseState) -> f32 {
        let n = self.manifold.len().min(other.manifold.len());
        let mut sum = 0.0f32;
        for i in 0..n {
            let dx = self.manifold[i][0] - other.manifold[i][0];
            let dy = self.manifold[i][1] - other.manifold[i][1];
            sum += dx * dx + dy * dy;
        }
        sum.sqrt()
    }

    /// Weighted distance with per-channel weights.
    pub fn weighted_distance(&self, other: &PhaseState, weights: &[f32]) -> f32 {
        let n = self.manifold.len().min(other.manifold.len()).min(weights.len());
        let mut sum = 0.0f32;
        for i in 0..n {
            let dx = self.manifold[i][0] - other.manifold[i][0];
            let dy = self.manifold[i][1] - other.manifold[i][1];
            sum += (dx * dx + dy * dy) * weights[i];
        }
        sum.sqrt()
    }
}

// ── Complete manifold for a motion clip ───────────────────────

/// DeepPhase manifold for an entire motion clip.
/// Stores per-frame phase states for all channels.
pub struct DeepPhaseManifold {
    /// Per-frame phase states.
    pub states: Vec<PhaseState>,
    /// Channel assignments (bone index → channel index).
    pub channel_map: Vec<usize>,
    /// Number of channels.
    pub num_channels: usize,
    /// Configuration used for extraction.
    pub config: DeepPhaseConfig,
    /// Per-channel detected dominant frequency.
    pub dominant_frequencies: Vec<f32>,
}

impl DeepPhaseManifold {
    /// Get the phase state at a specific frame.
    pub fn get_state(&self, frame: usize) -> Option<&PhaseState> {
        self.states.get(frame)
    }

    /// Get interpolated phase state at a timestamp.
    pub fn get_state_interpolated(&self, timestamp: f32, framerate: f32) -> Option<PhaseState> {
        if self.states.is_empty() { return None; }
        let t = timestamp * framerate;
        let idx = t.floor() as usize;
        let frac = t - t.floor();

        if idx >= self.states.len() - 1 {
            return self.states.last().cloned();
        }

        let a = &self.states[idx];
        let b = &self.states[idx + 1];
        let nc = a.num_channels();

        // Interpolate manifold points (linear in manifold space)
        let mut manifold = Vec::with_capacity(nc);
        let mut amplitudes = Vec::with_capacity(nc);
        let mut frequencies = Vec::with_capacity(nc);
        let mut phases = Vec::with_capacity(nc);

        for i in 0..nc {
            let mx = a.manifold[i][0] + (b.manifold[i][0] - a.manifold[i][0]) * frac;
            let my = a.manifold[i][1] + (b.manifold[i][1] - a.manifold[i][1]) * frac;
            manifold.push([mx, my]);

            amplitudes.push(a.amplitudes[i] + (b.amplitudes[i] - a.amplitudes[i]) * frac);
            frequencies.push(a.frequencies[i] + (b.frequencies[i] - a.frequencies[i]) * frac);

            // Circular interpolation for phase
            let mut delta = b.phases[i] - a.phases[i];
            if delta > std::f32::consts::PI { delta -= std::f32::consts::TAU; }
            if delta < -std::f32::consts::PI { delta += std::f32::consts::TAU; }
            let p = (a.phases[i] + delta * frac).rem_euclid(std::f32::consts::TAU);
            phases.push(p);
        }

        Some(PhaseState { manifold, amplitudes, frequencies, phases })
    }

    /// Find the frame with the closest phase state to the given target.
    /// Returns (frame_index, distance).
    pub fn find_best_match(&self, target: &PhaseState) -> (usize, f32) {
        let mut best_frame = 0;
        let mut best_dist = f32::MAX;
        for (i, state) in self.states.iter().enumerate() {
            let d = state.distance(target);
            if d < best_dist {
                best_dist = d;
                best_frame = i;
            }
        }
        (best_frame, best_dist)
    }

    /// Find the best match with per-channel weights.
    pub fn find_best_match_weighted(
        &self,
        target: &PhaseState,
        weights: &[f32],
    ) -> (usize, f32) {
        let mut best_frame = 0;
        let mut best_dist = f32::MAX;
        for (i, state) in self.states.iter().enumerate() {
            let d = state.weighted_distance(target, weights);
            if d < best_dist {
                best_dist = d;
                best_frame = i;
            }
        }
        (best_frame, best_dist)
    }

    /// Find the N best matches.
    pub fn find_top_matches(&self, target: &PhaseState, n: usize) -> Vec<(usize, f32)> {
        let mut matches: Vec<(usize, f32)> = self.states.iter()
            .enumerate()
            .map(|(i, s)| (i, s.distance(target)))
            .collect();
        matches.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(n);
        matches
    }

    /// Number of frames.
    pub fn num_frames(&self) -> usize {
        self.states.len()
    }
}

// ── Phase extraction ──────────────────────────────────────────

/// Extract the DeepPhase manifold from a motion clip.
pub fn extract_deep_phase(motion: &Motion, config: DeepPhaseConfig) -> DeepPhaseManifold {
    let n = motion.num_frames();
    let dt = motion.delta_time();
    let framerate = motion.framerate;
    let nc = config.num_channels.min(5).max(1);

    if n < 3 {
        return DeepPhaseManifold {
            states: Vec::new(),
            channel_map: Vec::new(),
            num_channels: nc,
            config,
            dominant_frequencies: vec![0.0; nc],
        };
    }

    // Assign bones to channels
    let channel_map = assign_channels(&motion.hierarchy.bone_names);

    // Step 1: Compute per-bone velocity signals
    let num_joints = motion.hierarchy.bone_names.len();
    let mut bone_velocities: Vec<Vec<f32>> = vec![Vec::with_capacity(n); num_joints];

    for f in 0..n {
        let transforms = &motion.frames[f];
        let prev_transforms = if f > 0 { &motion.frames[f - 1] } else { transforms };

        for j in 0..num_joints.min(transforms.len()) {
            let pos = transforms[j].get_position();
            let prev_pos = if f > 0 && j < prev_transforms.len() {
                prev_transforms[j].get_position()
            } else {
                pos
            };
            let vel = (pos - prev_pos).length() / dt.max(1e-6);
            bone_velocities[j].push(vel);
        }
    }

    // Step 2: Aggregate bone velocities into channel signals
    let mut channel_signals: Vec<Vec<f32>> = vec![vec![0.0; n]; nc];
    let mut channel_bone_counts: Vec<usize> = vec![0; nc];

    for j in 0..num_joints {
        let ch = if j < channel_map.len() { channel_map[j] } else { 0 };
        if ch < nc {
            channel_bone_counts[ch] += 1;
            for f in 0..bone_velocities[j].len().min(n) {
                channel_signals[ch][f] += bone_velocities[j][f];
            }
        }
    }

    // Normalize by bone count
    for ch in 0..nc {
        let count = channel_bone_counts[ch].max(1) as f32;
        for f in 0..n {
            channel_signals[ch][f] /= count;
        }
    }

    // Step 3: Extract amplitude, frequency, and phase per channel per frame
    let half_window = config.window_size / 2;
    let mut dominant_frequencies = vec![1.0f32; nc];

    let mut channel_amplitudes: Vec<Vec<f32>> = vec![vec![0.0; n]; nc];
    let mut channel_frequencies: Vec<Vec<f32>> = vec![vec![1.0; n]; nc];
    let mut channel_phases: Vec<Vec<f32>> = vec![vec![0.0; n]; nc];

    for ch in 0..nc {
        let signal = &channel_signals[ch];

        // Compute global dominant frequency for this channel using autocorrelation
        let global_freq = estimate_frequency_autocorrelation(
            signal, framerate,
            config.min_frequency, config.max_frequency,
        );
        dominant_frequencies[ch] = global_freq;

        // Per-frame: compute local amplitude and phase
        for f in 0..n {
            let start = f.saturating_sub(half_window);
            let end = (f + half_window).min(n);
            let window = &signal[start..end];

            // Amplitude: RMS of the window
            let mean: f32 = window.iter().sum::<f32>() / window.len() as f32;
            let rms = (window.iter()
                .map(|&v| (v - mean) * (v - mean))
                .sum::<f32>() / window.len() as f32)
                .sqrt();

            // Phase: use the analytic signal approach (simplified)
            // Compute phase from signal position in the cycle
            let freq = global_freq;
            let period_frames = if freq > 0.0 { framerate / freq } else { n as f32 };
            let raw_phase = (f as f32 / period_frames).fract() * std::f32::consts::TAU;

            // Refine phase using local signal shape (zero-crossing detection)
            let refined_phase = refine_phase(signal, f, raw_phase, period_frames);

            channel_amplitudes[ch][f] = rms;
            channel_frequencies[ch][f] = freq;
            channel_phases[ch][f] = refined_phase;
        }

        // Smooth amplitudes
        smooth_signal(&mut channel_amplitudes[ch], config.amplitude_smoothing);
    }

    // Step 4: Build manifold states
    let mut states = Vec::with_capacity(n);
    for f in 0..n {
        let mut manifold = Vec::with_capacity(nc);
        let mut amplitudes = Vec::with_capacity(nc);
        let mut frequencies = Vec::with_capacity(nc);
        let mut phases = Vec::with_capacity(nc);

        for ch in 0..nc {
            let a = channel_amplitudes[ch][f];
            let phi = channel_phases[ch][f];

            // Manifold point: (A·cos(φ), A·sin(φ))
            manifold.push([a * phi.cos(), a * phi.sin()]);
            amplitudes.push(a);
            frequencies.push(channel_frequencies[ch][f]);
            phases.push(phi);
        }

        states.push(PhaseState { manifold, amplitudes, frequencies, phases });
    }

    DeepPhaseManifold {
        states,
        channel_map,
        num_channels: nc,
        config,
        dominant_frequencies,
    }
}

// ── Internal helper functions ─────────────────────────────────

/// Estimate dominant frequency using autocorrelation.
/// Finds the first strong positive peak in the autocorrelation function,
/// which corresponds to the fundamental period.
fn estimate_frequency_autocorrelation(
    signal: &[f32],
    framerate: f32,
    min_freq: f32,
    max_freq: f32,
) -> f32 {
    let n = signal.len();
    if n < 4 { return 1.0; }

    // Remove DC component
    let mean: f32 = signal.iter().sum::<f32>() / n as f32;

    // Compute lag range from frequency range
    let min_lag = (framerate / max_freq).ceil() as usize;
    let max_lag = (framerate / min_freq).floor() as usize;
    let max_lag = max_lag.min(n / 2);

    if min_lag >= max_lag { return 1.0; }

    // Compute normalized autocorrelation for all lags
    let mut correlations: Vec<f32> = Vec::with_capacity(max_lag - min_lag + 1);
    for lag in min_lag..=max_lag {
        let mut corr = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;
        let count = n - lag;

        for i in 0..count {
            let a = signal[i] - mean;
            let b = signal[i + lag] - mean;
            corr += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }

        let denom = (norm_a * norm_b).sqrt();
        let normalized = if denom > 1e-8 { corr / denom } else { 0.0 };
        correlations.push(normalized);
    }

    // Find the first strong peak: a local maximum above threshold.
    // This gives us the fundamental period (shortest repeating unit).
    let threshold = 0.3;
    let num_lags = correlations.len();

    for i in 1..num_lags.saturating_sub(1) {
        if correlations[i] > threshold
            && correlations[i] >= correlations[i - 1]
            && correlations[i] >= correlations[i + 1]
        {
            let lag = min_lag + i;
            return framerate / lag as f32;
        }
    }

    // Fallback: find global maximum
    let mut best_idx = 0;
    let mut best_val = f32::MIN;
    for (i, &c) in correlations.iter().enumerate() {
        if c > best_val {
            best_val = c;
            best_idx = i;
        }
    }

    if best_val > 0.1 {
        framerate / (min_lag + best_idx) as f32
    } else {
        (min_freq + max_freq) * 0.5
    }
}

/// Refine phase estimation using local signal shape.
fn refine_phase(signal: &[f32], frame: usize, raw_phase: f32, period_frames: f32) -> f32 {
    let n = signal.len();
    if n < 3 || frame == 0 || frame >= n - 1 {
        return raw_phase;
    }

    // Look for nearest zero-crossing in the mean-subtracted signal
    let half_period = (period_frames * 0.5) as usize;
    let search_start = frame.saturating_sub(half_period);
    let search_end = (frame + half_period).min(n);

    let local_window = &signal[search_start..search_end];
    let local_mean: f32 = local_window.iter().sum::<f32>() / local_window.len() as f32;

    // Find nearest ascending zero-crossing
    let mut nearest_crossing: Option<usize> = None;
    let mut nearest_dist = usize::MAX;
    for i in (search_start + 1)..search_end {
        let prev_val = signal[i - 1] - local_mean;
        let curr_val = signal[i] - local_mean;
        if prev_val <= 0.0 && curr_val > 0.0 {
            // Ascending zero-crossing
            let dist = if i > frame { i - frame } else { frame - i };
            if dist < nearest_dist {
                nearest_dist = dist;
                nearest_crossing = Some(i);
            }
        }
    }

    if let Some(crossing) = nearest_crossing {
        // Phase at crossing is 0 (or 2π). Offset from there.
        let offset_frames = if frame >= crossing {
            (frame - crossing) as f32
        } else {
            period_frames - (crossing - frame) as f32
        };
        (offset_frames / period_frames * std::f32::consts::TAU)
            .rem_euclid(std::f32::consts::TAU)
    } else {
        raw_phase
    }
}

/// In-place exponential moving average smoothing.
fn smooth_signal(signal: &mut [f32], alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    if signal.len() < 2 || alpha == 0.0 { return; }

    // Forward pass
    for i in 1..signal.len() {
        signal[i] = signal[i - 1] * (1.0 - alpha) + signal[i] * alpha;
    }
    // Backward pass for zero-phase filtering
    for i in (0..signal.len() - 1).rev() {
        signal[i] = signal[i + 1] * (1.0 - alpha) + signal[i] * alpha;
    }
}

// ── Phase-aware transition scoring ────────────────────────────

/// Score how well two clips transition at given frames based on phase alignment.
/// Lower score = better transition.
pub fn transition_score(
    manifold_a: &DeepPhaseManifold,
    frame_a: usize,
    manifold_b: &DeepPhaseManifold,
    frame_b: usize,
) -> f32 {
    let state_a = match manifold_a.get_state(frame_a) {
        Some(s) => s,
        None => return f32::MAX,
    };
    let state_b = match manifold_b.get_state(frame_b) {
        Some(s) => s,
        None => return f32::MAX,
    };
    state_a.distance(state_b)
}

/// Find the best frame in clip B to transition from frame_a of clip A.
/// Returns (best_frame_b, score).
pub fn find_best_transition(
    manifold_a: &DeepPhaseManifold,
    frame_a: usize,
    manifold_b: &DeepPhaseManifold,
) -> (usize, f32) {
    let state_a = match manifold_a.get_state(frame_a) {
        Some(s) => s,
        None => return (0, f32::MAX),
    };
    manifold_b.find_best_match(state_a)
}

/// Find transition candidates within a score threshold.
pub fn find_transition_candidates(
    manifold_a: &DeepPhaseManifold,
    frame_a: usize,
    manifold_b: &DeepPhaseManifold,
    max_score: f32,
) -> Vec<(usize, f32)> {
    let state_a = match manifold_a.get_state(frame_a) {
        Some(s) => s,
        None => return Vec::new(),
    };

    manifold_b.states.iter()
        .enumerate()
        .map(|(i, s)| (i, s.distance(state_a)))
        .filter(|&(_, d)| d <= max_score)
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_state_distance() {
        let a = PhaseState {
            manifold: vec![[1.0, 0.0], [0.0, 1.0]],
            amplitudes: vec![1.0, 1.0],
            frequencies: vec![1.0, 1.0],
            phases: vec![0.0, std::f32::consts::FRAC_PI_2],
        };
        let b = PhaseState {
            manifold: vec![[0.0, 1.0], [1.0, 0.0]],
            amplitudes: vec![1.0, 1.0],
            frequencies: vec![1.0, 1.0],
            phases: vec![std::f32::consts::FRAC_PI_2, 0.0],
        };
        let d = a.distance(&b);
        // Each channel has distance sqrt(2), total = sqrt(2 + 2) = 2
        assert!((d - 2.0).abs() < 0.01, "got {}", d);
    }

    #[test]
    fn test_phase_state_self_distance() {
        let a = PhaseState {
            manifold: vec![[0.5, 0.3], [-0.2, 0.8], [1.0, 0.0]],
            amplitudes: vec![0.6, 0.83, 1.0],
            frequencies: vec![2.0, 1.5, 1.0],
            phases: vec![0.5, 1.2, 0.0],
        };
        assert!(a.distance(&a) < 1e-6);
    }

    #[test]
    fn test_weighted_distance() {
        let a = PhaseState {
            manifold: vec![[1.0, 0.0], [0.0, 0.0]],
            amplitudes: vec![1.0, 0.0],
            frequencies: vec![1.0, 1.0],
            phases: vec![0.0, 0.0],
        };
        let b = PhaseState {
            manifold: vec![[0.0, 0.0], [1.0, 0.0]],
            amplitudes: vec![0.0, 1.0],
            frequencies: vec![1.0, 1.0],
            phases: vec![0.0, 0.0],
        };
        // With uniform weights
        let d1 = a.weighted_distance(&b, &[1.0, 1.0]);
        // With only first channel weighted
        let d2 = a.weighted_distance(&b, &[1.0, 0.0]);
        assert!(d2 < d1);
        assert!((d2 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_assign_channels() {
        let names = vec![
            "Hips".to_string(),
            "Spine".to_string(),
            "LeftUpLeg".to_string(),
            "LeftLeg".to_string(),
            "LeftFoot".to_string(),
            "RightUpLeg".to_string(),
            "RightLeg".to_string(),
            "RightFoot".to_string(),
            "LeftArm".to_string(),
            "LeftHand".to_string(),
            "RightArm".to_string(),
            "RightHand".to_string(),
            "Head".to_string(),
        ];
        let channels = assign_channels(&names);
        assert_eq!(channels[0], ChannelGroup::Core.index());      // Hips
        assert_eq!(channels[1], ChannelGroup::Core.index());      // Spine
        assert_eq!(channels[2], ChannelGroup::LeftLeg.index());   // LeftUpLeg
        assert_eq!(channels[5], ChannelGroup::RightLeg.index());  // RightUpLeg
        assert_eq!(channels[8], ChannelGroup::LeftArm.index());   // LeftArm
        assert_eq!(channels[10], ChannelGroup::RightArm.index()); // RightArm
        assert_eq!(channels[12], ChannelGroup::Core.index());     // Head
    }

    #[test]
    fn test_autocorrelation_sinusoid() {
        // Generate a known sinusoidal signal at 2 Hz, 60 fps
        let framerate = 60.0;
        let freq = 2.0;
        let n = 600; // 10 seconds — plenty of cycles
        let signal: Vec<f32> = (0..n)
            .map(|i| (i as f32 / framerate * freq * std::f32::consts::TAU).sin())
            .collect();

        let detected = estimate_frequency_autocorrelation(&signal, framerate, 1.0, 4.0);
        // Should be close to 2 Hz
        assert!((detected - 2.0).abs() < 0.3, "detected freq: {}", detected);
    }

    #[test]
    fn test_smooth_signal() {
        let mut signal = vec![0.0, 10.0, 0.0, 10.0, 0.0];
        smooth_signal(&mut signal, 0.5);
        // After smoothing, the signal should have reduced variation
        let range = signal.iter().cloned().fold(f32::MIN, f32::max)
            - signal.iter().cloned().fold(f32::MAX, f32::min);
        assert!(range < 10.0, "range after smoothing: {}", range);
    }

    #[test]
    fn test_manifold_find_best_match() {
        // Create a simple manifold with 3 states
        let states = vec![
            PhaseState {
                manifold: vec![[1.0, 0.0]],
                amplitudes: vec![1.0],
                frequencies: vec![1.0],
                phases: vec![0.0],
            },
            PhaseState {
                manifold: vec![[0.0, 1.0]],
                amplitudes: vec![1.0],
                frequencies: vec![1.0],
                phases: vec![std::f32::consts::FRAC_PI_2],
            },
            PhaseState {
                manifold: vec![[-1.0, 0.0]],
                amplitudes: vec![1.0],
                frequencies: vec![1.0],
                phases: vec![std::f32::consts::PI],
            },
        ];

        let manifold = DeepPhaseManifold {
            states,
            channel_map: vec![0],
            num_channels: 1,
            config: DeepPhaseConfig::default(),
            dominant_frequencies: vec![1.0],
        };

        // Query close to state 1
        let query = PhaseState {
            manifold: vec![[0.1, 0.9]],
            amplitudes: vec![0.9],
            frequencies: vec![1.0],
            phases: vec![std::f32::consts::FRAC_PI_2],
        };

        let (best, dist) = manifold.find_best_match(&query);
        assert_eq!(best, 1); // Should match state 1 (0, 1)
        assert!(dist < 0.2);
    }
}
