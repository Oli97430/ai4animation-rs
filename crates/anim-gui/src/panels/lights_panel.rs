//! Multi-light panel — manage the scene's light list.

use egui::{Ui, RichText};
use anim_render::{Light, LightScene, LightType};
use glam::Vec3;
use crate::app_state::AppState;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.heading(RichText::new("💡 Éclairage").size(14.0));
    ui.separator();

    ui.label("Préréglages:");
    ui.horizontal_wrapped(|ui| {
        if ui.button("📸 3 points").clicked() {
            state.light_scene = LightScene::three_point_lighting();
        }
        if ui.button("🌤 Extérieur").clicked() {
            state.light_scene = LightScene::outdoor_daylight();
        }
        if ui.button("🎬 Studio").clicked() {
            state.light_scene = LightScene::studio_setup();
        }
        if ui.button("🧹 Vider").clicked() {
            state.light_scene.clear();
        }
    });

    ui.add_space(8.0);
    ui.label("Ambiance:");
    ui.horizontal(|ui| {
        ui.label("Couleur");
        ui.color_edit_button_rgb(&mut state.light_scene.ambient_color);
    });
    ui.add(egui::Slider::new(&mut state.light_scene.ambient_intensity, 0.0..=2.0).text("Intensité ambiante"));

    ui.add_space(8.0);
    ui.label(RichText::new(format!("Lumières ({}):", state.light_scene.lights.len())).strong());

    let mut to_remove: Option<usize> = None;
    for (i, light) in state.light_scene.lights.iter_mut().enumerate() {
        ui.collapsing(format!("{} #{}: {}", type_icon(&light.light_type), i, light.name), |ui| {
            ui.checkbox(&mut light.enabled, "Actif");
            ui.checkbox(&mut light.cast_shadows, "Ombres");

            ui.horizontal(|ui| {
                ui.label("Couleur");
                ui.color_edit_button_rgb(&mut light.color);
            });
            ui.add(egui::Slider::new(&mut light.intensity, 0.0..=10.0).text("Intensité"));

            match light.light_type {
                LightType::Directional => {
                    ui.add(egui::DragValue::new(&mut light.direction.x).speed(0.01).prefix("dir.X: "));
                    ui.add(egui::DragValue::new(&mut light.direction.y).speed(0.01).prefix("dir.Y: "));
                    ui.add(egui::DragValue::new(&mut light.direction.z).speed(0.01).prefix("dir.Z: "));
                }
                LightType::Point => {
                    ui.add(egui::DragValue::new(&mut light.position.x).speed(0.1).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut light.position.y).speed(0.1).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut light.position.z).speed(0.1).prefix("Z: "));
                    ui.add(egui::Slider::new(&mut light.range, 0.0..=50.0).text("Portée"));
                }
                LightType::Spot => {
                    ui.add(egui::DragValue::new(&mut light.position.x).speed(0.1).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut light.position.y).speed(0.1).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut light.position.z).speed(0.1).prefix("Z: "));
                    ui.add(egui::Slider::new(&mut light.range, 0.0..=50.0).text("Portée"));
                    ui.add(egui::Slider::new(&mut light.outer_angle, 0.0..=1.5).text("Angle ext."));
                    ui.add(egui::Slider::new(&mut light.inner_angle, 0.0..=1.5).text("Angle int."));
                }
            }

            if ui.button("🗑 Supprimer").clicked() {
                to_remove = Some(i);
            }
        });
    }

    if let Some(idx) = to_remove {
        state.light_scene.remove_light(idx);
    }

    ui.add_space(8.0);
    ui.label("Ajouter une lumière:");
    ui.horizontal(|ui| {
        if ui.button("+ Directionnelle").clicked() {
            state.light_scene.add_light(Light::directional(
                &format!("Dir {}", state.light_scene.lights.len()),
                Vec3::new(-0.5, -0.8, -0.3).normalize(),
                [1.0, 1.0, 1.0],
                1.5,
            ));
        }
        if ui.button("+ Point").clicked() {
            state.light_scene.add_light(Light::point(
                &format!("Point {}", state.light_scene.lights.len()),
                Vec3::new(0.0, 3.0, 0.0),
                [1.0, 1.0, 1.0],
                2.0,
                10.0,
            ));
        }
        if ui.button("+ Spot").clicked() {
            state.light_scene.add_light(Light::spot(
                &format!("Spot {}", state.light_scene.lights.len()),
                Vec3::new(0.0, 5.0, 0.0),
                Vec3::NEG_Y,
                [1.0, 1.0, 1.0],
                3.0,
                15.0,
                0.6,
            ));
        }
    });
}

fn type_icon(light_type: &LightType) -> &'static str {
    match light_type {
        LightType::Directional => "☀",
        LightType::Point => "💡",
        LightType::Spot => "🔦",
    }
}
