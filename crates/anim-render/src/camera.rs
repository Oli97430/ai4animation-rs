//! Camera system with orbit, free, and third-person modes.

use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    Orbit,
    Free,
    ThirdPerson,
}

/// 3D camera with multiple control modes.
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub mode: CameraMode,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub move_speed: f32,
    pub rotate_speed: f32,
    pub zoom_speed: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(3.0, 2.0, 3.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0,
            near: 0.01,
            far: 100.0,
            mode: CameraMode::Orbit,
            distance: 5.0,
            yaw: -45.0f32.to_radians(),
            pitch: 30.0f32.to_radians(),
            move_speed: 5.0,
            rotate_speed: 0.005,
            zoom_speed: 0.5,
        }
    }

    /// View matrix.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Projection matrix.
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, self.near, self.far)
    }

    /// View-projection matrix.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    /// Update orbit camera from mouse input.
    pub fn orbit_rotate(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * self.rotate_speed;
        self.pitch = (self.pitch - dy * self.rotate_speed)
            .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
        self.update_orbit_position();
    }

    /// Zoom (change distance).
    pub fn orbit_zoom(&mut self, delta: f32) {
        self.distance = (self.distance - delta * self.zoom_speed).max(0.5);
        self.update_orbit_position();
    }

    /// Pan camera target.
    pub fn orbit_pan(&mut self, dx: f32, dy: f32) {
        let right = (self.target - self.position).cross(self.up).normalize();
        let up = right.cross((self.target - self.position).normalize());
        let pan_speed = self.distance * 0.002;
        self.target += right * (-dx * pan_speed) + up * (dy * pan_speed);
        self.update_orbit_position();
    }

    fn update_orbit_position(&mut self) {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.position = self.target + Vec3::new(x, y, z);
    }

    /// Set target and recalculate position for orbit mode.
    pub fn look_at(&mut self, target: Vec3) {
        self.target = target;
        self.update_orbit_position();
    }

    /// Reset to default view.
    pub fn reset(&mut self) {
        self.target = Vec3::ZERO;
        self.distance = 5.0;
        self.yaw = -45.0f32.to_radians();
        self.pitch = 30.0f32.to_radians();
        self.update_orbit_position();
    }

    // ── Orthographic presets ────────────────────────────────

    /// Set camera to Front view (looking along -Z).
    pub fn view_front(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.update_orbit_position();
    }

    /// Set camera to Back view (looking along +Z).
    pub fn view_back(&mut self) {
        self.yaw = std::f32::consts::PI;
        self.pitch = 0.0;
        self.update_orbit_position();
    }

    /// Set camera to Right view (looking along -X).
    pub fn view_right(&mut self) {
        self.yaw = -std::f32::consts::FRAC_PI_2;
        self.pitch = 0.0;
        self.update_orbit_position();
    }

    /// Set camera to Left view (looking along +X).
    pub fn view_left(&mut self) {
        self.yaw = std::f32::consts::FRAC_PI_2;
        self.pitch = 0.0;
        self.update_orbit_position();
    }

    /// Set camera to Top view (looking down along -Y).
    pub fn view_top(&mut self) {
        self.yaw = 0.0;
        self.pitch = 89.0f32.to_radians();
        self.update_orbit_position();
    }

    /// Set camera to Bottom view (looking up along +Y).
    pub fn view_bottom(&mut self) {
        self.yaw = 0.0;
        self.pitch = -89.0f32.to_radians();
        self.update_orbit_position();
    }

    /// Compute a world-space ray from screen coordinates (0..width, 0..height).
    /// Returns (ray_origin, ray_direction).
    pub fn screen_ray(&self, screen_x: f32, screen_y: f32, width: f32, height: f32) -> (Vec3, Vec3) {
        let aspect = width / height;
        let vp = self.view_projection(aspect);
        let inv_vp = vp.inverse();

        // NDC: x in [-1,1], y in [-1,1] (note: y is flipped)
        let ndc_x = (2.0 * screen_x / width) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y / height);

        let near_ndc = glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_ndc = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near_world = inv_vp * near_ndc;
        let far_world = inv_vp * far_ndc;

        let near_pos = near_world.truncate() / near_world.w;
        let far_pos = far_world.truncate() / far_world.w;

        let dir = (far_pos - near_pos).normalize();
        (near_pos, dir)
    }

    /// Walk-mode update: move camera with WASD-style input.
    pub fn walk_move(&mut self, forward: f32, right: f32, up: f32, speed: f32, dt: f32) {
        let fwd = (self.target - self.position).normalize();
        let r = fwd.cross(self.up).normalize();
        let u = Vec3::Y;

        let movement = fwd * forward + r * right + u * up;
        if movement.length_squared() > 0.0001 {
            let delta = movement.normalize() * speed * dt;
            self.position += delta;
            self.target += delta;
        }
    }

    /// Walk-mode look: rotate view direction with mouse delta.
    pub fn walk_look(&mut self, dx: f32, dy: f32) {
        let sensitivity = 0.003;
        self.yaw -= dx * sensitivity;
        self.pitch = (self.pitch - dy * sensitivity)
            .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());

        // Reconstruct target from yaw/pitch
        let dir = Vec3::new(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        );
        self.target = self.position + dir;
    }
}

impl Default for Camera {
    fn default() -> Self {
        let mut cam = Self::new();
        cam.update_orbit_position();
        cam
    }
}
