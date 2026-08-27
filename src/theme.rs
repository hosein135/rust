//! Color theme for the IDE chrome and editor.

use egui::{Color32, Stroke, Style, Visuals};

pub const BG: Color32 = Color32::from_rgb(18, 20, 24);
pub const PANEL: Color32 = Color32::from_rgb(28, 31, 38);
pub const PANEL_ALT: Color32 = Color32::from_rgb(34, 38, 46);
pub const BORDER: Color32 = Color32::from_rgb(52, 58, 70);
pub const ACCENT: Color32 = Color32::from_rgb(62, 166, 140);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(42, 110, 94);
pub const TEXT: Color32 = Color32::from_rgb(220, 224, 230);
pub const TEXT_DIM: Color32 = Color32::from_rgb(140, 148, 160);
pub const DANGER: Color32 = Color32::from_rgb(220, 90, 90);
pub const WARN: Color32 = Color32::from_rgb(220, 170, 70);
pub const OK: Color32 = Color32::from_rgb(90, 190, 120);

pub const KW: Color32 = Color32::from_rgb(198, 120, 221);
pub const TYPE: Color32 = Color32::from_rgb(97, 175, 239);
pub const NUM: Color32 = Color32::from_rgb(209, 154, 102);
pub const STR: Color32 = Color32::from_rgb(152, 195, 121);
pub const COMMENT: Color32 = Color32::from_rgb(92, 99, 112);
pub const MACRO: Color32 = Color32::from_rgb(86, 182, 194);
pub const IDENT: Color32 = Color32::from_rgb(224, 226, 232);

pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();
    style.visuals = Visuals::dark();
    style.visuals.window_fill = PANEL;
    style.visuals.panel_fill = BG;
    style.visuals.extreme_bg_color = PANEL_ALT;
    style.visuals.faint_bg_color = PANEL;
    style.visuals.widgets.noninteractive.bg_fill = PANEL;
    style.visuals.widgets.inactive.bg_fill = PANEL_ALT;
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(44, 50, 62);
    style.visuals.widgets.active.bg_fill = ACCENT_DIM;
    style.visuals.selection.bg_fill = Color32::from_rgb(45, 90, 80);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    style.visuals.window_stroke = Stroke::new(1.0, BORDER);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(8);
    ctx.set_style(style);
}
