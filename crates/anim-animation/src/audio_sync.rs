//! Audio synchronization system for animation.
//!
//! Provides WAV parsing, beat/onset detection, energy-based lip sync, and a
//! playback controller that ties audio events to animation time.

// ---------------------------------------------------------------------------
// Frequency band classification
// ---------------------------------------------------------------------------

/// Frequency band for onset classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqBand {
    Low,
    Mid,
    High,
    Full,
}

// ---------------------------------------------------------------------------
// AudioOnset
// ---------------------------------------------------------------------------

/// A single detected onset event.
#[derive(Debug, Clone)]
pub struct AudioOnset {
    /// Time of the onset in seconds.
    pub time: f32,
    /// Onset strength in [0.0, 1.0].
    pub strength: f32,
    /// Dominant frequency band.
    pub frequency_band: FreqBand,
}

// ---------------------------------------------------------------------------
// AudioClip — loaded audio data
// ---------------------------------------------------------------------------

/// A loaded audio clip stored as mono f32 samples.
#[derive(Debug, Clone)]
pub struct AudioClip {
    /// Friendly name.
    pub name: String,
    /// Mono audio samples normalised to [-1.0, 1.0].
    pub samples: Vec<f32>,
    /// Sample rate in Hz (e.g. 44100).
    pub sample_rate: u32,
    /// Total duration in seconds.
    pub duration: f32,
    /// Original channel count before downmix.
    pub channels: u16,
}

impl AudioClip {
    // -- WAV helpers ---------------------------------------------------------

    fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, String> {
        if offset + 2 > data.len() {
            return Err("Unexpected end of WAV data (u16)".into());
        }
        Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
    }

    fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, String> {
        if offset + 4 > data.len() {
            return Err("Unexpected end of WAV data (u32)".into());
        }
        Ok(u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]))
    }

    fn read_i16_le(data: &[u8], offset: usize) -> Result<i16, String> {
        if offset + 2 > data.len() {
            return Err("Unexpected end of WAV data (i16)".into());
        }
        Ok(i16::from_le_bytes([data[offset], data[offset + 1]]))
    }

    fn read_f32_le(data: &[u8], offset: usize) -> Result<f32, String> {
        if offset + 4 > data.len() {
            return Err("Unexpected end of WAV data (f32)".into());
        }
        Ok(f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]))
    }

    /// Parse a WAV file from raw bytes.
    ///
    /// Supports RIFF/WAVE with PCM 16-bit (format tag 1) and IEEE float
    /// 32-bit (format tag 3), mono or stereo.  Stereo is downmixed to mono
    /// by averaging the two channels.
    pub fn from_wav(data: &[u8]) -> Result<Self, String> {
        // -- RIFF header -----------------------------------------------------
        if data.len() < 44 {
            return Err("Data too short to be a valid WAV file".into());
        }
        if &data[0..4] != b"RIFF" {
            return Err("Missing RIFF header".into());
        }
        if &data[8..12] != b"WAVE" {
            return Err("Missing WAVE identifier".into());
        }

        // -- Walk chunks -----------------------------------------------------
        let mut pos: usize = 12;
        let mut fmt_found = false;
        let mut audio_format: u16 = 0;
        let mut num_channels: u16 = 0;
        let mut sample_rate: u32 = 0;
        let mut bits_per_sample: u16 = 0;
        let mut samples: Vec<f32> = Vec::new();

        while pos + 8 <= data.len() {
            let chunk_id = &data[pos..pos + 4];
            let chunk_size = Self::read_u32_le(data, pos + 4)? as usize;
            let chunk_data_start = pos + 8;

            if chunk_id == b"fmt " {
                if chunk_size < 16 {
                    return Err("fmt chunk too small".into());
                }
                audio_format = Self::read_u16_le(data, chunk_data_start)?;
                num_channels = Self::read_u16_le(data, chunk_data_start + 2)?;
                sample_rate = Self::read_u32_le(data, chunk_data_start + 4)?;
                // bytes_per_sec @ +8, block_align @ +12 — skipped
                bits_per_sample = Self::read_u16_le(data, chunk_data_start + 14)?;

                if audio_format != 1 && audio_format != 3 {
                    return Err(format!(
                        "Unsupported audio format tag {audio_format} (expected PCM=1 or IEEE float=3)"
                    ));
                }
                if num_channels == 0 || num_channels > 2 {
                    return Err(format!(
                        "Unsupported channel count {num_channels} (expected 1 or 2)"
                    ));
                }
                if audio_format == 1 && bits_per_sample != 16 {
                    return Err(format!(
                        "Unsupported PCM bit depth {bits_per_sample} (expected 16)"
                    ));
                }
                if audio_format == 3 && bits_per_sample != 32 {
                    return Err(format!(
                        "Unsupported float bit depth {bits_per_sample} (expected 32)"
                    ));
                }
                fmt_found = true;
            } else if chunk_id == b"data" {
                if !fmt_found {
                    return Err("data chunk before fmt chunk".into());
                }
                let end = (chunk_data_start + chunk_size).min(data.len());
                let raw = &data[chunk_data_start..end];

                match (audio_format, bits_per_sample) {
                    // PCM 16-bit
                    (1, 16) => {
                        let frame_bytes = 2 * num_channels as usize;
                        let num_frames = raw.len() / frame_bytes;
                        samples.reserve(num_frames);
                        for f in 0..num_frames {
                            let base = f * frame_bytes;
                            if num_channels == 1 {
                                let s = Self::read_i16_le(raw, base)
                                    .unwrap_or(0) as f32
                                    / 32768.0;
                                samples.push(s);
                            } else {
                                let l = Self::read_i16_le(raw, base)
                                    .unwrap_or(0) as f32
                                    / 32768.0;
                                let r = Self::read_i16_le(raw, base + 2)
                                    .unwrap_or(0) as f32
                                    / 32768.0;
                                samples.push((l + r) * 0.5);
                            }
                        }
                    }
                    // IEEE float 32-bit
                    (3, 32) => {
                        let frame_bytes = 4 * num_channels as usize;
                        let num_frames = raw.len() / frame_bytes;
                        samples.reserve(num_frames);
                        for f in 0..num_frames {
                            let base = f * frame_bytes;
                            if num_channels == 1 {
                                let s = Self::read_f32_le(raw, base).unwrap_or(0.0);
                                samples.push(s);
                            } else {
                                let l = Self::read_f32_le(raw, base).unwrap_or(0.0);
                                let r = Self::read_f32_le(raw, base + 4).unwrap_or(0.0);
                                samples.push((l + r) * 0.5);
                            }
                        }
                    }
                    _ => {
                        return Err("Unhandled format / bit-depth combination".into());
                    }
                }
            }

            // Advance to next chunk (chunks are word-aligned).
            let advance = chunk_size + if chunk_size % 2 != 0 { 1 } else { 0 };
            pos = chunk_data_start + advance;
        }

        if !fmt_found {
            return Err("No fmt chunk found".into());
        }
        if samples.is_empty() {
            return Err("No audio data found".into());
        }

        let duration = samples.len() as f32 / sample_rate as f32;

        Ok(Self {
            name: String::new(),
            samples,
            sample_rate,
            duration,
            channels: num_channels,
        })
    }

    // -- Sample-level queries -----------------------------------------------

    /// Index into the sample buffer for a given time.
    fn time_to_index(&self, time: f32) -> usize {
        let idx = (time * self.sample_rate as f32) as usize;
        idx.min(self.samples.len().saturating_sub(1))
    }

    /// Absolute amplitude at the given time (nearest-sample lookup).
    pub fn amplitude_at(&self, time: f32) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let idx = self.time_to_index(time.clamp(0.0, self.duration));
        self.samples[idx].abs()
    }

    /// Root-mean-square energy over a time window.
    pub fn rms_window(&self, start: f32, duration: f32) -> f32 {
        if self.samples.is_empty() || duration <= 0.0 {
            return 0.0;
        }
        let i0 = self.time_to_index(start.max(0.0));
        let i1 = self.time_to_index((start + duration).min(self.duration));
        if i0 >= i1 {
            return 0.0;
        }
        let sum: f32 = self.samples[i0..=i1]
            .iter()
            .map(|s| s * s)
            .sum();
        let count = (i1 - i0 + 1) as f32;
        (sum / count).sqrt()
    }

    /// Peak absolute amplitude in [start, end].
    pub fn peak_in_range(&self, start: f32, end: f32) -> f32 {
        if self.samples.is_empty() || end <= start {
            return 0.0;
        }
        let i0 = self.time_to_index(start.max(0.0));
        let i1 = self.time_to_index(end.min(self.duration));
        if i0 > i1 {
            return 0.0;
        }
        self.samples[i0..=i1]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max)
    }

    /// Downsample the waveform to `num_points` values for UI display.
    ///
    /// Each point is the peak amplitude within its window.
    pub fn downsample_waveform(&self, num_points: usize) -> Vec<f32> {
        if num_points == 0 || self.samples.is_empty() {
            return Vec::new();
        }
        let n = self.samples.len();
        let mut out = Vec::with_capacity(num_points);
        for i in 0..num_points {
            let start = i * n / num_points;
            let end = ((i + 1) * n / num_points).min(n);
            let peak = self.samples[start..end]
                .iter()
                .map(|s| s.abs())
                .fold(0.0_f32, f32::max);
            out.push(peak);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// BeatDetector
// ---------------------------------------------------------------------------

/// Simple energy-based beat / onset detector.
#[derive(Debug, Clone)]
pub struct BeatDetector {
    /// Onset detection threshold in [0.0, 1.0].
    pub threshold: f32,
    /// Minimum interval between consecutive beats (seconds).
    pub min_interval: f32,
}

impl BeatDetector {
    pub fn new(threshold: f32, min_interval: f32) -> Self {
        Self { threshold, min_interval }
    }

    /// Detect beat times using a simple energy-envelope onset detector.
    ///
    /// The algorithm slides a short analysis window across the clip, computes
    /// RMS energy, and marks a beat wherever the energy exceeds the running
    /// mean by more than `threshold`.
    pub fn detect_beats(&self, clip: &AudioClip) -> Vec<f32> {
        if clip.samples.is_empty() {
            return Vec::new();
        }

        let window_samples = (clip.sample_rate as f32 * 0.02) as usize; // 20 ms
        let hop = window_samples / 2;
        if window_samples == 0 || hop == 0 {
            return Vec::new();
        }

        // Compute per-frame energy.
        let num_frames = (clip.samples.len().saturating_sub(window_samples)) / hop + 1;
        let mut energies: Vec<f32> = Vec::with_capacity(num_frames);
        for f in 0..num_frames {
            let start = f * hop;
            let end = (start + window_samples).min(clip.samples.len());
            let rms: f32 = clip.samples[start..end]
                .iter()
                .map(|s| s * s)
                .sum::<f32>()
                / (end - start) as f32;
            energies.push(rms.sqrt());
        }

        // Running mean with a ~200 ms look-back.
        let mean_frames = ((0.2 * clip.sample_rate as f32) / hop as f32).max(1.0) as usize;
        let mut beats: Vec<f32> = Vec::new();
        let mut last_beat_time: f32 = -self.min_interval;

        for (i, &e) in energies.iter().enumerate() {
            let look_start = i.saturating_sub(mean_frames);
            let local_mean: f32 =
                energies[look_start..=i].iter().copied().sum::<f32>()
                    / (i - look_start + 1) as f32;

            if e > local_mean + self.threshold {
                let time = (i * hop) as f32 / clip.sample_rate as f32;
                if time - last_beat_time >= self.min_interval {
                    beats.push(time);
                    last_beat_time = time;
                }
            }
        }

        beats
    }

    /// Detect onsets with strength and rough frequency-band classification.
    pub fn detect_onsets(&self, clip: &AudioClip) -> Vec<AudioOnset> {
        let beat_times = self.detect_beats(clip);
        let window_dur = 0.02_f32;

        beat_times
            .iter()
            .map(|&time| {
                let strength = clip.rms_window(time, window_dur).min(1.0);

                // Rough band classification by comparing low-pass vs high-pass
                // energy around the onset.
                let idx = clip.time_to_index(time);
                let half_win = (clip.sample_rate as usize / 100).max(1); // ~10 ms
                let start = idx.saturating_sub(half_win);
                let end = (idx + half_win).min(clip.samples.len().saturating_sub(1));

                // Very simple: sum of absolute differences (proxy for HF content).
                let mut diff_energy: f32 = 0.0;
                let mut abs_energy: f32 = 0.0;
                for i in start..end {
                    abs_energy += self.samples_abs(clip, i);
                    if i > start {
                        diff_energy += (clip.samples[i] - clip.samples[i - 1]).abs();
                    }
                }
                let count = (end - start).max(1) as f32;
                abs_energy /= count;
                diff_energy /= count;

                let ratio = if abs_energy > 1e-8 {
                    diff_energy / abs_energy
                } else {
                    0.0
                };

                let band = if ratio < 0.3 {
                    FreqBand::Low
                } else if ratio < 0.7 {
                    FreqBand::Mid
                } else {
                    FreqBand::High
                };

                AudioOnset {
                    time,
                    strength,
                    frequency_band: band,
                }
            })
            .collect()
    }

    /// Estimate tempo (BPM) from detected beats.
    pub fn estimate_bpm(&self, clip: &AudioClip) -> f32 {
        let beats = self.detect_beats(clip);
        if beats.len() < 2 {
            return 0.0;
        }
        let total_interval: f32 = beats.windows(2).map(|w| w[1] - w[0]).sum();
        let avg_interval = total_interval / (beats.len() - 1) as f32;
        if avg_interval <= 0.0 {
            return 0.0;
        }
        60.0 / avg_interval
    }

    // internal helper
    fn samples_abs(&self, clip: &AudioClip, idx: usize) -> f32 {
        clip.samples.get(idx).map(|s| s.abs()).unwrap_or(0.0)
    }
}

// ---------------------------------------------------------------------------
// Viseme types
// ---------------------------------------------------------------------------

/// Weights for 6 basic viseme shapes.
#[derive(Debug, Clone, Copy)]
pub struct VisemeWeights {
    pub silence: f32,
    pub aa: f32,
    pub ee: f32,
    pub ih: f32,
    pub oh: f32,
    pub oo: f32,
}

impl Default for VisemeWeights {
    fn default() -> Self {
        Self {
            silence: 1.0,
            aa: 0.0,
            ee: 0.0,
            ih: 0.0,
            oh: 0.0,
            oo: 0.0,
        }
    }
}

impl VisemeWeights {
    /// Linear interpolation between two viseme weight sets.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            silence: mix(self.silence, other.silence),
            aa: mix(self.aa, other.aa),
            ee: mix(self.ee, other.ee),
            ih: mix(self.ih, other.ih),
            oh: mix(self.oh, other.oh),
            oo: mix(self.oo, other.oo),
        }
    }
}

/// A single viseme keyframe.
#[derive(Debug, Clone)]
pub struct VisemeFrame {
    /// Time in seconds.
    pub time: f32,
    /// Viseme weights at this time.
    pub weights: VisemeWeights,
}

// ---------------------------------------------------------------------------
// LipSyncData
// ---------------------------------------------------------------------------

/// Viseme-based lip sync data extracted from audio energy.
#[derive(Debug, Clone)]
pub struct LipSyncData {
    /// Ordered viseme keyframes.
    pub visemes: Vec<VisemeFrame>,
}

impl LipSyncData {
    /// Extract a viseme timeline from an audio clip using energy bands.
    ///
    /// This is a simplified energy-based approach (not phoneme recognition).
    /// It maps overall energy and spectral tilt to the six basic visemes.
    pub fn from_audio(clip: &AudioClip) -> Self {
        if clip.samples.is_empty() {
            return Self { visemes: Vec::new() };
        }

        let frame_dur = 0.02_f32; // 20 ms per frame
        let num_frames = (clip.duration / frame_dur).ceil() as usize;
        let mut visemes = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let time = i as f32 * frame_dur;
            let rms = clip.rms_window(time, frame_dur);

            // Very simple spectral tilt: ratio of sample-to-sample differences
            // (high-frequency proxy) vs absolute energy.
            let idx_start = clip.time_to_index(time);
            let idx_end = clip.time_to_index(time + frame_dur);
            let (mut diff_sum, mut abs_sum) = (0.0_f32, 0.0_f32);
            for j in idx_start..idx_end {
                abs_sum += clip.samples[j].abs();
                if j > idx_start {
                    diff_sum += (clip.samples[j] - clip.samples[j - 1]).abs();
                }
            }
            let count = (idx_end - idx_start).max(1) as f32;
            abs_sum /= count;
            diff_sum /= count;
            let spectral_tilt = if abs_sum > 1e-8 { diff_sum / abs_sum } else { 0.0 };

            // Map energy and spectral tilt to visemes.
            let energy = (rms * 4.0).min(1.0); // scale up for typical speech
            let weights = if energy < 0.05 {
                VisemeWeights { silence: 1.0, ..Default::default() }
            } else {
                let base = energy;
                // Distribute across visemes based on spectral tilt.
                let aa = base * (1.0 - spectral_tilt).max(0.0) * 0.6;
                let oh = base * (1.0 - spectral_tilt).max(0.0) * 0.4;
                let ee = base * spectral_tilt.min(1.0) * 0.5;
                let ih = base * spectral_tilt.min(1.0) * 0.3;
                let oo = base * 0.2;
                let silence = (1.0 - energy).max(0.0);
                VisemeWeights { silence, aa, ee, ih, oh, oo }
            };

            visemes.push(VisemeFrame { time, weights });
        }

        Self { visemes }
    }

    /// Interpolated viseme weights at an arbitrary time.
    pub fn viseme_at(&self, time: f32) -> VisemeWeights {
        if self.visemes.is_empty() {
            return VisemeWeights::default();
        }
        if self.visemes.len() == 1 || time <= self.visemes[0].time {
            return self.visemes[0].weights;
        }
        let last = &self.visemes[self.visemes.len() - 1];
        if time >= last.time {
            return last.weights;
        }

        // Binary-search for the surrounding frames.
        let idx = self.visemes.partition_point(|f| f.time <= time);
        if idx == 0 {
            return self.visemes[0].weights;
        }
        let a = &self.visemes[idx - 1];
        let b = &self.visemes[idx];
        let span = b.time - a.time;
        let t = if span > 0.0 { (time - a.time) / span } else { 0.0 };
        a.weights.lerp(&b.weights, t)
    }

    /// Map the current viseme weights to standard blend-shape / shape-key names.
    pub fn to_shape_key_weights(&self, time: f32) -> Vec<(&str, f32)> {
        let w = self.viseme_at(time);
        vec![
            ("viseme_sil", w.silence),
            ("viseme_aa", w.aa),
            ("viseme_ee", w.ee),
            ("viseme_ih", w.ih),
            ("viseme_oh", w.oh),
            ("viseme_oo", w.oo),
        ]
    }
}

// ---------------------------------------------------------------------------
// AudioSyncController
// ---------------------------------------------------------------------------

/// Top-level controller that synchronises animation playback with audio.
#[derive(Debug, Clone)]
pub struct AudioSyncController {
    /// Currently loaded audio clip.
    pub clip: Option<AudioClip>,
    /// Pre-computed beat times (seconds).
    pub beat_times: Vec<f32>,
    /// Pre-computed lip-sync data.
    pub lip_sync: Option<LipSyncData>,
    /// Whether audio/animation is playing.
    pub playing: bool,
    /// Current playback time in seconds.
    pub current_time: f32,
    /// Playback volume [0.0, 1.0].
    pub volume: f32,
}

impl AudioSyncController {
    pub fn new() -> Self {
        Self {
            clip: None,
            beat_times: Vec::new(),
            lip_sync: None,
            playing: false,
            current_time: 0.0,
            volume: 1.0,
        }
    }

    /// Load a clip and pre-compute beats + lip sync.
    pub fn load_clip(&mut self, clip: AudioClip) {
        let detector = BeatDetector::new(0.15, 0.15);
        self.beat_times = detector.detect_beats(&clip);
        self.lip_sync = Some(LipSyncData::from_audio(&clip));
        self.clip = Some(clip);
        self.current_time = 0.0;
        self.playing = false;
    }

    /// Advance playback by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if !self.playing {
            return;
        }
        if let Some(ref clip) = self.clip {
            self.current_time += dt;
            if self.current_time > clip.duration {
                self.current_time = clip.duration;
                self.playing = false;
            }
        }
    }

    /// Snap a time value to the nearest detected beat.
    pub fn snap_to_beat(&self, time: f32) -> f32 {
        if self.beat_times.is_empty() {
            return time;
        }
        let mut best = self.beat_times[0];
        let mut best_dist = (time - best).abs();
        for &bt in &self.beat_times[1..] {
            let d = (time - bt).abs();
            if d < best_dist {
                best = bt;
                best_dist = d;
            }
        }
        best
    }

    /// Return all beat times within [start, end].
    pub fn beats_in_range(&self, start: f32, end: f32) -> Vec<f32> {
        self.beat_times
            .iter()
            .copied()
            .filter(|&t| t >= start && t <= end)
            .collect()
    }

    /// Current audio energy level at `current_time`.
    pub fn current_energy(&self) -> f32 {
        match self.clip {
            Some(ref clip) => clip.rms_window(self.current_time, 0.02),
            None => 0.0,
        }
    }
}

impl Default for AudioSyncController {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers: build a minimal WAV in memory (used by tests)
// ---------------------------------------------------------------------------

/// Build a minimal valid WAV file (PCM 16-bit, mono) from f32 samples.
fn build_wav_pcm16_mono(sample_rate: u32, samples: &[f32]) -> Vec<u8> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let block_align = num_channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_size = samples.len() as u32 * 2;
    let file_size = 36 + data_size; // RIFF header says size after first 8 bytes

    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_size as usize);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let i = (clamped * 32767.0) as i16;
        buf.extend_from_slice(&i.to_le_bytes());
    }

    buf
}

/// Build a minimal valid WAV file (IEEE float 32-bit, stereo) from f32 samples.
/// `samples` is interleaved L, R, L, R, ...
fn build_wav_float32_stereo(sample_rate: u32, samples: &[f32]) -> Vec<u8> {
    let num_channels: u16 = 2;
    let bits_per_sample: u16 = 32;
    let block_align = num_channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_size = samples.len() as u32 * 4;
    let file_size = 36 + data_size;

    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_size as usize);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    buf
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a mono sine wave at the given frequency.
    fn sine_wave(sample_rate: u32, freq: f32, duration: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * duration) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect()
    }

    // -- WAV parsing --------------------------------------------------------

    #[test]
    fn wav_pcm16_mono_roundtrip() {
        let samples = sine_wave(44100, 440.0, 0.1);
        let wav = build_wav_pcm16_mono(44100, &samples);
        let clip = AudioClip::from_wav(&wav).expect("parse failed");

        assert_eq!(clip.sample_rate, 44100);
        assert_eq!(clip.channels, 1);
        assert!((clip.duration - 0.1).abs() < 0.001);
        assert_eq!(clip.samples.len(), samples.len());
        // 16-bit quantisation: max error < 1/32768 ≈ 3.1e-5
        for (a, b) in clip.samples.iter().zip(samples.iter()) {
            assert!((a - b).abs() < 0.001, "sample mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn wav_float32_stereo_roundtrip() {
        let n = 4410; // 0.1 s at 44100 Hz
        let mut interleaved = Vec::with_capacity(n * 2);
        for i in 0..n {
            let t = i as f32 / 44100.0;
            let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            interleaved.push(s); // L
            interleaved.push(s); // R (same)
        }
        let wav = build_wav_float32_stereo(44100, &interleaved);
        let clip = AudioClip::from_wav(&wav).expect("parse failed");

        assert_eq!(clip.sample_rate, 44100);
        assert_eq!(clip.channels, 2);
        assert!((clip.duration - 0.1).abs() < 0.001);
        // After downmix, each mono sample should equal the original sine.
        for (i, &s) in clip.samples.iter().enumerate() {
            let t = i as f32 / 44100.0;
            let expected = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            assert!((s - expected).abs() < 1e-5, "frame {i}: {s} vs {expected}");
        }
    }

    #[test]
    fn wav_reject_invalid_header() {
        assert!(AudioClip::from_wav(&[]).is_err());
        assert!(AudioClip::from_wav(b"NOT_A_WAV_FILE_AT_ALL_1234567890123456789012345678901234567890").is_err());
    }

    // -- AudioClip methods --------------------------------------------------

    #[test]
    fn amplitude_at_and_peak() {
        let samples = vec![0.0, 0.5, -1.0, 0.25, 0.0];
        let wav = build_wav_pcm16_mono(5, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        // peak over full range
        let peak = clip.peak_in_range(0.0, clip.duration);
        assert!(peak > 0.9, "expected peak ~1.0, got {peak}");
    }

    #[test]
    fn rms_window_basic() {
        // Constant signal of 0.5
        let samples = vec![0.5_f32; 1000];
        let wav = build_wav_pcm16_mono(1000, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        let rms = clip.rms_window(0.0, 1.0);
        assert!((rms - 0.5).abs() < 0.01, "expected rms ~0.5, got {rms}");
    }

    #[test]
    fn downsample_waveform_length() {
        let samples = sine_wave(44100, 440.0, 1.0);
        let wav = build_wav_pcm16_mono(44100, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        let ds = clip.downsample_waveform(100);
        assert_eq!(ds.len(), 100);
        // All values should be >= 0 (absolute)
        assert!(ds.iter().all(|&v| v >= 0.0));
    }

    // -- BeatDetector -------------------------------------------------------

    #[test]
    fn detect_beats_on_impulse_train() {
        // Create a signal with clear impulses every 0.5 s (120 BPM).
        let sr = 8000u32;
        let dur = 3.0_f32;
        let n = (sr as f32 * dur) as usize;
        let mut samples = vec![0.0f32; n];
        let interval_samples = sr as usize / 2; // 0.5 s
        for i in (0..n).step_by(interval_samples) {
            let end = (i + 80).min(n);
            for j in i..end {
                samples[j] = 0.9;
            }
        }
        let wav = build_wav_pcm16_mono(sr, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        let detector = BeatDetector::new(0.1, 0.3);
        let beats = detector.detect_beats(&clip);

        // We expect roughly 6 beats in 3 s at 120 BPM.
        assert!(beats.len() >= 3, "too few beats detected: {}", beats.len());
    }

    #[test]
    fn estimate_bpm_silent_clip() {
        let samples = vec![0.0f32; 8000];
        let wav = build_wav_pcm16_mono(8000, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        let detector = BeatDetector::new(0.1, 0.2);
        let bpm = detector.estimate_bpm(&clip);
        assert!(bpm.abs() < 1.0, "expected ~0 BPM for silence, got {bpm}");
    }

    #[test]
    fn detect_onsets_returns_freq_bands() {
        let sr = 8000u32;
        let n = (sr as f32 * 1.0) as usize;
        let mut samples = vec![0.0f32; n];
        // Put a burst at 0.5 s.
        for j in 4000..4100 {
            samples[j] = 0.8;
        }
        let wav = build_wav_pcm16_mono(sr, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        let detector = BeatDetector::new(0.05, 0.1);
        let onsets = detector.detect_onsets(&clip);
        // At least one onset should be detected near the burst.
        assert!(!onsets.is_empty(), "no onsets detected");
        for o in &onsets {
            assert!(o.strength >= 0.0 && o.strength <= 1.0);
        }
    }

    // -- Viseme / LipSync ---------------------------------------------------

    #[test]
    fn viseme_weights_lerp() {
        let a = VisemeWeights { silence: 1.0, aa: 0.0, ee: 0.0, ih: 0.0, oh: 0.0, oo: 0.0 };
        let b = VisemeWeights { silence: 0.0, aa: 1.0, ee: 1.0, ih: 1.0, oh: 1.0, oo: 1.0 };
        let mid = a.lerp(&b, 0.5);
        assert!((mid.silence - 0.5).abs() < 1e-5);
        assert!((mid.aa - 0.5).abs() < 1e-5);
    }

    #[test]
    fn lip_sync_from_audio_generates_frames() {
        let samples = sine_wave(8000, 300.0, 0.5);
        let wav = build_wav_pcm16_mono(8000, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        let lip = LipSyncData::from_audio(&clip);
        assert!(!lip.visemes.is_empty(), "expected viseme frames");
        // First frame at time 0
        assert!((lip.visemes[0].time).abs() < 1e-5);
    }

    #[test]
    fn lip_sync_shape_key_names() {
        let samples = sine_wave(8000, 300.0, 0.2);
        let wav = build_wav_pcm16_mono(8000, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();

        let lip = LipSyncData::from_audio(&clip);
        let keys = lip.to_shape_key_weights(0.05);
        let names: Vec<&str> = keys.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"viseme_sil"));
        assert!(names.contains(&"viseme_aa"));
        assert!(names.contains(&"viseme_ee"));
        assert!(names.contains(&"viseme_ih"));
        assert!(names.contains(&"viseme_oh"));
        assert!(names.contains(&"viseme_oo"));
    }

    // -- AudioSyncController ------------------------------------------------

    #[test]
    fn controller_snap_to_beat() {
        let mut ctrl = AudioSyncController::new();
        let samples = vec![0.0f32; 8000];
        let wav = build_wav_pcm16_mono(8000, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();
        ctrl.load_clip(clip);

        // Manually insert beat times for deterministic testing.
        ctrl.beat_times = vec![0.0, 0.5, 1.0];

        assert!((ctrl.snap_to_beat(0.3) - 0.5).abs() < 0.01);
        assert!((ctrl.snap_to_beat(0.7) - 0.5).abs() < 0.01);
        assert!((ctrl.snap_to_beat(0.9) - 1.0).abs() < 0.01);
    }

    #[test]
    fn controller_update_advances_time() {
        let mut ctrl = AudioSyncController::new();
        let samples = vec![0.0f32; 8000];
        let wav = build_wav_pcm16_mono(8000, &samples);
        let clip = AudioClip::from_wav(&wav).unwrap();
        ctrl.load_clip(clip);
        ctrl.playing = true;

        ctrl.update(0.1);
        assert!((ctrl.current_time - 0.1).abs() < 1e-5);

        // Advance past end
        ctrl.update(100.0);
        assert!(!ctrl.playing);
    }

    #[test]
    fn controller_beats_in_range() {
        let mut ctrl = AudioSyncController::new();
        ctrl.beat_times = vec![0.1, 0.5, 1.0, 1.5, 2.0];

        let range = ctrl.beats_in_range(0.4, 1.1);
        assert_eq!(range, vec![0.5, 1.0]);
    }
}
