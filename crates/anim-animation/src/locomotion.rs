//! Locomotion controller — neural network-driven motion synthesis.
//!
//! Ports the Python `Demos/Locomotion/Biped/Program.py` inference loop to Rust.
//! The pipeline: Control() -> Predict() -> Animate() every frame.
//!
//! Prediction runs at PREDICTION_FPS (10 Hz), producing a Sequence of
//! SEQUENCE_LENGTH (16) future frames spanning SEQUENCE_WINDOW (0.5s).
//! Animation interpolates between previous and current sequences every frame.

use glam::{Mat4, Vec3, Quat};
use anyhow::{Result, Context};
use crate::onnx_inference::{OnnxModel, ModelMetadata};
use crate::actor::Actor;
use anim_math::transform::Transform;

// ── Constants (matching Python) ─────────────────────────────

const SEQUENCE_WINDOW: f32 = 0.5;
const SEQUENCE_LENGTH: usize = 16;
const SEQUENCE_FPS: f32 = 30.0;
const PREDICTION_FPS: f32 = 10.0;
const MIN_TIMESCALE: f32 = 1.0;
const MAX_TIMESCALE: f32 = 1.5;
const SYNCHRONIZATION_SENSITIVITY: f32 = 5.0;
const TIMESCALE_SENSITIVITY: f32 = 5.0;
const DEFAULT_NUM_BONES: usize = 23;

// ── FeedTensor / ReadTensor ─────────────────────────────────

/// Accumulates data into a flat f32 vector (mirrors Python FeedTensor).
struct FeedTensor {
    data: Vec<f32>,
    pivot: usize,
}

impl FeedTensor {
    fn new(dim: usize) -> Self {
        Self {
            data: vec![0.0; dim],
            pivot: 0,
        }
    }

    /// Feed raw f32 values sequentially.
    fn feed(&mut self, values: &[f32]) {
        let n = values.len().min(self.data.len() - self.pivot);
        self.data[self.pivot..self.pivot + n].copy_from_slice(&values[..n]);
        self.pivot += n;
    }

    /// Feed Vec3 values as flat [x, y, z, x, y, z, ...].
    fn feed_vec3(&mut self, values: &[Vec3]) {
        for v in values {
            self.feed(&[v.x, v.y, v.z]);
        }
    }

    /// Feed Vec3 values but only XZ components (skip Y).
    fn feed_vec3_xz(&mut self, values: &[Vec3]) {
        for v in values {
            self.feed(&[v.x, v.z]);
        }
    }

    /// Feed Mat4 positions as [x, y, z, ...].
    fn feed_positions(&mut self, transforms: &[Mat4]) {
        for t in transforms {
            let p = t.get_position();
            self.feed(&[p.x, p.y, p.z]);
        }
    }

    /// Feed Mat4 Z-axis directions.
    fn feed_axis_z(&mut self, transforms: &[Mat4]) {
        for t in transforms {
            let z = t.get_axis_z();
            self.feed(&[z.x, z.y, z.z]);
        }
    }

    /// Feed Mat4 Y-axis directions.
    fn feed_axis_y(&mut self, transforms: &[Mat4]) {
        for t in transforms {
            let y = t.get_axis_y();
            self.feed(&[y.x, y.y, y.z]);
        }
    }

    /// Get the accumulated tensor data.
    fn as_slice(&self) -> &[f32] {
        &self.data
    }
}

/// Reads structured data from a flat f32 vector (mirrors Python ReadTensor).
struct ReadTensor<'a> {
    data: &'a [f32],
    pivot: usize,
    /// Stride per frame (output_dim).
    frame_stride: usize,
    /// Total frames in the output.
    num_frames: usize,
}

impl<'a> ReadTensor<'a> {
    fn new(data: &'a [f32], num_frames: usize) -> Self {
        let frame_stride = if num_frames > 0 { data.len() / num_frames } else { data.len() };
        Self {
            data,
            pivot: 0,
            frame_stride,
            num_frames,
        }
    }

    /// Read `count` f32 values per frame -> [num_frames, count].
    fn read(&mut self, count: usize) -> Vec<Vec<f32>> {
        let mut result = Vec::with_capacity(self.num_frames);
        for f in 0..self.num_frames {
            let start = f * self.frame_stride + self.pivot;
            let end = start + count;
            if end <= self.data.len() {
                result.push(self.data[start..end].to_vec());
            }
        }
        self.pivot += count;
        result
    }

    /// Read Vec3 values: [num_frames, bone_count] -> Vec<Vec<Vec3>>.
    fn read_vec3(&mut self, bone_count: usize) -> Vec<Vec<Vec3>> {
        let raw = self.read(bone_count * 3);
        raw.iter().map(|frame| {
            frame.chunks_exact(3)
                .map(|c| Vec3::new(c[0], c[1], c[2]))
                .collect()
        }).collect()
    }

    /// Read Rotation3D: 6 values per bone (z_vec + y_vec) -> [num_frames, bone_count] of Quat.
    fn read_rotation3d(&mut self, bone_count: usize) -> Vec<Vec<Quat>> {
        let raw = self.read(bone_count * 6);
        raw.iter().map(|frame| {
            frame.chunks_exact(6).map(|c| {
                let z = Vec3::new(c[0], c[1], c[2]).normalize_or_zero();
                let y = Vec3::new(c[3], c[4], c[5]).normalize_or_zero();
                quat_from_look(z, y)
            }).collect()
        }).collect()
    }

    /// Read scalar values per frame: [num_frames, count].
    fn read_scalars(&mut self, count: usize) -> Vec<Vec<f32>> {
        self.read(count)
    }
}

// ── Sequence ────────────────────────────────────────────────

/// A predicted motion sequence (mirrors Python Sequence class).
#[derive(Clone)]
#[allow(dead_code)]
struct Sequence {
    /// Timestamps for each frame (relative to prediction time).
    timestamps: Vec<f32>,
    /// Root transforms [SEQUENCE_LENGTH].
    root_transforms: Vec<Mat4>,
    /// Root velocities [SEQUENCE_LENGTH].
    root_velocities: Vec<Vec3>,
    /// Per-bone positions [SEQUENCE_LENGTH][num_bones].
    bone_positions: Vec<Vec<Vec3>>,
    /// Per-bone rotations [SEQUENCE_LENGTH][num_bones].
    bone_rotations: Vec<Vec<Quat>>,
    /// Per-bone velocities [SEQUENCE_LENGTH][num_bones].
    bone_velocities: Vec<Vec<Vec3>>,
    /// Contact states [SEQUENCE_LENGTH][4] (left ankle, left ball, right ankle, right ball).
    contacts: Vec<Vec<f32>>,
    /// Guidance positions [SEQUENCE_LENGTH][num_bones].
    guidances: Vec<Vec<Vec3>>,
}

impl Sequence {
    fn empty() -> Self {
        Self {
            timestamps: vec![0.0; SEQUENCE_LENGTH],
            root_transforms: vec![Mat4::IDENTITY; SEQUENCE_LENGTH],
            root_velocities: vec![Vec3::ZERO; SEQUENCE_LENGTH],
            bone_positions: Vec::new(),
            bone_rotations: Vec::new(),
            bone_velocities: Vec::new(),
            contacts: Vec::new(),
            guidances: Vec::new(),
        }
    }

    /// Get the total trajectory length.
    fn get_length(&self) -> f32 {
        let mut length = 0.0f32;
        for i in 1..self.root_transforms.len() {
            let p0 = self.root_transforms[i - 1].get_position();
            let p1 = self.root_transforms[i].get_position();
            length += (p1 - p0).length();
        }
        length
    }

    /// Linear interpolation helper: find frame indices and blend factor for a timestamp.
    fn sample_index(&self, t: f32) -> (usize, usize, f32) {
        if self.timestamps.len() < 2 {
            return (0, 0, 0.0);
        }
        // Find where t falls in the timestamps array
        for i in 0..self.timestamps.len() - 1 {
            if t <= self.timestamps[i + 1] {
                let span = self.timestamps[i + 1] - self.timestamps[i];
                let blend = if span > 1e-6 { (t - self.timestamps[i]) / span } else { 0.0 };
                return (i, i + 1, blend.clamp(0.0, 1.0));
            }
        }
        let last = self.timestamps.len() - 1;
        (last, last, 0.0)
    }

    /// Sample root transform at time t.
    fn sample_root(&self, t: f32) -> Mat4 {
        let (a, b, blend) = self.sample_index(t);
        self.root_transforms[a].interpolate(&self.root_transforms[b], blend)
    }

    /// Sample bone positions at time t.
    fn sample_positions(&self, t: f32) -> Vec<Vec3> {
        if self.bone_positions.is_empty() {
            return Vec::new();
        }
        let (a, b, blend) = self.sample_index(t);
        lerp_vec3_arrays(&self.bone_positions[a], &self.bone_positions[b], blend)
    }

    /// Sample bone rotations at time t.
    fn sample_rotations(&self, t: f32) -> Vec<Quat> {
        if self.bone_rotations.is_empty() {
            return Vec::new();
        }
        let (a, b, blend) = self.sample_index(t);
        self.bone_rotations[a].iter().zip(&self.bone_rotations[b])
            .map(|(&qa, &qb)| qa.slerp(qb, blend))
            .collect()
    }

    /// Sample bone velocities at time t.
    fn sample_velocities(&self, t: f32) -> Vec<Vec3> {
        if self.bone_velocities.is_empty() {
            return Vec::new();
        }
        let (a, b, blend) = self.sample_index(t);
        lerp_vec3_arrays(&self.bone_velocities[a], &self.bone_velocities[b], blend)
    }

    /// Sample contact values at time t.
    fn sample_contacts(&self, t: f32) -> Vec<f32> {
        if self.contacts.is_empty() {
            return vec![0.0; 4];
        }
        let (a, b, blend) = self.sample_index(t);
        self.contacts[a].iter().zip(&self.contacts[b])
            .map(|(ca, cb)| ca + (cb - ca) * blend)
            .collect()
    }

    /// Sample guidance positions at time t.
    #[allow(dead_code)]
    fn sample_guidance(&self, t: f32) -> Vec<Vec3> {
        if self.guidances.is_empty() {
            return Vec::new();
        }
        let (a, b, blend) = self.sample_index(t);
        lerp_vec3_arrays(&self.guidances[a], &self.guidances[b], blend)
    }
}

// ── Root control (trajectory) ───────────────────────────────

/// Root trajectory control (mirrors Python RootModule.Series).
struct RootControl {
    transforms: Vec<Mat4>,
    velocities: Vec<Vec3>,
    timestamps: Vec<f32>,
}

impl RootControl {
    fn new() -> Self {
        let ts = TimestampsSeries::new(0.0, SEQUENCE_WINDOW, SEQUENCE_LENGTH);
        Self {
            transforms: vec![Mat4::IDENTITY; SEQUENCE_LENGTH],
            velocities: vec![Vec3::ZERO; SEQUENCE_LENGTH],
            timestamps: ts.values,
        }
    }

    /// Update the trajectory based on user control input.
    fn control(&mut self, position: Vec3, direction: Vec3, velocity: Vec3, _dt: f32) {
        if self.transforms.is_empty() {
            return;
        }
        let n = self.transforms.len();

        // Set current frame (first sample)
        let mut root = Mat4::IDENTITY;
        root.set_position(position);
        if direction.length_squared() > 0.001 {
            let forward = direction.normalize();
            root = look_at_transform(position, forward);
        }
        self.transforms[0] = root;
        self.velocities[0] = velocity;

        // Extrapolate future frames
        for i in 1..n {
            let t = self.timestamps[i];
            let future_pos = position + velocity * t;
            let mut future_root = root;
            future_root.set_position(future_pos);
            self.transforms[i] = future_root;
            self.velocities[i] = velocity;
        }
    }

    /// Get position at sample index.
    fn get_position(&self, index: usize) -> Vec3 {
        self.transforms[index].get_position()
    }
}

// ── Simple timestamps series ────────────────────────────────

struct TimestampsSeries {
    values: Vec<f32>,
}

impl TimestampsSeries {
    fn new(start: f32, end: f32, count: usize) -> Self {
        let values = if count <= 1 {
            vec![(start + end) * 0.5]
        } else {
            (0..count).map(|i| {
                start + (end - start) * i as f32 / (count - 1) as f32
            }).collect()
        };
        Self { values }
    }
}

// ── Locomotion Controller ───────────────────────────────────

/// Neural network-driven locomotion controller.
///
/// Runs a trained CodebookMatching model to produce continuous character motion
/// from user control inputs (velocity, direction).
pub struct LocomotionController {
    /// ONNX model for inference.
    model: OnnxModel,
    /// Model metadata (dimensions, etc.).
    pub metadata: ModelMetadata,
    /// Number of bones in the skeleton.
    num_bones: usize,

    // ── State ────────────────────────────────────────────
    /// Previous predicted sequence (for blending).
    previous: Sequence,
    /// Current predicted sequence.
    current: Sequence,
    /// Time of last prediction.
    prediction_timestamp: f32,
    /// Accumulated total time.
    total_time: f32,

    /// Adaptive timescale for synchronization.
    timescale: f32,
    /// Synchronization factor [0, 1].
    synchronization: f32,

    // ── Control ──────────────────────────────────────────
    /// Root trajectory control.
    root_control: RootControl,
    /// Guidance positions [num_bones, 3] for style control.
    guidance_positions: Vec<Vec3>,
    /// Trajectory correction blend factor.
    #[allow(dead_code)]
    trajectory_correction: f32,
    /// Guidance correction blend factor.
    #[allow(dead_code)]
    guidance_correction: f32,

    /// Whether the controller is active and ready.
    pub active: bool,
}

impl LocomotionController {
    /// Create a new locomotion controller from model files.
    ///
    /// `model_path`: Path to Network.onnx
    /// `meta_path`: Path to Network_meta.npz
    /// `num_bones`: Number of skeleton bones (default: 23 for Geno)
    pub fn new(
        model_path: &std::path::Path,
        meta_path: &std::path::Path,
        num_bones: Option<usize>,
    ) -> Result<Self> {
        let model = OnnxModel::load(model_path)
            .with_context(|| "Impossible de charger le modèle locomotion")?;

        let metadata = ModelMetadata::load(meta_path)
            .with_context(|| "Impossible de charger les métadonnées")?;

        let num_bones = num_bones
            .or(metadata.num_bones)
            .unwrap_or(DEFAULT_NUM_BONES);

        log::info!(
            "Contrôleur locomotion initialisé: {} bones, input={}, latent={}",
            num_bones, metadata.input_dim, metadata.latent_dim
        );

        Ok(Self {
            model,
            metadata,
            num_bones,
            previous: Sequence::empty(),
            current: Sequence::empty(),
            prediction_timestamp: 0.0,
            total_time: 0.0,
            timescale: 1.0,
            synchronization: 0.0,
            root_control: RootControl::new(),
            guidance_positions: vec![Vec3::ZERO; num_bones],
            trajectory_correction: 0.25,
            guidance_correction: 0.0,
            active: true,
        })
    }

    /// Set guidance positions (style control). Call before update().
    pub fn set_guidance(&mut self, positions: &[Vec3]) {
        let n = positions.len().min(self.num_bones);
        self.guidance_positions[..n].copy_from_slice(&positions[..n]);
    }

    /// Main update loop — called every frame.
    ///
    /// `actor`: current skeleton state
    /// `velocity`: desired movement velocity (world space)
    /// `direction`: desired facing direction (world space)
    /// `dt`: frame delta time
    pub fn update(
        &mut self,
        actor: &mut Actor,
        velocity: Vec3,
        direction: Vec3,
        dt: f32,
    ) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        self.total_time += dt;

        // ── Control (every frame) ────────────────────────
        let position = Vec3::lerp(
            self.root_control.get_position(0),
            actor.root_position(),
            self.synchronization,
        );
        self.root_control.control(position, direction, velocity, dt);

        // ── Predict (at PREDICTION_FPS) ──────────────────
        if self.prediction_timestamp == 0.0
            || self.total_time - self.prediction_timestamp > 1.0 / PREDICTION_FPS
        {
            self.prediction_timestamp = self.total_time;
            self.predict(actor)?;
        }

        // ── Animate (every frame) ────────────────────────
        self.animate(actor, dt);

        Ok(())
    }

    /// Run neural network prediction to generate a future motion sequence.
    fn predict(&mut self, actor: &Actor) -> Result<()> {
        let input_dim = self.metadata.input_dim;
        let latent_dim = self.metadata.latent_dim;
        let mut feed = FeedTensor::new(input_dim);

        // ── Assemble input tensor ────────────────────────

        // 1. Current bone state relative to root
        let root = actor.root;
        let root_inv = root.inverse();

        // Transform all bone transforms to root-local space
        let local_transforms: Vec<Mat4> = actor.transforms.iter()
            .map(|t| root_inv * *t)
            .collect();

        // Positions [num_bones, 3]
        feed.feed_positions(&local_transforms);
        // Z-axes [num_bones, 3]
        feed.feed_axis_z(&local_transforms);
        // Y-axes [num_bones, 3]
        feed.feed_axis_y(&local_transforms);

        // Velocities in root frame [num_bones, 3]
        let root_rot_inv = Mat4::from_cols(
            root.x_axis,
            root.y_axis,
            root.z_axis,
            glam::Vec4::W,
        ).inverse();
        let local_velocities: Vec<Vec3> = actor.velocities.iter()
            .map(|v| root_rot_inv.transform_vector3(*v))
            .collect();
        feed.feed_vec3(&local_velocities);

        // 2. Future root trajectory (XZ only) [SEQUENCE_LENGTH, 2] each
        let local_root_transforms: Vec<Mat4> = self.root_control.transforms.iter()
            .map(|t| root_inv * *t)
            .collect();
        let local_root_velocities: Vec<Vec3> = self.root_control.velocities.iter()
            .map(|v| root_rot_inv.transform_vector3(*v))
            .collect();

        // Root positions XZ
        feed.feed_vec3_xz(&local_root_transforms.iter()
            .map(|t| t.get_position())
            .collect::<Vec<_>>());
        // Root directions XZ
        feed.feed_vec3_xz(&local_root_transforms.iter()
            .map(|t| t.get_axis_z().normalize_or_zero())
            .collect::<Vec<_>>());
        // Root velocities XZ
        feed.feed_vec3_xz(&local_root_velocities);

        // 3. Guidance positions [num_bones, 3]
        feed.feed_vec3(&self.guidance_positions);

        // ── Normalize input ──────────────────────────────
        let normalized_input = self.normalize_input(feed.as_slice());

        // ── Run inference ────────────────────────────────
        let noise: Vec<f32> = vec![0.0; latent_dim]; // deterministic
        let seed: Vec<f32> = vec![0.0; latent_dim];

        let raw_output = self.model.run_locomotion(
            &normalized_input,
            input_dim,
            &noise,
            latent_dim,
            &seed,
        )?;

        // ── Denormalize output ───────────────────────────
        let output = self.denormalize_output(&raw_output);

        // ── Parse output ─────────────────────────────────
        let mut read = ReadTensor::new(&output, SEQUENCE_LENGTH);

        // Root velocity vectors [SEQUENCE_LENGTH, 3]
        let root_vectors = read.read_vec3(1);
        let root_vectors: Vec<Vec3> = root_vectors.into_iter()
            .map(|v| v.into_iter().next().unwrap_or(Vec3::ZERO))
            .collect();

        // Cumulative sum -> root delta positions
        let mut root_deltas = vec![Vec3::ZERO; SEQUENCE_LENGTH];
        for i in 1..SEQUENCE_LENGTH {
            root_deltas[i] = root_deltas[i - 1] + root_vectors[i];
        }

        // Convert to world-space root transforms
        let future_root_transforms: Vec<Mat4> = root_deltas.iter().map(|delta| {
            let mut t = root;
            let world_pos = root.get_position() + root.transform_vector3(*delta);
            t.set_position(world_pos);
            t
        }).collect();

        // Root velocities from vectors
        let future_root_velocities: Vec<Vec3> = root_vectors.iter().map(|v| {
            let world_v = Vec3::new(v.x * SEQUENCE_FPS, 0.0, v.z * SEQUENCE_FPS);
            root.transform_vector3(world_v)
        }).collect();

        // Bone positions [SEQUENCE_LENGTH, num_bones, 3]
        let bone_positions = read.read_vec3(self.num_bones);
        // Bone rotations [SEQUENCE_LENGTH, num_bones, 6] -> Quat
        let bone_rotations = read.read_rotation3d(self.num_bones);
        // Bone velocities [SEQUENCE_LENGTH, num_bones, 3]
        let bone_velocities = read.read_vec3(self.num_bones);
        // Contacts [SEQUENCE_LENGTH, 4]
        let contacts = read.read_scalars(4);
        // Guidance [SEQUENCE_LENGTH, num_bones, 3]
        let guidances = read.read_vec3(self.num_bones);

        // ── Build new sequence ───────────────────────────
        self.previous = self.current.clone();
        self.current = Sequence {
            timestamps: TimestampsSeries::new(0.0, SEQUENCE_WINDOW, SEQUENCE_LENGTH).values,
            root_transforms: future_root_transforms,
            root_velocities: future_root_velocities,
            bone_positions,
            bone_rotations,
            bone_velocities,
            contacts: contacts.into_iter().map(|c| {
                c.into_iter().map(|v| v.clamp(0.0, 1.0)).collect()
            }).collect(),
            guidances,
        };

        Ok(())
    }

    /// Animate the actor based on predicted sequences (every frame).
    fn animate(&mut self, actor: &mut Actor, dt: f32) {
        // Adaptive timescale for trajectory synchronization
        let required_speed = {
            let dist = (actor.root_position() - self.root_control.get_position(0)).length();
            (dist + self.current.get_length()) / SEQUENCE_WINDOW
        };
        let predicted_speed = self.current.get_length() / SEQUENCE_WINDOW;

        let (ts, sync) = if required_speed > 0.1 && predicted_speed > 0.1 {
            (required_speed / predicted_speed, 1.0)
        } else {
            (1.0, 0.0)
        };

        self.timescale = interpolate_dt(self.timescale, ts, dt, TIMESCALE_SENSITIVITY);
        self.timescale = self.timescale.clamp(MIN_TIMESCALE, MAX_TIMESCALE);
        self.synchronization = interpolate_dt(self.synchronization, sync, dt, SYNCHRONIZATION_SENSITIVITY);

        let sdt = dt * self.timescale;

        // Blend between previous and current sequences
        let blend = (self.total_time - self.prediction_timestamp) * PREDICTION_FPS;
        let blend = blend.clamp(0.0, 1.0);

        // Sample from sequences
        let root = Mat4::interpolate(
            &self.previous.sample_root(sdt),
            &self.current.sample_root(sdt),
            blend,
        );

        let prev_positions = self.previous.sample_positions(sdt);
        let curr_positions = self.current.sample_positions(sdt);
        let positions = lerp_vec3_arrays(&prev_positions, &curr_positions, blend);

        let prev_rotations = self.previous.sample_rotations(sdt);
        let curr_rotations = self.current.sample_rotations(sdt);
        let rotations: Vec<Quat> = prev_rotations.iter().zip(&curr_rotations)
            .map(|(&a, &b)| a.slerp(b, blend))
            .collect();

        let prev_velocities = self.previous.sample_velocities(sdt);
        let curr_velocities = self.current.sample_velocities(sdt);
        let velocities = lerp_vec3_arrays(&prev_velocities, &curr_velocities, blend);

        let contacts_blend = {
            let pc = self.previous.sample_contacts(sdt);
            let cc = self.current.sample_contacts(sdt);
            pc.iter().zip(&cc).map(|(a, b)| a + (b - a) * blend).collect::<Vec<_>>()
        };

        // Apply to actor
        actor.root = root;

        // Blend positions: average of velocity-projected and sampled
        let n = positions.len().min(actor.transforms.len());
        for i in 0..n {
            let velocity_projected = actor.positions()[i] + velocities[i] * sdt;
            let blended_pos = Vec3::lerp(velocity_projected, positions[i], 0.5);
            let rotation = if i < rotations.len() { rotations[i] } else { Quat::IDENTITY };
            actor.transforms[i] = Mat4::from_rotation_translation(rotation, blended_pos);
        }

        // Update velocities
        let vel_n = velocities.len().min(actor.velocities.len());
        actor.velocities[..vel_n].copy_from_slice(&velocities[..vel_n]);

        // Restore bone constraints
        actor.restore_bone_lengths();

        // Leg IK: lock feet to ground during contact phases
        Self::solve_leg_ik(actor, &contacts_blend, self.num_bones);

        // Advance sequence timestamps
        for t in &mut self.previous.timestamps { *t -= sdt; }
        for t in &mut self.current.timestamps { *t -= sdt; }
    }

    /// Get the current contact states [4] (for visualization).
    pub fn contacts(&self) -> Vec<f32> {
        self.current.sample_contacts(0.0)
    }

    /// Get the current timescale.
    pub fn timescale(&self) -> f32 {
        self.timescale
    }

    /// Get the synchronization factor.
    pub fn synchronization(&self) -> f32 {
        self.synchronization
    }

    /// Check if a prediction is due.
    pub fn needs_prediction(&self) -> bool {
        self.prediction_timestamp == 0.0
            || self.total_time - self.prediction_timestamp > 1.0 / PREDICTION_FPS
    }

    // ── Normalization ───────────────────────────────────────

    /// Normalize input tensor: (x - mean) / std.
    fn normalize_input(&self, input: &[f32]) -> Vec<f32> {
        match (&self.metadata.input_mean, &self.metadata.input_std) {
            (Some(mean), Some(std)) if mean.len() == input.len() && std.len() == input.len() => {
                input.iter().enumerate().map(|(i, &x)| {
                    let s = std[i];
                    if s.abs() > 1e-8 { (x - mean[i]) / s } else { 0.0 }
                }).collect()
            }
            _ => input.to_vec(),
        }
    }

    /// Denormalize output tensor: x * std + mean.
    /// Output shape is [num_frames, output_dim] — stats are per-frame (output_dim length).
    fn denormalize_output(&self, output: &[f32]) -> Vec<f32> {
        match (&self.metadata.output_mean, &self.metadata.output_std) {
            (Some(mean), Some(std)) if !mean.is_empty() && !std.is_empty() => {
                let frame_dim = mean.len();
                output.iter().enumerate().map(|(i, &x)| {
                    let j = i % frame_dim;
                    x * std[j] + mean[j]
                }).collect()
            }
            _ => output.to_vec(),
        }
    }

    // ── Leg IK with contact locking ─────────────────────────

    /// Apply simple leg IK to lock feet when in contact with ground.
    fn solve_leg_ik(actor: &mut Actor, contacts: &[f32], num_bones: usize) {
        // Contact indices correspond to: LeftFoot(2), LeftToeBase(3), RightFoot(7), RightToeBase(8)
        // in the 23-bone Geno skeleton layout.
        let contact_bones: [(usize, usize, f32); 4] = [
            (2, 2, contacts.get(0).copied().unwrap_or(0.0)),  // LeftFoot
            (3, 3, contacts.get(1).copied().unwrap_or(0.0)),  // LeftToeBase
            (7, 7, contacts.get(2).copied().unwrap_or(0.0)),  // RightFoot
            (8, 8, contacts.get(3).copied().unwrap_or(0.0)),  // RightToeBase
        ];

        for &(bone_idx, _target_idx, contact) in &contact_bones {
            if bone_idx >= num_bones || bone_idx >= actor.transforms.len() {
                continue;
            }
            if contact > 0.5 {
                // Lock foot to ground plane (Y=0)
                let mut pos = actor.transforms[bone_idx].get_position();
                let ground_y = 0.0;
                pos.y = pos.y.min(ground_y);
                actor.transforms[bone_idx].set_position(pos);
            }
        }
    }
}

// ── Utility functions ───────────────────────────────────────

/// Exponential decay interpolation: value += (target - value) * (1 - exp(-sensitivity * dt)).
fn interpolate_dt(current: f32, target: f32, dt: f32, sensitivity: f32) -> f32 {
    current + (target - current) * (1.0 - (-sensitivity * dt).exp())
}

/// Lerp two Vec3 arrays element-wise.
fn lerp_vec3_arrays(a: &[Vec3], b: &[Vec3], t: f32) -> Vec<Vec3> {
    a.iter().zip(b.iter())
        .map(|(&va, &vb)| Vec3::lerp(va, vb, t))
        .collect()
}

/// Build a quaternion from look-at z and y vectors.
fn quat_from_look(forward: Vec3, up: Vec3) -> Quat {
    let z = forward.normalize_or_zero();
    let x = up.cross(z).normalize_or_zero();
    let y = z.cross(x).normalize_or_zero();
    Quat::from_mat3(&glam::Mat3::from_cols(x, y, z))
}

/// Build a transform looking along a direction from a position.
fn look_at_transform(position: Vec3, forward: Vec3) -> Mat4 {
    let z = forward.normalize_or_zero();
    let x = Vec3::Y.cross(z).normalize_or_zero();
    let y = z.cross(x).normalize_or_zero();
    Mat4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        position.extend(1.0),
    )
}
