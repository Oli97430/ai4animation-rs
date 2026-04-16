//! Video recorder — captures viewport frames as PNG sequence.
//!
//! Exports numbered PNG files that can be assembled into a video:
//! `ffmpeg -framerate 30 -i frame_%04d.png -c:v libx264 output.mp4`

use egui::{Ui, RichText, Color32};
use crate::app_state::AppState;
use crate::theme::accent;

/// State for the frame recorder.
pub struct RecorderState {
    pub recording: bool,
    pub output_dir: String,
    pub frame_count: usize,
    pub target_fps: f32,
    pub elapsed: f32,
}

impl Default for RecorderState {
    fn default() -> Self {
        Self {
            recording: false,
            output_dir: "recordings".to_string(),
            frame_count: 0,
            target_fps: 30.0,
            elapsed: 0.0,
        }
    }
}

impl RecorderState {
    /// Start recording.
    pub fn start(&mut self) {
        let dir = std::path::Path::new(&self.output_dir);
        if !dir.exists() {
            let _ = std::fs::create_dir_all(dir);
        }
        self.recording = true;
        self.frame_count = 0;
        self.elapsed = 0.0;
    }

    /// Stop recording.
    pub fn stop(&mut self) {
        self.recording = false;
    }

    /// Tick the recorder; returns true if a frame should be captured.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.recording { return false; }
        self.elapsed += dt;
        let frame_interval = 1.0 / self.target_fps;
        if self.elapsed >= frame_interval {
            self.elapsed -= frame_interval;
            self.frame_count += 1;
            true
        } else {
            false
        }
    }

    /// Get the output path for the current frame.
    pub fn frame_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.output_dir)
            .join(format!("frame_{:04}.png", self.frame_count))
    }

    /// Recording duration in seconds.
    pub fn duration(&self) -> f32 {
        self.frame_count as f32 / self.target_fps
    }
}

pub fn show(ui: &mut Ui, recorder: &mut RecorderState, state: &mut AppState) {
    ui.horizontal(|ui| {
        if recorder.recording {
            let color = Color32::from_rgb(220, 60, 60);
            if ui.button(RichText::new("⏹ Arreter").size(11.0).color(color)).clicked() {
                recorder.stop();
                state.log_info(&format!(
                    "Enregistrement termine: {} images ({:.1}s) dans {}",
                    recorder.frame_count, recorder.duration(), recorder.output_dir
                ));
            }
            ui.label(RichText::new(format!(
                "● REC  {} images  {:.1}s",
                recorder.frame_count, recorder.duration()
            )).size(11.0).color(color));
        } else {
            if ui.button(RichText::new("⏺ Enregistrer").size(11.0)).clicked() {
                recorder.start();
                state.log_info(&format!("Enregistrement demarre dans {}", recorder.output_dir));
            }
            ui.label(RichText::new("Sortie:").size(10.5).color(accent::MUTED));
            ui.text_edit_singleline(&mut recorder.output_dir);
            ui.label(RichText::new("FPS:").size(10.5).color(accent::MUTED));
            ui.add(egui::DragValue::new(&mut recorder.target_fps)
                .range(1.0..=120.0).speed(1.0).fixed_decimals(0));
        }
    });

    if !recorder.recording && recorder.frame_count > 0 {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!(
                "Dernier: {} images — ffmpeg -framerate {:.0} -i {}/frame_%04d.png -c:v libx264 output.mp4",
                recorder.frame_count, recorder.target_fps, recorder.output_dir
            )).size(9.5).color(accent::DIM));
        });
    }
}
