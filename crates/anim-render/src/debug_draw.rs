//! Immediate-mode debug drawing (lines, spheres, transforms, skeletons).

use glam::{Mat4, Vec3};
use crate::vertex::BasicVertex;

/// Color constants.
pub mod colors {
    pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    pub const RED: [f32; 4] = [1.0, 0.2, 0.2, 1.0];
    pub const GREEN: [f32; 4] = [0.2, 1.0, 0.2, 1.0];
    pub const BLUE: [f32; 4] = [0.3, 0.3, 1.0, 1.0];
    pub const YELLOW: [f32; 4] = [1.0, 1.0, 0.2, 1.0];
    pub const CYAN: [f32; 4] = [0.2, 1.0, 1.0, 1.0];
    pub const MAGENTA: [f32; 4] = [1.0, 0.2, 1.0, 1.0];
    pub const GRAY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
    pub const BONE: [f32; 4] = [0.9, 0.85, 0.7, 1.0];
    pub const JOINT: [f32; 4] = [0.2, 0.7, 1.0, 1.0];
    pub const SELECTED: [f32; 4] = [1.0, 0.8, 0.0, 1.0];
}

/// Collects line and point primitives for debug rendering.
pub struct DebugDraw {
    pub lines: Vec<BasicVertex>,
    pub points: Vec<(Vec3, f32, [f32; 4])>,
}

impl DebugDraw {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            points: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.points.clear();
    }

    /// Draw a line segment.
    pub fn line(&mut self, start: Vec3, end: Vec3, color: [f32; 4]) {
        self.lines.push(BasicVertex::new(start.into(), color));
        self.lines.push(BasicVertex::new(end.into(), color));
    }

    /// Draw a point (rendered as a small cube in the shader).
    pub fn point(&mut self, position: Vec3, size: f32, color: [f32; 4]) {
        self.points.push((position, size, color));
    }

    /// Draw a 3-axis transform gizmo.
    pub fn transform_gizmo(&mut self, matrix: Mat4, size: f32) {
        use anim_math::transform::Transform;
        let pos = matrix.get_position();
        let ax = matrix.get_axis_x().normalize() * size;
        let ay = matrix.get_axis_y().normalize() * size;
        let az = matrix.get_axis_z().normalize() * size;
        self.line(pos, pos + ax, colors::RED);
        self.line(pos, pos + ay, colors::GREEN);
        self.line(pos, pos + az, colors::BLUE);
    }

    /// Draw a skeleton from bone positions and parent indices.
    pub fn skeleton(
        &mut self,
        positions: &[Vec3],
        parent_indices: &[i32],
        color: [f32; 4],
        joint_size: f32,
    ) {
        for (i, &pos) in positions.iter().enumerate() {
            self.point(pos, joint_size, colors::JOINT);
            if i >= parent_indices.len() { continue; }
            let parent = parent_indices[i];
            if parent >= 0 && (parent as usize) < positions.len() {
                self.line(positions[parent as usize], pos, color);
            }
        }
    }

    /// Draw a velocity vector.
    pub fn velocity(&mut self, origin: Vec3, velocity: Vec3, scale: f32, color: [f32; 4]) {
        self.line(origin, origin + velocity * scale, color);
    }

    /// Draw a circle (approximated with line segments) in XZ plane.
    pub fn circle_xz(&mut self, center: Vec3, radius: f32, color: [f32; 4], segments: usize) {
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a = i as f32 * step;
            let b = (i + 1) as f32 * step;
            let p1 = center + Vec3::new(a.cos() * radius, 0.0, a.sin() * radius);
            let p2 = center + Vec3::new(b.cos() * radius, 0.0, b.sin() * radius);
            self.line(p1, p2, color);
        }
    }

    /// Draw a box wireframe.
    pub fn wire_box(&mut self, center: Vec3, half_size: Vec3, color: [f32; 4]) {
        let corners = [
            center + Vec3::new(-half_size.x, -half_size.y, -half_size.z),
            center + Vec3::new( half_size.x, -half_size.y, -half_size.z),
            center + Vec3::new( half_size.x, -half_size.y,  half_size.z),
            center + Vec3::new(-half_size.x, -half_size.y,  half_size.z),
            center + Vec3::new(-half_size.x,  half_size.y, -half_size.z),
            center + Vec3::new( half_size.x,  half_size.y, -half_size.z),
            center + Vec3::new( half_size.x,  half_size.y,  half_size.z),
            center + Vec3::new(-half_size.x,  half_size.y,  half_size.z),
        ];
        let edges = [
            (0,1),(1,2),(2,3),(3,0),(4,5),(5,6),(6,7),(7,4),(0,4),(1,5),(2,6),(3,7)
        ];
        for (a, b) in edges {
            self.line(corners[a], corners[b], color);
        }
    }

    /// Generate line vertices for the grid floor.
    pub fn grid_lines(&self, size: f32, divisions: usize) -> Vec<BasicVertex> {
        let mut verts = Vec::new();
        let step = size * 2.0 / divisions as f32;
        let half = size;
        let color = [0.3, 0.3, 0.3, 0.5];
        let accent = [0.4, 0.4, 0.4, 0.7];

        for i in 0..=divisions {
            let t = -half + i as f32 * step;
            let c = if i == divisions / 2 { accent } else { color };
            // X-parallel line
            verts.push(BasicVertex::new([-half, 0.0, t], c));
            verts.push(BasicVertex::new([half, 0.0, t], c));
            // Z-parallel line
            verts.push(BasicVertex::new([t, 0.0, -half], c));
            verts.push(BasicVertex::new([t, 0.0, half], c));
        }
        // Red X axis
        verts.push(BasicVertex::new([0.0, 0.001, 0.0], colors::RED));
        verts.push(BasicVertex::new([half, 0.001, 0.0], colors::RED));
        // Blue Z axis
        verts.push(BasicVertex::new([0.0, 0.001, 0.0], colors::BLUE));
        verts.push(BasicVertex::new([0.0, 0.001, half], colors::BLUE));

        verts
    }

    /// Draw a wireframe sphere (3 orthogonal circles).
    pub fn wire_sphere(&mut self, center: Vec3, radius: f32, color: [f32; 4], segments: usize) {
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a = i as f32 * step;
            let b = (i + 1) as f32 * step;
            // XZ ring
            self.line(
                center + Vec3::new(a.cos() * radius, 0.0, a.sin() * radius),
                center + Vec3::new(b.cos() * radius, 0.0, b.sin() * radius),
                color,
            );
            // XY ring
            self.line(
                center + Vec3::new(a.cos() * radius, a.sin() * radius, 0.0),
                center + Vec3::new(b.cos() * radius, b.sin() * radius, 0.0),
                color,
            );
            // YZ ring
            self.line(
                center + Vec3::new(0.0, a.cos() * radius, a.sin() * radius),
                center + Vec3::new(0.0, b.cos() * radius, b.sin() * radius),
                color,
            );
        }
    }

    /// Draw a wireframe cylinder between two points.
    pub fn wire_cylinder(
        &mut self,
        start: Vec3,
        end: Vec3,
        radius_start: f32,
        radius_end: f32,
        color: [f32; 4],
        segments: usize,
    ) {
        let axis = (end - start).normalize_or_zero();
        if axis.length_squared() < 0.0001 { return; }

        // Build a perpendicular frame
        let up = if axis.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let right = axis.cross(up).normalize();
        let forward = right.cross(axis).normalize();

        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a = i as f32 * step;
            let b = (i + 1) as f32 * step;
            let dir_a = right * a.cos() + forward * a.sin();
            let dir_b = right * b.cos() + forward * b.sin();
            // Start ring
            self.line(start + dir_a * radius_start, start + dir_b * radius_start, color);
            // End ring
            self.line(end + dir_a * radius_end, end + dir_b * radius_end, color);
            // Side lines (every other segment)
            if i % 2 == 0 {
                self.line(start + dir_a * radius_start, end + dir_a * radius_end, color);
            }
        }
    }

    /// Draw an arrow from origin along direction.
    pub fn arrow(&mut self, origin: Vec3, direction: Vec3, length: f32, color: [f32; 4]) {
        let tip = origin + direction.normalize_or_zero() * length;
        self.line(origin, tip, color);
        // Arrowhead
        let head_len = length * 0.15;
        let dir = direction.normalize_or_zero();
        let perp = if dir.y.abs() < 0.99 {
            dir.cross(Vec3::Y).normalize()
        } else {
            dir.cross(Vec3::X).normalize()
        };
        let head_size = length * 0.06;
        self.line(tip, tip - dir * head_len + perp * head_size, color);
        self.line(tip, tip - dir * head_len - perp * head_size, color);
    }

    /// Draw a line strip (polyline).
    pub fn line_strip(&mut self, positions: &[Vec3], color: [f32; 4]) {
        for i in 1..positions.len() {
            self.line(positions[i - 1], positions[i], color);
        }
    }

    /// Draw a ground plane rectangle in XZ.
    pub fn plane(&mut self, center: Vec3, half_size: f32, color: [f32; 4]) {
        let corners = [
            center + Vec3::new(-half_size, 0.0, -half_size),
            center + Vec3::new( half_size, 0.0, -half_size),
            center + Vec3::new( half_size, 0.0,  half_size),
            center + Vec3::new(-half_size, 0.0,  half_size),
        ];
        self.line(corners[0], corners[1], color);
        self.line(corners[1], corners[2], color);
        self.line(corners[2], corners[3], color);
        self.line(corners[3], corners[0], color);
        // Diagonal cross
        self.line(corners[0], corners[2], [color[0], color[1], color[2], color[3] * 0.3]);
        self.line(corners[1], corners[3], [color[0], color[1], color[2], color[3] * 0.3]);
    }

    pub fn line_count(&self) -> usize {
        self.lines.len() / 2
    }
}

impl Default for DebugDraw {
    fn default() -> Self {
        Self::new()
    }
}
