use egui::{
    Color32, FontData, FontDefinitions, FontFamily, FontId, Painter, Pos2, Stroke, Visuals,
};
use std::sync::Arc;

pub const WIN_W: f32 = 1600.0;
pub const WIN_H: f32 = 1027.0;

pub const TXT: Color32 = Color32::from_rgb(232, 229, 234);
pub const TXT_DIM: Color32 = Color32::from_rgb(164, 158, 168);
pub const TXT_FAINT: Color32 = Color32::from_rgb(112, 106, 116);
pub const MAGENTA: Color32 = Color32::from_rgb(236, 22, 199);
pub const MAGENTA_SOFT: Color32 = Color32::from_rgb(156, 32, 130);
pub const FRAME: Color32 = Color32::from_rgb(150, 24, 120);
pub const FRAME_HI: Color32 = Color32::from_rgb(205, 58, 170);
pub const PURPLE: Color32 = Color32::from_rgb(142, 27, 216);
pub const GREEN: Color32 = Color32::from_rgb(122, 214, 96);
pub const AMBER: Color32 = Color32::from_rgb(244, 181, 61);
pub const RED: Color32 = Color32::from_rgb(255, 84, 112);
pub const PANEL: Color32 = Color32::from_rgb(15, 13, 17);

pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn sans(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Aldrich".into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    if let Some(dir) = crate::assets::font_dir() {
        let mut add = |key: &str, file: &str| {
            if let Ok(bytes) = std::fs::read(dir.join(file)) {
                fonts
                    .font_data
                    .insert(key.to_owned(), Arc::new(FontData::from_owned(bytes)));
            }
        };
        add("LiberationSans", "LiberationSans-Regular.ttf");
        add("NotoSans", "NotoSans-Regular.ttf");
        add("NotoSansBold", "NotoSans-Bold.ttf");
        add("Aldrich", "Aldrich-Regular.ttf");
    }

    let proportional = fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default();
    if fonts.font_data.contains_key("NotoSans") {
        proportional.retain(|name| name != "NotoSans");
        proportional.insert(0, "NotoSans".to_owned());
    }
    if fonts.font_data.contains_key("LiberationSans") {
        proportional.retain(|name| name != "LiberationSans");
        proportional.insert(0, "LiberationSans".to_owned());
    }
    if fonts.font_data.contains_key("NotoSansBold") {
        proportional.push("NotoSansBold".to_owned());
    }

    if fonts.font_data.contains_key("Aldrich") {
        let family = fonts
            .families
            .entry(FontFamily::Name("Aldrich".into()))
            .or_default();
        family.push("Aldrich".to_owned());
        family.push("NotoSans".to_owned());
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TXT);
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(9, 8, 11);
    visuals.faint_bg_color = Color32::from_rgb(19, 17, 22);
    visuals.selection.bg_fill = alpha(MAGENTA, 90);
    visuals.selection.stroke = Stroke::new(1.0, MAGENTA);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TXT_DIM);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TXT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, MAGENTA);
    visuals.slider_trailing_fill = true;
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.text_styles = [
        (egui::TextStyle::Body, sans(16.0)),
        (egui::TextStyle::Button, sans(16.0)),
        (egui::TextStyle::Small, sans(13.0)),
        (egui::TextStyle::Heading, display(30.0)),
    ]
    .into();
    ctx.set_style_of(egui::Theme::Dark, style);
    ctx.set_theme(egui::ThemePreference::Dark);
}

/// Draw text with manual letter spacing, centered on `center`.
pub fn tracked_center(
    painter: &Painter,
    center: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    tracking: f32,
) -> f32 {
    let widths: Vec<f32> = text
        .chars()
        .map(|ch| {
            painter
                .layout_no_wrap(ch.to_string(), font.clone(), color)
                .size()
                .x
        })
        .collect();
    let total = widths.iter().sum::<f32>() + tracking * text.chars().count().saturating_sub(1) as f32;
    let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), color);
    let height = galley.size().y;
    let mut x = center.x - total * 0.5;
    let y = center.y - height * 0.5;
    for (ch, width) in text.chars().zip(widths) {
        let g = painter.layout_no_wrap(ch.to_string(), font.clone(), color);
        painter.galley(Pos2::new(x, y), g, color);
        x += width + tracking;
    }
    total
}

pub fn tracked_left(
    painter: &Painter,
    pos: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    tracking: f32,
) -> f32 {
    let widths: Vec<f32> = text
        .chars()
        .map(|ch| {
            painter
                .layout_no_wrap(ch.to_string(), font.clone(), color)
                .size()
                .x
        })
        .collect();
    let mut x = pos.x;
    for (ch, width) in text.chars().zip(widths) {
        let g = painter.layout_no_wrap(ch.to_string(), font.clone(), color);
        painter.galley(Pos2::new(x, pos.y), g, color);
        x += width + tracking;
    }
    x - pos.x
}

pub fn glow_circle(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    for i in 0..4 {
        let r = radius * (1.0 + i as f32 * 0.24);
        painter.circle_filled(center, r, alpha(color, (26.0 / (i as f32 + 1.0)) as u8));
    }
}
