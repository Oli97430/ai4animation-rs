//! Entity with parent/child hierarchy and component storage.

use std::any::{Any, TypeId};
use std::collections::HashMap;
/// Unique identifier for an entity (index into Scene arrays).
pub type EntityId = usize;

/// An entity in the scene hierarchy.
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub parent: Option<EntityId>,
    pub children: Vec<EntityId>,
    /// All descendant indices (for FK propagation).
    pub successors: Vec<EntityId>,
    /// Type-erased component storage.
    components: HashMap<TypeId, Box<dyn Any>>,
    pub visible: bool,
}

impl Entity {
    pub fn new(id: EntityId, name: String) -> Self {
        Self {
            id,
            name,
            parent: None,
            children: Vec::new(),
            successors: Vec::new(),
            components: HashMap::new(),
            visible: true,
        }
    }

    /// Add a component of type T.
    pub fn add_component<T: 'static>(&mut self, component: T) {
        self.components.insert(TypeId::of::<T>(), Box::new(component));
    }

    /// Get a reference to a component of type T.
    pub fn get_component<T: 'static>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|c| c.downcast_ref::<T>())
    }

    /// Get a mutable reference to a component of type T.
    pub fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.components
            .get_mut(&TypeId::of::<T>())
            .and_then(|c| c.downcast_mut::<T>())
    }

    /// Check if entity has a component of type T.
    pub fn has_component<T: 'static>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    /// Remove a component of type T.
    pub fn remove_component<T: 'static>(&mut self) -> Option<T> {
        self.components
            .remove(&TypeId::of::<T>())
            .and_then(|c| c.downcast::<T>().ok())
            .map(|c| *c)
    }
}
