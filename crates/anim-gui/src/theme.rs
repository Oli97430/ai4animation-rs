//! Professional dark theme for the editor — Blender/Unity inspired.
//!
//! Refined color palette, smooth rounding, subtle gradients, and rich accent system.

use egui::{Visuals, Color32, Rounding, Stroke, FontFamily, FontId, TextStyle, Shadow};

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // ── Background Colors ───────────────────────────────────
    // Layered depth: darker = further back
    visuals.window_fill = Color32::from_rgb(32, 33, 38);
    visuals.panel_fill = Color32::from_rgb(27, 28, 33);
    visuals.extreme_bg_color = Color32::from_rgb(17, 18, 22);
    visuals.faint_bg_color = Color32::from_rgb(38, 39, 46);

    // ── Widget Colors ───────────────────────────────────────
    // Noninteractive: labels, disabled elements
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(42, 43, 50);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(38, 39, 46);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(170, 172, 185));
    visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(48, 50, 60));

    // Inactive: buttons, sliders at rest
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(52, 54, 64);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(46, 48, 57);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(195, 198, 212));
    visuals.widgets.inactive.rounding = Rounding::same(5.0);
    visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, Color32::from_rgb(60, 62, 74));

    // Hovered: mouse-over glow
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(62, 66, 82);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(56, 58, 72);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::from_rgb(225, 228, 240));
    visuals.widgets.hovered.rounding = Rounding::same(5.0);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(90, 100, 140));

    // Active: pressed buttons, focused inputs
    visuals.widgets.active.bg_fill = Color32::from_rgb(70, 110, 195);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(60, 95, 170);
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(5.0);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(100, 150, 255));

    // Open: menus, popups
    visuals.widgets.open.bg_fill = Color32::from_rgb(55, 58, 72);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, Color32::from_rgb(210, 213, 228));

    // ── Selection ───────────────────────────────────────────
    visuals.selection.bg_fill = Color32::from_rgb(50, 90, 170);
    visuals.selection.stroke = Stroke::new(1.5, Color32::from_rgb(95, 155, 255));

    // ── Window Chrome ───────────────────────────────────────
    visuals.window_rounding = Rounding::same(8.0);
    visuals.window_shadow = Shadow {
        offset: [0.0, 4.0].into(),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 60),
    };
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(52, 55, 66));

    // ── Misc ────────────────────────────────────────────────
    visuals.popup_shadow = Shadow {
        offset: [0.0, 3.0].into(),
        blur: 10.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 50),
    };
    visuals.resize_corner_size = 10.0;
    visuals.menu_rounding = Rounding::same(6.0);
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    visuals.striped = true;
    visuals.slider_trailing_fill = true;

    // Hyperlink color
    visuals.hyperlink_color = Color32::from_rgb(100, 160, 255);

    ctx.set_visuals(visuals);

    // ── Typography ──────────────────────────────────────────
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(12.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(12.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(10.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(11.5, FontFamily::Monospace));

    // ── Spacing ─────────────────────────────────────────────
    style.spacing.item_spacing = [6.0, 3.5].into();
    style.spacing.button_padding = [10.0, 4.0].into();
    style.spacing.indent = 18.0;
    style.spacing.icon_width = 14.0;
    style.spacing.icon_width_inner = 10.0;
    style.spacing.icon_spacing = 6.0;
    style.spacing.combo_width = 100.0;
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.floating = true;

    ctx.set_style(style);
}

// ═══════════════════════════════════════════════════════════
// Accent color system
// ═══════════════════════════════════════════════════════════

pub mod accent {
    use egui::Color32;

    // ── Brand / Primary ─────────────────────────────────────
    pub const PRIMARY:     Color32 = Color32::from_rgb(75, 135, 255);
    pub const PRIMARY_DIM: Color32 = Color32::from_rgb(50, 90, 175);

    // ── Status ──────────────────────────────────────────────
    pub const SUCCESS:     Color32 = Color32::from_rgb(72, 199, 120);
    pub const WARNING:     Color32 = Color32::from_rgb(255, 185, 50);
    pub const ERROR:       Color32 = Color32::from_rgb(245, 75, 75);

    // ── Scene / 3D ──────────────────────────────────────────
    pub const BONE:        Color32 = Color32::from_rgb(45, 175, 255);
    pub const JOINT:       Color32 = Color32::from_rgb(255, 205, 55);
    pub const SELECTED:    Color32 = Color32::from_rgb(255, 165, 0);
    pub const IK_CHAIN:    Color32 = Color32::from_rgb(200, 80, 255);

    // ── Axes ────────────────────────────────────────────────
    pub const AXIS_X:      Color32 = Color32::from_rgb(240, 68, 68);
    pub const AXIS_Y:      Color32 = Color32::from_rgb(68, 220, 90);
    pub const AXIS_Z:      Color32 = Color32::from_rgb(68, 120, 255);

    // ── Playback ────────────────────────────────────────────
    pub const PLAY:        Color32 = Color32::from_rgb(72, 199, 120);
    pub const PAUSE:       Color32 = Color32::from_rgb(245, 105, 75);

    // ── UI Chrome ───────────────────────────────────────────
    pub const HEADER_BG:   Color32 = Color32::from_rgb(35, 37, 45);
    pub const SECTION_BG:  Color32 = Color32::from_rgb(30, 32, 40);
    pub const BORDER:      Color32 = Color32::from_rgb(50, 52, 64);
    pub const MUTED:       Color32 = Color32::from_rgb(105, 108, 125);
    pub const DIM:         Color32 = Color32::from_rgb(75, 78, 92);
    pub const TEXT:        Color32 = Color32::from_rgb(200, 203, 218);
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(235, 237, 248);
}

// ═══════════════════════════════════════════════════════════
// Reusable UI helpers
// ═══════════════════════════════════════════════════════════

/// Draw a styled section header with icon and colored left-border accent.
pub fn section_header(ui: &mut egui::Ui, icon: &str, label: &str, color: Color32) {
    ui.add_space(6.0);
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 22.0));
    let painter = ui.painter();

    // Left accent bar
    let bar_rect = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(3.0, rect.height()),
    );
    painter.rect_filled(bar_rect, 1.0, color);

    // Background
    let bg_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 3.0, rect.min.y),
        rect.max,
    );
    painter.rect_filled(bg_rect, egui::Rounding { nw: 0.0, ne: 4.0, sw: 0.0, se: 4.0 }, accent::HEADER_BG);

    // Icon + text
    painter.text(
        egui::pos2(rect.min.x + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(12.0),
        color,
    );
    painter.text(
        egui::pos2(rect.min.x + 28.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(11.5),
        accent::TEXT_BRIGHT,
    );
    ui.add_space(4.0);
}

/// Draw a subtle separator line.
pub fn thin_separator(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(0.5, accent::BORDER),
    );
    ui.add_space(2.0);
}

/// Small pill badge (e.g. "24 joints", "mesh").
/// Uses a non-interactive Button to avoid layout expansion issues.
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(10.0).color(color))
            .fill(Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 25))
            .rounding(10.0)
            .stroke(Stroke::new(0.5, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 50)))
            .sense(egui::Sense::hover())
    );
}

/// Styled icon button (fixed size, optional highlight color).
pub fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, highlight: Option<Color32>) -> bool {
    let text = match highlight {
        Some(c) => egui::RichText::new(icon).color(c),
        None => egui::RichText::new(icon),
    };
    let btn = ui.add_sized(
        egui::vec2(26.0, 22.0),
        egui::Button::new(text).rounding(4.0),
    );
    btn.on_hover_text(tooltip).clicked()
}
