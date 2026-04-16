//! 3D Viewport input handling, overlays, and SketchUp-style tools.
//!
//! Interaction model:
//!   - Right-click drag: orbit/look camera
//!   - Middle-click drag: pan camera
//!   - Scroll: zoom
//!   - Left-click: select joint (ray pick)
//!   - Left-drag (Move tool): grab and drag a joint in 3D
//!   - Left-drag (Rotate tool): rotate selected entity
//!   - Double-click: reset camera

use egui::{Ui, Response, Color32, RichText, Stroke};
use glam::{Vec3, Mat4, Quat};
use crate::app_state::{AppState, Tool, GizmoAxis, UndoSnapshot};
use crate::theme::accent;
use anim_render::CameraMode;

// ────────────────────────────────────────────────────────────
// Geometry helpers
// ────────────────────────────────────────────────────────────

fn point_ray_distance(point: Vec3, ray_origin: Vec3, ray_dir: Vec3) -> f32 {
    let v = point - ray_origin;
    let t = v.dot(ray_dir);
    if t < 0.0 {
        return v.length();
    }
    let closest = ray_origin + ray_dir * t;
    (point - closest).length()
}

fn ray_plane_intersect(ray_origin: Vec3, ray_dir: Vec3, plane_point: Vec3, plane_normal: Vec3) -> Option<Vec3> {
    let denom = ray_dir.dot(plane_normal);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (plane_point - ray_origin).dot(plane_normal) / denom;
    if t < 0.0 {
        return None;
    }
    Some(ray_origin + ray_dir * t)
}

fn pick_joint(
    state: &AppState,
    screen_x: f32,
    screen_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    threshold: f32,
) -> Option<(usize, Vec3)> {
    let (ray_origin, ray_dir) = state.camera.screen_ray(screen_x, screen_y, viewport_w, viewport_h);

    let mut best: Option<(usize, f32, Vec3)> = None;

    for asset in &state.loaded_models {
        for &eid in &asset.joint_entity_ids {
            let pos = anim_math::transform::Transform::get_position(&state.scene.transforms[eid]);
            let dist = point_ray_distance(pos, ray_origin, ray_dir);
            let cam_dist = (pos - state.camera.position).length().max(0.1);
            let screen_dist = dist / cam_dist * viewport_h;

            if screen_dist < threshold {
                let is_closer = match &best {
                    Some((_, d, _)) => dist < *d,
                    None => true,
                };
                if is_closer {
                    best = Some((eid, dist, pos));
                }
            }
        }
    }

    best.map(|(eid, _, pos)| (eid, pos))
}

fn drag_plane(entity_pos: Vec3, axis: GizmoAxis, camera_forward: Vec3) -> (Vec3, Vec3) {
    match axis {
        GizmoAxis::X => {
            let normal = Vec3::X.cross(camera_forward).cross(Vec3::X).normalize_or_zero();
            let normal = if normal.length_squared() < 0.01 { Vec3::Y } else { normal };
            (entity_pos, normal)
        }
        GizmoAxis::Y => {
            let normal = Vec3::Y.cross(camera_forward).cross(Vec3::Y).normalize_or_zero();
            let normal = if normal.length_squared() < 0.01 { Vec3::Z } else { normal };
            (entity_pos, normal)
        }
        GizmoAxis::Z => {
            let normal = Vec3::Z.cross(camera_forward).cross(Vec3::Z).normalize_or_zero();
            let normal = if normal.length_squared() < 0.01 { Vec3::Y } else { normal };
            (entity_pos, normal)
        }
        GizmoAxis::None => {
            (entity_pos, -camera_forward)
        }
    }
}

fn constrain_to_axis(movement: Vec3, axis: GizmoAxis) -> Vec3 {
    match axis {
        GizmoAxis::X => Vec3::new(movement.x, 0.0, 0.0),
        GizmoAxis::Y => Vec3::new(0.0, movement.y, 0.0),
        GizmoAxis::Z => Vec3::new(0.0, 0.0, movement.z),
        GizmoAxis::None => movement,
    }
}

// ────────────────────────────────────────────────────────────
// Input handling
// ────────────────────────────────────────────────────────────

pub fn handle_input(ui: &Ui, response: &Response, state: &mut AppState) {
    let camera_mode = state.camera.mode;
    let rect = response.rect;
    let vw = rect.width();
    let vh = rect.height();

    // Camera controls
    match camera_mode {
        CameraMode::Free => {
            if response.dragged_by(egui::PointerButton::Secondary) {
                let delta = response.drag_delta();
                state.camera.walk_look(delta.x, delta.y);
            }
        }
        _ => {
            if response.dragged_by(egui::PointerButton::Secondary) {
                let delta = response.drag_delta();
                state.camera.orbit_rotate(delta.x, delta.y);
            }
            if response.dragged_by(egui::PointerButton::Middle) {
                let delta = response.drag_delta();
                state.camera.orbit_pan(delta.x, delta.y);
            }
            if response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll.abs() > 0.0 {
                    state.camera.orbit_zoom(scroll * 0.1);
                }
            }
            if response.double_clicked() {
                state.camera.reset();
            }
        }
    }

    // Walk mode WASD
    if camera_mode == CameraMode::Free && response.hovered() {
        let dt = state.time.delta_time;
        let speed = state.walk_speed;
        let (fwd, right, up): (f32, f32, f32) = ui.input(|i| {
            let f: f32 = if i.key_down(egui::Key::W) { 1.0 } else { 0.0 }
                       - if i.key_down(egui::Key::S) { 1.0 } else { 0.0 };
            let r: f32 = if i.key_down(egui::Key::D) { 1.0 } else { 0.0 }
                       - if i.key_down(egui::Key::A) { 1.0 } else { 0.0 };
            let u: f32 = if i.key_down(egui::Key::E) { 1.0 } else { 0.0 }
                       - if i.key_down(egui::Key::Q) { 1.0 } else { 0.0 };
            (f, r, u)
        });
        if fwd.abs() > 0.0 || right.abs() > 0.0 || up.abs() > 0.0 {
            state.camera.walk_move(fwd, right, up, speed, dt);
        }
    }

    // Left-click: begin drag or select
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(pos) = response.interact_pointer_pos() {
            let local_x = pos.x - rect.min.x;
            let local_y = pos.y - rect.min.y;

            match state.active_tool {
                Tool::Move | Tool::Rotate => {
                    if let Some((eid, world_pos)) = pick_joint(state, local_x, local_y, vw, vh, 25.0) {
                        state.scene.selected = Some(eid);
                        let transform = state.scene.get_transform(eid);
                        let name = state.scene.get_entity(eid).name.clone();
                        let desc = match state.active_tool {
                            Tool::Move => format!("Deplacer {}", name),
                            Tool::Rotate => format!("Rotation {}", name),
                            _ => name,
                        };
                        state.history.push(UndoSnapshot {
                            description: desc,
                            entity_id: eid,
                            transform,
                        });
                        state.drag.entity_id = Some(eid);
                        state.drag.start_world = world_pos;
                        state.drag.start_entity_pos = world_pos;
                        state.drag.active = true;
                        state.drag.rotate_angle = 0.0;
                    }
                }
                Tool::Select => {
                    if let Some((eid, _)) = pick_joint(state, local_x, local_y, vw, vh, 25.0) {
                        // Check for Ctrl modifier (multi-select)
                        let ctrl = ui.input(|i| i.modifiers.ctrl);
                        if ctrl {
                            state.toggle_multi_select(eid);
                        } else {
                            state.select_single(eid);
                        }
                        state.log_info(&format!("Selectionne: {}", state.scene.get_entity(eid).name));
                    }
                }
                Tool::Measure => {
                    if let Some((_, world_pos)) = pick_joint(state, local_x, local_y, vw, vh, 25.0) {
                        if state.measure.start.is_none() || state.measure.end.is_some() {
                            state.measure.start = Some(world_pos);
                            state.measure.end = None;
                        } else {
                            state.measure.end = Some(world_pos);
                            if let Some(d) = state.measure.distance() {
                                state.log_info(&format!("Distance: {:.4} unites", d));
                            }
                        }
                    }
                }
                Tool::Ik => {
                    if let Some((eid, world_pos)) = pick_joint(state, local_x, local_y, vw, vh, 25.0) {
                        if state.ik_chain_root.is_none() {
                            state.ik_chain_root = Some(eid);
                            state.scene.selected = Some(eid);
                            state.log_info(&format!("IK root: {}", state.scene.get_entity(eid).name));
                        } else if state.ik_chain_tip.is_none() || state.ik_chain_tip == state.ik_chain_root {
                            state.ik_chain_tip = Some(eid);
                            state.scene.selected = Some(eid);
                            state.log_info(&format!("IK tip: {} — glissez pour resoudre", state.scene.get_entity(eid).name));
                            state.drag.entity_id = Some(eid);
                            state.drag.start_world = world_pos;
                            state.drag.start_entity_pos = world_pos;
                            state.drag.active = true;
                        } else {
                            state.ik_chain_root = Some(eid);
                            state.ik_chain_tip = None;
                            state.scene.selected = Some(eid);
                            state.log_info(&format!("IK root: {}", state.scene.get_entity(eid).name));
                        }
                    }
                }
            }
        }
    }

    // Left-drag: move/rotate the entity
    if response.dragged_by(egui::PointerButton::Primary) && state.drag.active {
        if let Some(eid) = state.drag.entity_id {
            if let Some(pos) = response.interact_pointer_pos() {
                let local_x = pos.x - rect.min.x;
                let local_y = pos.y - rect.min.y;
                let camera_fwd = (state.camera.target - state.camera.position).normalize();

                match state.active_tool {
                    Tool::Move => {
                        let (plane_pt, plane_n) = drag_plane(
                            state.drag.start_entity_pos,
                            state.gizmo_axis,
                            camera_fwd,
                        );
                        let (ray_o, ray_d) = state.camera.screen_ray(local_x, local_y, vw, vh);
                        if let Some(hit) = ray_plane_intersect(ray_o, ray_d, plane_pt, plane_n) {
                            let movement = hit - state.drag.start_world;
                            let constrained = constrain_to_axis(movement, state.gizmo_axis);
                            let mut new_pos = state.drag.start_entity_pos + constrained;
                            new_pos = state.snap_position(new_pos);
                            state.scene.set_position(eid, new_pos, true);
                        }
                    }
                    Tool::Rotate => {
                        let delta = response.drag_delta();
                        let angle = delta.x * 0.01;
                        state.drag.rotate_angle += angle;
                        let axis = match state.gizmo_axis {
                            GizmoAxis::X => Vec3::X,
                            GizmoAxis::Z => Vec3::Z,
                            _ => Vec3::Y,
                        };
                        let entity_pos = anim_math::transform::Transform::get_position(
                            &state.scene.transforms[eid]
                        );
                        let rotation = Mat4::from_translation(entity_pos)
                            * Mat4::from_quat(Quat::from_axis_angle(axis, angle))
                            * Mat4::from_translation(-entity_pos);
                        let current = state.scene.get_transform(eid);
                        state.scene.set_transform(eid, rotation * current, true);
                    }
                    Tool::Ik => {
                        if let (Some(root_eid), Some(tip_eid)) = (state.ik_chain_root, state.ik_chain_tip) {
                            let camera_fwd = (state.camera.target - state.camera.position).normalize();
                            let (plane_pt, plane_n) = drag_plane(
                                state.drag.start_entity_pos,
                                state.gizmo_axis,
                                camera_fwd,
                            );
                            let (ray_o, ray_d) = state.camera.screen_ray(local_x, local_y, vw, vh);
                            if let Some(target) = ray_plane_intersect(ray_o, ray_d, plane_pt, plane_n) {
                                let chain_eids = state.scene.get_chain(root_eid, tip_eid);
                                if chain_eids.len() >= 2 {
                                    let chain_positions: Vec<Vec3> = chain_eids.iter()
                                        .map(|&e| state.scene.get_position(e))
                                        .collect();
                                    let mut solver = anim_ik::FabrikSolver::new(chain_positions);
                                    solver.max_iterations = 20;

                                    // Apply IK constraints if enabled
                                    if state.ik_use_constraints {
                                        use crate::app_state::IkPreset;
                                        let n = chain_eids.len();
                                        let constraints: Vec<anim_ik::JointConstraint> = match state.ik_preset {
                                            IkPreset::HumanArm => {
                                                // shoulder → elbow → wrist
                                                (0..n).map(|i| {
                                                    if i == 0 { anim_ik::JointConstraint::shoulder() }
                                                    else if i == n - 1 { anim_ik::JointConstraint::free() }
                                                    else { anim_ik::JointConstraint::elbow() }
                                                }).collect()
                                            }
                                            IkPreset::HumanLeg => {
                                                // hip → knee → ankle
                                                (0..n).map(|i| {
                                                    if i == 0 { anim_ik::JointConstraint::shoulder() }
                                                    else if i == n - 1 { anim_ik::JointConstraint::free() }
                                                    else { anim_ik::JointConstraint::knee() }
                                                }).collect()
                                            }
                                            _ => {
                                                // Default: moderate limits on interior joints
                                                (0..n).map(|i| {
                                                    if i == 0 || i == n - 1 {
                                                        anim_ik::JointConstraint::free()
                                                    } else {
                                                        anim_ik::JointConstraint::new(0.0, std::f32::consts::PI * 0.8)
                                                    }
                                                }).collect()
                                            }
                                        };
                                        solver.constraints = constraints;
                                    }

                                    // Apply pole target if enabled
                                    if state.ik_use_pole_target {
                                        solver.pole_target = Some(anim_ik::PoleTarget::new(
                                            state.ik_pole_position,
                                            state.ik_pole_weight,
                                        ));
                                    }

                                    solver.solve(target);
                                    let solved = solver.get_positions();
                                    for (i, &chain_eid) in chain_eids.iter().enumerate() {
                                        if i < solved.len() {
                                            state.scene.set_position(chain_eid, solved[i], false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Left-release: end drag
    if response.drag_stopped_by(egui::PointerButton::Primary) && state.drag.active {
        if let Some(eid) = state.drag.entity_id {
            if state.active_tool == Tool::Move {
                let name = state.scene.get_entity(eid).name.clone();
                let new_pos = state.scene.get_position(eid);
                state.log_info(&format!("{}: ({:.3}, {:.3}, {:.3})", name, new_pos.x, new_pos.y, new_pos.z));
            } else if state.active_tool == Tool::Rotate {
                let name = state.scene.get_entity(eid).name.clone();
                state.log_info(&format!("{}: rotation {:.1}\u{00b0}", name, state.drag.rotate_angle.to_degrees()));
            }
            // Auto-key: record transform into motion data
            state.record_auto_key_joint(eid);
        }
        state.drag.active = false;
        state.drag.entity_id = None;
    }
}

// ────────────────────────────────────────────────────────────
// Overlays — floating UI over the 3D viewport
// ────────────────────────────────────────────────────────────

/// Frosted glass background for viewport overlays.
fn overlay_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(Color32::from_rgba_premultiplied(22, 23, 28, 210))
        .rounding(6.0)
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .stroke(Stroke::new(0.5, Color32::from_rgba_premultiplied(70, 75, 90, 100)))
}

/// Camera mode selector overlay (top-left).
pub fn camera_overlay(ui: &mut Ui, state: &mut AppState) {
    overlay_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;

            // Reset button
            if ui.add(
                egui::Button::new(RichText::new("↺").size(12.0).color(accent::MUTED))
                    .fill(Color32::TRANSPARENT)
                    .rounding(3.0)
            ).on_hover_text("Réinitialiser caméra (R)").clicked() {
                state.camera.reset();
            }

            // Divider
            let (_, r) = ui.allocate_space(egui::vec2(1.0, 16.0));
            ui.painter().line_segment(
                [r.center_top(), r.center_bottom()],
                Stroke::new(0.5, accent::BORDER),
            );

            // Camera mode selector
            let cam_mode_label = match state.camera.mode {
                CameraMode::Orbit => "Orbite",
                CameraMode::Free => "Libre",
                CameraMode::ThirdPerson => "3e Personne",
            };
            egui::ComboBox::from_label("")
                .selected_text(RichText::new(cam_mode_label).size(11.0).color(accent::TEXT))
                .width(72.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.camera.mode, CameraMode::Orbit, "🔘 Orbite");
                    ui.selectable_value(&mut state.camera.mode, CameraMode::Free, "🚶 Libre");
                });

            // Divider
            let (_, r) = ui.allocate_space(egui::vec2(1.0, 16.0));
            ui.painter().line_segment(
                [r.center_top(), r.center_bottom()],
                Stroke::new(0.5, accent::BORDER),
            );

            // View preset buttons
            let view_btn = |ui: &mut Ui, label: &str, tooltip: &str| -> bool {
                ui.add(
                    egui::Button::new(RichText::new(label).size(10.0).color(accent::MUTED))
                        .fill(Color32::TRANSPARENT)
                        .rounding(3.0)
                ).on_hover_text(tooltip).clicked()
            };

            if view_btn(ui, "F", "Vue face") { state.camera.view_front(); }
            if view_btn(ui, "R", "Vue droite") { state.camera.view_right(); }
            if view_btn(ui, "T", "Vue dessus") { state.camera.view_top(); }
        });
    });
}

/// Toolbar overlay (top-center) for tool selection.
pub fn toolbar_overlay(ui: &mut Ui, state: &mut AppState) {
    overlay_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            let tools = [
                ("⊙", "Sélection", Tool::Select),
                ("✥", "Déplacer", Tool::Move),
                ("↻", "Rotation", Tool::Rotate),
                ("📏", "Mesure", Tool::Measure),
                ("🦴", "IK", Tool::Ik),
            ];

            for (icon, label, tool) in &tools {
                let selected = state.active_tool == *tool;
                let (text_color, bg) = if selected {
                    (accent::PRIMARY, Color32::from_rgba_premultiplied(75, 135, 255, 30))
                } else {
                    (accent::MUTED, Color32::TRANSPARENT)
                };

                if ui.add(
                    egui::Button::new(
                        RichText::new(format!("{} {}", icon, label)).size(11.0).color(text_color)
                    )
                        .fill(bg)
                        .rounding(4.0)
                        .stroke(if selected {
                            Stroke::new(0.5, accent::PRIMARY_DIM)
                        } else {
                            Stroke::NONE
                        })
                ).clicked() {
                    state.active_tool = *tool;
                }
            }

            // Axis constraint
            if matches!(state.active_tool, Tool::Move | Tool::Rotate) {
                // Vertical divider
                let (_, r) = ui.allocate_space(egui::vec2(1.0, 16.0));
                ui.painter().line_segment(
                    [r.center_top(), r.center_bottom()],
                    Stroke::new(0.5, accent::BORDER),
                );

                let (axis_label, axis_color) = match state.gizmo_axis {
                    GizmoAxis::X => ("X", accent::AXIS_X),
                    GizmoAxis::Y => ("Y", accent::AXIS_Y),
                    GizmoAxis::Z => ("Z", accent::AXIS_Z),
                    GizmoAxis::None => ("Libre", accent::DIM),
                };
                ui.add(
                    egui::Button::new(
                        RichText::new(axis_label).monospace().size(10.5).strong().color(axis_color)
                    )
                        .fill(Color32::from_rgba_premultiplied(axis_color.r(), axis_color.g(), axis_color.b(), 20))
                        .rounding(3.0)
                        .stroke(Stroke::NONE)
                        .sense(egui::Sense::hover())
                );
            }

            // Snap toggle
            if matches!(state.active_tool, Tool::Move) {
                let snap_active = state.snap_to_grid;
                let snap_color = if snap_active { accent::WARNING } else { accent::DIM };
                if ui.add(
                    egui::Button::new(
                        RichText::new("⊞").size(11.0).color(snap_color)
                    )
                        .fill(if snap_active { Color32::from_rgba_premultiplied(255, 185, 50, 20) } else { Color32::TRANSPARENT })
                        .rounding(3.0)
                ).on_hover_text("Accrochage grille").clicked() {
                    state.snap_to_grid = !state.snap_to_grid;
                }
            }

            // Measure distance display
            if state.active_tool == Tool::Measure {
                if let Some(d) = state.measure.distance() {
                    let (_, r) = ui.allocate_space(egui::vec2(1.0, 16.0));
                    ui.painter().line_segment(
                        [r.center_top(), r.center_bottom()],
                        Stroke::new(0.5, accent::BORDER),
                    );
                    ui.label(
                        RichText::new(format!("{:.4}", d))
                            .monospace().size(11.0).color(accent::WARNING)
                    );
                }
            }

            // Auto-key toggle
            let (_, r) = ui.allocate_space(egui::vec2(1.0, 16.0));
            ui.painter().line_segment(
                [r.center_top(), r.center_bottom()],
                Stroke::new(0.5, accent::BORDER),
            );
            let ak_color = if state.auto_key { accent::ERROR } else { accent::DIM };
            let ak_bg = if state.auto_key {
                Color32::from_rgba_premultiplied(255, 70, 70, 30)
            } else {
                Color32::TRANSPARENT
            };
            if ui.add(
                egui::Button::new(RichText::new("● REC").size(10.0).color(ak_color))
                    .fill(ak_bg)
                    .rounding(4.0)
                    .stroke(if state.auto_key {
                        Stroke::new(0.5, accent::ERROR)
                    } else {
                        Stroke::NONE
                    })
            ).on_hover_text("Auto-key: enregistrer les modifications").clicked() {
                state.auto_key = !state.auto_key;
            }

            // Drag indicator
            if state.drag.active {
                ui.label(RichText::new("⟳").size(10.0).color(accent::SUCCESS));
            }
        });
    });
}

/// Application logo/splash in the center of the viewport.
pub fn logo_overlay(ui: &mut Ui, _state: &AppState) {
    let rect = ui.available_rect_before_wrap();
    let center = rect.center();
    let painter = ui.painter();

    let logo_blue = Color32::from_rgb(75, 135, 255);
    let logo_dim = Color32::from_rgb(38, 68, 130);
    let logo_accent = Color32::from_rgb(255, 165, 50);
    let text_dim = Color32::from_rgb(72, 75, 90);

    // ── Subtle background glow ──────────────────────────────
    painter.circle_filled(center, 120.0, Color32::from_rgba_premultiplied(75, 135, 255, 6));
    painter.circle_filled(center, 80.0, Color32::from_rgba_premultiplied(75, 135, 255, 8));

    // ── Stylized bone icon ──────────────────────────────────
    let bone_cx = center.x;
    let bone_cy = center.y - 35.0;
    let bone_len = 42.0;
    let joint_r = 9.0;

    let top = egui::pos2(bone_cx, bone_cy - bone_len);
    let bot = egui::pos2(bone_cx, bone_cy + bone_len);

    // Bone shaft with glow
    painter.line_segment([top, bot], Stroke::new(5.0, Color32::from_rgba_premultiplied(75, 135, 255, 20)));
    painter.line_segment([top, bot], Stroke::new(3.0, logo_dim));

    // Diamond joints
    let diamond = |cx: f32, cy: f32| -> Vec<egui::Pos2> {
        let d = joint_r;
        vec![
            egui::pos2(cx, cy - d),
            egui::pos2(cx + d, cy),
            egui::pos2(cx, cy + d),
            egui::pos2(cx - d, cy),
        ]
    };

    painter.add(egui::Shape::convex_polygon(diamond(top.x, top.y), logo_blue, Stroke::new(1.5, logo_accent)));
    painter.add(egui::Shape::convex_polygon(diamond(bot.x, bot.y), logo_blue, Stroke::new(1.5, logo_accent)));

    // Motion arcs
    for i in 0..3 {
        let offset = (i as f32 - 1.0) * 14.0;
        let arc_x = bone_cx + 22.0 + i as f32 * 6.0;
        let arc_y = bone_cy + offset;
        let alpha = (200 - i * 60) as u8;
        let c = Color32::from_rgba_premultiplied(75, 135, 255, alpha);
        painter.line_segment(
            [egui::pos2(arc_x, arc_y - 7.0), egui::pos2(arc_x + 9.0, arc_y)],
            Stroke::new(2.0, c),
        );
        painter.line_segment(
            [egui::pos2(arc_x + 9.0, arc_y), egui::pos2(arc_x, arc_y + 7.0)],
            Stroke::new(2.0, c),
        );
    }

    // ── Title text ──────────────────────────────────────────
    painter.text(
        egui::pos2(center.x, bone_cy + bone_len + 32.0),
        egui::Align2::CENTER_CENTER,
        "AI4Animation",
        egui::FontId::proportional(34.0),
        logo_blue,
    );

    painter.text(
        egui::pos2(center.x, bone_cy + bone_len + 55.0),
        egui::Align2::CENTER_CENTER,
        "ENGINE",
        egui::FontId::monospace(13.0),
        logo_accent,
    );

    // Subtitle
    painter.text(
        egui::pos2(center.x, bone_cy + bone_len + 78.0),
        egui::Align2::CENTER_CENTER,
        "Rust  ·  wgpu  ·  egui  ·  Deferred Rendering",
        egui::FontId::proportional(10.5),
        text_dim,
    );

    // Drop hint with styled background
    let hint_y = bone_cy + bone_len + 108.0;
    let hint_text = "Glissez un fichier GLB / BVH ici ou utilisez Fichier > Importer";
    let hint_rect = egui::Rect::from_center_size(
        egui::pos2(center.x, hint_y),
        egui::vec2(380.0, 28.0),
    );
    painter.rect_filled(hint_rect, 14.0, Color32::from_rgba_premultiplied(75, 135, 255, 10));
    painter.rect_stroke(hint_rect, 14.0, Stroke::new(0.5, Color32::from_rgba_premultiplied(75, 135, 255, 30)));
    painter.text(
        hint_rect.center(),
        egui::Align2::CENTER_CENTER,
        hint_text,
        egui::FontId::proportional(11.0),
        Color32::from_rgb(95, 100, 120),
    );

    // Version
    painter.text(
        egui::pos2(center.x, hint_y + 28.0),
        egui::Align2::CENTER_CENTER,
        "v0.3.0",
        egui::FontId::monospace(9.5),
        Color32::from_rgb(50, 52, 62),
    );
}

/// 3D orientation compass (bottom-right of viewport).
pub fn compass_overlay(ui: &mut Ui, state: &AppState) {
    let size = 50.0;
    let center = egui::pos2(size * 0.5, size * 0.5);

    let cam_fwd = (state.camera.target - state.camera.position).normalize();
    let cam_right = cam_fwd.cross(state.camera.up).normalize();
    let cam_up = cam_right.cross(cam_fwd).normalize();

    let axes = [
        (Vec3::X, accent::AXIS_X, "X"),
        (Vec3::Y, accent::AXIS_Y, "Y"),
        (Vec3::Z, accent::AXIS_Z, "Z"),
    ];

    let painter = ui.painter();
    let rect = ui.available_rect_before_wrap();
    let origin = egui::pos2(rect.min.x + center.x, rect.min.y + center.y);
    let axis_len = size * 0.38;

    // Background circle with glass effect
    painter.circle_filled(origin, size * 0.46, Color32::from_rgba_premultiplied(18, 19, 24, 200));
    painter.circle_stroke(origin, size * 0.46, Stroke::new(0.5, Color32::from_rgba_premultiplied(60, 65, 80, 150)));

    for (world_dir, color, label) in &axes {
        let screen_x = world_dir.dot(cam_right);
        let screen_y = -world_dir.dot(cam_up);
        let endpoint = egui::pos2(
            origin.x + screen_x * axis_len,
            origin.y + screen_y * axis_len,
        );

        // Axis line with slight glow
        painter.line_segment([origin, endpoint], Stroke::new(2.5, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 40)));
        painter.line_segment([origin, endpoint], Stroke::new(1.5, *color));

        // Label
        painter.text(
            endpoint,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::monospace(9.0),
            *color,
        );
    }

    // Center dot
    painter.circle_filled(origin, 2.0, Color32::from_rgb(120, 125, 140));
}

/// Render stats overlay (bottom-left of viewport).
pub fn stats_overlay(ui: &mut Ui, state: &AppState, viewport_w: u32, viewport_h: u32) {
    overlay_frame()
        .inner_margin(egui::Margin::symmetric(6.0, 3.0))
        .show(ui, |ui| {
            let s = &state.render_settings;
            let model_count = state.loaded_models.len();
            let mesh_count = state.loaded_models.iter().filter(|a| a.skinned_mesh.is_some()).count();
            let joint_count: usize = state.loaded_models.iter().map(|a| a.joint_entity_ids.len()).sum();

            ui.vertical(|ui| {
                // Frame / time info
                let frame_info = if let Some(motion) = state.active_motion() {
                    let frame = (state.timestamp * motion.framerate) as usize;
                    let total = motion.num_frames();
                    format!("Frame {}/{}  ·  {:.2}s/{:.2}s  ·  {}x{}",
                        frame, total, state.timestamp, motion.total_time(),
                        viewport_w, viewport_h)
                } else {
                    format!("{}x{}", viewport_w, viewport_h)
                };
                ui.label(
                    RichText::new(frame_info)
                        .monospace().size(10.0).color(accent::DIM)
                );

                if model_count > 0 {
                    ui.label(
                        RichText::new(format!(
                            "{} model{}  ·  {} mesh  ·  {} joints",
                            model_count,
                            if model_count > 1 { "s" } else { "" },
                            mesh_count,
                            joint_count,
                        )).monospace().size(10.0).color(accent::MUTED)
                    );
                }

                // Active features as small pills
                let mut features = Vec::new();
                if s.shadows_enabled { features.push("Shadows"); }
                if s.ssao_enabled { features.push("SSAO"); }
                if s.bloom_enabled { features.push("Bloom"); }
                if s.fxaa_enabled { features.push("FXAA"); }
                if !features.is_empty() {
                    ui.horizontal(|ui| {
                        for feat in &features {
                            ui.add(
                                egui::Button::new(
                                    RichText::new(*feat).size(9.0).color(accent::DIM)
                                )
                                    .fill(Color32::from_rgba_premultiplied(75, 135, 255, 10))
                                    .rounding(6.0)
                                    .stroke(Stroke::NONE)
                                    .sense(egui::Sense::hover())
                            );
                        }
                    });
                }
            });
        });
}
