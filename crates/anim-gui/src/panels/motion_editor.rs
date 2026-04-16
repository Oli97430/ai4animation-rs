//! Motion Editor panel — visual asset selection, module controls, IK configuration.

use egui::{Ui, RichText, Vec2, Color32};
use crate::app_state::AppState;
use crate::theme::accent;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Éditeur de mouvement").size(11.0).color(accent::TEXT));
        ui.separator();

        // Asset selector (dropdown of loaded models)
        let active_name = state.active_model
            .and_then(|idx| state.loaded_models.get(idx))
            .map(|a| a.name.as_str())
            .unwrap_or("(aucun)");

        egui::ComboBox::from_id_salt("motion_asset_select")
            .selected_text(RichText::new(active_name).size(11.0))
            .width(180.0)
            .show_ui(ui, |ui| {
                for (i, asset) in state.loaded_models.iter().enumerate() {
                    let label = format!("{} ({} fr)", asset.name,
                        asset.motion.as_ref().map_or(0, |m| m.num_frames()));
                    if ui.selectable_label(state.active_model == Some(i),
                        RichText::new(&label).size(10.5)).clicked() {
                        state.active_model = Some(i);
                        state.timestamp = 0.0;
                    }
                }
            });

        ui.separator();

        // Frame info
        if let Some(motion) = state.active_motion() {
            let frame = motion.frame_index(state.timestamp);
            let total = motion.num_frames();
            ui.label(RichText::new(format!("{}/{}", frame, total)).size(10.5).color(accent::MUTED));

            ui.separator();

            // Frame slider (scrub through animation)
            let mut frame_f = frame as f32;
            let resp = ui.add(
                egui::Slider::new(&mut frame_f, 0.0..=(total.saturating_sub(1)) as f32)
                    .show_value(false)
                    .text("")
            );
            if resp.changed() {
                let framerate = motion.framerate;
                state.timestamp = frame_f / framerate;
                state.playing = false;
            }
        }
    });

    // Module visibility section
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Modules").size(10.0).color(accent::MUTED)); // "Modules" is the same in French
        ui.separator();

        // Module toggles
        ui.checkbox(&mut state.show_skeleton, RichText::new("Squelette").size(10.0));
        ui.checkbox(&mut state.show_mesh, RichText::new("Maillage").size(10.0));
        ui.checkbox(&mut state.show_contacts, RichText::new("Contacts").size(10.0));
        ui.checkbox(&mut state.show_trajectory, RichText::new("Trajectoire").size(10.0));
        ui.checkbox(&mut state.show_velocities, RichText::new("Velocites").size(10.0));
        ui.checkbox(&mut state.show_guidance, RichText::new("Guidage").size(10.0));
        ui.checkbox(&mut state.show_tracking, RichText::new("Suivi").size(10.0));
        ui.checkbox(&mut state.show_root_motion, RichText::new("Racine").size(10.0));
        ui.checkbox(&mut state.onion_skinning, RichText::new("Pelure").size(10.0));
    });

    // Guidance smoothing control
    if state.show_guidance {
        if let Some(ref mut gm) = state.guidance_module {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Guidance lissage:").size(10.0).color(accent::MUTED));
                ui.add(egui::DragValue::new(&mut gm.smoothing_window)
                    .range(0.0..=2.0).speed(0.05).suffix("s").fixed_decimals(1));
                ui.label(RichText::new(format!("{} points", gm.point_count())).size(10.0).color(accent::MUTED));
            });
        }
    }

    // Tracking smoothing control
    if state.show_tracking {
        if let Some(ref mut tm) = state.tracking_module {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Tracking fenetre:").size(10.0).color(accent::MUTED));
                ui.add(egui::DragValue::new(&mut tm.smoothing_window)
                    .range(0.0..=2.0).speed(0.05).suffix("s").fixed_decimals(1));
                ui.label(RichText::new(format!("{} joints", tm.joint_count())).size(10.0).color(accent::MUTED));
            });
        }
    }

    // IK constraints section (shown when IK tool is active)
    if state.active_tool == crate::app_state::Tool::Ik {
        ui.separator();
        ik_constraints_ui(ui, state);
    }

    // Retargeting section
    if state.loaded_models.len() >= 2 {
        ui.separator();
        retarget_ui(ui, state);
    }
}

/// IK constraints UI — configure joint limits and pole targets.
fn ik_constraints_ui(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("IK Contraintes").size(10.0).color(accent::MUTED));
        ui.separator();

        // Constraint toggle
        let constraint_color = if state.ik_use_constraints { accent::PRIMARY } else { accent::DIM };
        if ui.add(
            egui::Button::new(RichText::new("⚙ Limites").size(10.0).color(constraint_color))
                .fill(if state.ik_use_constraints {
                    Color32::from_rgba_premultiplied(75, 135, 255, 25)
                } else {
                    Color32::TRANSPARENT
                })
                .rounding(10.0)
        ).clicked() {
            state.ik_use_constraints = !state.ik_use_constraints;
        }

        // Pole target toggle
        let pole_color = if state.ik_use_pole_target { accent::WARNING } else { accent::DIM };
        if ui.add(
            egui::Button::new(RichText::new("◎ Pole").size(10.0).color(pole_color))
                .fill(if state.ik_use_pole_target {
                    Color32::from_rgba_premultiplied(255, 185, 50, 20)
                } else {
                    Color32::TRANSPARENT
                })
                .rounding(10.0)
        ).clicked() {
            state.ik_use_pole_target = !state.ik_use_pole_target;
        }

        // Preset selector
        ui.label(RichText::new("Preset:").size(10.0).color(accent::DIM));
        let preset_name = match state.ik_preset {
            crate::app_state::IkPreset::None => "Aucun",
            crate::app_state::IkPreset::HumanArm => "Bras",
            crate::app_state::IkPreset::HumanLeg => "Jambe",
            crate::app_state::IkPreset::Custom => "Perso.",
        };
        egui::ComboBox::from_id_salt("ik_preset")
            .selected_text(RichText::new(preset_name).size(10.0))
            .width(80.0)
            .show_ui(ui, |ui| {
                use crate::app_state::IkPreset;
                if ui.selectable_label(state.ik_preset == IkPreset::None, "Aucun").clicked() {
                    state.ik_preset = IkPreset::None;
                    state.ik_use_constraints = false;
                }
                if ui.selectable_label(state.ik_preset == IkPreset::HumanArm, "Bras humain").clicked() {
                    state.ik_preset = IkPreset::HumanArm;
                    state.ik_use_constraints = true;
                }
                if ui.selectable_label(state.ik_preset == IkPreset::HumanLeg, "Jambe humaine").clicked() {
                    state.ik_preset = IkPreset::HumanLeg;
                    state.ik_use_constraints = true;
                }
                if ui.selectable_label(state.ik_preset == IkPreset::Custom, "Personnalise").clicked() {
                    state.ik_preset = IkPreset::Custom;
                    state.ik_use_constraints = true;
                }
            });
    });

    // Pole target position controls
    if state.ik_use_pole_target {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(RichText::new("Pole:").size(9.5).color(accent::DIM));
            ui.add(egui::DragValue::new(&mut state.ik_pole_position.x).prefix("X:").speed(0.05).fixed_decimals(1));
            ui.add(egui::DragValue::new(&mut state.ik_pole_position.y).prefix("Y:").speed(0.05).fixed_decimals(1));
            ui.add(egui::DragValue::new(&mut state.ik_pole_position.z).prefix("Z:").speed(0.05).fixed_decimals(1));
            ui.add_sized(Vec2::new(60.0, 16.0),
                egui::Slider::new(&mut state.ik_pole_weight, 0.0..=1.0)
                    .text("Poids").show_value(true)
            );
        });
    }

    // Chain info
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        match (state.ik_chain_root, state.ik_chain_tip) {
            (Some(r), Some(t)) => {
                let root_name = state.scene.get_entity(r).name.clone();
                let tip_name = state.scene.get_entity(t).name.clone();
                let chain = state.scene.get_chain(r, t);
                ui.label(RichText::new(format!(
                    "Chaine: {} → {} ({} os)", root_name, tip_name, chain.len()
                )).size(9.5).color(accent::MUTED));
            }
            (Some(_), None) => {
                ui.label(RichText::new("Selectionnez le bout de la chaine (tip)")
                    .size(9.5).color(accent::WARNING));
            }
            _ => {
                ui.label(RichText::new("Selectionnez la racine puis le bout de la chaine IK")
                    .size(9.5).color(accent::DIM));
            }
        }
    });
}

/// Retargeting UI — bind a mesh to an animation source.
fn retarget_ui(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Reciblage").size(10.0).color(accent::MUTED));
        ui.separator();

        // Show existing retarget binding
        if let Some(idx) = state.active_model {
            if let Some(ref binding) = state.loaded_models[idx].retarget {
                let src = &state.loaded_models[binding.source_asset].name;
                let quality = binding.map.quality();
                ui.label(RichText::new(format!("← {} ({:.0}%)", src, quality * 100.0))
                    .size(10.0).color(egui::Color32::from_rgb(100, 220, 140)));
                if ui.small_button("✕").on_hover_text("Supprimer le retarget").clicked() {
                    state.loaded_models[idx].retarget = None;
                    state.log_info("Retarget supprime");
                }
                return;
            }
        }

        // "Bind mesh to animation" dropdowns
        // Source (animation provider)
        let models_with_anim: Vec<(usize, String)> = state.loaded_models.iter()
            .enumerate()
            .filter(|(_, a)| a.motion.is_some())
            .map(|(i, a)| (i, a.name.clone()))
            .collect();

        // Target (mesh with skin)
        let models_with_mesh: Vec<(usize, String)> = state.loaded_models.iter()
            .enumerate()
            .filter(|(_, a)| a.skinned_mesh.is_some())
            .map(|(i, a)| (i, a.name.clone()))
            .collect();

        if models_with_anim.is_empty() || models_with_mesh.is_empty() {
            ui.label(RichText::new("(besoin: 1 mesh + 1 animation)").size(9.5).color(accent::DIM));
            return;
        }

        ui.label(RichText::new("Mesh:").size(10.0).color(accent::MUTED));

        // For the mesh selector, we use a static mut-like pattern via a combo id
        let mesh_idx = state.active_model.unwrap_or(0);
        let mesh_name = state.loaded_models.get(mesh_idx).map_or("?", |a| &a.name);
        ui.label(RichText::new(mesh_name).size(10.0));

        ui.label(RichText::new("Anim:").size(10.0).color(accent::MUTED));

        // Find the first asset with animation that isn't the current one
        let mut bind_to: Option<usize> = None;
        for (i, name) in &models_with_anim {
            if *i != mesh_idx {
                if ui.small_button(RichText::new(format!("← {}", name)).size(10.0)).clicked() {
                    bind_to = Some(*i);
                }
            }
        }

        if let Some(anim_idx) = bind_to {
            state.retarget_mesh(mesh_idx, anim_idx);
        }
    });
}
