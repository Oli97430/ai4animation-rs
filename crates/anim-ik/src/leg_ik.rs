//! Two-bone biped leg IK — ankle + ball solver with contact-driven blending.
//!
//! Mirrors Python Demos/Locomotion/Biped/LegIK.py.
//! Uses two FABRIK chains: one for the ankle, one for the ball (toe).

use glam::Vec3;
use crate::FabrikSolver;

/// Biped leg IK solver: ankle chain + ball chain, contact-driven.
pub struct LegIk {
    pub ankle_solver: FabrikSolver,
    pub ball_solver: FabrikSolver,

    /// Resting height of the ankle joint.
    pub ankle_baseline: f32,
    /// Resting height of the ball joint.
    pub ball_baseline: f32,
    /// Rest distance from ankle to ball.
    pub ankle_ball_distance: f32,

    /// Current ankle target position.
    pub ankle_target_pos: Vec3,
    /// Current ball target position.
    pub ball_target_pos: Vec3,
}

impl LegIk {
    /// Create from two solvers with their current tip positions as baselines.
    pub fn new(ankle_solver: FabrikSolver, ball_solver: FabrikSolver) -> Self {
        let ankle_pos = *ankle_solver.positions.last().unwrap_or(&Vec3::ZERO);
        let ball_pos = *ball_solver.positions.last().unwrap_or(&Vec3::ZERO);
        let ankle_baseline = ankle_pos.y;
        let ball_baseline = ball_pos.y;
        let ankle_ball_distance = ankle_pos.distance(ball_pos);

        Self {
            ankle_solver,
            ball_solver,
            ankle_baseline,
            ball_baseline,
            ankle_ball_distance,
            ankle_target_pos: ankle_pos,
            ball_target_pos: ball_pos,
        }
    }

    /// Solve both ankle and ball IK, driven by foot contact signals [0..1].
    pub fn solve(&mut self, ankle_contact: f32, ball_contact: f32) {
        self.solve_ankle(ankle_contact);
        self.solve_ball(ball_contact);
    }

    /// Solve the ankle chain: lock ankle to ground when contact > 0.
    fn solve_ankle(&mut self, contact: f32) {
        let current_pos = *self.ankle_solver.positions.last().unwrap_or(&Vec3::ZERO);

        // Blend toward locked position based on contact strength
        let mut locked_pos = self.ankle_target_pos;
        locked_pos.y = lerp(locked_pos.y, self.ankle_baseline, contact).max(self.ankle_baseline);

        self.ankle_target_pos = Vec3::lerp(current_pos, locked_pos, contact);

        self.ankle_solver.solve(self.ankle_target_pos);
    }

    /// Solve the ball (toe) chain: lock ball to ground, constrained to ankle-ball distance.
    fn solve_ball(&mut self, contact: f32) {
        let current_pos = *self.ball_solver.positions.last().unwrap_or(&Vec3::ZERO);

        // Blend toward locked position
        let mut locked_pos = self.ball_target_pos;
        locked_pos.y = lerp(locked_pos.y, self.ball_baseline, contact).max(self.ball_baseline);

        self.ball_target_pos = Vec3::lerp(current_pos, locked_pos, contact);

        // Re-project ball target onto a sphere around the ankle
        let ankle_pos = self.ankle_target_pos;
        let dir = (self.ball_target_pos - ankle_pos).normalize_or_zero();
        self.ball_target_pos = ankle_pos + dir * self.ankle_ball_distance;

        self.ball_solver.solve(self.ball_target_pos);
    }

    /// Get the solved ankle position.
    pub fn ankle_position(&self) -> Vec3 {
        *self.ankle_solver.positions.last().unwrap_or(&Vec3::ZERO)
    }

    /// Get the solved ball position.
    pub fn ball_position(&self) -> Vec3 {
        *self.ball_solver.positions.last().unwrap_or(&Vec3::ZERO)
    }

    /// Update internal chain positions from external bone positions.
    pub fn update_positions(&mut self, ankle_chain: &[Vec3], ball_chain: &[Vec3]) {
        if ankle_chain.len() == self.ankle_solver.positions.len() {
            self.ankle_solver.positions.copy_from_slice(ankle_chain);
        }
        if ball_chain.len() == self.ball_solver.positions.len() {
            self.ball_solver.positions.copy_from_slice(ball_chain);
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
