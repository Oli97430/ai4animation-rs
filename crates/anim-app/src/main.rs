//! AI4Animation Rust Engine - Main application.
//!
//! Professional 3D animation editor with GLB/BVH import, skeletal animation,
//! timeline playback, IK, and AI-powered motion synthesis.

use eframe::egui;
use anim_gui::AppState;
use anim_gui::panels;
use anim_render::SceneRenderer;
use anim_import::AssetManager;
use anim_ai::AiCommand;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("AI4Animation Engine")
            .with_inner_size([1600.0, 900.0])
            .with_min_inner_size([1024.0, 600.0])
            .with_drag_and_drop(true),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "AI4Animation Engine",
        options,
        Box::new(|cc| {
            // Apply professional dark theme
            anim_gui::apply_theme(&cc.egui_ctx);

            // Configure Chinese font support
            let fonts = egui::FontDefinitions::default();
            // egui includes basic CJK support; for full Chinese we'd add a font here
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(AnimApp::new(cc)))
        }),
    )
}

struct AnimApp {
    state: AppState,
    renderer: Option<SceneRenderer>,
    last_instant: std::time::Instant,
    // Render texture for 3D viewport
    viewport_texture: Option<ViewportTexture>,
    // Profiler state
    profiler: panels::profiler::ProfilerState,
    // Batch converter state
    batch: panels::batch_panel::BatchState,
    // Video recorder state
    recorder: panels::recorder::RecorderState,
    // Asset browser state
    asset_browser: panels::asset_browser::AssetBrowserState,
    // Asset manager with caching
    asset_manager: AssetManager,
    // Keyboard shortcuts
    shortcuts: anim_gui::ShortcutMap,
    shortcut_editor: panels::shortcut_editor::ShortcutEditorState,
    show_shortcut_editor: bool,
    // AI chat
    ai_chat: panels::ai_chat::AiChatState,
    // Training panel
    training_panel: panels::training::TrainingPanel,
    // Motion matching panel
    motion_matching_panel: panels::motion_matching::MotionMatchingPanel,
    // State machine editor
    state_machine_panel: panels::state_machine_editor::StateMachinePanel,
    // Pose editor
    pose_editor_panel: panels::pose_editor::PoseEditorPanel,
    // Blend tree editor
    blend_tree_panel: panels::blend_tree_editor::BlendTreePanel,
    // Graph editor (curve viewer)
    graph_editor_panel: panels::graph_editor::GraphEditorPanel,
    // Ragdoll physics panel
    ragdoll_panel: panels::ragdoll_panel::RagdollPanel,
    // DeepPhase panel
    deep_phase_panel: panels::deep_phase_panel::DeepPhasePanel,
    // Animation recorder panel
    anim_recorder_panel: panels::anim_recorder_panel::AnimRecorderPanel,
    // Material editor panel
    material_editor_panel: panels::material_editor::MaterialEditorPanel,
    // Cloth panel
    cloth_panel: panels::cloth_panel::ClothPanel,
    // IK panel
    ik_panel: panels::ik_panel::IkPanel,
    // Recent files
    recent_files: anim_gui::recent_files::RecentFiles,
    // Auto-save
    auto_save: anim_gui::recent_files::AutoSave,
}

struct ViewportTexture {
    texture_id: egui::TextureId,
    size: (u32, u32),
    #[allow(dead_code)]
    wgpu_texture: wgpu::Texture,
    wgpu_view: wgpu::TextureView,
}

impl AnimApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut state = AppState::new();
        state.log_info("AI4Animation Engine démarré");
        state.log_info("Utilisez Fichier > Importer pour charger un modèle GLB ou BVH");

        let renderer = if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let device = &render_state.device;
            let format = render_state.target_format;
            Some(SceneRenderer::new(device, format))
        } else {
            state.log_error("wgpu non disponible - rendu 3D désactivé");
            None
        };

        Self {
            state,
            renderer,
            last_instant: std::time::Instant::now(),
            viewport_texture: None,
            profiler: panels::profiler::ProfilerState::default(),
            batch: panels::batch_panel::BatchState::default(),
            recorder: panels::recorder::RecorderState::default(),
            asset_browser: panels::asset_browser::AssetBrowserState::default(),
            asset_manager: AssetManager::new(),
            shortcuts: anim_gui::ShortcutMap::default(),
            shortcut_editor: panels::shortcut_editor::ShortcutEditorState::default(),
            show_shortcut_editor: false,
            ai_chat: panels::ai_chat::AiChatState::default(),
            training_panel: panels::training::TrainingPanel::default(),
            motion_matching_panel: panels::motion_matching::MotionMatchingPanel::default(),
            state_machine_panel: panels::state_machine_editor::StateMachinePanel::default(),
            pose_editor_panel: panels::pose_editor::PoseEditorPanel::default(),
            blend_tree_panel: panels::blend_tree_editor::BlendTreePanel::default(),
            graph_editor_panel: panels::graph_editor::GraphEditorPanel::default(),
            ragdoll_panel: panels::ragdoll_panel::RagdollPanel::default(),
            deep_phase_panel: panels::deep_phase_panel::DeepPhasePanel::default(),
            anim_recorder_panel: panels::anim_recorder_panel::AnimRecorderPanel::default(),
            material_editor_panel: panels::material_editor::MaterialEditorPanel::default(),
            cloth_panel: panels::cloth_panel::ClothPanel::default(),
            ik_panel: panels::ik_panel::IkPanel::default(),
            recent_files: anim_gui::recent_files::RecentFiles::load(),
            auto_save: anim_gui::recent_files::AutoSave::default(),
        }
    }

    fn ensure_viewport_texture(
        &mut self,
        render_state: &egui_wgpu::RenderState,
        _painter: &egui::Painter,
        width: u32,
        height: u32,
    ) {
        let needs_recreate = match &self.viewport_texture {
            Some(vt) => vt.size != (width, height),
            None => true,
        };

        if needs_recreate && width > 0 && height > 0 {
            // Clean up old texture
            if let Some(old) = self.viewport_texture.take() {
                render_state.renderer.write().free_texture(&old.texture_id);
            }

            let device = &render_state.device;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("viewport_color"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: render_state.target_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());

            let texture_id = render_state.renderer.write().register_native_texture(
                device,
                &view,
                wgpu::FilterMode::Linear,
            );

            self.viewport_texture = Some(ViewportTexture {
                texture_id,
                size: (width, height),
                wgpu_texture: texture,
                wgpu_view: view,
            });
        }
    }

    fn render_3d_scene(&mut self, render_state: &egui_wgpu::RenderState) {
        let renderer = match &mut self.renderer {
            Some(r) => r,
            None => return,
        };
        let vt = match &self.viewport_texture {
            Some(vt) => vt,
            None => return,
        };

        let device = &render_state.device;
        let queue = &render_state.queue;
        let (width, height) = vt.size;

        // Collect skinned meshes to render
        let skinned_meshes: Vec<&anim_render::skinned_mesh::SkinnedMeshData> = self.state.loaded_models
            .iter()
            .filter_map(|a| a.skinned_mesh.as_ref())
            .collect();

        let show_mesh = self.state.show_mesh;
        let meshes_to_render: Vec<&anim_render::skinned_mesh::SkinnedMeshData> = if show_mesh {
            skinned_meshes
        } else {
            vec![]
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("3d_encoder"),
        });

        renderer.render(
            device,
            queue,
            &mut encoder,
            &vt.wgpu_view,
            width,
            height,
            &self.state.camera,
            &self.state.debug_draw,
            &self.state.grid_config,
            &meshes_to_render,
            &self.state.render_settings,
        );

        queue.submit(std::iter::once(encoder.finish()));
    }
}

impl eframe::App for AnimApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Calculate delta time
        let now = std::time::Instant::now();
        let dt = (now - self.last_instant).as_secs_f32();
        self.last_instant = now;

        // Update animation
        self.state.update(dt);

        // Motion matching controller update
        if self.state.motion_matching_controller.active && self.state.motion_database.built {
            // Build a query from current state
            let query = if let Some(idx) = self.state.active_model {
                if let Some(ref motion) = self.state.loaded_models[idx].motion {
                    let transforms = motion.get_transforms_interpolated(self.state.timestamp, self.state.mirrored);
                    let positions: Vec<glam::Vec3> = transforms.iter()
                        .map(|t| anim_math::transform::Transform::get_position(t))
                        .collect();
                    let velocities = motion.get_velocities(self.state.timestamp, self.state.mirrored);
                    let root_pos = positions.first().copied().unwrap_or(glam::Vec3::ZERO);
                    let root_dir = if !transforms.is_empty() {
                        glam::Vec3::new(transforms[0].z_axis.x, 0.0, transforms[0].z_axis.z)
                            .normalize_or_zero()
                    } else {
                        glam::Vec3::Z
                    };
                    // Build trajectory from current animation (future samples)
                    let traj_cfg = &self.state.motion_database.trajectory_config;
                    let mut trajectory = Vec::new();
                    let _anim_dt = motion.delta_time();
                    for s in 0..traj_cfg.num_samples {
                        let future_t = self.state.timestamp + (s + 1) as f32 * traj_cfg.sample_interval;
                        let future_t_clamped = future_t.min(motion.total_time());
                        let ft = motion.get_transforms_interpolated(future_t_clamped, self.state.mirrored);
                        let fp = if !ft.is_empty() {
                            anim_math::transform::Transform::get_position(&ft[0])
                        } else {
                            root_pos
                        };
                        let fd = if !ft.is_empty() {
                            glam::Vec3::new(ft[0].z_axis.x, 0.0, ft[0].z_axis.z).normalize_or_zero()
                        } else {
                            root_dir
                        };
                        trajectory.push((fp, fd));
                    }
                    Some(self.state.motion_database.build_query(
                        &positions, &velocities, root_pos, root_dir,
                        &trajectory, false, false,
                    ))
                } else { None }
            } else { None };

            if let Some(query_features) = query {
                if let Some(pose) = self.state.motion_matching_controller.update(
                    &self.state.motion_database, dt, &query_features,
                ) {
                    // Apply the matched pose to the scene
                    if let Some(idx) = self.state.active_model {
                        let entity_ids = &self.state.loaded_models[idx].joint_entity_ids;
                        for (i, &eid) in entity_ids.iter().enumerate() {
                            if i < pose.len() {
                                self.state.scene.transforms[eid] = pose[i];
                            }
                        }
                        if let Some(ref mut skin) = self.state.loaded_models[idx].skinned_mesh {
                            skin.update_bones(&pose);
                        }
                        if let Some(ref mut actor) = self.state.loaded_models[idx].actor {
                            actor.set_pose_with_velocities(&pose, dt);
                        }
                    }
                }
            }
        }

        // State machine update
        {
            let sm_event = if let Some(ref mut sm) = self.state.state_machine {
                let anim_ended = if let Some(idx) = self.state.active_model {
                    if let Some(ref motion) = self.state.loaded_models[idx].motion {
                        !self.state.looping && self.state.timestamp >= motion.total_time()
                    } else { false }
                } else { false };
                sm.update(dt, anim_ended)
            } else { None };

            // Process state change event (borrow on state_machine is released)
            if let Some(event) = sm_event {
                // Read the new state's info
                let (model_idx, state_name) = if let Some(ref sm) = self.state.state_machine {
                    let st = &sm.states[event.to];
                    let mi = match &st.motion_source {
                        anim_animation::state_machine::MotionSource::Clip { model_index } => Some(*model_index),
                        _ => None,
                    };
                    (mi, st.name.clone())
                } else {
                    (None, String::new())
                };

                if let Some(idx) = model_idx {
                    if idx < self.state.loaded_models.len() {
                        self.state.start_crossfade();
                        self.state.active_model = Some(idx);
                        self.state.timestamp = 0.0;
                        self.state.log_info(&format!(
                            "[SM] Transition complète -> {}", state_name
                        ));
                    }
                }
            }
        }

        // Blend tree evaluation
        if let Some(ref bt) = self.state.blend_tree {
            if bt.num_nodes() > 0 {
                // Build the get_pose closure that reads from loaded models
                let timestamp = self.state.timestamp;
                let mirrored = self.state.mirrored;
                let get_pose = |model_idx: usize| -> Option<Vec<glam::Mat4>> {
                    self.state.loaded_models.get(model_idx)
                        .and_then(|asset| asset.motion.as_ref())
                        .map(|motion| motion.get_transforms_interpolated(timestamp, mirrored))
                };
                let result = bt.evaluate(&get_pose);
                if let Some(blend_result) = result {
                    // Apply blended pose to the active model
                    if let Some(idx) = self.state.active_model {
                        let entity_ids = &self.state.loaded_models[idx].joint_entity_ids;
                        for (i, &eid) in entity_ids.iter().enumerate() {
                            if i < blend_result.pose.len() {
                                self.state.scene.transforms[eid] = blend_result.pose[i];
                            }
                        }
                        if let Some(ref mut skin) = self.state.loaded_models[idx].skinned_mesh {
                            skin.update_bones(&blend_result.pose);
                        }
                        if let Some(ref mut actor) = self.state.loaded_models[idx].actor {
                            actor.set_pose_with_velocities(&blend_result.pose, dt);
                        }
                    }
                }
            }
        }

        // Ragdoll physics step
        if let Some(ref mut ragdoll) = self.state.ragdoll {
            ragdoll.step(dt);
            // Apply ragdoll transforms to the active model's scene entities
            if ragdoll.active {
                if let Some(idx) = self.state.active_model {
                    let transforms = ragdoll.get_transforms();
                    let entity_ids = &self.state.loaded_models[idx].joint_entity_ids;
                    for (i, &eid) in entity_ids.iter().enumerate() {
                        if i < transforms.len() {
                            self.state.scene.transforms[eid] = transforms[i];
                        }
                    }
                    if let Some(ref mut skin) = self.state.loaded_models[idx].skinned_mesh {
                        skin.update_bones(&transforms);
                    }
                }
            }
        }

        // Cloth / soft-body step
        if let Some(ref mut cloth) = self.state.cloth_sim {
            cloth.step(dt);
        }

        // Animation recorder: capture transforms each frame while recording
        if self.anim_recorder_panel.recorder.is_recording() {
            if let Some(idx) = self.state.active_model {
                let entity_ids = &self.state.loaded_models[idx].joint_entity_ids;
                let transforms: Vec<glam::Mat4> = entity_ids.iter()
                    .map(|&eid| self.state.scene.transforms[eid])
                    .collect();
                self.anim_recorder_panel.recorder.capture_frame(&transforms, dt);
            }
        }

        self.profiler.push_frame_time(dt);

        // Poll AI for completed responses + execute any commands
        if let Some(commands) = self.ai_chat.session.poll() {
            for cmd in commands {
                execute_ai_command(cmd, &mut self.state, &mut self.asset_manager);
            }
        }
        // Also drain pending commands from the chat state
        if !self.ai_chat.pending_commands.is_empty() {
            let cmds: Vec<AiCommand> = self.ai_chat.pending_commands.drain(..).collect();
            for cmd in cmds {
                execute_ai_command(cmd, &mut self.state, &mut self.asset_manager);
            }
        }

        // Drain background task log messages (training, conversion, etc.)
        {
            let messages: Vec<String> = {
                let mut q = self.state.bg_log_queue.lock().unwrap();
                q.drain(..).collect()
            };
            for msg in messages {
                if msg.ends_with("DONE") {
                    self.state.training_active = false;
                } else {
                    self.state.log_info(&msg);
                }
            }
        }

        // Track newly imported files in recent list
        if let Some(path) = self.state.last_imported_path.take() {
            self.recent_files.add(&path);
        }

        // Sync recent files for menu display
        self.state.recent_files_display = self.recent_files.entries.iter()
            .map(|e| (e.path.clone(), e.name.clone(), e.file_type.clone()))
            .collect();

        // Handle pending recent file import
        if let Some(path) = self.state.pending_recent_import.take() {
            if path.extension().map(|e| e == "a4a").unwrap_or(false) {
                // It's a project file — load as project
                self.state.pending_project_load = Some(path);
            } else {
                // It's a model file — import directly
                match self.asset_manager.load(&path) {
                    Ok(model) => {
                        self.state.import_model_from_path(model, &path);
                        self.recent_files.add(&path);
                    }
                    Err(e) => self.state.log_error(&format!("Erreur import récent: {:#}", e)),
                }
            }
        }

        // Auto-save
        if self.auto_save.tick(dt) && !self.state.loaded_models.is_empty() {
            let backup = self.auto_save.get_backup_path();
            match anim_gui::scene_io::save_project(&backup, &self.state) {
                Ok(()) => self.state.log_info(&format!("Auto-sauvegarde: {}", backup.display())),
                Err(e) => self.state.log_error(&format!("Erreur auto-save: {}", e)),
            }
        }

        // Handle pending project load (deferred from menu bar)
        if let Some(project_path) = self.state.pending_project_load.take() {
            match anim_gui::scene_io::load_project(&project_path) {
                Ok(project) => {
                    // Clear current scene
                    self.state.loaded_models.clear();
                    self.state.active_model = None;
                    self.state.scene = anim_core::Scene::new();

                    // Re-import model files
                    let loaded = anim_gui::scene_io::reload_models(
                        &mut self.state, &project, &mut self.asset_manager
                    );

                    // Apply saved settings (camera, display, render, panels)
                    anim_gui::scene_io::apply_project(&mut self.state, &project);

                    self.recent_files.add(&project_path);
                    self.state.log_info(&format!(
                        "Projet chargé: {} ({} modèle(s) rechargé(s))",
                        project_path.display(), loaded
                    ));
                }
                Err(e) => {
                    self.state.log_error(&format!("Erreur chargement projet: {}", e));
                }
            }
        }

        // Handle pending export frame (AI command or menu)
        if let Some(export_path) = self.state.pending_export_frame.take() {
            if let (Some(ref vt), Some(render_state)) = (&self.viewport_texture, frame.wgpu_render_state()) {
                let (w, h) = vt.size;
                let p = std::path::PathBuf::from(&export_path);
                match anim_render::capture_texture_to_png(
                    &render_state.device, &render_state.queue,
                    &vt.wgpu_texture, w, h, &p,
                ) {
                    Ok(()) => self.state.log_info(&format!("Frame exportée: {}", export_path)),
                    Err(e) => self.state.log_error(&format!("Erreur export: {}", e)),
                }
            } else {
                self.state.log_error("Export impossible: pas de viewport actif");
            }
        }

        // Recorder: tick and capture viewport frames as PNG
        if self.recorder.tick(dt) {
            if let (Some(ref vt), Some(render_state)) = (&self.viewport_texture, frame.wgpu_render_state()) {
                let path = self.recorder.frame_path();
                let (w, h) = vt.size;
                match anim_render::capture_texture_to_png(
                    &render_state.device, &render_state.queue,
                    &vt.wgpu_texture, w, h, &path,
                ) {
                    Ok(()) => {} // silent on success — logging every frame is too noisy
                    Err(e) => {
                        self.state.log_error(&format!("Capture echouee: {}", e));
                        self.recorder.stop();
                    }
                }
            }
        }

        // Sync asset browser visibility
        self.asset_browser.visible = self.state.show_asset_browser;
        // Sync shortcut editor visibility
        if self.state.show_shortcut_editor && !self.show_shortcut_editor {
            self.show_shortcut_editor = true;
            self.state.show_shortcut_editor = false; // one-shot trigger
        }

        // Handle file drag-and-drop (GLB/BVH/FBX dropped onto window)
        let dropped_paths: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw.dropped_files.iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped_paths {
            match self.asset_manager.load(&path) {
                Ok(model) => {
                    self.state.import_model_from_path(model, &path);
                    self.recent_files.add(&path);
                }
                Err(e) => self.state.log_error(&format!("Erreur: {:#}", e)),
            }
        }

        // ── Top: Menu Bar ────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(44, 46, 56))
                .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 63, 76)))
                .inner_margin(egui::Margin::symmetric(6.0, 4.0))
            )
            .show(ctx, |ui| {
                panels::menu_bar::show(ui, &mut self.state);
            });

        // ── Bottom: Timeline + Console ──────────────────────
        // Calculate desired bottom panel height based on visible sub-panels,
        // but cap it so the 3D viewport always has at least 200px.
        let screen_height = ctx.screen_rect().height();
        let bottom_desired: f32 = {
            let mut h = 56.0; // timeline always visible
            if self.state.show_dope_sheet { h += 120.0; }
            if self.state.show_motion_editor { h += 60.0; }
            if self.state.show_recorder { h += 30.0; }
            if self.state.show_console { h += 80.0; }
            if self.state.show_profiler { h += 100.0; }
            if self.state.show_batch { h += 200.0; }
            if self.state.show_asset_browser { h += 220.0; }
            h
        };
        // Never let the bottom panel take more than 60% of the window
        let bottom_max = (screen_height * 0.6).max(56.0);
        let bottom_height = bottom_desired.min(bottom_max);

        egui::TopBottomPanel::bottom("bottom_panel")
            .default_height(bottom_height)
            .height_range(56.0..=bottom_max)
            .resizable(true)
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(24, 25, 30))
                .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(40, 42, 52)))
                .inner_margin(egui::Margin::symmetric(6.0, 4.0))
            )
            .show(ctx, |ui| {
                panels::timeline::show(ui, &mut self.state);

                if self.state.show_dope_sheet {
                    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 58)),
                    );
                    panels::dope_sheet::show(ui, &mut self.state);
                }

                if self.state.show_motion_editor {
                    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 58)),
                    );
                    panels::motion_editor::show(ui, &mut self.state);
                }

                if self.state.show_recorder {
                    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 58)),
                    );
                    panels::recorder::show(ui, &mut self.recorder, &mut self.state);
                }

                if self.state.show_console {
                    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 58)),
                    );
                    panels::console::show(ui, &mut self.state);
                }

                if self.state.show_profiler {
                    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 58)),
                    );
                    panels::profiler::show(ui, &self.state, &self.profiler);
                }

                if self.state.show_batch {
                    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 58)),
                    );
                    panels::batch_panel::show(ui, &mut self.batch, &mut self.state);
                }

                if self.asset_browser.visible {
                    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 58)),
                    );
                    panels::asset_browser::show_with_manager(
                        ui, &mut self.asset_browser, &mut self.state,
                        Some(&mut self.asset_manager),
                    );
                }
            });

        // ── Left: Scene Hierarchy ───────────────────────────
        egui::SidePanel::left("hierarchy_panel")
            .default_width(230.0)
            .min_width(160.0)
            .max_width(400.0)
            .resizable(true)
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(27, 28, 33))
                .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(40, 42, 52)))
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
            )
            .show(ctx, |ui| {
                panels::hierarchy::show(ui, &mut self.state);
            });

        // ── Right: Inspector + Render Settings ──────────────
        egui::SidePanel::right("inspector_panel")
            .default_width(280.0)
            .min_width(200.0)
            .resizable(true)
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(27, 28, 33))
                .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(40, 42, 52)))
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    panels::inspector::show(ui, &mut self.state);

                    if self.state.show_render_settings {
                        ui.add_space(8.0);
                        panels::render_settings::show(ui, &mut self.state);
                    }
                });
            });

        // Central panel: 3D Viewport
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (response, _painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());

            // Handle viewport camera input
            panels::viewport::handle_input(ui, &response, &mut self.state);

            // Render 3D scene to texture
            if let Some(render_state) = frame.wgpu_render_state() {
                let width = (available.x * ctx.pixels_per_point()) as u32;
                let height = (available.y * ctx.pixels_per_point()) as u32;

                self.ensure_viewport_texture(render_state, &_painter, width, height);
                self.render_3d_scene(render_state);

                // Display the rendered texture
                if let Some(ref vt) = self.viewport_texture {
                    let uv = egui::Rect::from_min_max(
                        egui::pos2(0.0, 0.0),
                        egui::pos2(1.0, 1.0),
                    );
                    _painter.image(
                        vt.texture_id,
                        response.rect,
                        uv,
                        egui::Color32::WHITE,
                    );
                }

                // Camera overlay (top-left)
                let overlay_rect = egui::Rect::from_min_size(
                    response.rect.min + egui::vec2(8.0, 8.0),
                    egui::vec2(220.0, 30.0),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(overlay_rect), |ui| {
                    panels::viewport::camera_overlay(ui, &mut self.state);
                });

                // Toolbar overlay (top-center)
                let toolbar_width = 420.0;
                let toolbar_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        response.rect.center().x - toolbar_width * 0.5,
                        response.rect.min.y + 6.0,
                    ),
                    egui::vec2(toolbar_width, 28.0),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
                    panels::viewport::toolbar_overlay(ui, &mut self.state);
                });

                // Stats overlay (bottom-left)
                let stats_rect = egui::Rect::from_min_size(
                    egui::pos2(response.rect.min.x + 8.0, response.rect.max.y - 52.0),
                    egui::vec2(300.0, 48.0),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(stats_rect), |ui| {
                    panels::viewport::stats_overlay(ui, &self.state, width, height);
                });

                // Logo/splash when no models loaded
                if self.state.loaded_models.is_empty() {
                    let logo_rect = egui::Rect::from_min_size(
                        response.rect.min,
                        response.rect.size(),
                    );
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(logo_rect), |ui| {
                        panels::viewport::logo_overlay(ui, &self.state);
                    });
                }

                // Orientation compass (bottom-right)
                let compass_size = 60.0;
                let compass_rect = egui::Rect::from_min_size(
                    egui::pos2(response.rect.max.x - compass_size - 8.0, response.rect.max.y - compass_size - 8.0),
                    egui::vec2(compass_size, compass_size),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(compass_rect), |ui| {
                    panels::viewport::compass_overlay(ui, &self.state);
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Rendu 3D non disponible");
                });
            }
        });

        // Keyboard shortcuts (configurable via ShortcutMap)
        {
            use anim_gui::Action;
            let shortcuts = &self.shortcuts;
            let mut actions: Vec<Action> = Vec::new();
            ctx.input(|i| {
                for &action in Action::all() {
                    if shortcuts.pressed(action, i) {
                        actions.push(action);
                    }
                }
            });
            for action in actions {
                match action {
                    Action::Save => {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Projet AI4Anim", &["a4a"])
                            .set_file_name("project.a4a")
                            .save_file()
                        {
                            match anim_gui::scene_io::save_project(&path, &self.state) {
                                Ok(()) => {
                                    self.recent_files.add(&path);
                                    self.state.log_info(&format!("Sauvegardé: {}", path.display()));
                                }
                                Err(e) => self.state.log_error(&format!("Erreur: {:#}", e)),
                            }
                        }
                    }
                    Action::OpenProject => {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Projet AI4Anim", &["a4a"])
                            .pick_file()
                        {
                            self.state.pending_project_load = Some(path);
                        }
                    }
                    Action::Undo => self.state.undo(),
                    Action::Redo => self.state.redo(),
                    Action::PlayPause => self.state.playing = !self.state.playing,
                    Action::GoToStart => self.state.timestamp = 0.0,
                    Action::GoToEnd => self.state.timestamp = self.state.total_time(),
                    Action::PrevFrame => {
                        if let Some(motion) = self.state.active_motion() {
                            let dt = motion.delta_time();
                            self.state.timestamp = (self.state.timestamp - dt).max(0.0);
                        }
                    }
                    Action::NextFrame => {
                        if let Some(motion) = self.state.active_motion() {
                            let dt = motion.delta_time();
                            let total = motion.total_time();
                            self.state.timestamp = (self.state.timestamp + dt).min(total);
                        }
                    }
                    Action::ResetCamera => self.state.camera.reset(),
                    Action::FocusModel => {
                        if let Some(idx) = self.state.active_model {
                            if !self.state.loaded_models[idx].joint_entity_ids.is_empty() {
                                let root_eid = self.state.loaded_models[idx].joint_entity_ids[0];
                                let pos = anim_math::transform::Transform::get_position(
                                    &self.state.scene.transforms[root_eid]
                                );
                                self.state.camera.look_at(pos);
                            }
                        }
                    }
                    Action::ToggleLoop => self.state.looping = !self.state.looping,
                    Action::ToggleMirror => self.state.mirrored = !self.state.mirrored,
                    Action::ToggleGrid => self.state.show_grid = !self.state.show_grid,
                    Action::ToolSelect => self.state.active_tool = anim_gui::app_state::Tool::Select,
                    Action::ToolMove => self.state.active_tool = anim_gui::app_state::Tool::Move,
                    Action::ToolRotate => self.state.active_tool = anim_gui::app_state::Tool::Rotate,
                    Action::ToolMeasure => {
                        self.state.active_tool = anim_gui::app_state::Tool::Measure;
                        self.state.measure.reset();
                    }
                    Action::ToolIk => {
                        self.state.active_tool = anim_gui::app_state::Tool::Ik;
                        self.state.ik_chain_root = None;
                        self.state.ik_chain_tip = None;
                    }
                    Action::ResetTool => {
                        self.state.active_tool = anim_gui::app_state::Tool::Select;
                        self.state.gizmo_axis = anim_gui::app_state::GizmoAxis::None;
                        self.state.measure.reset();
                        self.state.ik_chain_root = None;
                        self.state.ik_chain_tip = None;
                    }
                    Action::AxisX => self.state.gizmo_axis = anim_gui::app_state::GizmoAxis::X,
                    Action::AxisY => self.state.gizmo_axis = anim_gui::app_state::GizmoAxis::Y,
                    Action::AxisZ => self.state.gizmo_axis = anim_gui::app_state::GizmoAxis::Z,
                    Action::SelectAll => self.state.select_all(),
                    Action::ViewFront => self.state.camera.view_front(),
                    Action::ViewRight => self.state.camera.view_right(),
                    Action::ViewTop => self.state.camera.view_top(),
                    Action::ToggleOnionSkin => self.state.onion_skinning = !self.state.onion_skinning,
                    Action::ToggleDopeSheet => self.state.show_dope_sheet = !self.state.show_dope_sheet,
                    Action::CopyPose => self.state.copy_pose(),
                    Action::PastePose => self.state.paste_pose(),
                    Action::MirrorPose => self.state.mirror_pose(),
                }
            }
        }

        // Sync AI chat visibility
        self.ai_chat.visible = self.state.show_ai_chat;

        // Floating AI Chat window
        if self.ai_chat.visible {
            let mut visible = self.ai_chat.visible;
            egui::Window::new("🤖 IA Assistant")
                .open(&mut visible)
                .default_size([400.0, 500.0])
                .min_width(300.0)
                .min_height(200.0)
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 420.0, 60.0])
                .show(ctx, |ui| {
                    panels::ai_chat::show(ui, &mut self.ai_chat, &mut self.state);
                });
            self.ai_chat.visible = visible;
            self.state.show_ai_chat = visible;
        }

        // Floating shortcut editor window
        if self.show_shortcut_editor {
            egui::Window::new("Raccourcis clavier")
                .open(&mut self.show_shortcut_editor)
                .default_size([400.0, 500.0])
                .resizable(true)
                .show(ctx, |ui| {
                    panels::shortcut_editor::show(ui, &mut self.shortcuts, &mut self.shortcut_editor);
                });
        }

        // Floating training panel
        if self.state.show_training {
            let mut visible = true;
            egui::Window::new("🧠 Entraînement")
                .open(&mut visible)
                .default_size([360.0, 400.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 420.0, 200.0])
                .show(ctx, |ui| {
                    panels::training::show(ui, &mut self.state, &mut self.training_panel);
                });
            self.state.show_training = visible;
        }

        // Floating state machine editor
        if self.state.show_state_machine {
            let mut visible = true;
            egui::Window::new("Machine d'états")
                .open(&mut visible)
                .default_size([600.0, 450.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([200.0, 100.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::state_machine_editor::show(ui, &mut self.state, &mut self.state_machine_panel);
                    });
                });
            self.state.show_state_machine = visible;
        }

        // Floating pose editor
        if self.state.show_pose_editor {
            let mut visible = true;
            egui::Window::new("Édition de pose")
                .open(&mut visible)
                .default_size([320.0, 380.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 340.0, 60.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::pose_editor::show(ui, &mut self.state, &mut self.pose_editor_panel);
                    });
                });
            self.state.show_pose_editor = visible;
        }

        // Floating motion matching panel
        if self.state.show_motion_matching {
            let mut visible = true;
            egui::Window::new("Motion Matching")
                .open(&mut visible)
                .default_size([380.0, 450.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 430.0, 300.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::motion_matching::show(ui, &mut self.state, &mut self.motion_matching_panel);
                    });
                });
            self.state.show_motion_matching = visible;
        }

        // Floating blend tree editor
        if self.state.show_blend_tree {
            let mut visible = true;
            egui::Window::new("🌿 Blend Tree")
                .open(&mut visible)
                .default_size([650.0, 420.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([150.0, 150.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::blend_tree_editor::show(ui, &mut self.state, &mut self.blend_tree_panel);
                    });
                });
            self.state.show_blend_tree = visible;
        }

        // Floating graph editor
        if self.state.show_graph_editor {
            let mut visible = true;
            egui::Window::new("📈 Éditeur de courbes")
                .open(&mut visible)
                .default_size([700.0, 350.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([100.0, ctx.screen_rect().max.y - 400.0])
                .show(ctx, |ui| {
                    panels::graph_editor::show(ui, &mut self.state, &mut self.graph_editor_panel);
                });
            self.state.show_graph_editor = visible;
        }

        // Floating ragdoll panel
        if self.state.show_ragdoll {
            let mut visible = true;
            egui::Window::new("🦴 Ragdoll Physics")
                .open(&mut visible)
                .default_size([320.0, 420.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 340.0, 150.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::ragdoll_panel::show(ui, &mut self.state, &mut self.ragdoll_panel);
                    });
                });
            self.state.show_ragdoll = visible;
        }

        // Floating DeepPhase panel
        if self.state.show_deep_phase {
            let mut visible = true;
            egui::Window::new("🌊 DeepPhase Manifold")
                .open(&mut visible)
                .default_size([380.0, 520.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([150.0, 80.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::deep_phase_panel::show(ui, &mut self.state, &mut self.deep_phase_panel);
                    });
                });
            self.state.show_deep_phase = visible;
        }

        // Floating animation recorder panel
        if self.state.show_anim_recorder {
            let mut visible = true;
            egui::Window::new("🎬 Enregistreur d'animation")
                .open(&mut visible)
                .default_size([340.0, 380.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 360.0, 250.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::anim_recorder_panel::show(ui, &mut self.state, &mut self.anim_recorder_panel);
                    });
                });
            self.state.show_anim_recorder = visible;
        }

        // Floating material editor panel
        if self.state.show_material_editor {
            let mut visible = true;
            egui::Window::new("🎨 Éditeur de matériaux")
                .open(&mut visible)
                .default_size([300.0, 380.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 320.0, 350.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::material_editor::show(ui, &mut self.state, &mut self.material_editor_panel);
                    });
                });
            self.state.show_material_editor = visible;
        }

        // Floating cloth panel
        if self.state.show_cloth {
            let mut visible = true;
            egui::Window::new("🧵 Tissu / Soft-body")
                .open(&mut visible)
                .default_size([320.0, 400.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([150.0, 200.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::cloth_panel::show(ui, &mut self.state, &mut self.cloth_panel);
                    });
                });
            self.state.show_cloth = visible;
        }

        // Floating IK panel
        if self.state.show_ik_panel {
            let mut visible = true;
            egui::Window::new("🎯 IK avancé")
                .open(&mut visible)
                .default_size([340.0, 450.0])
                .resizable(true)
                .collapsible(true)
                .default_pos([ctx.screen_rect().max.x - 360.0, 60.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        panels::ik_panel::show(ui, &mut self.state, &mut self.ik_panel);
                    });
                });
            self.state.show_ik_panel = visible;
        }

        // Drain training panel pending commands
        if !self.training_panel.pending_commands.is_empty() {
            let cmds: Vec<AiCommand> = self.training_panel.pending_commands.drain(..).collect();
            for cmd in cmds {
                execute_ai_command(cmd, &mut self.state, &mut self.asset_manager);
            }
        }

        // Request continuous repaint for smooth interaction
        ctx.request_repaint();
    }
}

/// Execute a structured AI command on the editor state.
fn execute_ai_command(cmd: AiCommand, state: &mut AppState, asset_manager: &mut AssetManager) {
    match cmd {
        // ── File operations ────────────────────────────────
        AiCommand::ImportFile { path } => {
            let p = std::path::PathBuf::from(&path);
            match asset_manager.load(&p) {
                Ok(model) => {
                    state.import_model_from_path(model, &p);
                    state.log_info(&format!("[IA] Importé: {}", path));
                }
                Err(e) => state.log_error(&format!("[IA] Erreur import: {:#}", e)),
            }
        }
        AiCommand::ExportFrame { path } => {
            state.pending_export_frame = Some(path.clone());
            state.log_info(&format!("[IA] Export frame → {}", path));
        }

        // ── Playback ───────────────────────────────────────
        AiCommand::Play => {
            state.playing = true;
            state.log_info("[IA] Lecture");
        }
        AiCommand::Pause => {
            state.playing = false;
            state.log_info("[IA] Pause");
        }
        AiCommand::Stop => {
            state.playing = false;
            state.timestamp = 0.0;
            state.log_info("[IA] Stop");
        }
        AiCommand::SetFrame { frame } => {
            if let Some(motion) = state.active_motion() {
                let dt = motion.delta_time();
                let total = motion.total_time();
                state.timestamp = (frame as f32 * dt).clamp(0.0, total);
                state.log_info(&format!("[IA] Frame → {}", frame));
            }
        }
        AiCommand::SetSpeed { speed } => {
            state.playback_speed = speed.clamp(0.01, 10.0);
            state.log_info(&format!("[IA] Vitesse → {:.1}x", speed));
        }
        AiCommand::SetTime { time } => {
            let total = state.total_time();
            state.timestamp = if total > 0.0 { time.clamp(0.0, total) } else { time.max(0.0) };
            state.log_info(&format!("[IA] Temps → {:.2}s", time));
        }
        AiCommand::ToggleLoop { enabled } => {
            state.looping = enabled;
            state.log_info(&format!("[IA] Boucle → {}", if enabled { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleMirror { enabled } => {
            state.mirrored = enabled;
            state.log_info(&format!("[IA] Miroir → {}", if enabled { "ON" } else { "OFF" }));
        }

        // ── Camera ─────────────────────────────────────────
        AiCommand::CameraReset => {
            state.camera.reset();
            state.log_info("[IA] Caméra réinitialisée");
        }
        AiCommand::CameraLookAt { x, y, z } => {
            state.camera.look_at(glam::Vec3::new(x, y, z));
            state.log_info(&format!("[IA] Caméra → ({:.1}, {:.1}, {:.1})", x, y, z));
        }
        AiCommand::CameraView { view } => {
            match view.to_lowercase().as_str() {
                "front" => state.camera.view_front(),
                "right" => state.camera.view_right(),
                "top" => state.camera.view_top(),
                "back" => state.camera.view_back(),
                "left" => state.camera.view_left(),
                "bottom" => state.camera.view_bottom(),
                _ => state.log_error(&format!("[IA] Vue inconnue: {}", view)),
            }
            state.log_info(&format!("[IA] Vue → {}", view));
        }
        AiCommand::CameraDistance { distance } => {
            state.camera.distance = distance.clamp(0.1, 100.0);
            state.log_info(&format!("[IA] Distance caméra → {:.1}", distance));
        }

        // ── Selection ──────────────────────────────────────
        AiCommand::SelectEntity { id } => {
            state.multi_selection.clear();
            state.multi_selection.push(id);
            state.scene.selected = Some(id);
            state.log_info(&format!("[IA] Sélection entité #{}", id));
        }
        AiCommand::SelectBone { name } => {
            if let Some(idx) = state.active_model {
                let asset = &state.loaded_models[idx];
                if let Some(bone_idx) = asset.model.joint_names.iter().position(|n| n == &name) {
                    if bone_idx < asset.joint_entity_ids.len() {
                        let eid = asset.joint_entity_ids[bone_idx];
                        state.multi_selection.clear();
                        state.multi_selection.push(eid);
                        state.scene.selected = Some(eid);
                        state.log_info(&format!("[IA] Sélection os \"{}\" (entité #{})", name, eid));
                    }
                } else {
                    state.log_error(&format!("[IA] Os \"{}\" introuvable", name));
                }
            }
        }
        AiCommand::DeselectAll => {
            state.multi_selection.clear();
            state.scene.selected = None;
            state.log_info("[IA] Désélection");
        }

        // ── Transform ──────────────────────────────────────
        AiCommand::SetPosition { entity, x, y, z } => {
            use anim_math::transform::Transform;
            if entity < state.scene.transforms.len() {
                state.scene.transforms[entity].set_position(glam::Vec3::new(x, y, z));
                state.log_info(&format!("[IA] Position #{} → ({:.2}, {:.2}, {:.2})", entity, x, y, z));
            }
        }
        AiCommand::SetRotation { entity, rx, ry, rz } => {
            use anim_math::transform::Transform;
            if entity < state.scene.transforms.len() {
                let q = glam::Quat::from_euler(glam::EulerRot::XYZ,
                    rx.to_radians(), ry.to_radians(), rz.to_radians());
                state.scene.transforms[entity].set_rotation(glam::Mat3::from_quat(q));
                state.log_info(&format!("[IA] Rotation #{} → ({:.0}°, {:.0}°, {:.0}°)", entity, rx, ry, rz));
            }
        }
        AiCommand::SetScale { entity, sx, sy, sz } => {
            if entity < state.scene.scales.len() {
                state.scene.scales[entity] = glam::Vec3::new(sx, sy, sz);
                state.log_info(&format!("[IA] Scale #{} → ({:.2}, {:.2}, {:.2})", entity, sx, sy, sz));
            }
        }

        // ── Tools ──────────────────────────────────────────
        AiCommand::SetTool { tool } => {
            match tool.to_lowercase().as_str() {
                "select" => state.active_tool = anim_gui::app_state::Tool::Select,
                "move" => state.active_tool = anim_gui::app_state::Tool::Move,
                "rotate" => state.active_tool = anim_gui::app_state::Tool::Rotate,
                "measure" => {
                    state.active_tool = anim_gui::app_state::Tool::Measure;
                    state.measure.reset();
                }
                "ik" => {
                    state.active_tool = anim_gui::app_state::Tool::Ik;
                    state.ik_chain_root = None;
                    state.ik_chain_tip = None;
                }
                _ => state.log_error(&format!("[IA] Outil inconnu: {}", tool)),
            }
            state.log_info(&format!("[IA] Outil → {}", tool));
        }
        AiCommand::SetAxis { axis } => {
            match axis.to_lowercase().as_str() {
                "x" => state.gizmo_axis = anim_gui::app_state::GizmoAxis::X,
                "y" => state.gizmo_axis = anim_gui::app_state::GizmoAxis::Y,
                "z" => state.gizmo_axis = anim_gui::app_state::GizmoAxis::Z,
                _ => state.gizmo_axis = anim_gui::app_state::GizmoAxis::None,
            }
        }

        // ── Display ────────────────────────────────────────
        AiCommand::ToggleSkeleton { visible } => {
            state.show_skeleton = visible;
            state.log_info(&format!("[IA] Squelette → {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleMesh { visible } => {
            state.show_mesh = visible;
            state.log_info(&format!("[IA] Mesh → {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleGrid { visible } => {
            state.show_grid = visible;
            state.log_info(&format!("[IA] Grille → {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleVelocities { visible } => {
            state.show_velocities = visible;
            state.log_info(&format!("[IA] Vélocités → {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleContacts { visible } => {
            state.show_contacts = visible;
            state.log_info(&format!("[IA] Contacts → {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleTrajectory { visible } => {
            state.show_trajectory = visible;
            state.log_info(&format!("[IA] Trajectoire → {}", if visible { "ON" } else { "OFF" }));
        }

        // ── Render settings ────────────────────────────────
        AiCommand::SetRender { key, value } => {
            let rs = &mut state.render_settings;
            match key.as_str() {
                "ambient_strength" => { if let Some(v) = value.as_f64() { rs.ambient_strength = v as f32; } }
                "sun_strength" => { if let Some(v) = value.as_f64() { rs.sun_strength = v as f32; } }
                "sky_strength" => { if let Some(v) = value.as_f64() { rs.sky_strength = v as f32; } }
                "ground_strength" => { if let Some(v) = value.as_f64() { rs.ground_strength = v as f32; } }
                "exposure" => { if let Some(v) = value.as_f64() { rs.exposure = v as f32; } }
                "light_yaw" => { if let Some(v) = value.as_f64() { rs.light_yaw = v as f32; } }
                "light_pitch" => { if let Some(v) = value.as_f64() { rs.light_pitch = v as f32; } }
                "ssao_enabled" => { if let Some(v) = value.as_bool() { rs.ssao_enabled = v; } }
                "ssao_radius" => { if let Some(v) = value.as_f64() { rs.ssao_radius = v as f32; } }
                "ssao_bias" => { if let Some(v) = value.as_f64() { rs.ssao_bias = v as f32; } }
                "ssao_intensity" => { if let Some(v) = value.as_f64() { rs.ssao_intensity = v as f32; } }
                "shadows_enabled" => { if let Some(v) = value.as_bool() { rs.shadows_enabled = v; } }
                "shadow_bias" => { if let Some(v) = value.as_f64() { rs.shadow_bias = v as f32; } }
                "bloom_enabled" => { if let Some(v) = value.as_bool() { rs.bloom_enabled = v; } }
                "bloom_intensity" => { if let Some(v) = value.as_f64() { rs.bloom_intensity = v as f32; } }
                "bloom_spread" => { if let Some(v) = value.as_f64() { rs.bloom_spread = v as f32; } }
                "fxaa_enabled" => { if let Some(v) = value.as_bool() { rs.fxaa_enabled = v; } }
                _ => { state.log_error(&format!("[IA] Paramètre rendu inconnu: {}", key)); return; }
            }
            state.log_info(&format!("[IA] Rendu {} → {}", key, value));
        }

        // ── IK ─────────────────────────────────────────────
        AiCommand::IkSolve { root, tip, target_x, target_y, target_z } => {
            state.log_info(&format!(
                "[IA] IK: {} → {} cible ({:.2}, {:.2}, {:.2})",
                root, tip, target_x, target_y, target_z
            ));
            // Set up IK tool with the specified chain
            if let Some(idx) = state.active_model {
                let asset = &state.loaded_models[idx];
                let root_idx = asset.model.joint_names.iter().position(|n| n == &root);
                let tip_idx = asset.model.joint_names.iter().position(|n| n == &tip);
                if let (Some(ri), Some(ti)) = (root_idx, tip_idx) {
                    if ri < asset.joint_entity_ids.len() && ti < asset.joint_entity_ids.len() {
                        state.active_tool = anim_gui::app_state::Tool::Ik;
                        state.ik_chain_root = Some(asset.joint_entity_ids[ri]);
                        state.ik_chain_tip = Some(asset.joint_entity_ids[ti]);
                    }
                } else {
                    if root_idx.is_none() {
                        state.log_error(&format!("[IA] Os racine \"{}\" introuvable", root));
                    }
                    if tip_idx.is_none() {
                        state.log_error(&format!("[IA] Os cible \"{}\" introuvable", tip));
                    }
                }
            }
        }

        // ── Console ────────────────────────────────────────
        AiCommand::Log { text } => {
            state.log_info(&format!("[IA] {}", text));
        }

        // ── Panel visibility ───────────────────────────────
        AiCommand::ShowPanel { panel } => {
            set_panel_visibility(state, &panel, true);
        }
        AiCommand::HidePanel { panel } => {
            set_panel_visibility(state, &panel, false);
        }

        // ── Scene queries ──────────────────────────────────
        AiCommand::QueryScene | AiCommand::QueryEntity { .. } | AiCommand::ListBones => {
            // These are handled by the AI context injection, not as direct commands
            state.log_info("[IA] Requête scene (contexte injecté automatiquement)");
        }

        // ── Generative commands ───────────────────────────
        AiCommand::CreateHumanoid {
            name, height, animation, duration,
            skin_color, shirt_color, pants_color, shoes_color, hair_color,
        } => {
            let mut config = anim_import::HumanoidConfig {
                name: if name.is_empty() { "Humanoide".into() } else { name.clone() },
                height: height.clamp(0.5, 3.0),
                ..Default::default()
            };

            // Apply custom colors if provided
            if let Some([r, g, b]) = skin_color {
                config.colors.skin = [r, g, b, 255];
            }
            if let Some([r, g, b]) = shirt_color {
                config.colors.shirt = [r, g, b, 255];
            }
            if let Some([r, g, b]) = pants_color {
                config.colors.pants = [r, g, b, 255];
            }
            if let Some([r, g, b]) = shoes_color {
                config.colors.shoes = [r, g, b, 255];
            }
            if let Some([r, g, b]) = hair_color {
                config.colors.hair = [r, g, b, 255];
            }

            let anim_type = if animation.is_empty() { "idle" } else { &animation };
            let dur = duration.clamp(0.5, 30.0);

            let model = anim_import::generate_humanoid_with_animation(
                &config, anim_type, dur,
            );

            let desc = format!("{} ({}m, {}, {:.1}s)",
                config.name, config.height, anim_type, dur);
            state.import_model(model);
            state.playing = true;
            state.looping = true;
            state.log_info(&format!("[IA] Créé humanoïde: {}", desc));
        }

        AiCommand::CreateAnimation { anim_type, duration } => {
            if let Some(idx) = state.active_model {
                // Capture current pose for crossfade BEFORE anything else
                state.start_crossfade();

                let asset = &state.loaded_models[idx];
                // Get rest positions from current model
                let rest_positions: Vec<glam::Vec3> = asset.joint_entity_ids.iter()
                    .map(|&eid| {
                        use anim_math::transform::Transform;
                        if eid < state.scene.transforms.len() {
                            state.scene.transforms[eid].get_position()
                        } else {
                            glam::Vec3::ZERO
                        }
                    })
                    .collect();

                let dur = duration.clamp(0.5, 30.0);
                let anim_data = match anim_type.to_lowercase().as_str() {
                    "run" | "course" | "courir" =>
                        anim_import::procedural::generate_run_animation(&rest_positions, dur, 30.0),
                    "walk" | "marche" | "marcher" =>
                        anim_import::procedural::generate_walk_animation(&rest_positions, dur, 30.0),
                    "jump" | "saut" | "sauter" =>
                        anim_import::procedural::generate_jump_animation(&rest_positions, dur, 30.0),
                    _ =>
                        anim_import::procedural::generate_idle_animation(&rest_positions, dur, 30.0),
                };

                // Replace the motion on the active model
                let motion = anim_animation::Motion::from_animation_data(
                    &asset.model.joint_names,
                    &asset.model.parent_indices,
                    &anim_data.frames,
                    anim_data.framerate,
                );
                state.loaded_models[idx].motion = Some(motion);
                state.timestamp = 0.0;
                state.playing = true;
                state.looping = true;
                state.log_info(&format!("[IA] Animation {} créée ({:.1}s, crossfade {:.1}s)", anim_type, dur, state.crossfade_duration));
            } else {
                state.log_error("[IA] Aucun modèle actif pour l'animation");
            }
        }

        AiCommand::DeleteModel => {
            if let Some(idx) = state.active_model {
                let name = state.loaded_models[idx].name.clone();
                state.loaded_models.remove(idx);
                state.active_model = if state.loaded_models.is_empty() {
                    None
                } else {
                    Some(idx.min(state.loaded_models.len() - 1))
                };
                state.multi_selection.clear();
                state.scene.selected = None;
                state.log_info(&format!("[IA] Supprimé: {}", name));
            } else {
                state.log_error("[IA] Aucun modèle à supprimer");
            }
        }

        AiCommand::SetColor { r, g, b } => {
            if let Some(idx) = state.active_model {
                if let Some(ref mut mesh) = state.loaded_models[idx].skinned_mesh {
                    mesh.color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
                    state.log_info(&format!("[IA] Couleur → ({}, {}, {})", r, g, b));
                }
            }
        }

        AiCommand::Generate { description } => {
            // The AI should decompose this into specific commands.
            // If it reaches here directly, log what was requested.
            state.log_info(&format!("[IA] Génération demandée: {}", description));
            state.log_info("[IA] Astuce: décomposez en commandes spécifiques (create_humanoid, create_animation, etc.)");
        }

        AiCommand::LoadLocomotion { model_path, meta_path } => {
            let model_p = std::path::Path::new(&model_path);
            let meta_p = std::path::Path::new(&meta_path);
            match anim_animation::LocomotionController::new(model_p, meta_p, None) {
                Ok(controller) => {
                    state.log_info(&format!(
                        "[IA] Modèle locomotion chargé: input={}, latent={}",
                        controller.metadata.input_dim,
                        controller.metadata.latent_dim
                    ));
                    state.locomotion_controller = Some(controller);
                }
                Err(e) => {
                    state.log_error(&format!("[IA] Erreur chargement locomotion: {:#}", e));
                }
            }
        }

        AiCommand::ToggleLocomotion { enabled } => {
            if let Some(ref mut ctrl) = state.locomotion_controller {
                ctrl.active = enabled;
                state.log_info(&format!("[IA] Locomotion {}", if enabled { "activée" } else { "désactivée" }));
            } else {
                state.log_error("[IA] Aucun modèle locomotion chargé. Utilisez load_locomotion d'abord.");
            }
        }

        AiCommand::TrainModel { data_dir, output_dir, epochs, batch_size, learning_rate } => {
            state.log_info(&format!(
                "[IA] Lancement entraînement: data={}, epochs={}, batch={}, lr={}",
                data_dir, epochs, batch_size, learning_rate
            ));

            let script = find_tool_script("train_locomotion.py");
            if !script.exists() {
                state.log_error(&format!("[IA] Script introuvable: {}", script.display()));
                return;
            }

            state.training_active = true;
            let log_queue = state.bg_log_queue.clone();

            std::thread::spawn(move || {
                let result = std::process::Command::new("python")
                    .arg(&script)
                    .arg("--data-dir").arg(&data_dir)
                    .arg("--output-dir").arg(&output_dir)
                    .arg("--epochs").arg(epochs.to_string())
                    .arg("--batch-size").arg(batch_size.to_string())
                    .arg("--lr").arg(learning_rate.to_string())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .output();

                match result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let mut q = log_queue.lock().unwrap();
                        for line in stdout.lines() {
                            q.push(format!("[Train] {}", line));
                        }
                        if output.status.success() {
                            q.push("[Train] ✓ Entraînement terminé avec succès!".to_string());
                            q.push("[Train] DONE".to_string());
                        } else {
                            q.push(format!("[Train] ERREUR: {}", stderr.trim()));
                            q.push("[Train] DONE".to_string());
                        }
                    }
                    Err(e) => {
                        let mut q = log_queue.lock().unwrap();
                        q.push(format!("[Train] Impossible de lancer Python: {}", e));
                        q.push("[Train] DONE".to_string());
                    }
                }
            });
        }

        AiCommand::BuildMotionDb => {
            // Add all loaded clips to the database and build
            let clips: Vec<(String, anim_animation::Motion)> = state.loaded_models.iter()
                .filter_map(|a| a.motion.as_ref().map(|m| (a.name.clone(), m.clone())))
                .collect();
            if clips.is_empty() {
                state.log_error("[IA] Aucun clip d'animation chargé pour le motion matching");
                return;
            }
            state.motion_database = anim_animation::MotionDatabase::new();
            let count = clips.len();
            for (name, motion) in clips {
                state.motion_database.add_clip(name, motion);
            }
            state.motion_database.build();
            let entries = state.motion_database.num_entries();
            state.motion_matching_controller.db_built = true;
            state.log_info(&format!(
                "[IA] Motion DB construite: {} clips, {} entrées", count, entries
            ));
            state.show_motion_matching = true;
        }

        AiCommand::SetStateMachineParam { name, value } => {
            if state.state_machine.is_none() {
                state.state_machine = Some(anim_animation::StateMachine::new("Principale"));
                state.show_state_machine = true;
            }
            if let Some(ref mut sm) = state.state_machine {
                if let Some(b) = value.as_bool() {
                    sm.set_bool(&name, b);
                    state.log_info(&format!("[SM] Param {}={}", name, b));
                } else if let Some(f) = value.as_f64() {
                    sm.set_float(&name, f as f32);
                    state.log_info(&format!("[SM] Param {}={:.2}", name, f));
                } else {
                    state.log_error(&format!("[SM] Valeur invalide pour '{}': {:?}", name, value));
                }
            }
        }

        AiCommand::CreateStateMachineState { name, model_index } => {
            if state.state_machine.is_none() {
                state.state_machine = Some(anim_animation::StateMachine::new("Principale"));
                state.show_state_machine = true;
            }
            if let Some(ref mut sm) = state.state_machine {
                let source = match model_index {
                    Some(idx) if idx < state.loaded_models.len() => {
                        anim_animation::state_machine::MotionSource::Clip { model_index: idx }
                    }
                    _ => anim_animation::state_machine::MotionSource::None,
                };
                let n = sm.num_states();
                let pos = [50.0 + n as f32 * 150.0, 100.0];
                let id = sm.add_state(name.clone(), source, pos);
                state.log_info(&format!("[SM] État créé: {} (id={})", name, id));
            }
        }

        AiCommand::ToggleMotionMatching { enabled } => {
            if !state.motion_database.built || state.motion_database.num_entries() == 0 {
                state.log_error("[IA] Base motion matching non construite. Utilisez build_motion_db d'abord.");
                return;
            }
            state.motion_matching_controller.active = enabled;
            state.motion_matching_controller.db_built = true;
            state.log_info(&format!("[IA] Motion matching {}",
                if enabled { "activé" } else { "désactivé" }));
        }

        AiCommand::SelectModel { index } => {
            if index < state.loaded_models.len() {
                state.active_model = Some(index);
                state.timestamp = 0.0;
                state.log_info(&format!("[IA] Modèle actif → {} ({})",
                    index, state.loaded_models[index].name));
            } else {
                state.log_error(&format!("[IA] Index modèle invalide: {} (max={})",
                    index, state.loaded_models.len().saturating_sub(1)));
            }
        }

        AiCommand::SetModelPlayback { index, playing, speed, looping } => {
            if index < state.loaded_models.len() {
                let asset = &mut state.loaded_models[index];
                asset.independent_playback = true;
                if let Some(p) = playing { asset.local_playing = p; }
                if let Some(s) = speed { asset.local_speed = s.clamp(0.01, 10.0); }
                if let Some(l) = looping { asset.local_looping = l; }
                state.log_info(&format!("[IA] Playback modèle {}: playing={}, speed={:.1}, loop={}",
                    index,
                    state.loaded_models[index].local_playing,
                    state.loaded_models[index].local_speed,
                    state.loaded_models[index].local_looping));
            } else {
                state.log_error(&format!("[IA] Index modèle invalide: {}", index));
            }
        }

        AiCommand::SetModelOffset { index, x, y, z } => {
            if index < state.loaded_models.len() {
                state.loaded_models[index].world_offset = glam::Vec3::new(x, y, z);
                state.log_info(&format!("[IA] Offset modèle {} → ({:.1}, {:.1}, {:.1})",
                    index, x, y, z));
            } else {
                state.log_error(&format!("[IA] Index modèle invalide: {}", index));
            }
        }

        AiCommand::SetModelVisible { index, visible } => {
            if index < state.loaded_models.len() {
                state.loaded_models[index].visible = visible;
                state.log_info(&format!("[IA] Modèle {} → {}",
                    index, if visible { "visible" } else { "masqué" }));
            } else {
                state.log_error(&format!("[IA] Index modèle invalide: {}", index));
            }
        }

        AiCommand::ExportGlb { path } => {
            if let Some(idx) = state.active_model {
                let p = std::path::PathBuf::from(&path);
                let asset = &state.loaded_models[idx];
                let frames = asset.motion.as_ref().map(|m| &m.frames);
                let framerate = asset.motion.as_ref().map_or(30.0, |m| m.framerate);
                match anim_import::export_glb(&p, &asset.model, frames, framerate) {
                    Ok(()) => state.log_info(&format!("[IA] Exporté GLB: {}", path)),
                    Err(e) => state.log_error(&format!("[IA] Erreur export GLB: {}", e)),
                }
            } else {
                state.log_error("[IA] Aucun modèle actif pour l'export GLB");
            }
        }

        AiCommand::SetBlendTreeParam { name, value } => {
            if state.blend_tree.is_none() {
                state.blend_tree = Some(anim_animation::BlendTree::new("Principal"));
                state.show_blend_tree = true;
            }
            if let Some(ref mut bt) = state.blend_tree {
                bt.set_parameter(&name, value);
                state.log_info(&format!("[BT] Param {}={:.2}", name, value));
            }
        }

        AiCommand::CreateBlendTreeNode { node_type, name, parameter, model_index } => {
            if state.blend_tree.is_none() {
                state.blend_tree = Some(anim_animation::BlendTree::new("Principal"));
                state.show_blend_tree = true;
            }
            if let Some(ref mut bt) = state.blend_tree {
                let n = bt.num_nodes();
                let pos = [50.0 + (n as f32 % 4.0) * 150.0, 50.0 + (n as f32 / 4.0).floor() * 70.0];
                let node = match node_type.to_lowercase().as_str() {
                    "clip" => anim_animation::BlendTreeNode::Clip(anim_animation::ClipNode {
                        name: name.clone(),
                        model_index: model_index.unwrap_or(0),
                        speed: 1.0,
                        position: pos,
                    }),
                    "blend1d" | "1d" => anim_animation::BlendTreeNode::Blend1D(anim_animation::Blend1DNode {
                        name: name.clone(),
                        parameter: parameter.unwrap_or_else(|| "speed".into()),
                        children: Vec::new(),
                        position: pos,
                    }),
                    "blend2d" | "2d" => anim_animation::BlendTreeNode::Blend2D(anim_animation::Blend2DNode {
                        name: name.clone(),
                        param_x: parameter.unwrap_or_else(|| "dx".into()),
                        param_y: "dy".into(),
                        children: Vec::new(),
                        position: pos,
                    }),
                    _ => anim_animation::BlendTreeNode::Lerp(anim_animation::LerpNode {
                        name: name.clone(),
                        parameter: parameter.unwrap_or_else(|| "mix".into()),
                        child_a: 0,
                        child_b: 0,
                        position: pos,
                    }),
                };
                let idx = bt.add_node(node);
                if n == 0 { bt.root = idx; }
                state.log_info(&format!("[BT] Noeud créé: {} (type={}, id={})", name, node_type, idx));
            }
        }

        // ── Ragdoll Physics ─────────────────────────────────
        AiCommand::CreateRagdoll => {
            if let Some(idx) = state.active_model {
                if let Some(ref motion) = state.loaded_models[idx].motion {
                    let transforms = motion.get_transforms_interpolated(
                        state.timestamp, state.mirrored,
                    );
                    let mut ragdoll = anim_animation::Ragdoll::from_pose(
                        &transforms,
                        &motion.hierarchy.parent_indices,
                        anim_animation::RagdollConfig::default(),
                    );
                    ragdoll.set_pinned(0, true);
                    state.ragdoll = Some(ragdoll);
                    state.show_ragdoll = true;
                    state.log_info(&format!("[IA] Ragdoll créé ({} corps)", transforms.len()));
                } else {
                    state.log_error("[IA] Pas d'animation pour créer le ragdoll");
                }
            } else {
                state.log_error("[IA] Aucun modèle actif");
            }
        }

        AiCommand::DestroyRagdoll => {
            state.ragdoll = None;
            state.log_info("[IA] Ragdoll supprimé");
        }

        AiCommand::ToggleRagdoll { enabled } => {
            if let Some(ref mut ragdoll) = state.ragdoll {
                ragdoll.active = enabled;
                state.log_info(&format!("[IA] Ragdoll {}",
                    if enabled { "activé" } else { "en pause" }));
            } else if enabled {
                // Auto-create ragdoll if enabling and none exists
                if let Some(idx) = state.active_model {
                    if let Some(ref motion) = state.loaded_models[idx].motion {
                        let transforms = motion.get_transforms_interpolated(
                            state.timestamp, state.mirrored,
                        );
                        let mut ragdoll = anim_animation::Ragdoll::from_pose(
                            &transforms,
                            &motion.hierarchy.parent_indices,
                            anim_animation::RagdollConfig::default(),
                        );
                        ragdoll.set_pinned(0, true);
                        ragdoll.active = true;
                        state.ragdoll = Some(ragdoll);
                        state.show_ragdoll = true;
                        state.log_info("[IA] Ragdoll créé et activé");
                    }
                }
            }
        }

        AiCommand::RagdollImpulse { x, y, z } => {
            if let Some(ref mut ragdoll) = state.ragdoll {
                let imp = glam::Vec3::new(x, y, z);
                for i in 0..ragdoll.num_bodies() {
                    ragdoll.apply_impulse(i, imp);
                }
                state.log_info(&format!("[IA] Impulse ragdoll ({:.1}, {:.1}, {:.1})", x, y, z));
            } else {
                state.log_error("[IA] Pas de ragdoll actif");
            }
        }

        AiCommand::RagdollExplosion { force, radius } => {
            if let Some(ref mut ragdoll) = state.ragdoll {
                let f = force.unwrap_or(30.0);
                let r = radius.unwrap_or(3.0);
                ragdoll.apply_explosion(glam::Vec3::ZERO, f, r);
                state.log_info(&format!("[IA] Explosion ragdoll (F={:.0}, R={:.1})", f, r));
            } else {
                state.log_error("[IA] Pas de ragdoll actif");
            }
        }

        AiCommand::RagdollPin { body, pinned } => {
            if let Some(ref mut ragdoll) = state.ragdoll {
                if body < ragdoll.num_bodies() {
                    ragdoll.set_pinned(body, pinned);
                    state.log_info(&format!("[IA] Ragdoll corps {} → {}",
                        body, if pinned { "épinglé" } else { "libre" }));
                } else {
                    state.log_error(&format!("[IA] Index corps invalide: {}", body));
                }
            } else {
                state.log_error("[IA] Pas de ragdoll actif");
            }
        }

        // ── DeepPhase ────────────────────────────────────────
        AiCommand::ExtractDeepPhase => {
            if let Some(idx) = state.active_model {
                if let Some(ref motion) = state.loaded_models[idx].motion {
                    let config = anim_animation::DeepPhaseConfig::default();
                    let manifold = anim_animation::extract_deep_phase(motion, config);
                    let n = manifold.num_frames();
                    let freqs: Vec<String> = manifold.dominant_frequencies.iter()
                        .map(|f| format!("{:.2}Hz", f))
                        .collect();
                    state.log_info(&format!(
                        "[IA] DeepPhase extrait: {} frames, freqs: {}",
                        n, freqs.join(", ")
                    ));
                    state.deep_phase = Some(manifold);
                    state.show_deep_phase = true;
                } else {
                    state.log_error("[IA] Pas d'animation pour extraire la phase");
                }
            } else {
                state.log_error("[IA] Aucun modèle actif");
            }
        }

        AiCommand::ClearDeepPhase => {
            state.deep_phase = None;
            state.log_info("[IA] DeepPhase manifold effacé");
        }

        // ── FBX Export ─────────────────────────────────────
        AiCommand::ExportFbx { path } => {
            if let Some(idx) = state.active_model {
                let p = std::path::PathBuf::from(&path);
                let asset = &state.loaded_models[idx];
                let frames = asset.motion.as_ref().map(|m| &m.frames);
                let framerate = asset.motion.as_ref().map_or(30.0, |m| m.framerate);
                match anim_import::export_fbx(&p, &asset.model, frames, framerate) {
                    Ok(()) => state.log_info(&format!("[IA] Exporté FBX: {}", path)),
                    Err(e) => state.log_error(&format!("[IA] Erreur export FBX: {}", e)),
                }
            } else {
                state.log_error("[IA] Aucun modèle actif pour l'export FBX");
            }
        }

        // ── USD Export ──────────────────────────────────────
        AiCommand::ExportUsd { path } => {
            if let Some(idx) = state.active_model {
                let p = std::path::PathBuf::from(&path);
                let asset = &state.loaded_models[idx];
                let frames = asset.motion.as_ref().map(|m| &m.frames);
                let framerate = asset.motion.as_ref().map_or(30.0, |m| m.framerate);
                match anim_import::export_usd(&p, &asset.model, frames, framerate) {
                    Ok(()) => state.log_info(&format!("[IA] Exporté USD: {}", path)),
                    Err(e) => state.log_error(&format!("[IA] Erreur export USD: {}", e)),
                }
            } else {
                state.log_error("[IA] Aucun modèle actif pour l'export USD");
            }
        }

        // ── Animation Recording ────────────────────────────
        AiCommand::StartRecording => {
            if let Some(idx) = state.active_model {
                let num_joints = state.loaded_models[idx].joint_entity_ids.len();
                state.log_info(&format!("[IA] Enregistrement démarré ({} joints)", num_joints));
                state.show_anim_recorder = true;
            } else {
                state.log_error("[IA] Aucun modèle actif pour l'enregistrement");
            }
        }

        AiCommand::StopRecording => {
            state.log_info("[IA] Enregistrement arrêté (utilisez le panneau pour stop & sauver)");
        }

        AiCommand::PauseRecording => {
            state.log_info("[IA] Enregistrement en pause");
        }

        AiCommand::ResumeRecording => {
            state.log_info("[IA] Enregistrement repris");
        }

        // ── Cloth / Soft-body ──────────────────────────────
        AiCommand::CreateCloth { width, height, size } => {
            let origin = glam::Vec3::new(-size * 0.5, 2.0, -size * 0.5);
            let right = glam::Vec3::new(size, 0.0, 0.0);
            let down = glam::Vec3::new(0.0, 0.0, size);
            let mut cloth = anim_animation::ClothSim::new_grid(origin, right, down, width, height);
            cloth.pin_top_row();
            state.cloth_sim = Some(cloth);
            state.show_cloth = true;
            state.log_info(&format!("[IA] Tissu {}×{} créé", width, height));
        }

        AiCommand::DestroyCloth => {
            state.cloth_sim = None;
            state.log_info("[IA] Tissu supprimé");
        }

        AiCommand::ToggleCloth { enabled } => {
            if let Some(ref mut cloth) = state.cloth_sim {
                cloth.active = enabled;
                state.log_info(&format!("[IA] Tissu {}",
                    if enabled { "activé" } else { "en pause" }));
            } else if enabled {
                // Auto-create
                let mut cloth = anim_animation::ClothSim::new_grid(
                    glam::Vec3::new(-0.75, 2.0, -0.75),
                    glam::Vec3::new(1.5, 0.0, 0.0),
                    glam::Vec3::new(0.0, 0.0, 1.5),
                    12, 12,
                );
                cloth.pin_top_row();
                state.cloth_sim = Some(cloth);
                state.show_cloth = true;
                state.log_info("[IA] Tissu créé et activé");
            }
        }

        // ── Material ──────────────────────────────────────
        AiCommand::SetMaterial { color, metallic, roughness } => {
            if let Some(idx) = state.active_model {
                if let Some(ref mut mesh) = state.loaded_models[idx].skinned_mesh {
                    if let Some([r, g, b]) = color {
                        mesh.color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
                    }
                    if let Some(m) = metallic {
                        mesh.metallic = m.clamp(0.0, 1.0);
                    }
                    if let Some(r) = roughness {
                        mesh.roughness = r.clamp(0.0, 1.0);
                    }
                    state.log_info("[IA] Matériau mis à jour");
                    state.show_material_editor = true;
                }
            } else {
                state.log_error("[IA] Aucun modèle actif");
            }
        }

        // ── Procedural creatures ───────────────────────────
        AiCommand::CreateCreature { creature_type, height } => {
            let model = anim_import::generate_creature(&creature_type, height.clamp(0.1, 5.0));
            let name = model.name.clone();
            state.import_model(model);
            state.log_info(&format!("[IA] Créature créée: {} ({:.1}m)", name, height));
        }

        // ── Display toggles (advanced) ──────────────────────
        AiCommand::ToggleRootMotion { visible } => {
            state.show_root_motion = visible;
            state.log_info(&format!("[IA] Root motion: {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleOnionSkinning { visible } => {
            state.onion_skinning = visible;
            state.log_info(&format!("[IA] Onion skinning: {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleGuidance { visible } => {
            state.show_guidance = visible;
            state.log_info(&format!("[IA] Guidance: {}", if visible { "ON" } else { "OFF" }));
        }
        AiCommand::ToggleTracking { visible } => {
            state.show_tracking = visible;
            state.log_info(&format!("[IA] Tracking: {}", if visible { "ON" } else { "OFF" }));
        }

        // ── IK Configuration ──────────────────────────────
        AiCommand::SetIkConstraints { enabled } => {
            state.ik_use_constraints = enabled;
            state.log_info(&format!("[IA] Contraintes IK: {}", if enabled { "ON" } else { "OFF" }));
        }
        AiCommand::SetIkPoleTarget { enabled, x, y, z, weight } => {
            state.ik_use_pole_target = enabled;
            if let Some(px) = x { state.ik_pole_position.x = px; }
            if let Some(py) = y { state.ik_pole_position.y = py; }
            if let Some(pz) = z { state.ik_pole_position.z = pz; }
            if let Some(w) = weight { state.ik_pole_weight = w.clamp(0.0, 1.0); }
            state.log_info(&format!("[IA] Pole target: {}", if enabled { "ON" } else { "OFF" }));
        }
        AiCommand::SetIkPreset { preset } => {
            use anim_gui::app_state::IkPreset;
            state.ik_preset = match preset.to_lowercase().as_str() {
                "human_arm" | "bras" | "bras_humain" => IkPreset::HumanArm,
                "human_leg" | "jambe" | "jambe_humaine" => IkPreset::HumanLeg,
                "custom" | "personnalise" => IkPreset::Custom,
                _ => IkPreset::None,
            };
            state.log_info(&format!("[IA] Préréglage IK: {}", preset));
        }

        // ── Cloth configuration ───────────────────────────
        AiCommand::SetClothConfig { gravity, damping, stiffness, iterations, ground_y, wind_x, wind_z } => {
            if let Some(ref mut cloth) = state.cloth_sim {
                if let Some(g) = gravity { cloth.config.gravity.y = g; }
                if let Some(d) = damping { cloth.config.damping = d.clamp(0.0, 0.5); }
                if let Some(s) = stiffness { cloth.config.stiffness = s.clamp(0.0, 1.0); }
                if let Some(i) = iterations { cloth.config.iterations = i.clamp(1, 20); }
                if let Some(gy) = ground_y { cloth.config.ground_y = gy; }
                if let Some(wx) = wind_x { cloth.config.wind.x = wx; }
                if let Some(wz) = wind_z { cloth.config.wind.z = wz; }
                state.log_info("[IA] Configuration tissu mise à jour");
            } else {
                state.log_warn("[IA] Pas de tissu actif");
            }
        }

        AiCommand::ConvertModel { model_path, output_dir } => {
            state.log_info(&format!("[IA] Conversion PT→ONNX: {}", model_path));

            let script = find_tool_script("convert_pt_to_onnx.py");
            if !script.exists() {
                state.log_error(&format!("[IA] Script introuvable: {}", script.display()));
                return;
            }

            let log_queue = state.bg_log_queue.clone();

            std::thread::spawn(move || {
                let result = std::process::Command::new("python")
                    .arg(&script)
                    .arg(&model_path)
                    .arg(&output_dir)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .output();

                match result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let mut q = log_queue.lock().unwrap();
                        for line in stdout.lines() {
                            q.push(format!("[Convert] {}", line));
                        }
                        if !output.status.success() {
                            q.push(format!("[Convert] ERREUR: {}", stderr.trim()));
                        }
                    }
                    Err(e) => {
                        let mut q = log_queue.lock().unwrap();
                        q.push(format!("[Convert] Impossible de lancer Python: {}", e));
                    }
                }
            });
        }
    }
}

/// Find a tool script in the tools/ directory relative to the executable.
fn find_tool_script(name: &str) -> std::path::PathBuf {
    // Try relative to exe
    if let Ok(exe) = std::env::current_exe() {
        let tools = exe.parent().unwrap_or(std::path::Path::new("."))
            .join("../../../tools").join(name);
        if tools.exists() {
            return tools;
        }
    }
    // Try relative to CWD
    let cwd = std::path::PathBuf::from("tools").join(name);
    if cwd.exists() {
        return cwd;
    }
    // Fallback
    std::path::PathBuf::from("tools").join(name)
}

fn set_panel_visibility(state: &mut AppState, panel: &str, show: bool) {
    match panel.to_lowercase().as_str() {
        "console" => state.show_console = show,
        "profiler" => state.show_profiler = show,
        "dope_sheet" | "dopesheet" => state.show_dope_sheet = show,
        "motion_editor" => state.show_motion_editor = show,
        "recorder" => state.show_recorder = show,
        "batch" => state.show_batch = show,
        "asset_browser" | "assets" => state.show_asset_browser = show,
        "render_settings" | "render" => state.show_render_settings = show,
        "ai" | "ai_chat" => state.show_ai_chat = show,
        "training" | "train" => state.show_training = show,
        "motion_matching" | "matching" => state.show_motion_matching = show,
        "state_machine" | "sm" => state.show_state_machine = show,
        "pose_editor" | "pose" => state.show_pose_editor = show,
        "blend_tree" | "blend" => state.show_blend_tree = show,
        "graph_editor" | "graph" | "curves" => state.show_graph_editor = show,
        "ragdoll" | "physics" => state.show_ragdoll = show,
        "deep_phase" | "phase" | "deepphase" => state.show_deep_phase = show,
        "anim_recorder" | "enregistreur" | "animation_recorder" => state.show_anim_recorder = show,
        "material" | "materials" | "materiaux" => state.show_material_editor = show,
        "cloth" | "tissu" | "soft_body" => state.show_cloth = show,
        "ik" | "ik_panel" | "inverse_kinematics" => state.show_ik_panel = show,
        _ => state.log_error(&format!("[IA] Panneau inconnu: {}", panel)),
    }
}
