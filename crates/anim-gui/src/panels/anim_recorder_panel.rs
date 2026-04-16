//! Animation Recorder panel — capture live transforms to keyframe clips.

use egui::{Ui, RichText, Color32};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_animation::{AnimRecorder, Motion};

/// Persistent panel state.
pub struct AnimRecorderPanel {
    pub recorder: AnimRecorder,
    pub clip_name: String,
    pub status: String,
    /// List of saved recorded clips (name, frame count, duration).
    pub saved_clips: Vec<(String, usize, f32)>,
}

impl Default for AnimRecorderPanel {
    fn default() -> Self {
        Self {
            recorder: AnimRecorder::default(),
            clip_name: "Recording".to_string(),
            status: String::new(),
            saved_clips: Vec::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut AnimRecorderPanel) {
    ui.label(RichText::new("Enregistreur d'animation").size(13.0).color(accent::TEXT));
    ui.separator();

    let is_idle = panel.recorder.is_idle();
    let is_recording = panel.recorder.is_recording();
    let is_paused = panel.recorder.is_paused();

    // ── Recording controls ──────────────────────────
    ui.horizontal(|ui| {
        if is_idle {
            // Start button
            let can_record = state.active_model.is_some();
            let btn = egui::Button::new(RichText::new("⏺ Enregistrer").size(11.0)
                .color(Color32::from_rgb(220, 60, 60)));
            if ui.add_enabled(can_record, btn).clicked() {
                if let Some(idx) = state.active_model {
                    let num_joints = state.loaded_models[idx].joint_entity_ids.len();
                    panel.recorder.start(num_joints);
                    panel.status = format!("Enregistrement... ({} joints)", num_joints);
                    state.log_info(&format!(
                        "[Recorder] Démarré ({} joints, {:.0} fps)",
                        num_joints, panel.recorder.config.framerate
                    ));
                }
            }
        } else {
            // Recording/Paused controls
            if is_recording {
                if ui.button(RichText::new("⏸ Pause").size(10.5)).clicked() {
                    panel.recorder.pause();
                    panel.status = "En pause".to_string();
                }
            } else if is_paused {
                if ui.button(RichText::new("▶ Reprendre").size(10.5)).clicked() {
                    panel.recorder.resume();
                    panel.status = "Reprise...".to_string();
                }
            }

            // Stop (save)
            if ui.button(RichText::new("⏹ Stop & Sauver").size(10.5)).clicked() {
                if let Some(mut clip) = panel.recorder.stop() {
                    clip.name = if panel.clip_name.is_empty() {
                        format!("Clip_{}", panel.saved_clips.len() + 1)
                    } else {
                        panel.clip_name.clone()
                    };

                    let n_frames = clip.num_frames();
                    let duration = clip.duration;
                    let name = clip.name.clone();

                    // Convert to Motion and apply to active model
                    if let Some(idx) = state.active_model {
                        let joint_names = &state.loaded_models[idx].model.joint_names;
                        let parent_indices = &state.loaded_models[idx].model.parent_indices;
                        let (frames, framerate) = anim_animation::clip_to_motion_data(
                            &clip, joint_names, parent_indices,
                        );
                        let motion = Motion::from_animation_data(
                            joint_names, parent_indices, &frames, framerate,
                        );
                        state.loaded_models[idx].motion = Some(motion);
                        state.timestamp = 0.0;
                        state.playing = false;
                    }

                    panel.saved_clips.push((name.clone(), n_frames, duration));
                    panel.status = format!(
                        "Sauvé: {} ({} frames, {:.1}s)", name, n_frames, duration
                    );
                    state.log_info(&format!(
                        "[Recorder] Clip sauvé: {} ({} frames, {:.1}s)",
                        name, n_frames, duration
                    ));
                } else {
                    panel.status = "Rien à sauver".to_string();
                }
            }

            // Cancel
            if ui.button(RichText::new("✕ Annuler").size(10.0).color(accent::ERROR)).clicked() {
                panel.recorder.cancel();
                panel.status = "Enregistrement annulé".to_string();
            }
        }
    });

    // ── Recording info ──────────────────────────────
    if !is_idle {
        let dot_color = if is_recording {
            Color32::from_rgb(220, 60, 60) // red dot
        } else {
            Color32::from_rgb(220, 180, 60) // yellow (paused)
        };
        let state_label = if is_recording { "REC" } else { "PAUSE" };

        ui.horizontal(|ui| {
            ui.label(RichText::new("●").size(12.0).color(dot_color));
            ui.label(RichText::new(state_label).size(10.5).color(dot_color).strong());
            ui.label(RichText::new(format!(
                "  {} frames  |  {:.1}s",
                panel.recorder.captured_frame_count(),
                panel.recorder.elapsed_time(),
            )).size(10.0).color(accent::MUTED));
        });
    }

    ui.separator();

    // ── Clip name ───────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(RichText::new("Nom:").size(9.5));
        ui.text_edit_singleline(&mut panel.clip_name);
    });

    // ── Configuration ───────────────────────────────
    ui.collapsing(RichText::new("Configuration").size(10.5), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Framerate:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.recorder.config.framerate)
                .range(1.0..=120.0).speed(1.0).fixed_decimals(0).suffix(" fps"));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Durée max:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.recorder.config.max_duration)
                .range(0.0..=600.0).speed(1.0).fixed_decimals(0).suffix("s"));
            ui.label(RichText::new("(0=illimité)").size(8.5).color(accent::DIM));
        });
        ui.checkbox(
            &mut panel.recorder.config.record_every_frame,
            RichText::new("Capturer chaque frame").size(9.5),
        );
        if !panel.recorder.config.record_every_frame {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(RichText::new("Intervalle:").size(9.5));
                ui.add(egui::DragValue::new(&mut panel.recorder.config.sample_interval)
                    .range(0.001..=1.0).speed(0.005).fixed_decimals(3).suffix("s"));
            });
        }
    });

    // ── Saved clips ─────────────────────────────────
    if !panel.saved_clips.is_empty() {
        ui.separator();
        ui.label(RichText::new("Clips enregistrés").size(10.5).color(accent::MUTED));
        for (i, (name, frames, duration)) in panel.saved_clips.iter().enumerate() {
            ui.label(RichText::new(format!(
                "  {}. {} — {} frames, {:.1}s", i + 1, name, frames, duration
            )).size(9.5));
        }
    }

    // Status
    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
    }
}
