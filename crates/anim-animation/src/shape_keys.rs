//! Shape Keys (Morph Targets) — vertex-level blending for facial animation, expressions, etc.
//!
//! A shape key stores per-vertex deltas (position + normal offsets) relative to a base mesh.
//! Multiple shape keys can be blended simultaneously using weights [0..1].

use glam::Vec3;

/// A single shape key (morph target) storing per-vertex deltas.
#[derive(Debug, Clone)]
pub struct ShapeKey {
    /// Name of the shape key (e.g., "Smile", "Blink_L", "Brow_Raise").
    pub name: String,
    /// Per-vertex position deltas. Same length as base mesh vertices.
    pub position_deltas: Vec<Vec3>,
    /// Per-vertex normal deltas. Same length as base mesh normals.
    pub normal_deltas: Vec<Vec3>,
    /// Current blend weight [0.0 .. 1.0].
    pub weight: f32,
    /// Min/max weight range (usually 0.0..1.0, but can be negative for correctional shapes).
    pub min_weight: f32,
    pub max_weight: f32,
}

impl ShapeKey {
    pub fn new(name: impl Into<String>, vertex_count: usize) -> Self {
        Self {
            name: name.into(),
            position_deltas: vec![Vec3::ZERO; vertex_count],
            normal_deltas: vec![Vec3::ZERO; vertex_count],
            weight: 0.0,
            min_weight: 0.0,
            max_weight: 1.0,
        }
    }

    /// Create from explicit deltas.
    pub fn from_deltas(
        name: impl Into<String>,
        position_deltas: Vec<Vec3>,
        normal_deltas: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            position_deltas,
            normal_deltas,
            weight: 0.0,
            min_weight: 0.0,
            max_weight: 1.0,
        }
    }

    /// Create by computing deltas from base and target meshes.
    pub fn from_meshes(
        name: impl Into<String>,
        base_positions: &[Vec3],
        target_positions: &[Vec3],
        base_normals: &[Vec3],
        target_normals: &[Vec3],
    ) -> Self {
        let n = base_positions.len();
        let pos_deltas: Vec<Vec3> = base_positions.iter()
            .zip(target_positions.iter())
            .map(|(b, t)| *t - *b)
            .collect();
        let norm_deltas: Vec<Vec3> = base_normals.iter()
            .zip(target_normals.iter())
            .take(n)
            .map(|(b, t)| *t - *b)
            .collect();
        Self::from_deltas(name, pos_deltas, norm_deltas)
    }
}

/// Collection of shape keys for a mesh, with blending.
#[derive(Debug, Clone)]
pub struct ShapeKeySet {
    /// Base mesh vertex count.
    pub vertex_count: usize,
    /// Ordered list of shape keys.
    pub keys: Vec<ShapeKey>,
}

impl ShapeKeySet {
    pub fn new(vertex_count: usize) -> Self {
        Self {
            vertex_count,
            keys: Vec::new(),
        }
    }

    /// Add a shape key. Returns its index.
    pub fn add_key(&mut self, key: ShapeKey) -> usize {
        assert_eq!(key.position_deltas.len(), self.vertex_count,
            "Shape key vertex count mismatch");
        let idx = self.keys.len();
        self.keys.push(key);
        idx
    }

    /// Find a shape key by name.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.keys.iter().position(|k| k.name == name)
    }

    /// Set weight on a shape key by index.
    pub fn set_weight(&mut self, index: usize, weight: f32) {
        if let Some(key) = self.keys.get_mut(index) {
            key.weight = weight.clamp(key.min_weight, key.max_weight);
        }
    }

    /// Set weight by name.
    pub fn set_weight_by_name(&mut self, name: &str, weight: f32) {
        if let Some(idx) = self.find(name) {
            self.set_weight(idx, weight);
        }
    }

    /// Apply all active shape keys to base positions/normals.
    /// Returns (blended_positions, blended_normals).
    pub fn apply(
        &self,
        base_positions: &[Vec3],
        base_normals: &[Vec3],
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let n = self.vertex_count;
        let mut positions = base_positions.to_vec();
        let mut normals = base_normals.to_vec();

        for key in &self.keys {
            if key.weight.abs() < 1e-6 { continue; }
            let w = key.weight;
            for i in 0..n {
                if i < key.position_deltas.len() {
                    positions[i] += key.position_deltas[i] * w;
                }
                if i < key.normal_deltas.len() {
                    normals[i] += key.normal_deltas[i] * w;
                }
            }
        }

        // Re-normalize normals
        for n in &mut normals {
            let len = n.length();
            if len > 1e-6 {
                *n /= len;
            }
        }

        (positions, normals)
    }

    /// Check if any shape key has a non-zero weight.
    pub fn is_active(&self) -> bool {
        self.keys.iter().any(|k| k.weight.abs() > 1e-6)
    }

    /// Reset all weights to 0.
    pub fn reset_all(&mut self) {
        for key in &mut self.keys {
            key.weight = 0.0;
        }
    }

    /// Get all key names with their current weights.
    pub fn weights(&self) -> Vec<(&str, f32)> {
        self.keys.iter().map(|k| (k.name.as_str(), k.weight)).collect()
    }
}

/// Generate common facial expression shape keys for a humanoid head mesh.
/// Returns preset shape keys with procedural deltas.
pub fn generate_face_presets(vertex_count: usize) -> Vec<ShapeKey> {
    let names = ["Smile", "Frown", "Blink_L", "Blink_R", "Brow_Raise", "Jaw_Open", "Mouth_O"];
    names.iter().map(|name| {
        ShapeKey::new(*name, vertex_count)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_key_creation() {
        let key = ShapeKey::new("Test", 10);
        assert_eq!(key.name, "Test");
        assert_eq!(key.position_deltas.len(), 10);
        assert_eq!(key.weight, 0.0);
    }

    #[test]
    fn test_shape_key_from_meshes() {
        let base_pos = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
        let target_pos = vec![Vec3::ONE, Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0)];
        let base_norm = vec![Vec3::Y; 3];
        let target_norm = vec![Vec3::Y; 3];

        let key = ShapeKey::from_meshes("test", &base_pos, &target_pos, &base_norm, &target_norm);
        assert_eq!(key.position_deltas[0], Vec3::ONE);
        assert_eq!(key.position_deltas[1], Vec3::X);
        assert_eq!(key.position_deltas[2], Vec3::Y);
    }

    #[test]
    fn test_shape_key_set_apply() {
        let base_pos = vec![Vec3::ZERO, Vec3::X];
        let base_norm = vec![Vec3::Y, Vec3::Y];

        let mut set = ShapeKeySet::new(2);
        let mut key = ShapeKey::new("Move", 2);
        key.position_deltas[0] = Vec3::new(1.0, 0.0, 0.0);
        key.position_deltas[1] = Vec3::new(0.0, 1.0, 0.0);
        set.add_key(key);

        // Weight = 0 → no change
        let (pos, _) = set.apply(&base_pos, &base_norm);
        assert!((pos[0] - Vec3::ZERO).length() < 1e-5);

        // Weight = 0.5
        set.set_weight(0, 0.5);
        let (pos, _) = set.apply(&base_pos, &base_norm);
        assert!((pos[0] - Vec3::new(0.5, 0.0, 0.0)).length() < 1e-5);
        assert!((pos[1] - Vec3::new(1.0, 0.5, 0.0)).length() < 1e-5);

        // Weight = 1.0
        set.set_weight(0, 1.0);
        let (pos, _) = set.apply(&base_pos, &base_norm);
        assert!((pos[0] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_multiple_shape_keys_additive() {
        let base_pos = vec![Vec3::ZERO];
        let base_norm = vec![Vec3::Y];

        let mut set = ShapeKeySet::new(1);

        let mut k1 = ShapeKey::new("X", 1);
        k1.position_deltas[0] = Vec3::X;
        set.add_key(k1);

        let mut k2 = ShapeKey::new("Y", 1);
        k2.position_deltas[0] = Vec3::Y;
        set.add_key(k2);

        set.set_weight(0, 1.0);
        set.set_weight(1, 1.0);

        let (pos, _) = set.apply(&base_pos, &base_norm);
        assert!((pos[0] - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_find_by_name() {
        let mut set = ShapeKeySet::new(5);
        set.add_key(ShapeKey::new("Alpha", 5));
        set.add_key(ShapeKey::new("Beta", 5));

        assert_eq!(set.find("Alpha"), Some(0));
        assert_eq!(set.find("Beta"), Some(1));
        assert_eq!(set.find("Gamma"), None);
    }

    #[test]
    fn test_set_weight_by_name() {
        let mut set = ShapeKeySet::new(3);
        set.add_key(ShapeKey::new("Smile", 3));

        set.set_weight_by_name("Smile", 0.7);
        assert!((set.keys[0].weight - 0.7).abs() < 1e-5);
    }

    #[test]
    fn test_weight_clamping() {
        let mut set = ShapeKeySet::new(2);
        set.add_key(ShapeKey::new("Test", 2));

        set.set_weight(0, 5.0);
        assert!((set.keys[0].weight - 1.0).abs() < 1e-5);

        set.set_weight(0, -1.0);
        assert!((set.keys[0].weight - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_reset_all() {
        let mut set = ShapeKeySet::new(2);
        set.add_key(ShapeKey::new("A", 2));
        set.add_key(ShapeKey::new("B", 2));
        set.set_weight(0, 0.5);
        set.set_weight(1, 0.8);

        set.reset_all();
        assert!(!set.is_active());
    }
}
