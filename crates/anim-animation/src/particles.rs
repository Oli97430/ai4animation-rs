//! Particle system — emitter-based particles for effects (fire, smoke, dust, sparks).
//!
//! CPU-driven particle simulation with basic physics: gravity, drag, wind,
//! lifetime, size/color curves, and emission shapes.

use glam::Vec3;

// ---------------------------------------------------------------------------
// Particle
// ---------------------------------------------------------------------------

/// A single particle.
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub color: [f32; 4],       // RGBA, with alpha fade
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub size: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub life: f32,             // remaining life in seconds
    pub max_life: f32,         // initial lifetime
    pub rotation: f32,         // rotation angle (radians)
    pub angular_velocity: f32, // spin speed
    pub alive: bool,
}

impl Particle {
    fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            color: [1.0, 1.0, 1.0, 1.0],
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 0.0],
            size: 0.1,
            start_size: 0.1,
            end_size: 0.0,
            life: 1.0,
            max_life: 1.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            alive: false,
        }
    }

    /// Normalized age: 0.0 = just born, 1.0 = about to die.
    fn age_ratio(&self) -> f32 {
        if self.max_life > 0.0 {
            1.0 - (self.life / self.max_life)
        } else {
            1.0
        }
    }
}

// ---------------------------------------------------------------------------
// Emission shape
// ---------------------------------------------------------------------------

/// Shape from which particles are emitted.
#[derive(Debug, Clone, Copy)]
pub enum EmissionShape {
    /// Point emitter at origin.
    Point,
    /// Sphere surface with given radius.
    Sphere { radius: f32 },
    /// Box with half-extents.
    Box { half_extents: Vec3 },
    /// Cone pointing up (+Y) with angle in radians and height.
    Cone { angle: f32, height: f32 },
    /// Ring in the XZ plane.
    Ring { radius: f32, thickness: f32 },
}

impl Default for EmissionShape {
    fn default() -> Self {
        EmissionShape::Point
    }
}

// ---------------------------------------------------------------------------
// Emitter configuration
// ---------------------------------------------------------------------------

/// Configuration for a particle emitter.
#[derive(Debug, Clone)]
pub struct EmitterConfig {
    /// Emission rate (particles per second).
    pub rate: f32,
    /// Maximum number of particles alive at once.
    pub max_particles: usize,
    /// Emission shape.
    pub shape: EmissionShape,
    /// World position of the emitter.
    pub position: Vec3,

    // ── Initial particle properties ─────────
    /// Initial speed range [min, max].
    pub speed: [f32; 2],
    /// Lifetime range [min, max] in seconds.
    pub lifetime: [f32; 2],
    /// Start size range [min, max].
    pub start_size: [f32; 2],
    /// End size range [min, max].
    pub end_size: [f32; 2],
    /// Start color RGBA.
    pub start_color: [f32; 4],
    /// End color RGBA (faded out).
    pub end_color: [f32; 4],
    /// Angular velocity range [min, max] in radians/sec.
    pub angular_velocity: [f32; 2],

    // ── Forces ──────────────────────────────
    /// Gravity vector (usually [0, -9.81, 0]).
    pub gravity: Vec3,
    /// Drag coefficient [0..1] — 0 = no drag, 1 = full stop.
    pub drag: f32,
    /// Wind direction and strength.
    pub wind: Vec3,

    // ── Emitter lifecycle ───────────────────
    /// If Some, the emitter stops after this many seconds.
    pub duration: Option<f32>,
    /// Whether to loop the emission.
    pub looping: bool,
    /// If true, emit all particles in one burst at start.
    pub burst: bool,
    /// Number of particles in a burst (if burst mode).
    pub burst_count: usize,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            rate: 50.0,
            max_particles: 500,
            shape: EmissionShape::Point,
            position: Vec3::ZERO,
            speed: [1.0, 3.0],
            lifetime: [1.0, 2.0],
            start_size: [0.05, 0.1],
            end_size: [0.0, 0.02],
            start_color: [1.0, 0.8, 0.3, 1.0], // warm yellow
            end_color: [1.0, 0.2, 0.0, 0.0],    // red, faded out
            angular_velocity: [-1.0, 1.0],
            gravity: Vec3::new(0.0, -2.0, 0.0),
            drag: 0.02,
            wind: Vec3::ZERO,
            duration: None,
            looping: true,
            burst: false,
            burst_count: 20,
        }
    }
}

// ---------------------------------------------------------------------------
// Preset configurations
// ---------------------------------------------------------------------------

impl EmitterConfig {
    /// Fire emitter preset.
    pub fn fire() -> Self {
        Self {
            rate: 80.0,
            max_particles: 400,
            shape: EmissionShape::Sphere { radius: 0.1 },
            speed: [1.0, 3.0],
            lifetime: [0.5, 1.5],
            start_size: [0.08, 0.15],
            end_size: [0.0, 0.02],
            start_color: [1.0, 0.7, 0.1, 1.0],
            end_color: [0.8, 0.1, 0.0, 0.0],
            gravity: Vec3::new(0.0, 2.0, 0.0), // rises
            drag: 0.05,
            ..Default::default()
        }
    }

    /// Smoke emitter preset.
    pub fn smoke() -> Self {
        Self {
            rate: 30.0,
            max_particles: 200,
            shape: EmissionShape::Sphere { radius: 0.15 },
            speed: [0.3, 0.8],
            lifetime: [2.0, 4.0],
            start_size: [0.1, 0.2],
            end_size: [0.5, 0.8],
            start_color: [0.5, 0.5, 0.5, 0.6],
            end_color: [0.3, 0.3, 0.3, 0.0],
            gravity: Vec3::new(0.0, 0.5, 0.0),
            drag: 0.1,
            ..Default::default()
        }
    }

    /// Dust cloud preset.
    pub fn dust() -> Self {
        Self {
            rate: 40.0,
            max_particles: 300,
            shape: EmissionShape::Box { half_extents: Vec3::new(0.5, 0.01, 0.5) },
            speed: [0.2, 0.5],
            lifetime: [1.5, 3.0],
            start_size: [0.02, 0.05],
            end_size: [0.08, 0.15],
            start_color: [0.7, 0.6, 0.4, 0.5],
            end_color: [0.7, 0.6, 0.4, 0.0],
            gravity: Vec3::new(0.0, -0.3, 0.0),
            drag: 0.15,
            ..Default::default()
        }
    }

    /// Sparks preset (short-lived, fast).
    pub fn sparks() -> Self {
        Self {
            rate: 100.0,
            max_particles: 300,
            shape: EmissionShape::Point,
            speed: [3.0, 8.0],
            lifetime: [0.2, 0.6],
            start_size: [0.01, 0.03],
            end_size: [0.0, 0.0],
            start_color: [1.0, 0.9, 0.5, 1.0],
            end_color: [1.0, 0.3, 0.0, 0.0],
            gravity: Vec3::new(0.0, -9.81, 0.0),
            drag: 0.01,
            ..Default::default()
        }
    }

    /// Snow preset.
    pub fn snow() -> Self {
        Self {
            rate: 60.0,
            max_particles: 500,
            shape: EmissionShape::Box { half_extents: Vec3::new(5.0, 0.01, 5.0) },
            position: Vec3::new(0.0, 5.0, 0.0),
            speed: [0.1, 0.3],
            lifetime: [5.0, 8.0],
            start_size: [0.02, 0.04],
            end_size: [0.02, 0.04],
            start_color: [1.0, 1.0, 1.0, 0.9],
            end_color: [1.0, 1.0, 1.0, 0.0],
            gravity: Vec3::new(0.0, -0.5, 0.0),
            drag: 0.3,
            wind: Vec3::new(0.5, 0.0, 0.3),
            ..Default::default()
        }
    }

    /// Rain preset.
    pub fn rain() -> Self {
        Self {
            rate: 200.0,
            max_particles: 1000,
            shape: EmissionShape::Box { half_extents: Vec3::new(5.0, 0.01, 5.0) },
            position: Vec3::new(0.0, 8.0, 0.0),
            speed: [0.0, 0.1],
            lifetime: [0.5, 1.0],
            start_size: [0.01, 0.015],
            end_size: [0.01, 0.015],
            start_color: [0.7, 0.8, 1.0, 0.6],
            end_color: [0.7, 0.8, 1.0, 0.0],
            gravity: Vec3::new(0.0, -15.0, 0.0),
            drag: 0.0,
            wind: Vec3::new(1.0, 0.0, 0.0),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Particle emitter
// ---------------------------------------------------------------------------

/// A particle emitter manages a pool of particles.
pub struct ParticleEmitter {
    pub config: EmitterConfig,
    pub particles: Vec<Particle>,
    /// Accumulated time for emission rate.
    emit_accumulator: f32,
    /// Total elapsed time.
    elapsed: f32,
    /// Whether the emitter is active.
    pub active: bool,
    /// Simple RNG state (xorshift).
    rng_state: u32,
}

impl ParticleEmitter {
    pub fn new(config: EmitterConfig) -> Self {
        let max = config.max_particles;
        Self {
            config,
            particles: Vec::with_capacity(max),
            emit_accumulator: 0.0,
            elapsed: 0.0,
            active: true,
            rng_state: 12345,
        }
    }

    /// Simple pseudo-random float in [0, 1].
    fn rand_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32) / (u32::MAX as f32)
    }

    /// Random float in [min, max].
    fn rand_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.rand_f32() * (max - min)
    }

    /// Random direction on a unit sphere.
    fn rand_direction(&mut self) -> Vec3 {
        let theta = self.rand_f32() * std::f32::consts::TAU;
        let phi = (1.0 - 2.0 * self.rand_f32()).acos();
        Vec3::new(
            phi.sin() * theta.cos(),
            phi.sin() * theta.sin(),
            phi.cos(),
        )
    }

    /// Emit a single particle.
    fn emit_particle(&mut self) {
        if self.particles.len() >= self.config.max_particles {
            // Recycle dead particle
            let new_p = self.create_particle();
            if let Some(p) = self.particles.iter_mut().find(|p| !p.alive) {
                *p = new_p;
            }
            return;
        }
        let new_p = self.create_particle();
        self.particles.push(new_p);
    }

    /// Create a new particle according to config.
    fn create_particle(&mut self) -> Particle {
        let mut p = Particle::new();

        // Position based on emission shape
        let offset = match self.config.shape {
            EmissionShape::Point => Vec3::ZERO,
            EmissionShape::Sphere { radius } => {
                self.rand_direction() * radius * self.rand_f32()
            }
            EmissionShape::Box { half_extents } => {
                Vec3::new(
                    self.rand_range(-half_extents.x, half_extents.x),
                    self.rand_range(-half_extents.y, half_extents.y),
                    self.rand_range(-half_extents.z, half_extents.z),
                )
            }
            EmissionShape::Cone { angle, height } => {
                let r = self.rand_f32() * angle;
                let theta = self.rand_f32() * std::f32::consts::TAU;
                let h = self.rand_f32() * height;
                Vec3::new(r.sin() * theta.cos() * h, h, r.sin() * theta.sin() * h)
            }
            EmissionShape::Ring { radius, thickness } => {
                let theta = self.rand_f32() * std::f32::consts::TAU;
                let r = radius + self.rand_range(-thickness * 0.5, thickness * 0.5);
                Vec3::new(r * theta.cos(), 0.0, r * theta.sin())
            }
        };

        p.position = self.config.position + offset;

        // Velocity: random direction × speed
        let dir = self.rand_direction();
        let speed = self.rand_range(self.config.speed[0], self.config.speed[1]);
        p.velocity = dir * speed;

        // Lifetime
        p.life = self.rand_range(self.config.lifetime[0], self.config.lifetime[1]);
        p.max_life = p.life;

        // Size
        p.start_size = self.rand_range(self.config.start_size[0], self.config.start_size[1]);
        p.end_size = self.rand_range(self.config.end_size[0], self.config.end_size[1]);
        p.size = p.start_size;

        // Color
        p.start_color = self.config.start_color;
        p.end_color = self.config.end_color;
        p.color = p.start_color;

        // Rotation
        p.angular_velocity = self.rand_range(
            self.config.angular_velocity[0],
            self.config.angular_velocity[1],
        );

        p.alive = true;
        p
    }

    /// Update all particles and emit new ones.
    pub fn update(&mut self, dt: f32) {
        if !self.active {
            return;
        }

        self.elapsed += dt;

        // Check duration
        if let Some(dur) = self.config.duration {
            if !self.config.looping && self.elapsed >= dur {
                self.active = false;
            }
        }

        // Emit particles
        if self.active {
            if self.config.burst && self.elapsed <= dt * 2.0 {
                // Burst mode: emit all at once
                for _ in 0..self.config.burst_count {
                    self.emit_particle();
                }
            } else if !self.config.burst {
                self.emit_accumulator += dt * self.config.rate;
                while self.emit_accumulator >= 1.0 {
                    self.emit_particle();
                    self.emit_accumulator -= 1.0;
                }
            }
        }

        // Update existing particles
        let gravity = self.config.gravity;
        let drag = self.config.drag;
        let wind = self.config.wind;

        for p in &mut self.particles {
            if !p.alive {
                continue;
            }

            // Decrease life
            p.life -= dt;
            if p.life <= 0.0 {
                p.alive = false;
                continue;
            }

            // Forces
            p.acceleration = gravity + wind;
            p.velocity += p.acceleration * dt;
            p.velocity *= 1.0 - drag;
            p.position += p.velocity * dt;

            // Interpolate size and color by age
            let age = p.age_ratio();
            p.size = p.start_size + (p.end_size - p.start_size) * age;
            for i in 0..4 {
                p.color[i] = p.start_color[i] + (p.end_color[i] - p.start_color[i]) * age;
            }

            // Rotation
            p.rotation += p.angular_velocity * dt;
        }
    }

    /// Get all alive particles.
    pub fn alive_particles(&self) -> impl Iterator<Item = &Particle> {
        self.particles.iter().filter(|p| p.alive)
    }

    /// Number of alive particles.
    pub fn alive_count(&self) -> usize {
        self.particles.iter().filter(|p| p.alive).count()
    }

    /// Reset the emitter.
    pub fn reset(&mut self) {
        self.particles.clear();
        self.emit_accumulator = 0.0;
        self.elapsed = 0.0;
        self.active = true;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emitter_creation() {
        let emitter = ParticleEmitter::new(EmitterConfig::default());
        assert!(emitter.active);
        assert_eq!(emitter.particles.len(), 0);
    }

    #[test]
    fn test_emission() {
        let config = EmitterConfig {
            rate: 100.0,
            max_particles: 50,
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);

        // After 1 second at 100/sec, should have particles
        emitter.update(1.0);
        assert!(emitter.alive_count() > 0);
    }

    #[test]
    fn test_particle_death() {
        let config = EmitterConfig {
            rate: 100.0,
            max_particles: 200,
            lifetime: [0.1, 0.1], // very short life
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);

        emitter.update(0.05); // emit some
        let count = emitter.alive_count();
        assert!(count > 0);

        emitter.update(0.2); // enough time for first batch to die
        // Some may have died while new ones spawned
    }

    #[test]
    fn test_burst_mode() {
        let config = EmitterConfig {
            burst: true,
            burst_count: 10,
            max_particles: 20,
            lifetime: [5.0, 5.0],
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        emitter.update(0.016);

        assert_eq!(emitter.alive_count(), 10);
    }

    #[test]
    fn test_max_particles_cap() {
        let config = EmitterConfig {
            rate: 10000.0,
            max_particles: 10,
            lifetime: [10.0, 10.0],
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        emitter.update(1.0);

        assert!(emitter.particles.len() <= 10);
    }

    #[test]
    fn test_gravity_effect() {
        let config = EmitterConfig {
            burst: true,
            burst_count: 1,
            max_particles: 1,
            lifetime: [10.0, 10.0],
            speed: [0.0, 0.0], // no initial velocity
            gravity: Vec3::new(0.0, -10.0, 0.0),
            drag: 0.0,
            wind: Vec3::ZERO,
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        emitter.update(0.016); // emit
        emitter.update(1.0);   // simulate

        let has_fallen = emitter.particles.iter()
            .filter(|p| p.alive)
            .any(|p| p.position.y < 0.0);
        assert!(has_fallen, "At least one particle should fall below y=0");
    }

    #[test]
    fn test_presets() {
        // All presets should create valid configs
        let presets = [
            EmitterConfig::fire(),
            EmitterConfig::smoke(),
            EmitterConfig::dust(),
            EmitterConfig::sparks(),
            EmitterConfig::snow(),
            EmitterConfig::rain(),
        ];
        for config in &presets {
            assert!(config.max_particles > 0);
            assert!(config.rate > 0.0);
        }
    }

    #[test]
    fn test_reset() {
        let config = EmitterConfig {
            rate: 1000.0,
            max_particles: 100,
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        emitter.update(0.5);
        assert!(emitter.alive_count() > 0);

        emitter.reset();
        assert_eq!(emitter.alive_count(), 0);
        assert!(emitter.active);
    }

    #[test]
    fn test_duration_stops() {
        let config = EmitterConfig {
            rate: 100.0,
            max_particles: 100,
            duration: Some(0.5),
            looping: false,
            lifetime: [10.0, 10.0],
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        emitter.update(0.3); // still active
        assert!(emitter.active);
        emitter.update(0.3); // past duration
        assert!(!emitter.active);
    }

    #[test]
    fn test_color_fade() {
        let config = EmitterConfig {
            burst: true,
            burst_count: 1,
            max_particles: 1,
            lifetime: [1.0, 1.0],
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [0.0, 0.0, 0.0, 0.0],
            speed: [0.0, 0.0],
            gravity: Vec3::ZERO,
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        emitter.update(0.016); // emit

        // After half the lifetime, alpha should be ~0.5
        emitter.update(0.5);
        let alpha = emitter.particles.iter()
            .filter(|p| p.alive)
            .map(|p| p.color[3])
            .next()
            .unwrap_or(1.0);
        assert!(alpha < 0.7, "Alpha should have faded: {}", alpha);
    }
}
