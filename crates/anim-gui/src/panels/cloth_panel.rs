//! Cloth / Soft-body simulation panel.

use egui::{Ui, RichText, Color32};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_animation::cloth::ClothSim;

/// Persistent panel state.
pub struct ClothPanel {
    /// Grid width for new cloth.
    pub grid_w: usize,
    /// Grid height for new cloth.
    pub grid_h: usize,
    /// Cloth size in meters.
    pub cloth_size: f32,
    pub status: String,
}

impl Default for ClothPanel {
    fn default() -> Self {
        Self {
            grid_w: 12,
            grid_h: 12,
            cloth_size: 1.5,
            status: String::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut ClothPanel) {
    ui.label(RichText::new("Simulation tissu / soft-body").size(13.0).color(accent::TEXT));
    ui.separator();

    let has_cloth = state.cloth_sim.is_some();

    if !has_cloth {
        // Create cloth controls
        ui.label(RichText::new("Créer un tissu").size(11.0).color(accent::MUTED));

        ui.horizontal(|ui| {
            ui.label(RichText::new("Grille:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.grid_w).range(3..=50).speed(1).suffix("×"));
            ui.add(egui::DragValue::new(&mut panel.grid_h).range(3..=50).speed(1));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Taille:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.cloth_size)
                .range(0.1..=10.0).speed(0.1).suffix("m"));
        });

        if ui.button(RichText::new("🧵 Créer grille").size(10.5)).clicked() {
            let origin = glam::Vec3::new(-panel.cloth_size * 0.5, 2.0, -panel.cloth_size * 0.5);
            let right = glam::Vec3::new(panel.cloth_size, 0.0, 0.0);
            let down = glam::Vec3::new(0.0, 0.0, panel.cloth_size);
            let mut cloth = ClothSim::new_grid(origin, right, down, panel.grid_w, panel.grid_h);
            cloth.pin_top_row();
            state.cloth_sim = Some(cloth);
            panel.status = format!("Tissu {}×{} créé", panel.grid_w, panel.grid_h);
            state.log_info(&format!("[Cloth] Grille {}×{} créée ({} particules)",
                panel.grid_w, panel.grid_h, panel.grid_w * panel.grid_h));
        }

        if ui.button(RichText::new("🔗 Créer chaîne (corde)").size(10.5)).clicked() {
            let points: Vec<glam::Vec3> = (0..20)
                .map(|i| glam::Vec3::new(0.0, 2.0 - i as f32 * 0.08, 0.0))
                .collect();
            let cloth = ClothSim::new_chain(&points);
            state.cloth_sim = Some(cloth);
            panel.status = "Chaîne créée (20 particules)".to_string();
            state.log_info("[Cloth] Chaîne 20 particules créée");
        }
    } else {
        // Active cloth controls — read flags first, then mutate
        let is_active = state.cloth_sim.as_ref().unwrap().active;
        let n_particles = state.cloth_sim.as_ref().unwrap().num_particles();
        let n_constraints = state.cloth_sim.as_ref().unwrap().constraints.len();

        let mut do_destroy = false;

        ui.horizontal(|ui| {
            let label = if is_active { "⏸ Pause" } else { "▶ Reprendre" };
            if ui.button(RichText::new(label).size(10.5)).clicked() {
                if let Some(ref mut c) = state.cloth_sim {
                    c.active = !c.active;
                }
            }
            if ui.button(RichText::new("🔄 Reset").size(10.5)).clicked() {
                if let Some(ref mut c) = state.cloth_sim {
                    c.reset();
                }
            }
            if ui.button(RichText::new("✕ Supprimer").size(10.0).color(accent::ERROR)).clicked() {
                do_destroy = true;
            }
        });

        if do_destroy {
            state.cloth_sim = None;
            panel.status = "Tissu supprimé".to_string();
            return;
        }

        // Info
        ui.horizontal(|ui| {
            let dot = if is_active {
                Color32::from_rgb(80, 200, 120)
            } else {
                Color32::from_rgb(200, 200, 60)
            };
            ui.label(RichText::new("●").size(10.0).color(dot));
            ui.label(RichText::new(format!(
                "{} particules  |  {} contraintes",
                n_particles, n_constraints
            )).size(9.5).color(accent::MUTED));
        });

        ui.separator();

        // Configuration
        if let Some(ref mut cloth) = state.cloth_sim {
            ui.collapsing(RichText::new("Configuration").size(10.5), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Gravité Y:").size(9.5));
                    ui.add(egui::DragValue::new(&mut cloth.config.gravity.y)
                        .range(-30.0..=0.0).speed(0.5));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Amortissement:").size(9.5));
                    ui.add(egui::Slider::new(&mut cloth.config.damping, 0.0..=0.5).step_by(0.01));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Rigidité:").size(9.5));
                    ui.add(egui::Slider::new(&mut cloth.config.stiffness, 0.0..=1.0).step_by(0.01));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Itérations:").size(9.5));
                    ui.add(egui::DragValue::new(&mut cloth.config.iterations)
                        .range(1..=20).speed(1));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sol Y:").size(9.5));
                    ui.add(egui::DragValue::new(&mut cloth.config.ground_y)
                        .range(-10.0..=10.0).speed(0.1));
                });

                // Wind
                ui.add_space(4.0);
                ui.label(RichText::new("Vent:").size(9.5));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("X:").size(9.0));
                    ui.add(egui::DragValue::new(&mut cloth.config.wind.x)
                        .range(-20.0..=20.0).speed(0.5));
                    ui.label(RichText::new("Z:").size(9.0));
                    ui.add(egui::DragValue::new(&mut cloth.config.wind.z)
                        .range(-20.0..=20.0).speed(0.5));
                });
            });
        }
    }

    // Status
    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
    }
}
