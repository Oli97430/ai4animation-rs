//! Pose Editor panel — interactive bone transform editing with auto-key.
//!
//! Shows the selected bone's position and rotation as editable fields.
//! Changes are applied to the scene in real-time, and auto-keyed into
//! the active animation's current frame.

use egui::{Ui, RichText};
use glam::{Vec3, Mat4, Quat};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_math::transform::Transform;

/// Persistent state for the pose editor panel.
pub struct PoseEditorPanel {
    /// If true, every manipulation is recorded to the animation frame.
    pub always_key: bool,
    /// Display rotation as euler angles (degrees).
    pub euler_x: f32,
    pub euler_y: f32,
    pub euler_z: f32,
    /// Cached position for editing.
    pub edit_pos: [f32; 3],
    /// Last synced entity id (to detect selection changes).
    pub last_entity: Option<usize>,
    /// Mirror edits to the symmetric bone.
    pub mirror_edits: bool,
    /// Status message.
    pub status: String,
}

impl Default for PoseEditorPanel {
    fn default() -> Self {
        Self {
            always_key: true,
            euler_x: 0.0,
            euler_y: 0.0,
            euler_z: 0.0,
            edit_pos: [0.0; 3],
            last_entity: None,
            mirror_edits: false,
            status: String::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut PoseEditorPanel) {
    ui.label(RichText::new("Édition de pose").size(13.0).color(accent::TEXT));
    ui.separator();

    // Settings
    ui.horizontal(|ui| {
        ui.checkbox(&mut panel.always_key, RichText::new("Auto-clé").size(10.0));
        ui.checkbox(&mut panel.mirror_edits, RichText::new("Miroir").size(10.0));
        ui.checkbox(&mut state.auto_key, RichText::new("Auto-key global").size(10.0));
    });

    ui.separator();

    // ── Selected bone info ─────────────────────────────
    let selected_eid = state.scene.selected;
    if selected_eid.is_none() {
        ui.label(RichText::new("Aucun os sélectionné").size(10.5).color(accent::MUTED));
        ui.label(RichText::new("Cliquez sur un os dans le viewport pour le sélectionner")
            .size(9.5).color(accent::DIM));
        return;
    }
    let eid = selected_eid.unwrap();

    // Get bone info
    let bone_name = state.scene.get_entity(eid).name.clone();
    let transform = state.scene.get_transform(eid);
    let position = transform.get_position();

    // Decompose rotation to euler
    let (scale, rotation, _translation) = transform.to_scale_rotation_translation();
    let euler = rotation.to_euler(glam::EulerRot::XYZ);

    // Sync on selection change
    if panel.last_entity != Some(eid) {
        panel.last_entity = Some(eid);
        panel.edit_pos = [position.x, position.y, position.z];
        panel.euler_x = euler.0.to_degrees();
        panel.euler_y = euler.1.to_degrees();
        panel.euler_z = euler.2.to_degrees();
    }

    // Bone name and info
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Os: {}", bone_name)).size(11.0).color(accent::TEXT));

        // Find which model/joint this belongs to
        let mut found_info = None;
        for (mi, asset) in state.loaded_models.iter().enumerate() {
            if let Some(ji) = asset.joint_entity_ids.iter().position(|&e| e == eid) {
                found_info = Some((mi, ji));
                break;
            }
        }
        if let Some((mi, ji)) = found_info {
            ui.label(RichText::new(format!("[modèle {} joint {}]", mi, ji))
                .size(9.0).color(accent::DIM));
        }
    });

    // Frame info
    let frame = state.current_frame();
    let total = state.total_frames();
    ui.label(RichText::new(format!("Frame: {}/{}", frame, total)).size(10.0).color(accent::MUTED));

    ui.add_space(4.0);

    // ── Position editing ───────────────────────────────
    ui.label(RichText::new("Position").size(10.5).color(accent::MUTED));
    let mut pos_changed = false;

    ui.horizontal(|ui| {
        ui.colored_label(accent::AXIS_X, RichText::new("X").size(10.0));
        if ui.add(egui::DragValue::new(&mut panel.edit_pos[0]).speed(0.01).prefix("")).changed() {
            pos_changed = true;
        }
        ui.colored_label(accent::AXIS_Y, RichText::new("Y").size(10.0));
        if ui.add(egui::DragValue::new(&mut panel.edit_pos[1]).speed(0.01).prefix("")).changed() {
            pos_changed = true;
        }
        ui.colored_label(accent::AXIS_Z, RichText::new("Z").size(10.0));
        if ui.add(egui::DragValue::new(&mut panel.edit_pos[2]).speed(0.01).prefix("")).changed() {
            pos_changed = true;
        }
    });

    // ── Rotation editing (Euler degrees) ───────────────
    ui.label(RichText::new("Rotation (degrés)").size(10.5).color(accent::MUTED));
    let mut rot_changed = false;

    ui.horizontal(|ui| {
        ui.colored_label(accent::AXIS_X, RichText::new("X").size(10.0));
        if ui.add(egui::DragValue::new(&mut panel.euler_x).speed(0.5).suffix("°")).changed() {
            rot_changed = true;
        }
        ui.colored_label(accent::AXIS_Y, RichText::new("Y").size(10.0));
        if ui.add(egui::DragValue::new(&mut panel.euler_y).speed(0.5).suffix("°")).changed() {
            rot_changed = true;
        }
        ui.colored_label(accent::AXIS_Z, RichText::new("Z").size(10.0));
        if ui.add(egui::DragValue::new(&mut panel.euler_z).speed(0.5).suffix("°")).changed() {
            rot_changed = true;
        }
    });

    // Apply changes
    if pos_changed || rot_changed {
        let new_rotation = Quat::from_euler(
            glam::EulerRot::XYZ,
            panel.euler_x.to_radians(),
            panel.euler_y.to_radians(),
            panel.euler_z.to_radians(),
        );
        let new_transform = Mat4::from_scale_rotation_translation(
            scale,
            new_rotation,
            Vec3::new(panel.edit_pos[0], panel.edit_pos[1], panel.edit_pos[2]),
        );
        state.scene.set_transform(eid, new_transform, true);

        // Auto-key if enabled
        if panel.always_key || state.auto_key {
            state.record_auto_key_joint(eid);
            panel.status = format!("Clé enregistrée: frame {}", frame);
        }
    }

    ui.add_space(6.0);
    ui.separator();

    // ── Quick actions ──────────────────────────────────
    ui.horizontal(|ui| {
        if ui.button(RichText::new("Réinitialiser").size(10.0)).clicked() {
            // Reset to the animation's original pose for this frame
            if let Some(idx) = state.active_model {
                if let Some(ref motion) = state.loaded_models[idx].motion {
                    let transforms = motion.get_transforms_interpolated(state.timestamp, state.mirrored);
                    // Find which joint this entity is
                    if let Some(ji) = state.loaded_models[idx].joint_entity_ids.iter().position(|&e| e == eid) {
                        if ji < transforms.len() {
                            state.scene.set_transform(eid, transforms[ji], true);
                            let pos = transforms[ji].get_position();
                            panel.edit_pos = [pos.x, pos.y, pos.z];
                            let (_, rot, _) = transforms[ji].to_scale_rotation_translation();
                            let e = rot.to_euler(glam::EulerRot::XYZ);
                            panel.euler_x = e.0.to_degrees();
                            panel.euler_y = e.1.to_degrees();
                            panel.euler_z = e.2.to_degrees();
                            panel.status = "Pose réinitialisée".to_string();
                        }
                    }
                }
            }
        }

        if ui.button(RichText::new("Clé manuelle").size(10.0)).clicked() {
            state.record_auto_key_joint(eid);
            panel.status = format!("Clé enregistrée: frame {}", frame);
            state.log_info(&format!("[Pose] Clé manuelle: {} frame {}", bone_name, frame));
        }

        if ui.button(RichText::new("Copier pose").size(10.0)).clicked() {
            state.copy_pose();
            panel.status = "Pose copiée".to_string();
        }

        if ui.button(RichText::new("Coller pose").size(10.0)).clicked() {
            state.paste_pose();
            panel.status = "Pose collée".to_string();
        }
    });

    // ── Bone hierarchy context ─────────────────────────
    ui.add_space(4.0);
    if let Some(idx) = state.active_model {
        let asset = &state.loaded_models[idx];
        if let Some(ji) = asset.joint_entity_ids.iter().position(|&e| e == eid) {
            let parent_idx = if let Some(ref motion) = asset.motion {
                let pi = motion.hierarchy.parent_indices[ji];
                if pi >= 0 { Some(pi as usize) } else { None }
            } else { None };

            ui.collapsing(RichText::new("Hiérarchie").size(10.0), |ui| {
                // Parent
                if let Some(pi) = parent_idx {
                    let parent_name = asset.model.joint_names.get(pi)
                        .map(|s| s.as_str()).unwrap_or("?");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Parent:").size(9.5).color(accent::DIM));
                        if ui.button(RichText::new(parent_name).size(9.5)).clicked() {
                            if pi < asset.joint_entity_ids.len() {
                                state.scene.selected = Some(asset.joint_entity_ids[pi]);
                            }
                        }
                    });
                }
                // Children
                let children: Vec<usize> = asset.model.parent_indices.iter().enumerate()
                    .filter(|(_, &pi)| pi == ji as i32)
                    .map(|(i, _)| i)
                    .collect();
                if !children.is_empty() {
                    ui.label(RichText::new("Enfants:").size(9.5).color(accent::DIM));
                    for ci in children {
                        let child_name = asset.model.joint_names.get(ci)
                            .map(|s| s.as_str()).unwrap_or("?");
                        if ui.button(RichText::new(child_name).size(9.5)).clicked() {
                            if ci < asset.joint_entity_ids.len() {
                                state.scene.selected = Some(asset.joint_entity_ids[ci]);
                            }
                        }
                    }
                }
            });
        }
    }

    // Status
    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
    }
}
