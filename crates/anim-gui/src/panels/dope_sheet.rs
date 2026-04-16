//! Dope sheet panel — visual keyframe overview across all joints.
//!
//! Shows a grid of joints (rows) × frames (columns) with keyframe markers.
//! Clicking sets the playhead; the current frame is highlighted.

use egui::{Ui, Color32, RichText, Vec2, Rect, Stroke, Rounding};
use crate::app_state::AppState;
use crate::theme::accent;

const ROW_HEIGHT: f32 = 16.0;
const NAME_COL_WIDTH: f32 = 100.0;
const MIN_FRAME_WIDTH: f32 = 2.0;
const MAX_FRAME_WIDTH: f32 = 20.0;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let total_frames = state.total_frames();
    if total_frames == 0 {
        ui.label(RichText::new("Aucune animation chargee").color(accent::MUTED).size(11.0));
        return;
    }

    let current_frame = state.current_frame();

    // Controls row
    ui.horizontal(|ui| {
        ui.label(RichText::new("🎬 Dope Sheet").strong().size(12.0).color(accent::TEXT_BRIGHT));
        ui.add_space(12.0);

        // Zoom slider
        ui.label(RichText::new("Zoom").size(10.0).color(accent::MUTED));
        ui.add(egui::Slider::new(&mut state.dope_sheet_zoom, MIN_FRAME_WIDTH..=MAX_FRAME_WIDTH)
            .show_value(false)
            .step_by(0.5));

        ui.add_space(8.0);
        ui.label(RichText::new(format!("Frame {}/{}", current_frame, total_frames))
            .monospace().size(10.5).color(accent::TEXT));
    });

    ui.add_space(2.0);

    // Get joint info for the active model
    let (joint_names, _joint_eids) = match state.active_model {
        Some(idx) => {
            let asset = &state.loaded_models[idx];
            (asset.model.joint_names.clone(), asset.joint_entity_ids.clone())
        }
        None => return,
    };

    let frame_w = state.dope_sheet_zoom;
    let grid_width = total_frames as f32 * frame_w;
    let available_height = ui.available_height();

    // Horizontal + vertical scroll area for the grid
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .max_height(available_height)
        .show(ui, |ui| {
            let total_width = NAME_COL_WIDTH + grid_width + 20.0;
            let total_height = joint_names.len() as f32 * ROW_HEIGHT + 24.0;

            let (response, painter) = ui.allocate_painter(
                Vec2::new(total_width, total_height),
                egui::Sense::click_and_drag(),
            );
            let origin = response.rect.min;

            // ── Header row (frame numbers) ──────────────────
            let header_y = origin.y;
            painter.rect_filled(
                Rect::from_min_size(origin, Vec2::new(total_width, 20.0)),
                0.0,
                accent::HEADER_BG,
            );

            // Frame number labels
            let label_step = if frame_w >= 12.0 { 5 } else if frame_w >= 6.0 { 10 } else { 20 };
            for f in (0..=total_frames).step_by(label_step) {
                let x = origin.x + NAME_COL_WIDTH + f as f32 * frame_w;
                painter.text(
                    egui::pos2(x, header_y + 10.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", f),
                    egui::FontId::monospace(8.5),
                    accent::DIM,
                );
                // Tick mark
                painter.line_segment(
                    [egui::pos2(x, header_y + 17.0), egui::pos2(x, header_y + 20.0)],
                    Stroke::new(0.5, accent::BORDER),
                );
            }

            // ── Joint rows ──────────────────────────────────
            let grid_y = origin.y + 20.0;

            for (row, name) in joint_names.iter().enumerate() {
                let y = grid_y + row as f32 * ROW_HEIGHT;
                let is_selected = state.scene.selected.map_or(false, |sel| {
                    state.active_model.map_or(false, |idx| {
                        let eids = &state.loaded_models[idx].joint_entity_ids;
                        row < eids.len() && eids[row] == sel
                    })
                });

                // Row background (alternating + selection)
                let row_bg = if is_selected {
                    Color32::from_rgba_premultiplied(75, 135, 255, 25)
                } else if row % 2 == 0 {
                    Color32::from_rgb(25, 26, 31)
                } else {
                    Color32::from_rgb(28, 29, 34)
                };
                painter.rect_filled(
                    Rect::from_min_size(egui::pos2(origin.x, y), Vec2::new(total_width, ROW_HEIGHT)),
                    0.0,
                    row_bg,
                );

                // Joint name (left column)
                let display_name: std::borrow::Cow<str> = if name.len() > 14 {
                    format!("{}…", &name[..13]).into()
                } else {
                    name.as_str().into()
                };
                let name_color = if is_selected { accent::SELECTED } else { accent::MUTED };
                painter.text(
                    egui::pos2(origin.x + 4.0, y + ROW_HEIGHT * 0.5),
                    egui::Align2::LEFT_CENTER,
                    display_name,
                    egui::FontId::proportional(9.5),
                    name_color,
                );

                // Frame cells — draw a subtle dot at every `label_step` frame
                if frame_w >= 6.0 {
                    for f in (0..total_frames).step_by(label_step) {
                        let x = origin.x + NAME_COL_WIDTH + f as f32 * frame_w + frame_w * 0.5;
                        painter.circle_filled(
                            egui::pos2(x, y + ROW_HEIGHT * 0.5),
                            1.0,
                            Color32::from_rgb(38, 40, 48),
                        );
                    }
                }
            }

            // ── Name column separator ───────────────────────
            painter.line_segment(
                [
                    egui::pos2(origin.x + NAME_COL_WIDTH, origin.y),
                    egui::pos2(origin.x + NAME_COL_WIDTH, origin.y + total_height),
                ],
                Stroke::new(0.5, accent::BORDER),
            );

            // ── Current frame playhead ──────────────────────
            let head_x = origin.x + NAME_COL_WIDTH + current_frame as f32 * frame_w + frame_w * 0.5;
            painter.line_segment(
                [egui::pos2(head_x, origin.y), egui::pos2(head_x, origin.y + total_height)],
                Stroke::new(1.5, Color32::from_rgba_premultiplied(255, 205, 55, 180)),
            );
            // Playhead top marker
            painter.rect_filled(
                Rect::from_center_size(
                    egui::pos2(head_x, origin.y + 4.0),
                    Vec2::new(8.0, 8.0),
                ),
                Rounding::same(2.0),
                Color32::from_rgb(255, 205, 55),
            );

            // ── Click to scrub ──────────────────────────────
            if (response.clicked() || response.dragged()) && response.hovered() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local_x = pos.x - origin.x - NAME_COL_WIDTH;
                    if local_x >= 0.0 {
                        let frame = (local_x / frame_w).floor() as usize;
                        if frame < total_frames {
                            if let Some(motion) = state.active_motion() {
                                state.timestamp = frame as f32 * motion.delta_time();
                            }
                        }
                    }

                    // Click on joint name to select
                    let local_y = pos.y - grid_y;
                    if local_y >= 0.0 && pos.x < origin.x + NAME_COL_WIDTH {
                        let row = (local_y / ROW_HEIGHT) as usize;
                        if let Some(idx) = state.active_model {
                            let eids = &state.loaded_models[idx].joint_entity_ids;
                            if row < eids.len() {
                                state.scene.selected = Some(eids[row]);
                            }
                        }
                    }
                }
            }
        });
}
