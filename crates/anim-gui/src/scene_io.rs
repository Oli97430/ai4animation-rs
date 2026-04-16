//! Scene save/load to JSON format.

use serde::{Serialize, Deserialize};
use std::path::Path;

/// Serializable project state.
#[derive(Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub camera: CameraState,
    pub display: DisplayState,
    pub render_settings: RenderSettingsState,
    pub loaded_files: Vec<String>,
    pub active_model_index: Option<usize>,
    pub timestamp: f32,
    pub playback_speed: f32,
    pub looping: bool,
    /// Retarget bindings: (mesh_idx, anim_idx) pairs.
    #[serde(default)]
    pub retarget_bindings: Vec<(usize, usize)>,
    /// Module visibility flags.
    #[serde(default)]
    pub modules: ModuleVisibility,
    /// Panel visibility for extra panels.
    #[serde(default)]
    pub panels: PanelVisibility,
    /// AI provider configuration.
    #[serde(default)]
    pub ai_config: Option<AiConfigState>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ModuleVisibility {
    #[serde(default)]
    pub show_contacts: bool,
    #[serde(default)]
    pub show_trajectory: bool,
    #[serde(default)]
    pub show_guidance: bool,
    #[serde(default)]
    pub show_tracking: bool,
    #[serde(default)]
    pub show_root_motion: bool,
    #[serde(default)]
    pub mirrored: bool,
    #[serde(default)]
    pub onion_skinning: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct PanelVisibility {
    #[serde(default)]
    pub show_console: bool,
    #[serde(default)]
    pub show_profiler: bool,
    #[serde(default)]
    pub show_dope_sheet: bool,
    #[serde(default)]
    pub show_motion_editor: bool,
    #[serde(default)]
    pub show_recorder: bool,
    #[serde(default)]
    pub show_batch: bool,
    #[serde(default)]
    pub show_asset_browser: bool,
    #[serde(default)]
    pub show_render_settings: bool,
    #[serde(default)]
    pub show_ai_chat: bool,
    #[serde(default)]
    pub show_training: bool,
    #[serde(default)]
    pub show_motion_matching: bool,
    #[serde(default)]
    pub show_state_machine: bool,
    #[serde(default)]
    pub show_pose_editor: bool,
    #[serde(default)]
    pub show_blend_tree: bool,
    #[serde(default)]
    pub show_graph_editor: bool,
    #[serde(default)]
    pub show_ragdoll: bool,
    #[serde(default)]
    pub show_deep_phase: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AiConfigState {
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    #[serde(default)]
    pub max_tokens: usize,
    #[serde(default)]
    pub temperature: f32,
}

#[derive(Serialize, Deserialize)]
pub struct CameraState {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayState {
    pub show_skeleton: bool,
    pub show_mesh: bool,
    pub show_velocities: bool,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_gizmo: bool,
}

#[derive(Serialize, Deserialize)]
pub struct RenderSettingsState {
    pub exposure: f32,
    pub sun_strength: f32,
    pub sky_strength: f32,
    pub ground_strength: f32,
    pub ambient_strength: f32,
    pub light_yaw: f32,
    pub light_pitch: f32,
    pub sun_color: [f32; 3],
    pub shadows_enabled: bool,
    pub shadow_bias: f32,
    pub ssao_enabled: bool,
    pub ssao_radius: f32,
    pub ssao_intensity: f32,
    pub ssao_bias: f32,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_spread: f32,
    pub fxaa_enabled: bool,
}

/// Save the current project state to a JSON file.
pub fn save_project(path: &Path, state: &crate::app_state::AppState) -> Result<(), Box<dyn std::error::Error>> {
    // Store source paths for models loaded from files; use name as fallback for procedural models
    let loaded_files: Vec<String> = state.loaded_models.iter()
        .map(|a| if a.source_path.is_empty() { a.name.clone() } else { a.source_path.clone() })
        .collect();

    let s = &state.render_settings;
    let project = ProjectFile {
        version: 1,
        camera: CameraState {
            target: state.camera.target.into(),
            distance: state.camera.distance,
            yaw: state.camera.yaw,
            pitch: state.camera.pitch,
        },
        display: DisplayState {
            show_skeleton: state.show_skeleton,
            show_mesh: state.show_mesh,
            show_velocities: state.show_velocities,
            show_grid: state.show_grid,
            show_axes: state.show_axes,
            show_gizmo: state.show_gizmo,
        },
        render_settings: RenderSettingsState {
            exposure: s.exposure,
            sun_strength: s.sun_strength,
            sky_strength: s.sky_strength,
            ground_strength: s.ground_strength,
            ambient_strength: s.ambient_strength,
            light_yaw: s.light_yaw,
            light_pitch: s.light_pitch,
            sun_color: s.sun_color,
            shadows_enabled: s.shadows_enabled,
            shadow_bias: s.shadow_bias,
            ssao_enabled: s.ssao_enabled,
            ssao_radius: s.ssao_radius,
            ssao_intensity: s.ssao_intensity,
            ssao_bias: s.ssao_bias,
            bloom_enabled: s.bloom_enabled,
            bloom_intensity: s.bloom_intensity,
            bloom_spread: s.bloom_spread,
            fxaa_enabled: s.fxaa_enabled,
        },
        loaded_files,
        active_model_index: state.active_model,
        timestamp: state.timestamp,
        playback_speed: state.playback_speed,
        looping: state.looping,
        retarget_bindings: state.loaded_models.iter()
            .enumerate()
            .filter_map(|(i, a)| a.retarget.as_ref().map(|r| (i, r.source_asset)))
            .collect(),
        modules: ModuleVisibility {
            show_contacts: state.show_contacts,
            show_trajectory: state.show_trajectory,
            show_guidance: state.show_guidance,
            show_tracking: state.show_tracking,
            show_root_motion: state.show_root_motion,
            mirrored: state.mirrored,
            onion_skinning: state.onion_skinning,
        },
        panels: PanelVisibility {
            show_console: state.show_console,
            show_profiler: state.show_profiler,
            show_dope_sheet: state.show_dope_sheet,
            show_motion_editor: state.show_motion_editor,
            show_recorder: state.show_recorder,
            show_batch: state.show_batch,
            show_asset_browser: state.show_asset_browser,
            show_render_settings: state.show_render_settings,
            show_ai_chat: state.show_ai_chat,
            show_training: state.show_training,
            show_motion_matching: state.show_motion_matching,
            show_state_machine: state.show_state_machine,
            show_pose_editor: state.show_pose_editor,
            show_blend_tree: state.show_blend_tree,
            show_graph_editor: state.show_graph_editor,
            show_ragdoll: state.show_ragdoll,
            show_deep_phase: state.show_deep_phase,
        },
        ai_config: None, // AI config saved separately; API keys are not persisted for security
    };

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load project settings from a JSON file. Returns the project data.
/// Note: the caller must re-import the model files separately.
pub fn load_project(path: &Path) -> Result<ProjectFile, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let project: ProjectFile = serde_json::from_str(&json)?;
    Ok(project)
}

/// Apply loaded project settings to the app state.
pub fn apply_project(state: &mut crate::app_state::AppState, project: &ProjectFile) {
    // Camera
    state.camera.target = project.camera.target.into();
    state.camera.distance = project.camera.distance;
    state.camera.yaw = project.camera.yaw;
    state.camera.pitch = project.camera.pitch;
    state.camera.reset(); // recalculate position from yaw/pitch/distance

    // Display
    state.show_skeleton = project.display.show_skeleton;
    state.show_mesh = project.display.show_mesh;
    state.show_velocities = project.display.show_velocities;
    state.show_grid = project.display.show_grid;
    state.show_axes = project.display.show_axes;
    state.show_gizmo = project.display.show_gizmo;

    // Render settings
    let s = &mut state.render_settings;
    let rs = &project.render_settings;
    s.exposure = rs.exposure;
    s.sun_strength = rs.sun_strength;
    s.sky_strength = rs.sky_strength;
    s.ground_strength = rs.ground_strength;
    s.ambient_strength = rs.ambient_strength;
    s.light_yaw = rs.light_yaw;
    s.light_pitch = rs.light_pitch;
    s.sun_color = rs.sun_color;
    s.shadows_enabled = rs.shadows_enabled;
    s.shadow_bias = rs.shadow_bias;
    s.ssao_enabled = rs.ssao_enabled;
    s.ssao_radius = rs.ssao_radius;
    s.ssao_intensity = rs.ssao_intensity;
    s.ssao_bias = rs.ssao_bias;
    s.bloom_enabled = rs.bloom_enabled;
    s.bloom_intensity = rs.bloom_intensity;
    s.bloom_spread = rs.bloom_spread;
    s.fxaa_enabled = rs.fxaa_enabled;

    // Playback
    state.timestamp = project.timestamp;
    state.playback_speed = project.playback_speed;
    state.looping = project.looping;

    // Module visibility
    state.show_contacts = project.modules.show_contacts;
    state.show_trajectory = project.modules.show_trajectory;
    state.show_guidance = project.modules.show_guidance;
    state.show_tracking = project.modules.show_tracking;
    state.show_root_motion = project.modules.show_root_motion;
    state.mirrored = project.modules.mirrored;
    state.onion_skinning = project.modules.onion_skinning;

    // Panel visibility
    state.show_console = project.panels.show_console;
    state.show_profiler = project.panels.show_profiler;
    state.show_dope_sheet = project.panels.show_dope_sheet;
    state.show_motion_editor = project.panels.show_motion_editor;
    state.show_recorder = project.panels.show_recorder;
    state.show_batch = project.panels.show_batch;
    state.show_asset_browser = project.panels.show_asset_browser;
    state.show_render_settings = project.panels.show_render_settings;
    state.show_ai_chat = project.panels.show_ai_chat;
    state.show_training = project.panels.show_training;
    state.show_motion_matching = project.panels.show_motion_matching;
    state.show_state_machine = project.panels.show_state_machine;
    state.show_pose_editor = project.panels.show_pose_editor;
    state.show_blend_tree = project.panels.show_blend_tree;
    state.show_graph_editor = project.panels.show_graph_editor;
    state.show_ragdoll = project.panels.show_ragdoll;
    state.show_deep_phase = project.panels.show_deep_phase;

    // Active model index
    if let Some(idx) = project.active_model_index {
        if idx < state.loaded_models.len() {
            state.active_model = Some(idx);
        }
    }

    // Retarget bindings (must be applied after models are re-loaded)
    for &(mesh_idx, anim_idx) in &project.retarget_bindings {
        state.retarget_mesh(mesh_idx, anim_idx);
    }
}

/// Re-import models from the saved file paths. Returns the number of successfully loaded files.
/// Must be called after `apply_project` to populate `loaded_models`.
pub fn reload_models(
    state: &mut crate::app_state::AppState,
    project: &ProjectFile,
    asset_manager: &mut anim_import::AssetManager,
) -> usize {
    let mut loaded = 0;
    for file_ref in &project.loaded_files {
        let path = std::path::PathBuf::from(file_ref);
        if path.exists() {
            match asset_manager.load(&path) {
                Ok(model) => {
                    state.import_model_from_path(model, &path);
                    loaded += 1;
                }
                Err(e) => {
                    state.log_error(&format!("Erreur rechargement '{}': {:#}", file_ref, e));
                }
            }
        } else if !file_ref.contains('/') && !file_ref.contains('\\') {
            // Probably a procedural model name, not a file path — skip silently
            state.log_info(&format!("Modèle procédural ignoré: {}", file_ref));
        } else {
            state.log_warn(&format!("Fichier introuvable: {}", file_ref));
        }
    }
    loaded
}
