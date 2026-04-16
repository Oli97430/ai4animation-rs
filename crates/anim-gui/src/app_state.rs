//! Application state shared between all panels.

use glam::Vec3;
use anim_core::{Scene, Time};
use anim_animation::{Motion, ContactModule, Trajectory, TrajectoryConfig, GuidanceModule, TrackingModule, RootMotion, RootConfig, RetargetMap, PhaseData, Actor, AnimationTransition, AnimationLayer, MotionDatabase, MotionMatchingController, StateMachine, BlendTree, Ragdoll, DeepPhaseManifold};
use anim_render::{Camera, DebugDraw, RenderSettings};
use anim_render::grid::GridConfig;
use anim_render::skinned_mesh::SkinnedMeshData;
use anim_import::ImportedModel;

/// Active viewport tool (SketchUp-style).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    Move,
    Rotate,
    Measure,
    Ik,
}

/// IK constraint preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IkPreset {
    None,
    HumanArm,
    HumanLeg,
    Custom,
}

/// Gizmo axis for constrained manipulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoAxis {
    None,
    X,
    Y,
    Z,
}

/// Measurement state for the tape-measure tool.
#[derive(Default, Clone)]
pub struct MeasureState {
    pub start: Option<Vec3>,
    pub end: Option<Vec3>,
}

impl MeasureState {
    pub fn distance(&self) -> Option<f32> {
        match (self.start, self.end) {
            (Some(a), Some(b)) => Some(a.distance(b)),
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        self.start = None;
        self.end = None;
    }
}

/// State for interactive drag manipulation (SketchUp-style click-and-drag).
#[derive(Clone)]
pub struct DragState {
    /// Entity being dragged.
    pub entity_id: Option<usize>,
    /// World position at drag start.
    pub start_world: Vec3,
    /// Entity position at drag start.
    pub start_entity_pos: Vec3,
    /// Is the drag currently active?
    pub active: bool,
    /// Cumulative rotation during a rotate drag (radians).
    pub rotate_angle: f32,
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            entity_id: None,
            start_world: Vec3::ZERO,
            start_entity_pos: Vec3::ZERO,
            active: false,
            rotate_angle: 0.0,
        }
    }
}

/// A single undo/redo snapshot.
#[derive(Clone)]
pub struct UndoSnapshot {
    pub description: String,
    pub entity_id: usize,
    pub transform: glam::Mat4,
}

/// Undo/Redo history stack.
pub struct UndoHistory {
    pub undo_stack: Vec<UndoSnapshot>,
    pub redo_stack: Vec<UndoSnapshot>,
}

impl UndoHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Push a snapshot before a change is made.
    pub fn push(&mut self, snapshot: UndoSnapshot) {
        self.undo_stack.push(snapshot);
        self.redo_stack.clear(); // new action invalidates redo
        // Keep max 100 undos
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

impl Default for UndoHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Central application state.
pub struct AppState {
    pub scene: Scene,
    pub time: Time,
    pub camera: Camera,
    pub debug_draw: DebugDraw,
    pub grid_config: GridConfig,

    // Loaded data
    pub loaded_models: Vec<LoadedAsset>,
    pub active_model: Option<usize>,

    // Animation playback
    pub playing: bool,
    pub timestamp: f32,
    pub playback_speed: f32,
    pub looping: bool,
    pub mirrored: bool,

    // Display options
    pub show_skeleton: bool,
    pub show_mesh: bool,
    pub show_velocities: bool,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_gizmo: bool,

    // Render settings
    pub render_settings: RenderSettings,
    pub show_render_settings: bool,

    // Tools (SketchUp-style)
    pub active_tool: Tool,
    pub gizmo_axis: GizmoAxis,
    pub measure: MeasureState,
    pub drag: DragState,
    pub snap_to_grid: bool,
    pub snap_size: f32,

    // IK tool
    pub ik_chain_root: Option<usize>,  // entity id of IK chain root
    pub ik_chain_tip: Option<usize>,   // entity id of IK chain tip (end effector)
    pub ik_use_constraints: bool,      // enable joint angle limits
    pub ik_use_pole_target: bool,      // enable pole target
    pub ik_pole_position: Vec3,        // pole target world position
    pub ik_pole_weight: f32,           // pole target influence [0..1]
    pub ik_preset: IkPreset,           // selected constraint preset

    // Undo/Redo
    pub history: UndoHistory,

    // Walk mode (WASD + mouse look)
    pub walk_speed: f32,

    // Console
    pub console_messages: Vec<ConsoleMessage>,

    // UI state
    pub show_console: bool,

    // Multi-selection (Ctrl+click to add, Shift+click range)
    pub multi_selection: Vec<usize>,

    // Onion skinning (ghost frames before/after current)
    pub onion_skinning: bool,
    pub onion_before: usize,  // how many past frames to show
    pub onion_after: usize,   // how many future frames to show
    pub onion_step: usize,    // frame step between ghosts

    // Viewport camera presets
    pub show_view_cube: bool,

    // Auto-key mode: when moving/rotating, automatically insert a keyframe
    pub auto_key: bool,

    // Pose clipboard (copy/paste poses between frames)
    pub pose_clipboard: Option<PoseClipboard>,

    // Dope sheet state
    pub show_dope_sheet: bool,
    pub dope_sheet_zoom: f32,   // pixels per frame
    pub dope_sheet_scroll: f32, // scroll offset in frames

    // Context menu
    pub context_menu_entity: Option<usize>,

    // Rename state (for inline rename in hierarchy)
    pub rename_entity: Option<usize>,
    pub rename_buffer: String,

    // Contact detection
    pub show_contacts: bool,
    pub contact_module: Option<ContactModule>,

    // Trajectory visualization
    pub show_trajectory: bool,
    pub trajectory_config: TrajectoryConfig,

    // Guidance module
    pub show_guidance: bool,
    pub guidance_module: Option<GuidanceModule>,

    // Tracking module
    pub show_tracking: bool,
    pub tracking_module: Option<TrackingModule>,

    // Motion editor panel
    pub show_motion_editor: bool,

    // Profiler
    pub show_profiler: bool,

    // Batch converter
    pub show_batch: bool,

    // Hierarchy search filter
    pub hierarchy_filter: String,

    // Video recorder panel visibility
    pub show_recorder: bool,

    // Asset browser
    pub show_asset_browser: bool,

    // Shortcut editor
    pub show_shortcut_editor: bool,

    // AI chat panel
    pub show_ai_chat: bool,

    // Root motion
    pub show_root_motion: bool,
    pub root_config: RootConfig,
    pub root_motion: Option<RootMotion>,

    // Phase detection
    pub phase_data: Option<PhaseData>,

    // AI locomotion controller
    pub locomotion_controller: Option<anim_animation::LocomotionController>,
    /// WASD input velocity for locomotion (set by viewport each frame).
    pub locomotion_velocity: Vec3,
    /// Facing direction for locomotion (set by viewport/mouse).
    pub locomotion_direction: Vec3,
    /// Whether the locomotion controller is driving the active model.
    pub locomotion_active: bool,
    /// Sprint multiplier (Shift held).
    pub locomotion_sprint: bool,

    // Background task log messages (thread-safe)
    pub bg_log_queue: std::sync::Arc<std::sync::Mutex<Vec<String>>>,

    // Animation blending
    /// Crossfade transition when switching animations.
    pub animation_transition: AnimationTransition,
    /// Previous animation pose (for crossfade blending during transition).
    pub transition_source_pose: Vec<glam::Mat4>,
    /// Animation layers (upper body override, additive, etc.).
    pub animation_layers: Vec<AnimationLayer>,
    /// Crossfade duration in seconds (user-configurable).
    pub crossfade_duration: f32,

    // Training state
    pub training_active: bool,
    pub show_training: bool,

    // Motion matching
    pub motion_database: MotionDatabase,
    pub motion_matching_controller: MotionMatchingController,
    pub show_motion_matching: bool,

    // Animation state machine
    pub state_machine: Option<StateMachine>,
    pub show_state_machine: bool,

    // Pose editor
    pub show_pose_editor: bool,

    // Blend tree
    pub blend_tree: Option<BlendTree>,
    pub show_blend_tree: bool,

    // Graph editor (curve editor)
    pub show_graph_editor: bool,

    // Ragdoll physics
    pub ragdoll: Option<Ragdoll>,
    pub show_ragdoll: bool,

    // DeepPhase manifold
    pub deep_phase: Option<DeepPhaseManifold>,
    pub show_deep_phase: bool,

    // Animation recorder
    pub show_anim_recorder: bool,

    // Cloth / soft-body
    pub cloth_sim: Option<anim_animation::ClothSim>,
    pub show_cloth: bool,

    // Material editor
    pub show_material_editor: bool,

    // IK panel
    pub show_ik_panel: bool,

    // Flash-style timeline
    pub show_flash_timeline: bool,

    // Pending project load (set by menu, processed by main loop with AssetManager access)
    pub pending_project_load: Option<std::path::PathBuf>,
    // Pending frame export (set by AI command, processed by main loop with render access)
    pub pending_export_frame: Option<String>,
    // Pending file import from recent menu (path to import)
    pub pending_recent_import: Option<std::path::PathBuf>,
    // Recent files list (synced from AnimApp for display in menu bar)
    pub recent_files_display: Vec<(String, String, String)>, // (path, name, file_type)
    // Last imported file path (for recent files tracking by main loop)
    pub last_imported_path: Option<std::path::PathBuf>,
}

/// Stored pose for copy/paste.
#[derive(Clone)]
pub struct PoseClipboard {
    pub transforms: Vec<glam::Mat4>,
    pub source_frame: usize,
    pub joint_count: usize,
}

pub struct LoadedAsset {
    pub name: String,
    /// Original file path (for save/reload). Empty for procedural models.
    pub source_path: String,
    pub model: ImportedModel,
    pub motion: Option<Motion>,
    pub skinned_mesh: Option<SkinnedMeshData>,
    /// Entity IDs for this model's joints in the scene.
    pub joint_entity_ids: Vec<usize>,
    /// Retarget map: animation from another asset applied to this one's mesh.
    pub retarget: Option<RetargetBinding>,
    /// Actor component: skeletal character with transforms, velocities, FK.
    pub actor: Option<Actor>,

    // ── Per-model playback (multi-character) ──────────────
    /// Whether this model is animating independently.
    pub independent_playback: bool,
    /// Per-model timestamp (used when independent_playback is true).
    pub local_timestamp: f32,
    /// Per-model playback speed.
    pub local_speed: f32,
    /// Per-model looping.
    pub local_looping: bool,
    /// Per-model playing state.
    pub local_playing: bool,
    /// Per-model mirror.
    pub local_mirrored: bool,
    /// World-space offset for placing multiple characters.
    pub world_offset: Vec3,
    /// Visibility: if false, the model is hidden.
    pub visible: bool,
}

/// Binding that links this asset's mesh to another asset's animation.
#[derive(Clone)]
pub struct RetargetBinding {
    /// Index of the source asset (the one with animation).
    pub source_asset: usize,
    /// Joint mapping from this asset's skeleton to the source's skeleton.
    pub map: RetargetMap,
}

#[derive(Clone)]
pub struct ConsoleMessage {
    pub level: ConsoleLevel,
    pub text: String,
    pub time: f32, // total_time when message was logged
}

#[derive(Clone, Copy, PartialEq)]
pub enum ConsoleLevel {
    Info,
    Warning,
    Error,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            time: Time::new(),
            camera: Camera::default(),
            debug_draw: DebugDraw::new(),
            grid_config: GridConfig::default(),
            loaded_models: Vec::new(),
            active_model: None,
            playing: false,
            timestamp: 0.0,
            playback_speed: 1.0,
            looping: true,
            mirrored: false,
            show_skeleton: true,
            show_mesh: true,
            show_velocities: false,
            show_grid: true,
            show_axes: true,
            show_gizmo: true,
            render_settings: RenderSettings::default(),
            show_render_settings: false,
            active_tool: Tool::Select,
            gizmo_axis: GizmoAxis::None,
            measure: MeasureState::default(),
            drag: DragState::default(),
            snap_to_grid: false,
            snap_size: 0.1,
            ik_chain_root: None,
            ik_chain_tip: None,
            ik_use_constraints: false,
            ik_use_pole_target: false,
            ik_pole_position: Vec3::new(0.0, 0.0, 1.0),
            ik_pole_weight: 0.8,
            ik_preset: IkPreset::None,
            history: UndoHistory::new(),
            walk_speed: 3.0,
            console_messages: Vec::new(),
            show_console: true,
            multi_selection: Vec::new(),
            onion_skinning: false,
            onion_before: 3,
            onion_after: 3,
            onion_step: 5,
            show_view_cube: true,
            auto_key: false,
            pose_clipboard: None,
            show_dope_sheet: false,
            dope_sheet_zoom: 4.0,
            dope_sheet_scroll: 0.0,
            context_menu_entity: None,
            rename_entity: None,
            rename_buffer: String::new(),
            show_contacts: false,
            contact_module: None,
            show_trajectory: false,
            trajectory_config: TrajectoryConfig::default(),
            show_guidance: false,
            guidance_module: None,
            show_tracking: false,
            tracking_module: None,
            show_motion_editor: false,
            show_profiler: false,
            show_batch: false,
            hierarchy_filter: String::new(),
            show_recorder: false,
            show_asset_browser: false,
            show_shortcut_editor: false,
            show_ai_chat: true, // Visible by default
            show_root_motion: false,
            root_config: RootConfig::default(),
            root_motion: None,
            phase_data: None,
            locomotion_controller: None,
            locomotion_velocity: Vec3::ZERO,
            locomotion_direction: Vec3::ZERO,
            locomotion_active: false,
            locomotion_sprint: false,
            bg_log_queue: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            animation_transition: AnimationTransition::new(0.3),
            transition_source_pose: Vec::new(),
            animation_layers: Vec::new(),
            crossfade_duration: 0.3,
            training_active: false,
            show_training: false,
            motion_database: MotionDatabase::new(),
            motion_matching_controller: MotionMatchingController::new(),
            show_motion_matching: false,
            state_machine: None,
            show_state_machine: false,
            show_pose_editor: false,
            blend_tree: None,
            show_blend_tree: false,
            show_graph_editor: false,
            ragdoll: None,
            show_ragdoll: false,
            deep_phase: None,
            show_deep_phase: false,
            show_anim_recorder: false,
            cloth_sim: None,
            show_cloth: false,
            show_material_editor: false,
            show_ik_panel: false,
            show_flash_timeline: false,
            pending_project_load: None,
            pending_export_frame: None,
            pending_recent_import: None,
            recent_files_display: Vec::new(),
            last_imported_path: None,
        }
    }

    pub fn log(&mut self, level: ConsoleLevel, text: &str) {
        self.console_messages.push(ConsoleMessage {
            level,
            text: text.to_string(),
            time: self.time.total_time,
        });
        // Keep last 500 messages
        if self.console_messages.len() > 500 {
            self.console_messages.remove(0);
        }
    }

    pub fn log_info(&mut self, text: &str) {
        self.log(ConsoleLevel::Info, text);
    }

    pub fn log_warn(&mut self, text: &str) {
        self.log(ConsoleLevel::Warning, text);
    }

    pub fn log_error(&mut self, text: &str) {
        self.log(ConsoleLevel::Error, text);
    }

    /// Undo the last transform change.
    pub fn undo(&mut self) {
        if let Some(snapshot) = self.history.undo_stack.pop() {
            // Save current state for redo
            let current_transform = self.scene.get_transform(snapshot.entity_id);
            self.history.redo_stack.push(UndoSnapshot {
                description: snapshot.description.clone(),
                entity_id: snapshot.entity_id,
                transform: current_transform,
            });
            // Restore old transform
            self.scene.set_transform(snapshot.entity_id, snapshot.transform, true);
            self.log_info(&format!("Annulé: {}", snapshot.description));
        }
    }

    /// Redo the last undone transform change.
    pub fn redo(&mut self) {
        if let Some(snapshot) = self.history.redo_stack.pop() {
            let current_transform = self.scene.get_transform(snapshot.entity_id);
            self.history.undo_stack.push(UndoSnapshot {
                description: snapshot.description.clone(),
                entity_id: snapshot.entity_id,
                transform: current_transform,
            });
            self.scene.set_transform(snapshot.entity_id, snapshot.transform, true);
            self.log_info(&format!("Refait: {}", snapshot.description));
        }
    }

    /// Start a crossfade transition. Captures the current pose as the source.
    pub fn start_crossfade(&mut self) {
        if let Some(idx) = self.active_model {
            if let Some(ref motion) = self.loaded_models[idx].motion {
                self.transition_source_pose = motion.get_transforms_interpolated(
                    self.timestamp, self.mirrored
                );
                self.animation_transition = AnimationTransition::new(self.crossfade_duration);
                self.animation_transition.start(self.timestamp, 0.0);
            }
        }
    }

    /// Get the current blended pose, accounting for any active transition.
    /// Returns None if no model/motion is loaded.
    pub fn get_blended_pose(&self, _dt: f32) -> Option<Vec<glam::Mat4>> {
        let idx = self.active_model?;
        let motion = self.loaded_models[idx].motion.as_ref()?;
        let current_pose = motion.get_transforms_interpolated(self.timestamp, self.mirrored);

        // Apply crossfade if transitioning
        let base_pose = if self.animation_transition.is_active() && !self.transition_source_pose.is_empty() {
            let weight = self.animation_transition.weight();
            anim_animation::blend_poses(&self.transition_source_pose, &current_pose, weight)
        } else {
            current_pose
        };

        // Apply animation layers
        if self.animation_layers.is_empty() {
            Some(base_pose)
        } else {
            let rest_pose = if let Some(ref actor) = self.loaded_models[idx].actor {
                actor.bones.iter().map(|b| b.zero_transform).collect()
            } else {
                base_pose.clone()
            };
            Some(anim_animation::apply_layers(&base_pose, &self.animation_layers, &rest_pose))
        }
    }

    /// Snap a position to the grid if snapping is enabled.
    pub fn snap_position(&self, pos: Vec3) -> Vec3 {
        if self.snap_to_grid && self.snap_size > 0.001 {
            let s = self.snap_size;
            Vec3::new(
                (pos.x / s).round() * s,
                (pos.y / s).round() * s,
                (pos.z / s).round() * s,
            )
        } else {
            pos
        }
    }

    /// Get the active motion (if any).
    pub fn active_motion(&self) -> Option<&Motion> {
        self.active_model
            .and_then(|idx| self.loaded_models.get(idx))
            .and_then(|asset| asset.motion.as_ref())
    }

    /// Get total animation time of active motion.
    pub fn total_time(&self) -> f32 {
        self.active_motion().map_or(0.0, |m| m.total_time())
    }

    /// Get total frame count of active motion.
    pub fn total_frames(&self) -> usize {
        self.active_motion().map_or(0, |m| m.num_frames())
    }

    /// Get current frame index.
    pub fn current_frame(&self) -> usize {
        self.active_motion().map_or(0, |m| m.frame_index(self.timestamp))
    }

    /// Update animation playback.
    pub fn update(&mut self, dt: f32) {
        self.time.update(dt);
        self.debug_draw.clear();

        if self.playing {
            self.timestamp += dt * self.playback_speed;
            let total = self.total_time();
            if total > 0.0 {
                if self.looping {
                    while self.timestamp > total {
                        self.timestamp -= total;
                    }
                } else {
                    if self.timestamp >= total {
                        self.timestamp = total;
                        self.playing = false;
                    }
                }
            }
        }

        // Advance crossfade transition
        if self.animation_transition.is_active() {
            self.animation_transition.update(dt);
        }

        // Update active model pose
        if let Some(idx) = self.active_model {
            if let Some(motion) = self.loaded_models[idx].motion.as_ref() {
                // Get current pose, blending with transition source if crossfading
                let raw_transforms = motion.get_transforms_interpolated(self.timestamp, self.mirrored);
                let transforms = if self.animation_transition.is_active() && !self.transition_source_pose.is_empty() {
                    let w = self.animation_transition.weight();
                    anim_animation::blend_poses(&self.transition_source_pose, &raw_transforms, w)
                } else {
                    raw_transforms
                };
                let positions: Vec<Vec3> = transforms.iter()
                    .map(|t| anim_math::transform::Transform::get_position(t))
                    .collect();

                // Update skeleton debug draw
                if self.show_skeleton {
                    self.debug_draw.skeleton(
                        &positions,
                        &motion.hierarchy.parent_indices,
                        anim_render::debug_draw::colors::BONE,
                        0.01,
                    );
                }

                // Update velocities debug draw
                if self.show_velocities {
                    let velocities = motion.get_velocities(self.timestamp, self.mirrored);
                    for (i, pos) in positions.iter().enumerate() {
                        self.debug_draw.velocity(
                            *pos,
                            velocities[i],
                            0.05,
                            anim_render::debug_draw::colors::GREEN,
                        );
                    }
                }

                // Contact detection visualization
                if self.show_contacts {
                    if let Some(ref contact) = self.contact_module {
                        let cf = contact.get_contacts(motion, self.timestamp, self.mirrored);
                        for i in 0..cf.positions.len() {
                            let color = if cf.contacts[i] {
                                anim_render::debug_draw::colors::GREEN
                            } else {
                                [0.0, 0.0, 0.0, 0.25]
                            };
                            self.debug_draw.point(cf.positions[i], 0.025, color);
                        }
                    }
                }

                // Trajectory visualization
                if self.show_trajectory {
                    let traj = Trajectory::compute(
                        motion, self.timestamp, self.mirrored, &self.trajectory_config,
                    );
                    // Draw trajectory line strip
                    let positions_traj = traj.positions();
                    for i in 1..positions_traj.len() {
                        let mid = positions_traj.len() / 2;
                        let color = if i <= mid {
                            // Past: fading blue
                            [0.3, 0.5, 1.0, 0.6]
                        } else {
                            // Future: orange
                            [1.0, 0.6, 0.2, 0.6]
                        };
                        self.debug_draw.line(positions_traj[i - 1], positions_traj[i], color);
                    }
                    // Draw sample points
                    for (i, s) in traj.samples.iter().enumerate() {
                        let mid = traj.samples.len() / 2;
                        let is_current = i == mid;
                        let size = if is_current { 0.02 } else { 0.01 };
                        let color = if is_current {
                            anim_render::debug_draw::colors::WHITE
                        } else if i < mid {
                            [0.3, 0.5, 1.0, 0.5]
                        } else {
                            [1.0, 0.6, 0.2, 0.5]
                        };
                        self.debug_draw.point(s.position, size, color);
                        // Direction arrow (forward)
                        let arrow_len = 0.15;
                        let dir_color = [1.0, 0.5, 0.0, 0.4];
                        self.debug_draw.line(s.position, s.position + s.direction * arrow_len, dir_color);
                    }
                }

                // Guidance visualization
                if self.show_guidance {
                    if let Some(ref gm) = self.guidance_module {
                        let gf = gm.compute(motion, self.timestamp, self.mirrored);
                        for pos in &gf.positions {
                            self.debug_draw.point(*pos, 0.03, anim_render::debug_draw::colors::MAGENTA);
                        }
                        // Connect guidance points with lines
                        for i in 1..gf.positions.len() {
                            self.debug_draw.line(gf.positions[i - 1], gf.positions[i],
                                [0.8, 0.2, 0.8, 0.3]);
                        }
                    }
                }

                // Tracking visualization
                if self.show_tracking {
                    if let Some(ref tm) = self.tracking_module {
                        let tf = tm.compute(motion, self.timestamp, self.mirrored);
                        let track_colors = [
                            [0.2, 0.8, 0.8, 0.6],  // cyan (head)
                            [0.8, 0.4, 0.2, 0.6],  // orange (left hand)
                            [0.2, 0.4, 0.8, 0.6],  // blue (right hand)
                        ];
                        for (ji, trajectory) in tf.trajectories.iter().enumerate() {
                            let color = track_colors.get(ji).copied()
                                .unwrap_or([0.5, 0.5, 0.5, 0.5]);
                            // Draw trajectory line
                            for i in 1..trajectory.len() {
                                self.debug_draw.line(
                                    trajectory[i - 1].position,
                                    trajectory[i].position,
                                    color,
                                );
                            }
                            // Draw sample points
                            for sample in trajectory {
                                self.debug_draw.point(sample.position, 0.008, color);
                                // Velocity vector (small)
                                let vel_color = [color[0], color[1], color[2], 0.3];
                                self.debug_draw.velocity(sample.position, sample.velocity, 0.02, vel_color);
                            }
                        }
                    }
                }

                // Root motion visualization
                if self.show_root_motion {
                    if let Some(ref rm) = self.root_motion {
                        let sample = rm.sample_at(self.timestamp);
                        // Draw root position marker
                        self.debug_draw.point(sample.position, 0.04, anim_render::debug_draw::colors::YELLOW);
                        // Draw forward direction arrow
                        let arrow_end = sample.position + sample.forward * 0.5;
                        self.debug_draw.line(sample.position, arrow_end, anim_render::debug_draw::colors::YELLOW);
                        // Draw velocity vector
                        let vel_end = sample.position + sample.velocity * 0.1;
                        self.debug_draw.line(sample.position, vel_end, [0.2, 1.0, 0.2, 0.6]);
                        // Draw root trail (past 30 frames)
                        let current_frame = rm.sample_at(self.timestamp).position;
                        let trail_frames = 30usize;
                        let dt = motion.delta_time();
                        let mut prev = current_frame;
                        for i in 1..=trail_frames {
                            let t = self.timestamp - i as f32 * dt;
                            if t < 0.0 { break; }
                            let s = rm.sample_at(t);
                            let alpha = 0.4 * (1.0 - i as f32 / trail_frames as f32);
                            self.debug_draw.line(prev, s.position, [1.0, 0.8, 0.2, alpha]);
                            prev = s.position;
                        }
                    }
                }

                // Update Actor component (stores current pose + velocities)
                if let Some(ref mut actor) = self.loaded_models[idx].actor {
                    let dt = self.time.delta_time;
                    actor.set_pose_with_velocities(&transforms, dt);
                }

                // Update skinned mesh bones
                if let Some(ref mut skin) = self.loaded_models[idx].skinned_mesh {
                    skin.update_bones(&transforms);
                }

                // Update scene entity transforms
                let entity_ids = &self.loaded_models[idx].joint_entity_ids;
                for (i, &eid) in entity_ids.iter().enumerate() {
                    if i < transforms.len() {
                        self.scene.transforms[eid] = transforms[i];
                    }
                }
            }
        }

        // ── Multi-character: update independently playing models ──
        {
            let dt = self.time.delta_time;
            let active = self.active_model;
            for mi in 0..self.loaded_models.len() {
                // Skip the active model (already updated above) and non-independent models
                if Some(mi) == active { continue; }
                let asset = &self.loaded_models[mi];
                if !asset.independent_playback || !asset.local_playing { continue; }
                if asset.motion.is_none() || !asset.visible { continue; }

                // Advance local timestamp
                let motion = asset.motion.as_ref().unwrap();
                let total = motion.total_time();
                let local_speed = asset.local_speed;
                let local_looping = asset.local_looping;
                let local_mirrored = asset.local_mirrored;
                let mut ts = asset.local_timestamp + dt * local_speed;
                if total > 0.0 {
                    if local_looping {
                        while ts > total { ts -= total; }
                    } else if ts >= total {
                        ts = total;
                        // Stop playback at end
                        self.loaded_models[mi].local_playing = false;
                    }
                }
                self.loaded_models[mi].local_timestamp = ts;

                // Get transforms and apply
                let transforms = self.loaded_models[mi].motion.as_ref().unwrap()
                    .get_transforms_interpolated(ts, local_mirrored);

                // Apply world offset
                let offset = self.loaded_models[mi].world_offset;
                let transforms = if offset != Vec3::ZERO {
                    transforms.iter().map(|t| {
                        let mut m = *t;
                        m.w_axis.x += offset.x;
                        m.w_axis.y += offset.y;
                        m.w_axis.z += offset.z;
                        m
                    }).collect()
                } else {
                    transforms
                };

                // Update scene entity transforms
                let entity_ids = &self.loaded_models[mi].joint_entity_ids;
                for (i, &eid) in entity_ids.iter().enumerate() {
                    if i < transforms.len() {
                        self.scene.transforms[eid] = transforms[i];
                    }
                }

                // Update skinned mesh
                if let Some(ref mut skin) = self.loaded_models[mi].skinned_mesh {
                    skin.update_bones(&transforms);
                }

                // Update actor
                if let Some(ref mut actor) = self.loaded_models[mi].actor {
                    actor.set_pose_with_velocities(&transforms, dt);
                }

                // Draw skeleton for independent models
                if self.show_skeleton {
                    let positions: Vec<Vec3> = transforms.iter()
                        .map(|t| anim_math::transform::Transform::get_position(t))
                        .collect();
                    let motion = self.loaded_models[mi].motion.as_ref().unwrap();
                    // Slightly different color for non-active models
                    let color = [0.6, 0.7, 0.85, 0.8];
                    self.debug_draw.skeleton(
                        &positions,
                        &motion.hierarchy.parent_indices,
                        color,
                        0.008,
                    );
                }
            }
        }

        // ── Retargeted meshes ───────────────────────────────
        // Update any mesh that has a retarget binding to the active model
        {
            let timestamp = self.timestamp;
            let mirrored = self.mirrored;
            // Collect retarget info to avoid multiple borrows
            let mut retarget_updates: Vec<(usize, Vec<glam::Mat4>)> = Vec::new();
            for (i, asset) in self.loaded_models.iter().enumerate() {
                if let Some(ref binding) = asset.retarget {
                    let src_idx = binding.source_asset;
                    if src_idx < self.loaded_models.len() {
                        if let Some(ref motion) = self.loaded_models[src_idx].motion {
                            let source_transforms = motion.get_transforms_interpolated(
                                timestamp, mirrored
                            );
                            let retargeted = binding.map.apply(&source_transforms, None);
                            retarget_updates.push((i, retargeted));
                        }
                    }
                }
            }
            for (idx, transforms) in retarget_updates {
                if let Some(ref mut skin) = self.loaded_models[idx].skinned_mesh {
                    skin.update_bones(&transforms);
                }
                // Also update scene entities
                let entity_ids = &self.loaded_models[idx].joint_entity_ids;
                for (i, &eid) in entity_ids.iter().enumerate() {
                    if i < transforms.len() {
                        self.scene.transforms[eid] = transforms[i];
                    }
                }
            }
        }

        // ── Onion skinning ──────────────────────────────────
        if self.onion_skinning {
            if let Some(idx) = self.active_model {
                if let Some(motion) = self.loaded_models[idx].motion.as_ref() {
                    let dt = motion.delta_time();
                    let step = self.onion_step.max(1);

                    // Past frames (fading blue)
                    for i in 1..=self.onion_before {
                        let t = self.timestamp - (i * step) as f32 * dt;
                        if t < 0.0 { break; }
                        let alpha = 0.15 / i as f32;
                        let color = [0.3, 0.5, 1.0, alpha];
                        let transforms = motion.get_transforms_interpolated(t, self.mirrored);
                        let positions: Vec<Vec3> = transforms.iter()
                            .map(|t| anim_math::transform::Transform::get_position(t))
                            .collect();
                        self.debug_draw.skeleton(
                            &positions,
                            &motion.hierarchy.parent_indices,
                            color,
                            0.005,
                        );
                    }

                    // Future frames (fading red/orange)
                    for i in 1..=self.onion_after {
                        let t = self.timestamp + (i * step) as f32 * dt;
                        if t > motion.total_time() { break; }
                        let alpha = 0.15 / i as f32;
                        let color = [1.0, 0.5, 0.2, alpha];
                        let transforms = motion.get_transforms_interpolated(t, self.mirrored);
                        let positions: Vec<Vec3> = transforms.iter()
                            .map(|t| anim_math::transform::Transform::get_position(t))
                            .collect();
                        self.debug_draw.skeleton(
                            &positions,
                            &motion.hierarchy.parent_indices,
                            color,
                            0.005,
                        );
                    }
                }
            }
        }

        // ── Multi-selection highlight ───────────────────────
        for &eid in &self.multi_selection {
            if self.scene.selected == Some(eid) { continue; } // primary selection drawn by gizmo
            if eid < self.scene.transforms.len() {
                let pos = self.scene.get_position(eid);
                self.debug_draw.point(pos, 0.012, anim_render::debug_draw::colors::CYAN);
            }
        }

        self.grid_config.visible = self.show_grid;

        // Draw origin axes (SketchUp-style: RGB = XYZ)
        if self.show_axes {
            let axis_len = 1.0;
            let origin = Vec3::ZERO;
            self.debug_draw.line(origin, origin + Vec3::X * axis_len, anim_render::debug_draw::colors::RED);
            self.debug_draw.line(origin, origin + Vec3::Y * axis_len, anim_render::debug_draw::colors::GREEN);
            self.debug_draw.line(origin, origin + Vec3::Z * axis_len, anim_render::debug_draw::colors::BLUE);
        }

        // Draw transform gizmo on selected entity
        if self.show_gizmo {
            if let Some(sel_id) = self.scene.selected {
                let transform = self.scene.get_transform(sel_id);
                let pos = anim_math::transform::Transform::get_position(&transform);
                let gizmo_size = 0.3;

                match self.active_tool {
                    Tool::Move => {
                        // Axis arrows with color coding
                        let ax = [
                            (Vec3::X, anim_render::debug_draw::colors::RED),
                            (Vec3::Y, anim_render::debug_draw::colors::GREEN),
                            (Vec3::Z, anim_render::debug_draw::colors::BLUE),
                        ];
                        for (dir, color) in &ax {
                            let highlight = match (&self.gizmo_axis, dir) {
                                (GizmoAxis::X, d) if *d == Vec3::X => true,
                                (GizmoAxis::Y, d) if *d == Vec3::Y => true,
                                (GizmoAxis::Z, d) if *d == Vec3::Z => true,
                                _ => false,
                            };
                            let c = if highlight {
                                anim_render::debug_draw::colors::YELLOW
                            } else {
                                *color
                            };
                            self.debug_draw.line(pos, pos + *dir * gizmo_size, c);
                            // Arrow head lines
                            let tip = pos + *dir * gizmo_size;
                            let head = gizmo_size * 0.1;
                            let perp1 = if dir.x.abs() > 0.9 { Vec3::Y } else { Vec3::X };
                            let perp2 = dir.cross(perp1).normalize();
                            self.debug_draw.line(tip, tip - *dir * head + perp2 * head, c);
                            self.debug_draw.line(tip, tip - *dir * head - perp2 * head, c);
                        }
                        // Center point
                        self.debug_draw.point(pos, 0.015, anim_render::debug_draw::colors::WHITE);
                    }
                    Tool::Rotate => {
                        // Rotation rings
                        let segments = 32;
                        // XY ring (Z rotation) = blue
                        self.debug_draw.circle_xz(pos, gizmo_size, anim_render::debug_draw::colors::GREEN, segments);
                        // YZ ring (X rotation) = red — draw as circle in YZ
                        let step = std::f32::consts::TAU / segments as f32;
                        for i in 0..segments {
                            let a = i as f32 * step;
                            let b = (i + 1) as f32 * step;
                            let p1 = pos + Vec3::new(0.0, a.cos() * gizmo_size, a.sin() * gizmo_size);
                            let p2 = pos + Vec3::new(0.0, b.cos() * gizmo_size, b.sin() * gizmo_size);
                            self.debug_draw.line(p1, p2, anim_render::debug_draw::colors::RED);
                        }
                        // XZ ring (Y rotation) = green — already drawn by circle_xz, draw XY ring
                        for i in 0..segments {
                            let a = i as f32 * step;
                            let b = (i + 1) as f32 * step;
                            let p1 = pos + Vec3::new(a.cos() * gizmo_size, a.sin() * gizmo_size, 0.0);
                            let p2 = pos + Vec3::new(b.cos() * gizmo_size, b.sin() * gizmo_size, 0.0);
                            self.debug_draw.line(p1, p2, anim_render::debug_draw::colors::BLUE);
                        }
                    }
                    Tool::Select => {
                        // Selection highlight: point + transform gizmo
                        self.debug_draw.point(pos, 0.02, anim_render::debug_draw::colors::SELECTED);
                        self.debug_draw.transform_gizmo(transform, 0.15);
                    }
                    Tool::Measure => {
                        // Draw measurement line
                        if let Some(start) = self.measure.start {
                            self.debug_draw.point(start, 0.015, anim_render::debug_draw::colors::YELLOW);
                            if let Some(end) = self.measure.end {
                                self.debug_draw.line(start, end, anim_render::debug_draw::colors::YELLOW);
                                self.debug_draw.point(end, 0.015, anim_render::debug_draw::colors::YELLOW);
                            }
                        }
                    }
                    Tool::Ik => {
                        // IK tool: highlight end effector, draw IK chain
                        self.debug_draw.point(pos, 0.025, anim_render::debug_draw::colors::MAGENTA);

                        // Draw IK chain if both root and tip are set
                        if let (Some(root_eid), Some(tip_eid)) = (self.ik_chain_root, self.ik_chain_tip) {
                            let chain = self.scene.get_chain(root_eid, tip_eid);
                            for i in 0..chain.len() {
                                let p = self.scene.get_position(chain[i]);
                                self.debug_draw.point(p, 0.018, anim_render::debug_draw::colors::MAGENTA);
                                if i > 0 {
                                    let prev_p = self.scene.get_position(chain[i - 1]);
                                    self.debug_draw.line(prev_p, p, anim_render::debug_draw::colors::MAGENTA);
                                }
                            }
                        }

                        // Draw pole target indicator
                        if self.ik_use_pole_target {
                            let pole = self.ik_pole_position;
                            self.debug_draw.point(pole, 0.02, [1.0, 0.8, 0.2, 0.8]);
                            // Diamond shape around pole target
                            let s = 0.04;
                            self.debug_draw.line(pole + Vec3::X * s, pole + Vec3::Z * s, [1.0, 0.8, 0.2, 0.5]);
                            self.debug_draw.line(pole + Vec3::Z * s, pole - Vec3::X * s, [1.0, 0.8, 0.2, 0.5]);
                            self.debug_draw.line(pole - Vec3::X * s, pole - Vec3::Z * s, [1.0, 0.8, 0.2, 0.5]);
                            self.debug_draw.line(pole - Vec3::Z * s, pole + Vec3::X * s, [1.0, 0.8, 0.2, 0.5]);
                        }
                    }
                }

                // Always draw selection highlight on the selected bone
                // (brighter joint + highlight chain to parent)
                self.debug_draw.point(pos, 0.018, anim_render::debug_draw::colors::SELECTED);
                if let Some(parent_eid) = self.scene.get_entity(sel_id).parent {
                    let parent_pos = self.scene.get_position(parent_eid);
                    self.debug_draw.line(parent_pos, pos, anim_render::debug_draw::colors::SELECTED);
                }
            }
        }
    }

    // ── Multi-selection helpers ────────────────────────────

    /// Add an entity to the multi-selection (Ctrl+click).
    pub fn toggle_multi_select(&mut self, entity_id: usize) {
        if let Some(pos) = self.multi_selection.iter().position(|&id| id == entity_id) {
            self.multi_selection.remove(pos);
        } else {
            self.multi_selection.push(entity_id);
        }
        self.scene.selected = Some(entity_id);
    }

    /// Select a single entity, clearing multi-selection.
    pub fn select_single(&mut self, entity_id: usize) {
        self.multi_selection.clear();
        self.multi_selection.push(entity_id);
        self.scene.selected = Some(entity_id);
    }

    /// Select all joints of the active model.
    pub fn select_all(&mut self) {
        self.multi_selection.clear();
        if let Some(idx) = self.active_model {
            for &eid in &self.loaded_models[idx].joint_entity_ids {
                self.multi_selection.push(eid);
            }
        }
    }

    /// Select entity and all its children/successors.
    pub fn select_hierarchy(&mut self, entity_id: usize) {
        self.multi_selection.clear();
        self.multi_selection.push(entity_id);
        let successors = self.scene.get_entity(entity_id).successors.clone();
        for s in successors {
            self.multi_selection.push(s);
        }
        self.scene.selected = Some(entity_id);
    }

    /// Deselect all.
    pub fn deselect_all(&mut self) {
        self.multi_selection.clear();
        self.scene.selected = None;
    }

    /// Check if an entity is in the multi-selection.
    pub fn is_selected(&self, entity_id: usize) -> bool {
        self.multi_selection.contains(&entity_id)
    }

    // ── Pose copy/paste ──────────────────────────────────

    /// Copy current pose to clipboard.
    pub fn copy_pose(&mut self) {
        if let Some(idx) = self.active_model {
            let eids = &self.loaded_models[idx].joint_entity_ids;
            let transforms: Vec<glam::Mat4> = eids.iter()
                .map(|&eid| self.scene.get_transform(eid))
                .collect();
            let frame = self.current_frame();
            self.pose_clipboard = Some(PoseClipboard {
                transforms,
                source_frame: frame,
                joint_count: eids.len(),
            });
            self.log_info(&format!("Pose copiee (frame {})", frame));
        }
    }

    /// Paste clipboard pose onto current model.
    pub fn paste_pose(&mut self) {
        if let Some(ref clip) = self.pose_clipboard.clone() {
            if let Some(idx) = self.active_model {
                let eids = &self.loaded_models[idx].joint_entity_ids;
                if eids.len() == clip.joint_count {
                    for (i, &eid) in eids.iter().enumerate() {
                        if i < clip.transforms.len() {
                            self.scene.set_transform(eid, clip.transforms[i], false);
                        }
                    }
                    self.log_info(&format!("Pose collee (depuis frame {})", clip.source_frame));
                } else {
                    self.log_warn("Pose incompatible: nombre de joints different");
                }
            }
        }
    }

    /// Mirror the current pose (swap left/right).
    pub fn mirror_pose(&mut self) {
        if let Some(idx) = self.active_model {
            let eids = &self.loaded_models[idx].joint_entity_ids;
            let names = &self.loaded_models[idx].model.joint_names;

            // Build left/right pairs
            let mut pairs: Vec<(usize, usize)> = Vec::new();
            let mut matched = vec![false; eids.len()];

            for i in 0..names.len() {
                if matched[i] { continue; }
                let name = &names[i];

                // Try to find the mirror partner
                let mirror_name = if name.contains("Left") {
                    name.replace("Left", "Right")
                } else if name.contains("Right") {
                    name.replace("Right", "Left")
                } else if name.starts_with('L') && name.len() > 1 {
                    format!("R{}", &name[1..])
                } else if name.starts_with('R') && name.len() > 1 {
                    format!("L{}", &name[1..])
                } else {
                    continue;
                };

                if let Some(j) = names.iter().position(|n| *n == mirror_name) {
                    if !matched[j] {
                        pairs.push((i, j));
                        matched[i] = true;
                        matched[j] = true;
                    }
                }
            }

            // Swap transforms for each pair (reflect X)
            for (a, b) in &pairs {
                let eid_a = eids[*a];
                let eid_b = eids[*b];
                let t_a = self.scene.get_transform(eid_a);
                let t_b = self.scene.get_transform(eid_b);

                // Mirror: negate X position
                let mut mirrored_a = t_b;
                mirrored_a.w_axis.x = -mirrored_a.w_axis.x;
                let mut mirrored_b = t_a;
                mirrored_b.w_axis.x = -mirrored_b.w_axis.x;

                self.scene.set_transform(eid_a, mirrored_a, false);
                self.scene.set_transform(eid_b, mirrored_b, false);
            }

            self.log_info(&format!("Pose miroir ({} paires)", pairs.len()));
        }
    }

    /// Confirm rename of an entity.
    pub fn rename_entity_confirmed(&mut self) {
        if let Some(eid) = self.rename_entity.take() {
            let new_name = self.rename_buffer.trim().to_string();
            if !new_name.is_empty() {
                let old = self.scene.get_entity(eid).name.clone();
                self.scene.get_entity_mut(eid).name = new_name.clone();
                self.log_info(&format!("Renomme: {} → {}", old, new_name));
            }
            self.rename_buffer.clear();
        }
    }

    /// Record current scene transforms into the active motion frame (auto-key).
    pub fn record_auto_key(&mut self) {
        if !self.auto_key { return; }
        if let Some(idx) = self.active_model {
            let frame = self.current_frame();
            let eids = self.loaded_models[idx].joint_entity_ids.clone();
            let transforms: Vec<glam::Mat4> = eids.iter()
                .map(|&eid| self.scene.get_transform(eid))
                .collect();
            if let Some(ref mut motion) = self.loaded_models[idx].motion {
                motion.set_frame(frame, &transforms);
                self.log_info(&format!("Auto-key: frame {} enregistree", frame));
            }
        }
    }

    /// Record a single joint's transform into the current frame (auto-key).
    pub fn record_auto_key_joint(&mut self, entity_id: usize) {
        if !self.auto_key { return; }
        if let Some(idx) = self.active_model {
            let frame = self.current_frame();
            let eids = &self.loaded_models[idx].joint_entity_ids;
            if let Some(joint_idx) = eids.iter().position(|&eid| eid == entity_id) {
                let transform = self.scene.get_transform(entity_id);
                if let Some(ref mut motion) = self.loaded_models[idx].motion {
                    motion.set_joint_transform(frame, joint_idx, transform);
                }
            }
        }
    }

    /// Import a model from a file path.
    pub fn import_model(&mut self, model: ImportedModel) {
        let name = model.name.clone();
        let num_joints = model.joint_names.len();
        let num_frames = model.num_frames();

        // Create entities in scene for each joint
        let mut joint_entity_ids = Vec::new();
        for (i, joint_name) in model.joint_names.iter().enumerate() {
            let parent_eid = if model.parent_indices[i] >= 0 {
                Some(joint_entity_ids[model.parent_indices[i] as usize])
            } else {
                None
            };
            let eid = self.scene.add_entity(joint_name, None, None, parent_eid);
            joint_entity_ids.push(eid);
        }

        // Create motion
        let motion = Motion::from_imported(&model);

        // Create skinned mesh
        let skinned_mesh = model.skin.as_ref().map(|skin| {
            SkinnedMeshData::from_imported(&model.meshes, skin)
        });

        let has_mesh = skinned_mesh.is_some();
        let has_motion = motion.is_some();

        // Create Actor component from imported model
        let actor = Some(Actor::from_imported(&model));

        let asset = LoadedAsset {
            name: name.clone(),
            source_path: String::new(), // set by caller if loaded from file
            model,
            motion,
            skinned_mesh,
            joint_entity_ids,
            retarget: None,
            actor,
            independent_playback: false,
            local_timestamp: 0.0,
            local_speed: 1.0,
            local_looping: true,
            local_playing: false,
            local_mirrored: false,
            world_offset: Vec3::ZERO,
            visible: true,
        };

        self.loaded_models.push(asset);
        self.active_model = Some(self.loaded_models.len() - 1);
        self.timestamp = 0.0;

        // Auto-detect modules for the loaded motion
        if let Some(ref motion) = self.loaded_models.last().unwrap().motion {
            let contact = ContactModule::auto_detect(motion);
            let sensor_count = contact.sensor_count();
            self.contact_module = Some(contact);

            let guidance = GuidanceModule::auto_detect(motion);
            let guide_count = guidance.point_count();
            self.guidance_module = Some(guidance);

            let tracking = TrackingModule::auto_detect(motion);
            let track_count = tracking.joint_count();
            self.tracking_module = Some(tracking);

            // Root motion extraction
            let root = RootMotion::compute(motion, &self.root_config);
            self.root_motion = Some(root);

            // Phase detection (use the contact module we just stored)
            let phase = anim_animation::detect_phase(motion, self.contact_module.as_ref().unwrap());
            if phase.num_cycles() > 0 {
                self.log_info(&format!(
                    "Phase: {} cycles, {:.1} Hz, ~{:.0} frames/cycle",
                    phase.num_cycles(), phase.frequency, phase.avg_cycle_length
                ));
            }
            self.phase_data = Some(phase);

            if sensor_count > 0 || guide_count > 0 || track_count > 0 {
                self.log_info(&format!(
                    "Modules: {} contacts, {} guidance, {} tracking",
                    sensor_count, guide_count, track_count
                ));
            }
        }

        self.log_info(&format!(
            "Charge: {} ({} os, {} images{}{})",
            name, num_joints, num_frames,
            if has_mesh { ", maillage" } else { "" },
            if has_motion { ", animation" } else { "" },
        ));
    }

    /// Import a model loaded from a file, storing the source path for save/reload.
    pub fn import_model_from_path(&mut self, model: ImportedModel, path: &std::path::Path) {
        self.import_model(model);
        // Tag the just-imported asset with its source path
        if let Some(asset) = self.loaded_models.last_mut() {
            asset.source_path = path.to_string_lossy().to_string();
        }
        // Signal for recent files tracking (picked up by main loop)
        self.last_imported_path = Some(path.to_path_buf());
    }

    /// Bind a mesh asset to an animation source (retargeting).
    /// `mesh_idx`: the asset with the GLB mesh (target).
    /// `anim_idx`: the asset with the animation data (source).
    pub fn retarget_mesh(&mut self, mesh_idx: usize, anim_idx: usize) {
        if mesh_idx >= self.loaded_models.len() || anim_idx >= self.loaded_models.len() {
            self.log_error("Indices de modeles invalides pour le retargeting");
            return;
        }
        if mesh_idx == anim_idx {
            self.log_warn("Le mesh et l'animation sont le meme modele");
            return;
        }

        let source_names = &self.loaded_models[anim_idx].model.joint_names;
        let target_names = &self.loaded_models[mesh_idx].model.joint_names;

        let map = anim_animation::build_retarget(source_names, target_names);
        let quality = map.quality();
        let mapped = map.mapped_count;
        let total = target_names.len();
        let unmapped = map.unmapped_targets().iter()
            .take(5)
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        self.loaded_models[mesh_idx].retarget = Some(RetargetBinding {
            source_asset: anim_idx,
            map,
        });

        self.log_info(&format!(
            "Retarget: {} ← {} ({}/{} joints, {:.0}% match{})",
            self.loaded_models[mesh_idx].name,
            self.loaded_models[anim_idx].name,
            mapped, total, quality * 100.0,
            if unmapped.is_empty() { String::new() }
            else { format!(", manquants: {}", unmapped.join(", ")) }
        ));
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
