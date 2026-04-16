//! GUI panels for the animation editor (egui).

pub mod panels;
pub mod app_state;
pub mod theme;
pub mod scene_io;
pub mod shortcuts;
pub mod recent_files;

pub use app_state::AppState;
pub use theme::apply_theme;
pub use shortcuts::{ShortcutMap, Action};
