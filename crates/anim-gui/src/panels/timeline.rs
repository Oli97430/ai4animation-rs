//! Timeline panel — professional animation playback controls with visual keyframe track.
//!
//! Features: transport buttons, painted timeline track with gradient progress,
//! keyframe tick marks, styled playhead, speed/loop/mirror controls.

use egui::{Ui, Color32, RichText, Slider, Vec2, Rect, Stroke, Rounding};
use crate::app_state::AppState;
use crate::theme::accent;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    // ── Row 1: Transport + Timeline scrubber ────────────────
    ui.horizontal(|ui| {
        // Transport controls in a styled group
        egui::Frame::none()
            .fill(accent::SECTION_BG)
            .rounding(6.0)
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .stroke(egui::Stroke::new(0.5, accent::BORDER))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let btn = Vec2::new(24.0, 20.0);

                    // |< (go to start)
                    if ui.add_sized(btn, egui::Button::new(
                        RichText::new("⏮").size(11.0).color(accent::MUTED)
                    ).fill(Color32::TRANSPARENT).rounding(3.0))
                        .on_hover_text("Debut (Home)").clicked()
                    {
                        state.timestamp = 0.0;
                    }

                    // < (previous frame)
                    if ui.add_sized(btn, egui::Button::new(
                        RichText::new("◀").size(10.0).color(accent::MUTED)
                    ).fill(Color32::TRANSPARENT).rounding(3.0))
                        .on_hover_text("Image precedente (←)").clicked()
                    {
                        if let Some(motion) = state.active_motion() {
                            let dt = motion.delta_time();
                            state.timestamp = (state.timestamp - dt).max(0.0);
                        }
                    }

                    // Play/Pause — larger, colored
                    let (play_icon, play_color) = if state.playing {
                        ("⏸", accent::PAUSE)
                    } else {
                        ("▶", accent::PLAY)
                    };
                    if ui.add_sized(Vec2::new(30.0, 20.0), egui::Button::new(
                        RichText::new(play_icon).size(12.0).color(play_color)
                    ).fill(Color32::from_rgba_premultiplied(play_color.r(), play_color.g(), play_color.b(), 20))
                        .rounding(4.0)
                    ).on_hover_text("Lecture/Pause (Space)").clicked()
                    {
                        state.playing = !state.playing;
                    }

                    // > (next frame)
                    if ui.add_sized(btn, egui::Button::new(
                        RichText::new("▶").size(10.0).color(accent::MUTED)
                    ).fill(Color32::TRANSPARENT).rounding(3.0))
                        .on_hover_text("Image suivante (→)").clicked()
                    {
                        if let Some(motion) = state.active_motion() {
                            let dt = motion.delta_time();
                            state.timestamp = (state.timestamp + dt).min(motion.total_time());
                        }
                    }

                    // >| (go to end)
                    if ui.add_sized(btn, egui::Button::new(
                        RichText::new("⏭").size(11.0).color(accent::MUTED)
                    ).fill(Color32::TRANSPARENT).rounding(3.0))
                        .on_hover_text("Fin (End)").clicked()
                    {
                        state.timestamp = state.total_time();
                    }
                });
            });

        ui.add_space(6.0);

        // ── Timeline scrubber ───────────────────────────────
        let total = state.total_time();
        let scrubber_width = (ui.available_width() - 200.0).max(120.0);

        if total > 0.0 {
            let (response, painter) = ui.allocate_painter(
                Vec2::new(scrubber_width, 24.0),
                egui::Sense::click_and_drag(),
            );
            let track = response.rect;

            // Track background with subtle inner shadow
            painter.rect_filled(track, 4.0, Color32::from_rgb(20, 21, 26));
            painter.rect_stroke(track, 4.0, Stroke::new(0.5, Color32::from_rgb(42, 44, 54)));

            // Progress bar with gradient feel
            let progress = state.timestamp / total;
            let bar_width = track.width() * progress;
            if bar_width > 1.0 {
                let bar_rect = Rect::from_min_size(track.min, Vec2::new(bar_width, track.height()));
                painter.rect_filled(
                    bar_rect,
                    Rounding { nw: 4.0, ne: 0.0, sw: 4.0, se: 0.0 },
                    Color32::from_rgba_premultiplied(75, 135, 255, 40),
                );
                // Brighter edge
                let edge = Rect::from_min_size(
                    egui::pos2(bar_rect.max.x - 2.0, bar_rect.min.y),
                    Vec2::new(2.0, bar_rect.height()),
                );
                painter.rect_filled(edge, 0.0, Color32::from_rgba_premultiplied(75, 135, 255, 70));
            }

            // Keyframe tick marks
            let total_frames = state.total_frames();
            if total_frames > 0 {
                let step = if total_frames > 200 { 20 } else if total_frames > 50 { 10 } else { 5 };
                for f in (0..=total_frames).step_by(step) {
                    let frac = f as f32 / total_frames as f32;
                    let x = track.min.x + frac * track.width();
                    let is_major = f % (step * 2) == 0;
                    let h = if is_major { 7.0 } else { 3.0 };
                    let c = if is_major {
                        Color32::from_rgb(65, 68, 82)
                    } else {
                        Color32::from_rgb(45, 47, 58)
                    };
                    painter.line_segment(
                        [egui::pos2(x, track.max.y - h), egui::pos2(x, track.max.y)],
                        Stroke::new(1.0, c),
                    );
                }
            }

            // Playhead (yellow line + rounded top indicator)
            let head_x = track.min.x + progress * track.width();
            // Vertical line
            painter.line_segment(
                [egui::pos2(head_x, track.min.y + 6.0), egui::pos2(head_x, track.max.y)],
                Stroke::new(1.5, Color32::from_rgb(255, 205, 55)),
            );
            // Top rounded indicator
            let indicator_rect = Rect::from_center_size(
                egui::pos2(head_x, track.min.y + 4.0),
                Vec2::new(8.0, 7.0),
            );
            painter.rect_filled(indicator_rect, 2.0, Color32::from_rgb(255, 205, 55));

            // Click/drag to scrub
            if response.dragged() || response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let frac = ((pos.x - track.min.x) / track.width()).clamp(0.0, 1.0);
                    state.timestamp = frac * total;
                }
            }
        } else {
            // No animation: disabled track
            let (_, painter) = ui.allocate_painter(Vec2::new(scrubber_width, 24.0), egui::Sense::hover());
            let rect = egui::Rect::from_min_size(
                egui::pos2(painter.clip_rect().min.x, painter.clip_rect().min.y),
                Vec2::new(scrubber_width, 24.0),
            );
            painter.rect_filled(rect, 4.0, Color32::from_rgb(22, 23, 28));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Aucune animation",
                egui::FontId::proportional(10.0),
                accent::DIM,
            );
        }

        ui.add_space(4.0);

        // Frame/time info
        let current_frame = state.current_frame();
        let total_frames = state.total_frames();

        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!("{}/{}", current_frame, total_frames))
                    .monospace()
                    .size(11.0)
                    .color(accent::TEXT)
            );
            ui.label(
                RichText::new(format!("{:.2}s", state.timestamp))
                    .monospace()
                    .size(10.0)
                    .color(accent::MUTED)
            );
        });
    });

    // ── Row 2: Speed, Loop, Mirror, FPS ─────────────────────
    ui.horizontal(|ui| {
        ui.add_space(4.0);

        // Speed control with label
        ui.label(RichText::new("⏱").size(10.0).color(accent::DIM));
        ui.add_sized(
            Vec2::new(80.0, 16.0),
            Slider::new(&mut state.playback_speed, 0.1..=3.0)
                .step_by(0.1)
                .show_value(true)
                .text("")
        );

        ui.add_space(8.0);

        // Toggle buttons with pill-style highlight
        let loop_active = state.looping;
        let loop_color = if loop_active { accent::PRIMARY } else { accent::DIM };
        if ui.add(
            egui::Button::new(RichText::new("↻ Boucle").size(10.5).color(loop_color))
                .fill(if loop_active { Color32::from_rgba_premultiplied(75, 135, 255, 25) } else { Color32::TRANSPARENT })
                .rounding(10.0)
                .stroke(Stroke::new(0.5, if loop_active { accent::PRIMARY_DIM } else { accent::BORDER }))
        ).clicked() {
            state.looping = !state.looping;
        }

        let mirror_active = state.mirrored;
        let mirror_color = if mirror_active { accent::WARNING } else { accent::DIM };
        if ui.add(
            egui::Button::new(RichText::new("⟷ Miroir").size(10.5).color(mirror_color))
                .fill(if mirror_active { Color32::from_rgba_premultiplied(255, 185, 50, 20) } else { Color32::TRANSPARENT })
                .rounding(10.0)
                .stroke(Stroke::new(0.5, if mirror_active { Color32::from_rgb(180, 130, 35) } else { accent::BORDER }))
        ).clicked() {
            state.mirrored = !state.mirrored;
        }

        // FPS + frame time (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let fps = 1.0 / state.time.delta_time.max(0.001);
            let dt_ms = state.time.delta_time * 1000.0;

            ui.label(
                RichText::new(format!("{:.1}ms", dt_ms))
                    .monospace()
                    .size(10.0)
                    .color(accent::DIM)
            );

            let fps_color = if fps >= 55.0 {
                accent::SUCCESS
            } else if fps >= 30.0 {
                accent::WARNING
            } else {
                accent::ERROR
            };

            // FPS pill
            let fps_text = format!("{:.0} FPS", fps);
            ui.add(
                egui::Button::new(RichText::new(&fps_text).monospace().size(10.0).color(fps_color))
                    .fill(Color32::from_rgba_premultiplied(fps_color.r(), fps_color.g(), fps_color.b(), 15))
                    .rounding(8.0)
                    .stroke(Stroke::NONE)
                    .sense(egui::Sense::hover())
            );

            // Phase indicator (if available)
            if let Some(ref phase) = state.phase_data {
                let framerate = state.active_motion().map_or(30.0, |m| m.framerate);
                let p = phase.get_phase(state.timestamp, framerate);
                let phase_color = Color32::from_rgb(
                    (180.0 + 75.0 * (p * std::f32::consts::TAU).cos()) as u8,
                    (180.0 + 75.0 * (p * std::f32::consts::TAU + 2.094).cos()) as u8,
                    (180.0 + 75.0 * (p * std::f32::consts::TAU + 4.189).cos()) as u8,
                );
                ui.add(
                    egui::Button::new(
                        RichText::new(format!("φ {:.2}", p)).monospace().size(10.0).color(phase_color)
                    )
                    .fill(Color32::from_rgba_premultiplied(phase_color.r(), phase_color.g(), phase_color.b(), 15))
                    .rounding(8.0)
                    .stroke(Stroke::NONE)
                    .sense(egui::Sense::hover())
                ).on_hover_text(format!(
                    "Phase: {:.2} ({} cycles, {:.1} Hz)",
                    p, phase.num_cycles(), phase.frequency
                ));
            }
        });
    });
}
