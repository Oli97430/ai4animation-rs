//! Skeleton retargeting — map joints between two skeletons by name.
//!
//! Allows applying animation from one skeleton onto a mesh skinned to
//! a different (but compatible) skeleton. Common use case: BVH/NPZ motion
//! data applied onto a GLB skinned character.

use glam::Mat4;

/// Mapping between a source skeleton (animation) and a target skeleton (mesh).
#[derive(Debug, Clone)]
pub struct RetargetMap {
    /// For each target joint index: the source joint index it maps to, or None.
    pub target_to_source: Vec<Option<usize>>,
    /// Source joint names (for display).
    pub source_names: Vec<String>,
    /// Target joint names (for display).
    pub target_names: Vec<String>,
    /// Number of successfully mapped joints.
    pub mapped_count: usize,
}

impl RetargetMap {
    /// Build a retarget map by matching joint names between source and target.
    ///
    /// Matching strategy (cascading):
    /// 1. Exact name match
    /// 2. Case-insensitive match
    /// 3. Partial match (source name contained in target or vice versa)
    /// 4. Common alias resolution (e.g., "Hips" ↔ "pelvis", "LeftUpLeg" ↔ "lThigh")
    pub fn build(source_names: &[String], target_names: &[String]) -> Self {
        let source_lower: Vec<String> = source_names.iter()
            .map(|n| n.to_lowercase())
            .collect();

        let mut target_to_source = Vec::with_capacity(target_names.len());
        let mut mapped = 0;

        for target_name in target_names {
            let target_lower = target_name.to_lowercase();

            // 1. Exact match
            let found = source_names.iter().position(|s| s == target_name)
                // 2. Case-insensitive match
                .or_else(|| source_lower.iter().position(|s| s == &target_lower))
                // 3. Partial match (target contains source or vice versa)
                .or_else(|| {
                    // Prefer exact substring over fuzzy
                    source_lower.iter().position(|s| {
                        target_lower.contains(s.as_str()) || s.contains(target_lower.as_str())
                    })
                })
                // 4. Alias resolution
                .or_else(|| {
                    let aliases = resolve_aliases(&target_lower);
                    for alias in aliases {
                        if let Some(idx) = source_lower.iter().position(|s| s == &alias) {
                            return Some(idx);
                        }
                    }
                    None
                });

            if found.is_some() { mapped += 1; }
            target_to_source.push(found);
        }

        Self {
            target_to_source,
            source_names: source_names.to_vec(),
            target_names: target_names.to_vec(),
            mapped_count: mapped,
        }
    }

    /// Apply source transforms onto target skeleton.
    /// Returns transforms for the target skeleton, using source where mapped,
    /// identity where not.
    pub fn apply(&self, source_transforms: &[Mat4], bind_poses: Option<&[Mat4]>) -> Vec<Mat4> {
        let n = self.target_names.len();
        let mut result = Vec::with_capacity(n);

        for i in 0..n {
            let transform = match self.target_to_source[i] {
                Some(src_idx) => {
                    if src_idx < source_transforms.len() {
                        source_transforms[src_idx]
                    } else {
                        bind_poses.and_then(|bp| bp.get(i).copied()).unwrap_or(Mat4::IDENTITY)
                    }
                }
                None => {
                    // No mapping: use bind pose or identity
                    bind_poses.and_then(|bp| bp.get(i).copied()).unwrap_or(Mat4::IDENTITY)
                }
            };
            result.push(transform);
        }

        result
    }

    /// Get mapping quality as a fraction [0, 1].
    pub fn quality(&self) -> f32 {
        if self.target_names.is_empty() { return 0.0; }
        self.mapped_count as f32 / self.target_names.len() as f32
    }

    /// Get unmapped target joint names.
    pub fn unmapped_targets(&self) -> Vec<&str> {
        self.target_to_source.iter()
            .enumerate()
            .filter(|(_, src)| src.is_none())
            .map(|(i, _)| self.target_names[i].as_str())
            .collect()
    }

    /// Manually set a mapping for a target joint.
    pub fn set_mapping(&mut self, target_idx: usize, source_idx: Option<usize>) {
        if target_idx < self.target_to_source.len() {
            let was_mapped = self.target_to_source[target_idx].is_some();
            let now_mapped = source_idx.is_some();
            self.target_to_source[target_idx] = source_idx;
            // Update count
            if was_mapped && !now_mapped { self.mapped_count -= 1; }
            if !was_mapped && now_mapped { self.mapped_count += 1; }
        }
    }
}

/// Common bone name aliases (bidirectional).
fn resolve_aliases(name: &str) -> Vec<String> {
    let alias_table: &[(&[&str], &[&str])] = &[
        // Hips / Pelvis
        (&["hips", "pelvis", "hip", "root"], &["hips", "pelvis", "hip", "root"]),
        // Spine
        (&["spine", "spine1", "spine2", "spine3"], &["spine", "spine1", "spine2", "spine3"]),
        // Chest
        (&["chest", "spine2", "upperchest"], &["chest", "spine2", "upperchest"]),
        // Head / Neck
        (&["head"], &["head"]),
        (&["neck", "neck1"], &["neck", "neck1"]),
        // Left leg
        (&["leftupleg", "lthigh", "l_thigh", "lefthip", "l_hip", "leftleg_upper"],
         &["leftupleg", "lthigh", "l_thigh", "lefthip", "l_hip"]),
        (&["leftleg", "lshin", "l_shin", "leftknee", "l_knee", "leftleg_lower"],
         &["leftleg", "lshin", "l_shin", "leftknee", "l_knee"]),
        (&["leftfoot", "lfoot", "l_foot", "leftankle", "l_ankle"],
         &["leftfoot", "lfoot", "l_foot", "leftankle", "l_ankle"]),
        (&["lefttoebase", "ltoe", "l_toe", "lefttoes"],
         &["lefttoebase", "ltoe", "l_toe", "lefttoes"]),
        // Right leg
        (&["rightupleg", "rthigh", "r_thigh", "righthip", "r_hip", "rightleg_upper"],
         &["rightupleg", "rthigh", "r_thigh", "righthip", "r_hip"]),
        (&["rightleg", "rshin", "r_shin", "rightknee", "r_knee", "rightleg_lower"],
         &["rightleg", "rshin", "r_shin", "rightknee", "r_knee"]),
        (&["rightfoot", "rfoot", "r_foot", "rightankle", "r_ankle"],
         &["rightfoot", "rfoot", "r_foot", "rightankle", "r_ankle"]),
        (&["righttoebase", "rtoe", "r_toe", "righttoes"],
         &["righttoebase", "rtoe", "r_toe", "righttoes"]),
        // Left arm
        (&["leftshoulder", "lshoulder", "l_shoulder", "leftclavicle", "l_clavicle"],
         &["leftshoulder", "lshoulder", "l_shoulder", "leftclavicle", "l_clavicle"]),
        (&["leftarm", "luparm", "l_uparm", "leftupperarm", "l_upperarm"],
         &["leftarm", "luparm", "l_uparm", "leftupperarm", "l_upperarm"]),
        (&["leftforearm", "lloarm", "l_loarm", "leftlowerarm", "l_lowerarm"],
         &["leftforearm", "lloarm", "l_loarm", "leftlowerarm", "l_lowerarm"]),
        (&["lefthand", "lhand", "l_hand", "leftwrist", "l_wrist"],
         &["lefthand", "lhand", "l_hand", "leftwrist", "l_wrist"]),
        // Right arm
        (&["rightshoulder", "rshoulder", "r_shoulder", "rightclavicle", "r_clavicle"],
         &["rightshoulder", "rshoulder", "r_shoulder", "rightclavicle", "r_clavicle"]),
        (&["rightarm", "ruparm", "r_uparm", "rightupperarm", "r_upperarm"],
         &["rightarm", "ruparm", "r_uparm", "rightupperarm", "r_upperarm"]),
        (&["rightforearm", "rloarm", "r_loarm", "rightlowerarm", "r_lowerarm"],
         &["rightforearm", "rloarm", "r_loarm", "rightlowerarm", "r_lowerarm"]),
        (&["righthand", "rhand", "r_hand", "rightwrist", "r_wrist"],
         &["righthand", "rhand", "r_hand", "rightwrist", "r_wrist"]),
    ];

    let mut result = Vec::new();
    for (group_a, group_b) in alias_table {
        if group_a.contains(&name) {
            for &alias in *group_b {
                if alias != name {
                    result.push(alias.to_string());
                }
            }
        }
    }
    result
}

/// Build a retarget map from loaded model data.
pub fn build_retarget(
    animation_joint_names: &[String],
    mesh_joint_names: &[String],
) -> RetargetMap {
    RetargetMap::build(animation_joint_names, mesh_joint_names)
}
