//! Render settings panel — exposes all deferred pipeline parameters with styled sections.

use egui::{Ui, Color32, RichText, Slider, Vec2};
use anim_core::i18n::t;
use crate::app_state::AppState;
use crate::theme::{accent, section_header, thin_separator};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("🎨").size(14.0));
        ui.label(RichText::new(t("render_settings")).strong().size(14.0).color(accent::TEXT_BRIGHT));
    });
    thin_separator(ui);

    egui::ScrollArea::vertical().id_salt("render_scroll").show(ui, |ui| {
        // ── Lighting ─────────────────────────────────────────
        section_header(ui, "☀", t("lighting"), Color32::from_rgb(255, 220, 80));

        styled_slider(ui, t("exposure"), &mut state.render_settings.exposure, 0.1..=3.0, 0.05, 2);
        styled_slider(ui, t("sun_strength"), &mut state.render_settings.sun_strength, 0.0..=2.0, 0.05, 2);
        styled_slider(ui, t("sky_strength"), &mut state.render_settings.sky_strength, 0.0..=1.0, 0.05, 2);
        styled_slider(ui, t("ground_strength"), &mut state.render_settings.ground_strength, 0.0..=1.0, 0.05, 2);
        styled_slider(ui, t("ambient_strength"), &mut state.render_settings.ambient_strength, 0.0..=3.0, 0.05, 2);

        // Light direction
        ui.add_space(2.0);
        ui.label(RichText::new(t("light_direction")).size(10.0).color(accent::MUTED));
        styled_slider_suffix(ui, "Lacet", &mut state.render_settings.light_yaw,
            -std::f32::consts::PI..=std::f32::consts::PI, 0.01, 2, " rad");
        styled_slider_suffix(ui, "Tangage", &mut state.render_settings.light_pitch,
            0.1..=std::f32::consts::FRAC_PI_2, 0.01, 2, " rad");

        // Sun color
        ui.horizontal(|ui| {
            ui.label(RichText::new(t("sun_color")).size(11.0));
            let c = state.render_settings.sun_color;
            let mut rgb = egui::Color32::from_rgb(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
            );
            if ui.color_edit_button_srgba(&mut rgb).changed() {
                state.render_settings.sun_color = [
                    rgb.r() as f32 / 255.0,
                    rgb.g() as f32 / 255.0,
                    rgb.b() as f32 / 255.0,
                ];
            }
        });

        thin_separator(ui);

        // ── Shadows ──────────────────────────────────────────
        section_header(ui, "🌑", t("shadows"), Color32::from_rgb(160, 140, 200));

        styled_checkbox(ui, &mut state.render_settings.shadows_enabled, t("enabled"));
        if state.render_settings.shadows_enabled {
            styled_slider_log(ui, t("shadow_bias"), &mut state.render_settings.shadow_bias, 0.0001..=0.05, 0.0005, 4);
        }

        thin_separator(ui);

        // ── SSAO ─────────────────────────────────────────────
        section_header(ui, "◐", "SSAO", Color32::from_rgb(140, 180, 220));

        styled_checkbox(ui, &mut state.render_settings.ssao_enabled, t("enabled"));
        if state.render_settings.ssao_enabled {
            styled_slider(ui, t("ssao_radius"), &mut state.render_settings.ssao_radius, 0.05..=3.0, 0.05, 2);
            styled_slider(ui, t("ssao_intensity"), &mut state.render_settings.ssao_intensity, 0.0..=1.0, 0.01, 2);
            styled_slider_log(ui, t("ssao_bias"), &mut state.render_settings.ssao_bias, 0.001..=0.1, 0.001, 3);
        }

        thin_separator(ui);

        // ── Bloom ────────────────────────────────────────────
        section_header(ui, "✦", "Bloom", Color32::from_rgb(255, 200, 130));

        styled_checkbox(ui, &mut state.render_settings.bloom_enabled, t("enabled"));
        if state.render_settings.bloom_enabled {
            styled_slider(ui, t("bloom_intensity"), &mut state.render_settings.bloom_intensity, 0.0..=1.0, 0.01, 2);
            styled_slider(ui, t("bloom_spread"), &mut state.render_settings.bloom_spread, 0.1..=4.0, 0.1, 1);
        }

        thin_separator(ui);

        // ── FXAA ─────────────────────────────────────────────
        section_header(ui, "▦", "FXAA", Color32::from_rgb(180, 200, 160));

        styled_checkbox(ui, &mut state.render_settings.fxaa_enabled, t("enabled"));

        thin_separator(ui);

        // ── Grid ─────────────────────────────────────────────
        section_header(ui, "⊞", t("grid"), Color32::from_rgb(120, 160, 200));

        styled_slider(ui, "Taille", &mut state.grid_config.size, 1.0..=50.0, 1.0, 0);
        let mut div = state.grid_config.divisions as f32;
        if styled_slider_raw(ui, "Subdivisions", &mut div, 4.0..=100.0, 2.0, 0) {
            state.grid_config.divisions = div as usize;
        }

        ui.add_space(12.0);

        // Reset button
        if ui.add_sized(
            Vec2::new(ui.available_width(), 26.0),
            egui::Button::new(RichText::new(t("reset_defaults")).size(11.0).color(accent::WARNING))
                .fill(Color32::from_rgba_premultiplied(255, 185, 50, 15))
                .rounding(5.0)
                .stroke(egui::Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 185, 50, 40)))
        ).clicked() {
            state.render_settings = anim_render::RenderSettings::default();
        }
    });
}

// ── Styled control helpers ──────────────────────────────────

fn styled_slider(ui: &mut Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, step: f64, decimals: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(accent::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(Slider::new(value, range).step_by(step).fixed_decimals(decimals));
        });
    });
}

fn styled_slider_raw(ui: &mut Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, step: f64, decimals: usize) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(accent::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(Slider::new(value, range).step_by(step).fixed_decimals(decimals)).changed() {
                changed = true;
            }
        });
    });
    changed
}

fn styled_slider_suffix(ui: &mut Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, step: f64, decimals: usize, suffix: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(accent::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(Slider::new(value, range).step_by(step).fixed_decimals(decimals).suffix(suffix));
        });
    });
}

fn styled_slider_log(ui: &mut Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, step: f64, decimals: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(accent::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(Slider::new(value, range).step_by(step).fixed_decimals(decimals).logarithmic(true));
        });
    });
}

fn styled_checkbox(ui: &mut Ui, value: &mut bool, label: &str) {
    ui.horizontal(|ui| {
        ui.checkbox(value, RichText::new(label).size(11.0));
    });
}
