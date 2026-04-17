//! Graph Editor — After Effects / Flash style animation curve editor.
//!
//! Displays per-joint position and rotation curves over time with a visual
//! grid, interactive keyframe selection, tween-type controls, snap toggle,
//! and cursor value readout.

use egui::{Ui, RichText, Color32, Pos2, Rect, Vec2, Stroke, Rounding};
use crate::app_state::AppState;
use crate::theme::accent;

// ─── Grid palette ────────────────────────────────────────────────────────────
const BG:         Color32 = Color32::from_rgb(35, 37, 45);
const MINOR_GRID: Color32 = Color32::from_rgb(45, 47, 55);
const MAJOR_GRID: Color32 = Color32::from_rgb(55, 58, 70);

// ─── Channel enum ────────────────────────────────────────────────────────────

/// Which property channel to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            Self::PosX | Self::RotX => accent::AXIS_X,
            Self::PosY | Self::RotY => accent::AXIS_Y,
            Self::PosZ | Self::RotZ => accent::AXIS_Z,
        }
    }

    pub fn all() -> &'static [CurveChannel] {
        &[
            Self::PosX, Self::PosY, Self::PosZ,
            Self::RotX, Self::RotY, Self::RotZ,
        ]
    }

    /// Index into the 6-element channel array.
    pub fn index(&self) -> usize {
        match self {
            Self::PosX => 0, Self::PosY => 1, Self::PosZ => 2,
            Self::RotX => 3, Self::RotY => 4, Self::RotZ => 5,
        }
    }
}

// ─── Tween button labels (matches TweenType order) ──────────────────────────

const TWEEN_LABELS: &[&str] = &[
    "None", "Linear", "EaseIn", "EaseOut", "EaseInOut", "Bezier",
];

fn tween_label(idx: usize) -> &'static str {
    TWEEN_LABELS.get(idx).copied().unwrap_or("?")
}

#[allow(dead_code)]
fn tween_from_index(idx: usize) -> anim_animation::TweenType {
    use anim_animation::TweenType;
    match idx {
        0 => TweenType::None,
        1 => TweenType::Linear,
        2 => TweenType::EaseIn,
        3 => TweenType::EaseOut,
        4 => TweenType::EaseInOut,
        5 => TweenType::Bezier { cx1: 0.25, cy1: 0.1, cx2: 0.25, cy2: 1.0 },
        _ => TweenType::Linear,
    }
}

#[allow(dead_code)]
fn tween_to_index(tw: &anim_animation::TweenType) -> usize {
    use anim_animation::TweenType;
    match tw {
        TweenType::None       => 0,
        TweenType::Linear     => 1,
        TweenType::EaseIn     => 2,
        TweenType::EaseOut    => 3,
        TweenType::EaseInOut  => 4,
        TweenType::Bezier { .. } => 5,
    }
}

// ─── Panel state ─────────────────────────────────────────────────────────────

/// Persistent state for the graph editor panel.
pub struct GraphEditorPanel {
    /// Whether the panel is visible (kept for external toggle compat).
    pub visible: bool,
    /// Horizontal zoom (pixels per second).
    pub zoom_x: f32,
    /// Vertical zoom (value-range visible).
    pub zoom_y: f32,
    /// Horizontal scroll (seconds offset).
    pub scroll_x: f32,
    /// Vertical scroll (value offset).
    pub scroll_y: f32,
    /// Currently selected channel index (0..5).
    pub selected_channel: Option<usize>,
    /// Selected keyframe frame number (within the active channel).
    pub selected_keyframe: Option<usize>,
    /// Whether we are mid-drag on a tangent handle (reserved for Bezier).
    pub dragging_handle: bool,
    /// Show all 6 channels overlaid, or only the selected one.
    pub show_all_channels: bool,
    /// Active channel toggles (6 bools: PosX..RotZ).
    pub active_channels: Vec<bool>,
    /// Snap cursor / keyframes to whole frames.
    pub snap_to_frames: bool,
    /// Auto-follow selected bone in hierarchy.
    pub auto_follow: bool,
    /// Which joint to inspect.
    pub selected_joint: Option<usize>,
    /// Cursor readout (frame, value).
    pub cursor_frame: f32,
    pub cursor_value: f32,
    /// Status line.
    pub status: String,
}

impl Default for GraphEditorPanel {
    fn default() -> Self {
        Self {
            visible: true,
            zoom_x: 200.0,
            zoom_y: 2.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            selected_channel: Some(0),
            selected_keyframe: None,
            dragging_handle: false,
            show_all_channels: true,
            active_channels: vec![true, true, true, false, false, false],
            snap_to_frames: true,
            auto_follow: true,
            selected_joint: None,
            cursor_frame: 0.0,
            cursor_value: 0.0,
            status: String::new(),
        }
    }
}

// ─── Main show function ──────────────────────────────────────────────────────

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut GraphEditorPanel) {
    ui.label(RichText::new("Editeur de courbes").size(13.0).color(accent::TEXT));
    ui.separator();

    // ── Toolbar row 1: channel toggles + zoom ────────────────────────────
    ui.horizontal(|ui| {
        ui.checkbox(&mut panel.auto_follow, RichText::new("Auto-suivre").size(10.0));
        ui.checkbox(&mut panel.show_all_channels, RichText::new("Tous").size(10.0));
        ui.separator();

        // Channel toggle buttons
        for (i, ch) in CurveChannel::all().iter().enumerate() {
            let active = panel.active_channels[i];
            let color = if active { ch.color() } else { accent::DIM };
            let label = RichText::new(ch.label()).size(9.5).color(color);
            if ui.selectable_label(panel.selected_channel == Some(i), label).clicked() {
                panel.active_channels[i] = !panel.active_channels[i];
                panel.selected_channel = Some(i);
                panel.selected_keyframe = None;
            }
        }

        ui.separator();

        // Zoom controls
        if ui.button(RichText::new("H+").size(9.5)).on_hover_text("Zoom horizontal +").clicked() {
            panel.zoom_x = (panel.zoom_x * 1.3).min(2000.0);
        }
        if ui.button(RichText::new("H-").size(9.5)).on_hover_text("Zoom horizontal -").clicked() {
            panel.zoom_x = (panel.zoom_x / 1.3).max(20.0);
        }
        if ui.button(RichText::new("V+").size(9.5)).on_hover_text("Zoom vertical +").clicked() {
            panel.zoom_y = (panel.zoom_y / 1.3).max(0.1);
        }
        if ui.button(RichText::new("V-").size(9.5)).on_hover_text("Zoom vertical -").clicked() {
            panel.zoom_y = (panel.zoom_y * 1.3).min(100.0);
        }

        ui.separator();

        // Snap toggle
        ui.checkbox(&mut panel.snap_to_frames, RichText::new("Snap").size(9.5));

        // Reset button
        if ui.button(RichText::new("Reinit.").size(9.5)).clicked() {
            panel.zoom_x = 200.0;
            panel.zoom_y = 2.0;
            panel.scroll_x = 0.0;
            panel.scroll_y = 0.0;
        }
    });

    // ── Toolbar row 2: joint selector + tween type + value readout ────────
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

        // Joint combo box
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
                                RichText::new(name).size(9.5)).clicked()
                            {
                                panel.selected_joint = Some(ji);
                                panel.selected_keyframe = None;
                            }
                        }
                    });
            }
        }

        ui.separator();

        // Tween type buttons (only when a keyframe is selected -- show always for visibility)
        ui.label(RichText::new("Tween:").size(9.5).color(accent::DIM));
        for ti in 0..TWEEN_LABELS.len() {
            let is_current = false; // will highlight below if matching
            let btn = ui.selectable_label(is_current, RichText::new(tween_label(ti)).size(9.0));
            if btn.clicked() {
                panel.status = format!("Tween -> {}", tween_label(ti));
                // Apply tween to selected keyframe if we had keyframe track data
                // (future: integrate with KeyframeAnimation on AppState)
            }
        }

        ui.separator();

        // Cursor readout
        ui.label(RichText::new(format!(
            "F:{:.0}  V:{:.3}", panel.cursor_frame, panel.cursor_value
        )).size(9.5).color(accent::TEXT));
    });

    ui.separator();

    // ── Graph canvas ─────────────────────────────────────────────────────
    let available = ui.available_size();
    let graph_height = available.y.min(300.0).max(80.0);

    let (response, painter) = ui.allocate_painter(
        Vec2::new(available.x, graph_height),
        egui::Sense::click_and_drag(),
    );
    let canvas = response.rect;

    // Background fill
    painter.rect_filled(canvas, Rounding::same(2.0), BG);

    // Handle drag-to-scroll
    if response.dragged() {
        let delta = response.drag_delta();
        panel.scroll_x -= delta.x / panel.zoom_x;
        panel.scroll_y += delta.y / (graph_height / panel.zoom_y / 2.0);
    }

    // Scroll-wheel zoom (if hovered)
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta);
        if scroll.y.abs() > 0.1 {
            let factor = 1.0 + scroll.y * 0.002;
            panel.zoom_x = (panel.zoom_x * factor).clamp(20.0, 2000.0);
        }
    }

    // ── Coordinate mapping closures ──────────────────────────────────────
    let time_to_x = |t: f32| -> f32 {
        canvas.min.x + (t - panel.scroll_x) * panel.zoom_x
    };
    let value_to_y = |v: f32| -> f32 {
        canvas.center().y - (v - panel.scroll_y) * (graph_height / panel.zoom_y / 2.0)
    };
    let x_to_time = |x: f32| -> f32 {
        panel.scroll_x + (x - canvas.min.x) / panel.zoom_x
    };
    let y_to_value = |y: f32| -> f32 {
        panel.scroll_y - (y - canvas.center().y) / (graph_height / panel.zoom_y / 2.0)
    };

    // ── Grid drawing ─────────────────────────────────────────────────────
    let time_start = x_to_time(canvas.min.x);
    let time_end = x_to_time(canvas.max.x);
    let val_top = y_to_value(canvas.min.y);
    let val_bottom = y_to_value(canvas.max.y);

    // Determine nice grid step for time (frames axis)
    let motion_dt = state.active_model
        .and_then(|idx| state.loaded_models.get(idx))
        .and_then(|a| a.motion.as_ref())
        .map(|m| m.delta_time())
        .unwrap_or(1.0 / 30.0);
    let framerate = 1.0 / motion_dt;

    // Minor grid: every frame; major grid: every 10 frames (or adapted)
    let minor_time_step = motion_dt; // 1 frame
    let major_time_step = find_nice_step((time_end - time_start) / 8.0);

    // Time minor gridlines
    {
        let step = minor_time_step;
        if step > 0.0 && (time_end - time_start) / step < 600.0 {
            let mut t = (time_start / step).floor() * step;
            while t <= time_end {
                let x = time_to_x(t);
                if x >= canvas.min.x && x <= canvas.max.x {
                    painter.line_segment(
                        [Pos2::new(x, canvas.min.y), Pos2::new(x, canvas.max.y)],
                        Stroke::new(0.5, MINOR_GRID),
                    );
                }
                t += step;
            }
        }
    }

    // Time major gridlines + labels
    {
        let step = major_time_step;
        let mut t = (time_start / step).floor() * step;
        while t <= time_end {
            let x = time_to_x(t);
            if x >= canvas.min.x && x <= canvas.max.x {
                painter.line_segment(
                    [Pos2::new(x, canvas.min.y), Pos2::new(x, canvas.max.y)],
                    Stroke::new(1.0, MAJOR_GRID),
                );
                let frame_num = (t / motion_dt).round() as i64;
                painter.text(
                    Pos2::new(x + 2.0, canvas.max.y - 12.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("F{}", frame_num),
                    egui::FontId::proportional(8.5),
                    accent::DIM,
                );
            }
            t += step;
        }
    }

    // Value horizontal gridlines
    let val_step = find_nice_step((val_top - val_bottom).abs() / 6.0);
    {
        let mut v = (val_bottom.min(val_top) / val_step).floor() * val_step;
        let v_end = val_bottom.max(val_top);
        while v <= v_end {
            let y = value_to_y(v);
            if y >= canvas.min.y && y <= canvas.max.y {
                // Minor or major?
                let is_major = (v / (val_step * 5.0)).fract().abs() < 0.01 || v.abs() < 0.001;
                let stroke = if is_major {
                    Stroke::new(0.8, MAJOR_GRID)
                } else {
                    Stroke::new(0.5, MINOR_GRID)
                };
                painter.line_segment(
                    [Pos2::new(canvas.min.x, y), Pos2::new(canvas.max.x, y)],
                    stroke,
                );
                painter.text(
                    Pos2::new(canvas.min.x + 3.0, y - 1.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{:.2}", v),
                    egui::FontId::proportional(8.0),
                    accent::DIM,
                );
            }
            v += val_step;
        }
    }

    // Zero line (emphasized)
    let zero_y = value_to_y(0.0);
    if zero_y >= canvas.min.y && zero_y <= canvas.max.y {
        painter.line_segment(
            [Pos2::new(canvas.min.x, zero_y), Pos2::new(canvas.max.x, zero_y)],
            Stroke::new(1.2, Color32::from_rgb(70, 73, 88)),
        );
    }

    // ── Draw curves from motion data ─────────────────────────────────────
    let motion_data = state.active_model.and_then(|idx| {
        state.loaded_models.get(idx).and_then(|a| a.motion.as_ref())
    });

    let joint_idx = panel.selected_joint.unwrap_or(0);

    if let Some(motion) = motion_data {
        let total_time = motion.total_time();
        let num_frames = motion.num_frames();
        let dt = motion.delta_time();

        if num_frames == 0 || total_time <= 0.0 {
            painter.text(
                canvas.center(), egui::Align2::CENTER_CENTER,
                "Pas de donnees d'animation",
                egui::FontId::proportional(12.0), accent::DIM,
            );
        } else {
            // Determine which channels to draw
            let channels = CurveChannel::all();

            for (ci, ch) in channels.iter().enumerate() {
                // Skip inactive channels unless show_all is off and this is not selected
                if !panel.show_all_channels {
                    if panel.selected_channel != Some(ci) { continue; }
                } else if !panel.active_channels[ci] {
                    continue;
                }

                let base_color = ch.color();
                let is_selected_ch = panel.selected_channel == Some(ci);
                let line_width = if is_selected_ch { 1.8 } else { 1.0 };
                let alpha = if is_selected_ch { 255 } else { 140 };
                let color = Color32::from_rgba_premultiplied(
                    (base_color.r() as u32 * alpha as u32 / 255) as u8,
                    (base_color.g() as u32 * alpha as u32 / 255) as u8,
                    (base_color.b() as u32 * alpha as u32 / 255) as u8,
                    alpha,
                );

                let mut prev_point: Option<Pos2> = None;
                let mut keyframe_dots: Vec<(Pos2, usize)> = Vec::new();

                // Sample at each frame, capping to 800 samples for performance
                let frame_step = (num_frames / 800).max(1);
                for fi in (0..num_frames).step_by(frame_step) {
                    let t = fi as f32 * dt;
                    let x = time_to_x(t);

                    // Off-screen culling
                    if x < canvas.min.x - 20.0 || x > canvas.max.x + 20.0 {
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

                    // Draw line segment from previous point
                    if let Some(prev) = prev_point {
                        painter.line_segment([prev, point], Stroke::new(line_width, color));
                    }
                    prev_point = Some(point);

                    // Collect keyframe dots (every N-th frame for visual density control)
                    // Mark every sampled frame as a potential keyframe dot if step is small
                    if frame_step <= 2 || fi % (frame_step * 5) == 0 {
                        keyframe_dots.push((point, fi));
                    }
                }

                // Draw keyframe diamond/circle markers on the selected channel
                if is_selected_ch {
                    let dot_radius = 3.5;
                    for &(pos, fi) in &keyframe_dots {
                        if pos.x < canvas.min.x || pos.x > canvas.max.x { continue; }
                        if pos.y < canvas.min.y || pos.y > canvas.max.y { continue; }

                        let is_selected_kf = panel.selected_keyframe == Some(fi);
                        let fill = if is_selected_kf { accent::SELECTED } else { base_color };
                        let outline = if is_selected_kf {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(20, 22, 28)
                        };

                        // Diamond shape for keyframes
                        let r = dot_radius;
                        let diamond = vec![
                            Pos2::new(pos.x, pos.y - r),
                            Pos2::new(pos.x + r, pos.y),
                            Pos2::new(pos.x, pos.y + r),
                            Pos2::new(pos.x - r, pos.y),
                        ];
                        painter.add(egui::Shape::convex_polygon(
                            diamond.clone(),
                            fill,
                            Stroke::new(1.0, outline),
                        ));
                    }
                }
            }

            // ── Playhead (current time vertical line) ────────────────────
            let playhead_x = time_to_x(state.timestamp);
            if playhead_x >= canvas.min.x && playhead_x <= canvas.max.x {
                painter.line_segment(
                    [Pos2::new(playhead_x, canvas.min.y), Pos2::new(playhead_x, canvas.max.y)],
                    Stroke::new(1.5, Color32::from_rgb(255, 80, 60)),
                );

                // Playhead triangle at top
                let tri_h = 8.0;
                let tri_w = 6.0;
                let tri = vec![
                    Pos2::new(playhead_x, canvas.min.y + tri_h),
                    Pos2::new(playhead_x - tri_w, canvas.min.y),
                    Pos2::new(playhead_x + tri_w, canvas.min.y),
                ];
                painter.add(egui::Shape::convex_polygon(
                    tri,
                    Color32::from_rgb(255, 80, 60),
                    Stroke::NONE,
                ));

                // Frame label
                let frame = (state.timestamp / dt).round() as usize;
                painter.text(
                    Pos2::new(playhead_x + 4.0, canvas.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("F{}", frame),
                    egui::FontId::proportional(9.0),
                    Color32::from_rgb(255, 100, 80),
                );
            }

            // ── Cursor crosshair + value readout ─────────────────────────
            if let Some(hover_pos) = response.hover_pos() {
                if canvas.contains(hover_pos) {
                    let cursor_t = x_to_time(hover_pos.x);
                    let cursor_v = y_to_value(hover_pos.y);
                    let cursor_f = cursor_t / dt;

                    let display_frame = if panel.snap_to_frames {
                        cursor_f.round()
                    } else {
                        cursor_f
                    };
                    panel.cursor_frame = display_frame;
                    panel.cursor_value = cursor_v;

                    // Vertical dashed cursor line
                    let cursor_color = Color32::from_rgba_premultiplied(200, 200, 200, 60);
                    painter.line_segment(
                        [Pos2::new(hover_pos.x, canvas.min.y), Pos2::new(hover_pos.x, canvas.max.y)],
                        Stroke::new(0.7, cursor_color),
                    );
                    // Horizontal dashed cursor line
                    painter.line_segment(
                        [Pos2::new(canvas.min.x, hover_pos.y), Pos2::new(canvas.max.x, hover_pos.y)],
                        Stroke::new(0.7, cursor_color),
                    );

                    // Tooltip at cursor
                    painter.text(
                        Pos2::new(hover_pos.x + 8.0, hover_pos.y - 14.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("F{:.0} = {:.3}", display_frame, cursor_v),
                        egui::FontId::proportional(9.0),
                        accent::TEXT_BRIGHT,
                    );
                }
            }

            // ── Click interaction: set time or select keyframe ───────────
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let clicked_time = x_to_time(pos.x);
                    let snapped_time = if panel.snap_to_frames {
                        (clicked_time / dt).round() * dt
                    } else {
                        clicked_time
                    };
                    let clamped = snapped_time.clamp(0.0, total_time);
                    state.timestamp = clamped;

                    let frame = (clamped / dt).round() as usize;
                    panel.status = format!(
                        "Temps -> {:.3}s  (frame {})",
                        clamped, frame
                    );

                    // Try to select nearest keyframe dot on selected channel
                    if let Some(_ci) = panel.selected_channel {
                        let click_frame = (clamped / dt).round() as usize;
                        // Simple: select the clicked frame as keyframe
                        panel.selected_keyframe = Some(click_frame);
                    }
                }
            }
        }
    } else {
        // No animation loaded
        painter.text(
            canvas.center(), egui::Align2::CENTER_CENTER,
            "Aucune animation chargee",
            egui::FontId::proportional(12.0), accent::DIM,
        );
    }

    // ── Canvas border ────────────────────────────────────────────────────
    painter.rect_stroke(canvas, Rounding::same(2.0), Stroke::new(1.0, accent::BORDER));

    // ── Bottom status bar ────────────────────────────────────────────────
    ui.horizontal(|ui| {
        if !panel.status.is_empty() {
            ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let sel_ch_name = panel.selected_channel
                .and_then(|i| CurveChannel::all().get(i))
                .map(|c| c.label())
                .unwrap_or("-");
            ui.label(RichText::new(format!(
                "Canal: {}  |  Zoom: {:.0}x{:.1}",
                sel_ch_name, panel.zoom_x, panel.zoom_y
            )).size(9.0).color(accent::DIM));
        });
    });
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Find a "nice" step value for grid lines (1, 2, 5, 10, 20, 50 ...).
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
