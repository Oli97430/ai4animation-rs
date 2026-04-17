//! Skybox / environment panel — edit the sky settings.

use egui::{Ui, RichText};
use anim_render::{SkyEnvironment, SkyMode};
use crate::app_state::AppState;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.heading(RichText::new("🌅 Environnement").size(14.0));
    ui.separator();

    ui.label("Préréglages:");
    ui.horizontal_wrapped(|ui| {
        if ui.button("☀ Jour").clicked() {
            state.sky_environment = SkyEnvironment::daylight();
        }
        if ui.button("🌇 Coucher").clicked() {
            state.sky_environment = SkyEnvironment::sunset();
        }
        if ui.button("🌙 Nuit").clicked() {
            state.sky_environment = SkyEnvironment::night();
        }
        if ui.button("☁ Nuageux").clicked() {
            state.sky_environment = SkyEnvironment::overcast();
        }
        if ui.button("🎨 Studio").clicked() {
            state.sky_environment = SkyEnvironment::studio();
        }
    });

    ui.add_space(8.0);

    ui.label("Mode:");
    egui::ComboBox::from_id_salt("sky_mode")
        .selected_text(format!("{:?}", state.sky_environment.mode))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut state.sky_environment.mode, SkyMode::SolidColor, "Couleur unie");
            ui.selectable_value(&mut state.sky_environment.mode, SkyMode::Gradient, "Gradient");
            ui.selectable_value(&mut state.sky_environment.mode, SkyMode::Procedural, "Procédural");
        });

    ui.add_space(4.0);
    ui.label("Couleurs:");
    color_edit(ui, "Ciel", &mut state.sky_environment.sky_color);
    color_edit(ui, "Horizon", &mut state.sky_environment.horizon_color);
    color_edit(ui, "Sol", &mut state.sky_environment.ground_color);

    ui.add_space(4.0);
    ui.label("Soleil:");
    color_edit(ui, "Couleur soleil", &mut state.sky_environment.sun_color);
    ui.add(egui::Slider::new(&mut state.sky_environment.sun_intensity, 0.0..=5.0).text("Intensité"));
    ui.add(egui::Slider::new(&mut state.sky_environment.exposure, 0.0..=4.0).text("Exposition"));
    ui.add(egui::Slider::new(&mut state.sky_environment.fog_density, 0.0..=0.5).text("Brouillard"));
}

fn color_edit(ui: &mut Ui, label: &str, color: &mut [f32; 3]) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.color_edit_button_rgb(color);
    });
}
