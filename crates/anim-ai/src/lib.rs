//! AI integration — local (Ollama) and cloud (OpenAI, Claude) providers.
//!
//! Everything in the editor is promptable: scene manipulation, animation
//! playback, file import, camera control, IK, rendering settings, etc.
//! The AI sees a structured description of the current scene and returns
//! structured commands that the editor executes.

pub mod provider;
pub mod ollama;
pub mod openai;
pub mod claude;
pub mod command;
pub mod context;
pub mod session;

pub use provider::{AiProvider, AiConfig, AiMessage, AiRole, ModelInfo};
pub use command::{AiCommand, CommandResult};
pub use context::SceneContext;
pub use session::AiSession;
