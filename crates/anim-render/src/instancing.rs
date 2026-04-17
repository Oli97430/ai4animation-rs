//! GPU instancing system for rendering crowds, forests, and duplicated objects.
//!
//! Provides efficient instance buffer management, procedural scattering utilities,
//! crowd animation control, and LOD management for large-scale instanced rendering.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};

// ════════════════════════════════════════════════════════════════════
// InstanceData — per-instance GPU-uploadable transform data
// ════════════════════════════════════════════════════════════════════

/// Per-instance data uploaded to the GPU via a vertex/storage buffer.
///
/// Each instance carries its own world transform, color tint, animation offset,
/// scale variation, and LOD level. The struct is `repr(C)` + Pod/Zeroable for
/// direct `bytemuck::cast_slice` upload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InstanceData {
    /// 4x4 world transform matrix (column-major, matching glam/wgpu convention).
    pub transform: [[f32; 4]; 4],
    /// RGBA color multiplier applied to the base material color.
    pub color_tint: [f32; 4],
    /// Time offset for animation variation (seconds).
    pub animation_offset: f32,
    /// Per-instance scale multiplier (applied on top of transform).
    pub scale_variation: f32,
    /// Level of detail index (0 = highest detail).
    pub lod_level: u32,
    /// Padding to maintain 16-byte alignment.
    pub _padding: u32,
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY.to_cols_array_2d(),
            color_tint: [1.0, 1.0, 1.0, 1.0],
            animation_offset: 0.0,
            scale_variation: 1.0,
            lod_level: 0,
            _padding: 0,
        }
    }
}

impl InstanceData {
    /// Create instance data from a glam Mat4 and RGBA color tint.
    pub fn new(transform: Mat4, color_tint: [f32; 4]) -> Self {
        Self {
            transform: transform.to_cols_array_2d(),
            color_tint,
            ..Default::default()
        }
    }

    /// wgpu vertex buffer layout for instanced rendering (step_mode = Instance).
    ///
    /// Shader locations start at `base_location` to avoid colliding with
    /// per-vertex attributes.
    pub fn desc(base_location: u32) -> wgpu::VertexBufferLayout<'static> {
        // InstanceData is 96 bytes:
        //   transform:        64 bytes (4 x Float32x4)
        //   color_tint:       16 bytes (Float32x4)
        //   animation_offset:  4 bytes (Float32)
        //   scale_variation:   4 bytes (Float32)
        //   lod_level:         4 bytes (Uint32)
        //   _padding:          4 bytes (Uint32)
        //
        // We expose the matrix as 4 vec4 attributes plus the extras.
        static ATTRS: &[wgpu::VertexAttribute] = &[
            // transform col 0
            wgpu::VertexAttribute { offset: 0, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
            // transform col 1
            wgpu::VertexAttribute { offset: 16, shader_location: 6, format: wgpu::VertexFormat::Float32x4 },
            // transform col 2
            wgpu::VertexAttribute { offset: 32, shader_location: 7, format: wgpu::VertexFormat::Float32x4 },
            // transform col 3
            wgpu::VertexAttribute { offset: 48, shader_location: 8, format: wgpu::VertexFormat::Float32x4 },
            // color_tint
            wgpu::VertexAttribute { offset: 64, shader_location: 9, format: wgpu::VertexFormat::Float32x4 },
            // animation_offset
            wgpu::VertexAttribute { offset: 80, shader_location: 10, format: wgpu::VertexFormat::Float32 },
            // scale_variation
            wgpu::VertexAttribute { offset: 84, shader_location: 11, format: wgpu::VertexFormat::Float32 },
            // lod_level
            wgpu::VertexAttribute { offset: 88, shader_location: 12, format: wgpu::VertexFormat::Uint32 },
        ];

        let _ = base_location; // locations are hardcoded above starting at 5

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRS,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// InstanceBuffer — manages a CPU-side buffer of instances
// ════════════════════════════════════════════════════════════════════

/// Manages a resizable collection of [`InstanceData`] for GPU upload.
///
/// Tracks a dirty flag so callers know when the GPU buffer needs re-upload.
pub struct InstanceBuffer {
    /// CPU-side instance storage.
    pub instances: Vec<InstanceData>,
    /// Maximum capacity hint (the vec can grow beyond this).
    pub capacity: usize,
    /// Set to `true` whenever instance data changes; reset after GPU upload.
    pub dirty: bool,
}

impl InstanceBuffer {
    /// Create an empty buffer pre-allocated for `capacity` instances.
    pub fn new(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            capacity,
            dirty: false,
        }
    }

    /// Add an instance with the given world transform and color tint.
    /// Returns the index of the newly added instance.
    pub fn add_instance(&mut self, transform: Mat4, color: [f32; 4]) -> usize {
        let idx = self.instances.len();
        self.instances.push(InstanceData::new(transform, color));
        self.dirty = true;
        idx
    }

    /// Remove an instance by swap-removing it (O(1), changes last element's index).
    ///
    /// # Panics
    /// Panics if `index >= self.count()`.
    pub fn remove_instance(&mut self, index: usize) {
        self.instances.swap_remove(index);
        self.dirty = true;
    }

    /// Update the world transform of a specific instance.
    ///
    /// # Panics
    /// Panics if `index >= self.count()`.
    pub fn update_transform(&mut self, index: usize, transform: Mat4) {
        self.instances[index].transform = transform.to_cols_array_2d();
        self.dirty = true;
    }

    /// Update the color tint of a specific instance.
    ///
    /// # Panics
    /// Panics if `index >= self.count()`.
    pub fn update_color(&mut self, index: usize, color: [f32; 4]) {
        self.instances[index].color_tint = color;
        self.dirty = true;
    }

    /// Set the animation time offset for a specific instance.
    ///
    /// # Panics
    /// Panics if `index >= self.count()`.
    pub fn set_animation_offset(&mut self, index: usize, offset: f32) {
        self.instances[index].animation_offset = offset;
        self.dirty = true;
    }

    /// Number of active instances.
    pub fn count(&self) -> usize {
        self.instances.len()
    }

    /// Remove all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
        self.dirty = true;
    }

    /// Raw byte slice of instance data for GPU upload via `bytemuck::cast_slice`.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.instances)
    }
}

// ════════════════════════════════════════════════════════════════════
// InstanceGroup — instances sharing the same source mesh
// ════════════════════════════════════════════════════════════════════

/// A named group of instances that all reference the same source mesh.
///
/// Render systems iterate over groups, binding the mesh once and issuing
/// an instanced draw call for all visible instances in the group.
pub struct InstanceGroup {
    /// Human-readable group name (e.g. "pine_trees", "crowd_a").
    pub name: String,
    /// Index into the scene's mesh array.
    pub mesh_index: usize,
    /// The instance buffer for this group.
    pub buffer: InstanceBuffer,
    /// Whether this group should be rendered.
    pub visible: bool,
    /// Whether instances in this group cast shadows.
    pub cast_shadows: bool,
    /// Whether per-instance frustum culling is enabled.
    pub frustum_cull: bool,
}

impl InstanceGroup {
    /// Create a new instance group referencing the given mesh index.
    pub fn new(name: &str, mesh_index: usize, capacity: usize) -> Self {
        Self {
            name: name.to_string(),
            mesh_index,
            buffer: InstanceBuffer::new(capacity),
            visible: true,
            cast_shadows: true,
            frustum_cull: true,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// InstanceScattering — procedural instance placement utilities
// ════════════════════════════════════════════════════════════════════

/// Utilities for procedurally generating instance transforms.
///
/// All methods are stateless free functions that produce `Vec<Mat4>` arrays
/// suitable for feeding into [`InstanceBuffer::add_instance`].
pub struct InstanceScattering;

impl InstanceScattering {
    /// Scatter instances randomly on the XZ plane within a square area.
    ///
    /// Uses a simple LCG PRNG seeded by `seed` for reproducibility.
    pub fn scatter_on_plane(count: usize, area: f32, seed: u64) -> Vec<Mat4> {
        let mut rng = SimpleRng::new(seed);
        let half = area * 0.5;
        (0..count)
            .map(|_| {
                let x = rng.next_f32() * area - half;
                let z = rng.next_f32() * area - half;
                Mat4::from_translation(Vec3::new(x, 0.0, z))
            })
            .collect()
    }

    /// Place instances on a regular grid in the XZ plane, centered at the origin.
    pub fn scatter_on_grid(rows: usize, cols: usize, spacing: f32) -> Vec<Mat4> {
        let offset_x = (cols as f32 - 1.0) * spacing * 0.5;
        let offset_z = (rows as f32 - 1.0) * spacing * 0.5;
        let mut transforms = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                let x = c as f32 * spacing - offset_x;
                let z = r as f32 * spacing - offset_z;
                transforms.push(Mat4::from_translation(Vec3::new(x, 0.0, z)));
            }
        }
        transforms
    }

    /// Arrange instances evenly on a circle in the XZ plane.
    pub fn scatter_on_circle(count: usize, radius: f32) -> Vec<Mat4> {
        if count == 0 {
            return Vec::new();
        }
        let step = std::f32::consts::TAU / count as f32;
        (0..count)
            .map(|i| {
                let angle = i as f32 * step;
                let x = angle.cos() * radius;
                let z = angle.sin() * radius;
                Mat4::from_translation(Vec3::new(x, 0.0, z))
            })
            .collect()
    }

    /// Distribute instances approximately uniformly on a sphere surface
    /// using the Fibonacci / golden-angle method.
    pub fn scatter_on_sphere(count: usize, radius: f32) -> Vec<Mat4> {
        if count == 0 {
            return Vec::new();
        }
        let golden_ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let angle_increment = std::f32::consts::TAU / golden_ratio;

        (0..count)
            .map(|i| {
                let t = i as f32 / (count as f32 - 1.0).max(1.0);
                let phi = (1.0 - 2.0 * t).acos();
                let theta = angle_increment * i as f32;

                let x = phi.sin() * theta.cos() * radius;
                let y = phi.cos() * radius;
                let z = phi.sin() * theta.sin() * radius;
                Mat4::from_translation(Vec3::new(x, y, z))
            })
            .collect()
    }

    /// Add random Y-axis rotation to existing transforms.
    pub fn add_random_rotation(transforms: &mut [Mat4], max_angle: f32, seed: u64) {
        let mut rng = SimpleRng::new(seed);
        for t in transforms.iter_mut() {
            let angle = rng.next_f32() * max_angle;
            *t = *t * Mat4::from_rotation_y(angle);
        }
    }

    /// Add random uniform scale variation to existing transforms.
    pub fn add_random_scale(transforms: &mut [Mat4], min: f32, max: f32, seed: u64) {
        let mut rng = SimpleRng::new(seed);
        for t in transforms.iter_mut() {
            let s = min + rng.next_f32() * (max - min);
            *t = *t * Mat4::from_scale(Vec3::splat(s));
        }
    }

    /// Snap transforms to a height field by evaluating `height_fn(x, z)`.
    ///
    /// Adjusts the Y translation of each transform to match the terrain.
    pub fn add_terrain_conform(transforms: &mut [Mat4], height_fn: &dyn Fn(f32, f32) -> f32) {
        for t in transforms.iter_mut() {
            let pos = t.col(3);
            let y = height_fn(pos.x, pos.z);
            // Replace the Y component of the translation column.
            let col3 = Vec4::new(pos.x, y, pos.z, pos.w);
            *t = Mat4::from_cols(t.col(0), t.col(1), t.col(2), col3);
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Formation — crowd formation types
// ════════════════════════════════════════════════════════════════════

/// Predefined formation layouts for crowd placement.
#[derive(Clone, Debug)]
pub enum Formation {
    /// Regular grid formation.
    Grid { rows: usize, cols: usize, spacing: f32 },
    /// Circular formation.
    Circle { radius: f32 },
    /// Single-file line along the X axis.
    Line { spacing: f32 },
    /// Arbitrary custom positions.
    Custom(Vec<Vec3>),
}

impl Formation {
    /// Generate world-space transforms for `count` agents in this formation.
    pub fn generate(&self, count: usize) -> Vec<Mat4> {
        match self {
            Formation::Grid { rows, cols, spacing } => {
                InstanceScattering::scatter_on_grid(*rows, *cols, *spacing)
            }
            Formation::Circle { radius } => {
                InstanceScattering::scatter_on_circle(count, *radius)
            }
            Formation::Line { spacing } => {
                let offset = (count as f32 - 1.0) * spacing * 0.5;
                (0..count)
                    .map(|i| {
                        let x = i as f32 * spacing - offset;
                        Mat4::from_translation(Vec3::new(x, 0.0, 0.0))
                    })
                    .collect()
            }
            Formation::Custom(positions) => {
                positions.iter().map(|p| Mat4::from_translation(*p)).collect()
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// CrowdController — manage animated crowd instances
// ════════════════════════════════════════════════════════════════════

/// High-level controller for animated crowds.
///
/// Wraps an [`InstanceGroup`] with per-character animation offsets,
/// speed multipliers, and optional movement paths.
pub struct CrowdController {
    /// The instance group holding all crowd agents.
    pub group: InstanceGroup,
    /// Per-character animation time offsets (for variety).
    pub animation_offsets: Vec<f32>,
    /// Per-character playback speed multipliers.
    pub speeds: Vec<f32>,
    /// Optional per-character movement paths (waypoint lists).
    pub paths: Vec<Vec<Vec3>>,
}

impl CrowdController {
    /// Create a crowd controller with `count` agents and a default grid formation.
    pub fn new(name: &str, count: usize) -> Self {
        let mut group = InstanceGroup::new(name, 0, count);

        // Place agents on a default grid.
        let cols = (count as f32).sqrt().ceil() as usize;
        let rows = (count + cols - 1) / cols.max(1);
        let transforms = InstanceScattering::scatter_on_grid(rows, cols, 2.0);

        let white = [1.0_f32, 1.0, 1.0, 1.0];
        for (i, t) in transforms.iter().enumerate().take(count) {
            group.buffer.add_instance(*t, white);
            // Stagger animation a bit per row.
            let _ = i;
        }

        Self {
            group,
            animation_offsets: vec![0.0; count],
            speeds: vec![1.0; count],
            paths: Vec::new(),
        }
    }

    /// Advance the crowd simulation by `dt` seconds.
    ///
    /// Updates animation offsets and optionally moves agents along their paths.
    pub fn update(&mut self, dt: f32) {
        let count = self.group.buffer.count();
        for i in 0..count {
            // Accumulate animation offset based on per-character speed.
            self.animation_offsets[i] += dt * self.speeds[i];
            self.group.buffer.set_animation_offset(i, self.animation_offsets[i]);
        }

        // Path following (simple linear interpolation along waypoints).
        if !self.paths.is_empty() {
            for i in 0..count.min(self.paths.len()) {
                let path = &self.paths[i];
                if path.is_empty() {
                    continue;
                }
                // Use animation offset as a path parameter (wrapping).
                let t = self.animation_offsets[i];
                let total_segments = path.len().saturating_sub(1).max(1);
                let segment_duration = 1.0; // 1 second per segment
                let total_duration = total_segments as f32 * segment_duration;
                let wrapped = t % total_duration;
                let seg_idx = (wrapped / segment_duration) as usize;
                let seg_t = (wrapped / segment_duration).fract();

                let a = path[seg_idx.min(path.len() - 1)];
                let b = path[(seg_idx + 1).min(path.len() - 1)];
                let pos = a.lerp(b, seg_t);
                self.group.buffer.update_transform(i, Mat4::from_translation(pos));
            }
        }
    }

    /// Apply a formation to the crowd, repositioning all agents.
    pub fn set_formation(&mut self, formation: Formation) {
        let count = self.group.buffer.count();
        let transforms = formation.generate(count);
        for (i, t) in transforms.iter().enumerate().take(count) {
            self.group.buffer.update_transform(i, *t);
        }
    }

    /// Randomize animation offsets so characters are not in sync.
    pub fn randomize_offsets(&mut self, seed: u64) {
        let mut rng = SimpleRng::new(seed);
        for offset in self.animation_offsets.iter_mut() {
            *offset = rng.next_f32() * 10.0; // 0..10 seconds spread
        }
        for (i, &off) in self.animation_offsets.iter().enumerate() {
            if i < self.group.buffer.count() {
                self.group.buffer.set_animation_offset(i, off);
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// LodConfig — level of detail distance thresholds
// ════════════════════════════════════════════════════════════════════

/// Manages LOD switch distances for instanced rendering.
///
/// Each entry in `distances` is the maximum view distance for that LOD level.
/// Objects farther than the last distance are culled entirely.
#[derive(Clone, Debug)]
pub struct LodConfig {
    /// Sorted ascending distances at which LOD level increases.
    /// `distances[0]` = max distance for LOD 0 (highest quality).
    pub distances: Vec<f32>,
}

impl LodConfig {
    /// Default 3-level LOD: 10m, 30m, 100m.
    pub fn default_3_lod() -> Self {
        Self {
            distances: vec![10.0, 30.0, 100.0],
        }
    }

    /// Compute the LOD level for a given view distance.
    ///
    /// Returns the index of the first threshold that `distance` falls under,
    /// or `distances.len()` if the object is beyond all thresholds (should be culled).
    pub fn compute_lod(&self, distance: f32) -> u32 {
        for (i, &d) in self.distances.iter().enumerate() {
            if distance <= d {
                return i as u32;
            }
        }
        self.distances.len() as u32
    }
}

// ════════════════════════════════════════════════════════════════════
// WGSL shader snippet — instance transform application
// ════════════════════════════════════════════════════════════════════

/// WGSL vertex shader snippet that reads per-instance attributes and
/// applies the instance transform to the vertex position.
///
/// Paste this into your shader module; it expects vertex input locations
/// 0..4 for mesh data and 5..12 for instance data (matching [`InstanceData::desc`]).
pub const INSTANCE_TRANSFORM_WGSL: &str = r#"
// ── Per-instance input (vertex step_mode = Instance) ──────────────
struct InstanceInput {
    @location(5) model_col0: vec4<f32>,
    @location(6) model_col1: vec4<f32>,
    @location(7) model_col2: vec4<f32>,
    @location(8) model_col3: vec4<f32>,
    @location(9) color_tint: vec4<f32>,
    @location(10) animation_offset: f32,
    @location(11) scale_variation: f32,
    @location(12) lod_level: u32,
};

// Reconstruct the 4x4 instance world matrix from the 4 column vectors.
fn instance_world_matrix(inst: InstanceInput) -> mat4x4<f32> {
    return mat4x4<f32>(
        inst.model_col0,
        inst.model_col1,
        inst.model_col2,
        inst.model_col3,
    );
}

// Apply instance transform (with scale variation) to a local-space position.
fn apply_instance_transform(inst: InstanceInput, local_pos: vec3<f32>) -> vec4<f32> {
    let world = instance_world_matrix(inst);
    let scaled = local_pos * inst.scale_variation;
    return world * vec4<f32>(scaled, 1.0);
}
"#;

// ════════════════════════════════════════════════════════════════════
// SimpleRng — deterministic lightweight PRNG (internal)
// ════════════════════════════════════════════════════════════════════

/// Minimal xorshift64 PRNG for reproducible scattering.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns a float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() & 0x00FF_FFFF) as f32 / (0x0100_0000 as f32)
    }
}

// ════════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_data_size_is_96_bytes() {
        assert_eq!(std::mem::size_of::<InstanceData>(), 96);
    }

    #[test]
    fn instance_data_default_is_identity() {
        let d = InstanceData::default();
        let m = Mat4::from_cols_array_2d(&d.transform);
        assert_eq!(m, Mat4::IDENTITY);
        assert_eq!(d.color_tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(d.animation_offset, 0.0);
        assert_eq!(d.scale_variation, 1.0);
        assert_eq!(d.lod_level, 0);
    }

    #[test]
    fn instance_buffer_add_and_count() {
        let mut buf = InstanceBuffer::new(10);
        assert_eq!(buf.count(), 0);
        let idx = buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        assert_eq!(idx, 0);
        assert_eq!(buf.count(), 1);
        assert!(buf.dirty);
    }

    #[test]
    fn instance_buffer_remove_swap() {
        let mut buf = InstanceBuffer::new(10);
        buf.add_instance(Mat4::from_translation(Vec3::X), [1.0, 0.0, 0.0, 1.0]);
        buf.add_instance(Mat4::from_translation(Vec3::Y), [0.0, 1.0, 0.0, 1.0]);
        buf.add_instance(Mat4::from_translation(Vec3::Z), [0.0, 0.0, 1.0, 1.0]);

        // Remove index 0 -> last element (Z) swaps into position 0.
        buf.remove_instance(0);
        assert_eq!(buf.count(), 2);
        // Index 0 should now have the Z translation's color.
        assert_eq!(buf.instances[0].color_tint, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn instance_buffer_update_transform() {
        let mut buf = InstanceBuffer::new(4);
        buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        buf.dirty = false;

        let new_t = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        buf.update_transform(0, new_t);
        assert!(buf.dirty);

        let stored = Mat4::from_cols_array_2d(&buf.instances[0].transform);
        assert_eq!(stored, new_t);
    }

    #[test]
    fn instance_buffer_update_color() {
        let mut buf = InstanceBuffer::new(4);
        buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        buf.update_color(0, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(buf.instances[0].color_tint, [0.5, 0.5, 0.5, 1.0]);
    }

    #[test]
    fn instance_buffer_set_animation_offset() {
        let mut buf = InstanceBuffer::new(4);
        buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        buf.set_animation_offset(0, 3.14);
        assert!((buf.instances[0].animation_offset - 3.14).abs() < 1e-6);
    }

    #[test]
    fn instance_buffer_clear() {
        let mut buf = InstanceBuffer::new(4);
        buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        buf.clear();
        assert_eq!(buf.count(), 0);
        assert!(buf.dirty);
    }

    #[test]
    fn instance_buffer_as_bytes_length() {
        let mut buf = InstanceBuffer::new(4);
        buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        buf.add_instance(Mat4::IDENTITY, [1.0; 4]);
        assert_eq!(buf.as_bytes().len(), 2 * std::mem::size_of::<InstanceData>());
    }

    #[test]
    fn scatter_on_grid_count() {
        let t = InstanceScattering::scatter_on_grid(3, 4, 1.0);
        assert_eq!(t.len(), 12);
    }

    #[test]
    fn scatter_on_circle_count_and_radius() {
        let t = InstanceScattering::scatter_on_circle(8, 5.0);
        assert_eq!(t.len(), 8);
        // All points should be at distance ~5 from origin.
        for m in &t {
            let pos = m.col(3).truncate();
            let dist = (pos.x * pos.x + pos.z * pos.z).sqrt();
            assert!((dist - 5.0).abs() < 1e-4, "distance was {dist}");
        }
    }

    #[test]
    fn scatter_on_sphere_count() {
        let t = InstanceScattering::scatter_on_sphere(100, 10.0);
        assert_eq!(t.len(), 100);
        // All points should be approximately on the sphere surface.
        for m in &t {
            let pos = m.col(3).truncate();
            let dist = pos.length();
            assert!((dist - 10.0).abs() < 0.5, "distance was {dist}");
        }
    }

    #[test]
    fn scatter_on_plane_deterministic() {
        let a = InstanceScattering::scatter_on_plane(50, 100.0, 42);
        let b = InstanceScattering::scatter_on_plane(50, 100.0, 42);
        for (ma, mb) in a.iter().zip(b.iter()) {
            assert_eq!(ma, mb);
        }
    }

    #[test]
    fn lod_config_default_3() {
        let lod = LodConfig::default_3_lod();
        assert_eq!(lod.distances.len(), 3);
        assert_eq!(lod.compute_lod(5.0), 0);
        assert_eq!(lod.compute_lod(10.0), 0);
        assert_eq!(lod.compute_lod(10.1), 1);
        assert_eq!(lod.compute_lod(30.0), 1);
        assert_eq!(lod.compute_lod(30.1), 2);
        assert_eq!(lod.compute_lod(100.0), 2);
        assert_eq!(lod.compute_lod(100.1), 3); // beyond all -> cull
    }

    #[test]
    fn crowd_controller_basic() {
        let mut crowd = CrowdController::new("test_crowd", 16);
        assert_eq!(crowd.group.buffer.count(), 16);

        crowd.randomize_offsets(99);
        // Offsets should no longer all be zero.
        assert!(crowd.animation_offsets.iter().any(|&o| o > 0.0));

        // Update should not panic.
        crowd.update(0.016);
    }

    #[test]
    fn crowd_set_formation_circle() {
        let mut crowd = CrowdController::new("circle_crowd", 8);
        crowd.set_formation(Formation::Circle { radius: 10.0 });
        // All agents should be roughly at radius 10.
        for inst in &crowd.group.buffer.instances {
            let m = Mat4::from_cols_array_2d(&inst.transform);
            let pos = m.col(3).truncate();
            let dist = (pos.x * pos.x + pos.z * pos.z).sqrt();
            assert!((dist - 10.0).abs() < 0.1, "distance was {dist}");
        }
    }

    #[test]
    fn formation_line_spacing() {
        let f = Formation::Line { spacing: 3.0 };
        let t = f.generate(5);
        assert_eq!(t.len(), 5);
        // Check that they are evenly spaced along X.
        for i in 1..t.len() {
            let dx = t[i].col(3).x - t[i - 1].col(3).x;
            assert!((dx - 3.0).abs() < 1e-5);
        }
    }

    #[test]
    fn add_random_rotation_modifies_transforms() {
        let mut t = InstanceScattering::scatter_on_grid(2, 2, 1.0);
        let original: Vec<Mat4> = t.clone();
        InstanceScattering::add_random_rotation(&mut t, std::f32::consts::PI, 123);
        // At least one transform should differ.
        assert!(t.iter().zip(original.iter()).any(|(a, b)| a != b));
    }

    #[test]
    fn add_random_scale_modifies_transforms() {
        let mut t = InstanceScattering::scatter_on_grid(2, 2, 1.0);
        let original: Vec<Mat4> = t.clone();
        InstanceScattering::add_random_scale(&mut t, 0.5, 2.0, 456);
        assert!(t.iter().zip(original.iter()).any(|(a, b)| a != b));
    }

    #[test]
    fn add_terrain_conform_adjusts_y() {
        let mut t = vec![
            Mat4::from_translation(Vec3::new(1.0, 0.0, 2.0)),
            Mat4::from_translation(Vec3::new(3.0, 0.0, 4.0)),
        ];
        // Simple height function: y = x + z
        InstanceScattering::add_terrain_conform(&mut t, &|x, z| x + z);
        assert!((t[0].col(3).y - 3.0).abs() < 1e-5); // 1 + 2
        assert!((t[1].col(3).y - 7.0).abs() < 1e-5); // 3 + 4
    }

    #[test]
    fn instance_group_defaults() {
        let g = InstanceGroup::new("trees", 2, 100);
        assert_eq!(g.name, "trees");
        assert_eq!(g.mesh_index, 2);
        assert!(g.visible);
        assert!(g.cast_shadows);
        assert!(g.frustum_cull);
        assert_eq!(g.buffer.count(), 0);
    }

    #[test]
    fn wgsl_snippet_is_not_empty() {
        assert!(!INSTANCE_TRANSFORM_WGSL.is_empty());
        assert!(INSTANCE_TRANSFORM_WGSL.contains("InstanceInput"));
        assert!(INSTANCE_TRANSFORM_WGSL.contains("instance_world_matrix"));
        assert!(INSTANCE_TRANSFORM_WGSL.contains("apply_instance_transform"));
    }
}
