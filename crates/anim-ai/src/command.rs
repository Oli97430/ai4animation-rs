//! AI command dispatch — structured actions the AI can execute on the editor.

use serde::{Deserialize, Serialize};

/// A command the AI wants to execute on the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AiCommand {
    // ── File operations ────────────────────────────
    ImportFile { path: String },
    ExportFrame { path: String },

    // ── Playback ───────────────────────────────────
    Play,
    Pause,
    Stop,
    SetFrame { frame: usize },
    SetSpeed { speed: f32 },
    SetTime { time: f32 },
    ToggleLoop { enabled: bool },
    ToggleMirror { enabled: bool },

    // ── Camera ─────────────────────────────────────
    CameraReset,
    CameraLookAt { x: f32, y: f32, z: f32 },
    CameraView { view: String }, // "front", "right", "top"
    CameraDistance { distance: f32 },

    // ── Selection ──────────────────────────────────
    SelectEntity { id: usize },
    SelectBone { name: String },
    DeselectAll,

    // ── Transform ──────────────────────────────────
    SetPosition { entity: usize, x: f32, y: f32, z: f32 },
    SetRotation { entity: usize, rx: f32, ry: f32, rz: f32 },
    SetScale { entity: usize, sx: f32, sy: f32, sz: f32 },

    // ── Tools ──────────────────────────────────────
    SetTool { tool: String }, // "select", "move", "rotate", "measure", "ik"
    SetAxis { axis: String }, // "x", "y", "z", "none"

    // ── Display ────────────────────────────────────
    ToggleSkeleton { visible: bool },
    ToggleMesh { visible: bool },
    ToggleGrid { visible: bool },
    ToggleVelocities { visible: bool },
    ToggleContacts { visible: bool },
    ToggleTrajectory { visible: bool },

    // ── Render settings ────────────────────────────
    SetRender { key: String, value: serde_json::Value },

    // ── IK ─────────────────────────────────────────
    IkSolve {
        root: String,
        tip: String,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    },

    // ── Console ────────────────────────────────────
    Log { text: String },

    // ── Panel visibility ───────────────────────────
    ShowPanel { panel: String },  // "console", "profiler", "timeline", etc.
    HidePanel { panel: String },

    // ── Scene info queries ─────────────────────────
    /// Query current scene state — AI gets context back
    QueryScene,
    /// Query specific entity info
    QueryEntity { id: usize },
    /// List all bones in the active model
    ListBones,

    // ── Generative commands ───────────────────────────
    /// Create a procedural humanoid with mesh, skinning, and animation.
    CreateHumanoid {
        #[serde(default = "default_name")]
        name: String,
        #[serde(default = "default_height")]
        height: f32,
        #[serde(default)]
        animation: String,
        #[serde(default = "default_duration")]
        duration: f32,
        #[serde(default)]
        skin_color: Option<[u8; 3]>,
        #[serde(default)]
        shirt_color: Option<[u8; 3]>,
        #[serde(default)]
        pants_color: Option<[u8; 3]>,
        #[serde(default)]
        shoes_color: Option<[u8; 3]>,
        #[serde(default)]
        hair_color: Option<[u8; 3]>,
    },

    /// Add a procedural animation to the active model.
    CreateAnimation {
        /// Animation type: "run", "walk", "idle", "jump"
        #[serde(rename = "type")]
        anim_type: String,
        #[serde(default = "default_duration")]
        duration: f32,
    },

    /// Delete the active model from the scene.
    DeleteModel,

    /// Set material color on the active model.
    SetColor {
        r: u8, g: u8, b: u8,
    },

    /// Free-form generation: AI interprets the description and chains commands.
    Generate {
        description: String,
    },

    // ── AI Locomotion ────────────────────────────────
    /// Load a locomotion ONNX model for neural motion synthesis.
    LoadLocomotion {
        /// Path to the .onnx model file.
        model_path: String,
        /// Path to the _meta.npz metadata file.
        meta_path: String,
    },

    /// Start/stop AI-driven locomotion on the active model.
    ToggleLocomotion { enabled: bool },

    // ── Training ─────────────────────────────────────
    /// Train a new locomotion model from motion data.
    TrainModel {
        /// Directory containing BVH/NPZ motion files for training data.
        data_dir: String,
        /// Output directory for the trained model.
        #[serde(default = "default_model_output")]
        output_dir: String,
        /// Number of training epochs (default: 100).
        #[serde(default = "default_epochs")]
        epochs: usize,
        /// Batch size (default: 32).
        #[serde(default = "default_batch_size")]
        batch_size: usize,
        /// Learning rate (default: 1e-4).
        #[serde(default = "default_lr")]
        learning_rate: f64,
    },

    /// Convert a PyTorch .pt model to ONNX format.
    ConvertModel {
        /// Path to the .pt PyTorch model.
        model_path: String,
        /// Output directory for .onnx + _meta.json.
        #[serde(default = "default_model_output")]
        output_dir: String,
    },

    // ── Motion Matching ─────────────────────────────────
    /// Add all loaded clips to the motion matching database and build it.
    BuildMotionDb,

    /// Toggle the motion matching controller on/off.
    ToggleMotionMatching { enabled: bool },

    // ── State Machine ───────────────────────────────────
    /// Set a state machine parameter (bool or float).
    SetStateMachineParam {
        name: String,
        value: serde_json::Value,
    },

    /// Create a state machine state referencing a loaded model.
    CreateStateMachineState {
        name: String,
        #[serde(default)]
        model_index: Option<usize>,
    },

    // ── Blend Tree ─────────────────────────────────────
    /// Set a blend tree parameter value.
    SetBlendTreeParam {
        name: String,
        value: f32,
    },

    /// Export the active model as GLB.
    ExportGlb { path: String },

    // ── Multi-character ──────────────────────────────────
    /// Select a model by index.
    SelectModel { index: usize },

    /// Enable independent playback on a model.
    SetModelPlayback {
        index: usize,
        #[serde(default)]
        playing: Option<bool>,
        #[serde(default)]
        speed: Option<f32>,
        #[serde(default)]
        looping: Option<bool>,
    },

    /// Set world offset for a model (multi-character placement).
    SetModelOffset {
        index: usize,
        x: f32,
        y: f32,
        z: f32,
    },

    /// Toggle model visibility.
    SetModelVisible {
        index: usize,
        visible: bool,
    },

    // ── Ragdoll Physics ────────────────────────────────
    /// Toggle ragdoll simulation on/off.
    ToggleRagdoll { enabled: bool },

    /// Create a ragdoll from the current pose.
    CreateRagdoll,

    /// Destroy the active ragdoll.
    DestroyRagdoll,

    /// Apply an impulse to all ragdoll bodies.
    RagdollImpulse {
        x: f32,
        y: f32,
        z: f32,
    },

    /// Apply an explosion force to the ragdoll.
    RagdollExplosion {
        #[serde(default)]
        force: Option<f32>,
        #[serde(default)]
        radius: Option<f32>,
    },

    /// Pin or unpin a ragdoll body.
    RagdollPin {
        body: usize,
        pinned: bool,
    },

    // ── DeepPhase ─────────────────────────────────────
    /// Extract the DeepPhase manifold from the active animation.
    ExtractDeepPhase,

    /// Clear the DeepPhase manifold.
    ClearDeepPhase,

    // ── FBX Export ───────────────────────────────────
    /// Export the active model as FBX (ASCII 7.4).
    ExportFbx { path: String },

    /// Export the active model as USD (USDA ASCII).
    ExportUsd { path: String },

    // ── Animation Recording ─────────────────────────
    /// Start recording animation transforms.
    StartRecording,

    /// Stop recording and save the clip.
    StopRecording,

    /// Pause the current recording.
    PauseRecording,

    /// Resume a paused recording.
    ResumeRecording,

    // ── Cloth / Soft-body ─────────────────────────────
    /// Create a cloth grid.
    CreateCloth {
        #[serde(default = "default_cloth_w")]
        width: usize,
        #[serde(default = "default_cloth_h")]
        height: usize,
        #[serde(default = "default_cloth_size")]
        size: f32,
    },

    /// Destroy the cloth simulation.
    DestroyCloth,

    /// Toggle cloth simulation on/off.
    ToggleCloth { enabled: bool },

    // ── Material ────────────────────────────────────
    /// Set material properties on the active model.
    SetMaterial {
        #[serde(default)]
        color: Option<[u8; 3]>,
        #[serde(default)]
        metallic: Option<f32>,
        #[serde(default)]
        roughness: Option<f32>,
    },

    // ── Procedural creatures ────────────────────────
    /// Create a procedural creature (spider, crab, bird, etc.).
    CreateCreature {
        /// Creature type: "spider", "crab", "bird", "snake", "quadruped"
        creature_type: String,
        #[serde(default = "default_height")]
        height: f32,
    },

    /// Create a node in the blend tree.
    CreateBlendTreeNode {
        /// Node type: "clip", "blend1d" / "1d", "blend2d" / "2d", "lerp"
        node_type: String,
        name: String,
        /// Parameter name (for blend/lerp nodes).
        #[serde(default)]
        parameter: Option<String>,
        /// Model index (for clip nodes).
        #[serde(default)]
        model_index: Option<usize>,
    },
}

fn default_cloth_w() -> usize { 12 }
fn default_cloth_h() -> usize { 12 }
fn default_cloth_size() -> f32 { 1.5 }
fn default_name() -> String { "Humanoide".into() }
fn default_height() -> f32 { 1.75 }
fn default_duration() -> f32 { 3.0 }
fn default_model_output() -> String { "models/locomotion".into() }
fn default_epochs() -> usize { 100 }
fn default_batch_size() -> usize { 32 }
fn default_lr() -> f64 { 1e-4 }

/// Result of executing a command.
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Command executed successfully.
    Ok,
    /// Command executed with a response message for the AI.
    OkWithInfo(String),
    /// Command failed.
    Error(String),
    /// Command is a query — return scene data to the AI.
    Data(String),
}

/// Parse AI response text into commands.
/// The AI can return:
///   1. A single JSON object: {"action": "play"}
///   2. A JSON array: [{"action": "play"}, {"action": "set_speed", "speed": 2.0}]
///   3. Mixed text with embedded JSON (extracted from ```json ... ``` blocks)
///   4. Plain text (no commands — just conversational response)
pub fn parse_commands(response: &str) -> (Vec<AiCommand>, String) {
    let trimmed = response.trim();

    // Try parsing as a JSON array directly
    if trimmed.starts_with('[') {
        if let Ok(cmds) = serde_json::from_str::<Vec<AiCommand>>(trimmed) {
            return (cmds, String::new());
        }
    }

    // Try parsing as a single JSON object
    if trimmed.starts_with('{') {
        if let Ok(cmd) = serde_json::from_str::<AiCommand>(trimmed) {
            return (vec![cmd], String::new());
        }
    }

    // Look for ```json ... ``` code blocks in the text
    let mut commands = Vec::new();
    let mut text_parts = Vec::new();
    let mut remaining = trimmed;

    while let Some(start) = remaining.find("```json") {
        // Text before the code block
        let before = &remaining[..start];
        if !before.trim().is_empty() {
            text_parts.push(before.trim());
        }

        let json_start = start + 7; // skip "```json"
        if let Some(end) = remaining[json_start..].find("```") {
            let json_str = remaining[json_start..json_start + end].trim();

            // Try array
            if json_str.starts_with('[') {
                if let Ok(cmds) = serde_json::from_str::<Vec<AiCommand>>(json_str) {
                    commands.extend(cmds);
                }
            }
            // Try single object
            else if json_str.starts_with('{') {
                if let Ok(cmd) = serde_json::from_str::<AiCommand>(json_str) {
                    commands.push(cmd);
                }
            }

            remaining = &remaining[json_start + end + 3..];
        } else {
            break;
        }
    }

    if !remaining.trim().is_empty() {
        text_parts.push(remaining.trim());
    }

    let text = text_parts.join("\n");
    (commands, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_command() {
        let (cmds, text) = parse_commands(r#"{"action": "play"}"#);
        assert_eq!(cmds.len(), 1);
        assert!(text.is_empty());
    }

    #[test]
    fn parse_array_commands() {
        let input = r#"[{"action": "play"}, {"action": "set_speed", "speed": 2.0}]"#;
        let (cmds, _) = parse_commands(input);
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn parse_text_with_code_block() {
        let input = "Voici ce que je fais:\n```json\n{\"action\": \"play\"}\n```\nC'est fait!";
        let (cmds, text) = parse_commands(input);
        assert_eq!(cmds.len(), 1);
        assert!(text.contains("Voici"));
        assert!(text.contains("fait"));
    }

    #[test]
    fn parse_plain_text() {
        let (cmds, text) = parse_commands("Bonjour, comment puis-je vous aider?");
        assert!(cmds.is_empty());
        assert!(!text.is_empty());
    }
}
