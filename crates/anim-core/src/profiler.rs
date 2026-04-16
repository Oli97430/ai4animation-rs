//! Scoped profiler for timing code sections (mirrors Python Profiler.py).
//!
//! Usage:
//! ```ignore
//! let _guard = ScopedTimer::new("physics_step");
//! // ... code to profile ...
//! // timer is stopped and recorded when `_guard` is dropped
//! ```

use std::collections::HashMap;
use std::time::Instant;

/// Global profiler collecting timing data from scoped timers.
pub struct Profiler {
    entries: HashMap<&'static str, TimingEntry>,
    enabled: bool,
}

struct TimingEntry {
    total_time: f64,
    call_count: u64,
    min_time: f64,
    max_time: f64,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a timing measurement.
    pub fn record(&mut self, name: &'static str, duration_secs: f64) {
        let entry = self.entries.entry(name).or_insert(TimingEntry {
            total_time: 0.0,
            call_count: 0,
            min_time: f64::MAX,
            max_time: 0.0,
        });
        entry.total_time += duration_secs;
        entry.call_count += 1;
        entry.min_time = entry.min_time.min(duration_secs);
        entry.max_time = entry.max_time.max(duration_secs);
    }

    /// Reset all timings.
    pub fn reset(&mut self) {
        self.entries.clear();
    }

    /// Get timing summary sorted by total time (descending).
    pub fn summary(&self) -> Vec<TimingSummary> {
        let mut results: Vec<TimingSummary> = self.entries.iter()
            .map(|(name, entry)| TimingSummary {
                name,
                total_ms: entry.total_time * 1000.0,
                avg_ms: if entry.call_count > 0 {
                    entry.total_time / entry.call_count as f64 * 1000.0
                } else { 0.0 },
                min_ms: entry.min_time * 1000.0,
                max_ms: entry.max_time * 1000.0,
                calls: entry.call_count,
            })
            .collect();
        results.sort_by(|a, b| b.total_ms.partial_cmp(&a.total_ms).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

impl Default for Profiler {
    fn default() -> Self { Self::new() }
}

/// Summary of one profiled section.
pub struct TimingSummary {
    pub name: &'static str,
    pub total_ms: f64,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub calls: u64,
}

/// RAII timer guard — records elapsed time on drop.
///
/// Store the return value in a `let _guard = ...` binding.
pub struct ScopedTimer {
    name: &'static str,
    start: Instant,
}

impl ScopedTimer {
    pub fn new(name: &'static str) -> Self {
        Self { name, start: Instant::now() }
    }

    /// Get elapsed time so far (without stopping).
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        log::trace!("[profiler] {} took {:.3}ms", self.name, duration * 1000.0);
        // Note: in a full implementation, this would record to a thread-local
        // or global profiler instance. For now it logs via the `log` crate.
    }
}

/// Simple one-shot timer for manual use.
pub struct StopWatch {
    start: Instant,
}

impl StopWatch {
    pub fn start() -> Self {
        Self { start: Instant::now() }
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    pub fn restart(&mut self) -> f64 {
        let elapsed = self.elapsed_ms();
        self.start = Instant::now();
        elapsed
    }
}
