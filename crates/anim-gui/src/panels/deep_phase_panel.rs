//! DeepPhase panel — phase manifold visualization and transition analysis.

use egui::{Ui, RichText, Color32, Pos2, Vec2, Stroke};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_animation::{DeepPhaseConfig, ChannelGroup, extract_deep_phase};

/// Persistent panel state.
pub struct DeepPhasePanel {
    pub config: DeepPhaseConfig,
    pub status: String,
    /// Which channel to highlight in the manifold view.
    pub selected_channel: usize,
    /// Show all channels overlaid or just the selected one.
    pub show_all_channels: bool,
    /// Manifold view zoom.
    pub zoom: f32,
}

impl Default for DeepPhasePanel {
    fn default() -> Self {
        Self {
            config: DeepPhaseConfig::default(),
            status: String::new(),
            selected_channel: 0,
            show_all_channels: true,
            zoom: 80.0,
        }
    }
}

/// Channel colors for visualization.
fn channel_color(ch: usize) -> Color32 {
    match ch {
        0 => Color32::from_rgb(180, 180, 220), // Core — blue-grey
        1 => Color32::from_rgb(100, 220, 100), // Left leg — green
        2 => Color32::from_rgb(220, 100, 100), // Right leg — red
        3 => Color32::from_rgb(100, 180, 220), // Left arm — cyan
        4 => Color32::from_rgb(220, 180, 100), // Right arm — orange
        _ => accent::MUTED,
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut DeepPhasePanel) {
    ui.label(RichText::new("DeepPhase Manifold").size(13.0).color(accent::TEXT));
    ui.separator();

    // ── Extract / Refresh ───────────────────────────
    ui.horizontal(|ui| {
        if ui.button(RichText::new("⟳ Extraire phase").size(10.5)).clicked() {
            if let Some(idx) = state.active_model {
                if let Some(ref motion) = state.loaded_models[idx].motion {
                    let manifold = extract_deep_phase(motion, panel.config.clone());
                    let n = manifold.num_frames();
                    let freqs: Vec<String> = manifold.dominant_frequencies.iter()
                        .map(|f| format!("{:.2}Hz", f))
                        .collect();
                    panel.status = format!(
                        "Extrait: {} frames, {} canaux [{}]",
                        n, manifold.num_channels, freqs.join(", ")
                    );
                    state.log_info(&format!(
                        "[DeepPhase] Manifold extrait: {} frames, freqs: {}",
                        n, freqs.join(", ")
                    ));
                    state.deep_phase = Some(manifold);
                } else {
                    panel.status = "Erreur: pas d'animation".to_string();
                }
            } else {
                panel.status = "Erreur: pas de modèle actif".to_string();
            }
        }

        if state.deep_phase.is_some() {
            if ui.button(RichText::new("✕ Effacer").size(10.0).color(accent::ERROR)).clicked() {
                state.deep_phase = None;
                panel.status = "Manifold effacé".to_string();
            }
        }
    });

    // ── Info ────────────────────────────────────────
    if let Some(ref manifold) = state.deep_phase {
        ui.add_space(2.0);
        ui.label(RichText::new(format!(
            "{} frames, {} canaux",
            manifold.num_frames(), manifold.num_channels,
        )).size(10.0).color(accent::MUTED));

        // Dominant frequencies
        ui.horizontal(|ui| {
            ui.label(RichText::new("Fréquences:").size(9.5).color(accent::DIM));
            for (i, &f) in manifold.dominant_frequencies.iter().enumerate() {
                let color = channel_color(i);
                ui.label(RichText::new(format!("{:.1}Hz", f)).size(9.5).color(color));
            }
        });

        // Current phase state at timestamp
        if let Some(phase_state) = manifold.get_state_interpolated(
            state.timestamp, state.loaded_models.get(state.active_model.unwrap_or(0))
                .and_then(|a| a.motion.as_ref())
                .map_or(30.0, |m| m.framerate)
        ) {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Phase courante:").size(9.5).color(accent::DIM));
                for (i, &[mx, my]) in phase_state.manifold.iter().enumerate() {
                    let a = phase_state.amplitudes[i];
                    let color = channel_color(i);
                    ui.label(RichText::new(format!("A={:.2}", a)).size(9.0).color(color));
                    ui.label(RichText::new(format!("({:.2},{:.2})", mx, my)).size(9.0).color(accent::DIM));
                }
            });
        }

        ui.separator();

        // ── Channel selector ────────────────────────
        ui.horizontal(|ui| {
            ui.checkbox(&mut panel.show_all_channels, RichText::new("Tous").size(9.5));
            for ch in ChannelGroup::all() {
                let idx = ch.index();
                if idx < manifold.num_channels {
                    let color = if panel.selected_channel == idx {
                        channel_color(idx)
                    } else {
                        accent::DIM
                    };
                    if ui.button(RichText::new(ch.label()).size(9.5).color(color)).clicked() {
                        panel.selected_channel = idx;
                    }
                }
            }
        });

        // Zoom
        ui.horizontal(|ui| {
            ui.label(RichText::new("Zoom:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.zoom)
                .range(10.0..=500.0).speed(2.0).fixed_decimals(0));
        });

        ui.separator();

        // ── Manifold 2D view ────────────────────────
        let available = ui.available_size();
        let canvas_size = available.x.min(available.y.min(220.0)).max(100.0);

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available.x, canvas_size),
            egui::Sense::hover(),
        );
        let canvas = response.rect;
        let center = canvas.center();

        // Background
        painter.rect_filled(canvas, 2.0, Color32::from_rgb(18, 20, 25));

        // Grid circles
        for r in 1..=3 {
            let radius = r as f32 * panel.zoom * 0.33;
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(0.5, Color32::from_rgb(35, 38, 48)),
            );
        }

        // Crosshairs
        painter.line_segment(
            [Pos2::new(canvas.min.x, center.y), Pos2::new(canvas.max.x, center.y)],
            Stroke::new(0.5, Color32::from_rgb(40, 42, 52)),
        );
        painter.line_segment(
            [Pos2::new(center.x, canvas.min.y), Pos2::new(center.x, canvas.max.y)],
            Stroke::new(0.5, Color32::from_rgb(40, 42, 52)),
        );

        // Draw phase trajectory for each channel
        let channels_to_draw: Vec<usize> = if panel.show_all_channels {
            (0..manifold.num_channels).collect()
        } else {
            vec![panel.selected_channel.min(manifold.num_channels.saturating_sub(1))]
        };

        let frame_step = (manifold.num_frames() / 500).max(1);

        for &ch in &channels_to_draw {
            let color = channel_color(ch);
            let dim_color = Color32::from_rgba_premultiplied(
                color.r() / 2, color.g() / 2, color.b() / 2, 120,
            );

            let mut prev_point: Option<Pos2> = None;
            for fi in (0..manifold.num_frames()).step_by(frame_step) {
                let state_ref = &manifold.states[fi];
                if ch >= state_ref.manifold.len() { break; }

                let [mx, my] = state_ref.manifold[ch];
                let px = center.x + mx * panel.zoom;
                let py = center.y - my * panel.zoom; // flip Y

                let point = Pos2::new(
                    px.clamp(canvas.min.x, canvas.max.x),
                    py.clamp(canvas.min.y, canvas.max.y),
                );

                if let Some(prev) = prev_point {
                    painter.line_segment([prev, point], Stroke::new(0.8, dim_color));
                }
                prev_point = Some(point);
            }
        }

        // Draw current position as bright dot
        let framerate = state.loaded_models.get(state.active_model.unwrap_or(0))
            .and_then(|a| a.motion.as_ref())
            .map_or(30.0, |m| m.framerate);
        if let Some(current_state) = manifold.get_state_interpolated(state.timestamp, framerate) {
            for &ch in &channels_to_draw {
                if ch >= current_state.manifold.len() { continue; }
                let [mx, my] = current_state.manifold[ch];
                let px = center.x + mx * panel.zoom;
                let py = center.y - my * panel.zoom;
                let color = channel_color(ch);

                if px >= canvas.min.x && px <= canvas.max.x
                    && py >= canvas.min.y && py <= canvas.max.y
                {
                    painter.circle_filled(Pos2::new(px, py), 4.0, color);
                    painter.circle_stroke(
                        Pos2::new(px, py), 6.0,
                        Stroke::new(1.0, Color32::WHITE),
                    );
                }
            }
        }

        // ── Amplitude timeline ──────────────────────
        ui.add_space(4.0);
        ui.label(RichText::new("Amplitudes").size(10.0).color(accent::MUTED));

        let timeline_height = 60.0f32;
        let (tl_response, tl_painter) = ui.allocate_painter(
            Vec2::new(available.x, timeline_height),
            egui::Sense::click(),
        );
        let tl_rect = tl_response.rect;
        tl_painter.rect_filled(tl_rect, 2.0, Color32::from_rgb(18, 20, 25));

        let total_frames = manifold.num_frames();
        if total_frames > 0 {
            // Find max amplitude for normalization
            let mut max_amp = 0.01f32;
            for state_ref in &manifold.states {
                for &a in &state_ref.amplitudes {
                    if a > max_amp { max_amp = a; }
                }
            }

            let tl_step = (total_frames / 400).max(1);
            for &ch in &channels_to_draw {
                let color = channel_color(ch);
                let mut prev: Option<Pos2> = None;

                for fi in (0..total_frames).step_by(tl_step) {
                    let state_ref = &manifold.states[fi];
                    if ch >= state_ref.amplitudes.len() { break; }

                    let x = tl_rect.min.x + (fi as f32 / total_frames as f32) * tl_rect.width();
                    let amp_norm = state_ref.amplitudes[ch] / max_amp;
                    let y = tl_rect.max.y - amp_norm * tl_rect.height();

                    let point = Pos2::new(x, y);
                    if let Some(p) = prev {
                        tl_painter.line_segment([p, point], Stroke::new(1.0, color));
                    }
                    prev = Some(point);
                }
            }

            // Current time indicator
            let motion_ref = state.loaded_models.get(state.active_model.unwrap_or(0))
                .and_then(|a| a.motion.as_ref());
            if let Some(motion) = motion_ref {
                let total_time = motion.total_time();
                if total_time > 0.0 {
                    let x = tl_rect.min.x + (state.timestamp / total_time) * tl_rect.width();
                    if x >= tl_rect.min.x && x <= tl_rect.max.x {
                        tl_painter.line_segment(
                            [Pos2::new(x, tl_rect.min.y), Pos2::new(x, tl_rect.max.y)],
                            Stroke::new(1.5, Color32::from_rgb(255, 80, 60)),
                        );
                    }
                }
            }

            // Click to set time
            if tl_response.clicked() {
                if let Some(pos) = tl_response.interact_pointer_pos() {
                    let frac = (pos.x - tl_rect.min.x) / tl_rect.width();
                    if let Some(motion) = state.loaded_models.get(state.active_model.unwrap_or(0))
                        .and_then(|a| a.motion.as_ref())
                    {
                        state.timestamp = (frac * motion.total_time()).clamp(0.0, motion.total_time());
                    }
                }
            }
        }
    } else {
        ui.add_space(8.0);
        ui.label(RichText::new("Aucun manifold extrait.\nCliquez 'Extraire phase' pour analyser l'animation.")
            .size(10.0).color(accent::DIM));
    }

    // ── Configuration ───────────────────────────────
    ui.add_space(4.0);
    ui.collapsing(RichText::new("Configuration").size(10.5), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Canaux:").size(9.5));
            let mut nc = panel.config.num_channels as i32;
            if ui.add(egui::DragValue::new(&mut nc).range(1..=5).speed(0.2)).changed() {
                panel.config.num_channels = nc.clamp(1, 5) as usize;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Fenêtre:").size(9.5));
            let mut w = panel.config.window_size as i32;
            if ui.add(egui::DragValue::new(&mut w).range(10..=200).speed(1.0)).changed() {
                panel.config.window_size = w.max(10) as usize;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Freq min:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.config.min_frequency)
                .range(0.1..=5.0).speed(0.05).fixed_decimals(2).suffix(" Hz"));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Freq max:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.config.max_frequency)
                .range(0.5..=10.0).speed(0.1).fixed_decimals(1).suffix(" Hz"));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Lissage amplitude:").size(9.5));
            ui.add(egui::DragValue::new(&mut panel.config.amplitude_smoothing)
                .range(0.0..=1.0).speed(0.01).fixed_decimals(2));
        });
    });

    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.0).color(accent::MUTED));
    }
}
