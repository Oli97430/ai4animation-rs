//! Skeletal hierarchy (bone names + parent relationships).

use std::collections::HashMap;

/// Skeletal hierarchy mapping bone names to indices and parents.
#[derive(Clone, Debug)]
pub struct Hierarchy {
    pub bone_names: Vec<String>,
    pub parent_indices: Vec<i32>,
    name_to_index: HashMap<String, usize>,
}

impl Hierarchy {
    pub fn new(bone_names: Vec<String>, parent_indices: Vec<i32>) -> Self {
        let name_to_index = bone_names.iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();
        Self { bone_names, parent_indices, name_to_index }
    }

    pub fn num_joints(&self) -> usize {
        self.bone_names.len()
    }

    pub fn get_bone_index(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }

    pub fn get_bone_name(&self, index: usize) -> Option<&str> {
        self.bone_names.get(index).map(|s| s.as_str())
    }

    pub fn get_parent_index(&self, index: usize) -> i32 {
        self.parent_indices.get(index).copied().unwrap_or(-1)
    }

    pub fn is_root(&self, index: usize) -> bool {
        self.get_parent_index(index) < 0
    }

    pub fn get_children(&self, index: usize) -> Vec<usize> {
        self.parent_indices.iter()
            .enumerate()
            .filter(|(_, &p)| p == index as i32)
            .map(|(i, _)| i)
            .collect()
    }

    /// Find bone symmetry mapping (left <-> right).
    pub fn detect_symmetry(&self) -> Vec<usize> {
        let n = self.bone_names.len();
        let mut symmetry = (0..n).collect::<Vec<_>>();

        for i in 0..n {
            let name = &self.bone_names[i];
            let mirror_name = Self::mirror_name(name);
            if let Some(&j) = self.name_to_index.get(&mirror_name) {
                symmetry[i] = j;
            }
        }
        symmetry
    }

    fn mirror_name(name: &str) -> String {
        // Order matters — try the most specific patterns first
        let replacements = [
            // Full word (common in Mixamo, BVH)
            ("Left", "Right"), ("Right", "Left"),
            ("left", "right"), ("right", "left"),
            // Prefix (FBX style)
            ("L_", "R_"), ("R_", "L_"),
            ("l_", "r_"), ("r_", "l_"),
            // Infix
            ("_L_", "_R_"), ("_R_", "_L_"),
            ("_l_", "_r_"), ("_r_", "_l_"),
            // Suffix (Blender style)
            (".L", ".R"), (".R", ".L"),
            (".l", ".r"), (".r", ".l"),
            ("_L", "_R"), ("_R", "_L"),
            // Mixamo-style lowercase prefix
            ("lShldr", "rShldr"), ("rShldr", "lShldr"),
            ("lForeArm", "rForeArm"), ("rForeArm", "lForeArm"),
            ("lHand", "rHand"), ("rHand", "lHand"),
            ("lThigh", "rThigh"), ("rThigh", "lThigh"),
            ("lShin", "rShin"), ("rShin", "lShin"),
            ("lFoot", "rFoot"), ("rFoot", "lFoot"),
        ];
        let mut result = name.to_string();
        for (from, to) in &replacements {
            if result.contains(from) {
                result = result.replacen(from, to, 1);
                break;
            }
        }
        result
    }

    /// Get all bone names that have a symmetric counterpart.
    pub fn symmetric_pairs(&self) -> Vec<(usize, usize)> {
        let symmetry = self.detect_symmetry();
        let mut pairs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (i, &j) in symmetry.iter().enumerate() {
            if i != j && !seen.contains(&(i.min(j), i.max(j))) {
                pairs.push((i, j));
                seen.insert((i.min(j), i.max(j)));
            }
        }
        pairs
    }
}
