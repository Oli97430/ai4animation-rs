//! Video export panel — export scene as GIF, MP4 (ffmpeg), or PNG sequence.

use egui::{Ui, RichText};
use crate::app_state::AppState;

#[derive(Default)]
pub struct VideoExportState {
    pub output_path: String,
    pub format: ExportFormatChoice,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub duration_secs: f32,
    pub recording: bool,
    pub frames_captured: u32,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ExportFormatChoice {
    #[default]
    Gif,
    Mp4,
    PngSequence,
}

impl VideoExportState {
    pub fn new() -> Self {
        Self {
            output_path: "export/animation.gif".to_string(),
            format: ExportFormatChoice::Gif,
            width: 800,
            height: 600,
            framerate: 30,
            duration_secs: 4.0,
            recording: false,
            frames_captured: 0,
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, export_state: &mut VideoExportState) {
    ui.heading(RichText::new("🎞️ Export vidéo").size(14.0));
    ui.separator();

    ui.label("Format:");
    ui.horizontal(|ui| {
        ui.radio_value(&mut export_state.format, ExportFormatChoice::Gif, "GIF animé");
        ui.radio_value(&mut export_state.format, ExportFormatChoice::Mp4, "MP4 (ffmpeg)");
        ui.radio_value(&mut export_state.format, ExportFormatChoice::PngSequence, "PNG séquence");
    });

    ui.add_space(4.0);
    ui.label("Résolution:");
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut export_state.width).range(64..=4096).prefix("L: "));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut export_state.height).range(64..=4096).prefix("H: "));
    });

    ui.horizontal(|ui| {
        if ui.button("720p").clicked() { export_state.width = 1280; export_state.height = 720; }
        if ui.button("1080p").clicked() { export_state.width = 1920; export_state.height = 1080; }
        if ui.button("Square 512").clicked() { export_state.width = 512; export_state.height = 512; }
        if ui.button("Square 800").clicked() { export_state.width = 800; export_state.height = 800; }
    });

    ui.add_space(4.0);
    ui.add(egui::Slider::new(&mut export_state.framerate, 10..=60).text("FPS"));
    ui.add(egui::Slider::new(&mut export_state.duration_secs, 0.5..=30.0).text("Durée (s)"));

    ui.add_space(8.0);
    ui.label("Chemin de sortie:");
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut export_state.output_path);
        if ui.button("📁").clicked() {
            let default_ext = match export_state.format {
                ExportFormatChoice::Gif => "gif",
                ExportFormatChoice::Mp4 => "mp4",
                ExportFormatChoice::PngSequence => "png",
            };
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Vidéo", &[default_ext])
                .set_file_name(&format!("animation.{}", default_ext))
                .save_file()
            {
                export_state.output_path = path.to_string_lossy().to_string();
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();

    let total_frames = (export_state.duration_secs * export_state.framerate as f32) as u32;
    ui.label(format!("Total: {} frames ({} × {} × {}fps)",
        total_frames, export_state.width, export_state.height, export_state.framerate));

    ui.add_space(4.0);
    if export_state.recording {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(format!("Capture en cours: {}/{}",
                export_state.frames_captured, total_frames));
        });
        if ui.button("⏹ Arrêter").clicked() {
            export_state.recording = false;
            state.log_info("[Video] Capture interrompue");
        }
    } else {
        if ui.button(RichText::new("🔴 Démarrer export").strong().size(14.0)).clicked() {
            export_state.recording = true;
            export_state.frames_captured = 0;
            state.log_info(&format!(
                "[Video] Export démarré: {} → {}",
                match export_state.format {
                    ExportFormatChoice::Gif => "GIF",
                    ExportFormatChoice::Mp4 => "MP4",
                    ExportFormatChoice::PngSequence => "PNG sequence",
                },
                export_state.output_path
            ));
        }
    }

    ui.add_space(8.0);
    ui.label(RichText::new("ℹ MP4 nécessite ffmpeg dans le PATH, sinon fallback PNG sequence").weak().size(10.0));
}
