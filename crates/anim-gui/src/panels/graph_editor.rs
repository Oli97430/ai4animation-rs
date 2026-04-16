//! Graph Editor — animation curve viewer/editor.
//!
//! Displays per-joint position and rotation curves over time,
//! allowing visual inspection and keyframe editing.

use egui::{Ui, RichText, Color32, Pos2, Vec2, Stroke};
use crate::app_state::AppState;
use crate::theme::accent;

/// Which property channel to display.
#[derive(Clone, Copy, PartialEq)]
pub enum CurveChannel {
    PosX,
    PosY,
    PosZ,
    RotX,
    RotY,
    RotZ,
}

impl CurveChannel {
    pub fn label(&self) -> &str {
        match self {
            Self::PosX => "Pos X",
            Self::PosY => "Pos Y",
            Self::PosZ => "Pos Z",
            Self::RotX => "Rot X",
            Self::RotY => "Rot Y",
            Self::RotZ => "Rot Z",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::PosX | Self::RotX => Color32::from_rgb(220, 80, 80),   // red
            Self::PosY | Self::RotY => Color32::from_rgb(80, 200, 80),   // green
            Self::PosZ | Self::RotZ => Color32::from_rgb(80, 120, 220),  // blue
        }
    }

    pub fn all() -> &'static [CurveChannel] {
        &[
            Self::PosX, Self::PosY, Self::PosZ,
            Self::RotX, Self::RotY, Self::RotZ,
        ]
    }
}

/// Persistent state for the graph editor.
pub struct GraphEditorPanel {
    /// Active channels (togglable).
    pub active_channels: Vec<bool>,
    /// Vertical zoom (value range).
    pub value_zoom: f32,
    /// Vertical offset.
    pub value_offset: f32,
    /// Time zoom (pixels per second).
    pub time_zoom: f32,
    /// Time scroll offset.
    pub time_scroll: f32,
    /// Selected joint index to display curves for.
    pub selected_joint: Option<usize>,
    /// Status text.
    pub status: String,
    /// Whether to auto-follow selected bone.
    pub auto_follow: bool,
}

impl Default for GraphEditorPanel {
    fn default() -> Self {
        Self {
            active_channels: vec![true, true, true, false, false, false], // pos XYZ on, rot off
            value_zoom: 2.0,
            value_offset: 0.0,
            time_zoom: 200.0,
            time_scroll: 0.0,
            selected_joint: None,
            status: String::new(),
            auto_follow: true,
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut GraphEditorPanel) {
    ui.label(RichText::new("Éditeur de courbes").size(13.0).color(accent::TEXT));
    ui.separator();

    // ── Toolbar ──────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.checkbox(&mut panel.auto_follow, RichText::new("Auto-suivre sélection").size(10.0));

        ui.separator();

        // Channel toggles
        for (i, ch) in CurveChannel::all().iter().enumerate() {
            let color = if panel.active_channels[i] { ch.color() } else { accent::DIM };
            if ui.button(RichText::new(ch.label()).size(9.5).color(color)).clicked() {
                panel.active_channels[i] = !panel.active_channels[i];
            }
        }

        ui.separator();

        // Zoom controls
        if ui.button(RichText::new("⊕").size(10.0)).clicked() {
            panel.time_zoom = (panel.time_zoom * 1.3).min(2000.0);
        }
        if ui.button(RichText::new("⊖").size(10.0)).clicked() {
            panel.time_zoom = (panel.time_zoom / 1.3).max(20.0);
        }
        if ui.button(RichText::new("↕").size(10.0)).clicked() {
            panel.value_zoom = (panel.value_zoom * 1.3).min(50.0);
        }
        if ui.button(RichText::new("Réinit.").size(9.5)).clicked() {
            panel.value_zoom = 2.0;
            panel.value_offset = 0.0;
            panel.time_scroll = 0.0;
            panel.time_zoom = 200.0;
        }
    });

    // ── Joint selector ──────────────────────────────────
    // Auto-follow selected bone
    if panel.auto_follow {
        if let Some(eid) = state.scene.selected {
            if let Some(idx) = state.active_model {
                let asset = &state.loaded_models[idx];
                if let Some(ji) = asset.joint_entity_ids.iter().position(|&e| e == eid) {
                    panel.selected_joint = Some(ji);
                }
            }
        }
    }

    let joint_name = if let (Some(ji), Some(idx)) = (panel.selected_joint, state.active_model) {
        state.loaded_models.get(idx)
            .and_then(|a| a.model.joint_names.get(ji))
            .cloned()
            .unwrap_or_else(|| format!("Joint {}", ji))
    } else {
        "Aucun joint".into()
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Joint: {}", joint_name)).size(10.5).color(accent::MUTED));

        // Manual joint selector
        if let Some(idx) = state.active_model {
            let joint_count = state.loaded_models[idx].model.joint_names.len();
            if joint_count > 0 {
                let current = panel.selected_joint.unwrap_or(0);
                egui::ComboBox::from_id_salt("graph_joint_sel")
                    .selected_text(RichText::new(
                        state.loaded_models[idx].model.joint_names.get(current)
                            .map(|s| s.as_str()).unwrap_or("?")
                    ).size(9.5))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for ji in 0..joint_count {
                            let name = &state.loaded_models[idx].model.joint_names[ji];
                            if ui.selectable_label(panel.selected_joint == Some(ji),
                                RichText::new(name).size(9.5)).clicked() {
                                panel.selected_joint = Some(ji);
                            }
                        }
                    });
            }
        }
    });

    ui.separator();

    // ── Graph canvas ─────────────────────────────────────
    let available = ui.available_size();
    let graph_height = available.y.min(250.0).max(80.0);

    let (response, painter) = ui.allocate_painter(
        Vec2::new(available.x, graph_height),
        egui::Sense::click_and_drag(),
    );
    let canvas = response.rect;

    // Background
    painter.rect_filled(canvas, 2.0, Color32::from_rgb(18, 20, 25));

    // Handle scroll/zoom
    if response.dragged() {
        let delta = response.drag_delta();
        panel.time_scroll -= delta.x / panel.time_zoom;
        panel.value_offset += delta.y / (graph_height / panel.value_zoom / 2.0);
    }

    // Get motion data
    let motion_data = state.active_model.and_then(|idx| {
        state.loaded_models.get(idx).and_then(|a| a.motion.as_ref())
    });

    let joint_idx = panel.selected_joint.unwrap_or(0);

    if let Some(motion) = motion_data {
        let total_time = motion.total_time();
        let num_frames = motion.num_frames();
        if num_frames == 0 || total_time <= 0.0 {
            painter.text(canvas.center(), egui::Align2::CENTER_CENTER,
                "Pas de données", egui::FontId::proportional(11.0), accent::DIM);
        } else {
            let dt = motion.delta_time();
            let _mirrored = state.mirrored;

            // Coordinate mapping functions
            let time_to_x = |t: f32| -> f32 {
                canvas.min.x + (t - panel.time_scroll) * panel.time_zoom
            };
            let value_to_y = |v: f32| -> f32 {
                canvas.center().y - (v - panel.value_offset) * (graph_height / panel.value_zoom / 2.0)
            };

            // Draw grid lines
            // Time grid
            let time_start = panel.time_scroll;
            let time_end = panel.time_scroll + canvas.width() / panel.time_zoom;
            let time_step = find_nice_step((time_end - time_start) / 8.0);
            let mut t = (time_start / time_step).floor() * time_step;
            while t <= time_end {
                let x = time_to_x(t);
                if x >= canvas.min.x && x <= canvas.max.x {
                    painter.line_segment(
                        [Pos2::new(x, canvas.min.y), Pos2::new(x, canvas.max.y)],
                        Stroke::new(0.5, Color32::from_rgb(35, 38, 48)),
                    );
                    painter.text(
                        Pos2::new(x + 2.0, canvas.max.y - 12.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{:.2}s", t),
                        egui::FontId::proportional(8.0),
                        accent::DIM,
                    );
                }
                t += time_step;
            }

            // Value grid (horizontal lines)
            let val_top = panel.value_offset + panel.value_zoom;
            let val_bottom = panel.value_offset - panel.value_zoom;
            let val_step = find_nice_step((val_top - val_bottom) / 6.0);
            let mut v = (val_bottom / val_step).floor() * val_step;
            while v <= val_top {
                let y = value_to_y(v);
                if y >= canvas.min.y && y <= canvas.max.y {
                    painter.line_segment(
                        [Pos2::new(canvas.min.x, y), Pos2::new(canvas.max.x, y)],
                        Stroke::new(0.5, Color32::from_rgb(35, 38, 48)),
                    );
                    painter.text(
                        Pos2::new(canvas.min.x + 2.0, y - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{:.2}", v),
                        egui::FontId::proportional(8.0),
                        accent::DIM,
                    );
                }
                v += val_step;
            }

            // Zero line
            let zero_y = value_to_y(0.0);
            if zero_y >= canvas.min.y && zero_y <= canvas.max.y {
                painter.line_segment(
                    [Pos2::new(canvas.min.x, zero_y), Pos2::new(canvas.max.x, zero_y)],
                    Stroke::new(1.0, Color32::from_rgb(55, 58, 68)),
                );
            }

            // Draw curves
            // Sample every frame and extract position/rotation for the selected joint
            let channels = CurveChannel::all();
            for (ci, ch) in channels.iter().enumerate() {
                if !panel.active_channels[ci] { continue; }

                let color = ch.color();
                let mut prev_point: Option<Pos2> = None;

                // Sample at each frame
                let frame_step = (num_frames / 500).max(1); // limit to 500 samples for perf
                for fi in (0..num_frames).step_by(frame_step) {
                    let t = fi as f32 * dt;
                    let x = time_to_x(t);

                    // Skip if off-screen
                    if x < canvas.min.x - 10.0 || x > canvas.max.x + 10.0 {
                        prev_point = None;
                        continue;
                    }

                    let transforms = &motion.frames[fi];
                    if joint_idx >= transforms.len() { break; }

                    let mat = transforms[joint_idx];
                    let (_scale, rot, trans) = mat.to_scale_rotation_translation();
                    let euler = rot.to_euler(glam::EulerRot::XYZ);

                    let value = match ch {
                        CurveChannel::PosX => trans.x,
                        CurveChannel::PosY => trans.y,
                        CurveChannel::PosZ => trans.z,
                        CurveChannel::RotX => euler.0.to_degrees(),
                        CurveChannel::RotY => euler.1.to_degrees(),
                        CurveChannel::RotZ => euler.2.to_degrees(),
                    };

                    let y = value_to_y(value);
                    let point = Pos2::new(x, y.clamp(canvas.min.y, canvas.max.y));

                    if let Some(prev) = prev_point {
                        painter.line_segment([prev, point], Stroke::new(1.2, color));
                    }
                    prev_point = Some(point);
                }
            }

            // Current time indicator (vertical red line)
            let current_x = time_to_x(state.timestamp);
            if current_x >= canvas.min.x && current_x <= canvas.max.x {
                painter.line_segment(
                    [Pos2::new(current_x, canvas.min.y), Pos2::new(current_x, canvas.max.y)],
                    Stroke::new(1.5, Color32::from_rgb(255, 80, 60)),
                );
                // Frame number label
                let frame = (state.timestamp / dt).round() as usize;
                painter.text(
                    Pos2::new(current_x + 3.0, canvas.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("F{}", frame),
                    egui::FontId::proportional(9.0),
                    Color32::from_rgb(255, 100, 80),
                );
            }

            // Click to set time
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let clicked_time = panel.time_scroll + (pos.x - canvas.min.x) / panel.time_zoom;
                    let clamped = clicked_time.clamp(0.0, total_time);
                    state.timestamp = clamped;
                    panel.status = format!("Temps → {:.3}s (frame {})",
                        clamped, (clamped / dt).round() as usize);
                }
            }
        }
    } else {
        painter.text(canvas.center(), egui::Align2::CENTER_CENTER,
            "Aucune animation chargée", egui::FontId::proportional(11.0), accent::DIM);
    }

    // Status
    if !panel.status.is_empty() {
        ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
    }
}

/// Find a "nice" step value for grid lines (1, 2, 5, 10, 20, 50...).
fn find_nice_step(rough: f32) -> f32 {
    if rough <= 0.0 { return 1.0; }
    let magnitude = 10.0f32.powf(rough.log10().floor());
    let residual = rough / magnitude;
    let nice = if residual <= 1.0 { 1.0 }
        else if residual <= 2.0 { 2.0 }
        else if residual <= 5.0 { 5.0 }
        else { 10.0 };
    nice * magnitude
}
