use crate::assets::Assets;
use crate::theme::{self, alpha};
use egui::{
    Color32, CornerRadius, Id, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Power,
    Fan,
    Keyboard,
    Gpu,
    Flexikey,
}

#[derive(Clone, Debug)]
pub enum Action {
    Go(Page),
    Profile(u8),
    Backlight(u8),
    FanSpeed(u8, u8),
    FanAuto(bool),
    Color([u8; 3], bool),
    ChargeLimit(u8),
    Close,
    Minimize,
}

pub struct Canvas<'a> {
    pub ui: &'a mut Ui,
    pub assets: &'a Assets,
    pub scale: f32,
    pub ox: f32,
    pub oy: f32,
    pub interactive: bool,
}

impl<'a> Canvas<'a> {
    pub fn new(ui: &'a mut Ui, assets: &'a Assets, area: Rect) -> Self {
        Self::with_pixel_scale(ui, assets, area, None)
    }

    /// `pixel_scale` is physical pixels per design pixel. `None` fits the canvas.
    pub fn with_pixel_scale(
        ui: &'a mut Ui,
        assets: &'a Assets,
        area: Rect,
        pixel_scale: Option<f32>,
    ) -> Self {
        let ppp = ui.ctx().pixels_per_point().max(0.001);
        let physical_w = area.width() * ppp;
        let physical_h = area.height() * ppp;
        let scale = match pixel_scale {
            Some(value) => value / ppp,
            None => ((physical_w / theme::WIN_W).min(physical_h / theme::WIN_H)) / ppp,
        };
        let ox = area.left() + (area.width() - theme::WIN_W * scale) * 0.5;
        let oy = area.top() + (area.height() - theme::WIN_H * scale) * 0.5;
        // Keep textures and text on whole physical pixels; the screenshot crop uses the same floor.
        let ox = (ox * ppp).floor() / ppp;
        let oy = (oy * ppp).floor() / ppp;
        Self {
            ui,
            assets,
            scale,
            ox,
            oy,
            interactive: true,
        }
    }

    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.ox + x * self.scale, self.oy + y * self.scale)
    }

    pub fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), Vec2::new(w * self.scale, h * self.scale))
    }

    pub fn painter(&self) -> egui::Painter {
        self.ui.painter().clone()
    }

    pub fn image(&self, name: &str, rect: Rect, tint: Color32) -> bool {
        if let Some(texture) = self.assets.get(name) {
            self.painter().image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
            true
        } else {
            false
        }
    }

    pub fn image_at(&self, name: &str, x: f32, y: f32, w: f32, h: f32, tint: Color32) -> bool {
        self.image(name, self.r(x, y, w, h), tint)
    }

    pub fn interact(&mut self, id: &str, rect: Rect, sense: Sense) -> Response {
        if self.interactive {
            self.ui
                .interact(rect, Id::new(("acer-cc", id)), sense)
        } else {
            self.ui.interact(
                Rect::from_min_size(Pos2::new(-500.0, -500.0), Vec2::splat(1.0)),
                Id::new(("acer-cc", id)),
                Sense::hover(),
            )
        }
    }

    pub fn hovered(&self, rect: Rect) -> bool {
        self.interactive
            && self
                .ui
                .ctx()
                .pointer_hover_pos()
                .map(|p| rect.contains(p))
                .unwrap_or(false)
    }

    pub fn pointer(&self) -> Option<Pos2> {
        self.ui.ctx().pointer_hover_pos()
    }

    pub fn text(
        &self,
        center: Pos2,
        text: &str,
        font: egui::FontId,
        color: Color32,
    ) -> Rect {
        let painter = self.painter();
        let galley = painter.layout_no_wrap(text.to_owned(), font, color);
        let pos = Pos2::new(center.x - galley.size().x * 0.5, center.y - galley.size().y * 0.5);
        let rect = Rect::from_min_size(pos, galley.size());
        painter.galley(pos, galley, color);
        rect
    }

    pub fn text_left(
        &self,
        pos: Pos2,
        text: &str,
        font: egui::FontId,
        color: Color32,
    ) -> Rect {
        let painter = self.painter();
        let galley = painter.layout_no_wrap(text.to_owned(), font, color);
        let rect = Rect::from_min_size(pos, galley.size());
        painter.galley(pos, galley, color);
        rect
    }

    pub fn text_top_center(
        &self,
        pos: Pos2,
        text: &str,
        font: egui::FontId,
        color: Color32,
    ) -> Rect {
        let painter = self.painter();
        let galley = painter.layout_no_wrap(text.to_owned(), font, color);
        let rect = Rect::from_min_size(
            Pos2::new(pos.x - galley.size().x * 0.5, pos.y),
            galley.size(),
        );
        painter.galley(rect.min, galley, color);
        rect
    }

    pub fn tracked(
        &self,
        center: Pos2,
        text: &str,
        font: egui::FontId,
        color: Color32,
        tracking: f32,
    ) {
        theme::tracked_center(&self.painter(), center, text, font, color, tracking * self.scale);
    }

    pub fn tracked_left(
        &self,
        pos: Pos2,
        text: &str,
        font: egui::FontId,
        color: Color32,
        tracking: f32,
    ) {
        theme::tracked_left(&self.painter(), pos, text, font, color, tracking * self.scale);
    }

    pub fn line(&self, a: Pos2, b: Pos2, width: f32, color: Color32) {
        self.painter()
            .line_segment([a, b], Stroke::new(width * self.scale, color));
    }

    pub fn dotted_frame(&self, rect: Rect, bright: bool) {
        let color = if bright {
            alpha(theme::FRAME_HI, 245)
        } else {
            alpha(theme::FRAME_HI, 215)
        };
        let painter = self.painter();
        let dot = (1.15 * self.scale).max(1.0);
        let step = (2.6 * self.scale).max(2.0);
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let delta = b - a;
            let length = delta.length();
            if length <= 0.0 {
                continue;
            }
            let dir = delta / length;
            let mut t = 0.0;
            while t <= length {
                let center = a + dir * t;
                painter.rect_filled(
                    Rect::from_center_size(center, Vec2::splat(dot)),
                    0,
                    color,
                );
                t += step;
            }
        }
    }

    pub fn panel(&self, rect: Rect, tint: Color32, radius: f32) {
        self.painter()
            .rect_filled(rect, CornerRadius::from(radius * self.scale), tint);
    }

    pub fn card(&self, rect: Rect, title: Option<&str>) {
        self.painter().rect_filled(
            rect,
            CornerRadius::from(2.0 * self.scale),
            Color32::from_rgba_unmultiplied(10, 8, 12, 165),
        );
        self.painter().rect_stroke(
            rect,
            CornerRadius::from(2.0 * self.scale),
            Stroke::new(1.0, alpha(theme::FRAME, 115)),
            StrokeKind::Inside,
        );
        if let Some(title) = title {
            self.tracked_left(
                Pos2::new(rect.left() + 22.0 * self.scale, rect.top() + 17.0 * self.scale),
                title,
                theme::display(14.0 * self.scale),
                alpha(theme::TXT, 215),
                1.6,
            );
        }
    }
}

pub struct SliderOutcome {
    pub changed: bool,
    pub released: bool,
}

pub fn slider(
    c: &mut Canvas,
    id: &str,
    rect: Rect,
    value: &mut f32,
    min: f32,
    max: f32,
    accent: Color32,
    enabled: bool,
) -> SliderOutcome {
    let hit = rect.expand(10.0 * c.scale);
    let response = c.interact(
        id,
        hit,
        if enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        },
    );
    let mut changed = false;
    let mut released = false;
    if enabled {
        let pointer = c.ui.ctx().pointer_interact_pos();
        if (response.dragged() || response.clicked()) && pointer.is_some() {
            if let Some(pos) = pointer {
                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                let next = min + (max - min) * t;
                if (next - *value).abs() > f32::EPSILON {
                    *value = next;
                    changed = true;
                }
            }
        }
        released = response.drag_stopped();
    }

    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let painter = c.painter();
    let track = Rect::from_min_max(
        Pos2::new(rect.left(), rect.center().y - 2.0 * c.scale),
        Pos2::new(rect.right(), rect.center().y + 2.0 * c.scale),
    );
    painter.rect_filled(track, 99, Color32::from_rgb(38, 34, 42));
    let filled = Rect::from_min_max(
        track.left_top(),
        Pos2::new(track.left() + track.width() * t, track.bottom()),
    );
    let tint = if enabled {
        accent
    } else {
        alpha(accent, 90)
    };
    painter.rect_filled(filled, 99, tint);
    let knob_center = Pos2::new(track.left() + track.width() * t, track.center().y);
    let hovered = response.hovered() && enabled;
    painter.circle_filled(
        knob_center,
        (7.0 + if hovered { 1.5 } else { 0.0 }) * c.scale,
        if enabled {
            Color32::from_rgb(238, 234, 242)
        } else {
            Color32::from_rgb(120, 116, 126)
        },
    );
    painter.circle_filled(
        knob_center,
        3.0 * c.scale,
        if enabled { tint } else { alpha(tint, 150) },
    );
    if hovered {
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    SliderOutcome { changed, released }
}

pub fn toggle(c: &mut Canvas, id: &str, rect: Rect, on: &mut bool) -> bool {
    let response = c.interact(id, rect.expand(6.0 * c.scale), Sense::click());
    let mut changed = false;
    if response.clicked() {
        *on = !*on;
        changed = true;
    }
    let painter = c.painter();
    let track = Rect::from_min_max(
        Pos2::new(rect.left(), rect.center().y - 11.0 * c.scale),
        Pos2::new(rect.right(), rect.center().y + 11.0 * c.scale),
    );
    let fill = if *on {
        alpha(theme::MAGENTA, 190)
    } else {
        Color32::from_rgb(44, 40, 49)
    };
    painter.rect_filled(track, 99, fill);
    let knob_x = if *on {
        track.right() - track.height() * 0.5
    } else {
        track.left() + track.height() * 0.5
    };
    painter.circle_filled(
        Pos2::new(knob_x, track.center().y),
        track.height() * 0.38,
        Color32::from_rgb(240, 237, 244),
    );
    if response.hovered() {
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    changed
}

pub fn back_button(c: &mut Canvas, rect: Rect, id: &str) -> bool {
    let response = c.interact(id, rect, Sense::click());
    let hovered = response.hovered();
    if hovered {
        theme::glow_circle(
            &c.painter(),
            rect.center(),
            rect.width() * 0.48,
            theme::MAGENTA,
        );
    }
    if !hovered {
        // bright center
        c.painter().circle_filled(
            rect.center(),
            rect.width() * 0.30,
            Color32::from_rgba_unmultiplied(255, 255, 255, 100),
        );
        // outer ring approximating reference thickness
        c.painter().circle_stroke(
            rect.center(),
            rect.width() * 0.45,
            (2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 50)),
        );
    }
    c.image(
        "qlabel_back_circle",
        rect,
        if hovered {
            Color32::WHITE
        } else {
            Color32::from_rgba_unmultiplied(80, 70, 80, 180)
        },
    );
    c.tracked(
        Pos2::new(rect.center().x, rect.bottom() + 25.0 * c.scale),
        "BACK",
        theme::sans(30.0 * c.scale),
        Color32::WHITE,
        0.0,
    );
    let clicked = response.clicked();
    if hovered {
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    clicked
}

pub fn icon_button(c: &mut Canvas, rect: Rect, glyph: &str, baked: bool) -> bool {
    let response = c.interact(glyph, rect, Sense::click());
    let hovered = response.hovered();
    if hovered {
        c.painter().circle_filled(
            rect.center(),
            21.0 * c.scale,
            Color32::from_rgba_unmultiplied(255, 255, 255, 14),
        );
    }
    if hovered || !baked {
        let color = if hovered {
            Color32::WHITE
        } else {
            Color32::from_rgb(225, 222, 229)
        };
        let center = rect.center();
        match glyph {
            "close" => {
                let sx = 15.0 * c.scale;
                let sy = 14.5 * c.scale;
                c.line(
                    Pos2::new(center.x - sx, center.y - sy),
                    Pos2::new(center.x + sx, center.y + sy),
                    2.9,
                    color,
                );
                c.line(
                    Pos2::new(center.x - sx, center.y + sy),
                    Pos2::new(center.x + sx, center.y - sy),
                    2.9,
                    color,
                );
            }
            "minimize" => {
                c.line(
                    Pos2::new(center.x - 20.0 * c.scale, center.y),
                    Pos2::new(center.x + 20.0 * c.scale, center.y),
                    3.6,
                    color,
                );
            }
            _ => {}
        }
    }
    let clicked = response.clicked();
    if hovered {
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    clicked
}

pub fn pill(c: &Canvas, rect: Rect, text: &str, color: Color32) {
    c.painter().rect_filled(
        rect,
        CornerRadius::from(rect.height() * 0.5),
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 26),
    );
    c.painter().rect_stroke(
        rect,
        CornerRadius::from(rect.height() * 0.5),
        Stroke::new(1.0, alpha(color, 110)),
        StrokeKind::Inside,
    );
    c.text(rect.center(), text, theme::sans(12.0 * c.scale), color);
}

pub fn battery_glyph(painter: &egui::Painter, rect: Rect, pct: f32, color: Color32) {
    let body = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + rect.height() * 0.12),
        Pos2::new(rect.right() - rect.width() * 0.08, rect.bottom() - rect.height() * 0.12),
    );
    painter.rect_stroke(
        body,
        CornerRadius::from(rect.height() * 0.12),
        Stroke::new(1.4, alpha(color, 190)),
        StrokeKind::Inside,
    );
    let nub = Rect::from_min_max(
        Pos2::new(body.right(), body.center().y - body.height() * 0.16),
        Pos2::new(rect.right(), body.center().y + body.height() * 0.16),
    );
    painter.rect_filled(nub, CornerRadius::from(1.0), alpha(color, 170));
    let inset = 2.6;
    let fill_w = (body.width() - inset * 2.0) * (pct / 100.0).clamp(0.0, 1.0);
    let fill = Rect::from_min_max(
        Pos2::new(body.left() + inset, body.top() + inset),
        Pos2::new(body.left() + inset + fill_w, body.bottom() - inset),
    );
    painter.rect_filled(fill, CornerRadius::from(1.5), alpha(color, 210));
}

pub fn section_label(c: &Canvas, pos: Pos2, text: &str) {
    c.tracked_left(pos, text, theme::display(13.0 * c.scale), alpha(theme::TXT_DIM, 235), 1.8);
}

pub fn kv(c: &Canvas, pos: Pos2, key: &str, value: &str) {
    c.text_left(pos, key, theme::sans(13.0 * c.scale), theme::TXT_FAINT);
    let x = pos.x + 132.0 * c.scale;
    c.text_left(
        Pos2::new(x, pos.y),
        value,
        theme::sans(13.0 * c.scale),
        theme::TXT_DIM,
    );
}

pub fn centered_text(c: &Canvas, center: Pos2, text: &str, size: f32, color: Color32) -> Rect {
    c.text(center, text, theme::sans(size * c.scale), color)
}

pub fn subtitle(c: &Canvas, pos: Pos2, text: &str) {
    c.text_left(pos, text, theme::sans(15.0 * c.scale), theme::TXT_DIM);
}

pub fn value_text(c: &Canvas, pos: Pos2, text: &str, size: f32, color: Color32) {
    c.text_left(pos, text, theme::display(size * c.scale), color);
}

pub fn hover_glow(c: &Canvas, rect: Rect, color: Color32) {
    let p = c.painter();
    p.rect_filled(
        rect,
        CornerRadius::from(3.0 * c.scale),
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 18),
    );
}
