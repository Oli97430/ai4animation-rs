//! Scene hierarchy panel — tree view of entities with styled nodes and search.
//!
//! Uses `ui.set_width()` to prevent content from pushing the side panel wider.

use egui::{Ui, CollapsingHeader, RichText};
use anim_core::i18n::t;
use crate::app_state::AppState;
use crate::theme::{accent, thin_separator};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    // CRITICAL: lock the width to prevent infinite expansion.
    // Once set, child widgets wrap/clip instead of pushing the panel wider.
    let w = ui.available_width();
    ui.set_width(w);

    // ── Panel Header ────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(RichText::new("🏗").size(14.0));
        ui.label(RichText::new(t("scene_hierarchy")).strong().size(14.0).color(accent::TEXT_BRIGHT));
    });
    thin_separator(ui);

    // ── Asset Selector ──────────────────────────────────────
    if !state.loaded_models.is_empty() {
        let active_name = state.active_model
            .map(|i| state.loaded_models[i].name.clone())
            .unwrap_or_else(|| "---".to_string());

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("model_selector")
                .selected_text(truncate_str(&active_name, 20))
                .width(w - 40.0)
                .show_ui(ui, |ui| {
                    for (i, asset) in state.loaded_models.iter().enumerate() {
                        let selected = state.active_model == Some(i);
                        if ui.selectable_label(selected, &asset.name).clicked() {
                            state.active_model = Some(i);
                            state.timestamp = 0.0;
                        }
                    }
                });

            if state.active_model.is_some() {
                if ui.small_button(RichText::new("✕").color(accent::ERROR))
                    .on_hover_text("Supprimer").clicked()
                {
                    if let Some(idx) = state.active_model {
                        let name = state.loaded_models[idx].name.clone();
                        state.loaded_models.remove(idx);
                        if state.loaded_models.is_empty() {
                            state.active_model = None;
                            state.scene.clear();
                        } else {
                            state.active_model = Some(0.min(state.loaded_models.len() - 1));
                        }
                        state.timestamp = 0.0;
                        state.log_info(&format!("Supprime: {}", name));
                    }
                }
            }
        });

        // Model info line (compact, no badges — just text)
        if let Some(idx) = state.active_model {
            let asset = &state.loaded_models[idx];
            let mut info = format!("{} joints", asset.joint_entity_ids.len());
            if let Some(ref motion) = asset.motion {
                info.push_str(&format!(" · {} frames", motion.num_frames()));
            }
            if asset.skinned_mesh.is_some() {
                info.push_str(" · mesh");
            }
            ui.label(RichText::new(info).size(10.0).color(accent::MUTED));
        }

        ui.add_space(2.0);
    }

    // ── Search Filter ───────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍").size(10.0).color(accent::DIM));
        ui.add(
            egui::TextEdit::singleline(&mut state.hierarchy_filter)
                .hint_text(RichText::new(t("search")).color(accent::DIM))
                .desired_width(w - 30.0)
        );
        if !state.hierarchy_filter.is_empty() {
            if ui.small_button(RichText::new("✕").size(9.0).color(accent::MUTED)).clicked() {
                state.hierarchy_filter.clear();
            }
        }
    });
    ui.add_space(2.0);
    let filter = state.hierarchy_filter.clone();

    // ── Entity Tree ─────────────────────────────────────────
    let roots = state.scene.get_roots();
    if roots.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("🦴").size(28.0));
            ui.add_space(6.0);
            ui.label(RichText::new("Aucun modele charge").color(accent::MUTED).italics().size(11.0));
            ui.add_space(4.0);
            ui.label(RichText::new("Fichier > Importer\nou glissez un fichier ici").color(accent::DIM).size(10.0));
        });
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(w);
                for &root_id in &roots {
                    show_entity_tree(ui, state, root_id, &filter, 0);
                }
            });
    }
}

fn show_entity_tree(ui: &mut Ui, state: &mut AppState, entity_id: usize, filter: &str, depth: usize) {
    let entity = state.scene.get_entity(entity_id);
    let name = entity.name.clone();
    let children = entity.children.clone();
    let selected = state.scene.selected == Some(entity_id);

    // Filter
    if !filter.is_empty() {
        let has_match = name.to_lowercase().contains(&filter.to_lowercase())
            || children.iter().any(|&c| entity_matches_filter(state, c, filter));
        if !has_match {
            return;
        }
    }

    // Scope widget IDs by entity_id to prevent collisions when multiple
    // models have joints with the same name (e.g. two "Hips" nodes).
    ui.push_id(entity_id, |ui| {
        // Inline rename mode
        if state.rename_entity == Some(entity_id) {
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.rename_buffer)
                        .desired_width(100.0)
                        .font(egui::TextStyle::Small)
                );
                if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    state.rename_entity_confirmed();
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    state.rename_entity = None;
                    state.rename_buffer.clear();
                }
                // Auto-focus
                response.request_focus();
            });
            return;
        }

        let text_color = if selected { accent::SELECTED } else { accent::TEXT };
        let display_name = truncate_str(&name, 22);

        if children.is_empty() {
            // Leaf node
            let label = if selected {
                RichText::new(format!("● {}", display_name)).color(text_color).size(11.0)
            } else {
                RichText::new(format!("  {}", display_name)).color(text_color).size(11.0)
            };
            let response = ui.selectable_label(selected, label);
            if response.clicked() {
                state.scene.selected = Some(entity_id);
            }
            if response.double_clicked() {
                let pos = state.scene.get_position(entity_id);
                state.camera.look_at(pos);
            }
            entity_context_menu(&response, state, entity_id);
        } else {
            // Branch node
            let header_text = if selected {
                RichText::new(format!("▸ {}", display_name)).color(text_color).size(11.0)
            } else {
                RichText::new(display_name.to_string()).color(text_color).size(11.0)
            };

            let header = CollapsingHeader::new(header_text)
                .id_salt(entity_id)
                .default_open(depth == 0 && children.len() < 30)
                .show(ui, |ui| {
                    for &child_id in &children {
                        show_entity_tree(ui, state, child_id, filter, depth + 1);
                    }
                });
            if header.header_response.clicked() {
                state.scene.selected = Some(entity_id);
            }
            if header.header_response.double_clicked() {
                let pos = state.scene.get_position(entity_id);
                state.camera.look_at(pos);
            }
            entity_context_menu(&header.header_response, state, entity_id);
        }
    });
}

/// Right-click context menu for an entity node in the hierarchy.
fn entity_context_menu(response: &egui::Response, state: &mut AppState, entity_id: usize) {
    response.context_menu(|ui| {
        let name = state.scene.get_entity(entity_id).name.clone();
        ui.label(RichText::new(&name).strong().size(11.0).color(accent::TEXT_BRIGHT));
        ui.separator();

        // Focus camera
        if ui.button(RichText::new("📷 Centrer").size(11.0)).clicked() {
            let pos = state.scene.get_position(entity_id);
            state.camera.look_at(pos);
            ui.close_menu();
        }

        // Rename
        if ui.button(RichText::new("✏ Renommer").size(11.0)).clicked() {
            state.rename_entity = Some(entity_id);
            state.rename_buffer = name.clone();
            ui.close_menu();
        }

        ui.separator();

        // Select hierarchy
        let child_count = state.scene.get_entity(entity_id).successors.len();
        if child_count > 0 {
            if ui.button(RichText::new(format!("⊕ Selectionner hierarchie ({})", child_count + 1)).size(11.0)).clicked() {
                state.select_hierarchy(entity_id);
                ui.close_menu();
            }
        }

        // Deselect
        if ui.button(RichText::new("⊘ Deselectionner").size(11.0)).clicked() {
            state.deselect_all();
            ui.close_menu();
        }

        ui.separator();

        // Copy pose
        if ui.button(RichText::new("📋 Copier pose").size(11.0)).clicked() {
            state.copy_pose();
            ui.close_menu();
        }

        // Paste pose
        let has_clip = state.pose_clipboard.is_some();
        if ui.add_enabled(has_clip, egui::Button::new(RichText::new("📌 Coller pose").size(11.0))).clicked() {
            state.paste_pose();
            ui.close_menu();
        }

        // Mirror pose
        if ui.button(RichText::new("🪞 Miroir pose").size(11.0)).clicked() {
            state.mirror_pose();
            ui.close_menu();
        }
    });
}

fn entity_matches_filter(state: &AppState, entity_id: usize, filter: &str) -> bool {
    let entity = state.scene.get_entity(entity_id);
    if entity.name.to_lowercase().contains(&filter.to_lowercase()) {
        return true;
    }
    entity.children.iter().any(|&c| entity_matches_filter(state, c, filter))
}

/// Truncate a string to max_chars, adding "…" if truncated.
fn truncate_str(s: &str, max_chars: usize) -> std::borrow::Cow<'_, str> {
    if s.len() <= max_chars {
        std::borrow::Cow::Borrowed(s)
    } else {
        // Find a safe UTF-8 boundary (leave room for ellipsis)
        let mut end = max_chars.saturating_sub(1);
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        std::borrow::Cow::Owned(format!("{}…", &s[..end]))
    }
}
