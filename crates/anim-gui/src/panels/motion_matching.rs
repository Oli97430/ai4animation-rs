//! Motion Matching panel — load clips, build database, control real-time matching.

use egui::{Ui, RichText};
use crate::app_state::AppState;
use crate::theme::accent;

/// Persistent panel state that lives on AnimApp (not AppState).
pub struct MotionMatchingPanel {
    /// Status message for user feedback.
    pub status: String,
}

impl Default for MotionMatchingPanel {
    fn default() -> Self {
        Self {
            status: String::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut MotionMatchingPanel) {
    ui.label(RichText::new("Motion Matching").size(13.0).color(accent::TEXT));
    ui.separator();

    // ── Database info ──────────────────────────────────
    ui.label(RichText::new("Base de données").size(11.0).color(accent::MUTED));

    let db = &state.motion_database;
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Clips: {}", db.num_clips())).size(10.5));
        ui.separator();
        ui.label(RichText::new(format!("Frames: {}", db.total_frames())).size(10.5));
        ui.separator();
        ui.label(RichText::new(format!("Entrées: {}", db.num_entries())).size(10.5));
        ui.separator();
        let built_label = if db.built { "Prête" } else { "Non construite" };
        let built_color = if db.built { accent::SUCCESS } else { accent::MUTED };
        ui.label(RichText::new(built_label).size(10.5).color(built_color));
    });

    ui.add_space(4.0);

    // ── Add clips from loaded models ───────────────────
    ui.horizontal(|ui| {
        if ui.button(RichText::new("+ Ajouter clip actif").size(10.5)).clicked() {
            if let Some(idx) = state.active_model {
                let asset = &state.loaded_models[idx];
                if let Some(ref motion) = asset.motion {
                    let name = asset.name.clone();
                    let motion_clone = motion.clone();
                    state.motion_database.add_clip(name.clone(), motion_clone);
                    panel.status = format!("Clip ajouté: {}", name);
                    state.log_info(&format!("[MM] Clip ajouté: {}", name));
                } else {
                    panel.status = "Le modèle actif n'a pas d'animation".to_string();
                }
            } else {
                panel.status = "Aucun modèle actif".to_string();
            }
        }

        if ui.button(RichText::new("+ Tous les clips").size(10.5)).clicked() {
            let mut added = 0;
            // Collect data first to avoid borrow issues
            let clips: Vec<(String, anim_animation::Motion)> = state.loaded_models.iter()
                .filter_map(|a| a.motion.as_ref().map(|m| (a.name.clone(), m.clone())))
                .collect();
            for (name, motion) in clips {
                state.motion_database.add_clip(name, motion);
                added += 1;
            }
            panel.status = format!("{} clip(s) ajouté(s)", added);
            state.log_info(&format!("[MM] {} clip(s) ajouté(s) à la base", added));
        }
    });

    // ── Build database ─────────────────────────────────
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new("Construire base").size(10.5)).clicked() {
            if state.motion_database.num_clips() > 0 {
                state.motion_database.build();
                let n = state.motion_database.num_entries();
                panel.status = format!("Base construite: {} entrées", n);
                state.log_info(&format!("[MM] Base construite: {} entrées indexées", n));
                state.motion_matching_controller.db_built = true;
            } else {
                panel.status = "Aucun clip chargé".to_string();
            }
        }

        if ui.button(RichText::new("Réinitialiser").size(10.5)).clicked() {
            state.motion_database = anim_animation::MotionDatabase::new();
            state.motion_matching_controller = anim_animation::MotionMatchingController::new();
            panel.status = "Base réinitialisée".to_string();
            state.log_info("[MM] Base de données réinitialisée");
        }
    });

    // ── Weights ────────────────────────────────────────
    ui.add_space(6.0);
    ui.collapsing(RichText::new("Poids des features").size(10.5), |ui| {
        let w = &mut state.motion_database.weights;
        ui.add(egui::Slider::new(&mut w.trajectory_position, 0.0..=5.0)
            .text(RichText::new("Trajectoire (pos)").size(10.0)));
        ui.add(egui::Slider::new(&mut w.trajectory_direction, 0.0..=5.0)
            .text(RichText::new("Trajectoire (dir)").size(10.0)));
        ui.add(egui::Slider::new(&mut w.pose_position, 0.0..=5.0)
            .text(RichText::new("Pose (pos)").size(10.0)));
        ui.add(egui::Slider::new(&mut w.pose_velocity, 0.0..=5.0)
            .text(RichText::new("Pose (vitesse)").size(10.0)));
        ui.add(egui::Slider::new(&mut w.contact, 0.0..=5.0)
            .text(RichText::new("Contacts").size(10.0)));
    });

    // ── Controller settings ────────────────────────────
    ui.add_space(6.0);
    ui.collapsing(RichText::new("Contrôleur").size(10.5), |ui| {
        let ctrl = &mut state.motion_matching_controller;
        ui.add(egui::Slider::new(&mut ctrl.query_interval, 0.02..=1.0)
            .text(RichText::new("Intervalle requête (s)").size(10.0)));
        ui.add(egui::Slider::new(&mut ctrl.transition_threshold, 0.0..=5.0)
            .text(RichText::new("Seuil transition").size(10.0)));
        ui.add(egui::Slider::new(&mut ctrl.exclusion_window, 1..=60)
            .text(RichText::new("Fenêtre exclusion (frames)").size(10.0)));

        // Crossfade duration
        let mut dur = ctrl.transition.duration;
        if ui.add(egui::Slider::new(&mut dur, 0.05..=1.0)
            .text(RichText::new("Durée crossfade (s)").size(10.0))).changed() {
            ctrl.transition = anim_animation::AnimationTransition::new(dur);
        }
    });

    // ── Activate / Deactivate ──────────────────────────
    ui.add_space(6.0);
    ui.separator();

    let is_active = state.motion_matching_controller.active;
    let can_activate = state.motion_database.built && state.motion_database.num_entries() > 0;

    let label = if is_active { "Désactiver" } else { "Activer" };
    let btn = ui.add_enabled(
        can_activate || is_active,
        egui::Button::new(RichText::new(label).size(11.0)),
    );
    if btn.clicked() {
        let new_active = !is_active;
        state.motion_matching_controller.active = new_active;
        if new_active {
            state.motion_matching_controller.db_built = true;
            panel.status = "Motion matching activé".to_string();
            state.log_info("[MM] Motion matching activé");
        } else {
            panel.status = "Motion matching désactivé".to_string();
            state.log_info("[MM] Motion matching désactivé");
        }
    }

    if is_active {
        let (clip, frame, cost) = state.motion_matching_controller.status();
        let clip_name = state.motion_database.clips.get(clip)
            .map(|c| c.name.as_str())
            .unwrap_or("?");
        ui.label(RichText::new(format!(
            "Clip: {} | Frame: {} | Coût: {:.2}", clip_name, frame, cost
        )).size(10.0).color(accent::SUCCESS));
    }

    // ── Status ─────────────────────────────────────────
    if !panel.status.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(&panel.status).size(10.0).color(accent::MUTED));
    }

    // ── Clip list ──────────────────────────────────────
    if state.motion_database.num_clips() > 0 {
        ui.add_space(6.0);
        ui.collapsing(RichText::new("Clips chargés").size(10.5), |ui| {
            for (i, clip) in state.motion_database.clips.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!(
                        "{}. {} ({} frames, {:.1}s)",
                        i, clip.name, clip.motion.num_frames(), clip.motion.total_time()
                    )).size(10.0));
                });
            }
        });
    }
}
