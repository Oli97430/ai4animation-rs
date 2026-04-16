//! Grid floor rendering.

/// Grid configuration.
pub struct GridConfig {
    pub size: f32,
    pub divisions: usize,
    pub visible: bool,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            size: 10.0,
            divisions: 20,
            visible: true,
        }
    }
}
