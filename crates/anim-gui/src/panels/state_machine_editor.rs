//! State Machine Editor — visual node graph for animation states and transitions.

use egui::{Ui, RichText, Color32, Pos2, Vec2, Rect, Stroke};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_animation::state_machine::*;

/// Persistent panel state.
pub struct StateMachinePanel {
    pub status: String,
    /// Scroll/pan offset for the node graph canvas.
    pub scroll_offset: Vec2,
    /// State currently being dragged in the graph.
    pub dragging_state: Option<usize>,
    /// If set, we're creating a transition from this state (drag arrow).
    pub creating_transition_from: Option<usize>,
    /// Currently selected state for editing.
    pub selected_state: Option<usize>,
    /// Currently selected transition for editing.
    pub selected_transition: Option<usize>,
    /// Name buffer for creating new states.
    pub new_state_name: String,
    /// Model index for new state motion source.
    pub new_state_model: usize,
    // Transition creation
    pub new_transition_target: usize,
    pub new_transition_condition_type: usize,
    pub new_transition_param_name: String,
    pub new_transition_duration: f32,
    pub new_transition_value: f32,
    pub new_transition_bool_val: bool,
    pub new_transition_op: usize,
}

impl Default for StateMachinePanel {
    fn default() -> Self {
        Self {
            status: String::new(),
            scroll_offset: Vec2::ZERO,
            dragging_state: None,
            creating_transition_from: None,
            selected_state: None,
            selected_transition: None,
            new_state_name: "Nouvel état".to_string(),
            new_state_model: 0,
            new_transition_target: 0,
            new_transition_condition_type: 0,
            new_transition_param_name: String::new(),
            new_transition_duration: 0.3,
            new_transition_value: 0.0,
            new_transition_bool_val: true,
            new_transition_op: 0,
        }
    }
}

const STATE_WIDTH: f32 = 120.0;
const STATE_HEIGHT: f32 = 36.0;
const STATE_ROUNDING: f32 = 6.0;

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut StateMachinePanel) {
    ui.label(RichText::new("Machine d'états").size(13.0).color(accent::TEXT));
    ui.separator();

    // ── Create / manage state machine ──────────────────
    if state.state_machine.is_none() {
        if ui.button(RichText::new("+ Créer machine d'états").size(11.0)).clicked() {
            state.state_machine = Some(StateMachine::new("Principale"));
            state.log_info("[SM] Machine d'états créée");
            panel.status = "Machine d'états créée".to_string();
        }
        return;
    }

    // ── Toolbar ────────────────────────────────────────
    ui.horizontal(|ui| {
        // Add state button
        if ui.button(RichText::new("+ État").size(10.5)).clicked() {
            panel.selected_state = None;
            panel.selected_transition = None;
        }

        if ui.button(RichText::new("Supprimer SM").size(10.0).color(accent::ERROR)).clicked() {
            state.state_machine = None;
            panel.status = "Machine d'états supprimée".to_string();
            state.log_info("[SM] Machine d'états supprimée");
            return;
        }

        // Status indicator
        if let Some(ref sm) = state.state_machine {
            let active_name = sm.states.get(sm.active_state)
                .map(|s| s.name.as_str()).unwrap_or("?");
            let transitioning = if sm.is_transitioning() {
                let target_name = sm.target_state()
                    .map(|s| s.name.as_str()).unwrap_or("?");
                format!(" -> {}", target_name)
            } else {
                String::new()
            };
            ui.label(RichText::new(format!(
                "État: {}{} | {:.1}s",
                active_name, transitioning, sm.state_elapsed
            )).size(10.0).color(accent::SUCCESS));
        }
    });

    ui.separator();

    // Split: left = node graph, right = inspector
    let available = ui.available_size();
    let graph_width = (available.x * 0.65).max(200.0);

    ui.horizontal(|ui| {
        // ── Node graph canvas ──────────────────────────────
        ui.allocate_ui(Vec2::new(graph_width, available.y.min(300.0)), |ui| {
            draw_node_graph(ui, state, panel);
        });

        ui.separator();

        // ── Inspector sidebar ──────────────────────────────
        ui.allocate_ui(Vec2::new(available.x - graph_width - 10.0, available.y.min(300.0)), |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                draw_inspector(ui, state, panel);
            });
        });
    });

    // ── Parameters section ─────────────────────────────
    if let Some(ref mut sm) = state.state_machine {
        ui.separator();
        ui.collapsing(RichText::new("Paramètres").size(10.5), |ui| {
            // Bool params
            let bool_keys: Vec<String> = sm.parameters.bools.keys().cloned().collect();
            for key in &bool_keys {
                let mut val = sm.parameters.get_bool(key);
                if ui.checkbox(&mut val, RichText::new(key).size(10.0)).changed() {
                    sm.parameters.set_bool(key, val);
                }
            }
            // Float params
            let float_keys: Vec<String> = sm.parameters.floats.keys().cloned().collect();
            for key in &float_keys {
                let mut val = sm.parameters.get_float(key);
                if ui.add(egui::Slider::new(&mut val, -10.0..=10.0)
                    .text(RichText::new(key).size(10.0))).changed() {
                    sm.parameters.set_float(key, val);
                }
            }
            // Add new parameter
            ui.horizontal(|ui| {
                if ui.button(RichText::new("+ Bool").size(9.5)).clicked() {
                    let name = format!("bool_{}", sm.parameters.bools.len());
                    sm.parameters.set_bool(&name, false);
                }
                if ui.button(RichText::new("+ Float").size(9.5)).clicked() {
                    let name = format!("float_{}", sm.parameters.floats.len());
                    sm.parameters.set_float(&name, 0.0);
                }
            });
        });
    }

    // Status line
    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.5).color(accent::MUTED));
    }
}

fn draw_node_graph(ui: &mut Ui, state: &mut AppState, panel: &mut StateMachinePanel) {
    let (response, painter) = ui.allocate_painter(
        ui.available_size(),
        egui::Sense::click_and_drag(),
    );
    let canvas = response.rect;

    // Background
    painter.rect_filled(canvas, 2.0, Color32::from_rgb(20, 22, 28));
    // Grid dots
    let grid_step = 30.0;
    let offset = panel.scroll_offset;
    let start_x = ((canvas.min.x - offset.x) / grid_step).floor() as i32;
    let start_y = ((canvas.min.y - offset.y) / grid_step).floor() as i32;
    let end_x = ((canvas.max.x - offset.x) / grid_step).ceil() as i32;
    let end_y = ((canvas.max.y - offset.y) / grid_step).ceil() as i32;
    for gx in start_x..=end_x {
        for gy in start_y..=end_y {
            let px = gx as f32 * grid_step + offset.x;
            let py = gy as f32 * grid_step + offset.y;
            if canvas.contains(Pos2::new(px, py)) {
                painter.circle_filled(Pos2::new(px, py), 1.0, Color32::from_rgb(40, 42, 52));
            }
        }
    }

    let sm = match state.state_machine.as_ref() {
        Some(sm) => sm,
        None => return,
    };

    // ── Draw transitions (arrows) ──────────────────────
    for (ti, trans) in sm.transitions.iter().enumerate() {
        if trans.from >= sm.states.len() || trans.to >= sm.states.len() { continue; }
        let from_state = &sm.states[trans.from];
        let to_state = &sm.states[trans.to];

        let from_center = Pos2::new(
            canvas.min.x + from_state.position[0] + offset.x + STATE_WIDTH * 0.5,
            canvas.min.y + from_state.position[1] + offset.y + STATE_HEIGHT * 0.5,
        );
        let to_center = Pos2::new(
            canvas.min.x + to_state.position[0] + offset.x + STATE_WIDTH * 0.5,
            canvas.min.y + to_state.position[1] + offset.y + STATE_HEIGHT * 0.5,
        );

        let is_active = sm.active_transition.as_ref()
            .map(|at| at.transition_index == ti)
            .unwrap_or(false);
        let is_selected = panel.selected_transition == Some(ti);

        let color = if is_active {
            accent::SUCCESS
        } else if is_selected {
            accent::WARNING
        } else {
            Color32::from_rgb(80, 85, 100)
        };
        let width = if is_active || is_selected { 2.5 } else { 1.5 };

        painter.line_segment([from_center, to_center], Stroke::new(width, color));

        // Arrow head
        let dir = (to_center - from_center).normalized();
        let perp = Vec2::new(-dir.y, dir.x);
        let arrow_pos = from_center + (to_center - from_center) * 0.7;
        let arrow_size = 6.0;
        painter.line_segment(
            [arrow_pos, arrow_pos - dir * arrow_size + perp * arrow_size * 0.5],
            Stroke::new(width, color),
        );
        painter.line_segment(
            [arrow_pos, arrow_pos - dir * arrow_size - perp * arrow_size * 0.5],
            Stroke::new(width, color),
        );

        // Condition label on transition midpoint
        let mid = Pos2::new(
            (from_center.x + to_center.x) * 0.5,
            (from_center.y + to_center.y) * 0.5 - 8.0,
        );
        let cond_text = condition_label(&trans.condition);
        painter.text(
            mid,
            egui::Align2::CENTER_BOTTOM,
            &cond_text,
            egui::FontId::proportional(8.5),
            Color32::from_rgb(140, 145, 160),
        );
    }

    // ── Draw states (rectangles) ───────────────────────
    for (si, st) in sm.states.iter().enumerate() {
        let rect = Rect::from_min_size(
            Pos2::new(
                canvas.min.x + st.position[0] + offset.x,
                canvas.min.y + st.position[1] + offset.y,
            ),
            Vec2::new(STATE_WIDTH, STATE_HEIGHT),
        );

        if !canvas.intersects(rect) { continue; }

        let is_active = sm.active_state == si && !sm.is_transitioning();
        let is_target = sm.active_transition.as_ref()
            .map(|at| at.target_state == si).unwrap_or(false);
        let is_selected = panel.selected_state == Some(si);

        let bg = if is_active {
            Color32::from_rgb(30, 75, 45)
        } else if is_target {
            Color32::from_rgb(60, 65, 30)
        } else {
            Color32::from_rgb(35, 37, 48)
        };
        let border = if is_selected {
            accent::WARNING
        } else if is_active {
            accent::SUCCESS
        } else {
            Color32::from_rgb(55, 58, 72)
        };

        painter.rect(rect, STATE_ROUNDING, bg, Stroke::new(1.5, border));

        // State name
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &st.name,
            egui::FontId::proportional(10.5),
            accent::TEXT,
        );
    }

    // ── Handle clicks on states ────────────────────────
    if response.clicked() {
        let click_pos = response.interact_pointer_pos().unwrap_or(canvas.center());

        let mut clicked_state = None;
        let mut clicked_transition = None;

        // Check state clicks
        if let Some(ref sm) = state.state_machine {
            for (si, st) in sm.states.iter().enumerate() {
                let rect = Rect::from_min_size(
                    Pos2::new(
                        canvas.min.x + st.position[0] + offset.x,
                        canvas.min.y + st.position[1] + offset.y,
                    ),
                    Vec2::new(STATE_WIDTH, STATE_HEIGHT),
                );
                if rect.contains(click_pos) {
                    clicked_state = Some(si);
                    break;
                }
            }

            // Check transition clicks (near midpoint)
            if clicked_state.is_none() {
                for (ti, trans) in sm.transitions.iter().enumerate() {
                    if trans.from >= sm.states.len() || trans.to >= sm.states.len() { continue; }
                    let from_pos = &sm.states[trans.from].position;
                    let to_pos = &sm.states[trans.to].position;
                    let mid = Pos2::new(
                        canvas.min.x + (from_pos[0] + to_pos[0]) * 0.5 + offset.x + STATE_WIDTH * 0.5,
                        canvas.min.y + (from_pos[1] + to_pos[1]) * 0.5 + offset.y + STATE_HEIGHT * 0.5,
                    );
                    if mid.distance(click_pos) < 20.0 {
                        clicked_transition = Some(ti);
                        break;
                    }
                }
            }
        }

        if let Some(si) = clicked_state {
            panel.selected_state = Some(si);
            panel.selected_transition = None;
        } else if let Some(ti) = clicked_transition {
            panel.selected_transition = Some(ti);
            panel.selected_state = None;
        } else {
            panel.selected_state = None;
            panel.selected_transition = None;
        }
    }

    // ── Handle dragging states ─────────────────────────
    if response.drag_started() {
        if let Some(interact_pos) = response.interact_pointer_pos() {
            if let Some(ref sm) = state.state_machine {
                for (si, st) in sm.states.iter().enumerate() {
                    let rect = Rect::from_min_size(
                        Pos2::new(
                            canvas.min.x + st.position[0] + offset.x,
                            canvas.min.y + st.position[1] + offset.y,
                        ),
                        Vec2::new(STATE_WIDTH, STATE_HEIGHT),
                    );
                    if rect.contains(interact_pos) {
                        panel.dragging_state = Some(si);
                        break;
                    }
                }
            }
            // If not dragging a state, pan the canvas
            if panel.dragging_state.is_none() {
                // Canvas panning handled below
            }
        }
    }

    if response.dragged() {
        let delta = response.drag_delta();
        if let Some(si) = panel.dragging_state {
            if let Some(ref mut sm) = state.state_machine {
                if si < sm.states.len() {
                    sm.states[si].position[0] += delta.x;
                    sm.states[si].position[1] += delta.y;
                }
            }
        } else {
            // Pan canvas
            panel.scroll_offset += delta;
        }
    }

    if response.drag_stopped() {
        panel.dragging_state = None;
    }
}

fn draw_inspector(ui: &mut Ui, state: &mut AppState, panel: &mut StateMachinePanel) {
    // ── Add new state ──────────────────────────────────
    ui.label(RichText::new("Ajouter état").size(10.5).color(accent::MUTED));
    ui.text_edit_singleline(&mut panel.new_state_name);

    // Model selector
    egui::ComboBox::from_id_salt("sm_new_state_model")
        .selected_text(RichText::new(
            state.loaded_models.get(panel.new_state_model)
                .map(|a| a.name.as_str()).unwrap_or("(aucun)")
        ).size(10.0))
        .width(120.0)
        .show_ui(ui, |ui| {
            for (i, asset) in state.loaded_models.iter().enumerate() {
                if ui.selectable_label(panel.new_state_model == i,
                    RichText::new(&asset.name).size(10.0)).clicked() {
                    panel.new_state_model = i;
                }
            }
        });

    if ui.button(RichText::new("+ Créer état").size(10.0)).clicked() {
        if let Some(ref mut sm) = state.state_machine {
            let pos = [50.0 + (sm.num_states() as f32 * 30.0), 50.0 + (sm.num_states() as f32 * 20.0)];
            let source = if panel.new_state_model < state.loaded_models.len() {
                MotionSource::Clip { model_index: panel.new_state_model }
            } else {
                MotionSource::None
            };
            let id = sm.add_state(panel.new_state_name.clone(), source, pos);
            panel.status = format!("État '{}' créé (id={})", panel.new_state_name, id);
            state.log_info(&format!("[SM] État ajouté: {}", panel.new_state_name));
        }
    }

    ui.separator();

    // ── Selected state info ────────────────────────────
    if let Some(si) = panel.selected_state {
        if let Some(ref sm) = state.state_machine {
            if let Some(st) = sm.states.get(si) {
                ui.label(RichText::new(format!("État: {}", st.name)).size(11.0).color(accent::TEXT));
                let source_text = match &st.motion_source {
                    MotionSource::Clip { model_index } => {
                        state.loaded_models.get(*model_index)
                            .map(|a| format!("Clip: {}", a.name))
                            .unwrap_or("Clip: (invalide)".to_string())
                    }
                    MotionSource::Procedural { anim_type } => format!("Procédural: {}", anim_type),
                    MotionSource::None => "Aucune source".to_string(),
                };
                ui.label(RichText::new(source_text).size(10.0).color(accent::MUTED));

                if sm.active_state == si {
                    ui.label(RichText::new("(état actif)").size(9.5).color(accent::SUCCESS));
                }
            }
        }

        // Set as active
        if ui.button(RichText::new("Définir comme actif").size(10.0)).clicked() {
            if let Some(ref mut sm) = state.state_machine {
                sm.active_state = si;
                sm.state_elapsed = 0.0;
                sm.active_transition = None;
                panel.status = format!("État actif: {}", sm.states[si].name);
            }
        }

        // Delete state
        if ui.button(RichText::new("Supprimer état").size(10.0).color(accent::ERROR)).clicked() {
            if let Some(ref mut sm) = state.state_machine {
                if sm.num_states() > 1 {
                    let name = sm.states[si].name.clone();
                    sm.states.remove(si);
                    // Remove transitions referencing this state
                    sm.transitions.retain(|t| t.from != si && t.to != si);
                    // Fix indices in remaining transitions
                    for t in &mut sm.transitions {
                        if t.from > si { t.from -= 1; }
                        if t.to > si { t.to -= 1; }
                    }
                    if sm.active_state >= sm.states.len() {
                        sm.active_state = 0;
                    }
                    panel.selected_state = None;
                    panel.status = format!("État '{}' supprimé", name);
                }
            }
        }

        ui.separator();

        // ── Add transition from selected state ─────────
        ui.label(RichText::new("Ajouter transition").size(10.5).color(accent::MUTED));

        if let Some(ref sm) = state.state_machine {
            // Target state
            let target_name = sm.states.get(panel.new_transition_target)
                .map(|s| s.name.as_str()).unwrap_or("?");
            egui::ComboBox::from_id_salt("sm_trans_target")
                .selected_text(RichText::new(target_name).size(10.0))
                .width(100.0)
                .show_ui(ui, |ui| {
                    for (i, st) in sm.states.iter().enumerate() {
                        if i == si { continue; } // skip self
                        if ui.selectable_label(panel.new_transition_target == i,
                            RichText::new(&st.name).size(10.0)).clicked() {
                            panel.new_transition_target = i;
                        }
                    }
                });
        }

        // Condition type
        let cond_types = ["Bool", "Float", "Temps", "Fin anim", "Toujours"];
        egui::ComboBox::from_id_salt("sm_trans_cond")
            .selected_text(RichText::new(cond_types[panel.new_transition_condition_type]).size(10.0))
            .width(100.0)
            .show_ui(ui, |ui| {
                for (i, ct) in cond_types.iter().enumerate() {
                    if ui.selectable_label(panel.new_transition_condition_type == i,
                        RichText::new(*ct).size(10.0)).clicked() {
                        panel.new_transition_condition_type = i;
                    }
                }
            });

        // Condition-specific fields
        match panel.new_transition_condition_type {
            0 => { // Bool
                ui.text_edit_singleline(&mut panel.new_transition_param_name);
                ui.checkbox(&mut panel.new_transition_bool_val, RichText::new("Valeur").size(10.0));
            }
            1 => { // Float
                ui.text_edit_singleline(&mut panel.new_transition_param_name);
                let ops = CompareOp::all();
                egui::ComboBox::from_id_salt("sm_trans_op")
                    .selected_text(RichText::new(ops[panel.new_transition_op].label()).size(10.0))
                    .width(50.0)
                    .show_ui(ui, |ui| {
                        for (i, op) in ops.iter().enumerate() {
                            if ui.selectable_label(panel.new_transition_op == i,
                                RichText::new(op.label()).size(10.0)).clicked() {
                                panel.new_transition_op = i;
                            }
                        }
                    });
                ui.add(egui::DragValue::new(&mut panel.new_transition_value).speed(0.1)
                    .prefix("Seuil: "));
            }
            2 => { // Time
                ui.add(egui::DragValue::new(&mut panel.new_transition_value).speed(0.1)
                    .prefix("Secondes: ").suffix("s"));
            }
            _ => {} // AnimationEnd / Always — no extra fields
        }

        // Duration
        ui.add(egui::Slider::new(&mut panel.new_transition_duration, 0.05..=2.0)
            .text(RichText::new("Crossfade (s)").size(10.0)));

        if ui.button(RichText::new("+ Créer transition").size(10.0)).clicked() {
            let condition = match panel.new_transition_condition_type {
                0 => TransitionCondition::BoolParam {
                    name: panel.new_transition_param_name.clone(),
                    value: panel.new_transition_bool_val,
                },
                1 => {
                    let op = CompareOp::all()[panel.new_transition_op];
                    TransitionCondition::FloatThreshold {
                        name: panel.new_transition_param_name.clone(),
                        op,
                        value: panel.new_transition_value,
                    }
                }
                2 => TransitionCondition::TimeElapsed { seconds: panel.new_transition_value },
                3 => TransitionCondition::AnimationEnd,
                _ => TransitionCondition::Always,
            };

            if let Some(ref mut sm) = state.state_machine {
                sm.add_transition(si, panel.new_transition_target, condition,
                    panel.new_transition_duration, 1);
                panel.status = format!("Transition ajoutée: {} -> {}",
                    sm.states[si].name, sm.states[panel.new_transition_target].name);
                state.log_info(&panel.status);
            }
        }
    }

    // ── Selected transition info ───────────────────────
    if let Some(ti) = panel.selected_transition {
        if let Some(ref sm) = state.state_machine {
            if let Some(trans) = sm.transitions.get(ti) {
                ui.separator();
                let from_name = sm.states.get(trans.from).map(|s| s.name.as_str()).unwrap_or("?");
                let to_name = sm.states.get(trans.to).map(|s| s.name.as_str()).unwrap_or("?");
                ui.label(RichText::new(format!("Transition: {} -> {}", from_name, to_name))
                    .size(11.0).color(accent::TEXT));
                ui.label(RichText::new(format!("Condition: {}", condition_label(&trans.condition)))
                    .size(10.0).color(accent::MUTED));
                ui.label(RichText::new(format!("Crossfade: {:.2}s", trans.crossfade_duration))
                    .size(10.0).color(accent::MUTED));
            }
        }

        // Delete transition
        if ui.button(RichText::new("Supprimer transition").size(10.0).color(accent::ERROR)).clicked() {
            if let Some(ref mut sm) = state.state_machine {
                if ti < sm.transitions.len() {
                    sm.transitions.remove(ti);
                    panel.selected_transition = None;
                    panel.status = "Transition supprimée".to_string();
                }
            }
        }
    }
}

fn condition_label(cond: &TransitionCondition) -> String {
    match cond {
        TransitionCondition::BoolParam { name, value } => {
            format!("{}={}", name, value)
        }
        TransitionCondition::FloatThreshold { name, op, value } => {
            format!("{}{}{:.1}", name, op.label(), value)
        }
        TransitionCondition::TimeElapsed { seconds } => {
            format!("t>={:.1}s", seconds)
        }
        TransitionCondition::AnimationEnd => "fin anim".to_string(),
        TransitionCondition::Always => "toujours".to_string(),
    }
}
