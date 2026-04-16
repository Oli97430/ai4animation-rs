//! General-purpose utility functions (mirrors Python Utility.py).

use glam::Vec3;

/// Linear remap: maps `value` from [in_min, in_max] to [out_min, out_max].
pub fn normalize(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    if (in_max - in_min).abs() < f32::EPSILON {
        return out_min;
    }
    out_min + (value - in_min) / (in_max - in_min) * (out_max - out_min)
}

/// Clamped ratio in [0, 1].
pub fn ratio(current: f32, start: f32, end: f32) -> f32 {
    if (end - start).abs() < f32::EPSILON {
        return 0.0;
    }
    ((current - start) / (end - start)).clamp(0.0, 1.0)
}

/// RGBA color with adjusted alpha.
pub fn opacity(color: [f32; 4], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

/// Build symmetry index mapping from joint names (Left↔Right swap).
pub fn symmetry_indices(joint_names: &[String]) -> Vec<usize> {
    let n = joint_names.len();
    let mut mapping: Vec<usize> = (0..n).collect();

    let replacements = [
        ("Left", "Right"), ("Right", "Left"),
        ("left", "right"), ("right", "left"),
        ("_l_", "_r_"), ("_r_", "_l_"),
        ("_L_", "_R_"), ("_R_", "_L_"),
        (".L", ".R"), (".R", ".L"),
        (".l", ".r"), (".r", ".l"),
        ("L_", "R_"), ("R_", "L_"),
        ("l_", "r_"), ("r_", "l_"),
    ];

    let lower_names: Vec<String> = joint_names.iter().map(|n| n.to_lowercase()).collect();

    for i in 0..n {
        let name = &joint_names[i];
        for (from, to) in &replacements {
            if name.contains(from) {
                let mirror = name.replacen(from, to, 1);
                if let Some(j) = joint_names.iter().position(|n| n == &mirror) {
                    mapping[i] = j;
                    break;
                }
                // Try case-insensitive
                let mirror_lower = mirror.to_lowercase();
                if let Some(j) = lower_names.iter().position(|n| n == &mirror_lower) {
                    mapping[i] = j;
                    break;
                }
            }
        }
    }
    mapping
}

/// Detect the center of mass from joint positions (average).
pub fn center_of_mass(positions: &[Vec3]) -> Vec3 {
    if positions.is_empty() { return Vec3::ZERO; }
    let sum: Vec3 = positions.iter().copied().sum();
    sum / positions.len() as f32
}

/// Compute bounding box (min, max) of positions.
pub fn bounding_box(positions: &[Vec3]) -> (Vec3, Vec3) {
    if positions.is_empty() { return (Vec3::ZERO, Vec3::ZERO); }
    let mut min = positions[0];
    let mut max = positions[0];
    for p in positions.iter().skip(1) {
        min = min.min(*p);
        max = max.max(*p);
    }
    (min, max)
}

/// Height of the character (max Y - min Y).
pub fn character_height(positions: &[Vec3]) -> f32 {
    let (min, max) = bounding_box(positions);
    max.y - min.y
}
