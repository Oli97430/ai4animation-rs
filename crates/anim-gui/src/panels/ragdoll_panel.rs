//! Ragdoll Physics panel — control ragdoll simulation parameters.

use egui::{Ui, RichText};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_animation::{Ragdoll, RagdollConfig};

/// Persistent panel state.
pub struct RagdollPanel {
    pub config: RagdollConfig,
    pub impulse_strength: f32,
    pub explosion_force: f32,
    pub explosion_radius: f32,
    pub status: String,
}

impl Default for RagdollPanel {
    fn default() -> Self {
        Self {
            config: RagdollConfig::default(),
            impulse_strength: 5.0,
            explosion_force: 30.0,
            explosion_radius: 3.0,
            status: String::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut RagdollPanel) {
    ui.label(RichText::new("Ragdoll Physics").size(13.0).color(accent::TEXT));
    ui.separator();

    // Create / Destroy
    if state.ragdoll.is_none() {
        if ui.button(RichText::new("+ Créer Ragdoll").size(11.0)).clicked() {
            if let Some(idx) = state.active_model {
                if let Some(ref motion) = state.loaded_models[idx].motion {
                    let transforms = motion.get_transforms_interpolated(
                        state.timestamp, state.mirrored
                    );
                    let mut ragdoll = Ragdoll::from_pose(
                        &transforms,
                        &motion.hierarchy.parent_indices,
                        panel.config.clone(),
                    );
                    // Pin the root by default
                    ragdoll.set_pinned(0, true);
                    state.ragdoll = Some(ragdoll);
                    panel.status = format!("Ragdoll créé ({} corps)", transforms.len());
                    state.log_info("[Ragdoll] Créé depuis la pose courante");
                } else {
                    panel.status = "Erreur: pas d'animation".to_string();
                }
            } else {
                panel.status = "Erreur: pas de modèle actif".to_string();
            }
        }
        ui.add_space(4.0);
        ui.label(RichText::new("Paramètres").size(10.5).color(accent::MUTED));
    } else {
        // Active ragdoll controls
        ui.horizontal(|ui| {
            let is_active = state.ragdoll.as_ref().map_or(false, |r| r.active);
            let toggle_label = if is_active { "⏸ Pause" } else { "▶ Simuler" };
            if ui.button(RichText::new(toggle_label).size(10.0)).clicked() {
                if let Some(ref mut ragdoll) = state.ragdoll {
                    ragdoll.active = !ragdoll.active;
                    let status = if ragdoll.active { "activée" } else { "en pause" };
                    state.log_info(&format!("[Ragdoll] Simulation {}", status));
                }
            }

            if ui.button(RichText::new("↺ Reset").size(10.0)).clicked() {
                if let Some(idx) = state.active_model {
                    if let Some(ref motion) = state.loaded_models[idx].motion {
                        let transforms = motion.get_transforms_interpolated(
                            state.timestamp, state.mirrored
                        );
                        if let Some(ref mut ragdoll) = state.ragdoll {
                            ragdoll.reset_to_pose(&transforms);
                            ragdoll.active = false;
                            panel.status = "Ragdoll réinitialisé".to_string();
                        }
                    }
                }
            }

            if ui.button(RichText::new("✕ Supprimer").size(10.0).color(accent::ERROR)).clicked() {
                state.ragdoll = None;
                panel.status = "Ragdoll supprimé".to_string();
                state.log_info("[Ragdoll] Supprimé");
                return;
            }
        });

        // Info
        if let Some(ref ragdoll) = state.ragdoll {
            let pinned = ragdoll.bodies.iter().filter(|b| b.pinned).count();
            ui.label(RichText::new(format!(
                "{} corps, {} contraintes, {} épinglés",
                ragdoll.num_bodies(),
                ragdoll.distance_constraints.len(),
                pinned
            )).size(10.0).color(accent::MUTED));
        }

        ui.separator();

        // Impulse tools
        ui.label(RichText::new("Impulsions").size(10.5).color(accent::MUTED));
        ui.horizontal(|ui| {
            ui.label(RichText::new("Force:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.impulse_strength)
                .range(0.1..=100.0).speed(0.5).fixed_decimals(1));
        });

        ui.horizontal(|ui| {
            if ui.button(RichText::new("↑ Impulse haut").size(9.5)).clicked() {
                if let Some(ref mut ragdoll) = state.ragdoll {
                    let imp = glam::Vec3::new(0.0, panel.impulse_strength, 0.0);
                    for i in 0..ragdoll.num_bodies() {
                        ragdoll.apply_impulse(i, imp);
                    }
                    panel.status = format!("Impulse Y+{:.1}", panel.impulse_strength);
                }
            }

            if ui.button(RichText::new("💥 Explosion").size(9.5)).clicked() {
                if let Some(ref mut ragdoll) = state.ragdoll {
                    ragdoll.apply_explosion(
                        glam::Vec3::ZERO,
                        panel.explosion_force,
                        panel.explosion_radius,
                    );
                    panel.status = "Explosion appliquée".to_string();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Explosion:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.explosion_force)
                .range(1.0..=200.0).speed(1.0).prefix("F:").fixed_decimals(0));
            ui.add(egui::DragValue::new(&mut panel.explosion_radius)
                .range(0.5..=20.0).speed(0.1).prefix("R:").fixed_decimals(1));
        });

        ui.separator();

        // Pin/unpin bones
        ui.label(RichText::new("Corps épinglés").size(10.5).color(accent::MUTED));
        if let Some(ref mut ragdoll) = state.ragdoll {
            let mut pin_changes: Vec<(usize, bool)> = Vec::new();
            for (i, body) in ragdoll.bodies.iter().enumerate() {
                if i > 8 { // Show first 9, then collapse
                    ui.label(RichText::new(format!("  ... +{} corps", ragdoll.num_bodies() - 9))
                        .size(9.0).color(accent::DIM));
                    break;
                }
                let bone_name = state.active_model.and_then(|idx|
                    state.loaded_models.get(idx)
                        .and_then(|a| a.model.joint_names.get(body.bone_index))
                        .map(|s| s.as_str())
                ).unwrap_or("?");

                ui.horizontal(|ui| {
                    let mut pinned = body.pinned;
                    if ui.checkbox(&mut pinned, RichText::new(
                        format!("{} [{}]", bone_name, i)
                    ).size(9.5)).changed() {
                        pin_changes.push((i, pinned));
                    }
                });
            }
            for (idx, pinned) in pin_changes {
                ragdoll.set_pinned(idx, pinned);
            }
        }

        ui.separator();
    }

    // ── Configuration ────────────────────────────────────
    ui.collapsing(RichText::new("Configuration").size(10.5), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Gravité Y:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.config.gravity.y)
                .range(-50.0..=50.0).speed(0.1).fixed_decimals(2));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Amortissement:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.config.damping)
                .range(0.0..=1.0).speed(0.005).fixed_decimals(3));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Rebond:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.config.restitution)
                .range(0.0..=1.0).speed(0.01).fixed_decimals(2));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Friction:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.config.friction)
                .range(0.0..=1.0).speed(0.01).fixed_decimals(2));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Itérations solveur:").size(9.5));
            let mut iters = panel.config.solver_iterations as i32;
            if ui.add(egui::DragValue::new(&mut iters).range(1..=32).speed(0.2)).changed() {
                panel.config.solver_iterations = iters.max(1) as usize;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Sous-pas:").size(9.5));
            let mut sub = panel.config.substeps as i32;
            if ui.add(egui::DragValue::new(&mut sub).range(1..=16).speed(0.2)).changed() {
                panel.config.substeps = sub.max(1) as usize;
            }
        });

        // Apply config to active ragdoll
        if ui.button(RichText::new("Appliquer config").size(9.5)).clicked() {
            if let Some(ref mut ragdoll) = state.ragdoll {
                ragdoll.config = panel.config.clone();
                panel.status = "Configuration appliquée".to_string();
            }
        }
    });

    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
    }
}
