//! Cloth / Soft-body simulation — spring-mass particle system.
//!
//! Implements a simple position-based dynamics (PBD) cloth sim that can
//! attach to skeleton joints for capes, hair, tails, etc.

use glam::Vec3;

/// Configuration for cloth simulation.
#[derive(Clone)]
pub struct ClothConfig {
    /// Gravity acceleration.
    pub gravity: Vec3,
    /// Damping factor (0=none, 1=fully damped).
    pub damping: f32,
    /// Number of constraint iterations per step.
    pub iterations: usize,
    /// Stiffness of distance constraints [0..1].
    pub stiffness: f32,
    /// Ground plane Y coordinate.
    pub ground_y: f32,
    /// Wind force.
    pub wind: Vec3,
}

impl Default for ClothConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.01,
            iterations: 5,
            stiffness: 0.8,
            ground_y: 0.0,
            wind: Vec3::ZERO,
        }
    }
}

/// A single particle in the cloth.
#[derive(Clone)]
pub struct Particle {
    pub position: Vec3,
    pub prev_position: Vec3,
    pub acceleration: Vec3,
    pub inv_mass: f32,
    pub pinned: bool,
    /// If Some, this particle is attached to a skeleton joint index.
    pub attached_joint: Option<usize>,
}

/// Distance constraint between two particles.
#[derive(Clone)]
pub struct DistanceConstraint {
    pub a: usize,
    pub b: usize,
    pub rest_length: f32,
}

/// A cloth simulation instance.
pub struct ClothSim {
    pub particles: Vec<Particle>,
    pub constraints: Vec<DistanceConstraint>,
    pub config: ClothConfig,
    pub active: bool,
    /// Grid dimensions (for structured cloth).
    pub width: usize,
    pub height: usize,
}

impl ClothSim {
    /// Create a rectangular cloth grid.
    ///
    /// - `origin`: top-left corner position
    /// - `right`: direction and length of width
    /// - `down`: direction and length of height
    /// - `w`, `h`: number of particles along width/height
    pub fn new_grid(origin: Vec3, right: Vec3, down: Vec3, w: usize, h: usize) -> Self {
        let mut particles = Vec::with_capacity(w * h);
        let mut constraints = Vec::new();

        // Create particles
        for row in 0..h {
            for col in 0..w {
                let u = col as f32 / (w - 1).max(1) as f32;
                let v = row as f32 / (h - 1).max(1) as f32;
                let pos = origin + right * u + down * v;
                particles.push(Particle {
                    position: pos,
                    prev_position: pos,
                    acceleration: Vec3::ZERO,
                    inv_mass: 1.0,
                    pinned: false,
                    attached_joint: None,
                });
            }
        }

        // Structural constraints (horizontal + vertical)
        for row in 0..h {
            for col in 0..w {
                let idx = row * w + col;
                // Horizontal
                if col + 1 < w {
                    let neighbor = idx + 1;
                    let rest = (particles[idx].position - particles[neighbor].position).length();
                    constraints.push(DistanceConstraint { a: idx, b: neighbor, rest_length: rest });
                }
                // Vertical
                if row + 1 < h {
                    let neighbor = idx + w;
                    let rest = (particles[idx].position - particles[neighbor].position).length();
                    constraints.push(DistanceConstraint { a: idx, b: neighbor, rest_length: rest });
                }
                // Diagonal (shear)
                if col + 1 < w && row + 1 < h {
                    let neighbor = idx + w + 1;
                    let rest = (particles[idx].position - particles[neighbor].position).length();
                    constraints.push(DistanceConstraint { a: idx, b: neighbor, rest_length: rest });
                }
                if col > 0 && row + 1 < h {
                    let neighbor = idx + w - 1;
                    let rest = (particles[idx].position - particles[neighbor].position).length();
                    constraints.push(DistanceConstraint { a: idx, b: neighbor, rest_length: rest });
                }
            }
        }

        Self {
            particles,
            constraints,
            config: ClothConfig::default(),
            active: true,
            width: w,
            height: h,
        }
    }

    /// Create a chain of particles (for hair, tails, ropes, etc.).
    pub fn new_chain(points: &[Vec3]) -> Self {
        let mut particles = Vec::with_capacity(points.len());
        let mut constraints = Vec::new();

        for (i, &pos) in points.iter().enumerate() {
            particles.push(Particle {
                position: pos,
                prev_position: pos,
                acceleration: Vec3::ZERO,
                inv_mass: 1.0,
                pinned: i == 0, // pin the first particle
                attached_joint: None,
            });

            if i > 0 {
                let rest = (pos - points[i - 1]).length();
                constraints.push(DistanceConstraint {
                    a: i - 1,
                    b: i,
                    rest_length: rest,
                });
            }
        }

        Self {
            particles,
            constraints,
            config: ClothConfig::default(),
            active: true,
            width: points.len(),
            height: 1,
        }
    }

    /// Pin the top row of a grid cloth.
    pub fn pin_top_row(&mut self) {
        for col in 0..self.width {
            self.particles[col].pinned = true;
        }
    }

    /// Attach a particle to a skeleton joint.
    pub fn attach_to_joint(&mut self, particle_idx: usize, joint_idx: usize) {
        if particle_idx < self.particles.len() {
            self.particles[particle_idx].attached_joint = Some(joint_idx);
            self.particles[particle_idx].pinned = true;
        }
    }

    /// Update attached particles from skeleton transforms.
    pub fn update_attachments(&mut self, joint_positions: &[Vec3]) {
        for p in &mut self.particles {
            if let Some(ji) = p.attached_joint {
                if ji < joint_positions.len() {
                    p.position = joint_positions[ji];
                    p.prev_position = p.position;
                }
            }
        }
    }

    /// Step the simulation forward by dt seconds.
    pub fn step(&mut self, dt: f32) {
        if !self.active || dt <= 0.0 {
            return;
        }

        let dt = dt.min(1.0 / 30.0); // Cap timestep
        let damping = 1.0 - self.config.damping;

        // Verlet integration
        for p in &mut self.particles {
            if p.pinned {
                continue;
            }
            let accel = self.config.gravity + self.config.wind;
            let velocity = (p.position - p.prev_position) * damping;
            p.prev_position = p.position;
            p.position += velocity + accel * dt * dt;
        }

        // Satisfy constraints (PBD)
        for _ in 0..self.config.iterations {
            for ci in 0..self.constraints.len() {
                let c = &self.constraints[ci];
                let a_idx = c.a;
                let b_idx = c.b;
                let rest = c.rest_length;

                let pa = self.particles[a_idx].position;
                let pb = self.particles[b_idx].position;
                let delta = pb - pa;
                let dist = delta.length();
                if dist < 1e-6 {
                    continue;
                }

                let diff = (dist - rest) / dist;
                let correction = delta * diff * self.config.stiffness * 0.5;

                let a_pinned = self.particles[a_idx].pinned;
                let b_pinned = self.particles[b_idx].pinned;

                if !a_pinned && !b_pinned {
                    self.particles[a_idx].position += correction;
                    self.particles[b_idx].position -= correction;
                } else if !a_pinned {
                    self.particles[a_idx].position += correction * 2.0;
                } else if !b_pinned {
                    self.particles[b_idx].position -= correction * 2.0;
                }
            }
        }

        // Ground collision
        let ground = self.config.ground_y;
        for p in &mut self.particles {
            if !p.pinned && p.position.y < ground {
                p.position.y = ground;
            }
        }
    }

    /// Get particle positions as a flat Vec for rendering.
    pub fn get_positions(&self) -> Vec<Vec3> {
        self.particles.iter().map(|p| p.position).collect()
    }

    /// Get the number of particles.
    pub fn num_particles(&self) -> usize {
        self.particles.len()
    }

    /// Reset all particles to their initial positions.
    pub fn reset(&mut self) {
        // Reset non-pinned particles to rest
        for p in &mut self.particles {
            p.prev_position = p.position;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloth_grid_creation() {
        let cloth = ClothSim::new_grid(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            5, 5,
        );
        assert_eq!(cloth.num_particles(), 25);
        // 4*5 horizontal + 5*4 vertical + 4*4 diagonal + 4*4 diagonal = 20+20+16+16 = 72
        assert!(cloth.constraints.len() > 40);
    }

    #[test]
    fn test_cloth_gravity() {
        let mut cloth = ClothSim::new_grid(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3, 3,
        );
        cloth.pin_top_row();
        let initial_y = cloth.particles[6].position.y; // bottom-center
        cloth.step(1.0 / 60.0);
        cloth.step(1.0 / 60.0);
        assert!(cloth.particles[6].position.y < initial_y,
            "particle should fall under gravity");
    }

    #[test]
    fn test_chain_creation() {
        let points: Vec<Vec3> = (0..10)
            .map(|i| Vec3::new(0.0, 2.0 - i as f32 * 0.1, 0.0))
            .collect();
        let chain = ClothSim::new_chain(&points);
        assert_eq!(chain.num_particles(), 10);
        assert_eq!(chain.constraints.len(), 9);
        assert!(chain.particles[0].pinned);
        assert!(!chain.particles[9].pinned);
    }

    #[test]
    fn test_ground_collision() {
        let mut cloth = ClothSim::new_chain(&[
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(0.0, 0.3, 0.0),
            Vec3::new(0.0, -0.5, 0.0),
        ]);
        cloth.particles[0].pinned = true;
        cloth.config.ground_y = 0.0;
        for _ in 0..100 {
            cloth.step(1.0 / 60.0);
        }
        for p in &cloth.particles {
            assert!(p.position.y >= 0.0, "particle below ground");
        }
    }

    #[test]
    fn test_wind_effect() {
        let mut cloth = ClothSim::new_chain(&[
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 1.5, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]);
        cloth.particles[0].pinned = true;
        cloth.config.wind = Vec3::new(5.0, 0.0, 0.0);
        for _ in 0..60 {
            cloth.step(1.0 / 60.0);
        }
        // Wind should push particles in +X
        assert!(cloth.particles[2].position.x > 0.1,
            "wind should push particle in +X direction, got {}", cloth.particles[2].position.x);
    }
}
