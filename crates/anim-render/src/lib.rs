//! wgpu-based 3D renderer with camera, grid, debug draw, and skinned mesh.

pub mod renderer;
pub mod render_settings;
pub mod camera;
pub mod grid;
pub mod debug_draw;
pub mod skinned_mesh;
pub mod vertex;
pub mod capture;
pub mod primitive;
pub mod skybox;
pub mod lights;
pub mod video_export;
pub mod post_process;
pub mod instancing;

pub use renderer::SceneRenderer;
pub use render_settings::RenderSettings;
pub use camera::{Camera, CameraMode};
pub use debug_draw::DebugDraw;
pub use capture::{capture_texture_to_png, capture_texture_to_rgba, CaptureError};
pub use skybox::{SkyEnvironment, SkyMode};
pub use lights::{Light, LightType, LightScene};
pub use video_export::{VideoEncoder, VideoConfig, VideoFormat};
pub use post_process::PostProcessConfig;
pub use instancing::{InstanceBuffer, InstanceGroup, InstanceData};
