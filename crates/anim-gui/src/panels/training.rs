//! Training panel — launch and monitor locomotion model training.

use egui::{Ui, RichText};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_ai::AiCommand;

/// Persistent state for the training panel (lives on AnimApp, not AppState).
pub struct TrainingPanel {
    pub data_dir: String,
    pub output_dir: String,
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: String,
    /// Pending commands to execute (drained by main loop).
    pub pending_commands: Vec<AiCommand>,
}

impl Default for TrainingPanel {
    fn default() -> Self {
        Self {
            data_dir: String::new(),
            output_dir: "models/locomotion".to_string(),
            epochs: 100,
            batch_size: 32,
            learning_rate: "0.0001".to_string(),
            pending_commands: Vec::new(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut TrainingPanel) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Entraînement").strong().size(13.0).color(accent::TEXT_BRIGHT));
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // ── Data directory ──────────────────────────────
    ui.label(RichText::new("Dossier données (BVH/FBX)").size(11.0).color(accent::MUTED));
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut panel.data_dir)
                .desired_width(ui.available_width() - 60.0)
                .hint_text("Chemin vers les fichiers BVH...")
        );
        if ui.small_button("...").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                panel.data_dir = path.display().to_string();
            }
        }
    });

    ui.add_space(4.0);

    // ── Output directory ────────────────────────────
    ui.label(RichText::new("Dossier sortie").size(11.0).color(accent::MUTED));
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut panel.output_dir)
                .desired_width(ui.available_width() - 60.0)
        );
        if ui.small_button("...").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                panel.output_dir = path.display().to_string();
            }
        }
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // ── Hyperparameters ─────────────────────────────
    ui.label(RichText::new("Hyperparamètres").size(11.0).color(accent::TEXT_BRIGHT));
    ui.add_space(2.0);

    egui::Grid::new("train_params").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
        ui.label(RichText::new("Epochs").size(11.0));
        ui.add(egui::DragValue::new(&mut panel.epochs).range(1..=10000).speed(1));
        ui.end_row();

        ui.label(RichText::new("Batch size").size(11.0));
        ui.add(egui::DragValue::new(&mut panel.batch_size).range(1..=512).speed(1));
        ui.end_row();

        ui.label(RichText::new("Learning rate").size(11.0));
        ui.add(
            egui::TextEdit::singleline(&mut panel.learning_rate)
                .desired_width(80.0)
        );
        ui.end_row();
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    // ── Actions ─────────────────────────────────────
    let can_train = !panel.data_dir.is_empty() && !state.training_active;

    if state.training_active {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new("Entraînement en cours...").color(accent::WARNING).size(12.0));
        });
    } else {
        ui.horizontal(|ui| {
            let train_btn = ui.add_enabled(
                can_train,
                egui::Button::new(RichText::new("Entraîner le modèle").size(12.0)),
            );
            if train_btn.clicked() {
                let lr: f64 = panel.learning_rate.parse().unwrap_or(1e-4);
                panel.pending_commands.push(AiCommand::TrainModel {
                    data_dir: panel.data_dir.clone(),
                    output_dir: panel.output_dir.clone(),
                    epochs: panel.epochs,
                    batch_size: panel.batch_size,
                    learning_rate: lr,
                });
            }

            if ui.button(RichText::new("Convertir .pt → ONNX").size(11.0)).clicked() {
                let pt_path = format!("{}/Network.pt", panel.output_dir);
                panel.pending_commands.push(AiCommand::ConvertModel {
                    model_path: pt_path,
                    output_dir: panel.output_dir.clone(),
                });
            }
        });
    }

    ui.add_space(4.0);

    // ── Quick load ──────────────────────────────────
    let onnx_path = format!("{}/Network.onnx", panel.output_dir);
    let meta_path = format!("{}/Network_meta.json", panel.output_dir);
    let model_exists = std::path::Path::new(&onnx_path).exists();

    if model_exists {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Modèle disponible").size(11.0).color(accent::SUCCESS));
            if ui.button(RichText::new("Charger locomotion").size(11.0)).clicked() {
                panel.pending_commands.push(AiCommand::LoadLocomotion {
                    model_path: onnx_path,
                    meta_path,
                });
            }
        });
    }
}
