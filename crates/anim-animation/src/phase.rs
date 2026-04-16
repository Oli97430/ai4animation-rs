//! Phase detection — locomotion cycle analysis.
//!
//! Detects periodic gait cycles from foot contact patterns and computes
//! a normalized phase [0, 1] for each frame.

use glam::Vec3;
use crate::motion::Motion;
use crate::contact::ContactModule;

/// Phase data for the entire motion.
#[derive(Debug, Clone)]
pub struct PhaseData {
    /// Normalized phase [0, 1] per frame (0 = start of cycle, 1 = end).
    pub phases: Vec<f32>,
    /// Detected cycle boundaries (frame indices where phase resets to 0).
    pub cycle_starts: Vec<usize>,
    /// Average cycle length in frames.
    pub avg_cycle_length: f32,
    /// Frequency in Hz.
    pub frequency: f32,
}

impl PhaseData {
    /// Get phase at a given timestamp (interpolated).
    pub fn get_phase(&self, timestamp: f32, framerate: f32) -> f32 {
        if self.phases.is_empty() { return 0.0; }
        let t = timestamp * framerate;
        let idx = t.floor() as usize;
        let frac = t - t.floor();
        if idx >= self.phases.len() - 1 {
            return *self.phases.last().unwrap();
        }
        let a = self.phases[idx];
        let b = self.phases[idx + 1];
        // Handle phase wrap-around
        if (b - a).abs() > 0.5 {
            // Phase wraps — interpolate through the wrap
            if b > a {
                let unwrapped = a + 1.0;
                (unwrapped + (b - unwrapped) * frac) % 1.0
            } else {
                let unwrapped = b + 1.0;
                (a + (unwrapped - a) * frac) % 1.0
            }
        } else {
            a + (b - a) * frac
        }
    }

    /// Number of detected cycles.
    pub fn num_cycles(&self) -> usize {
        if self.cycle_starts.len() <= 1 { 0 }
        else { self.cycle_starts.len() - 1 }
    }
}

/// Detect locomotion phase from foot contacts.
///
/// Uses a primary foot sensor's contact pattern to define cycles.
/// A cycle starts when the foot transitions from airborne to grounded.
pub fn detect_phase(motion: &Motion, contacts: &ContactModule) -> PhaseData {
    let n = motion.num_frames();
    if n == 0 {
        return PhaseData {
            phases: Vec::new(),
            cycle_starts: Vec::new(),
            avg_cycle_length: 0.0,
            frequency: 0.0,
        };
    }

    // Use the first contact sensor (typically left foot)
    let dt = motion.delta_time();

    // Compute contact state per frame for the primary sensor
    let mut contact_states: Vec<bool> = Vec::with_capacity(n);
    for f in 0..n {
        let t = f as f32 * dt;
        let cf = contacts.get_contacts(motion, t, false);
        // Primary foot = first sensor
        contact_states.push(cf.contacts.first().copied().unwrap_or(false));
    }

    // Find cycle boundaries: transition from false→true (foot lands)
    let mut cycle_starts = Vec::new();
    for i in 1..n {
        if contact_states[i] && !contact_states[i - 1] {
            cycle_starts.push(i);
        }
    }

    // Need at least 2 boundaries for one complete cycle
    if cycle_starts.len() < 2 {
        // Fallback: try velocity-based cycle detection
        return detect_phase_velocity(motion, n, dt);
    }

    // Compute phases: linear interpolation between cycle boundaries
    let mut phases = vec![0.0f32; n];

    // Before first cycle start
    for f in 0..cycle_starts[0] {
        phases[f] = 0.0;
    }

    // Between cycle boundaries
    for c in 0..cycle_starts.len() - 1 {
        let start = cycle_starts[c];
        let end = cycle_starts[c + 1];
        let len = (end - start) as f32;
        for f in start..end {
            phases[f] = (f - start) as f32 / len;
        }
    }

    // After last cycle start
    if let Some(&last_start) = cycle_starts.last() {
        // Estimate cycle length from average
        let avg_len = if cycle_starts.len() >= 2 {
            let total: usize = cycle_starts.windows(2).map(|w| w[1] - w[0]).sum();
            total as f32 / (cycle_starts.len() - 1) as f32
        } else {
            (n - last_start) as f32
        };
        for f in last_start..n {
            phases[f] = ((f - last_start) as f32 / avg_len).min(1.0);
        }
    }

    let avg_cycle_length = if cycle_starts.len() >= 2 {
        let total: usize = cycle_starts.windows(2).map(|w| w[1] - w[0]).sum();
        total as f32 / (cycle_starts.len() - 1) as f32
    } else {
        n as f32
    };

    let frequency = if avg_cycle_length > 0.0 {
        motion.framerate / avg_cycle_length
    } else {
        0.0
    };

    PhaseData {
        phases,
        cycle_starts,
        avg_cycle_length,
        frequency,
    }
}

/// Fallback: detect phase from root velocity oscillation.
fn detect_phase_velocity(motion: &Motion, n: usize, dt: f32) -> PhaseData {
    if n < 3 {
        return PhaseData {
            phases: vec![0.0; n],
            cycle_starts: Vec::new(),
            avg_cycle_length: 0.0,
            frequency: 0.0,
        };
    }

    // Compute root velocity magnitude
    let mut speeds: Vec<f32> = Vec::with_capacity(n);
    for f in 0..n {
        let t = f as f32 * dt;
        let vels = motion.get_velocities(t, false);
        let root_speed = vels.first().map_or(0.0, |v| Vec3::new(v.x, 0.0, v.z).length());
        speeds.push(root_speed);
    }

    // Find local minima in speed (stride boundaries)
    let mut cycle_starts = Vec::new();
    for i in 2..n - 2 {
        if speeds[i] < speeds[i - 1] && speeds[i] < speeds[i + 1]
            && speeds[i] < speeds[i - 2] && speeds[i] < speeds[i + 2]
        {
            // Avoid duplicates within 5 frames
            if cycle_starts.last().map_or(true, |&last: &usize| i - last > 5) {
                cycle_starts.push(i);
            }
        }
    }

    if cycle_starts.len() < 2 {
        return PhaseData {
            phases: (0..n).map(|f| f as f32 / n as f32).collect(),
            cycle_starts: vec![0],
            avg_cycle_length: n as f32,
            frequency: motion.framerate / n as f32,
        };
    }

    // Build phases
    let mut phases = vec![0.0f32; n];
    for f in 0..cycle_starts[0] {
        phases[f] = 0.0;
    }
    for c in 0..cycle_starts.len() - 1 {
        let start = cycle_starts[c];
        let end = cycle_starts[c + 1];
        let len = (end - start) as f32;
        for f in start..end {
            phases[f] = (f - start) as f32 / len;
        }
    }
    if let Some(&last) = cycle_starts.last() {
        let avg = cycle_starts.windows(2).map(|w| w[1] - w[0]).sum::<usize>() as f32
            / (cycle_starts.len() - 1) as f32;
        for f in last..n {
            phases[f] = ((f - last) as f32 / avg).min(1.0);
        }
    }

    let avg_cycle_length = cycle_starts.windows(2).map(|w| w[1] - w[0]).sum::<usize>() as f32
        / (cycle_starts.len() - 1) as f32;

    PhaseData {
        phases,
        cycle_starts,
        avg_cycle_length,
        frequency: motion.framerate / avg_cycle_length,
    }
}
