//! Keyboard shortcut editor panel.

use egui::{Ui, RichText, ScrollArea, Color32};
use crate::shortcuts::{ShortcutMap, Action, KeyBinding, all_keys};
use crate::theme::accent;

/// State for the shortcut editor.
pub struct ShortcutEditorState {
    /// Which action is currently being rebound (waiting for key press).
    pub rebinding: Option<Action>,
    /// Temporary modifiers for the rebind.
    pub rebind_ctrl: bool,
    pub rebind_shift: bool,
    pub rebind_alt: bool,
}

impl Default for ShortcutEditorState {
    fn default() -> Self {
        Self {
            rebinding: None,
            rebind_ctrl: false,
            rebind_shift: false,
            rebind_alt: false,
        }
    }
}

/// Show the shortcut editor panel.
pub fn show(ui: &mut Ui, shortcuts: &mut ShortcutMap, editor: &mut ShortcutEditorState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("⌨ Raccourcis clavier").size(13.0).strong().color(accent::PRIMARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("↻ Réinit.").on_hover_text("Restaurer les raccourcis par defaut").clicked() {
                shortcuts.reset_defaults();
                editor.rebinding = None;
            }
        });
    });
    ui.add_space(4.0);

    // Check for key press if we're rebinding
    if let Some(action) = editor.rebinding {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Appuyez une touche pour «{}»...", action.label()))
                .size(11.0).color(Color32::from_rgb(255, 200, 100)).italics());
            if ui.small_button("Annuler").clicked() {
                editor.rebinding = None;
            }
        });

        // Modifier checkboxes during rebind
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.checkbox(&mut editor.rebind_ctrl, RichText::new("Ctrl").size(10.0));
            ui.checkbox(&mut editor.rebind_shift, RichText::new("Shift").size(10.0));
            ui.checkbox(&mut editor.rebind_alt, RichText::new("Alt").size(10.0));
        });

        // Listen for key press
        let mut captured = None;
        ui.input(|i| {
            for &key in all_keys() {
                if i.key_pressed(key) {
                    // Skip modifier keys being used as modifiers
                    captured = Some(key);
                    break;
                }
            }
        });

        if let Some(key) = captured {
            let binding = KeyBinding {
                key,
                ctrl: editor.rebind_ctrl,
                shift: editor.rebind_shift,
                alt: editor.rebind_alt,
            };
            shortcuts.set(action, binding);
            editor.rebinding = None;
            editor.rebind_ctrl = false;
            editor.rebind_shift = false;
            editor.rebind_alt = false;
        }
    }

    ui.add_space(2.0);

    ScrollArea::vertical()
        .max_height(300.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new("shortcut_grid")
                .num_columns(3)
                .spacing([8.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.label(RichText::new("Action").size(10.5).strong().color(accent::MUTED));
                    ui.label(RichText::new("Touche").size(10.5).strong().color(accent::MUTED));
                    ui.label(RichText::new("").size(10.5));
                    ui.end_row();

                    for &action in Action::all() {
                        let binding_label = shortcuts.label(action);
                        let is_rebinding = editor.rebinding == Some(action);

                        ui.label(RichText::new(action.label()).size(11.0));

                        // Key badge
                        let badge_color = if is_rebinding {
                            Color32::from_rgb(255, 200, 100)
                        } else {
                            accent::PRIMARY
                        };
                        let badge_text = if is_rebinding {
                            "...".to_string()
                        } else {
                            binding_label
                        };
                        ui.add(
                            egui::Button::new(
                                RichText::new(&badge_text).monospace().size(10.0).color(badge_color)
                            )
                            .fill(Color32::from_rgba_premultiplied(75, 135, 255, 15))
                            .rounding(3.0)
                            .stroke(egui::Stroke::NONE)
                            .sense(egui::Sense::hover())
                        );

                        // Rebind button
                        if ui.small_button("✏").on_hover_text("Changer le raccourci").clicked() {
                            editor.rebinding = Some(action);
                            // Pre-fill modifiers from current binding
                            if let Some(b) = shortcuts.get(action) {
                                editor.rebind_ctrl = b.ctrl;
                                editor.rebind_shift = b.shift;
                                editor.rebind_alt = b.alt;
                            }
                        }

                        ui.end_row();
                    }
                });
        });
}
