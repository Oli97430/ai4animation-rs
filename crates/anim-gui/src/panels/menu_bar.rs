//! Top menu bar — styled with icons and refined layout.

use egui::{menu, Ui, RichText, Color32};
use anim_core::i18n::{t, Lang};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_import::{GlbImporter, BvhImporter, NpzImporter, FbxImporter, SkeletonPreset};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    // Top-level menu labels use a slightly larger font for readability
    // in the narrow, dark menu bar.
    let menu_size = 13.0;

    menu::bar(ui, |ui| {
        ui.add_space(4.0);

        // ── File ────────────────────────────────────────────
        ui.menu_button(RichText::new(format!("📁 {}", t("file"))).size(menu_size).color(accent::TEXT_BRIGHT), |ui| {
            if menu_item(ui, "💾", "Sauvegarder projet", "Ctrl+S") {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Projet AI4Anim", &["a4a"])
                    .set_file_name("project.a4a")
                    .save_file()
                {
                    match crate::scene_io::save_project(&path, state) {
                        Ok(()) => state.log_info(&format!("Projet sauvegarde: {}", path.display())),
                        Err(e) => state.log_error(&format!("Erreur sauvegarde: {}", e)),
                    }
                }
                ui.close_menu();
            }
            if menu_item(ui, "📂", "Ouvrir projet", "Ctrl+O") {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Projet AI4Anim", &["a4a"])
                    .pick_file()
                {
                    // Defer to main loop where AssetManager is available
                    state.pending_project_load = Some(path);
                }
                ui.close_menu();
            }

            // Recent files
            if !state.recent_files_display.is_empty() {
                ui.menu_button(RichText::new("🕐 Fichiers récents").size(12.0), |ui| {
                    for (path, name, file_type) in &state.recent_files_display {
                        let icon = crate::recent_files::RecentFiles::icon_for(file_type);
                        let label = format!("{} {} (.{})", icon, name, file_type);
                        let btn = ui.button(RichText::new(&label).size(11.5));
                        if btn.on_hover_text(path).clicked() {
                            state.pending_recent_import = Some(std::path::PathBuf::from(path));
                            ui.close_menu();
                        }
                    }
                });
            }

            ui.separator();

            // Skeleton presets submenu
            ui.menu_button(RichText::new("🦴 Nouveau squelette").size(12.0), |ui| {
                for preset in SkeletonPreset::all() {
                    let label = format!("{} {}", preset.icon(), preset.label());
                    if ui.button(RichText::new(&label).size(11.5)).clicked() {
                        let model = preset.generate();
                        state.import_model(model);
                        state.log_info(&format!("Cree: squelette {}", preset.short_name()));
                        ui.close_menu();
                    }
                }
            });

            ui.separator();

            if menu_item(ui, "📥", t("import_glb"), "") {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("GLB/glTF", &["glb", "gltf"])
                    .pick_file()
                {
                    match GlbImporter::load(&path) {
                        Ok(model) => state.import_model_from_path(model, &path),
                        Err(e) => state.log_error(&format!("Erreur GLB: {:#}", e)),
                    }
                }
                ui.close_menu();
            }
            if menu_item(ui, "📥", t("import_bvh"), "") {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("BVH", &["bvh"])
                    .pick_file()
                {
                    match BvhImporter::load(&path, 0.01) {
                        Ok(model) => state.import_model_from_path(model, &path),
                        Err(e) => state.log_error(&format!("Erreur BVH: {:#}", e)),
                    }
                }
                ui.close_menu();
            }
            if menu_item(ui, "📥", "Importer NPZ", "") {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("NPZ Motion", &["npz"])
                    .pick_file()
                {
                    match NpzImporter::load(&path) {
                        Ok(model) => state.import_model_from_path(model, &path),
                        Err(e) => state.log_error(&format!("Erreur NPZ: {:#}", e)),
                    }
                }
                ui.close_menu();
            }
            if menu_item(ui, "📥", "Importer FBX", "") {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("FBX", &["fbx"])
                    .pick_file()
                {
                    match FbxImporter::load(&path) {
                        Ok(model) => state.import_model_from_path(model, &path),
                        Err(e) => state.log_error(&format!("Erreur FBX: {:#}", e)),
                    }
                }
                ui.close_menu();
            }
            if menu_item(ui, "📥", "Importer USD", "") {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("USD/USDA", &["usd", "usda"])
                    .pick_file()
                {
                    match anim_import::import_usd(&path) {
                        Ok(model) => state.import_model_from_path(model, &path),
                        Err(e) => state.log_error(&format!("Erreur USD: {}", e)),
                    }
                }
                ui.close_menu();
            }

            ui.separator();

            // Export BVH (full sequence or current pose)
            let has_model = state.active_model.is_some();
            let has_anim = state.active_model
                .and_then(|i| state.loaded_models.get(i))
                .and_then(|a| a.motion.as_ref())
                .map_or(false, |m| m.num_frames() > 1);

            // Full sequence export
            let btn_seq = egui::Button::new(RichText::new("📤 Exporter BVH (sequence)").size(12.0));
            if ui.add_enabled(has_anim, btn_seq).clicked() {
                if let Some(idx) = state.active_model {
                    if let Some(ref motion) = state.loaded_models[idx].motion {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("BVH", &["bvh"])
                            .set_file_name(&format!("{}.bvh", state.loaded_models[idx].name))
                            .save_file()
                        {
                            match anim_import::export_bvh_sequence(
                                &path,
                                &motion.hierarchy.bone_names,
                                &motion.hierarchy.parent_indices,
                                &motion.frames,
                                motion.framerate,
                            ) {
                                Ok(()) => state.log_info(&format!(
                                    "Exporte BVH: {} ({} frames)", path.display(), motion.num_frames()
                                )),
                                Err(e) => state.log_error(&format!("Erreur export: {}", e)),
                            }
                        }
                    }
                }
                ui.close_menu();
            }

            // Single pose export
            let btn_pose = egui::Button::new(RichText::new("📤 Exporter BVH (pose)").size(12.0));
            if ui.add_enabled(has_model, btn_pose).clicked() {
                if let Some(idx) = state.active_model {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("BVH", &["bvh"])
                        .set_file_name(&format!("{}_pose.bvh", state.loaded_models[idx].name))
                        .save_file()
                    {
                        let asset = &state.loaded_models[idx];
                        let transforms: Vec<glam::Mat4> = asset.joint_entity_ids.iter()
                            .map(|&eid| state.scene.get_transform(eid))
                            .collect();
                        let framerate = asset.motion.as_ref().map_or(30.0, |m| m.framerate);
                        match anim_import::export_bvh_pose(
                            &path,
                            &asset.model.joint_names,
                            &asset.model.parent_indices,
                            &transforms,
                            framerate,
                        ) {
                            Ok(()) => state.log_info(&format!("Exporte pose: {}", path.display())),
                            Err(e) => state.log_error(&format!("Erreur export: {}", e)),
                        }
                    }
                }
                ui.close_menu();
            }

            // Export GLB
            let btn_glb = egui::Button::new(RichText::new("📤 Exporter GLB").size(12.0));
            if ui.add_enabled(has_model, btn_glb).clicked() {
                if let Some(idx) = state.active_model {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("GLB", &["glb"])
                        .set_file_name(&format!("{}.glb", state.loaded_models[idx].name))
                        .save_file()
                    {
                        let asset = &state.loaded_models[idx];
                        let frames = asset.motion.as_ref().map(|m| &m.frames);
                        let framerate = asset.motion.as_ref().map_or(30.0, |m| m.framerate);
                        match anim_import::export_glb(
                            &path,
                            &asset.model,
                            frames,
                            framerate,
                        ) {
                            Ok(()) => {
                                let frame_count = asset.motion.as_ref().map_or(0, |m| m.num_frames());
                                let mesh_count = asset.model.meshes.len();
                                state.log_info(&format!(
                                    "Exporté GLB: {} ({} meshes, {} joints, {} frames)",
                                    path.display(), mesh_count, asset.model.num_joints(), frame_count
                                ));
                            }
                            Err(e) => state.log_error(&format!("Erreur export GLB: {}", e)),
                        }
                    }
                }
                ui.close_menu();
            }

            // Export FBX
            let btn_fbx = egui::Button::new(RichText::new("📤 Exporter FBX").size(12.0));
            if ui.add_enabled(has_model, btn_fbx).clicked() {
                if let Some(idx) = state.active_model {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("FBX", &["fbx"])
                        .set_file_name(&format!("{}.fbx", state.loaded_models[idx].name))
                        .save_file()
                    {
                        let asset = &state.loaded_models[idx];
                        let frames = asset.motion.as_ref().map(|m| &m.frames);
                        let framerate = asset.motion.as_ref().map_or(30.0, |m| m.framerate);
                        match anim_import::export_fbx(
                            &path,
                            &asset.model,
                            frames,
                            framerate,
                        ) {
                            Ok(()) => {
                                let frame_count = asset.motion.as_ref().map_or(0, |m| m.num_frames());
                                state.log_info(&format!(
                                    "Exporté FBX: {} ({} joints, {} frames)",
                                    path.display(), asset.model.num_joints(), frame_count
                                ));
                            }
                            Err(e) => state.log_error(&format!("Erreur export FBX: {}", e)),
                        }
                    }
                }
                ui.close_menu();
            }

            // Export USD
            let btn_usd = egui::Button::new(RichText::new("📤 Exporter USD").size(12.0));
            if ui.add_enabled(has_model, btn_usd).clicked() {
                if let Some(idx) = state.active_model {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("USDA", &["usda", "usd"])
                        .set_file_name(&format!("{}.usda", state.loaded_models[idx].name))
                        .save_file()
                    {
                        let asset = &state.loaded_models[idx];
                        let frames = asset.motion.as_ref().map(|m| &m.frames);
                        let framerate = asset.motion.as_ref().map_or(30.0, |m| m.framerate);
                        match anim_import::export_usd(
                            &path,
                            &asset.model,
                            frames,
                            framerate,
                        ) {
                            Ok(()) => {
                                let frame_count = asset.motion.as_ref().map_or(0, |m| m.num_frames());
                                state.log_info(&format!(
                                    "Exporté USD: {} ({} joints, {} frames)",
                                    path.display(), asset.model.num_joints(), frame_count
                                ));
                            }
                            Err(e) => state.log_error(&format!("Erreur export USD: {}", e)),
                        }
                    }
                }
                ui.close_menu();
            }

            // Export NPZ
            let btn_npz = egui::Button::new(RichText::new("📤 Exporter NPZ").size(12.0));
            if ui.add_enabled(has_model, btn_npz).clicked() {
                if let Some(idx) = state.active_model {
                    if let Some(ref motion) = state.loaded_models[idx].motion {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("NPZ", &["npz"])
                            .set_file_name(&format!("{}.npz", state.loaded_models[idx].name))
                            .save_file()
                        {
                            match anim_import::export_npz(
                                &path,
                                &motion.hierarchy.bone_names,
                                &motion.hierarchy.parent_indices,
                                &motion.frames,
                                motion.framerate,
                            ) {
                                Ok(()) => state.log_info(&format!("Exporte NPZ: {}", path.display())),
                                Err(e) => state.log_error(&format!("Erreur export NPZ: {}", e)),
                            }
                        }
                    }
                }
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "⏻", t("quit"), "") {
                std::process::exit(0);
            }
        });

        // ── Edit ────────────────────────────────────────────
        ui.menu_button(RichText::new(format!("✏ {}", t("edit"))).size(menu_size).color(accent::TEXT_BRIGHT), |ui| {
            let can_undo = state.history.can_undo();
            let can_redo = state.history.can_redo();
            if ui.add_enabled(can_undo, egui::Button::new(
                RichText::new("↩ Annuler").size(12.0)
            )).on_hover_text("Ctrl+Z").clicked() {
                state.undo();
                ui.close_menu();
            }
            if ui.add_enabled(can_redo, egui::Button::new(
                RichText::new("↪ Refaire").size(12.0)
            )).on_hover_text("Ctrl+Y").clicked() {
                state.redo();
                ui.close_menu();
            }
            ui.separator();

            ui.horizontal(|ui| {
                ui.checkbox(&mut state.snap_to_grid, RichText::new("⊞ Accrochage grille").size(11.5));
            });
            if state.snap_to_grid {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Taille:").size(11.0).color(accent::MUTED));
                    ui.add(egui::DragValue::new(&mut state.snap_size)
                        .range(0.01..=1.0)
                        .speed(0.01)
                        .fixed_decimals(2));
                });
            }
        });

        // ── View ────────────────────────────────────────────
        ui.menu_button(RichText::new(format!("👁 {}", t("view"))).size(menu_size).color(accent::TEXT_BRIGHT), |ui| {
            // Display toggles
            ui.checkbox(&mut state.show_skeleton, RichText::new(format!("🦴 {}", t("skeleton"))).size(11.5));
            ui.checkbox(&mut state.show_mesh, RichText::new(format!("🧊 {}", t("mesh"))).size(11.5));
            ui.checkbox(&mut state.show_velocities, RichText::new(format!("→ {}", t("velocities"))).size(11.5));
            ui.checkbox(&mut state.show_contacts, RichText::new("⬤ Contacts (pieds)").size(11.5));
            ui.checkbox(&mut state.show_trajectory, RichText::new("⟿ Trajectoire").size(11.5));
            ui.checkbox(&mut state.show_guidance, RichText::new("◆ Guidage").size(11.5));
            ui.checkbox(&mut state.show_tracking, RichText::new("◎ Suivi").size(11.5));
            ui.checkbox(&mut state.show_root_motion, RichText::new("⊕ Mouvement racine").size(11.5));
            if state.show_trajectory {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Passe:").size(10.5).color(accent::MUTED));
                    ui.add(egui::DragValue::new(&mut state.trajectory_config.past_window)
                        .range(0.0..=3.0).speed(0.05).suffix("s").fixed_decimals(1));
                    ui.label(RichText::new("Futur:").size(10.5).color(accent::MUTED));
                    ui.add(egui::DragValue::new(&mut state.trajectory_config.future_window)
                        .range(0.0..=3.0).speed(0.05).suffix("s").fixed_decimals(1));
                });
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Échantillons:").size(10.5).color(accent::MUTED));
                    ui.add(egui::DragValue::new(&mut state.trajectory_config.sample_count)
                        .range(5..=61).speed(1.0));
                });
            }
            ui.checkbox(&mut state.show_grid, RichText::new(format!("⊞ {}", t("grid"))).size(11.5));
            ui.checkbox(&mut state.show_axes, RichText::new(format!("✛ {}", t("axes"))).size(11.5));
            ui.checkbox(&mut state.show_gizmo, RichText::new(format!("✥ {}", t("gizmo"))).size(11.5));
            ui.separator();

            // Onion skinning
            ui.checkbox(&mut state.onion_skinning, RichText::new("🧅 Pelure d'oignon").size(11.5));
            if state.onion_skinning {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Avant:").size(10.5).color(accent::MUTED));
                    ui.add(egui::DragValue::new(&mut state.onion_before).range(0..=10).speed(0.1));
                    ui.label(RichText::new("Apres:").size(10.5).color(accent::MUTED));
                    ui.add(egui::DragValue::new(&mut state.onion_after).range(0..=10).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Pas:").size(10.5).color(accent::MUTED));
                    ui.add(egui::DragValue::new(&mut state.onion_step).range(1..=30).speed(0.2));
                });
            }

            ui.separator();

            // Camera view presets
            ui.menu_button(RichText::new("📷 Vues camera").size(11.5), |ui| {
                if ui.button(RichText::new("⬛ Face (Numpad 1)").size(11.5)).clicked() {
                    state.camera.view_front();
                    ui.close_menu();
                }
                if ui.button(RichText::new("⬛ Arrière").size(11.5)).clicked() {
                    state.camera.view_back();
                    ui.close_menu();
                }
                if ui.button(RichText::new("⬛ Droite (Numpad 3)").size(11.5)).clicked() {
                    state.camera.view_right();
                    ui.close_menu();
                }
                if ui.button(RichText::new("⬛ Gauche").size(11.5)).clicked() {
                    state.camera.view_left();
                    ui.close_menu();
                }
                if ui.button(RichText::new("⬛ Dessus (Numpad 7)").size(11.5)).clicked() {
                    state.camera.view_top();
                    ui.close_menu();
                }
                if ui.button(RichText::new("⬛ Dessous").size(11.5)).clicked() {
                    state.camera.view_bottom();
                    ui.close_menu();
                }
            });

            ui.separator();
            ui.checkbox(&mut state.show_dope_sheet, RichText::new("🎬 Feuille d'expo").size(11.5));
            ui.checkbox(&mut state.show_motion_editor, RichText::new("🎛 Éditeur de mouvement").size(11.5));
            ui.checkbox(&mut state.show_console, RichText::new(format!("⌨ {}", t("console"))).size(11.5));
            ui.checkbox(&mut state.show_render_settings, RichText::new(format!("🎨 {}", t("render_settings"))).size(11.5));
            ui.checkbox(&mut state.show_profiler, RichText::new("📊 Profileur").size(11.5));
            ui.checkbox(&mut state.show_recorder, RichText::new("📹 Enregistreur vidéo").size(11.5));
            ui.checkbox(&mut state.show_batch, RichText::new("📦 Convertisseur batch").size(11.5));
            ui.checkbox(&mut state.show_asset_browser, RichText::new("📁 Navigateur d'assets").size(11.5));
            ui.checkbox(&mut state.show_ai_chat, RichText::new("🤖 Chat IA").size(11.5));
            ui.checkbox(&mut state.show_training, RichText::new("🧠 Entraînement").size(11.5));
            ui.checkbox(&mut state.show_motion_matching, RichText::new("🎯 Motion Matching").size(11.5));
            ui.checkbox(&mut state.show_state_machine, RichText::new("🔀 Machine d'états").size(11.5));
            ui.checkbox(&mut state.show_pose_editor, RichText::new("🦴 Éditeur de pose").size(11.5));
            ui.checkbox(&mut state.show_blend_tree, RichText::new("🌿 Blend Tree").size(11.5));
            ui.checkbox(&mut state.show_graph_editor, RichText::new("📈 Éditeur de courbes").size(11.5));
            ui.checkbox(&mut state.show_ragdoll, RichText::new("🦴 Ragdoll Physics").size(11.5));
            ui.checkbox(&mut state.show_deep_phase, RichText::new("🌊 DeepPhase").size(11.5));
            ui.checkbox(&mut state.show_anim_recorder, RichText::new("🎬 Enregistreur").size(11.5));
            ui.checkbox(&mut state.show_cloth, RichText::new("🧵 Tissu / Soft-body").size(11.5));
            ui.checkbox(&mut state.show_material_editor, RichText::new("🎨 Matériaux").size(11.5));
            ui.checkbox(&mut state.show_ik_panel, RichText::new("🎯 IK avancé").size(11.5));
        });

        // ── Animation ───────────────────────────────────────
        ui.menu_button(RichText::new(format!("🎬 {}", t("animation"))).size(menu_size).color(accent::TEXT_BRIGHT), |ui| {
            let play_label = if state.playing {
                RichText::new(format!("⏸ {}", t("pause"))).size(11.5)
            } else {
                RichText::new(format!("▶ {}", t("play"))).size(11.5)
            };
            if ui.button(play_label).clicked() {
                state.playing = !state.playing;
                ui.close_menu();
            }
            if ui.button(RichText::new(format!("⏹ {}", t("stop"))).size(11.5)).clicked() {
                state.playing = false;
                state.timestamp = 0.0;
                ui.close_menu();
            }
            ui.separator();
            ui.checkbox(&mut state.looping, RichText::new(format!("↻ {}", t("loop"))).size(11.5));
            ui.checkbox(&mut state.mirrored, RichText::new(format!("⟷ {}", t("mirror"))).size(11.5));
            ui.checkbox(&mut state.auto_key, RichText::new("● Clé auto").size(11.5));

            ui.separator();
            let has_model = state.active_model.is_some();
            if ui.add_enabled(has_model, egui::Button::new(
                RichText::new("📋 Copier pose").size(11.5)
            )).on_hover_text("Ctrl+C").clicked() {
                state.copy_pose();
                ui.close_menu();
            }
            let has_clip = state.pose_clipboard.is_some() && has_model;
            if ui.add_enabled(has_clip, egui::Button::new(
                RichText::new("📌 Coller pose").size(11.5)
            )).on_hover_text("Ctrl+V").clicked() {
                state.paste_pose();
                ui.close_menu();
            }
            if ui.add_enabled(has_model, egui::Button::new(
                RichText::new("🪞 Miroir pose").size(11.5)
            )).on_hover_text("Ctrl+M").clicked() {
                state.mirror_pose();
                ui.close_menu();
            }

            ui.separator();
            // Retarget submenu
            let can_retarget = state.loaded_models.len() >= 2;
            ui.menu_button(
                RichText::new("🔗 Retarget (mesh ← anim)").size(11.5),
                |ui| {
                    if !can_retarget {
                        ui.label(RichText::new("Chargez au moins 2 modeles").size(10.5).color(accent::DIM));
                        return;
                    }
                    // List all pairs: mesh targets × animation sources
                    let active = state.active_model.unwrap_or(0);
                    let mesh_name = state.loaded_models.get(active)
                        .map_or("?".to_string(), |a| a.name.clone());
                    ui.label(RichText::new(format!("Cible: {}", mesh_name)).size(10.5));
                    ui.separator();

                    let mut bind_to = None;
                    for (i, asset) in state.loaded_models.iter().enumerate() {
                        if i != active && asset.motion.is_some() {
                            let label = format!("← {} ({} fr)", asset.name,
                                asset.motion.as_ref().unwrap().num_frames());
                            if ui.button(RichText::new(&label).size(11.0)).clicked() {
                                bind_to = Some(i);
                                ui.close_menu();
                            }
                        }
                    }
                    if let Some(anim_idx) = bind_to {
                        state.retarget_mesh(active, anim_idx);
                    }

                    if state.loaded_models.iter().enumerate()
                        .all(|(i, a)| i == active || a.motion.is_none())
                    {
                        ui.label(RichText::new("(aucune animation disponible)").size(10.0).color(accent::DIM));
                    }
                },
            );
        });

        // ── IA ──────────────────────────────────────────────
        ui.menu_button(RichText::new("🤖 IA").size(menu_size).color(accent::TEXT_BRIGHT), |ui| {
            ui.checkbox(&mut state.show_ai_chat, RichText::new("💬 Chat IA").size(11.5));
            ui.separator();
            if ui.button(RichText::new("ℹ À propos").size(11.5)).clicked() {
                state.log_info("IA: Ollama (local) / OpenAI / Claude — tout est promptable");
                ui.close_menu();
            }
        });

        // ── Language ────────────────────────────────────────
        ui.menu_button(RichText::new(format!("🌐 {}", t("language"))).size(menu_size).color(accent::TEXT_BRIGHT), |ui| {
            for &lang in Lang::all() {
                if ui.radio(Lang::get() == lang, RichText::new(lang.label()).size(11.5)).clicked() {
                    Lang::set(lang);
                    ui.close_menu();
                }
            }
        });

        // ── Help ────────────────────────────────────────────
        ui.menu_button(RichText::new(format!("❓ {}", t("help"))).size(menu_size).color(accent::TEXT_BRIGHT), |ui| {
            ui.menu_button(RichText::new(format!("⌨ {}", t("shortcuts"))).size(11.5), |ui| {
                let shortcuts = [
                    ("Ctrl+S", "Sauvegarder le projet"),
                    ("Ctrl+Z", "Annuler"),
                    ("Ctrl+Y", "Refaire"),
                    ("Space", "Lecture / Pause"),
                    ("Home / End", "Début / Fin"),
                    ("← / →", "Frame préc. / suiv."),
                    ("R", "Réinitialiser caméra"),
                    ("F", "Centrer sur le modèle"),
                    ("G", "Afficher/masquer grille"),
                    ("L", "Activer/désactiver boucle"),
                    ("M", "Activer/désactiver miroir"),
                    ("1-5", "Outils"),
                    ("X/Y/Z", "Contrainte d'axe"),
                    ("Ctrl+C", "Copier la pose"),
                    ("Ctrl+V", "Coller la pose"),
                    ("Ctrl+M", "Miroir de la pose"),
                    ("D", "Dope sheet"),
                    ("O", "Pelure d'oignon"),
                    ("Ctrl+A", "Sélectionner tous les os"),
                    ("Esc", "Réinitialiser l'outil"),
                    ("WASD", "Déplacement (caméra libre)"),
                ];
                for (key, desc) in &shortcuts {
                    ui.horizontal(|ui| {
                        // Key badge
                        ui.add(
                            egui::Button::new(
                                RichText::new(*key).monospace().strong().size(10.0).color(accent::PRIMARY)
                            )
                                .fill(Color32::from_rgba_premultiplied(75, 135, 255, 20))
                                .rounding(3.0)
                                .stroke(egui::Stroke::NONE)
                                .sense(egui::Sense::hover())
                        );
                        ui.label(RichText::new(*desc).size(11.0));
                    });
                }
            });
            if ui.button(RichText::new("✏ Configurer raccourcis").size(11.5)).clicked() {
                state.show_shortcut_editor = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button(RichText::new(format!("ℹ {}", t("about"))).size(11.5)).clicked() {
                state.log_info("AI4Animation Engine v0.3.0 — Rust/wgpu/egui — Deferred Rendering Pipeline");
                ui.close_menu();
            }
        });

        // ── Version on the right ────────────────────────────
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(6.0);
            ui.add(
                egui::Button::new(
                    RichText::new("v0.3.0").size(10.0).color(accent::DIM)
                )
                    .fill(Color32::TRANSPARENT)
                    .rounding(6.0)
                    .stroke(egui::Stroke::new(0.5, accent::BORDER))
                    .sense(egui::Sense::hover())
            );
        });
    });
}

/// Styled menu item with icon, label and optional shortcut.
fn menu_item(ui: &mut Ui, icon: &str, label: &str, shortcut: &str) -> bool {
    let response = ui.horizontal(|ui| {
        let text = format!("{} {}", icon, label);
        let btn = ui.button(RichText::new(&text).size(12.0));
        if !shortcut.is_empty() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(shortcut).size(10.0).color(accent::DIM));
            });
        }
        btn.clicked()
    });
    response.inner
}
