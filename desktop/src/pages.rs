use crate::backend::Snapshot;
use crate::theme::{self, alpha};
use crate::widgets::{
    self, back_button, battery_glyph, centered_text, kv, section_label, slider, subtitle, toggle,
    value_text, Action, Canvas, Page,
};
use egui::{Color32, Pos2, Rect, Sense};

const LABEL_SCALE: f32 = 1600.0 / 2601.0;
const BAKED_HOME: bool = true;

pub struct Controls {
    pub fan_auto: bool,
    pub fan: [f32; 3],
    pub backlight: f32,
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
    pub charge: f32,
    pub touched: Option<std::time::Instant>,
    pub initialized: bool,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            fan_auto: false,
            fan: [0.0; 3],
            backlight: 0.0,
            hue: 0.83,
            sat: 0.85,
            val: 1.0,
            charge: 80.0,
            touched: None,
            initialized: false,
        }
    }
}

impl Controls {
    pub fn touch(&mut self) {
        self.touched = Some(std::time::Instant::now());
    }

    pub fn sync(&mut self, state: &Snapshot) {
        if !self.initialized {
            self.fan = [
                state.fans[0].speed as f32,
                state.fans[1].speed as f32,
                state.fans[2].speed as f32,
            ];
            self.backlight = state.backlight as f32;
            self.charge = state.charge_limit as f32;
            let (h, s, v) = rgb_to_hsv(state.color);
            self.hue = h;
            self.sat = s;
            self.val = v;
            self.initialized = true;
            return;
        }
        let idle = self
            .touched
            .map(|t| t.elapsed().as_millis() > 1600)
            .unwrap_or(true);
        if !idle {
            return;
        }
        for i in 0..3 {
            let value = state.fans[i].speed as f32;
            if (self.fan[i] - value).abs() > 1.5 {
                self.fan[i] = value;
            }
        }
        if (self.backlight - state.backlight as f32).abs() > 2.0 {
            self.backlight = state.backlight as f32;
        }
        if (self.charge - state.charge_limit as f32).abs() > 0.5 {
            self.charge = state.charge_limit as f32;
        }
        let (h, s, v) = rgb_to_hsv(state.color);
        if (self.hue - h).abs() > 0.02 || (self.sat - s).abs() > 0.05 {
            self.hue = h;
            self.sat = s;
            self.val = v;
        }
    }
}

fn rgb_to_hsv(rgb: [u8; 3]) -> (f32, f32, f32) {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= f32::EPSILON {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    let s = if max <= f32::EPSILON { 0.0 } else { d / max };
    (h, s, max)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}

pub fn current_rgb(c: &Controls) -> [u8; 3] {
    hsv_to_rgb(c.hue, c.sat, c.val)
}

fn asset_label(c: &Canvas, name: &str, cx: f32, cy: f32) {
    if let Some(texture) = c.assets.get(name) {
        let [w, h] = texture.size();
        let width = w as f32 * LABEL_SCALE;
        let height = h as f32 * LABEL_SCALE;
        let rect = c.r(cx - width * 0.5, cy - height * 0.5, width, height);
        c.image(name, rect, Color32::WHITE);
    }
}

struct Tile {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    patch: &'static str,
    px: f32,
    py: f32,
    pw: f32,
    ph: f32,
    label: &'static str,
    lx: f32,
    ly: f32,
    page: Page,
    tip: &'static str,
}

pub fn home(c: &mut Canvas, actions: &mut Vec<Action>) {
    let tiles = [
        Tile {
            x: 19.5,
            y: 210.5,
            w: 382.0,
            h: 382.0,
            patch: "home_power",
            px: 73.0,
            py: 250.0,
            pw: 262.0,
            ph: 262.0,
            label: "label_power",
            lx: 216.0,
            ly: 531.0,
            page: Page::Power,
            tip: "Select preferred\noperation mode.",
        },
        Tile {
            x: 410.5,
            y: 210.5,
            w: 385.0,
            h: 382.0,
            patch: "home_keyboard",
            px: 456.0,
            py: 241.0,
            pw: 312.0,
            ph: 272.0,
            label: "label_keyboard",
            lx: 607.0,
            ly: 531.0,
            page: Page::Keyboard,
            tip: "Setup the LED color and\nlighting effect.",
        },
        Tile {
            x: 804.5,
            y: 210.5,
            w: 774.0,
            h: 382.0,
            patch: "home_gpu",
            px: 1056.0,
            py: 249.0,
            pw: 272.0,
            ph: 272.0,
            label: "label_gpu",
            lx: 1193.5,
            ly: 524.5,
            page: Page::Gpu,
            tip: "View graphics processor\ninformation.",
        },
        Tile {
            x: 19.5,
            y: 611.5,
            w: 776.0,
            h: 384.0,
            patch: "home_flexikey",
            px: 279.0,
            py: 658.0,
            pw: 272.0,
            ph: 256.0,
            label: "label_flexikey",
            lx: 414.5,
            ly: 926.5,
            page: Page::Flexikey,
            tip: "Configure Flexikey\nshortcuts.",
        },
        Tile {
            x: 804.5,
            y: 611.5,
            w: 774.0,
            h: 384.0,
            patch: "home_fan",
            px: 1057.0,
            py: 660.0,
            pw: 272.0,
            ph: 254.0,
            label: "label_fan",
            lx: 1192.0,
            ly: 926.5,
            page: Page::Fan,
            tip: "Adjust fan speeds and\nmonitor temperatures.",
        },
    ];

    for tile in tiles {
        let rect = c.r(tile.x, tile.y, tile.w, tile.h);
        let id = format!("tile-{:?}", tile.page);
        let response = c.interact(&id, rect, Sense::click());
        let hovered = response.hovered();
        if !BAKED_HOME {
            c.image_at(tile.patch, tile.px, tile.py, tile.pw, tile.ph, Color32::WHITE);
            asset_label(c, tile.label, tile.lx, tile.ly);
        }
        if hovered {
            c.panel(
                rect.shrink(1.5),
                Color32::from_rgba_unmultiplied(4, 3, 6, 196),
                0.0,
            );
            let painter = c.painter();
            let galley = painter.layout(
                tile.tip.to_owned(),
                theme::sans(15.5 * c.scale),
                Color32::from_rgb(233, 230, 236),
                (tile.w - 40.0) * c.scale,
            );
            painter.galley(
                Pos2::new(
                    rect.left() + 18.0 * c.scale,
                    rect.top() + 16.0 * c.scale,
                ),
                galley,
                theme::TXT,
            );
        }
        if hovered || !BAKED_HOME {
            c.dotted_frame(rect, hovered);
        }
        if response.clicked() {
            actions.push(Action::Go(tile.page));
        }
        if hovered {
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }
}

struct Mode {
    pos: &'static str,
    icon: &'static str,
    label: &'static str,
    id: u8,
    /// Reference-space rect of the metal sector image.
    metal: (f32, f32, f32, f32),
    /// BAML metal inset inside the full radio-button rect.
    metal_margin: (f32, f32),
    metal_size: (f32, f32),
    full_size: (f32, f32),
    /// BAML icon bottom margin and (inactive, active) icon size.
    icon_bottom: f32,
    icon_sizes: (f32, f32),
    /// BAML label margins (left, top, right, bottom) and label box height.
    label_margin: (f32, f32, f32, f32),
    label_height: f32,
    /// Empirical nudge from the BAML label box, in reference pixels.
    label_offset: (f32, f32),
    /// Label point sizes for the inactive and active states.
    label_font: (f32, f32),
}

impl Mode {
    fn scale(&self) -> f32 {
        self.metal.2 / self.metal_size.0
    }

    fn full(&self) -> (f32, f32, f32, f32) {
        let s = self.scale();
        (
            self.metal.0 - self.metal_margin.0 * s,
            self.metal.1 - self.metal_margin.1 * s,
            self.full_size.0 * s,
            self.full_size.1 * s,
        )
    }

    fn icon_rect(&self, active: bool) -> (f32, f32, f32, f32) {
        let s = self.scale();
        let (fx, fy, fw, fh) = self.full();
        let size = if active {
            self.icon_sizes.1
        } else {
            self.icon_sizes.0
        } * s;
        (
            fx + fw * 0.5 - size * 0.5,
            fy + fh - self.icon_bottom * s - size,
            size,
            size,
        )
    }

    fn label_rect(&self) -> (f32, f32, f32, f32) {
        let s = self.scale();
        let (fx, fy, fw, fh) = self.full();
        let (left, _top, right, bottom) = self.label_margin;
        (
            fx + left * s + self.label_offset.0,
            fy + fh - bottom * s - self.label_height * s + self.label_offset.1,
            fw - (left + right) * s,
            self.label_height * s,
        )
    }

    fn font(&self, active: bool) -> f32 {
        let size = if active {
            self.label_font.1
        } else {
            self.label_font.0
        };
        size * self.scale()
    }
}

const MODES: [Mode; 4] = [
    Mode {
        pos: "top",
        icon: "mode_quiet",
        label: "Quiet",
        id: 0,
        metal: (423.9, 281.7, 706.2, 199.9),
        metal_margin: (73.889, 20.0),
        metal_size: (459.0, 130.0),
        full_size: (600.002, 150.0),
        icon_bottom: 51.0,
        icon_sizes: (140.0, 160.0),
        label_margin: (181.111, 0.0, 158.0, 28.815),
        label_height: 40.756,
        label_offset: (2.5, -1.0),
        label_font: (24.0, 29.333),
    },
    Mode {
        pos: "left",
        icon: "mode_power_saving",
        label: "Power Saving",
        id: 1,
        metal: (155.6, 347.6, 485.2, 498.3),
        metal_margin: (48.0, 36.667),
        metal_size: (315.0, 324.0),
        full_size: (363.0, 403.0),
        icon_bottom: 166.0,
        icon_sizes: (140.0, 160.0),
        label_margin: (80.222, 0.0, 60.334, 105.616),
        label_height: 77.311,
        label_offset: (0.0, -3.0),
        label_font: (24.0, 29.333),
    },
    Mode {
        pos: "right",
        icon: "mode_performance",
        label: "Performance",
        id: 2,
        metal: (913.4, 291.6, 567.1, 631.7),
        metal_margin: (0.0, 0.0),
        metal_size: (369.0, 411.0),
        full_size: (369.0, 411.0),
        icon_bottom: 175.0,
        icon_sizes: (140.0, 160.0),
        label_margin: (47.0, 0.0, 47.0, 99.8),
        label_height: 100.0, // ponytail: raise to match reference height
        label_offset: (-3.0, -3.0),
        label_font: (24.0, 29.333),
    },
    Mode {
        pos: "bottom",
        icon: "mode_entertainment",
        label: "Entertainment",
        id: 3,
        metal: (288.5, 629.3, 980.9, 355.6),
        metal_margin: (53.0, 0.0),
        metal_size: (638.0, 231.0),
        full_size: (738.0, 250.0),
        icon_bottom: 70.0,
        icon_sizes: (140.0, 180.0),
        label_margin: (134.0, 0.0, 130.0, 33.0),
        label_height: 57.2,
        label_offset: (2.5, 1.5),
        label_font: (24.0, 29.333),
    },
];

pub fn power(c: &mut Canvas, actions: &mut Vec<Action>, profile: u8, baked: bool) {
    let back = c.r(72.0, 136.0, 173.0, 173.0);
    if baked {
        let response = c.interact("back-power", back, Sense::click());
        if response.hovered() {
            theme::glow_circle(
                &c.painter(),
                back.center(),
                back.width() * 0.48,
                theme::MAGENTA,
            );
        }
        if response.clicked() {
            actions.push(Action::Go(Page::Home));
        }
    } else if back_button(c, back, "back-power") {
        actions.push(Action::Go(Page::Home));
    }

    let mut hovered_mode: Option<u8> = None;
    for mode in &MODES {
        let (ix, iy, isz, _) = mode.icon_rect(true);
        let (lx, ly, lw, lh) = mode.label_rect();
        let hx = ix.min(lx) - 12.0;
        let hy = iy.min(ly) - 12.0;
        let hw = (ix + isz).max(lx + lw) - hx + 12.0;
        let hh = (iy + isz).max(ly + lh) - hy + 12.0;
        let hit = c.r(hx, hy, hw, hh);
        let response = c.interact(&format!("mode-{}", mode.id), hit, Sense::click());
        if response.hovered() && mode.id != profile {
            hovered_mode = Some(mode.id);
        }
        if response.clicked() {
            actions.push(Action::Profile(mode.id));
        }
        if response.hovered() {
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }

    if baked {
        if let Some(mode) = hovered_mode.and_then(|id| MODES.iter().find(|m| m.id == id)) {
            let (fx, fy, fw, fh) = mode.full();
            c.image_at(
                &format!("quad_{}_fill", mode.pos),
                fx,
                fy,
                fw,
                fh,
                Color32::from_rgba_unmultiplied(153, 0, 250, 54),
            );
        }
        return;
    }

    c.image_at(
        "ring_decor",
        81.7,
        250.9,
        1399.1,
        762.6,
        Color32::WHITE,
    );

    // Solid sector silhouettes sit behind the metal, matching the reference shading.
    for mode in &MODES {
        let (x, y, w, h) = mode.metal;
        c.image_at(
            &format!("quad_{}_shadow", mode.pos),
            x,
            y,
            w,
            h,
            Color32::WHITE,
        );
    }

    for mode in &MODES {
        let (x, y, w, h) = mode.metal;
        let alpha = if hovered_mode == Some(mode.id) { 204 } else { 255 };
        c.image_at(
            &format!("quad_{}_metal", mode.pos),
            x,
            y,
            w,
            h,
            Color32::from_white_alpha(alpha),
        );
    }

    if let Some(mode) = MODES.iter().find(|m| m.id == profile) {
        let (fx, fy, fw, fh) = mode.full();
        c.image_at(
            &format!("quad_{}_fill", mode.pos),
            fx,
            fy,
            fw,
            fh,
            Color32::from_rgb(153, 0, 250),
        );
    }

    for mode in &MODES {
        let active = mode.id == profile;
        let (ix, iy, isz, _) = mode.icon_rect(active);
        c.image_at(mode.icon, ix, iy, isz, isz, Color32::WHITE);

        let (lx, ly, lw, _) = mode.label_rect();
        let color = if active {
            Color32::WHITE
        } else {
            Color32::from_rgb(173, 173, 173)
        };
        c.text_top_center(
            c.p(lx + lw * 0.5, ly),
            mode.label,
            theme::sans(mode.font(active) * c.scale),
            color,
        );
    }
}

pub fn fan(
    c: &mut Canvas,
    actions: &mut Vec<Action>,
    controls: &mut Controls,
    state: &Snapshot,
) {
    let back = c.r(72.0, 136.0, 173.0, 173.0);
    if back_button(c, back, "back-fan") {
        actions.push(Action::Go(Page::Home));
    }

    value_text(
        c,
        c.p(300.0, 286.0),
        "THERMAL CONTROL",
        41.0,
        theme::TXT,
    );
    subtitle(
        c,
        c.p(303.0, 341.0),
        "Fan curves, temperatures and battery care.",
    );
    let status_text = if state.online {
        "HARDWARE LINK ACTIVE"
    } else {
        "HARDWARE SERVICE OFFLINE"
    };
    let status_color = if state.online { theme::GREEN } else { theme::RED };
    widgets::pill(
        c,
        c.r(1130.0, 292.0, 260.0, 27.0),
        status_text,
        status_color,
    );

    let card_w = 366.0;
    let positions = [200.0, 617.0, 1034.0];
    let names = ["CPU FAN", "GPU FAN", "SYSTEM FAN"];
    for i in 0..3 {
        let x = positions[i];
        let y = 390.0;
        let rect = c.r(x, y, card_w, 294.0);
        c.card(rect, Some(names[i]));
        let detected = state.fans[i].speed > 0 || state.fans[i].temp > 0 || i < 2;
        c.image_at(
            "tile_fan",
            x + 24.0,
            y + 54.0,
            112.0,
            112.0,
            if detected {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(70)
            },
        );
        let speed = controls.fan[i].round() as u8;
        value_text(
            c,
            c.p(x + 152.0, y + 66.0),
            &format!("{speed:02}"),
            55.0,
            if controls.fan_auto {
                theme::TXT_DIM
            } else {
                theme::TXT
            },
        );
        c.text_left(
            c.p(x + 230.0, y + 92.0),
            "%",
            theme::display(20.0 * c.scale),
            theme::TXT_FAINT,
        );
        let temp = state.fans[i].temp;
        let temp_color = if temp >= 80 {
            theme::RED
        } else if temp >= 60 {
            theme::AMBER
        } else {
            theme::GREEN
        };
        let temp_text = if temp > 0 {
            format!("{temp} °C")
        } else {
            "— °C".to_owned()
        };
        c.text_left(
            c.p(x + 152.0, y + 139.0),
            &temp_text,
            theme::sans(17.0 * c.scale),
            temp_color,
        );
        if !detected {
            c.text_left(
                c.p(x + 24.0, y + 190.0),
                "No sensor detected",
                theme::sans(12.5 * c.scale),
                theme::TXT_FAINT,
            );
        }

        let auto_rect = c.r(x + card_w - 74.0, y + 18.0, 46.0, 22.0);
        section_label(c, c.p(x + card_w - 126.0, y + 22.0), "AUTO");
        if toggle(c, &format!("fanauto-{i}"), auto_rect, &mut controls.fan_auto) {
            actions.push(Action::FanAuto(controls.fan_auto));
            controls.touch();
        }

        let mut value = controls.fan[i];
        let out = slider(
            c,
            &format!("fanslider-{i}"),
            Rect::from_min_max(
                c.p(x + 24.0, y + 224.0),
                c.p(x + card_w - 24.0, y + 244.0),
            ),
            &mut value,
            0.0,
            100.0,
            theme::MAGENTA,
            !controls.fan_auto && detected,
        );
        if out.changed {
            controls.fan[i] = value;
            controls.touch();
            actions.push(Action::FanSpeed(i as u8, value.round() as u8));
        }
        c.text_left(
            c.p(x + 24.0, y + 252.0),
            "0",
            theme::sans(11.0 * c.scale),
            theme::TXT_FAINT,
        );
        c.text_left(
            c.p(x + card_w - 34.0, y + 252.0),
            "100",
            theme::sans(11.0 * c.scale),
            theme::TXT_FAINT,
        );
    }

    let battery_rect = c.r(200.0, 707.0, 1200.0, 250.0);
    c.card(battery_rect, Some("BATTERY CARE"));
    battery_glyph(
        &c.painter(),
        c.r(228.0, 760.0, 78.0, 42.0),
        state.battery as f32,
        theme::GREEN,
    );
    value_text(
        c,
        c.p(324.0, 748.0),
        &format!("{}%", state.battery),
        45.0,
        theme::TXT,
    );
    c.text_left(
        c.p(326.0, 812.0),
        if state.battery_status.is_empty() {
            "Battery status unavailable"
        } else {
            &state.battery_status
        },
        theme::sans(14.0 * c.scale),
        theme::TXT_DIM,
    );
    c.text_left(
        c.p(326.0, 862.0),
        "Charge thresholds are applied by the root service and survive reboots.",
        theme::sans(12.5 * c.scale),
        theme::TXT_FAINT,
    );

    section_label(c, c.p(760.0, 752.0), "CHARGE LIMIT");
    value_text(
        c,
        c.p(1090.0, 744.0),
        &format!("{}%", controls.charge.round() as u8),
        34.0,
        theme::MAGENTA,
    );
    let mut charge = controls.charge;
    let out = slider(
        c,
        "chargelimit",
        Rect::from_min_max(c.p(760.0, 812.0), c.p(1360.0, 832.0)),
        &mut charge,
        50.0,
        100.0,
        theme::MAGENTA,
        state.charge_limit > 0,
    );
    if out.changed {
        controls.charge = charge.round();
        controls.touch();
    }
    if out.released || (out.changed && !c.ui.ctx().input(|i| i.pointer.primary_down())) {
        actions.push(Action::ChargeLimit(controls.charge.round() as u8));
    }
    for (value, label) in [(60.0, "60"), (80.0, "80"), (100.0, "100")] {
        let x = 760.0 + (value - 50.0) / 50.0 * 600.0;
        c.line(
            c.p(x, 840.0),
            c.p(x, 846.0),
            1.0,
            theme::TXT_FAINT,
        );
        c.text_left(
            c.p(x - 8.0, 850.0),
            label,
            theme::sans(11.0 * c.scale),
            theme::TXT_FAINT,
        );
    }
}

pub fn keyboard(c: &mut Canvas, actions: &mut Vec<Action>, controls: &mut Controls) {
    let back = c.r(72.0, 136.0, 173.0, 173.0);
    if back_button(c, back, "back-keyboard") {
        actions.push(Action::Go(Page::Home));
    }
    value_text(c, c.p(300.0, 286.0), "LED KEYBOARD", 41.0, theme::TXT);
    subtitle(
        c,
        c.p(303.0, 341.0),
        "Backlight brightness and per-key RGB color.",
    );

    let rgb = current_rgb(controls);
    let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);

    let left = c.r(200.0, 390.0, 560.0, 560.0);
    c.card(left, Some("BACKLIGHT"));
    theme::glow_circle(&c.painter(), c.p(480.0, 556.0), 96.0 * c.scale, color);
    c.image_at("tile_keyboard", 330.0, 431.0, 300.0, 300.0, Color32::WHITE);

    section_label(c, c.p(232.0, 772.0), "BRIGHTNESS");
    value_text(
        c,
        c.p(600.0, 762.0),
        &format!("{}%", (controls.backlight / 255.0 * 100.0).round() as u8),
        27.0,
        theme::TXT,
    );
    let mut brightness = controls.backlight;
    let out = slider(
        c,
        "kbd-brightness",
        Rect::from_min_max(c.p(232.0, 816.0), c.p(700.0, 836.0)),
        &mut brightness,
        0.0,
        255.0,
        theme::MAGENTA,
        true,
    );
    if out.changed {
        controls.backlight = brightness.round();
        controls.touch();
        actions.push(Action::Backlight(controls.backlight.round() as u8));
    }
    c.text_left(
        c.p(232.0, 866.0),
        "Brightness is written to the TUXEDO keyboard LED interface.",
        theme::sans(12.0 * c.scale),
        theme::TXT_FAINT,
    );

    let right = c.r(800.0, 390.0, 600.0, 560.0);
    c.card(right, Some("COLOR"));

    let wheel_rect = c.r(1010.0, 436.0, 290.0, 290.0);
    c.image("color_wheel", wheel_rect, Color32::WHITE);
    let response = c.interact(
        "color-wheel",
        wheel_rect,
        Sense::click_and_drag(),
    );
    let mut changed = false;
    if response.dragged() || response.clicked() {
        if let Some(pointer) = c.ui.ctx().pointer_interact_pos() {
            let center = wheel_rect.center();
            let dx = (pointer.x - center.x) / (wheel_rect.width() * 0.5);
            let dy = (pointer.y - center.y) / (wheel_rect.height() * 0.5);
            let radius = (dx * dx + dy * dy).sqrt();
            if radius <= 1.02 {
                controls.hue = (-dy).atan2(dx) / std::f32::consts::TAU;
                if controls.hue < 0.0 {
                    controls.hue += 1.0;
                }
                controls.sat = radius.clamp(0.0, 1.0);
                controls.touch();
                changed = true;
            }
        }
    }
    let selected = if controls.sat <= 0.03 {
        Pos2::new(0.0, 0.0)
    } else {
        let angle = controls.hue * std::f32::consts::TAU;
        Pos2::new(angle.cos() * controls.sat, -angle.sin() * controls.sat)
    };
    let knob = Pos2::new(
        wheel_rect.center().x + selected.x * wheel_rect.width() * 0.5,
        wheel_rect.center().y + selected.y * wheel_rect.height() * 0.5,
    );
    c.painter()
        .circle_stroke(knob, 8.0 * c.scale, egui::Stroke::new(2.0, Color32::WHITE));
    c.painter()
        .circle_stroke(knob, 10.0 * c.scale, egui::Stroke::new(1.0, Color32::BLACK));

    let hex = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
    c.painter().rect_filled(
        c.r(1158.0, 470.0, 34.0, 34.0),
        egui::CornerRadius::from(4.0),
        color,
    );
    c.text_left(
        c.p(1202.0, 476.0),
        &hex,
        theme::display(17.0 * c.scale),
        theme::TXT,
    );

    section_label(c, c.p(900.0, 750.0), "INTENSITY");
    value_text(
        c,
        c.p(1330.0, 742.0),
        &format!("{}%", (controls.val * 100.0).round() as u8),
        24.0,
        theme::TXT_DIM,
    );
    let mut val = controls.val;
    let out = slider(
        c,
        "color-value",
        Rect::from_min_max(c.p(900.0, 790.0), c.p(1360.0, 810.0)),
        &mut val,
        0.02,
        1.0,
        color,
        true,
    );
    if out.changed {
        controls.val = val;
        controls.touch();
        changed = true;
    }

    let presets = [
        ([255u8, 255, 255], "WHITE"),
        ([255, 40, 60], "RED"),
        ([255, 150, 30], "AMBER"),
        ([70, 255, 120], "GREEN"),
        ([60, 180, 255], "CYAN"),
        ([120, 80, 255], "BLUE"),
        ([255, 50, 210], "MAGENTA"),
    ];
    for (i, (preset, _)) in presets.iter().enumerate() {
        let x = 840.0 + i as f32 * 72.0;
        let center = c.p(x, 870.0);
        c.painter().circle_filled(
            center,
            15.0 * c.scale,
            Color32::from_rgb(preset[0], preset[1], preset[2]),
        );
        let resp = c.interact(&format!("preset-{i}"), c.r(x - 18.0, 852.0, 36.0, 36.0), Sense::click());
        if resp.hovered() {
            c.painter()
                .circle_stroke(center, 18.0 * c.scale, egui::Stroke::new(1.5, Color32::WHITE));
        }
        if resp.clicked() {
            let (h, s, v) = rgb_to_hsv(*preset);
            controls.hue = h;
            controls.sat = s;
            controls.val = v.max(0.35);
            controls.touch();
            changed = true;
        }
    }

    if changed {
        actions.push(Action::Color(current_rgb(controls), response.drag_stopped() || response.clicked()));
    }

    c.text_left(
        c.p(200.0, 976.0),
        "Color writes require the updated root service; brightness works with the installed daemon.",
        theme::sans(12.0 * c.scale),
        theme::TXT_FAINT,
    );
}

pub fn gpu(c: &mut Canvas, actions: &mut Vec<Action>, controls: &mut Controls) {
    let back = c.r(72.0, 136.0, 173.0, 173.0);
    if back_button(c, back, "back-gpu") {
        actions.push(Action::Go(Page::Home));
    }
    value_text(c, c.p(300.0, 286.0), "GRAPHICS", 41.0, theme::TXT);
    subtitle(
        c,
        c.p(303.0, 341.0),
        "Adapters, driver state and display output.",
    );

    let nv = c.r(200.0, 396.0, 1200.0, 248.0);
    c.card(nv, Some("DISCRETE"));
    c.image_at("gpu_card", 232.0, 454.0, 160.0, 135.0, Color32::WHITE);
    value_text(
        c,
        c.p(430.0, 462.0),
        "NVIDIA GEFORCE RTX 3050",
        33.0,
        theme::TXT,
    );
    subtitle(
        c,
        c.p(432.0, 510.0),
        "GA107BM · 6 GB GDDR6 · PCI 0000:01:00.0",
    );
    let mode = std::fs::read_to_string("/sys/bus/pci/devices/0000:01:00.0/power/runtime_status")
        .ok()
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".into());
    kv(c, c.p(432.0, 556.0), "Runtime", &mode);
    kv(c, c.p(432.0, 584.0), "Driver", "nvidia (proprietary)");
    widgets::pill(c, c.r(1190.0, 420.0, 170.0, 27.0), "HYBRID MODE", theme::GREEN);

    let intel = c.r(200.0, 680.0, 1200.0, 248.0);
    c.card(intel, Some("INTEGRATED"));
    c.image_at("cpu", 240.0, 742.0, 130.0, 130.0, Color32::WHITE);
    value_text(
        c,
        c.p(430.0, 746.0),
        "INTEL UHD GRAPHICS",
        33.0,
        theme::TXT,
    );
    subtitle(
        c,
        c.p(432.0, 794.0),
        "Raptor Lake-P · shared memory · PCI 0000:00:02.0",
    );
    kv(c, c.p(432.0, 840.0), "Driver", "i915");
    kv(c, c.p(432.0, 868.0), "Display", "1920 × 1080 · 60 Hz");

    c.text_left(
        c.p(200.0, 958.0),
        "Hybrid graphics are managed by the firmware. Acer Control Center reports state only.",
        theme::sans(12.5 * c.scale),
        theme::TXT_FAINT,
    );

    let _ = controls;
}

pub fn flexikey(c: &mut Canvas, actions: &mut Vec<Action>) {
    let back = c.r(72.0, 136.0, 173.0, 173.0);
    if back_button(c, back, "back-flexikey") {
        actions.push(Action::Go(Page::Home));
    }
    value_text(c, c.p(300.0, 286.0), "FLEXIKEY", 41.0, theme::TXT);
    subtitle(
        c,
        c.p(303.0, 341.0),
        "Assign shortcuts and macros to dedicated keys.",
    );

    let card = c.r(200.0, 396.0, 1200.0, 532.0);
    c.card(card, None);
    theme::glow_circle(
        &c.painter(),
        c.p(800.0, 560.0),
        120.0 * c.scale,
        theme::MAGENTA,
    );
    c.image_at("tile_flexikey", 660.0, 420.0, 280.0, 280.0, Color32::WHITE);
    centered_text(c, c.p(800.0, 758.0), "FLEXIKEY", 30.0, theme::TXT);
    centered_text(
        c,
        c.p(800.0, 800.0),
        "No compatible Flexikey device was detected on this system.",
        16.0,
        theme::TXT_DIM,
    );
    centered_text(
        c,
        c.p(800.0, 828.0),
        "Standard F-key behaviour stays available without the Acer utility.",
        14.0,
        theme::TXT_FAINT,
    );
    let button = c.r(690.0, 862.0, 220.0, 40.0);
    c.painter().rect_stroke(
        button,
        egui::CornerRadius::from(4.0),
        egui::Stroke::new(1.0, alpha(theme::TXT_FAINT, 130)),
        egui::StrokeKind::Inside,
    );
    centered_text(c, button.center(), "DEVICE NOT FOUND", 12.5, theme::TXT_FAINT);
}
