//! Post-processing effects for the 3D viewport.
//!
//! Provides CPU-side image post-processing including motion blur,
//! depth of field, bloom, vignette, and color grading. Each effect
//! can be toggled independently and ships with sensible defaults.

// ---------------------------------------------------------------------------
// Configuration types
// ---------------------------------------------------------------------------

/// Shape of the bokeh highlight for depth-of-field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BokehShape {
    Circle,
    Hexagon,
}

impl Default for BokehShape {
    fn default() -> Self {
        Self::Circle
    }
}

/// Motion blur configuration.
#[derive(Debug, Clone)]
pub struct MotionBlurConfig {
    pub enabled: bool,
    /// Blend intensity between current and previous frame (0.0 - 1.0).
    pub intensity: f32,
    /// Number of intermediate samples (4 - 32).
    pub samples: u32,
    /// Maximum blur extent in pixels.
    pub max_blur_pixels: f32,
}

impl Default for MotionBlurConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.5,
            samples: 8,
            max_blur_pixels: 20.0,
        }
    }
}

/// Depth-of-field configuration.
#[derive(Debug, Clone)]
pub struct DepthOfFieldConfig {
    pub enabled: bool,
    /// Distance to the focal plane (world units).
    pub focus_distance: f32,
    /// Lens aperture in f-stops.
    pub aperture: f32,
    /// Focal length of the virtual lens (mm).
    pub focal_length: f32,
    /// Shape used for the bokeh highlight.
    pub bokeh_shape: BokehShape,
}

impl Default for DepthOfFieldConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            focus_distance: 5.0,
            aperture: 2.8,
            focal_length: 50.0,
            bokeh_shape: BokehShape::default(),
        }
    }
}

/// Bloom configuration.
#[derive(Debug, Clone)]
pub struct BloomConfig {
    pub enabled: bool,
    /// Luminance threshold above which pixels contribute to bloom.
    pub threshold: f32,
    /// Strength of the additive bloom overlay.
    pub intensity: f32,
    /// Blur radius in pixels.
    pub radius: f32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 1.0,
            intensity: 0.3,
            radius: 5.0,
        }
    }
}

/// Vignette configuration.
#[derive(Debug, Clone)]
pub struct VignetteConfig {
    pub enabled: bool,
    /// Darkening intensity at the edges (0.0 - 1.0).
    pub intensity: f32,
    /// Smoothness of the falloff curve.
    pub smoothness: f32,
    /// Tint colour applied to the darkened region (RGB, linear).
    pub color: [f32; 3],
}

impl Default for VignetteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.3,
            smoothness: 2.0,
            color: [0.0, 0.0, 0.0],
        }
    }
}

/// Colour-grading configuration.
#[derive(Debug, Clone)]
pub struct ColorGradingConfig {
    pub enabled: bool,
    /// Exposure multiplier (linear).
    pub exposure: f32,
    /// Contrast adjustment (1.0 = neutral).
    pub contrast: f32,
    /// Saturation adjustment (1.0 = neutral, 0.0 = grayscale).
    pub saturation: f32,
    /// Temperature shift (-1.0 cool .. 1.0 warm).
    pub temperature: f32,
    /// Tint shift (-1.0 green .. 1.0 magenta).
    pub tint: f32,
    /// Shadow colour adjustment (lift).
    pub lift: [f32; 3],
    /// Mid-tone colour adjustment (gamma).
    pub gamma: [f32; 3],
    /// Highlight colour adjustment (gain).
    pub gain: [f32; 3],
}

impl Default for ColorGradingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            exposure: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            lift: [0.0; 3],
            gamma: [1.0; 3],
            gain: [1.0; 3],
        }
    }
}

/// Master configuration for all post-processing effects.
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    /// Global on/off switch for the entire post-processing stack.
    pub enabled: bool,
    pub motion_blur: MotionBlurConfig,
    pub dof: DepthOfFieldConfig,
    pub bloom: BloomConfig,
    pub vignette: VignetteConfig,
    pub color_grading: ColorGradingConfig,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            motion_blur: MotionBlurConfig::default(),
            dof: DepthOfFieldConfig::default(),
            bloom: BloomConfig::default(),
            vignette: VignetteConfig::default(),
            color_grading: ColorGradingConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Preset constructors
// ---------------------------------------------------------------------------

/// Cinematic look: bloom + vignette + warm colour grading.
pub fn cinematic() -> PostProcessConfig {
    PostProcessConfig {
        enabled: true,
        bloom: BloomConfig {
            enabled: true,
            threshold: 0.8,
            intensity: 0.4,
            radius: 6.0,
        },
        vignette: VignetteConfig {
            enabled: true,
            intensity: 0.4,
            smoothness: 2.5,
            color: [0.0, 0.0, 0.0],
        },
        color_grading: ColorGradingConfig {
            enabled: true,
            exposure: 1.1,
            contrast: 1.15,
            saturation: 1.05,
            temperature: 0.15,
            tint: 0.0,
            lift: [0.02, 0.0, -0.02],
            gamma: [1.0, 1.0, 1.0],
            gain: [1.05, 1.0, 0.95],
        },
        ..Default::default()
    }
}

/// Documentary style: slight desaturation + vignette.
pub fn documentary() -> PostProcessConfig {
    PostProcessConfig {
        enabled: true,
        vignette: VignetteConfig {
            enabled: true,
            intensity: 0.25,
            smoothness: 2.0,
            color: [0.0, 0.0, 0.0],
        },
        color_grading: ColorGradingConfig {
            enabled: true,
            exposure: 1.0,
            contrast: 1.05,
            saturation: 0.85,
            temperature: -0.05,
            tint: 0.0,
            lift: [0.0; 3],
            gamma: [1.0; 3],
            gain: [1.0; 3],
        },
        ..Default::default()
    }
}

/// Game preview: bloom + motion blur.
pub fn game_preview() -> PostProcessConfig {
    PostProcessConfig {
        enabled: true,
        bloom: BloomConfig {
            enabled: true,
            threshold: 1.0,
            intensity: 0.35,
            radius: 5.0,
        },
        motion_blur: MotionBlurConfig {
            enabled: true,
            intensity: 0.4,
            samples: 8,
            max_blur_pixels: 16.0,
        },
        ..Default::default()
    }
}

/// Clean: every effect disabled.
pub fn clean() -> PostProcessConfig {
    PostProcessConfig {
        enabled: false,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// CPU-side post-processing pipeline
// ---------------------------------------------------------------------------

/// CPU-side post-processing pipeline operating on RGBA8 pixel buffers.
pub struct PostProcessPipeline {
    pub config: PostProcessConfig,
}

impl PostProcessPipeline {
    /// Create a new pipeline with the given configuration.
    pub fn new(config: PostProcessConfig) -> Self {
        Self { config }
    }

    /// Apply all enabled effects in order to an RGBA8 buffer.
    ///
    /// The pipeline order is: bloom -> vignette -> color grading.
    /// (Motion blur and DOF require extra data and must be called separately.)
    pub fn apply(&self, pixels: &mut [u8], width: u32, height: u32) {
        if !self.config.enabled {
            return;
        }
        if self.config.bloom.enabled {
            Self::apply_bloom(pixels, width, height, &self.config.bloom);
        }
        if self.config.vignette.enabled {
            Self::apply_vignette(pixels, width, height, &self.config.vignette);
        }
        if self.config.color_grading.enabled {
            Self::apply_color_grading(pixels, width, height, &self.config.color_grading);
        }
    }

    // ── Motion blur ──────────────────────────────────────────────────────

    /// Blend the current frame with a previous frame weighted by intensity.
    ///
    /// Both `pixels` and `prev_pixels` must be RGBA8 buffers of equal size.
    pub fn apply_motion_blur(
        pixels: &mut [u8],
        prev_pixels: &[u8],
        width: u32,
        height: u32,
        config: &MotionBlurConfig,
    ) {
        if !config.enabled {
            return;
        }
        let len = (width * height * 4) as usize;
        assert!(pixels.len() >= len && prev_pixels.len() >= len);

        let t = config.intensity.clamp(0.0, 1.0);
        let inv_t = 1.0 - t;
        for i in 0..len {
            let cur = pixels[i] as f32;
            let prev = prev_pixels[i] as f32;
            pixels[i] = (cur * inv_t + prev * t).round().min(255.0).max(0.0) as u8;
        }
    }

    // ── Bloom ────────────────────────────────────────────────────────────

    /// Threshold bright pixels, apply a Gaussian blur, and blend additively.
    pub fn apply_bloom(
        pixels: &mut [u8],
        width: u32,
        height: u32,
        config: &BloomConfig,
    ) {
        if !config.enabled {
            return;
        }
        let w = width as usize;
        let h = height as usize;
        let total = w * h;

        // 1. Extract bright pixels into a floating-point buffer.
        let threshold = config.threshold;
        let mut bright: Vec<[f32; 3]> = Vec::with_capacity(total);
        for i in 0..total {
            let base = i * 4;
            let r = pixels[base] as f32 / 255.0;
            let g = pixels[base + 1] as f32 / 255.0;
            let b = pixels[base + 2] as f32 / 255.0;
            let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            if luminance > threshold {
                let excess = luminance - threshold;
                bright.push([r * excess, g * excess, b * excess]);
            } else {
                bright.push([0.0, 0.0, 0.0]);
            }
        }

        // 2. Build a 1-D Gaussian kernel.
        let radius = (config.radius as usize).max(1);
        let sigma = radius as f32 / 2.0;
        let kernel = gaussian_kernel(radius, sigma);

        // 3. Separable blur: horizontal pass.
        let mut h_pass: Vec<[f32; 3]> = vec![[0.0; 3]; total];
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0.0f32; 3];
                for (ki, &weight) in kernel.iter().enumerate() {
                    let sx = (x as isize + ki as isize - radius as isize)
                        .max(0)
                        .min(w as isize - 1) as usize;
                    let src = bright[y * w + sx];
                    sum[0] += src[0] * weight;
                    sum[1] += src[1] * weight;
                    sum[2] += src[2] * weight;
                }
                h_pass[y * w + x] = sum;
            }
        }

        // 4. Vertical pass.
        let mut blurred: Vec<[f32; 3]> = vec![[0.0; 3]; total];
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0.0f32; 3];
                for (ki, &weight) in kernel.iter().enumerate() {
                    let sy = (y as isize + ki as isize - radius as isize)
                        .max(0)
                        .min(h as isize - 1) as usize;
                    let src = h_pass[sy * w + x];
                    sum[0] += src[0] * weight;
                    sum[1] += src[1] * weight;
                    sum[2] += src[2] * weight;
                }
                blurred[y * w + x] = sum;
            }
        }

        // 5. Additive blend back.
        let intensity = config.intensity;
        for i in 0..total {
            let base = i * 4;
            let r = (pixels[base] as f32 / 255.0 + blurred[i][0] * intensity).min(1.0);
            let g = (pixels[base + 1] as f32 / 255.0 + blurred[i][1] * intensity).min(1.0);
            let b = (pixels[base + 2] as f32 / 255.0 + blurred[i][2] * intensity).min(1.0);
            pixels[base] = (r * 255.0).round() as u8;
            pixels[base + 1] = (g * 255.0).round() as u8;
            pixels[base + 2] = (b * 255.0).round() as u8;
        }
    }

    // ── Vignette ─────────────────────────────────────────────────────────

    /// Darken edges using distance from the centre.
    pub fn apply_vignette(
        pixels: &mut [u8],
        width: u32,
        height: u32,
        config: &VignetteConfig,
    ) {
        if !config.enabled {
            return;
        }
        let w = width as f32;
        let h = height as f32;
        let cx = w * 0.5;
        let cy = h * 0.5;
        // Normalise so corners are at distance 1.0.
        let max_dist = (cx * cx + cy * cy).sqrt();

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt() / max_dist;
                // Smooth power-curve falloff.
                let factor = 1.0 - (config.intensity * dist.powf(config.smoothness));
                let factor = factor.clamp(0.0, 1.0);

                let base = ((y * width + x) * 4) as usize;
                for c in 0..3 {
                    let original = pixels[base + c] as f32 / 255.0;
                    let vig_color = config.color[c];
                    // Lerp toward the vignette colour as factor decreases.
                    let result = original * factor + vig_color * (1.0 - factor);
                    pixels[base + c] = (result.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
    }

    // ── Colour grading ───────────────────────────────────────────────────

    /// Apply exposure, contrast, saturation, temperature/tint, and
    /// lift/gamma/gain to every pixel.
    pub fn apply_color_grading(
        pixels: &mut [u8],
        width: u32,
        height: u32,
        config: &ColorGradingConfig,
    ) {
        if !config.enabled {
            return;
        }
        let total = (width * height) as usize;
        for i in 0..total {
            let base = i * 4;
            let mut r = pixels[base] as f32 / 255.0;
            let mut g = pixels[base + 1] as f32 / 255.0;
            let mut b = pixels[base + 2] as f32 / 255.0;

            // Exposure (linear multiply).
            r *= config.exposure;
            g *= config.exposure;
            b *= config.exposure;

            // Contrast (S-curve around 0.5).
            r = apply_contrast(r, config.contrast);
            g = apply_contrast(g, config.contrast);
            b = apply_contrast(b, config.contrast);

            // Saturation.
            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            r = lum + config.saturation * (r - lum);
            g = lum + config.saturation * (g - lum);
            b = lum + config.saturation * (b - lum);

            // Temperature / tint.
            r += config.temperature * 0.1;
            b -= config.temperature * 0.1;
            g += config.tint * 0.1;

            // Lift (shadows).
            r += config.lift[0];
            g += config.lift[1];
            b += config.lift[2];

            // Gamma (mid-tones).
            if config.gamma[0] > 0.0 {
                r = r.max(0.0).powf(1.0 / config.gamma[0]);
            }
            if config.gamma[1] > 0.0 {
                g = g.max(0.0).powf(1.0 / config.gamma[1]);
            }
            if config.gamma[2] > 0.0 {
                b = b.max(0.0).powf(1.0 / config.gamma[2]);
            }

            // Gain (highlights).
            r *= config.gain[0];
            g *= config.gain[1];
            b *= config.gain[2];

            pixels[base] = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixels[base + 1] = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixels[base + 2] = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    // ── Depth of field ───────────────────────────────────────────────────

    /// Apply depth-of-field blur. Returns a new RGBA8 buffer.
    ///
    /// `depth` is a per-pixel depth buffer (linear depth in world units)
    /// with one entry per pixel (width * height).
    pub fn apply_dof(
        pixels: &[u8],
        depth: &[f32],
        width: u32,
        height: u32,
        config: &DepthOfFieldConfig,
    ) -> Vec<u8> {
        if !config.enabled {
            return pixels.to_vec();
        }
        let w = width as usize;
        let h = height as usize;
        let total = w * h;
        assert!(pixels.len() >= total * 4);
        assert!(depth.len() >= total);

        // Compute circle-of-confusion for every pixel.
        let coc: Vec<f32> = depth
            .iter()
            .map(|&d| circle_of_confusion(d, config))
            .collect();

        let mut output = pixels.to_vec();

        // Variable-radius disc blur per pixel (approximation).
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let radius = coc[idx].round().max(0.0) as usize;
                if radius == 0 {
                    continue;
                }
                let radius = radius.min(16); // cap for performance
                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut weight_sum = 0.0f32;

                let r2 = (radius * radius) as f32;
                for dy in -(radius as isize)..=(radius as isize) {
                    for dx in -(radius as isize)..=(radius as isize) {
                        let d2 = (dx * dx + dy * dy) as f32;
                        if d2 > r2 {
                            continue;
                        }
                        let sx = (x as isize + dx).max(0).min(w as isize - 1) as usize;
                        let sy = (y as isize + dy).max(0).min(h as isize - 1) as usize;
                        let si = sy * w + sx;
                        let weight = 1.0; // uniform disc
                        sum_r += pixels[si * 4] as f32 * weight;
                        sum_g += pixels[si * 4 + 1] as f32 * weight;
                        sum_b += pixels[si * 4 + 2] as f32 * weight;
                        weight_sum += weight;
                    }
                }
                if weight_sum > 0.0 {
                    let oi = idx * 4;
                    output[oi] = (sum_r / weight_sum).round().min(255.0) as u8;
                    output[oi + 1] = (sum_g / weight_sum).round().min(255.0) as u8;
                    output[oi + 2] = (sum_b / weight_sum).round().min(255.0) as u8;
                    // Alpha stays unchanged.
                }
            }
        }
        output
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Build a normalised 1-D Gaussian kernel of size `2 * radius + 1`.
fn gaussian_kernel(radius: usize, sigma: f32) -> Vec<f32> {
    let size = 2 * radius + 1;
    let mut kernel = Vec::with_capacity(size);
    let two_sigma2 = 2.0 * sigma * sigma;
    let mut total = 0.0f32;
    for i in 0..size {
        let x = i as f32 - radius as f32;
        let w = (-x * x / two_sigma2).exp();
        kernel.push(w);
        total += w;
    }
    // Normalise.
    for w in &mut kernel {
        *w /= total;
    }
    kernel
}

/// Contrast adjustment around 0.5 using a simple power curve.
fn apply_contrast(value: f32, contrast: f32) -> f32 {
    let v = value.clamp(0.0, 1.0);
    ((v - 0.5) * contrast + 0.5).clamp(0.0, 1.0)
}

/// Compute the circle of confusion in pixels for a given depth.
fn circle_of_confusion(depth: f32, config: &DepthOfFieldConfig) -> f32 {
    let f = config.focal_length * 0.001; // mm -> m
    let a = f / config.aperture;
    let s = config.focus_distance;
    let d = depth.max(0.001);
    // Thin-lens formula: CoC = |A * f * (S - D) / (D * (S - f))|
    let sf = s - f;
    if sf.abs() < 1e-6 {
        return 0.0;
    }
    let coc = (a * f * (s - d) / (d * sf)).abs();
    // Scale to pixels (assume a ~36mm sensor mapped to width).
    let sensor_mm = 36.0;
    let pixels_per_mm = 1000.0; // rough mapping
    (coc / (sensor_mm * 0.001) * pixels_per_mm).min(32.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a solid-colour RGBA8 image.
    fn solid_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
        let total = (w * h) as usize;
        let mut buf = Vec::with_capacity(total * 4);
        for _ in 0..total {
            buf.push(r);
            buf.push(g);
            buf.push(b);
            buf.push(a);
        }
        buf
    }

    // ── Default configs ──────────────────────────────────────────────────

    #[test]
    fn default_config_has_effects_disabled() {
        let cfg = PostProcessConfig::default();
        assert!(cfg.enabled);
        assert!(!cfg.motion_blur.enabled);
        assert!(!cfg.dof.enabled);
        assert!(!cfg.bloom.enabled);
        assert!(!cfg.vignette.enabled);
        assert!(!cfg.color_grading.enabled);
    }

    #[test]
    fn clean_preset_disables_everything() {
        let cfg = clean();
        assert!(!cfg.enabled);
        assert!(!cfg.bloom.enabled);
        assert!(!cfg.vignette.enabled);
        assert!(!cfg.color_grading.enabled);
        assert!(!cfg.motion_blur.enabled);
        assert!(!cfg.dof.enabled);
    }

    #[test]
    fn cinematic_preset_enables_expected_effects() {
        let cfg = cinematic();
        assert!(cfg.enabled);
        assert!(cfg.bloom.enabled);
        assert!(cfg.vignette.enabled);
        assert!(cfg.color_grading.enabled);
        assert!(!cfg.motion_blur.enabled);
        assert!(!cfg.dof.enabled);
    }

    #[test]
    fn documentary_preset_enables_expected_effects() {
        let cfg = documentary();
        assert!(cfg.enabled);
        assert!(cfg.vignette.enabled);
        assert!(cfg.color_grading.enabled);
        assert!(cfg.color_grading.saturation < 1.0);
    }

    #[test]
    fn game_preview_preset_enables_expected_effects() {
        let cfg = game_preview();
        assert!(cfg.enabled);
        assert!(cfg.bloom.enabled);
        assert!(cfg.motion_blur.enabled);
    }

    // ── Pipeline: global off ─────────────────────────────────────────────

    #[test]
    fn apply_does_nothing_when_disabled() {
        let cfg = PostProcessConfig {
            enabled: false,
            vignette: VignetteConfig {
                enabled: true,
                intensity: 1.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let pipeline = PostProcessPipeline::new(cfg);
        let mut pixels = solid_image(4, 4, 200, 200, 200, 255);
        let original = pixels.clone();
        pipeline.apply(&mut pixels, 4, 4);
        assert_eq!(pixels, original);
    }

    // ── Vignette ─────────────────────────────────────────────────────────

    #[test]
    fn vignette_darkens_corners() {
        let w = 8u32;
        let h = 8u32;
        let mut pixels = solid_image(w, h, 200, 200, 200, 255);
        let config = VignetteConfig {
            enabled: true,
            intensity: 0.8,
            smoothness: 2.0,
            color: [0.0, 0.0, 0.0],
        };
        PostProcessPipeline::apply_vignette(&mut pixels, w, h, &config);
        // The centre pixel should be brighter than the corner pixel.
        let centre = ((4 * w + 4) * 4) as usize;
        let corner = 0usize; // (0, 0)
        assert!(pixels[centre] > pixels[corner]);
    }

    #[test]
    fn vignette_centre_unchanged_with_low_intensity() {
        let w = 16u32;
        let h = 16u32;
        let mut pixels = solid_image(w, h, 128, 128, 128, 255);
        let config = VignetteConfig {
            enabled: true,
            intensity: 0.01,
            smoothness: 2.0,
            color: [0.0, 0.0, 0.0],
        };
        PostProcessPipeline::apply_vignette(&mut pixels, w, h, &config);
        // Centre pixel should remain very close to 128.
        let cx = w / 2;
        let cy = h / 2;
        let idx = ((cy * w + cx) * 4) as usize;
        let diff = (pixels[idx] as i32 - 128).unsigned_abs();
        assert!(diff <= 2, "Centre pixel drifted by {diff}");
    }

    // ── Colour grading ───────────────────────────────────────────────────

    #[test]
    fn color_grading_exposure_brightens() {
        let w = 2u32;
        let h = 2u32;
        let mut pixels = solid_image(w, h, 100, 100, 100, 255);
        let config = ColorGradingConfig {
            enabled: true,
            exposure: 2.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            lift: [0.0; 3],
            gamma: [1.0; 3],
            gain: [1.0; 3],
        };
        PostProcessPipeline::apply_color_grading(&mut pixels, w, h, &config);
        // 100 / 255 * 2.0 = ~0.784 -> ~200
        assert!(pixels[0] > 150, "Expected brighter, got {}", pixels[0]);
    }

    #[test]
    fn color_grading_desaturation() {
        let w = 1u32;
        let h = 1u32;
        // A saturated red pixel.
        let mut pixels = vec![255, 0, 0, 255];
        let config = ColorGradingConfig {
            enabled: true,
            exposure: 1.0,
            contrast: 1.0,
            saturation: 0.0, // full desaturation
            temperature: 0.0,
            tint: 0.0,
            lift: [0.0; 3],
            gamma: [1.0; 3],
            gain: [1.0; 3],
        };
        PostProcessPipeline::apply_color_grading(&mut pixels, w, h, &config);
        // All channels should converge toward luminance.
        let r = pixels[0];
        let g = pixels[1];
        let b = pixels[2];
        // With full desaturation, r, g, b should be roughly equal.
        assert!((r as i32 - g as i32).unsigned_abs() < 5);
        assert!((g as i32 - b as i32).unsigned_abs() < 5);
    }

    // ── Motion blur ──────────────────────────────────────────────────────

    #[test]
    fn motion_blur_blends_frames() {
        let w = 2u32;
        let h = 2u32;
        let mut current = solid_image(w, h, 200, 200, 200, 255);
        let previous = solid_image(w, h, 100, 100, 100, 255);
        let config = MotionBlurConfig {
            enabled: true,
            intensity: 0.5,
            samples: 8,
            max_blur_pixels: 20.0,
        };
        PostProcessPipeline::apply_motion_blur(&mut current, &previous, w, h, &config);
        // 200 * 0.5 + 100 * 0.5 = 150
        assert_eq!(current[0], 150);
    }

    // ── Bloom ────────────────────────────────────────────────────────────

    #[test]
    fn bloom_does_not_darken_image() {
        let w = 4u32;
        let h = 4u32;
        let mut pixels = solid_image(w, h, 255, 255, 255, 255);
        let original = pixels.clone();
        let config = BloomConfig {
            enabled: true,
            threshold: 0.5,
            intensity: 0.3,
            radius: 2.0,
        };
        PostProcessPipeline::apply_bloom(&mut pixels, w, h, &config);
        // Bloom is additive; result should be >= original.
        for i in (0..pixels.len()).step_by(4) {
            assert!(pixels[i] >= original[i].saturating_sub(1));
        }
    }

    // ── DOF ──────────────────────────────────────────────────────────────

    #[test]
    fn dof_preserves_in_focus_region() {
        let w = 4u32;
        let h = 4u32;
        let pixels = solid_image(w, h, 128, 128, 128, 255);
        // All pixels at the exact focus distance -> CoC ~0.
        let depth = vec![5.0f32; (w * h) as usize];
        let config = DepthOfFieldConfig {
            enabled: true,
            focus_distance: 5.0,
            aperture: 2.8,
            focal_length: 50.0,
            bokeh_shape: BokehShape::Circle,
        };
        let result = PostProcessPipeline::apply_dof(&pixels, &depth, w, h, &config);
        // Should be identical since CoC is zero at focus distance.
        assert_eq!(result, pixels);
    }

    // ── Gaussian kernel ──────────────────────────────────────────────────

    #[test]
    fn gaussian_kernel_sums_to_one() {
        let k = gaussian_kernel(5, 2.5);
        let sum: f32 = k.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "Kernel sum = {sum}");
    }

    #[test]
    fn gaussian_kernel_is_symmetric() {
        let k = gaussian_kernel(4, 2.0);
        let n = k.len();
        for i in 0..n / 2 {
            assert!(
                (k[i] - k[n - 1 - i]).abs() < 1e-6,
                "Kernel not symmetric at index {i}"
            );
        }
    }
}
