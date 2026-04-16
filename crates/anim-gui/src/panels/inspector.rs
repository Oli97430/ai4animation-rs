//! Inspector panel — editable properties with SketchUp-style drag values.
//!
//! Color-coded axis fields, collapsible sections, professional layout.

use egui::{Ui, Color32, RichText, DragValue, Vec2};
use glam::{Vec3, Quat, EulerRot, Mat4};
use anim_core::i18n::t;
use anim_math::transform::Transform;
use crate::app_state::{AppState, UndoSnapshot};
use crate::theme::{accent, section_header, thin_separator};

/// Draw one axis row: colored label + DragValue.
fn axis_field(ui: &mut Ui, label: &str, value: &mut f32, color: Color32, speed: f64, decimals: usize, suffix: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        // Colored axis label pill
        let (_, label_rect) = ui.allocate_space(Vec2::new(16.0, 18.0));
        let painter = ui.painter();
        painter.rect_filled(label_rect, 3.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 35));
        painter.text(
            label_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::monospace(10.5),
            color,
        );

        // Value field
        let mut dv = DragValue::new(value)
            .speed(speed)
            .fixed_decimals(decimals)
            .min_decimals(decimals);
        if !suffix.is_empty() {
            dv = dv.suffix(suffix);
        }
        if ui.add_sized(Vec2::new(ui.available_width(), 18.0), dv).changed() {
            changed = true;
        }
    });
    changed
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    // ── Panel Header ────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(RichText::new("🔎").size(14.0));
        ui.label(RichText::new(t("inspector")).strong().size(14.0).color(accent::TEXT_BRIGHT));
    });
    thin_separator(ui);

    // ── Multi-personnages ──────────────────────────────────
    if state.loaded_models.len() > 1 {
        ui.collapsing(RichText::new("👥 Multi-personnages").size(11.0).color(accent::TEXT), |ui| {
            for mi in 0..state.loaded_models.len() {
                let is_active = state.active_model == Some(mi);
                let name = state.loaded_models[mi].name.clone();
                let has_motion = state.loaded_models[mi].motion.is_some();

                ui.horizontal(|ui| {
                    // Select button
                    let label = if is_active {
                        RichText::new(format!("▶ {} [{}]", name, mi)).size(10.0).color(accent::SUCCESS)
                    } else {
                        RichText::new(format!("  {} [{}]", name, mi)).size(10.0)
                    };
                    if ui.button(label).clicked() {
                        state.active_model = Some(mi);
                        state.timestamp = 0.0;
                    }

                    // Visibility toggle
                    let vis_icon = if state.loaded_models[mi].visible { "👁" } else { "◌" };
                    if ui.button(RichText::new(vis_icon).size(10.0)).clicked() {
                        state.loaded_models[mi].visible = !state.loaded_models[mi].visible;
                    }
                });

                // Independent playback controls (only for models with animation)
                if has_motion && !is_active {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        let asset = &mut state.loaded_models[mi];
                        ui.checkbox(&mut asset.independent_playback,
                            RichText::new("Indépendant").size(9.0));
                        if asset.independent_playback {
                            if ui.button(RichText::new(
                                if asset.local_playing { "⏸" } else { "▶" }
                            ).size(9.0)).clicked() {
                                asset.local_playing = !asset.local_playing;
                            }
                            ui.add(egui::DragValue::new(&mut asset.local_speed)
                                .range(0.1..=5.0).speed(0.05)
                                .suffix("x").fixed_decimals(1));
                        }
                    });

                    // World offset
                    if state.loaded_models[mi].independent_playback {
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Pos:").size(9.0).color(accent::DIM));
                            ui.add(egui::DragValue::new(&mut state.loaded_models[mi].world_offset.x)
                                .speed(0.05).prefix("X:").fixed_decimals(1));
                            ui.add(egui::DragValue::new(&mut state.loaded_models[mi].world_offset.z)
                                .speed(0.05).prefix("Z:").fixed_decimals(1));
                        });
                    }
                }
            }
        });
        thin_separator(ui);
    }

    let selected = match state.scene.selected {
        Some(id) => id,
        None => {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("👆").size(24.0));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t("no_selection"))
                        .color(accent::MUTED)
                        .italics()
                        .size(11.5)
                );
                ui.label(
                    RichText::new("Selectionnez un joint dans\nle viewport ou la hierarchie")
                        .color(accent::DIM)
                        .size(10.5)
                );
            });
            return;
        }
    };

    let entity = state.scene.get_entity(selected);
    let name = entity.name.clone();

    // ── Entity Name Card ────────────────────────────────────
    egui::Frame::none()
        .fill(accent::SECTION_BG)
        .rounding(6.0)
        .inner_margin(8.0)
        .stroke(egui::Stroke::new(0.5, accent::BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("◆").size(10.0).color(accent::SELECTED));
                ui.label(RichText::new(&name).strong().size(13.0).color(accent::TEXT_BRIGHT));
            });

            // Parent + children info inline
            let entity = state.scene.get_entity(selected);
            let parent_name = entity.parent
                .map(|p| state.scene.get_entity(p).name.clone())
                .unwrap_or_else(|| "---".to_string());
            let num_children = entity.children.len();

            ui.horizontal(|ui| {
                ui.label(RichText::new("↑").size(9.0).color(accent::DIM));
                ui.label(RichText::new(&parent_name).size(10.5).color(accent::MUTED));
                ui.label(RichText::new("·").color(accent::DIM));
                ui.label(RichText::new(format!("{} enfants", num_children)).size(10.5).color(accent::MUTED));
            });
        });

    ui.add_space(2.0);

    // ── Transform Section ───────────────────────────────────
    section_header(ui, "✥", t("transform"), accent::PRIMARY);

    let transform = state.scene.get_transform(selected);
    let pos = transform.get_position();
    let rot_mat = transform.get_rotation();
    let quat = Quat::from_mat3(&rot_mat);
    let (ry, rx, rz) = quat.to_euler(EulerRot::YXZ);
    let scale = state.scene.scales[selected];

    // Position
    egui::Frame::none()
        .fill(Color32::from_rgb(24, 25, 30))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.label(RichText::new(t("position")).size(10.0).color(accent::MUTED));
            let mut new_pos = pos;
            let mut changed = false;
            changed |= axis_field(ui, "X", &mut new_pos.x, accent::AXIS_X, 0.01, 3, "");
            changed |= axis_field(ui, "Y", &mut new_pos.y, accent::AXIS_Y, 0.01, 3, "");
            changed |= axis_field(ui, "Z", &mut new_pos.z, accent::AXIS_Z, 0.01, 3, "");

            if changed {
                state.history.push(UndoSnapshot {
                    description: format!("Position {}", name),
                    entity_id: selected,
                    transform,
                });
                let snapped = state.snap_position(new_pos);
                state.scene.set_position(selected, snapped, true);
            }
        });

    ui.add_space(2.0);

    // Rotation
    egui::Frame::none()
        .fill(Color32::from_rgb(24, 25, 30))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.label(RichText::new(t("rotation")).size(10.0).color(accent::MUTED));
            let mut euler_deg = [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()];
            let mut changed = false;
            changed |= axis_field(ui, "X", &mut euler_deg[0], accent::AXIS_X, 0.5, 1, "\u{00b0}");
            changed |= axis_field(ui, "Y", &mut euler_deg[1], accent::AXIS_Y, 0.5, 1, "\u{00b0}");
            changed |= axis_field(ui, "Z", &mut euler_deg[2], accent::AXIS_Z, 0.5, 1, "\u{00b0}");

            if changed {
                state.history.push(UndoSnapshot {
                    description: format!("Rotation {}", name),
                    entity_id: selected,
                    transform,
                });
                let new_quat = Quat::from_euler(
                    EulerRot::YXZ,
                    euler_deg[1].to_radians(),
                    euler_deg[0].to_radians(),
                    euler_deg[2].to_radians(),
                );
                let new_transform = Mat4::from_rotation_translation(new_quat, pos);
                state.scene.set_transform(selected, new_transform, true);
            }
        });

    ui.add_space(2.0);

    // Scale
    egui::Frame::none()
        .fill(Color32::from_rgb(24, 25, 30))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.label(RichText::new(t("scale")).size(10.0).color(accent::MUTED));
            let mut new_scale = scale;
            let mut changed = false;
            changed |= axis_field(ui, "X", &mut new_scale.x, accent::AXIS_X, 0.01, 3, "");
            changed |= axis_field(ui, "Y", &mut new_scale.y, accent::AXIS_Y, 0.01, 3, "");
            changed |= axis_field(ui, "Z", &mut new_scale.z, accent::AXIS_Z, 0.01, 3, "");

            if changed {
                state.scene.scales[selected] = new_scale;
            }
        });

    thin_separator(ui);

    // ── Quick Actions ───────────────────────────────────────
    ui.horizontal(|ui| {
        if ui.add(
            egui::Button::new(RichText::new("📍 Centrer").size(11.0))
                .rounding(4.0)
        ).clicked() {
            let pos = state.scene.get_position(selected);
            state.camera.look_at(pos);
        }
        if ui.add(
            egui::Button::new(RichText::new("↺ Réinit.").size(11.0))
                .rounding(4.0)
        ).clicked() {
            state.history.push(UndoSnapshot {
                description: format!("Réinitialiser {}", name),
                entity_id: selected,
                transform: state.scene.get_transform(selected),
            });
            state.scene.set_position(selected, Vec3::ZERO, false);
            // Also reset rotation and scale
            use anim_math::transform::Transform;
            let mut t = state.scene.get_transform(selected);
            t.set_rotation(glam::Mat3::IDENTITY);
            state.scene.transforms[selected] = t;
            state.scene.scales[selected] = Vec3::ONE;
        }
    });
}
