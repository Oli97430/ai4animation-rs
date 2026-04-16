//! Ragdoll physics — simple rigid body simulation for skeletal characters.
//!
//! Each bone becomes a rigid body with position, velocity, and mass.
//! Parent-child relationships are maintained via distance constraints.
//! Collision with a ground plane is supported.

use glam::{Vec3, Mat4, Quat};
use anim_math::transform::Transform;

/// Configuration for the ragdoll simulation.
#[derive(Clone)]
pub struct RagdollConfig {
    /// Gravity vector (m/s²). Default: (0, -9.81, 0).
    pub gravity: Vec3,
    /// Damping factor per frame (0 = no damping, 1 = full stop). Default: 0.02.
    pub damping: f32,
    /// Ground plane Y coordinate. Default: 0.0.
    pub ground_y: f32,
    /// Coefficient of restitution (bounciness). Default: 0.3.
    pub restitution: f32,
    /// Friction coefficient. Default: 0.5.
    pub friction: f32,
    /// Number of constraint solver iterations per step. Default: 8.
    pub solver_iterations: usize,
    /// Sub-steps per frame for stability. Default: 4.
    pub substeps: usize,
}

impl Default for RagdollConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.02,
            ground_y: 0.0,
            restitution: 0.3,
            friction: 0.5,
            solver_iterations: 8,
            substeps: 4,
        }
    }
}

/// A single rigid body in the ragdoll.
#[derive(Clone)]
pub struct RigidBody {
    pub position: Vec3,
    pub prev_position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
    pub inv_mass: f32,
    /// Whether this body is pinned (kinematic, not affected by physics).
    pub pinned: bool,
    /// Bone index in the skeleton.
    pub bone_index: usize,
    /// Rotation (maintained for visual output).
    pub rotation: Quat,
}

/// A distance constraint between two bodies (bone connection).
#[derive(Clone)]
pub struct DistanceConstraint {
    pub body_a: usize,
    pub body_b: usize,
    pub rest_length: f32,
    /// Stiffness: 0.0 = slack, 1.0 = rigid.
    pub stiffness: f32,
}

/// An angular constraint limiting the angle between connected bones.
#[derive(Clone)]
pub struct AngleConstraint {
    /// Parent body.
    pub body_parent: usize,
    /// Child body.
    pub body_child: usize,
    /// Maximum angle in radians from the rest direction.
    pub max_angle: f32,
}

/// Complete ragdoll simulation state.
pub struct Ragdoll {
    pub bodies: Vec<RigidBody>,
    pub distance_constraints: Vec<DistanceConstraint>,
    pub angle_constraints: Vec<AngleConstraint>,
    pub config: RagdollConfig,
    pub active: bool,
    /// Time accumulator for fixed sub-stepping.
    accumulator: f32,
}

impl Ragdoll {
    /// Create a ragdoll from a skeleton pose.
    ///
    /// `transforms` — current global transforms for each joint.
    /// `parent_indices` — parent index per joint (-1 for roots).
    pub fn from_pose(
        transforms: &[Mat4],
        parent_indices: &[i32],
        config: RagdollConfig,
    ) -> Self {
        let num = transforms.len();
        let mut bodies = Vec::with_capacity(num);
        let mut distance_constraints = Vec::new();
        let mut angle_constraints = Vec::new();

        // Create rigid bodies from joint positions
        for (i, t) in transforms.iter().enumerate() {
            let pos = t.get_position();
            let (_s, rot, _t) = t.to_scale_rotation_translation();

            // Mass based on bone depth (deeper = lighter, root = heavier)
            let depth = compute_depth(i, parent_indices);
            let mass = (1.0 / (depth as f32 + 1.0)).max(0.1);

            bodies.push(RigidBody {
                position: pos,
                prev_position: pos,
                velocity: Vec3::ZERO,
                mass,
                inv_mass: 1.0 / mass,
                pinned: false,
                bone_index: i,
                rotation: rot,
            });
        }

        // Create distance constraints from parent-child relationships
        for (i, &pi) in parent_indices.iter().enumerate() {
            if pi >= 0 {
                let parent = pi as usize;
                if parent < num {
                    let rest_length = bodies[i].position.distance(bodies[parent].position);
                    distance_constraints.push(DistanceConstraint {
                        body_a: parent,
                        body_b: i,
                        rest_length: rest_length.max(0.001),
                        stiffness: 0.95,
                    });

                    // Angular constraint: limit child angle from rest direction
                    angle_constraints.push(AngleConstraint {
                        body_parent: parent,
                        body_child: i,
                        max_angle: std::f32::consts::FRAC_PI_3, // 60 degrees
                    });
                }
            }
        }

        Self {
            bodies,
            distance_constraints,
            angle_constraints,
            config,
            active: false,
            accumulator: 0.0,
        }
    }

    /// Step the simulation forward by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        if !self.active || dt <= 0.0 {
            return;
        }

        let substeps = self.config.substeps.max(1);
        self.accumulator += dt;
        let sub_dt = dt / substeps as f32;

        for _ in 0..substeps {
            if self.accumulator < sub_dt { break; }
            self.accumulator -= sub_dt;
            self.substep(sub_dt);
        }
    }

    fn substep(&mut self, dt: f32) {
        let gravity = self.config.gravity;
        let damping = self.config.damping;

        // Verlet integration
        for body in &mut self.bodies {
            if body.pinned {
                continue;
            }

            let acceleration = gravity;
            let new_pos = body.position + body.velocity * dt + acceleration * dt * dt;

            body.prev_position = body.position;
            body.position = new_pos;
            body.velocity = (body.position - body.prev_position) / dt;
            body.velocity *= 1.0 - damping;
        }

        // Solve constraints
        for _ in 0..self.config.solver_iterations {
            self.solve_distance_constraints();
            self.solve_ground_constraints();
        }

        // Update velocities from position changes
        if dt > 0.0 {
            for body in &mut self.bodies {
                if !body.pinned {
                    body.velocity = (body.position - body.prev_position) / dt;
                }
            }
        }

        // Update rotations from bone directions
        self.update_rotations();
    }

    fn solve_distance_constraints(&mut self) {
        for ci in 0..self.distance_constraints.len() {
            let c = &self.distance_constraints[ci];
            let a = c.body_a;
            let b = c.body_b;
            let rest = c.rest_length;
            let stiffness = c.stiffness;

            let pos_a = self.bodies[a].position;
            let pos_b = self.bodies[b].position;
            let delta = pos_b - pos_a;
            let dist = delta.length();

            if dist < 1e-6 { continue; }

            let error = dist - rest;
            let correction = delta.normalize() * error * stiffness;

            let inv_a = if self.bodies[a].pinned { 0.0 } else { self.bodies[a].inv_mass };
            let inv_b = if self.bodies[b].pinned { 0.0 } else { self.bodies[b].inv_mass };
            let total_inv = inv_a + inv_b;
            if total_inv < 1e-6 { continue; }

            if !self.bodies[a].pinned {
                self.bodies[a].position += correction * (inv_a / total_inv);
            }
            if !self.bodies[b].pinned {
                self.bodies[b].position -= correction * (inv_b / total_inv);
            }
        }
    }

    fn solve_ground_constraints(&mut self) {
        let ground = self.config.ground_y;
        let restitution = self.config.restitution;
        let friction = self.config.friction;

        for body in &mut self.bodies {
            if body.pinned { continue; }

            if body.position.y < ground {
                body.position.y = ground;

                // Bounce
                if body.velocity.y < 0.0 {
                    body.velocity.y = -body.velocity.y * restitution;
                }
                // Friction
                body.velocity.x *= 1.0 - friction;
                body.velocity.z *= 1.0 - friction;
            }
        }
    }

    fn update_rotations(&mut self) {
        // Compute rotation for each body based on its parent direction
        for ci in 0..self.distance_constraints.len() {
            let c = &self.distance_constraints[ci];
            let parent_pos = self.bodies[c.body_a].position;
            let child_pos = self.bodies[c.body_b].position;

            let direction = (child_pos - parent_pos).normalize_or_zero();
            if direction.length_squared() > 0.5 {
                // Rotation that aligns Y-up to the bone direction
                let up = Vec3::Y;
                if direction.dot(up).abs() < 0.999 {
                    let rot = Quat::from_rotation_arc(up, direction);
                    self.bodies[c.body_b].rotation = rot;
                }
            }
        }
    }

    /// Apply an impulse to a specific body.
    pub fn apply_impulse(&mut self, body_index: usize, impulse: Vec3) {
        if body_index < self.bodies.len() && !self.bodies[body_index].pinned {
            let inv_mass = self.bodies[body_index].inv_mass;
            self.bodies[body_index].velocity += impulse * inv_mass;
        }
    }

    /// Apply an explosion force from a point.
    pub fn apply_explosion(&mut self, center: Vec3, force: f32, radius: f32) {
        for body in &mut self.bodies {
            if body.pinned { continue; }
            let delta = body.position - center;
            let dist = delta.length();
            if dist < radius && dist > 0.01 {
                let falloff = 1.0 - (dist / radius);
                let impulse = delta.normalize() * force * falloff * body.inv_mass;
                body.velocity += impulse;
            }
        }
    }

    /// Pin/unpin a body (pinned bodies are kinematic).
    pub fn set_pinned(&mut self, body_index: usize, pinned: bool) {
        if body_index < self.bodies.len() {
            self.bodies[body_index].pinned = pinned;
            if pinned {
                self.bodies[body_index].velocity = Vec3::ZERO;
            }
        }
    }

    /// Get the current poses as Mat4 transforms (for applying to the scene).
    pub fn get_transforms(&self) -> Vec<Mat4> {
        self.bodies.iter().map(|body| {
            Mat4::from_rotation_translation(body.rotation, body.position)
        }).collect()
    }

    /// Reset all bodies to a given pose.
    pub fn reset_to_pose(&mut self, transforms: &[Mat4]) {
        for (i, body) in self.bodies.iter_mut().enumerate() {
            if i < transforms.len() {
                let pos = transforms[i].get_position();
                let (_s, rot, _t) = transforms[i].to_scale_rotation_translation();
                body.position = pos;
                body.prev_position = pos;
                body.velocity = Vec3::ZERO;
                body.rotation = rot;
            }
        }
        self.accumulator = 0.0;
    }

    /// Number of bodies.
    pub fn num_bodies(&self) -> usize {
        self.bodies.len()
    }
}

/// Compute hierarchy depth for a joint.
fn compute_depth(joint: usize, parents: &[i32]) -> usize {
    let mut depth = 0;
    let mut current = joint;
    loop {
        if current >= parents.len() { break; }
        let pi = parents[current];
        if pi < 0 { break; }
        depth += 1;
        current = pi as usize;
        if depth > 50 { break; } // prevent infinite loop
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_skeleton() -> (Vec<Mat4>, Vec<i32>) {
        let transforms = vec![
            Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)),  // root (hips)
            Mat4::from_translation(Vec3::new(0.0, 1.5, 0.0)),  // spine
            Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0)),  // head
            Mat4::from_translation(Vec3::new(0.3, 1.4, 0.0)),  // left arm
            Mat4::from_translation(Vec3::new(-0.3, 1.4, 0.0)), // right arm
        ];
        let parents = vec![-1, 0, 1, 1, 1];
        (transforms, parents)
    }

    #[test]
    fn test_ragdoll_creation() {
        let (transforms, parents) = make_simple_skeleton();
        let ragdoll = Ragdoll::from_pose(&transforms, &parents, RagdollConfig::default());
        assert_eq!(ragdoll.bodies.len(), 5);
        assert_eq!(ragdoll.distance_constraints.len(), 4); // 4 parent-child pairs
    }

    #[test]
    fn test_ragdoll_gravity() {
        let (transforms, parents) = make_simple_skeleton();
        let mut ragdoll = Ragdoll::from_pose(&transforms, &parents, RagdollConfig::default());
        ragdoll.active = true;

        let initial_y = ragdoll.bodies[2].position.y; // head
        ragdoll.step(0.1);

        // Head should have fallen due to gravity
        assert!(ragdoll.bodies[2].position.y < initial_y);
    }

    #[test]
    fn test_ragdoll_pinned() {
        let (transforms, parents) = make_simple_skeleton();
        let mut ragdoll = Ragdoll::from_pose(&transforms, &parents, RagdollConfig::default());
        ragdoll.active = true;
        ragdoll.set_pinned(0, true); // pin root

        let root_pos = ragdoll.bodies[0].position;
        ragdoll.step(0.1);

        // Root should not have moved
        assert!((ragdoll.bodies[0].position - root_pos).length() < 0.001);
    }

    #[test]
    fn test_ragdoll_ground_collision() {
        let transforms = vec![
            Mat4::from_translation(Vec3::new(0.0, 0.05, 0.0)), // very close to ground
        ];
        let parents = vec![-1];
        let mut ragdoll = Ragdoll::from_pose(&transforms, &parents, RagdollConfig::default());
        ragdoll.active = true;

        // Step enough to hit ground
        for _ in 0..10 {
            ragdoll.step(0.05);
        }

        // Should be at or above ground level
        assert!(ragdoll.bodies[0].position.y >= 0.0);
    }

    #[test]
    fn test_ragdoll_explosion() {
        let (transforms, parents) = make_simple_skeleton();
        let mut ragdoll = Ragdoll::from_pose(&transforms, &parents, RagdollConfig::default());
        ragdoll.active = true;

        let initial_pos = ragdoll.bodies[2].position;
        ragdoll.apply_explosion(Vec3::ZERO, 50.0, 5.0);
        ragdoll.step(0.05);

        // Head should have moved away from explosion
        assert!(ragdoll.bodies[2].position.distance(initial_pos) > 0.01);
    }

    #[test]
    fn test_compute_depth() {
        let parents = vec![-1, 0, 1, 2, 1];
        assert_eq!(compute_depth(0, &parents), 0); // root
        assert_eq!(compute_depth(1, &parents), 1); // child of root
        assert_eq!(compute_depth(2, &parents), 2); // grandchild
        assert_eq!(compute_depth(3, &parents), 3); // great-grandchild
        assert_eq!(compute_depth(4, &parents), 2); // child of 1
    }

    #[test]
    fn test_get_transforms() {
        let (transforms, parents) = make_simple_skeleton();
        let ragdoll = Ragdoll::from_pose(&transforms, &parents, RagdollConfig::default());
        let output = ragdoll.get_transforms();
        assert_eq!(output.len(), 5);
        // Check that positions match the input
        for (i, m) in output.iter().enumerate() {
            let pos = m.get_position();
            let orig = transforms[i].get_position();
            assert!((pos - orig).length() < 0.01);
        }
    }
}
