//! Flash-style timeline panel — Macromedia Flash inspired keyframe editor.
//!
//! Visual layout with layers, keyframe cells (● filled / ○ empty), tween fills,
//! red playhead, visibility/lock toggles, and frame number rulers.

use egui::{Ui, Color32, RichText, Rect, Stroke, Rounding, Pos2, Vec2 as EVec2, Sense};
use crate::app_state::AppState;
use crate::theme::accent;

// ── Flash color palette ─────────────────────────────────────
const FLASH_BG: Color32 = Color32::from_rgb(22, 24, 30);         // Dark timeline bg
const FLASH_CELL_BG: Color32 = Color32::from_rgb(35, 37, 45);    // Normal cell
const FLASH_CELL_EVEN: Color32 = Color32::from_rgb(30, 32, 40);  // Alternating row
const FLASH_RULER_BG: Color32 = Color32::from_rgb(42, 44, 52);   // Ruler background
const FLASH_LAYER_BG: Color32 = Color32::from_rgb(28, 30, 38);   // Layer panel bg
const FLASH_KEYFRAME: Color32 = Color32::from_rgb(255, 215, 0);  // Gold keyframe ●
const FLASH_EMPTY: Color32 = Color32::from_rgb(80, 82, 95);      // Empty frame ○
const FLASH_TWEEN: Color32 = Color32::from_rgb(80, 130, 220);    // Tween fill (blue arrow)
const FLASH_PLAYHEAD: Color32 = Color32::from_rgb(255, 50, 50);  // Red playhead
const FLASH_SELECTION: Color32 = Color32::from_rgb(60, 100, 200); // Selected cell highlight
const FLASH_FIVE_MARK: Color32 = Color32::from_rgb(55, 58, 70);  // Every-5th frame line
const FLASH_LAYER_SEL: Color32 = Color32::from_rgb(45, 65, 110); // Selected layer bg

const LAYER_COL_WIDTH: f32 = 140.0;
const DEFAULT_FRAME_W: f32 = 14.0;
const DEFAULT_LAYER_H: f32 = 22.0;
const RULER_HEIGHT: f32 = 20.0;
const MIN_FRAME_W: f32 = 8.0;
const MAX_FRAME_W: f32 = 28.0;

/// Per-layer state in the Flash timeline.
#[derive(Clone)]
pub struct FlashLayer {
    pub name: String,
    pub joint_index: usize,
    pub visible: bool,
    pub locked: bool,
    /// Frames that have keyframes (sparse set).
    pub keyframes: Vec<usize>,
    /// Tween type: 0=none, 1=motion, 2=shape
    pub tween_type: u8,
}

/// State for the Flash-style timeline panel.
pub struct FlashTimelinePanel {
    pub visible: bool,
    pub frame_width: f32,
    pub layer_height: f32,
    pub layers: Vec<FlashLayer>,
    pub selected_layer: Option<usize>,
    pub selected_frame: Option<usize>,
    pub dragging_playhead: bool,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub show_toolbar: bool,
    /// Track which model these layers belong to
    pub model_index: Option<usize>,
}

impl Default for FlashTimelinePanel {
    fn default() -> Self {
        Self {
            visible: false,
            frame_width: DEFAULT_FRAME_W,
            layer_height: DEFAULT_LAYER_H,
            layers: Vec::new(),
            selected_layer: None,
            selected_frame: None,
            dragging_playhead: false,
            scroll_x: 0.0,
            scroll_y: 0.0,
            show_toolbar: true,
            model_index: None,
        }
    }
}

impl FlashTimelinePanel {
    pub fn new() -> Self { Self::default() }

    /// Rebuild layers from the active model's joint names.
    pub fn sync_with_model(&mut self, model_index: usize, joint_names: &[String]) {
        if self.model_index == Some(model_index) && self.layers.len() == joint_names.len() {
            return; // Already synced
        }
        self.model_index = Some(model_index);
        self.layers = joint_names.iter().enumerate().map(|(i, name)| {
            FlashLayer {
                name: name.clone(),
                joint_index: i,
                visible: true,
                locked: false,
                keyframes: vec![0], // First frame is always a keyframe
                tween_type: 1, // motion tween by default
            }
        }).collect();
        self.selected_layer = if self.layers.is_empty() { None } else { Some(0) };
    }

    /// Insert a keyframe on the selected layer at the given frame.
    pub fn insert_keyframe(&mut self, layer: usize, frame: usize) {
        if let Some(l) = self.layers.get_mut(layer) {
            if !l.keyframes.contains(&frame) {
                l.keyframes.push(frame);
                l.keyframes.sort_unstable();
            }
        }
    }

    /// Remove a keyframe from a layer.
    pub fn remove_keyframe(&mut self, layer: usize, frame: usize) {
        if let Some(l) = self.layers.get_mut(layer) {
            l.keyframes.retain(|&f| f != frame);
        }
    }

    /// Check if a frame has a keyframe on a layer.
    pub fn has_keyframe(&self, layer: usize, frame: usize) -> bool {
        self.layers.get(layer).map_or(false, |l| l.keyframes.contains(&frame))
    }

    /// Get tween range: if frame is between two keyframes with tween enabled
    fn is_in_tween(&self, layer: usize, frame: usize) -> bool {
        if let Some(l) = self.layers.get(layer) {
            if l.tween_type == 0 { return false; }
            // Check if frame is between two keyframes
            let mut prev = None;
            let mut next = None;
            for &kf in &l.keyframes {
                if kf <= frame { prev = Some(kf); }
                if kf > frame && next.is_none() { next = Some(kf); }
            }
            prev.is_some() && next.is_some()
        } else {
            false
        }
    }
}

/// Actions deferred from the draw loop.
enum TimelineAction {
    SetFrame(usize),
    SelectLayer(usize),
    TogglePlay,
    Stop,
    InsertKeyframe,
    RemoveKeyframe,
    SetTweenType(usize, u8),
    ToggleLayerVisible(usize),
    ToggleLayerLocked(usize),
}

/// Main show function for the Flash timeline panel.
pub fn show(ui: &mut Ui, panel: &mut FlashTimelinePanel, state: &mut AppState) {
    if !panel.visible { return; }

    let total_frames = state.total_frames();
    if total_frames == 0 {
        ui.label(RichText::new("Aucune animation chargee").color(accent::MUTED).size(11.0));
        return;
    }

    // Sync layers with current model
    if let Some(idx) = state.active_model {
        let names = state.loaded_models[idx].model.joint_names.clone();
        panel.sync_with_model(idx, &names);
    }

    let current_frame = state.current_frame();
    let mut action: Option<TimelineAction> = None;

    // ── Toolbar ─────────────────────────────────────────
    if panel.show_toolbar {
        ui.horizontal(|ui| {
            // Transport controls
            let play_icon = if state.playing { "\u{23F8}" } else { "\u{25B6}" };
            if ui.button(RichText::new(play_icon).size(14.0).color(if state.playing { accent::PAUSE } else { accent::PLAY })).clicked() {
                action = Some(TimelineAction::TogglePlay);
            }
            if ui.button(RichText::new("\u{23F9}").size(14.0)).clicked() {
                action = Some(TimelineAction::Stop);
            }

            ui.separator();

            // Frame display
            ui.label(RichText::new(format!("Frame: {}", current_frame)).monospace().size(11.0).color(accent::TEXT_BRIGHT));
            ui.label(RichText::new(format!("/ {}", total_frames)).monospace().size(10.0).color(accent::MUTED));

            ui.separator();

            // FPS display
            if let Some(idx) = state.active_model {
                if let Some(ref motion) = state.loaded_models[idx].motion {
                    ui.label(RichText::new(format!("{}fps", motion.framerate as u32)).size(10.0).color(accent::DIM));
                }
            }

            ui.separator();

            // Keyframe buttons
            if ui.button(RichText::new("\u{1F511} F6").size(10.0).color(FLASH_KEYFRAME))
                .on_hover_text("Inserer keyframe (F6)")
                .clicked() {
                action = Some(TimelineAction::InsertKeyframe);
            }
            if ui.button(RichText::new("\u{2715} F7").size(10.0).color(accent::MUTED))
                .on_hover_text("Supprimer keyframe (F7)")
                .clicked() {
                action = Some(TimelineAction::RemoveKeyframe);
            }

            ui.separator();

            // Tween dropdown
            let tween_label = match panel.selected_layer.and_then(|l| panel.layers.get(l)) {
                Some(l) => match l.tween_type {
                    0 => "Aucun",
                    1 => "Motion",
                    2 => "Shape",
                    _ => "?",
                },
                None => "\u{2014}",
            };
            egui::ComboBox::from_label(RichText::new("Tween").size(10.0).color(accent::MUTED))
                .selected_text(RichText::new(tween_label).size(10.0))
                .width(70.0)
                .show_ui(ui, |ui| {
                    if let Some(sel) = panel.selected_layer {
                        if ui.selectable_label(tween_label == "Aucun", "Aucun").clicked() {
                            action = Some(TimelineAction::SetTweenType(sel, 0));
                        }
                        if ui.selectable_label(tween_label == "Motion", "Motion").clicked() {
                            action = Some(TimelineAction::SetTweenType(sel, 1));
                        }
                        if ui.selectable_label(tween_label == "Shape", "Shape").clicked() {
                            action = Some(TimelineAction::SetTweenType(sel, 2));
                        }
                    }
                });

            ui.separator();

            // Zoom
            ui.label(RichText::new("\u{1F50D}").size(10.0));
            ui.add(egui::Slider::new(&mut panel.frame_width, MIN_FRAME_W..=MAX_FRAME_W)
                .show_value(false)
                .step_by(1.0_f64));
        });
    }

    ui.add_space(1.0);

    // ── Main timeline area ──────────────────────────────
    let available = ui.available_size();
    let grid_area_width = (available.x - LAYER_COL_WIDTH).max(100.0);
    let visible_frames = (grid_area_width / panel.frame_width) as usize;

    // Use a scroll area for the whole timeline
    egui::ScrollArea::vertical()
        .max_height(available.y)
        .auto_shrink([false, false])
        .show(ui, |ui| {

            // ── Ruler (frame numbers) ────────────────────
            ui.horizontal(|ui| {
                // Empty space above layer names
                ui.add_space(LAYER_COL_WIDTH);

                let (ruler_rect, ruler_response) = ui.allocate_exact_size(
                    EVec2::new(grid_area_width, RULER_HEIGHT),
                    Sense::click_and_drag(),
                );

                let painter = ui.painter_at(ruler_rect);
                painter.rect_filled(ruler_rect, Rounding::ZERO, FLASH_RULER_BG);

                // Draw frame numbers and tick marks
                let start_frame = (panel.scroll_x / panel.frame_width) as usize;
                let end_frame = (start_frame + visible_frames + 2).min(total_frames);

                for f in start_frame..end_frame {
                    let x = ruler_rect.min.x + (f as f32 - panel.scroll_x / panel.frame_width) * panel.frame_width;
                    if x < ruler_rect.min.x || x > ruler_rect.max.x { continue; }

                    // Every 5th frame: number label
                    if f % 5 == 0 {
                        painter.text(
                            Pos2::new(x + panel.frame_width * 0.5, ruler_rect.center().y),
                            egui::Align2::CENTER_CENTER,
                            format!("{}", f),
                            egui::FontId::monospace(9.0),
                            accent::MUTED,
                        );
                        // Tick mark
                        painter.line_segment(
                            [Pos2::new(x, ruler_rect.max.y - 4.0), Pos2::new(x, ruler_rect.max.y)],
                            Stroke::new(1.0, accent::DIM),
                        );
                    }
                }

                // Playhead indicator on ruler (red triangle)
                let playhead_x = ruler_rect.min.x + (current_frame as f32 - panel.scroll_x / panel.frame_width) * panel.frame_width + panel.frame_width * 0.5;
                if playhead_x >= ruler_rect.min.x && playhead_x <= ruler_rect.max.x {
                    let tri_size = 5.0;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            Pos2::new(playhead_x - tri_size, ruler_rect.max.y - tri_size * 2.0),
                            Pos2::new(playhead_x + tri_size, ruler_rect.max.y - tri_size * 2.0),
                            Pos2::new(playhead_x, ruler_rect.max.y),
                        ],
                        FLASH_PLAYHEAD,
                        Stroke::NONE,
                    ));
                }

                // Click on ruler to set frame
                if ruler_response.clicked() || (ruler_response.dragged() && panel.dragging_playhead) {
                    if let Some(pos) = ruler_response.interact_pointer_pos() {
                        let rel_x = pos.x - ruler_rect.min.x + panel.scroll_x;
                        let frame = (rel_x / panel.frame_width).floor() as usize;
                        let frame = frame.min(total_frames.saturating_sub(1));
                        action = Some(TimelineAction::SetFrame(frame));
                    }
                    panel.dragging_playhead = true;
                }
                if ruler_response.drag_stopped() {
                    panel.dragging_playhead = false;
                }
            });

            // ── Layer rows ────────────────────────────────
            for layer_idx in 0..panel.layers.len() {
                let layer = &panel.layers[layer_idx];
                let is_selected = panel.selected_layer == Some(layer_idx);
                let row_bg = if is_selected {
                    FLASH_LAYER_SEL
                } else if layer_idx % 2 == 0 {
                    FLASH_CELL_BG
                } else {
                    FLASH_CELL_EVEN
                };

                ui.horizontal(|ui| {
                    // ── Layer name panel ─────────────────
                    let (name_rect, name_response) = ui.allocate_exact_size(
                        EVec2::new(LAYER_COL_WIDTH, panel.layer_height),
                        Sense::click(),
                    );

                    let painter = ui.painter_at(name_rect);
                    painter.rect_filled(name_rect, Rounding::ZERO, if is_selected { FLASH_LAYER_SEL } else { FLASH_LAYER_BG });

                    // Visibility eye icon
                    let eye_rect = Rect::from_min_size(
                        Pos2::new(name_rect.min.x + 4.0, name_rect.min.y + 2.0),
                        EVec2::new(16.0, panel.layer_height - 4.0),
                    );
                    let eye_icon = if layer.visible { "\u{1F441}" } else { "\u{00B7}" };
                    painter.text(
                        eye_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        eye_icon,
                        egui::FontId::proportional(10.0),
                        if layer.visible { accent::TEXT } else { accent::DIM },
                    );

                    // Lock icon
                    let lock_rect = Rect::from_min_size(
                        Pos2::new(name_rect.min.x + 22.0, name_rect.min.y + 2.0),
                        EVec2::new(16.0, panel.layer_height - 4.0),
                    );
                    let lock_icon = if layer.locked { "\u{1F512}" } else { "\u{00B7}" };
                    painter.text(
                        lock_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        lock_icon,
                        egui::FontId::proportional(10.0),
                        if layer.locked { accent::WARNING } else { accent::DIM },
                    );

                    // Layer name (truncated)
                    let name_display = if layer.name.len() > 12 {
                        format!("{}\u{2026}", &layer.name[..11])
                    } else {
                        layer.name.clone()
                    };
                    painter.text(
                        Pos2::new(name_rect.min.x + 42.0, name_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &name_display,
                        egui::FontId::proportional(10.5),
                        if is_selected { accent::TEXT_BRIGHT } else { accent::TEXT },
                    );

                    // Right border
                    painter.line_segment(
                        [Pos2::new(name_rect.max.x, name_rect.min.y), Pos2::new(name_rect.max.x, name_rect.max.y)],
                        Stroke::new(1.0, accent::BORDER),
                    );

                    if name_response.clicked() {
                        action = Some(TimelineAction::SelectLayer(layer_idx));
                        // Check sub-areas for visibility/lock toggles
                        if let Some(pos) = name_response.interact_pointer_pos() {
                            if eye_rect.contains(pos) {
                                action = Some(TimelineAction::ToggleLayerVisible(layer_idx));
                            } else if lock_rect.contains(pos) {
                                action = Some(TimelineAction::ToggleLayerLocked(layer_idx));
                            }
                        }
                    }

                    // ── Frame cells grid ────────────────
                    let (grid_rect, grid_response) = ui.allocate_exact_size(
                        EVec2::new(grid_area_width, panel.layer_height),
                        Sense::click(),
                    );

                    let painter = ui.painter_at(grid_rect);
                    painter.rect_filled(grid_rect, Rounding::ZERO, row_bg);

                    // Draw each visible frame cell
                    let start_frame = (panel.scroll_x / panel.frame_width) as usize;
                    let end_frame = (start_frame + visible_frames + 2).min(total_frames);

                    for f in start_frame..end_frame {
                        let cell_x = grid_rect.min.x + (f as f32 - panel.scroll_x / panel.frame_width) * panel.frame_width;
                        if cell_x + panel.frame_width < grid_rect.min.x || cell_x > grid_rect.max.x { continue; }

                        let cell_rect = Rect::from_min_size(
                            Pos2::new(cell_x, grid_rect.min.y),
                            EVec2::new(panel.frame_width, panel.layer_height),
                        );

                        // Every-5th frame vertical line
                        if f % 5 == 0 {
                            painter.line_segment(
                                [Pos2::new(cell_x, grid_rect.min.y), Pos2::new(cell_x, grid_rect.max.y)],
                                Stroke::new(0.5, FLASH_FIVE_MARK),
                            );
                        }

                        // Selection highlight
                        if panel.selected_frame == Some(f) && is_selected {
                            painter.rect_filled(cell_rect, Rounding::ZERO, FLASH_SELECTION);
                        }

                        // Draw keyframe dot or tween fill
                        let center = cell_rect.center();
                        let has_kf = layer.keyframes.contains(&f);

                        if has_kf {
                            // Gold filled circle (keyframe)
                            painter.circle_filled(center, 4.0, FLASH_KEYFRAME);
                            painter.circle_stroke(center, 4.0, Stroke::new(0.5, Color32::from_rgb(180, 150, 0)));
                        } else if panel.is_in_tween(layer_idx, f) {
                            // Tween fill: blue-tinted cell with arrow hint
                            let tween_bg = Color32::from_rgba_premultiplied(80, 130, 220, 40);
                            painter.rect_filled(cell_rect, Rounding::ZERO, tween_bg);

                            // Small dot to show continuity
                            painter.circle_filled(center, 1.5, FLASH_TWEEN);
                        } else {
                            // Empty frame (small gray dot)
                            painter.circle_filled(center, 1.5, FLASH_EMPTY);
                        }
                    }

                    // Bottom border for the row
                    painter.line_segment(
                        [Pos2::new(grid_rect.min.x, grid_rect.max.y), Pos2::new(grid_rect.max.x, grid_rect.max.y)],
                        Stroke::new(0.5, Color32::from_rgb(40, 42, 52)),
                    );

                    // Playhead vertical line across all rows
                    let playhead_x = grid_rect.min.x + (current_frame as f32 - panel.scroll_x / panel.frame_width) * panel.frame_width + panel.frame_width * 0.5;
                    if playhead_x >= grid_rect.min.x && playhead_x <= grid_rect.max.x {
                        painter.line_segment(
                            [Pos2::new(playhead_x, grid_rect.min.y), Pos2::new(playhead_x, grid_rect.max.y)],
                            Stroke::new(2.0, FLASH_PLAYHEAD),
                        );
                    }

                    // Click on grid to select frame
                    if grid_response.clicked() {
                        if let Some(pos) = grid_response.interact_pointer_pos() {
                            let rel_x = pos.x - grid_rect.min.x + panel.scroll_x;
                            let frame = (rel_x / panel.frame_width).floor() as usize;
                            let frame = frame.min(total_frames.saturating_sub(1));
                            action = Some(TimelineAction::SetFrame(frame));
                        }
                    }
                });
            }

            // ── Horizontal scrollbar hint ────────────────
            let max_scroll = (total_frames as f32 * panel.frame_width - grid_area_width).max(0.0);
            if max_scroll > 0.0 {
                ui.add(egui::Slider::new(&mut panel.scroll_x, 0.0..=max_scroll)
                    .show_value(false)
                    .text(RichText::new("Scroll").size(9.0).color(accent::DIM)));
            }
        });

    // ── Process deferred actions ─────────────────────────
    match action {
        Some(TimelineAction::SetFrame(f)) => {
            state.timestamp = if let Some(idx) = state.active_model {
                if let Some(ref motion) = state.loaded_models[idx].motion {
                    f as f32 / motion.framerate
                } else { 0.0 }
            } else { 0.0 };
            panel.selected_frame = Some(f);
        }
        Some(TimelineAction::SelectLayer(l)) => {
            panel.selected_layer = Some(l);
        }
        Some(TimelineAction::TogglePlay) => {
            state.playing = !state.playing;
        }
        Some(TimelineAction::Stop) => {
            state.playing = false;
            state.timestamp = 0.0;
        }
        Some(TimelineAction::InsertKeyframe) => {
            if let Some(l) = panel.selected_layer {
                panel.insert_keyframe(l, current_frame);
            }
        }
        Some(TimelineAction::RemoveKeyframe) => {
            if let Some(l) = panel.selected_layer {
                panel.remove_keyframe(l, current_frame);
            }
        }
        Some(TimelineAction::SetTweenType(l, t)) => {
            if let Some(layer) = panel.layers.get_mut(l) {
                layer.tween_type = t;
            }
        }
        Some(TimelineAction::ToggleLayerVisible(l)) => {
            if let Some(layer) = panel.layers.get_mut(l) {
                layer.visible = !layer.visible;
            }
        }
        Some(TimelineAction::ToggleLayerLocked(l)) => {
            if let Some(layer) = panel.layers.get_mut(l) {
                layer.locked = !layer.locked;
            }
        }
        None => {}
    }
}
