//! Batch converter panel — convert multiple files at once.

use egui::{Ui, RichText, Color32};
use crate::app_state::AppState;
use crate::theme::accent;

/// State for the batch converter UI.
pub struct BatchState {
    pub visible: bool,
    pub input_dir: String,
    pub output_dir: String,
    pub output_format: String,
    pub bvh_scale: f32,
    pub results: Vec<anim_import::batch_converter::ConvertResult>,
    pub running: bool,
}

impl Default for BatchState {
    fn default() -> Self {
        Self {
            visible: false,
            input_dir: String::new(),
            output_dir: "converted".to_string(),
            output_format: "bvh".to_string(),
            bvh_scale: 0.01,
            results: Vec::new(),
            running: false,
        }
    }
}

pub fn show(ui: &mut Ui, batch: &mut BatchState, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Convertisseur par lot").size(12.0).color(accent::TEXT));
    });
    ui.separator();

    // Input directory
    ui.horizontal(|ui| {
        ui.label(RichText::new("Source:").size(10.5).color(accent::MUTED));
        ui.text_edit_singleline(&mut batch.input_dir);
        if ui.button(RichText::new("...").size(10.5)).clicked() {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                batch.input_dir = dir.to_string_lossy().to_string();
            }
        }
    });

    // Output directory
    ui.horizontal(|ui| {
        ui.label(RichText::new("Sortie:").size(10.5).color(accent::MUTED));
        ui.text_edit_singleline(&mut batch.output_dir);
        if ui.button(RichText::new("...").size(10.5)).clicked() {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                batch.output_dir = dir.to_string_lossy().to_string();
            }
        }
    });

    // Options
    ui.horizontal(|ui| {
        ui.label(RichText::new("Format:").size(10.5).color(accent::MUTED));
        egui::ComboBox::from_id_salt("batch_format")
            .selected_text(&batch.output_format)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut batch.output_format, "bvh".to_string(), "BVH");
            });
        ui.separator();
        ui.label(RichText::new("Scale BVH:").size(10.5).color(accent::MUTED));
        ui.add(egui::DragValue::new(&mut batch.bvh_scale)
            .range(0.001..=100.0).speed(0.001).fixed_decimals(3));
    });

    ui.separator();

    // Preview files
    if !batch.input_dir.is_empty() {
        let input_path = std::path::Path::new(&batch.input_dir);
        if input_path.is_dir() {
            let files = anim_import::collect_animation_files(input_path);
            ui.label(RichText::new(format!("{} fichiers trouves", files.len()))
                .size(10.5).color(accent::MUTED));

            // Convert button
            let can_convert = !files.is_empty() && !batch.running;
            if ui.add_enabled(can_convert,
                egui::Button::new(RichText::new("Convertir").size(11.0))
            ).clicked() {
                batch.running = true;
                let config = anim_import::BatchConfig {
                    output_dir: std::path::PathBuf::from(&batch.output_dir),
                    output_format: batch.output_format.clone(),
                    bvh_scale: batch.bvh_scale,
                    preserve_structure: true,
                };
                batch.results = anim_import::convert_directory(input_path, &config);
                batch.running = false;

                let ok = batch.results.iter().filter(|r| r.success).count();
                let fail = batch.results.len() - ok;
                state.log_info(&format!("Batch: {} OK, {} erreurs", ok, fail));
            }
        } else {
            ui.label(RichText::new("Repertoire invalide").size(10.5).color(Color32::from_rgb(200, 80, 80)));
        }
    }

    // Results
    if !batch.results.is_empty() {
        ui.separator();
        ui.label(RichText::new("Resultats").size(11.0).color(accent::TEXT));

        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
            for result in &batch.results {
                let (icon, color) = if result.success {
                    ("✓", Color32::from_rgb(100, 200, 100))
                } else {
                    ("✗", Color32::from_rgb(200, 80, 80))
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).size(10.0).color(color));
                    let name = result.source.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?");
                    ui.label(RichText::new(name).size(10.0).color(accent::MUTED));
                    ui.label(RichText::new(&result.message).size(9.5).color(accent::MUTED));
                });
            }
        });
    }
}
