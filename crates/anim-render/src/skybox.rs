//! Skybox / environment system — procedural sky colors and HDR environment data.

use glam::Vec3;

/// Sky rendering mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkyMode {
    /// Flat single color everywhere.
    SolidColor,
    /// Top -> horizon -> ground gradient based on vertical angle.
    Gradient,
    /// Simple atmospheric-scattering approximation (Rayleigh-like).
    Procedural,
    /// HDR cubemap loaded from disk (future).
    Cubemap,
}

/// Sky environment configuration.
///
/// Describes sun position, sky colors, fog, and the rendering mode used to
/// produce environment lighting and the background clear color.
#[derive(Debug, Clone)]
pub struct SkyEnvironment {
    pub mode: SkyMode,
    pub sun_direction: Vec3,
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub sky_color: [f32; 3],
    pub horizon_color: [f32; 3],
    pub ground_color: [f32; 3],
    pub exposure: f32,
    pub fog_enabled: bool,
    pub fog_density: f32,
    pub fog_color: [f32; 3],
}

impl Default for SkyEnvironment {
    /// Warm sunset defaults — pleasant out-of-the-box look.
    fn default() -> Self {
        Self::sunset()
    }
}

impl SkyEnvironment {
    // ------------------------------------------------------------------ presets

    /// Bright midday sun, blue sky.
    pub fn daylight() -> Self {
        Self {
            mode: SkyMode::Gradient,
            sun_direction: Vec3::new(0.2, -0.9, 0.1).normalize(),
            sun_color: [1.0, 0.98, 0.92],
            sun_intensity: 3.0,
            sky_color: [0.33, 0.55, 0.85],
            horizon_color: [0.70, 0.80, 0.90],
            ground_color: [0.25, 0.22, 0.18],
            exposure: 1.0,
            fog_enabled: false,
            fog_density: 0.0,
            fog_color: [0.7, 0.8, 0.9],
        }
    }

    /// Golden-hour sunset with warm tones.
    pub fn sunset() -> Self {
        Self {
            mode: SkyMode::Gradient,
            sun_direction: Vec3::new(-0.5, -0.2, 0.3).normalize(),
            sun_color: [1.0, 0.65, 0.25],
            sun_intensity: 2.5,
            sky_color: [0.15, 0.18, 0.40],
            horizon_color: [0.90, 0.50, 0.25],
            ground_color: [0.12, 0.10, 0.08],
            exposure: 1.2,
            fog_enabled: true,
            fog_density: 0.002,
            fog_color: [0.85, 0.55, 0.30],
        }
    }

    /// Dark night sky with a dim moon.
    pub fn night() -> Self {
        Self {
            mode: SkyMode::Gradient,
            sun_direction: Vec3::new(0.3, -0.8, -0.2).normalize(),
            sun_color: [0.6, 0.7, 0.9],
            sun_intensity: 0.3,
            sky_color: [0.02, 0.02, 0.06],
            horizon_color: [0.04, 0.04, 0.08],
            ground_color: [0.01, 0.01, 0.02],
            exposure: 2.5,
            fog_enabled: false,
            fog_density: 0.0,
            fog_color: [0.02, 0.02, 0.04],
        }
    }

    /// Overcast / cloudy — soft diffuse illumination.
    pub fn overcast() -> Self {
        Self {
            mode: SkyMode::Gradient,
            sun_direction: Vec3::new(0.0, -1.0, 0.0).normalize(),
            sun_color: [0.85, 0.85, 0.85],
            sun_intensity: 1.2,
            sky_color: [0.55, 0.58, 0.62],
            horizon_color: [0.62, 0.64, 0.66],
            ground_color: [0.30, 0.30, 0.30],
            exposure: 1.0,
            fog_enabled: true,
            fog_density: 0.005,
            fog_color: [0.60, 0.62, 0.64],
        }
    }

    /// Neutral gray studio backdrop — ideal for product / character shots.
    pub fn studio() -> Self {
        Self {
            mode: SkyMode::SolidColor,
            sun_direction: Vec3::new(0.0, -1.0, 0.0),
            sun_color: [1.0, 1.0, 1.0],
            sun_intensity: 0.0,
            sky_color: [0.22, 0.22, 0.22],
            horizon_color: [0.22, 0.22, 0.22],
            ground_color: [0.22, 0.22, 0.22],
            exposure: 1.0,
            fog_enabled: false,
            fog_density: 0.0,
            fog_color: [0.22, 0.22, 0.22],
        }
    }

    // ----------------------------------------------------------------- sampling

    /// Sample the sky color for a given world-space direction.
    ///
    /// Used by environment-lighting passes and the cubemap generator.
    pub fn sample(&self, direction: Vec3) -> [f32; 3] {
        let dir = direction.normalize_or_zero();
        if dir == Vec3::ZERO {
            return self.sky_color;
        }

        let base = match self.mode {
            SkyMode::SolidColor => self.sky_color,
            SkyMode::Gradient => self.sample_gradient(dir),
            SkyMode::Procedural => self.sample_procedural(dir),
            SkyMode::Cubemap => {
                // Cubemap look-up is a future GPU path; fall back to gradient.
                self.sample_gradient(dir)
            }
        };

        // Apply exposure tone-mapping (simple Reinhard per-channel).
        [
            base[0] * self.exposure,
            base[1] * self.exposure,
            base[2] * self.exposure,
        ]
    }

    /// Three-band gradient: ground -> horizon -> sky based on the Y component.
    fn sample_gradient(&self, dir: Vec3) -> [f32; 3] {
        let y = dir.y; // -1 = nadir, 0 = horizon, +1 = zenith
        if y >= 0.0 {
            // horizon -> sky
            let t = y.clamp(0.0, 1.0);
            lerp_color(&self.horizon_color, &self.sky_color, t)
        } else {
            // horizon -> ground
            let t = (-y).clamp(0.0, 1.0);
            lerp_color(&self.horizon_color, &self.ground_color, t)
        }
    }

    /// Cheap atmospheric scattering approximation (Rayleigh-ish).
    fn sample_procedural(&self, dir: Vec3) -> [f32; 3] {
        // Start with the gradient as base.
        let base = self.sample_gradient(dir);

        // Add a sun disc and glow around the sun direction.
        let sun_dir = (-self.sun_direction).normalize_or_zero();
        let cos_angle = dir.dot(sun_dir).max(0.0);

        // Tight sun disc.
        let sun_disc = cos_angle.powf(512.0) * self.sun_intensity * 2.0;
        // Broad atmospheric glow.
        let sun_glow = cos_angle.powf(8.0) * self.sun_intensity * 0.15;

        let factor = sun_disc + sun_glow;
        [
            base[0] + self.sun_color[0] * factor,
            base[1] + self.sun_color[1] * factor,
            base[2] + self.sun_color[2] * factor,
        ]
    }

    // -------------------------------------------------------------- cubemap gen

    /// Generate a small CPU-side cubemap (6 faces, each `size * size` pixels).
    ///
    /// Face order: +X, -X, +Y, -Y, +Z, -Z.
    /// Each pixel is an `[f32; 3]` RGB triplet.
    pub fn generate_cubemap(&self, size: u32) -> Vec<Vec<[f32; 3]>> {
        let s = size as usize;
        let pixel_count = s * s;

        // Direction basis for each cube face.
        let faces: [(Vec3, Vec3, Vec3); 6] = [
            (Vec3::X, Vec3::Y, -Vec3::Z),   // +X
            (-Vec3::X, Vec3::Y, Vec3::Z),    // -X
            (Vec3::Y, Vec3::Z, Vec3::X),     // +Y (up)
            (-Vec3::Y, -Vec3::Z, Vec3::X),   // -Y (down)
            (Vec3::Z, Vec3::Y, Vec3::X),     // +Z
            (-Vec3::Z, Vec3::Y, -Vec3::X),   // -Z
        ];

        faces
            .iter()
            .map(|&(forward, up, right)| {
                let mut pixels = Vec::with_capacity(pixel_count);
                for row in 0..s {
                    for col in 0..s {
                        let u = (col as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                        let v = (row as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                        let dir = (forward + right * u + up * v).normalize();
                        pixels.push(self.sample(dir));
                    }
                }
                pixels
            })
            .collect()
    }
}

// --------------------------------------------------------------------- helpers

/// Linear interpolation between two RGB colors.
fn lerp_color(a: &[f32; 3], b: &[f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

// ----------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_sunset() {
        let env = SkyEnvironment::default();
        assert_eq!(env.mode, SkyMode::Gradient);
        assert!(env.sun_intensity > 0.0);
    }

    #[test]
    fn presets_have_valid_sun_direction() {
        for env in &[
            SkyEnvironment::daylight(),
            SkyEnvironment::sunset(),
            SkyEnvironment::night(),
            SkyEnvironment::overcast(),
            SkyEnvironment::studio(),
        ] {
            let len = env.sun_direction.length();
            assert!(
                (len - 1.0).abs() < 0.01 || len == 0.0,
                "sun_direction should be normalized, got length {}",
                len,
            );
        }
    }

    #[test]
    fn solid_color_returns_sky_color() {
        let env = SkyEnvironment::studio();
        let c = env.sample(Vec3::Y);
        // Studio sky_color is 0.22 * exposure 1.0.
        assert!((c[0] - 0.22).abs() < 0.01);
        assert!((c[1] - 0.22).abs() < 0.01);
        assert!((c[2] - 0.22).abs() < 0.01);
    }

    #[test]
    fn gradient_zenith_is_sky_color() {
        let env = SkyEnvironment::daylight();
        let c = env.sample(Vec3::Y);
        // At zenith, t=1.0 so we expect sky_color * exposure.
        let expected = [
            env.sky_color[0] * env.exposure,
            env.sky_color[1] * env.exposure,
            env.sky_color[2] * env.exposure,
        ];
        for i in 0..3 {
            assert!(
                (c[i] - expected[i]).abs() < 0.01,
                "channel {} mismatch: {} vs {}",
                i,
                c[i],
                expected[i],
            );
        }
    }

    #[test]
    fn gradient_nadir_is_ground_color() {
        let env = SkyEnvironment::daylight();
        let c = env.sample(-Vec3::Y);
        let expected = [
            env.ground_color[0] * env.exposure,
            env.ground_color[1] * env.exposure,
            env.ground_color[2] * env.exposure,
        ];
        for i in 0..3 {
            assert!(
                (c[i] - expected[i]).abs() < 0.01,
                "channel {} mismatch: {} vs {}",
                i,
                c[i],
                expected[i],
            );
        }
    }

    #[test]
    fn cubemap_has_six_faces() {
        let env = SkyEnvironment::studio();
        let cubemap = env.generate_cubemap(4);
        assert_eq!(cubemap.len(), 6);
        for face in &cubemap {
            assert_eq!(face.len(), 16); // 4*4
        }
    }

    #[test]
    fn procedural_mode_adds_sun() {
        let mut env = SkyEnvironment::daylight();
        env.mode = SkyMode::Procedural;
        // Sample directly toward the sun — should be brighter than away from it.
        let toward_sun = env.sample(-env.sun_direction);
        let away_from_sun = env.sample(env.sun_direction);
        let brightness = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(
            brightness(toward_sun) > brightness(away_from_sun),
            "looking at sun should be brighter than looking away",
        );
    }

    #[test]
    fn zero_direction_does_not_panic() {
        let env = SkyEnvironment::default();
        let _ = env.sample(Vec3::ZERO);
    }
}
