//! GPU-accelerated motion matching using compute shaders.
//!
//! Mirrors the CPU [`MotionDatabase`](super::MotionDatabase) but stores features
//! in flat, GPU-friendly buffers and emits a WGSL compute shader for parallel
//! nearest-neighbour search. An [`InertializationBlender`] provides smooth
//! exponential-decay transitions between matched poses.

/// A result from the GPU (or CPU-fallback) motion matching search.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    /// Absolute frame index inside the flattened database.
    pub frame_index: usize,
    /// Weighted squared-difference cost.
    pub cost: f32,
    /// Index of the source clip that contains this frame.
    pub clip_index: usize,
}

// ── GpuMotionDatabase ────────────────────────────────────────

/// Stores motion features in GPU-friendly flat buffers.
///
/// Features are flattened row-major: element `[frame * feature_dim .. (frame+1) * feature_dim]`
/// holds the feature vector for that frame.
#[derive(Debug, Clone)]
pub struct GpuMotionDatabase {
    /// Flattened feature vectors (position, velocity, trajectory).
    pub features: Vec<f32>,
    /// Dimension of each feature vector.
    pub feature_dim: usize,
    /// Total frames in the database.
    pub num_frames: usize,
    /// `(start_frame, end_frame)` indices per clip (exclusive end).
    pub clip_boundaries: Vec<(usize, usize)>,
    /// Whether `build()` has been called after the last modification.
    built: bool,
    /// Staging buffer used while adding clips before `build()`.
    staging: Vec<Vec<f32>>,
    /// Per-clip staging so we can compute boundaries.
    clip_frame_counts: Vec<usize>,
}

impl GpuMotionDatabase {
    /// Create an empty database.
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
            feature_dim: 0,
            num_frames: 0,
            clip_boundaries: Vec::new(),
            built: false,
            staging: Vec::new(),
            clip_frame_counts: Vec::new(),
        }
    }

    /// Add a clip as a slice of per-frame feature vectors.
    ///
    /// Every inner `Vec<f32>` must have the same length; the first clip added
    /// sets `feature_dim` for the entire database.
    pub fn add_clip(&mut self, features: &[Vec<f32>]) {
        if features.is_empty() {
            return;
        }

        let dim = features[0].len();
        if self.feature_dim == 0 {
            self.feature_dim = dim;
        }
        assert_eq!(
            dim, self.feature_dim,
            "Feature dimension mismatch: expected {}, got {}",
            self.feature_dim, dim
        );

        self.clip_frame_counts.push(features.len());
        for f in features {
            assert_eq!(f.len(), dim, "All frames must have the same feature dimension");
            self.staging.push(f.clone());
        }
        self.built = false;
    }

    /// Flatten staging data into the final GPU-friendly buffers.
    pub fn build(&mut self) {
        self.features.clear();
        self.clip_boundaries.clear();
        self.num_frames = 0;

        let mut offset = 0usize;
        for &count in &self.clip_frame_counts {
            self.clip_boundaries.push((offset, offset + count));
            offset += count;
        }
        self.num_frames = offset;

        self.features.reserve(self.num_frames * self.feature_dim);
        for frame in &self.staging {
            self.features.extend_from_slice(frame);
        }

        self.built = true;
    }

    /// Pack into a [`GpuMatchData`] ready for upload.
    ///
    /// `weights` must have length `feature_dim`; if empty, uniform weights of 1.0
    /// are used. `query` must also have length `feature_dim`.
    pub fn to_shader_data(&self) -> GpuMatchData {
        assert!(self.built, "Call build() before to_shader_data()");

        GpuMatchData {
            feature_buffer: self.features.clone(),
            weight_buffer: vec![1.0; self.feature_dim],
            query_buffer: vec![0.0; self.feature_dim],
            result_buffer: vec![0u32; self.num_frames],
            num_candidates: self.num_frames as u32,
            feature_dim: self.feature_dim as u32,
        }
    }

    /// Look up which clip a given absolute frame index belongs to.
    pub fn clip_for_frame(&self, frame_index: usize) -> Option<usize> {
        self.clip_boundaries
            .iter()
            .position(|&(start, end)| frame_index >= start && frame_index < end)
    }
}

// ── GpuMatchData ─────────────────────────────────────────────

/// GPU buffer layout for the motion matching compute dispatch.
#[derive(Debug, Clone)]
pub struct GpuMatchData {
    /// Packed feature vectors (row-major, `num_candidates * feature_dim`).
    pub feature_buffer: Vec<f32>,
    /// Per-dimension weights (`feature_dim` elements).
    pub weight_buffer: Vec<f32>,
    /// Current query vector (`feature_dim` elements).
    pub query_buffer: Vec<f32>,
    /// Output: per-frame cost packed as `u32` (bit-cast from `f32`) for atomic ops.
    pub result_buffer: Vec<u32>,
    /// Number of candidate frames.
    pub num_candidates: u32,
    /// Dimension of each feature vector.
    pub feature_dim: u32,
}

// ── GpuMotionMatcher ─────────────────────────────────────────

/// Orchestrates GPU-accelerated motion matching.
pub struct GpuMotionMatcher {
    /// The underlying feature database.
    pub database: GpuMotionDatabase,
    /// Per-dimension feature weights (length = `feature_dim`).
    pub weights: Vec<f32>,
    /// Number of best matches to return.
    pub top_k: usize,
    /// Maximum cost threshold; candidates above this are discarded.
    pub search_radius: f32,
}

impl GpuMotionMatcher {
    /// Create a matcher from a **built** database.
    pub fn new(database: GpuMotionDatabase) -> Self {
        let dim = database.feature_dim;
        Self {
            database,
            weights: vec![1.0; dim],
            top_k: 8,
            search_radius: f32::MAX,
        }
    }

    /// CPU-fallback brute-force search. Returns up to `top_k` results sorted
    /// by ascending cost, filtered by `search_radius`.
    pub fn find_matches(&self, query: &[f32]) -> Vec<MatchResult> {
        assert_eq!(
            query.len(),
            self.database.feature_dim,
            "Query dimension mismatch"
        );

        let dim = self.database.feature_dim;
        let mut results: Vec<MatchResult> = Vec::with_capacity(self.database.num_frames);

        for frame in 0..self.database.num_frames {
            let base = frame * dim;
            let mut cost = 0.0f32;
            for d in 0..dim {
                let diff = query[d] - self.database.features[base + d];
                let w = if d < self.weights.len() {
                    self.weights[d]
                } else {
                    1.0
                };
                cost += w * diff * diff;
            }

            if cost <= self.search_radius {
                let clip_index = self.database.clip_for_frame(frame).unwrap_or(0);
                results.push(MatchResult {
                    frame_index: frame,
                    cost,
                    clip_index,
                });
            }
        }

        results.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(self.top_k);
        results
    }

    /// Generate the WGSL compute shader source.
    pub fn generate_compute_shader() -> String {
        MOTION_MATCH_WGSL.to_string()
    }

    /// Prepare a [`GpuMatchData`] struct ready for GPU dispatch.
    pub fn prepare_dispatch(&self, query: &[f32]) -> GpuMatchData {
        assert_eq!(
            query.len(),
            self.database.feature_dim,
            "Query dimension mismatch"
        );

        let mut data = self.database.to_shader_data();
        data.query_buffer = query.to_vec();

        // Copy matcher weights into the buffer.
        for (i, w) in data.weight_buffer.iter_mut().enumerate() {
            if i < self.weights.len() {
                *w = self.weights[i];
            }
        }

        // Initialize result buffer to u32::MAX (bit pattern of +inf for f32 costs).
        for r in data.result_buffer.iter_mut() {
            *r = u32::MAX;
        }

        data
    }
}

// ── WGSL Compute Shader ──────────────────────────────────────

/// WGSL compute shader for parallel motion matching.
///
/// Each invocation computes the weighted squared-distance cost between the
/// query vector and one database frame. Results are written to a storage
/// buffer so the CPU (or a follow-up reduction pass) can extract the top-K.
pub const MOTION_MATCH_WGSL: &str = r#"
// ── Motion Matching Compute Shader ──────────────────────────
// Workgroup size: 64 threads, each evaluates one candidate frame.

struct Params {
    num_candidates: u32,
    feature_dim: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> features: array<f32>;
@group(0) @binding(2) var<storage, read> query: array<f32>;
@group(0) @binding(3) var<storage, read> weights: array<f32>;
@group(0) @binding(4) var<storage, read_write> results: array<u32>;

// Shared memory for workgroup-local reduction.
var<workgroup> local_costs: array<f32, 64>;
var<workgroup> local_indices: array<u32, 64>;

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let frame_idx = global_id.x;
    let lid = local_id.x;

    // Guard: frames beyond the database are ignored.
    if frame_idx >= params.num_candidates {
        local_costs[lid] = bitcast<f32>(0x7F800000u); // +inf
        local_indices[lid] = 0xFFFFFFFFu;
        workgroupBarrier();
        return;
    }

    // Compute weighted squared-difference cost.
    let base = frame_idx * params.feature_dim;
    var cost: f32 = 0.0;
    for (var d: u32 = 0u; d < params.feature_dim; d = d + 1u) {
        let diff = query[d] - features[base + d];
        cost = cost + weights[d] * diff * diff;
    }

    // Store into shared memory for local reduction.
    local_costs[lid] = cost;
    local_indices[lid] = frame_idx;
    workgroupBarrier();

    // Workgroup reduction: find minimum cost within the workgroup.
    var stride: u32 = 32u;
    loop {
        if stride == 0u {
            break;
        }
        if lid < stride && (lid + stride) < 64u {
            if local_costs[lid + stride] < local_costs[lid] {
                local_costs[lid] = local_costs[lid + stride];
                local_indices[lid] = local_indices[lid + stride];
            }
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }

    // Write per-frame cost to results buffer for CPU top-K extraction.
    if frame_idx < params.num_candidates {
        results[frame_idx] = bitcast<u32>(cost);
    }
}
"#;

// ── InertializationBlender ───────────────────────────────────

/// Smooth pose transitions using inertialization (exponential decay).
///
/// Instead of a linear crossfade, inertialization models the offset between
/// the source and target pose as a decaying spring, producing more natural
/// motion without the "averaging" artifacts of linear blending.
#[derive(Debug, Clone)]
pub struct InertializationBlender {
    /// Total blend time (seconds).
    pub blend_time: f32,
    /// Elapsed time into the current blend.
    pub current_blend: f32,
    /// Source pose (flattened joint data).
    pub source_pose: Vec<f32>,
    /// Target pose (flattened joint data).
    pub target_pose: Vec<f32>,
    /// Cached offset at transition start (`source - target`).
    offset: Vec<f32>,
    /// Cached velocity at transition start.
    velocity: Vec<f32>,
    /// Half-life for the exponential decay (seconds). Smaller = snappier.
    half_life: f32,
}

impl InertializationBlender {
    /// Create a new blender with the given half-life.
    pub fn new(half_life: f32) -> Self {
        Self {
            blend_time: 0.0,
            current_blend: 0.0,
            source_pose: Vec::new(),
            target_pose: Vec::new(),
            offset: Vec::new(),
            velocity: Vec::new(),
            half_life: half_life.max(1e-4),
        }
    }

    /// Start a transition from `from` to `to` over `duration` seconds.
    pub fn start_transition(&mut self, from: &[f32], to: &[f32], duration: f32) {
        assert_eq!(from.len(), to.len(), "Pose dimensions must match");

        self.source_pose = from.to_vec();
        self.target_pose = to.to_vec();
        self.blend_time = duration.max(1e-4);
        self.current_blend = 0.0;

        // The offset is `source - target`; we will decay this to zero.
        self.offset = from
            .iter()
            .zip(to.iter())
            .map(|(&s, &t)| s - t)
            .collect();

        // Initial velocity is zero (instantaneous switch, smoothed by decay).
        self.velocity = vec![0.0; from.len()];
    }

    /// Advance the blend by `dt` seconds and return the interpolated pose.
    ///
    /// The returned pose starts at `source_pose` and decays toward `target_pose`
    /// using exponential inertialization.
    pub fn update(&mut self, dt: f32) -> Vec<f32> {
        if self.target_pose.is_empty() {
            return self.source_pose.clone();
        }

        self.current_blend += dt;

        if self.is_complete() {
            return self.target_pose.clone();
        }

        // Decay factor: exp(-ln(2) / half_life * t)
        // We use the spring-damper formulation from Inertialization of
        // All The Things (GDC 2018, David Bollo).
        let decay = self.decay_factor();

        let mut result = Vec::with_capacity(self.target_pose.len());
        for i in 0..self.target_pose.len() {
            // Decayed offset added to target.
            let blended = self.target_pose[i] + self.offset[i] * decay;
            result.push(blended);
        }

        result
    }

    /// Whether the transition has finished.
    pub fn is_complete(&self) -> bool {
        self.current_blend >= self.blend_time
    }

    /// Exponential decay factor for the current elapsed time.
    fn decay_factor(&self) -> f32 {
        let ln2 = std::f32::consts::LN_2;
        let lambda = ln2 / self.half_life;
        (-lambda * self.current_blend).exp()
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── GpuMotionDatabase ────────────────────────────────────

    #[test]
    fn test_empty_database() {
        let db = GpuMotionDatabase::new();
        assert_eq!(db.num_frames, 0);
        assert_eq!(db.feature_dim, 0);
        assert!(db.clip_boundaries.is_empty());
    }

    #[test]
    fn test_add_single_clip_and_build() {
        let mut db = GpuMotionDatabase::new();
        let clip = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        db.add_clip(&clip);
        db.build();

        assert_eq!(db.feature_dim, 3);
        assert_eq!(db.num_frames, 2);
        assert_eq!(db.clip_boundaries, vec![(0, 2)]);
        assert_eq!(db.features, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_add_multiple_clips() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[vec![1.0, 0.0], vec![2.0, 0.0], vec![3.0, 0.0]]);
        db.add_clip(&[vec![10.0, 0.0], vec![20.0, 0.0]]);
        db.build();

        assert_eq!(db.num_frames, 5);
        assert_eq!(db.clip_boundaries, vec![(0, 3), (3, 5)]);
        assert_eq!(db.clip_for_frame(0), Some(0));
        assert_eq!(db.clip_for_frame(2), Some(0));
        assert_eq!(db.clip_for_frame(3), Some(1));
        assert_eq!(db.clip_for_frame(4), Some(1));
        assert_eq!(db.clip_for_frame(5), None);
    }

    #[test]
    fn test_add_empty_clip_is_noop() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[]);
        db.build();
        assert_eq!(db.num_frames, 0);
    }

    #[test]
    #[should_panic(expected = "Feature dimension mismatch")]
    fn test_dimension_mismatch_panics() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[vec![1.0, 2.0]]);
        db.add_clip(&[vec![1.0, 2.0, 3.0]]); // wrong dimension
    }

    // ── GpuMatchData / to_shader_data ────────────────────────

    #[test]
    fn test_to_shader_data() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        db.build();

        let data = db.to_shader_data();
        assert_eq!(data.num_candidates, 2);
        assert_eq!(data.feature_dim, 2);
        assert_eq!(data.feature_buffer.len(), 4);
        assert_eq!(data.weight_buffer, vec![1.0, 1.0]);
        assert_eq!(data.query_buffer, vec![0.0, 0.0]);
    }

    // ── GpuMotionMatcher ─────────────────────────────────────

    #[test]
    fn test_find_matches_exact() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]]);
        db.build();

        let matcher = GpuMotionMatcher::new(db);
        let results = matcher.find_matches(&[1.0, 0.0]);

        assert!(!results.is_empty());
        // Frame 1 has features [1.0, 0.0], so cost should be 0.
        assert_eq!(results[0].frame_index, 1);
        assert!((results[0].cost).abs() < 1e-6);
    }

    #[test]
    fn test_find_matches_top_k() {
        let mut db = GpuMotionDatabase::new();
        // 20 frames, but top_k defaults to 8.
        let frames: Vec<Vec<f32>> = (0..20).map(|i| vec![i as f32]).collect();
        db.add_clip(&frames);
        db.build();

        let matcher = GpuMotionMatcher::new(db);
        let results = matcher.find_matches(&[5.0]);
        assert!(results.len() <= 8);
        // Best match should be frame 5 (cost = 0).
        assert_eq!(results[0].frame_index, 5);
    }

    #[test]
    fn test_find_matches_with_weights() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[vec![1.0, 0.0], vec![0.0, 1.0]]);
        db.build();

        let mut matcher = GpuMotionMatcher::new(db);
        // Weight the second dimension very heavily.
        matcher.weights = vec![0.0, 100.0];

        let results = matcher.find_matches(&[0.0, 0.0]);
        // Frame 0 has [1.0, 0.0] -> cost = 0*1 + 100*0 = 0
        // Frame 1 has [0.0, 1.0] -> cost = 0*0 + 100*1 = 100
        assert_eq!(results[0].frame_index, 0);
        assert!(results[0].cost < 1e-6);
    }

    #[test]
    fn test_search_radius_filtering() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[vec![0.0], vec![100.0]]);
        db.build();

        let mut matcher = GpuMotionMatcher::new(db);
        matcher.search_radius = 1.0;

        let results = matcher.find_matches(&[0.0]);
        // Only frame 0 (cost 0) is within radius; frame 1 cost = 10000.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].frame_index, 0);
    }

    #[test]
    fn test_prepare_dispatch() {
        let mut db = GpuMotionDatabase::new();
        db.add_clip(&[vec![1.0, 2.0]]);
        db.build();

        let mut matcher = GpuMotionMatcher::new(db);
        matcher.weights = vec![2.0, 3.0];

        let data = matcher.prepare_dispatch(&[5.0, 6.0]);
        assert_eq!(data.query_buffer, vec![5.0, 6.0]);
        assert_eq!(data.weight_buffer, vec![2.0, 3.0]);
        // Result buffer should be initialized to u32::MAX.
        assert!(data.result_buffer.iter().all(|&r| r == u32::MAX));
    }

    #[test]
    fn test_generate_compute_shader_not_empty() {
        let shader = GpuMotionMatcher::generate_compute_shader();
        assert!(shader.contains("@compute"));
        assert!(shader.contains("@workgroup_size(64)"));
        assert!(shader.contains("features"));
        assert!(shader.contains("query"));
    }

    // ── InertializationBlender ───────────────────────────────

    #[test]
    fn test_blender_start_at_source() {
        let mut blender = InertializationBlender::new(0.1);
        blender.start_transition(&[0.0, 10.0], &[100.0, 110.0], 1.0);

        // At t=0 the pose should be very close to the source.
        let pose = blender.update(0.0);
        assert!((pose[0] - 0.0).abs() < 0.01);
        assert!((pose[1] - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_blender_converges_to_target() {
        let mut blender = InertializationBlender::new(0.05);
        blender.start_transition(&[0.0], &[100.0], 2.0);

        // Run well past the blend time.
        let pose = blender.update(3.0);
        assert_eq!(pose, vec![100.0]); // is_complete triggers exact target
    }

    #[test]
    fn test_blender_is_complete() {
        let mut blender = InertializationBlender::new(0.1);
        blender.start_transition(&[0.0], &[1.0], 0.5);

        assert!(!blender.is_complete());
        blender.update(0.3);
        assert!(!blender.is_complete());
        blender.update(0.3);
        assert!(blender.is_complete());
    }

    #[test]
    fn test_blender_empty_poses() {
        let mut blender = InertializationBlender::new(0.1);
        // No transition started; update returns empty source.
        let pose = blender.update(0.1);
        assert!(pose.is_empty());
    }

    #[test]
    fn test_blender_monotonic_approach() {
        let mut blender = InertializationBlender::new(0.1);
        blender.start_transition(&[0.0], &[100.0], 1.0);

        let mut prev = 0.0f32;
        for _ in 0..10 {
            let pose = blender.update(0.05);
            // The blended value should monotonically approach the target.
            assert!(
                pose[0] >= prev,
                "Expected monotonic increase: {} >= {}",
                pose[0],
                prev
            );
            prev = pose[0];
        }
    }
}
