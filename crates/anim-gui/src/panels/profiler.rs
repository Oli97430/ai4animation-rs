//! Profiler panel — performance metrics.

use egui::{Ui, RichText};
use crate::app_state::AppState;
use crate::theme::accent;

/// Rolling average tracker.
pub struct ProfilerState {
    pub frame_times: Vec<f32>,
    pub max_samples: usize,
    pub visible: bool,
}

impl Default for ProfilerState {
    fn default() -> Self {
        Self {
            frame_times: Vec::with_capacity(120),
            max_samples: 120,
            visible: false,
        }
    }
}

impl ProfilerState {
    pub fn push_frame_time(&mut self, dt: f32) {
        if self.frame_times.len() >= self.max_samples {
            self.frame_times.remove(0);
        }
        self.frame_times.push(dt);
    }

    pub fn avg_fps(&self) -> f32 {
        if self.frame_times.is_empty() { return 0.0; }
        let avg_dt = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 }
    }

    pub fn avg_ms(&self) -> f32 {
        if self.frame_times.is_empty() { return 0.0; }
        (self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32) * 1000.0
    }

    pub fn min_fps(&self) -> f32 {
        self.frame_times.iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|&max_dt| if max_dt > 0.0 { 1.0 / max_dt } else { 0.0 })
            .unwrap_or(0.0)
    }

    pub fn max_fps(&self) -> f32 {
        self.frame_times.iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|&min_dt| if min_dt > 0.0 { 1.0 / min_dt } else { 0.0 })
            .unwrap_or(0.0)
    }
}

pub fn show(ui: &mut Ui, state: &AppState, profiler: &ProfilerState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Profileur").size(11.0).color(accent::TEXT));
    });
    ui.separator();

    // FPS stats
    ui.horizontal(|ui| {
        let fps = profiler.avg_fps();
        let color = if fps >= 55.0 {
            Color32::from_rgb(100, 220, 100)
        } else if fps >= 30.0 {
            Color32::from_rgb(220, 200, 80)
        } else {
            Color32::from_rgb(220, 80, 80)
        };
        ui.label(RichText::new(format!("FPS: {:.0}", fps)).size(12.0).color(color));
        ui.separator();
        ui.label(RichText::new(format!("{:.1} ms", profiler.avg_ms())).size(10.5).color(accent::MUTED));
        ui.separator();
        ui.label(RichText::new(format!("Min: {:.0}", profiler.min_fps())).size(10.0).color(accent::MUTED));
        ui.label(RichText::new(format!("Max: {:.0}", profiler.max_fps())).size(10.0).color(accent::MUTED));
    });

    // Frame time graph
    if !profiler.frame_times.is_empty() {
        let desired_size = egui::vec2(ui.available_width(), 40.0);
        let (_, rect) = ui.allocate_space(desired_size);

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 25));

        let n = profiler.frame_times.len();
        let max_dt = profiler.frame_times.iter().cloned().fold(0.0f32, f32::max).max(0.033);

        for i in 0..n {
            let x = rect.left() + (i as f32 / profiler.max_samples as f32) * rect.width();
            let dt = profiler.frame_times[i];
            let h = (dt / max_dt) * rect.height();
            let color = if dt < 0.017 {
                Color32::from_rgb(80, 180, 80)
            } else if dt < 0.033 {
                Color32::from_rgb(180, 180, 60)
            } else {
                Color32::from_rgb(200, 60, 60)
            };
            painter.line_segment(
                [egui::pos2(x, rect.bottom()), egui::pos2(x, rect.bottom() - h)],
                egui::Stroke::new(1.5, color),
            );
        }

        // 60fps line
        let y60 = rect.bottom() - (0.0167 / max_dt) * rect.height();
        painter.line_segment(
            [egui::pos2(rect.left(), y60), egui::pos2(rect.right(), y60)],
            egui::Stroke::new(0.5, Color32::from_rgba_premultiplied(100, 200, 100, 80)),
        );
    }

    ui.separator();

    // Scene stats
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Entités: {}", state.scene.entity_count())).size(10.0).color(accent::MUTED));
        ui.separator();
        ui.label(RichText::new(format!("Modèles: {}", state.loaded_models.len())).size(10.0).color(accent::MUTED));
        ui.separator();
        if let Some(motion) = state.active_motion() {
            ui.label(RichText::new(format!("Articulations: {}", motion.num_joints())).size(10.0).color(accent::MUTED));
            ui.label(RichText::new(format!("Images: {}", motion.num_frames())).size(10.0).color(accent::MUTED));
        }
        ui.separator();
        ui.label(RichText::new(format!("Lignes debug: {}", state.debug_draw.line_count())).size(10.0).color(accent::MUTED));
    });
}

use egui::Color32;
