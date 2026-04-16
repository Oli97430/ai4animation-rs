//! Core ECS: Scene, Entity, Component, Time.

pub mod scene;
pub mod entity;
pub mod component;
pub mod time;
pub mod i18n;
pub mod profiler;
pub mod input;

pub use scene::Scene;
pub use entity::{Entity, EntityId};
pub use component::Component;
pub use time::Time;
pub use i18n::{Lang, t};
pub use profiler::{Profiler, ScopedTimer, StopWatch};
pub use input::InputState;
