//! Render settings exposed to the GUI.
//!
//! All deferred-pipeline parameters are gathered here so that
//! the GUI can expose sliders/toggles without touching the renderer internals.

/// Full set of tunable rendering parameters.
pub struct RenderSettings {
    // ── SSAO ─────────────────────────────────────────────
    pub ssao_enabled:   bool,
    pub ssao_radius:    f32,
    pub ssao_bias:      f32,
    pub ssao_intensity: f32,

    // ── Shadows ──────────────────────────────────────────
    pub shadows_enabled: bool,
    pub shadow_bias:     f32,

    // ── Lighting ─────────────────────────────────────────
    pub exposure:         f32,
    pub sun_strength:     f32,
    pub sun_color:        [f32; 3],
    pub sky_strength:     f32,
    pub sky_color:        [f32; 3],
    pub ground_strength:  f32,
    pub ambient_strength: f32,

    /// Light direction as yaw/pitch (radians).
    pub light_yaw:   f32,
    pub light_pitch: f32,

    // ── Bloom ────────────────────────────────────────────
    pub bloom_enabled:   bool,
    pub bloom_intensity: f32,
    pub bloom_spread:    f32,

    // ── FXAA ─────────────────────────────────────────────
    pub fxaa_enabled: bool,
}

impl RenderSettings {
    /// Compute normalised light direction from yaw/pitch.
    pub fn light_direction(&self) -> [f32; 3] {
        let (sy, cy) = self.light_yaw.sin_cos();
        let (sp, cp) = self.light_pitch.sin_cos();
        // Negative Y = pointing downward
        [cp * sy, -sp, cp * cy]
    }
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            ssao_enabled:    true,
            ssao_radius:     0.5,
            ssao_bias:       0.025,
            ssao_intensity:  0.15,

            shadows_enabled: true,
            shadow_bias:     0.005,

            exposure:         0.9,
            sun_strength:     0.25,
            sun_color:        [1.0, 0.95, 0.9],
            sky_strength:     0.15,
            sky_color:        [0.4, 0.45, 0.55],
            ground_strength:  0.1,
            ambient_strength: 1.0,

            light_yaw:   -0.54,  // ≈ atan2(-0.5, -0.3)
            light_pitch:  0.86,  // ≈ asin(1/√1.34)

            bloom_enabled:   true,
            bloom_intensity: 0.35,
            bloom_spread:    1.0,

            fxaa_enabled: true,
        }
    }
}
