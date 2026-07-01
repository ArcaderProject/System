use egui::{Color32, FontFamily, FontId, Pos2, Rect, Rounding, Stroke, TextStyle, Vec2};

use crate::disks::DiskKind;

pub const SQUARE: Rounding = Rounding::ZERO;

pub const ACCENT: Color32 = Color32::from_rgb(0xE0, 0x28, 0x22);
pub const ACCENT_HI: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x30);
pub const AMBER: Color32 = Color32::from_rgb(0xFF, 0xB0, 0x00);
pub const DANGER: Color32 = ACCENT;
pub const PANEL: Color32 = Color32::from_rgb(0x16, 0x0E, 0x0D);
pub const CARD: Color32 = Color32::from_rgb(0x20, 0x15, 0x14);
pub const CARD_SELECTED: Color32 = Color32::from_rgb(0x3A, 0x12, 0x10);
pub const MUTED: Color32 = Color32::from_rgb(0xB0, 0x9A, 0x94);
pub const TEXT: Color32 = Color32::from_rgb(0xF2, 0xE7, 0xDF);

pub fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.push("phosphor".to_owned());
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    use FontFamily::Monospace as M;
    style.text_styles = [
        (TextStyle::Heading, FontId::new(26.0, M)),
        (TextStyle::Body, FontId::new(16.0, M)),
        (TextStyle::Monospace, FontId::new(14.0, M)),
        (TextStyle::Button, FontId::new(16.0, M)),
        (TextStyle::Small, FontId::new(12.0, M)),
    ]
    .into();

    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT_HI);
    style.visuals.hyperlink_color = ACCENT_HI;

    style.visuals.window_rounding = SQUARE;
    style.visuals.menu_rounding = SQUARE;
    for w in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        w.rounding = SQUARE;
    }
    let base_stroke = style.visuals.widgets.inactive.bg_stroke;
    style.visuals.widgets.hovered.bg_stroke = base_stroke;
    style.visuals.widgets.active.bg_stroke = base_stroke;

    style.spacing.item_spacing = Vec2::new(12.0, 12.0);
    style.spacing.button_padding = Vec2::new(20.0, 12.0);
    ctx.set_style(style);
}

pub fn icon_for(kind: DiskKind) -> &'static str {
    use egui_phosphor::regular;
    match kind {
        DiskKind::Usb => regular::USB,
        DiskKind::Ssd => regular::HARD_DRIVE,
        DiskKind::Hdd => regular::HARD_DRIVES,
    }
}

pub fn draw_scanlines(painter: &egui::Painter, rect: Rect) {
    let line = Color32::from_rgba_premultiplied(0, 0, 0, 46);
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(rect.left(), y), Pos2::new(rect.right(), y + 1.0)),
            SQUARE,
            line,
        );
        y += 3.0;
    }
}
