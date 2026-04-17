//! Multi-light system — point, spot, and directional lights.

use glam::Vec3;

/// Discriminant for the three supported light shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// Infinitely far away, parallel rays (e.g. the sun).
    Directional,
    /// Omni-directional point emitter with distance falloff.
    Point,
    /// Cone-shaped emitter with inner / outer angle falloff.
    Spot,
}

/// A single light source in the scene.
#[derive(Debug, Clone)]
pub struct Light {
    pub name: String,
    pub light_type: LightType,
    /// World-space position (ignored for directional lights).
    pub position: Vec3,
    /// Direction the light points toward (ignored for point lights).
    pub direction: Vec3,
    pub color: [f32; 3],
    pub intensity: f32,
    /// Maximum influence radius for point / spot lights (0 = unlimited).
    pub range: f32,
    /// Spot inner cone half-angle in radians (full brightness inside).
    pub inner_angle: f32,
    /// Spot outer cone half-angle in radians (falloff to zero outside).
    pub outer_angle: f32,
    pub cast_shadows: bool,
    pub enabled: bool,
}

impl Light {
    // ---------------------------------------------------------------- factories

    /// Create a directional light (sun / moon).
    pub fn directional(
        name: &str,
        direction: Vec3,
        color: [f32; 3],
        intensity: f32,
    ) -> Self {
        Self {
            name: name.to_string(),
            light_type: LightType::Directional,
            position: Vec3::ZERO,
            direction: direction.normalize_or_zero(),
            color,
            intensity,
            range: 0.0,
            inner_angle: 0.0,
            outer_angle: 0.0,
            cast_shadows: true,
            enabled: true,
        }
    }

    /// Create an omni-directional point light.
    pub fn point(
        name: &str,
        position: Vec3,
        color: [f32; 3],
        intensity: f32,
        range: f32,
    ) -> Self {
        Self {
            name: name.to_string(),
            light_type: LightType::Point,
            position,
            direction: Vec3::NEG_Y,
            color,
            intensity,
            range,
            inner_angle: 0.0,
            outer_angle: 0.0,
            cast_shadows: false,
            enabled: true,
        }
    }

    /// Create a spot light.
    ///
    /// `angle` is the outer cone half-angle in radians; the inner angle is set
    /// to 80 % of outer by default.
    pub fn spot(
        name: &str,
        position: Vec3,
        direction: Vec3,
        color: [f32; 3],
        intensity: f32,
        range: f32,
        angle: f32,
    ) -> Self {
        Self {
            name: name.to_string(),
            light_type: LightType::Spot,
            position,
            direction: direction.normalize_or_zero(),
            color,
            intensity,
            range,
            inner_angle: angle * 0.8,
            outer_angle: angle,
            cast_shadows: true,
            enabled: true,
        }
    }

    // --------------------------------------------------------------- shading

    /// Compute the light contribution at `world_pos` for a surface with the
    /// given `normal`.
    ///
    /// Returns an RGB color (linear, pre-multiplied by intensity).  This is a
    /// simple diffuse (Lambertian) evaluation intended for debug / preview
    /// overlays — the real lighting is done on the GPU.
    pub fn illuminate(&self, world_pos: Vec3, normal: Vec3) -> [f32; 3] {
        if !self.enabled || self.intensity <= 0.0 {
            return [0.0; 3];
        }

        let n = normal.normalize_or_zero();
        if n == Vec3::ZERO {
            return [0.0; 3];
        }

        match self.light_type {
            LightType::Directional => {
                let l = (-self.direction).normalize_or_zero();
                let n_dot_l = n.dot(l).max(0.0);
                scale_color(&self.color, self.intensity * n_dot_l)
            }
            LightType::Point => {
                let to_light = self.position - world_pos;
                let dist = to_light.length();
                if dist < 1e-6 {
                    return scale_color(&self.color, self.intensity);
                }
                let l = to_light / dist;
                let n_dot_l = n.dot(l).max(0.0);
                let atten = attenuation(dist, self.range);
                scale_color(&self.color, self.intensity * n_dot_l * atten)
            }
            LightType::Spot => {
                let to_light = self.position - world_pos;
                let dist = to_light.length();
                if dist < 1e-6 {
                    return scale_color(&self.color, self.intensity);
                }
                let l = to_light / dist;
                let n_dot_l = n.dot(l).max(0.0);
                let atten = attenuation(dist, self.range);
                let spot = spot_factor(l, self.direction, self.inner_angle, self.outer_angle);
                scale_color(&self.color, self.intensity * n_dot_l * atten * spot)
            }
        }
    }
}

// ----------------------------------------------------------------- LightScene

/// A collection of lights plus ambient terms, ready to be uploaded to the GPU.
pub struct LightScene {
    pub lights: Vec<Light>,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    /// Maximum number of lights the GPU shader supports.
    pub max_lights: usize,
}

impl Default for LightScene {
    fn default() -> Self {
        Self::new()
    }
}

impl LightScene {
    pub fn new() -> Self {
        Self {
            lights: Vec::new(),
            ambient_color: [0.15, 0.15, 0.18],
            ambient_intensity: 0.3,
            max_lights: 16,
        }
    }

    /// Add a light and return its index.
    pub fn add_light(&mut self, light: Light) -> usize {
        let idx = self.lights.len();
        self.lights.push(light);
        idx
    }

    /// Remove a light by index (swap-remove for speed).
    pub fn remove_light(&mut self, index: usize) {
        if index < self.lights.len() {
            self.lights.swap_remove(index);
        }
    }

    /// Remove all lights.
    pub fn clear(&mut self) {
        self.lights.clear();
    }

    // --------------------------------------------------------------- presets

    /// Classic three-point lighting: key, fill, and rim (back) light.
    pub fn three_point_lighting() -> Self {
        let mut scene = Self::new();
        scene.ambient_intensity = 0.15;

        // Key — bright directional, upper-left.
        scene.add_light(Light::directional(
            "Key",
            Vec3::new(-0.5, -0.7, -0.5).normalize(),
            [1.0, 0.97, 0.90],
            2.5,
        ));

        // Fill — softer, from the right.
        scene.add_light(Light::directional(
            "Fill",
            Vec3::new(0.6, -0.4, -0.3).normalize(),
            [0.60, 0.70, 0.85],
            0.8,
        ));

        // Rim — behind the subject, highlights edges.
        scene.add_light(Light::directional(
            "Rim",
            Vec3::new(0.0, -0.3, 0.9).normalize(),
            [0.90, 0.90, 1.0],
            1.2,
        ));

        scene
    }

    /// Outdoor daylight — single sun plus sky ambient.
    pub fn outdoor_daylight() -> Self {
        let mut scene = Self::new();
        scene.ambient_color = [0.40, 0.50, 0.65];
        scene.ambient_intensity = 0.4;

        scene.add_light(Light::directional(
            "Sun",
            Vec3::new(0.2, -0.85, 0.15).normalize(),
            [1.0, 0.98, 0.92],
            3.0,
        ));

        scene
    }

    /// Studio setup — key + fill directional plus two accent point lights.
    pub fn studio_setup() -> Self {
        let mut scene = Self::new();
        scene.ambient_color = [0.20, 0.20, 0.22];
        scene.ambient_intensity = 0.25;

        scene.add_light(Light::directional(
            "Key",
            Vec3::new(-0.4, -0.7, -0.5).normalize(),
            [1.0, 0.96, 0.90],
            2.2,
        ));
        scene.add_light(Light::directional(
            "Fill",
            Vec3::new(0.5, -0.3, -0.4).normalize(),
            [0.55, 0.60, 0.75],
            0.7,
        ));

        // Warm accent from the left.
        scene.add_light(Light::point(
            "Accent L",
            Vec3::new(-3.0, 2.0, 1.0),
            [1.0, 0.75, 0.40],
            1.5,
            10.0,
        ));

        // Cool accent from the right.
        scene.add_light(Light::point(
            "Accent R",
            Vec3::new(3.0, 1.5, -1.0),
            [0.40, 0.60, 1.0],
            1.2,
            10.0,
        ));

        scene
    }
}

// --------------------------------------------------------------------- helpers

/// Inverse-square attenuation with smooth windowing at `range`.
fn attenuation(distance: f32, range: f32) -> f32 {
    if range > 0.0 && distance >= range {
        return 0.0;
    }
    let inv = 1.0 / (distance * distance + 0.01);
    if range > 0.0 {
        let t = (distance / range).clamp(0.0, 1.0);
        let window = (1.0 - t * t).max(0.0);
        inv * window * window
    } else {
        inv
    }
}

/// Smooth spot cone falloff between `inner` and `outer` half-angles.
fn spot_factor(to_light: Vec3, light_dir: Vec3, inner: f32, outer: f32) -> f32 {
    let cos_angle = (-to_light).dot(light_dir.normalize_or_zero());
    let cos_outer = outer.cos();
    let cos_inner = inner.cos();
    if cos_inner <= cos_outer {
        // Degenerate — treat as point light.
        return 1.0;
    }
    ((cos_angle - cos_outer) / (cos_inner - cos_outer)).clamp(0.0, 1.0)
}

/// Multiply an RGB color by a scalar.
fn scale_color(c: &[f32; 3], s: f32) -> [f32; 3] {
    [c[0] * s, c[1] * s, c[2] * s]
}

// ----------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    // -- factory helpers

    #[test]
    fn directional_direction_is_normalized() {
        let l = Light::directional("d", Vec3::new(1.0, 2.0, 3.0), [1.0; 3], 1.0);
        let len = l.direction.length();
        assert!((len - 1.0).abs() < 1e-4);
    }

    #[test]
    fn spot_inner_angle_is_fraction_of_outer() {
        let l = Light::spot(
            "s",
            Vec3::ZERO,
            Vec3::NEG_Y,
            [1.0; 3],
            1.0,
            5.0,
            FRAC_PI_4,
        );
        assert!((l.inner_angle - FRAC_PI_4 * 0.8).abs() < 1e-5);
    }

    // -- illuminate

    #[test]
    fn directional_light_illuminates_facing_surface() {
        let l = Light::directional("sun", Vec3::NEG_Y, [1.0, 1.0, 1.0], 2.0);
        let c = l.illuminate(Vec3::ZERO, Vec3::Y);
        // normal faces up, light comes from above -> full contribution.
        assert!(c[0] > 1.5, "expected bright, got {:?}", c);
    }

    #[test]
    fn directional_light_zero_on_back_face() {
        let l = Light::directional("sun", Vec3::NEG_Y, [1.0, 1.0, 1.0], 2.0);
        let c = l.illuminate(Vec3::ZERO, Vec3::NEG_Y);
        assert!(c[0].abs() < 1e-6 && c[1].abs() < 1e-6 && c[2].abs() < 1e-6);
    }

    #[test]
    fn point_light_attenuates_with_distance() {
        let l = Light::point("p", Vec3::new(0.0, 5.0, 0.0), [1.0; 3], 10.0, 20.0);
        let close = l.illuminate(Vec3::new(0.0, 4.0, 0.0), Vec3::Y);
        let far = l.illuminate(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
        let brightness = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(
            brightness(close) > brightness(far),
            "closer surface should be brighter: close={:?} far={:?}",
            close,
            far,
        );
    }

    #[test]
    fn spot_light_outside_cone_is_dark() {
        let l = Light::spot(
            "spot",
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::NEG_Y,
            [1.0; 3],
            10.0,
            20.0,
            0.1, // very narrow cone ~5.7 degrees
        );
        // Point far off to the side.
        let c = l.illuminate(Vec3::new(50.0, 0.0, 0.0), Vec3::Y);
        let brightness = c[0] + c[1] + c[2];
        assert!(brightness < 0.01, "outside cone should be dark, got {:?}", c);
    }

    #[test]
    fn disabled_light_returns_zero() {
        let mut l = Light::directional("off", Vec3::NEG_Y, [1.0; 3], 5.0);
        l.enabled = false;
        let c = l.illuminate(Vec3::ZERO, Vec3::Y);
        assert_eq!(c, [0.0, 0.0, 0.0]);
    }

    // -- LightScene

    #[test]
    fn scene_add_and_remove() {
        let mut s = LightScene::new();
        assert_eq!(s.lights.len(), 0);
        let i = s.add_light(Light::point("a", Vec3::ZERO, [1.0; 3], 1.0, 5.0));
        assert_eq!(i, 0);
        assert_eq!(s.lights.len(), 1);
        s.remove_light(0);
        assert_eq!(s.lights.len(), 0);
    }

    #[test]
    fn three_point_has_three_lights() {
        let s = LightScene::three_point_lighting();
        assert_eq!(s.lights.len(), 3);
    }

    #[test]
    fn outdoor_has_one_light() {
        let s = LightScene::outdoor_daylight();
        assert_eq!(s.lights.len(), 1);
        assert_eq!(s.lights[0].light_type, LightType::Directional);
    }

    #[test]
    fn studio_has_four_lights() {
        let s = LightScene::studio_setup();
        assert_eq!(s.lights.len(), 4);
    }

    #[test]
    fn clear_empties_scene() {
        let mut s = LightScene::three_point_lighting();
        s.clear();
        assert!(s.lights.is_empty());
    }

    #[test]
    fn remove_out_of_bounds_is_safe() {
        let mut s = LightScene::new();
        s.remove_light(99); // should not panic
    }

    // -- helpers

    #[test]
    fn attenuation_zero_at_range() {
        assert_eq!(attenuation(10.0, 10.0), 0.0);
    }

    #[test]
    fn attenuation_positive_inside_range() {
        let a = attenuation(2.0, 10.0);
        assert!(a > 0.0);
    }
}
