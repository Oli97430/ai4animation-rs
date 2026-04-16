//! Blend Tree Editor — visual node graph for blend spaces and mix nodes.

use egui::{Ui, RichText, Color32, Pos2, Vec2, Rect, Stroke};
use crate::app_state::AppState;
use crate::theme::accent;
use anim_animation::blend_tree::*;

const NODE_W: f32 = 130.0;
const NODE_H: f32 = 40.0;

/// Persistent panel state.
pub struct BlendTreePanel {
    pub status: String,
    pub scroll_offset: Vec2,
    pub selected_node: Option<usize>,
    pub dragging_node: Option<usize>,
    // New node creation
    pub new_node_type: usize, // 0=Clip, 1=Blend1D, 2=Blend2D, 3=Lerp
    pub new_node_name: String,
    pub new_node_param: String,
    pub new_node_param2: String,
    pub new_node_model: usize,
}

impl Default for BlendTreePanel {
    fn default() -> Self {
        Self {
            status: String::new(),
            scroll_offset: Vec2::ZERO,
            selected_node: None,
            dragging_node: None,
            new_node_type: 0,
            new_node_name: "Nouveau".to_string(),
            new_node_param: "speed".to_string(),
            new_node_param2: "direction".to_string(),
            new_node_model: 0,
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, panel: &mut BlendTreePanel) {
    ui.label(RichText::new("Blend Tree").size(13.0).color(accent::TEXT));
    ui.separator();

    // Create / destroy
    if state.blend_tree.is_none() {
        if ui.button(RichText::new("+ Créer Blend Tree").size(11.0)).clicked() {
            state.blend_tree = Some(BlendTree::new("Principal"));
            state.log_info("[BT] Blend tree créé");
        }
        return;
    }

    // Toolbar
    ui.horizontal(|ui| {
        if ui.button(RichText::new("Supprimer").size(10.0).color(accent::ERROR)).clicked() {
            state.blend_tree = None;
            panel.status = "Blend tree supprimé".to_string();
            return;
        }
        if let Some(ref bt) = state.blend_tree {
            ui.label(RichText::new(format!("{} noeud(s)", bt.num_nodes()))
                .size(10.0).color(accent::MUTED));
        }
    });

    ui.separator();

    let available = ui.available_size();
    let graph_width = (available.x * 0.6).max(200.0);

    ui.horizontal(|ui| {
        // Node graph
        ui.allocate_ui(Vec2::new(graph_width, available.y.min(280.0)), |ui| {
            draw_graph(ui, state, panel);
        });

        ui.separator();

        // Inspector
        ui.allocate_ui(Vec2::new(available.x - graph_width - 10.0, available.y.min(280.0)), |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                draw_inspector(ui, state, panel);
            });
        });
    });

    // Parameters section
    if let Some(ref mut bt) = state.blend_tree {
        ui.separator();
        let params = bt.used_parameters();
        if !params.is_empty() {
            ui.label(RichText::new("Paramètres").size(10.5).color(accent::MUTED));
            for p in &params {
                let mut val = bt.get_parameter(p);
                if ui.add(egui::Slider::new(&mut val, -5.0..=5.0)
                    .text(RichText::new(p).size(10.0))).changed() {
                    bt.set_parameter(p, val);
                }
            }
        }
    }

    if !panel.status.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(&panel.status).size(9.5).color(accent::MUTED));
    }
}

fn draw_graph(ui: &mut Ui, state: &mut AppState, panel: &mut BlendTreePanel) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    let canvas = response.rect;

    // Background
    painter.rect_filled(canvas, 2.0, Color32::from_rgb(20, 22, 28));

    let bt = match state.blend_tree.as_ref() {
        Some(bt) => bt,
        None => return,
    };
    let offset = panel.scroll_offset;

    // Draw connections first
    for (ni, node) in bt.nodes.iter().enumerate() {
        let children_indices = get_children(node);
        let from_pos = node_center(node, canvas.min, offset);

        for &ci in &children_indices {
            if ci < bt.nodes.len() {
                let to_pos = node_center(&bt.nodes[ci], canvas.min, offset);
                let color = if ni == bt.root {
                    Color32::from_rgb(90, 180, 90)
                } else {
                    Color32::from_rgb(70, 75, 90)
                };
                painter.line_segment([from_pos, to_pos], Stroke::new(1.5, color));
                // Arrow
                let dir = (to_pos - from_pos).normalized();
                let perp = Vec2::new(-dir.y, dir.x);
                let mid = from_pos + (to_pos - from_pos) * 0.65;
                painter.line_segment([mid, mid - dir * 5.0 + perp * 3.0], Stroke::new(1.5, color));
                painter.line_segment([mid, mid - dir * 5.0 - perp * 3.0], Stroke::new(1.5, color));
            }
        }
    }

    // Draw nodes
    for (ni, node) in bt.nodes.iter().enumerate() {
        let pos = node_position(node);
        let rect = Rect::from_min_size(
            Pos2::new(canvas.min.x + pos[0] + offset.x, canvas.min.y + pos[1] + offset.y),
            Vec2::new(NODE_W, NODE_H),
        );
        if !canvas.intersects(rect) { continue; }

        let is_root = ni == bt.root;
        let is_selected = panel.selected_node == Some(ni);

        let bg = match node {
            BlendTreeNode::Clip(_) => Color32::from_rgb(35, 45, 55),
            BlendTreeNode::Blend1D(_) => Color32::from_rgb(45, 40, 55),
            BlendTreeNode::Blend2D(_) => Color32::from_rgb(50, 40, 45),
            BlendTreeNode::Lerp(_) => Color32::from_rgb(40, 50, 45),
        };
        let border = if is_selected { accent::WARNING }
            else if is_root { accent::SUCCESS }
            else { Color32::from_rgb(55, 58, 72) };

        painter.rect(rect, 5.0, bg, Stroke::new(if is_selected || is_root { 2.0 } else { 1.0 }, border));

        let label = node_label(node);
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, &label,
            egui::FontId::proportional(9.5), accent::TEXT);

        // Type badge
        let type_label = match node {
            BlendTreeNode::Clip(_) => "Clip",
            BlendTreeNode::Blend1D(_) => "1D",
            BlendTreeNode::Blend2D(_) => "2D",
            BlendTreeNode::Lerp(_) => "Lerp",
        };
        painter.text(
            Pos2::new(rect.min.x + 4.0, rect.min.y + 2.0),
            egui::Align2::LEFT_TOP, type_label,
            egui::FontId::proportional(7.5), accent::DIM,
        );
    }

    // Click handling
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let mut clicked = None;
            if let Some(ref bt) = state.blend_tree {
                for (ni, node) in bt.nodes.iter().enumerate() {
                    let np = node_position(node);
                    let rect = Rect::from_min_size(
                        Pos2::new(canvas.min.x + np[0] + offset.x, canvas.min.y + np[1] + offset.y),
                        Vec2::new(NODE_W, NODE_H),
                    );
                    if rect.contains(pos) { clicked = Some(ni); break; }
                }
            }
            panel.selected_node = clicked;
        }
    }

    // Drag
    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Some(ref bt) = state.blend_tree {
                for (ni, node) in bt.nodes.iter().enumerate() {
                    let np = node_position(node);
                    let rect = Rect::from_min_size(
                        Pos2::new(canvas.min.x + np[0] + offset.x, canvas.min.y + np[1] + offset.y),
                        Vec2::new(NODE_W, NODE_H),
                    );
                    if rect.contains(pos) { panel.dragging_node = Some(ni); break; }
                }
            }
        }
    }
    if response.dragged() {
        let delta = response.drag_delta();
        if let Some(ni) = panel.dragging_node {
            if let Some(ref mut bt) = state.blend_tree {
                if ni < bt.nodes.len() {
                    let pos = node_position_mut(&mut bt.nodes[ni]);
                    pos[0] += delta.x;
                    pos[1] += delta.y;
                }
            }
        } else {
            panel.scroll_offset += delta;
        }
    }
    if response.drag_stopped() { panel.dragging_node = None; }
}

fn draw_inspector(ui: &mut Ui, state: &mut AppState, panel: &mut BlendTreePanel) {
    // Add new node
    ui.label(RichText::new("Ajouter noeud").size(10.5).color(accent::MUTED));

    let types = ["Clip", "Blend 1D", "Blend 2D", "Lerp"];
    egui::ComboBox::from_id_salt("bt_new_type")
        .selected_text(RichText::new(types[panel.new_node_type]).size(10.0))
        .width(100.0)
        .show_ui(ui, |ui| {
            for (i, t) in types.iter().enumerate() {
                if ui.selectable_label(panel.new_node_type == i,
                    RichText::new(*t).size(10.0)).clicked() {
                    panel.new_node_type = i;
                }
            }
        });

    ui.text_edit_singleline(&mut panel.new_node_name);

    match panel.new_node_type {
        0 => { // Clip
            egui::ComboBox::from_id_salt("bt_clip_model")
                .selected_text(RichText::new(
                    state.loaded_models.get(panel.new_node_model)
                        .map(|a| a.name.as_str()).unwrap_or("(aucun)")
                ).size(10.0))
                .width(120.0)
                .show_ui(ui, |ui| {
                    for (i, a) in state.loaded_models.iter().enumerate() {
                        if ui.selectable_label(panel.new_node_model == i,
                            RichText::new(&a.name).size(10.0)).clicked() {
                            panel.new_node_model = i;
                        }
                    }
                });
        }
        1 => { // Blend1D
            ui.horizontal(|ui| {
                ui.label(RichText::new("Param:").size(9.5));
                ui.text_edit_singleline(&mut panel.new_node_param);
            });
        }
        2 => { // Blend2D
            ui.horizontal(|ui| {
                ui.label(RichText::new("X:").size(9.5));
                ui.text_edit_singleline(&mut panel.new_node_param);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Y:").size(9.5));
                ui.text_edit_singleline(&mut panel.new_node_param2);
            });
        }
        3 => { // Lerp
            ui.horizontal(|ui| {
                ui.label(RichText::new("Param:").size(9.5));
                ui.text_edit_singleline(&mut panel.new_node_param);
            });
        }
        _ => {}
    }

    if ui.button(RichText::new("+ Créer").size(10.0)).clicked() {
        if let Some(ref mut bt) = state.blend_tree {
            let n = bt.num_nodes();
            let pos = [50.0 + (n as f32 % 4.0) * 150.0, 50.0 + (n as f32 / 4.0).floor() * 70.0];
            let node = match panel.new_node_type {
                0 => BlendTreeNode::Clip(ClipNode {
                    name: panel.new_node_name.clone(),
                    model_index: panel.new_node_model,
                    speed: 1.0, position: pos,
                }),
                1 => BlendTreeNode::Blend1D(Blend1DNode {
                    name: panel.new_node_name.clone(),
                    parameter: panel.new_node_param.clone(),
                    children: Vec::new(), position: pos,
                }),
                2 => BlendTreeNode::Blend2D(Blend2DNode {
                    name: panel.new_node_name.clone(),
                    param_x: panel.new_node_param.clone(),
                    param_y: panel.new_node_param2.clone(),
                    children: Vec::new(), position: pos,
                }),
                _ => BlendTreeNode::Lerp(LerpNode {
                    name: panel.new_node_name.clone(),
                    parameter: panel.new_node_param.clone(),
                    child_a: 0, child_b: 0, position: pos,
                }),
            };
            let idx = bt.add_node(node);
            if n == 0 { bt.root = idx; }
            panel.status = format!("Noeud '{}' créé (id={})", panel.new_node_name, idx);
        }
    }

    ui.separator();

    // Selected node info
    if let Some(ni) = panel.selected_node {
        // Phase 1: read-only — collect display info + decisions
        let (node_info, is_root, can_set_root, is_blend_parent, child_candidates) = {
            if let Some(ref bt) = state.blend_tree {
                if let Some(node) = bt.nodes.get(ni) {
                    let label = node_label(node);
                    let info_text = match node {
                        BlendTreeNode::Clip(c) => {
                            let model_name = state.loaded_models.get(c.model_index)
                                .map(|a| a.name.as_str()).unwrap_or("?");
                            vec![
                                format!("Modèle: {} (idx={})", model_name, c.model_index),
                                format!("Vitesse: {:.1}x", c.speed),
                            ]
                        }
                        BlendTreeNode::Blend1D(b) => {
                            let mut lines = vec![
                                format!("Param: {}", b.parameter),
                                format!("{} enfants", b.children.len()),
                            ];
                            for (thresh, child_idx) in &b.children {
                                lines.push(format!("  [{:.2}] → noeud {}", thresh, child_idx));
                            }
                            lines
                        }
                        BlendTreeNode::Blend2D(b) => {
                            vec![
                                format!("Params: {} × {}", b.param_x, b.param_y),
                                format!("{} enfants", b.children.len()),
                            ]
                        }
                        BlendTreeNode::Lerp(l) => {
                            vec![
                                format!("Param: {}", l.parameter),
                                format!("A={}, B={}", l.child_a, l.child_b),
                            ]
                        }
                    };
                    let is_root = bt.root == ni;
                    let is_blend = matches!(node, BlendTreeNode::Blend1D(_) | BlendTreeNode::Blend2D(_));
                    // Collect candidates (index, label) for add-child
                    let candidates: Vec<(usize, String)> = if is_blend {
                        bt.nodes.iter().enumerate()
                            .filter(|(i, _)| *i != ni)
                            .map(|(i, n)| (i, node_label(n)))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    (Some((label, info_text)), is_root, !is_root, is_blend, candidates)
                } else {
                    (None, false, false, false, Vec::new())
                }
            } else {
                (None, false, false, false, Vec::new())
            }
        };

        // Phase 2: render UI + mutations
        if let Some((label, info_lines)) = node_info {
            ui.label(RichText::new(format!("Noeud #{}: {}", ni, label))
                .size(11.0).color(accent::TEXT));
            for line in &info_lines {
                ui.label(RichText::new(line).size(10.0).color(accent::MUTED));
            }

            // Set as root
            if is_root {
                ui.label(RichText::new("(racine)").size(9.5).color(accent::SUCCESS));
            } else if can_set_root {
                if ui.button(RichText::new("Définir comme racine").size(10.0)).clicked() {
                    if let Some(ref mut bt) = state.blend_tree {
                        bt.root = ni;
                        panel.status = "Racine mise à jour".to_string();
                    }
                }
            }

            // Add child (for Blend1D / Blend2D)
            if is_blend_parent && !child_candidates.is_empty() {
                ui.separator();
                ui.label(RichText::new("Ajouter enfant").size(10.0).color(accent::MUTED));
                for (ci, clabel) in &child_candidates {
                    if ui.button(RichText::new(format!("+ {} ({})", clabel, ci)).size(9.5)).clicked() {
                        if let Some(ref mut bt) = state.blend_tree {
                            match &mut bt.nodes[ni] {
                                BlendTreeNode::Blend1D(ref mut b) => {
                                    let thresh = b.children.len() as f32;
                                    b.children.push((thresh, *ci));
                                    b.children.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                                }
                                BlendTreeNode::Blend2D(ref mut b) => {
                                    let x = b.children.len() as f32;
                                    b.children.push((x, 0.0, *ci));
                                }
                                _ => {}
                            }
                            panel.status = format!("Enfant {} ajouté", ci);
                        }
                        break;
                    }
                }
            }
        }
    }
}

// Helpers
fn node_position(node: &BlendTreeNode) -> [f32; 2] {
    match node {
        BlendTreeNode::Clip(c) => c.position,
        BlendTreeNode::Blend1D(b) => b.position,
        BlendTreeNode::Blend2D(b) => b.position,
        BlendTreeNode::Lerp(l) => l.position,
    }
}

fn node_position_mut(node: &mut BlendTreeNode) -> &mut [f32; 2] {
    match node {
        BlendTreeNode::Clip(c) => &mut c.position,
        BlendTreeNode::Blend1D(b) => &mut b.position,
        BlendTreeNode::Blend2D(b) => &mut b.position,
        BlendTreeNode::Lerp(l) => &mut l.position,
    }
}

fn node_center(node: &BlendTreeNode, canvas_min: Pos2, offset: Vec2) -> Pos2 {
    let p = node_position(node);
    Pos2::new(canvas_min.x + p[0] + offset.x + NODE_W * 0.5,
              canvas_min.y + p[1] + offset.y + NODE_H * 0.5)
}

fn node_label(node: &BlendTreeNode) -> String {
    match node {
        BlendTreeNode::Clip(c) => c.name.clone(),
        BlendTreeNode::Blend1D(b) => b.name.clone(),
        BlendTreeNode::Blend2D(b) => b.name.clone(),
        BlendTreeNode::Lerp(l) => l.name.clone(),
    }
}

fn get_children(node: &BlendTreeNode) -> Vec<usize> {
    match node {
        BlendTreeNode::Clip(_) => vec![],
        BlendTreeNode::Blend1D(b) => b.children.iter().map(|(_, i)| *i).collect(),
        BlendTreeNode::Blend2D(b) => b.children.iter().map(|(_, _, i)| *i).collect(),
        BlendTreeNode::Lerp(l) => vec![l.child_a, l.child_b],
    }
}
