//! Scene graph with contiguous transform storage.

use glam::{Mat4, Vec3, Quat};
use anim_math::transform::Transform;
use crate::entity::{Entity, EntityId};

/// The scene holds all entities and their transforms in contiguous arrays.
pub struct Scene {
    pub entities: Vec<Entity>,
    /// All entity transforms stored contiguously: [N, 4x4].
    pub transforms: Vec<Mat4>,
    /// Per-entity scale.
    pub scales: Vec<Vec3>,
    /// Currently selected entity.
    pub selected: Option<EntityId>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            transforms: Vec::new(),
            scales: Vec::new(),
            selected: None,
        }
    }

    /// Add a new entity, returns its ID.
    pub fn add_entity(
        &mut self,
        name: &str,
        position: Option<Vec3>,
        rotation: Option<Quat>,
        parent: Option<EntityId>,
    ) -> EntityId {
        let id = self.entities.len();
        let mut entity = Entity::new(id, name.to_string());

        // Build transform
        let pos = position.unwrap_or(Vec3::ZERO);
        let rot = rotation.unwrap_or(Quat::IDENTITY);
        let transform = Mat4::from_rotation_translation(rot, pos);

        // Set parent
        if let Some(parent_id) = parent {
            entity.parent = Some(parent_id);
            self.entities[parent_id].children.push(id);
            // Add to all ancestors' successors
            self.add_successor(parent_id, id);
        }

        self.entities.push(entity);
        self.transforms.push(transform);
        self.scales.push(Vec3::ONE);
        id
    }

    fn add_successor(&mut self, ancestor_id: EntityId, new_id: EntityId) {
        self.entities[ancestor_id].successors.push(new_id);
        if let Some(parent) = self.entities[ancestor_id].parent {
            self.add_successor(parent, new_id);
        }
    }

    /// Get entity by ID.
    pub fn get_entity(&self, id: EntityId) -> &Entity {
        &self.entities[id]
    }

    /// Get entity mutably by ID.
    pub fn get_entity_mut(&mut self, id: EntityId) -> &mut Entity {
        &mut self.entities[id]
    }

    /// Get transform for an entity.
    pub fn get_transform(&self, id: EntityId) -> Mat4 {
        self.transforms[id]
    }

    /// Set transform for an entity, optionally propagating FK to successors.
    pub fn set_transform(&mut self, id: EntityId, transform: Mat4, fk: bool) {
        let old = self.transforms[id];
        self.transforms[id] = transform;

        if fk {
            let delta = transform * old.inverse();
            let successors: Vec<EntityId> = self.entities[id].successors.clone();
            for &succ in &successors {
                self.transforms[succ] = delta * self.transforms[succ];
            }
        }
    }

    /// Get position of an entity.
    pub fn get_position(&self, id: EntityId) -> Vec3 {
        self.transforms[id].get_position()
    }

    /// Set position of an entity with FK.
    pub fn set_position(&mut self, id: EntityId, pos: Vec3, fk: bool) {
        let mut t = self.transforms[id];
        t.set_position(pos);
        self.set_transform(id, t, fk);
    }

    /// Get all root entities (no parent).
    pub fn get_roots(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|e| e.parent.is_none())
            .map(|e| e.id)
            .collect()
    }

    /// Find entity by name.
    pub fn find_entity(&self, name: &str) -> Option<EntityId> {
        self.entities.iter().find(|e| e.name == name).map(|e| e.id)
    }

    /// Get entity count.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Clear all entities.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.transforms.clear();
        self.scales.clear();
        self.selected = None;
    }

    /// Get a chain of entity IDs from source to target (via parents).
    pub fn get_chain(&self, source: EntityId, target: EntityId) -> Vec<EntityId> {
        // BFS from source to target through children
        let mut chain = vec![target];
        let mut current = target;
        while current != source {
            if let Some(parent) = self.entities[current].parent {
                chain.push(parent);
                current = parent;
            } else {
                break;
            }
        }
        chain.reverse();
        chain
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
