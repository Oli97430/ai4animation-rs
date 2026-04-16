//! Material editor panel — edit PBR material properties per model.

use egui::{Ui, RichText};
use crate::app_state::AppState;
use crate::theme::accent;

/// Persistent panel state.
pub struct MaterialEditorPanel {
    pub status: String,
}

impl Default for MaterialEditorPanel {
    fn default() -> Self {
        Self {
            status: String::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, _panel: &mut MaterialEditorPanel) {
    ui.label(RichText::new("Éditeur de matériaux").size(13.0).color(accent::TEXT));
    ui.separator();

    if let Some(idx) = state.active_model {
        if let Some(ref mut mesh) = state.loaded_models[idx].skinned_mesh {
            // Base color
            ui.horizontal(|ui| {
                ui.label(RichText::new("Couleur:").size(10.0));
                let mut color = [
                    (mesh.color[0] * 255.0) as u8,
                    (mesh.color[1] * 255.0) as u8,
                    (mesh.color[2] * 255.0) as u8,
                ];
                if ui.color_edit_button_srgb(&mut color).changed() {
                    mesh.color[0] = color[0] as f32 / 255.0;
                    mesh.color[1] = color[1] as f32 / 255.0;
                    mesh.color[2] = color[2] as f32 / 255.0;
                }
            });

            // Alpha
            ui.horizontal(|ui| {
                ui.label(RichText::new("Alpha:").size(10.0));
                ui.add(egui::Slider::new(&mut mesh.color[3], 0.0..=1.0).step_by(0.01));
            });

            ui.add_space(4.0);
            ui.label(RichText::new("Propriétés PBR").size(11.0).color(accent::MUTED));
            ui.separator();

            // Metallic
            ui.horizontal(|ui| {
                ui.label(RichText::new("Métallique:").size(10.0));
                ui.add(egui::Slider::new(&mut mesh.metallic, 0.0..=1.0).step_by(0.01));
            });

            // Roughness
            ui.horizontal(|ui| {
                ui.label(RichText::new("Rugosité:").size(10.0));
                ui.add(egui::Slider::new(&mut mesh.roughness, 0.0..=1.0).step_by(0.01));
            });

            // Specularity
            ui.horizontal(|ui| {
                ui.label(RichText::new("Spéculaire:").size(10.0));
                ui.add(egui::Slider::new(&mut mesh.specularity, 0.0..=1.0).step_by(0.01));
            });

            // Glossiness
            ui.horizontal(|ui| {
                ui.label(RichText::new("Brillance:").size(10.0));
                ui.add(egui::Slider::new(&mut mesh.glossiness, 0.0..=128.0).step_by(1.0));
            });

            ui.add_space(4.0);

            // Preset buttons
            ui.label(RichText::new("Préréglages").size(11.0).color(accent::MUTED));
            ui.horizontal_wrapped(|ui| {
                if ui.button(RichText::new("🪨 Pierre").size(9.5)).clicked() {
                    mesh.color = [0.5, 0.5, 0.5, 1.0];
                    mesh.metallic = 0.0;
                    mesh.roughness = 0.9;
                    mesh.specularity = 0.1;
                    mesh.glossiness = 8.0;
                }
                if ui.button(RichText::new("🥇 Or").size(9.5)).clicked() {
                    mesh.color = [1.0, 0.84, 0.0, 1.0];
                    mesh.metallic = 1.0;
                    mesh.roughness = 0.2;
                    mesh.specularity = 0.9;
                    mesh.glossiness = 64.0;
                }
                if ui.button(RichText::new("🔵 Plastique").size(9.5)).clicked() {
                    mesh.color = [0.2, 0.4, 0.8, 1.0];
                    mesh.metallic = 0.0;
                    mesh.roughness = 0.4;
                    mesh.specularity = 0.5;
                    mesh.glossiness = 32.0;
                }
                if ui.button(RichText::new("🪵 Bois").size(9.5)).clicked() {
                    mesh.color = [0.55, 0.35, 0.18, 1.0];
                    mesh.metallic = 0.0;
                    mesh.roughness = 0.8;
                    mesh.specularity = 0.15;
                    mesh.glossiness = 10.0;
                }
                if ui.button(RichText::new("🔘 Chrome").size(9.5)).clicked() {
                    mesh.color = [0.77, 0.78, 0.78, 1.0];
                    mesh.metallic = 1.0;
                    mesh.roughness = 0.05;
                    mesh.specularity = 1.0;
                    mesh.glossiness = 128.0;
                }
                if ui.button(RichText::new("🧊 Verre").size(9.5)).clicked() {
                    mesh.color = [0.9, 0.95, 1.0, 0.3];
                    mesh.metallic = 0.0;
                    mesh.roughness = 0.05;
                    mesh.specularity = 0.9;
                    mesh.glossiness = 100.0;
                }
                if ui.button(RichText::new("🧱 Brique").size(9.5)).clicked() {
                    mesh.color = [0.65, 0.25, 0.15, 1.0];
                    mesh.metallic = 0.0;
                    mesh.roughness = 0.95;
                    mesh.specularity = 0.1;
                    mesh.glossiness = 5.0;
                }
            });

            // Texture info
            ui.add_space(4.0);
            if mesh.has_texture {
                if let Some(ref tex) = mesh.texture_data {
                    ui.label(RichText::new(format!(
                        "Texture: {}×{}", tex.width, tex.height
                    )).size(9.0).color(accent::DIM));
                }
            } else {
                ui.label(RichText::new("Pas de texture").size(9.0).color(accent::DIM));
            }
        } else {
            ui.label(RichText::new("Pas de mesh sur ce modèle").size(10.0).color(accent::MUTED));
        }
    } else {
        ui.label(RichText::new("Aucun modèle actif").size(10.0).color(accent::MUTED));
    }
}
