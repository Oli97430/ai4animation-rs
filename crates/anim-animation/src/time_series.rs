//! TimeSeries — temporal window with uniformly spaced samples.
//!
//! Defines a time window [start, end] with N samples uniformly distributed.
//! Used for motion feature extraction and smoothing.

/// A single sample within a time series.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Index within the time series.
    pub index: usize,
    /// Timestamp relative to the window center (or absolute after simulate).
    pub timestamp: f32,
}

/// Uniform temporal window with evenly spaced samples.
#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Start of the window (seconds, relative).
    pub start: f32,
    /// End of the window (seconds, relative).
    pub end: f32,
    /// Uniformly distributed samples.
    pub samples: Vec<Sample>,
}

impl TimeSeries {
    /// Create a new time series with `count` samples spanning [start, end].
    pub fn new(start: f32, end: f32, count: usize) -> Self {
        let count = count.max(1);
        let samples = if count == 1 {
            vec![Sample { index: 0, timestamp: (start + end) * 0.5 }]
        } else {
            (0..count)
                .map(|i| {
                    let t = i as f32 / (count - 1) as f32;
                    Sample {
                        index: i,
                        timestamp: start + t * (end - start),
                    }
                })
                .collect()
        };

        Self { start, end, samples }
    }

    /// Window duration in seconds.
    pub fn window(&self) -> f32 {
        self.end - self.start
    }

    /// Time interval between consecutive samples.
    pub fn delta_time(&self) -> f32 {
        if self.samples.len() <= 1 { 0.0 }
        else { self.window() / (self.samples.len() - 1) as f32 }
    }

    /// Number of samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Get all timestamps as a Vec.
    pub fn timestamps(&self) -> Vec<f32> {
        self.samples.iter().map(|s| s.timestamp).collect()
    }

    /// Shift the window to be centered at `timestamp` and return absolute timestamps.
    pub fn simulate_timestamps(&self, timestamp: f32) -> Vec<f32> {
        self.samples.iter().map(|s| timestamp + s.timestamp).collect()
    }

    /// Get the nearest sample for a query timestamp.
    pub fn get_sample(&self, timestamp: f32) -> &Sample {
        let clamped = timestamp.clamp(self.start, self.end);
        let mut best = 0;
        let mut best_dist = f32::MAX;
        for (i, s) in self.samples.iter().enumerate() {
            let d = (s.timestamp - clamped).abs();
            if d < best_dist {
                best_dist = d;
                best = i;
            }
        }
        &self.samples[best]
    }

    /// Get the center sample index.
    pub fn center_index(&self) -> usize {
        self.samples.len() / 2
    }
}

impl Default for TimeSeries {
    fn default() -> Self {
        Self::new(-1.0, 1.0, 13)
    }
}
