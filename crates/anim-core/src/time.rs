//! Global time state.

/// Time state for the application.
pub struct Time {
    pub total_time: f32,
    pub delta_time: f32,
    pub timescale: f32,
    pub frame_count: u64,
}

impl Time {
    pub fn new() -> Self {
        Self {
            total_time: 0.0,
            delta_time: 0.0,
            timescale: 1.0,
            frame_count: 0,
        }
    }

    /// Update time with raw delta (applies timescale).
    pub fn update(&mut self, raw_dt: f32) {
        self.delta_time = raw_dt * self.timescale;
        self.total_time += self.delta_time;
        self.frame_count += 1;
    }

    /// Scaled delta time.
    pub fn dt(&self) -> f32 {
        self.delta_time
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}
