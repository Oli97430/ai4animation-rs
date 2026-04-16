//! RunningStats — online mean/std normalization for inference.
//!
//! Mirrors Python AI/Stats.py. Uses Welford's online algorithm for
//! numerically stable incremental mean and variance computation.
//! This is the inference-only version (no PyTorch dependency).

use ndarray::{Array1, Array2};

/// Online running mean/std statistics for normalizing tensors.
pub struct RunningStats {
    /// Feature dimensionality.
    pub dim: usize,
    /// Sample count.
    n: f64,
    /// Running mean (Welford).
    mean: Array1<f32>,
    /// Running sum of squared deviations (Welford).
    s: Array1<f32>,
    /// Cached normalization parameters: (mean, std).
    pub norm_mean: Array1<f32>,
    pub norm_std: Array1<f32>,
}

impl RunningStats {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            n: 0.0,
            mean: Array1::zeros(dim),
            s: Array1::zeros(dim),
            norm_mean: Array1::zeros(dim),
            norm_std: Array1::ones(dim),
        }
    }

    /// Load pre-computed mean/std (e.g., from a saved model checkpoint).
    pub fn from_params(mean: Array1<f32>, std: Array1<f32>) -> Self {
        let dim = mean.len();
        Self {
            dim,
            n: 0.0,
            mean: mean.clone(),
            s: Array1::zeros(dim),
            norm_mean: mean,
            norm_std: std,
        }
    }

    /// Reset statistics.
    pub fn clear(&mut self) {
        self.n = 0.0;
        self.mean.fill(0.0);
        self.s.fill(0.0);
    }

    /// Update statistics with a batch of data [N, dim].
    pub fn update(&mut self, data: &Array2<f32>) {
        assert_eq!(data.ncols(), self.dim, "Data dimension mismatch");
        for row in data.rows() {
            self.update_single(row.as_slice().unwrap());
        }
        self.recompute_norm();
    }

    /// Update statistics with a single sample.
    pub fn update_single(&mut self, x: &[f32]) {
        self.n += 1.0;
        if self.n == 1.0 {
            for i in 0..self.dim {
                self.mean[i] = x[i];
                self.s[i] = 0.0;
            }
        } else {
            for i in 0..self.dim {
                let prev_mean = self.mean[i];
                self.mean[i] += (x[i] - self.mean[i]) / self.n as f32;
                self.s[i] += (x[i] - prev_mean) * (x[i] - self.mean[i]);
            }
        }
    }

    /// Recompute cached norm parameters from running stats.
    fn recompute_norm(&mut self) {
        self.norm_mean = self.mean.clone();
        let mut std = self.variance().mapv(f32::sqrt);
        // Clamp near-zero std to 1.0 to avoid division by zero
        std.mapv_inplace(|v| if v < 0.001 { 1.0 } else { v });
        self.norm_std = std;
    }

    /// Current variance estimate.
    pub fn variance(&self) -> Array1<f32> {
        if self.n > 1.0 {
            &self.s / (self.n as f32 - 1.0)
        } else {
            Array1::zeros(self.dim)
        }
    }

    /// Current standard deviation.
    pub fn std(&self) -> Array1<f32> {
        self.variance().mapv(f32::sqrt)
    }

    /// Number of samples seen.
    pub fn count(&self) -> f64 {
        self.n
    }

    /// Normalize a single sample: (x - mean) / std.
    pub fn normalize(&self, x: &[f32]) -> Vec<f32> {
        x.iter()
            .enumerate()
            .map(|(i, &v)| (v - self.norm_mean[i]) / self.norm_std[i])
            .collect()
    }

    /// Denormalize a single sample: x * std + mean.
    pub fn denormalize(&self, x: &[f32]) -> Vec<f32> {
        x.iter()
            .enumerate()
            .map(|(i, &v)| v * self.norm_std[i] + self.norm_mean[i])
            .collect()
    }

    /// Normalize an ndarray batch [N, dim].
    pub fn normalize_batch(&self, data: &Array2<f32>) -> Array2<f32> {
        let mean = self.norm_mean.view().insert_axis(ndarray::Axis(0));
        let std = self.norm_std.view().insert_axis(ndarray::Axis(0));
        (data - &mean) / &std
    }

    /// Denormalize an ndarray batch [N, dim].
    pub fn denormalize_batch(&self, data: &Array2<f32>) -> Array2<f32> {
        let mean = self.norm_mean.view().insert_axis(ndarray::Axis(0));
        let std = self.norm_std.view().insert_axis(ndarray::Axis(0));
        data * &std + &mean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn basic_stats() {
        let mut stats = RunningStats::new(2);
        let data = array![
            [1.0f32, 2.0],
            [3.0, 4.0],
            [5.0, 6.0],
        ];
        stats.update(&data);
        assert!((stats.norm_mean[0] - 3.0).abs() < 1e-5);
        assert!((stats.norm_mean[1] - 4.0).abs() < 1e-5);
        assert!(stats.count() == 3.0);
    }

    #[test]
    fn normalize_denormalize() {
        let stats = RunningStats::from_params(
            array![2.0f32, 5.0],
            array![1.0f32, 2.0],
        );
        let x = [4.0f32, 9.0];
        let normed = stats.normalize(&x);
        assert!((normed[0] - 2.0).abs() < 1e-5); // (4-2)/1
        assert!((normed[1] - 2.0).abs() < 1e-5); // (9-5)/2
        let denormed = stats.denormalize(&normed);
        assert!((denormed[0] - x[0]).abs() < 1e-5);
        assert!((denormed[1] - x[1]).abs() < 1e-5);
    }
}
