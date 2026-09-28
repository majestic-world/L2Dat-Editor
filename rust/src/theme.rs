use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, TextStyle};

pub const BG: Color32 = Color32::from_rgb(0x14, 0x17, 0x1e);
pub const SURFACE: Color32 = Color32::from_rgb(0x1b, 0x20, 0x28);
pub const RAISED: Color32 = Color32::from_rgb(0x20, 0x25, 0x2e);
pub const BORDER: Color32 = Color32::from_rgb(0x34, 0x39, 0x42);
pub const TEXT: Color32 = Color32::from_rgb(0xdd, 0xdf, 0xe5);
pub const MUTED: Color32 = Color32::from_rgb(0x93, 0x9b, 0xaa);
pub const ACCENT: Color32 = Color32::from_rgb(0xde, 0xa7, 0x7b);
pub const ERROR: Color32 = Color32::from_rgb(0xe0, 0x8d, 0x8d);
pub const SUCCESS: Color32 = Color32::from_rgb(0xa5, 0xc8, 0x90);

pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(13.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(11.0));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(18.0));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(13.0));
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 5.0);
    style.spacing.interact_size = egui::vec2(24.0, 28.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.icon_width = 16.0;
    style.spacing.icon_width_inner = 10.0;
    style.spacing.icon_spacing = 8.0;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = SURFACE;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = BG;
    visuals.code_bg_color = BG;
    visuals.faint_bg_color = RAISED;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_corner_radius = CornerRadius::same(4);
    visuals.menu_corner_radius = CornerRadius::same(4);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = ACCENT;
    visuals.error_fg_color = ERROR;
    visuals.selection.bg_fill = Color32::from_rgb(0x50, 0x41, 0x37);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.text_cursor.stroke = Stroke::new(1.5_f32, ACCENT);
    visuals.indent_has_left_vline = false;
    visuals.striped = false;

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(4);
        widget.expansion = 0.0;
        widget.fg_stroke = Stroke::new(1.0_f32, TEXT);
    }

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.bg_fill = RAISED;
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x2b, 0x31, 0x3b);
    visuals.widgets.hovered.weak_bg_fill = visuals.widgets.hovered.bg_fill;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, MUTED);
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x3e, 0x34, 0x2e);
    visuals.widgets.active.weak_bg_fill = visuals.widgets.active.bg_fill;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.open.bg_fill = RAISED;
    visuals.widgets.open.weak_bg_fill = RAISED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, ACCENT);

    style.visuals = visuals;
    ctx.set_style(style);
}
