//! Signal processing utilities: smoothing, filtering, interpolation.
//!
//! Reusable building blocks for motion analysis and feature extraction.

use glam::Vec3;

// ── Gaussian smoothing ──────────────────────────────────

/// Build a normalized 1D Gaussian kernel.
///
/// `radius` — number of samples on each side of center.
/// `sigma` — standard deviation in sample units.
pub fn gaussian_kernel(radius: usize, sigma: f32) -> Vec<f32> {
    let n = 2 * radius + 1;
    let mut kernel = Vec::with_capacity(n);
    let inv_2sigma2 = -1.0 / (2.0 * sigma * sigma);
    let mut sum = 0.0;
    for i in 0..n {
        let x = i as f32 - radius as f32;
        let w = (x * x * inv_2sigma2).exp();
        kernel.push(w);
        sum += w;
    }
    // Normalize
    let inv_sum = 1.0 / sum;
    for w in &mut kernel {
        *w *= inv_sum;
    }
    kernel
}

/// Gaussian-smooth a slice of f32 values.
///
/// `window_sec` — smoothing window in seconds.
/// `fps` — sample rate.
pub fn gaussian_smooth_f32(data: &[f32], window_sec: f32, fps: f32) -> Vec<f32> {
    if data.is_empty() || window_sec <= 0.0 { return data.to_vec(); }

    let radius = (window_sec * fps * 0.5).ceil() as usize;
    if radius == 0 { return data.to_vec(); }

    let sigma = radius as f32 / 3.0;
    let kernel = gaussian_kernel(radius, sigma);
    let n = data.len();
    let mut result = vec![0.0f32; n];

    for i in 0..n {
        let mut val = 0.0;
        let mut weight_sum = 0.0;
        for (ki, &w) in kernel.iter().enumerate() {
            let j = i as i64 + ki as i64 - radius as i64;
            if j >= 0 && (j as usize) < n {
                val += data[j as usize] * w;
                weight_sum += w;
            }
        }
        result[i] = if weight_sum > 0.0 { val / weight_sum } else { data[i] };
    }
    result
}

/// Gaussian-smooth a slice of Vec3 values.
pub fn gaussian_smooth_vec3(data: &[Vec3], window_sec: f32, fps: f32) -> Vec<Vec3> {
    if data.is_empty() || window_sec <= 0.0 { return data.to_vec(); }

    let radius = (window_sec * fps * 0.5).ceil() as usize;
    if radius == 0 { return data.to_vec(); }

    let sigma = radius as f32 / 3.0;
    let kernel = gaussian_kernel(radius, sigma);
    let n = data.len();
    let mut result = vec![Vec3::ZERO; n];

    for i in 0..n {
        let mut val = Vec3::ZERO;
        let mut weight_sum = 0.0;
        for (ki, &w) in kernel.iter().enumerate() {
            let j = i as i64 + ki as i64 - radius as i64;
            if j >= 0 && (j as usize) < n {
                val += data[j as usize] * w;
                weight_sum += w;
            }
        }
        result[i] = if weight_sum > 0.0 { val / weight_sum } else { data[i] };
    }
    result
}

// ── Moving average ──────────────────────────────────────

/// Simple moving average of f32 values.
pub fn moving_average_f32(data: &[f32], window: usize) -> Vec<f32> {
    if data.is_empty() || window == 0 { return data.to_vec(); }
    let n = data.len();
    let half = window / 2;
    let mut result = vec![0.0f32; n];

    for (i, out) in result.iter_mut().enumerate() {
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(n);
        let count = (hi - lo) as f32;
        let sum: f32 = data[lo..hi].iter().sum();
        *out = sum / count;
    }
    result
}

/// Simple moving average of Vec3 values.
pub fn moving_average_vec3(data: &[Vec3], window: usize) -> Vec<Vec3> {
    if data.is_empty() || window == 0 { return data.to_vec(); }
    let n = data.len();
    let half = window / 2;
    let mut result = vec![Vec3::ZERO; n];

    for (i, out) in result.iter_mut().enumerate() {
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(n);
        let count = (hi - lo) as f32;
        let sum: Vec3 = data[lo..hi].iter().copied().sum();
        *out = sum / count;
    }
    result
}

// ── Interpolation ───────────────────────────────────────

/// Cubic Hermite interpolation between two values with tangents.
///
/// `p0, p1` — values at t=0, t=1.
/// `m0, m1` — tangents at t=0, t=1.
/// `t` — interpolation parameter [0, 1].
pub fn cubic_hermite(p0: f32, p1: f32, m0: f32, m1: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1
}

/// Cubic Hermite interpolation for Vec3.
pub fn cubic_hermite_vec3(p0: Vec3, p1: Vec3, m0: Vec3, m1: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
}

/// Catmull-Rom spline interpolation through 4 points.
///
/// `p0..p3` — four control points; interpolation happens between p1 and p2.
/// `t` — [0, 1] parameter between p1 and p2.
pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let m0 = (p2 - p0) * 0.5;
    let m1 = (p3 - p1) * 0.5;
    cubic_hermite(p1, p2, m0, m1, t)
}

/// Catmull-Rom spline interpolation for Vec3.
pub fn catmull_rom_vec3(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let m0 = (p2 - p0) * 0.5;
    let m1 = (p3 - p1) * 0.5;
    cubic_hermite_vec3(p1, p2, m0, m1, t)
}

// ── Spring damper ───────────────────────────────────────

/// Critically damped spring smoothing (exponential decay).
///
/// Commonly used for smooth camera/character follow.
/// `current` — current value.
/// `target` — target value.
/// `velocity` — current velocity (mutated in place).
/// `half_life` — time to reach halfway to target.
/// `dt` — time step.
pub fn spring_damper(current: f32, target: f32, velocity: &mut f32, half_life: f32, dt: f32) -> f32 {
    if half_life <= 0.0 { *velocity = 0.0; return target; }
    let omega = 2.0 * std::f32::consts::LN_2 / half_life;
    let exp = (-omega * dt).exp();
    let diff = current - target;
    let new_value = target + (diff + (*velocity + omega * diff) * dt) * exp;
    *velocity = (*velocity - omega * (*velocity + omega * diff) * dt) * exp;
    new_value
}

/// Spring damper for Vec3.
pub fn spring_damper_vec3(
    current: Vec3,
    target: Vec3,
    velocity: &mut Vec3,
    half_life: f32,
    dt: f32,
) -> Vec3 {
    Vec3::new(
        spring_damper(current.x, target.x, &mut velocity.x, half_life, dt),
        spring_damper(current.y, target.y, &mut velocity.y, half_life, dt),
        spring_damper(current.z, target.z, &mut velocity.z, half_life, dt),
    )
}

// ── Numerical differentiation ───────────────────────────

/// Central finite difference (first derivative).
///
/// At boundaries, falls back to forward/backward difference.
pub fn finite_difference_f32(data: &[f32], dt: f32) -> Vec<f32> {
    let n = data.len();
    if n < 2 || dt.abs() < 1e-10 { return vec![0.0; n]; }
    let inv_dt = 1.0 / dt;
    let inv_2dt = 0.5 / dt;

    let mut result = vec![0.0; n];
    // Forward difference for first element
    result[0] = (data[1] - data[0]) * inv_dt;
    // Central difference for interior
    for i in 1..n - 1 {
        result[i] = (data[i + 1] - data[i - 1]) * inv_2dt;
    }
    // Backward difference for last element
    result[n - 1] = (data[n - 1] - data[n - 2]) * inv_dt;
    result
}

/// Central finite difference for Vec3 sequences.
pub fn finite_difference_vec3(data: &[Vec3], dt: f32) -> Vec<Vec3> {
    let n = data.len();
    if n < 2 || dt.abs() < 1e-10 { return vec![Vec3::ZERO; n]; }
    let inv_dt = 1.0 / dt;
    let inv_2dt = 0.5 / dt;

    let mut result = vec![Vec3::ZERO; n];
    result[0] = (data[1] - data[0]) * inv_dt;
    for i in 1..n - 1 {
        result[i] = (data[i + 1] - data[i - 1]) * inv_2dt;
    }
    result[n - 1] = (data[n - 1] - data[n - 2]) * inv_dt;
    result
}

// ── Misc ────────────────────────────────────────────────

/// Remap a value from [in_min, in_max] to [out_min, out_max].
pub fn remap(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let range = in_max - in_min;
    if range.abs() < 1e-10 { return out_min; }
    let t = (value - in_min) / range;
    out_min + t * (out_max - out_min)
}

/// Remap with clamping.
pub fn remap_clamped(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let range = in_max - in_min;
    if range.abs() < 1e-10 { return out_min; }
    let t = ((value - in_min) / range).clamp(0.0, 1.0);
    out_min + t * (out_max - out_min)
}

/// Smooth step (cubic ease).
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let range = edge1 - edge0;
    if range.abs() < 1e-10 { return if x >= edge1 { 1.0 } else { 0.0 }; }
    let t = ((x - edge0) / range).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Exponential decay (for framerate-independent lerp).
///
/// `lambda` — decay rate (higher = faster convergence).
/// `dt` — time step.
pub fn exp_decay(lambda: f32, dt: f32) -> f32 {
    1.0 - (-lambda * dt).exp()
}
