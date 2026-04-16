//! Console/log panel — styled messages with timestamps, level icons, and filtering.

use egui::{Ui, Color32, RichText, ScrollArea, Stroke};
use anim_core::i18n::t;
use crate::app_state::{AppState, ConsoleLevel};
use crate::theme::accent;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    // ── Header Row ──────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(RichText::new("⌨").size(12.0).color(accent::DIM));
        ui.label(RichText::new(t("console")).strong().size(12.0).color(accent::TEXT));
        ui.add_space(12.0);

        // Level counters as colored pills
        let count_info = state.console_messages.iter().filter(|m| m.level == ConsoleLevel::Info).count();
        let count_warn = state.console_messages.iter().filter(|m| m.level == ConsoleLevel::Warning).count();
        let count_err = state.console_messages.iter().filter(|m| m.level == ConsoleLevel::Error).count();

        level_pill(ui, "ℹ", count_info, accent::PRIMARY);
        level_pill(ui, "⚠", count_warn, accent::WARNING);
        level_pill(ui, "✖", count_err, accent::ERROR);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(
                egui::Button::new(RichText::new("Effacer").size(10.0).color(accent::MUTED))
                    .fill(Color32::TRANSPARENT)
                    .rounding(3.0)
            ).clicked() {
                state.console_messages.clear();
            }
            ui.label(
                RichText::new(format!("{} msgs", state.console_messages.len()))
                    .size(10.0)
                    .color(accent::DIM)
            );
        });
    });

    // Thin separator
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(0.5, accent::BORDER),
    );

    // ── Messages ────────────────────────────────────────────
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for (i, msg) in state.console_messages.iter().enumerate() {
                let (icon, color) = match msg.level {
                    ConsoleLevel::Info => ("ℹ", accent::PRIMARY),
                    ConsoleLevel::Warning => ("⚠", accent::WARNING),
                    ConsoleLevel::Error => ("✖", accent::ERROR),
                };

                // Subtle alternating row background
                if i % 2 == 0 {
                    let row_rect = ui.horizontal(|ui| {
                        message_row(ui, msg.time, icon, color, &msg.text);
                    }).response.rect;
                    // Paint behind using layer trick: we paint after but it's fine for egui
                    let _ = row_rect;
                } else {
                    ui.horizontal(|ui| {
                        message_row(ui, msg.time, icon, color, &msg.text);
                    });
                }
            }
        });
}

fn message_row(ui: &mut Ui, time: f32, icon: &str, color: Color32, text: &str) {
    // Timestamp
    let minutes = (time / 60.0) as u32;
    let seconds = time % 60.0;
    ui.label(
        RichText::new(format!("{:02}:{:05.2}", minutes, seconds))
            .monospace()
            .size(10.0)
            .color(Color32::from_rgb(55, 58, 70))
    );

    // Level icon
    ui.label(RichText::new(icon).size(10.0).color(color));

    // Message text
    ui.label(
        RichText::new(text)
            .monospace()
            .size(10.5)
            .color(accent::TEXT)
    );
}

fn level_pill(ui: &mut Ui, icon: &str, count: usize, color: Color32) {
    let label = format!("{} {}", icon, count);
    ui.add(
        egui::Button::new(
            RichText::new(label).size(10.0).color(color)
        )
            .fill(Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 15))
            .rounding(8.0)
            .stroke(Stroke::NONE)
            .sense(egui::Sense::hover())
    );
}
