//! Component trait for the ECS system.

/// Marker trait for components.
/// Components are stored as concrete types via Entity's type-erased HashMap.
pub trait Component: 'static {
    fn type_name(&self) -> &'static str;
}
