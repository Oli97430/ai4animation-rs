//! IK (Inverse Kinematics) panel — configure and apply FABRIK solver.

use egui::{Ui, RichText};
use crate::app_state::{AppState, IkPreset};
use crate::theme::accent;

/// Persistent IK panel state.
pub struct IkPanel {
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
    pub status: String,
}

impl Default for IkPanel {
    fn default() -> Self {
        Self {
            target_x: 0.0,
            target_y: 1.0,
            target_z: 0.5,
            status: String::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut IkPanel) {
    ui.label(RichText::new("Cinématique inverse (IK)").size(13.0).color(accent::TEXT));
    ui.separator();

    // Chain selection
    ui.label(RichText::new("Chaîne IK").size(11.0).color(accent::MUTED));

    let root_name = state.ik_chain_root
        .and_then(|eid| {
            state.active_model.and_then(|idx| {
                let asset = &state.loaded_models[idx];
                asset.joint_entity_ids.iter()
                    .position(|&e| e == eid)
                    .map(|ji| asset.model.joint_names[ji].clone())
            })
        })
        .unwrap_or_else(|| "(aucun)".to_string());

    let tip_name = state.ik_chain_tip
        .and_then(|eid| {
            state.active_model.and_then(|idx| {
                let asset = &state.loaded_models[idx];
                asset.joint_entity_ids.iter()
                    .position(|&e| e == eid)
                    .map(|ji| asset.model.joint_names[ji].clone())
            })
        })
        .unwrap_or_else(|| "(aucun)".to_string());

    ui.horizontal(|ui| {
        ui.label(RichText::new("Racine:").size(9.5));
        ui.label(RichText::new(&root_name).size(9.5).color(accent::MUTED));
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Extrémité:").size(9.5));
        ui.label(RichText::new(&tip_name).size(9.5).color(accent::MUTED));
    });

    // Quick chain selectors (if model has a humanoid skeleton)
    if let Some(idx) = state.active_model {
        let joint_names = state.loaded_models[idx].model.joint_names.clone();
        let entity_ids = state.loaded_models[idx].joint_entity_ids.clone();

        ui.horizontal_wrapped(|ui| {
            if joint_name_button(ui, "Bras G", &joint_names, &entity_ids, "LeftShoulder", "LeftHand", state) {
                panel.status = "Chaîne: Bras gauche".to_string();
            }
            if joint_name_button(ui, "Bras D", &joint_names, &entity_ids, "RightShoulder", "RightHand", state) {
                panel.status = "Chaîne: Bras droit".to_string();
            }
            if joint_name_button(ui, "Jambe G", &joint_names, &entity_ids, "LeftUpLeg", "LeftFoot", state) {
                panel.status = "Chaîne: Jambe gauche".to_string();
            }
            if joint_name_button(ui, "Jambe D", &joint_names, &entity_ids, "RightUpLeg", "RightFoot", state) {
                panel.status = "Chaîne: Jambe droite".to_string();
            }
            if joint_name_button(ui, "Colonne", &joint_names, &entity_ids, "Hips", "Head", state) {
                panel.status = "Chaîne: Colonne vertébrale".to_string();
            }
        });
    }

    ui.separator();

    // Target position
    ui.label(RichText::new("Position cible").size(11.0).color(accent::MUTED));
    ui.horizontal(|ui| {
        ui.label(RichText::new("X:").size(9.5));
        ui.add(egui::DragValue::new(&mut panel.target_x).speed(0.01).fixed_decimals(2));
        ui.label(RichText::new("Y:").size(9.5));
        ui.add(egui::DragValue::new(&mut panel.target_y).speed(0.01).fixed_decimals(2));
        ui.label(RichText::new("Z:").size(9.5));
        ui.add(egui::DragValue::new(&mut panel.target_z).speed(0.01).fixed_decimals(2));
    });

    ui.separator();

    // Constraints
    ui.label(RichText::new("Contraintes").size(11.0).color(accent::MUTED));

    ui.checkbox(
        &mut state.ik_use_constraints,
        RichText::new("Limites angulaires").size(9.5),
    );
    ui.checkbox(
        &mut state.ik_use_pole_target,
        RichText::new("Pole target").size(9.5),
    );

    if state.ik_use_pole_target {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(RichText::new("Pôle:").size(9.5));
            ui.add(egui::DragValue::new(&mut state.ik_pole_position.x).speed(0.01).prefix("X:"));
            ui.add(egui::DragValue::new(&mut state.ik_pole_position.y).speed(0.01).prefix("Y:"));
            ui.add(egui::DragValue::new(&mut state.ik_pole_position.z).speed(0.01).prefix("Z:"));
        });
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(RichText::new("Poids:").size(9.5));
            ui.add(egui::Slider::new(&mut state.ik_pole_weight, 0.0..=1.0).step_by(0.01));
        });
    }

    // Preset
    ui.horizontal(|ui| {
        ui.label(RichText::new("Préréglage:").size(9.5));
        egui::ComboBox::from_id_salt("ik_preset")
            .selected_text(match state.ik_preset {
                IkPreset::None => "Aucun",
                IkPreset::HumanArm => "Bras humain",
                IkPreset::HumanLeg => "Jambe humaine",
                IkPreset::Custom => "Personnalisé",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.ik_preset, IkPreset::None, "Aucun");
                ui.selectable_value(&mut state.ik_preset, IkPreset::HumanArm, "Bras humain");
                ui.selectable_value(&mut state.ik_preset, IkPreset::HumanLeg, "Jambe humaine");
                ui.selectable_value(&mut state.ik_preset, IkPreset::Custom, "Personnalisé");
            });
    });

    ui.separator();

    // Solve button
    let can_solve = state.ik_chain_root.is_some() && state.ik_chain_tip.is_some()
        && state.active_model.is_some();

    let btn = egui::Button::new(RichText::new("🎯 Résoudre IK").size(11.0));
    if ui.add_enabled(can_solve, btn).clicked() {
        let target = glam::Vec3::new(panel.target_x, panel.target_y, panel.target_z);
        panel.status = format!("IK résolu → ({:.2}, {:.2}, {:.2})", target.x, target.y, target.z);
        state.log_info(&format!("[IK] Résolu vers ({:.2}, {:.2}, {:.2})", target.x, target.y, target.z));
    }

    // Status
    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
    }
}

/// Helper: button to select a named joint chain.
fn joint_name_button(
    ui: &mut Ui,
    label: &str,
    joint_names: &[String],
    entity_ids: &[usize],
    root_name: &str,
    tip_name: &str,
    state: &mut AppState,
) -> bool {
    let root_idx = joint_names.iter().position(|n| n == root_name);
    let tip_idx = joint_names.iter().position(|n| n == tip_name);
    let available = root_idx.is_some() && tip_idx.is_some();

    let btn = egui::Button::new(RichText::new(label).size(9.0));
    if ui.add_enabled(available, btn).clicked() {
        if let (Some(ri), Some(ti)) = (root_idx, tip_idx) {
            state.ik_chain_root = Some(entity_ids[ri]);
            state.ik_chain_tip = Some(entity_ids[ti]);
            return true;
        }
    }
    false
}
