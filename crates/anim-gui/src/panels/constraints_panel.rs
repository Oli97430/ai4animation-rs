//! Constraints panel — manage joint constraints (parent, aim, copy, pin, follow path).

use egui::{Ui, RichText};
use crate::app_state::AppState;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.heading(RichText::new("🔗 Contraintes").size(14.0));
    ui.separator();

    ui.label("Les contraintes permettent de lier un joint à un autre:");
    ui.add_space(4.0);

    egui::CollapsingHeader::new("Types disponibles")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("• Parent — suit un autre joint avec un offset");
            ui.label("• Aim — oriente vers une cible");
            ui.label("• CopyPosition — copie la position");
            ui.label("• CopyRotation — copie la rotation");
            ui.label("• PinToWorld — fixe dans l'espace monde");
            ui.label("• FollowPath — suit une courbe Catmull-Rom");
        });

    ui.add_space(8.0);
    ui.separator();

    // Joint picker
    ui.label("Joint à contraindre:");
    let bone_names: Vec<String> = if let Some(idx) = state.active_model {
        state.loaded_models.get(idx)
            .map(|m| m.model.joint_names.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if bone_names.is_empty() {
        ui.label(RichText::new("Aucun modèle actif").italics());
        return;
    }

    ui.label(format!("{} joints disponibles", bone_names.len()));

    ui.add_space(8.0);
    ui.label(RichText::new("Exemple (via IA Chat):").strong());
    ui.code("{\"action\":\"add_constraint\",\"joint\":\"Hand_R\",\"constraint_type\":\"aim\",\"target\":\"Head\"}");

    ui.add_space(4.0);
    ui.code("{\"action\":\"add_constraint\",\"joint\":\"Root\",\"constraint_type\":\"follow_path\"}");

    ui.add_space(8.0);
    ui.label(RichText::new("Les chemins prédéfinis (circle/figure_eight/linear) peuvent être générés via l'API SplinePath.").weak());
}
