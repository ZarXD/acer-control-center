use crate::assets::Assets;
use crate::backend::{Backend, Command, Snapshot};
use crate::pages::{self, Controls};
use crate::theme::{self, alpha};
use crate::widgets::{icon_button, Action, Canvas, Page};
use egui::{Color32, Pos2, Rect, Vec2, ViewportCommand};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Launch {
    pub page: Page,
    pub screenshot: Option<PathBuf>,
    pub mode: Option<u8>,
}

struct Capture {
    path: PathBuf,
    frame: u64,
    requested: bool,
}

pub struct App {
    assets: Assets,
    backend: Backend,
    controls: Controls,
    page: Page,
    capture: Option<Capture>,
    mode_override: Option<u8>,
    optimistic_profile: Option<(u8, Instant)>,
    toast: Option<(String, Instant)>,
    last_error_seq: u64,
    last_backlight: Instant,
    last_fan: Instant,
    last_color: Instant,
    sized: bool,
    frames: u64,
    fixup_until: Instant,
    fixup_hard_until: Instant,
    fixup_done: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, launch: Launch) -> Self {
        
        theme::install(&cc.egui_ctx);
        let assets = Assets::load(&cc.egui_ctx);
        let backend = Backend::start();
        let mut app = Self {
            assets,
            backend,
            controls: Controls::default(),
            page: launch.page,
            capture: launch.screenshot.map(|path| Capture {
                path,
                frame: 0,
                requested: false,
            }),
            mode_override: launch.mode,
            optimistic_profile: None,
            toast: None,
            last_error_seq: 0,
            last_backlight: Instant::now() - Duration::from_secs(1),
            last_fan: Instant::now() - Duration::from_secs(1),
            last_color: Instant::now() - Duration::from_secs(1),
            sized: false,
            frames: 0,
            fixup_until: Instant::now() + Duration::from_millis(1500),
            fixup_hard_until: Instant::now() + Duration::from_secs(4),
            fixup_done: false,
        };
        if app.capture.is_some() {
            app.page = launch.page;
        }
        app
    }

    fn toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    fn handle_actions(&mut self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::Go(page) => self.page = page,
                Action::Profile(id) => {
                    self.optimistic_profile = Some((id, Instant::now()));
                    self.backend.send(Command::Profile(id));
                }
                Action::Backlight(value) => {
                    if self.last_backlight.elapsed() >= Duration::from_millis(55) {
                        self.last_backlight = Instant::now();
                        self.backend.send(Command::Backlight(value));
                    }
                }
                Action::FanSpeed(index, value) => {
                    if self.last_fan.elapsed() >= Duration::from_millis(85) {
                        self.last_fan = Instant::now();
                        self.backend.send(Command::FanSpeed(index, value));
                    }
                }
                Action::FanAuto(on) => {
                    self.backend.send(Command::FanAuto(on));
                }
                Action::Color(rgb, commit) => {
                    if commit || self.last_color.elapsed() >= Duration::from_millis(120) {
                        self.last_color = Instant::now();
                        self.backend.send(Command::Color(rgb));
                    }
                }
                Action::ChargeLimit(value) => {
                    self.backend.send(Command::ChargeLimit(value));
                }
                Action::Close => {
                    self.backend.send(Command::Refresh);
                }
                Action::Minimize => {}
            }
        }
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(capture) = self.capture.as_mut() else {
            return;
        };
        capture.frame += 1;
        ctx.request_repaint();
        let ppp = ctx.pixels_per_point();
        let area = ctx.content_rect();
        let physical = area.size() * ppp;
        if capture.frame % 20 == 0 {
            eprintln!(
                "capture frame {} size {:?} ppp {} requested {}",
                capture.frame, physical, ppp, capture.requested
            );
        }
        let ready = physical.x >= theme::WIN_W && physical.y >= theme::WIN_H;
        if !capture.requested && (ready && capture.frame >= 12 || capture.frame >= 180) {
            if !ready {
                eprintln!(
                    "capture fallback at {}x{} physical",
                    physical.x, physical.y
                );
            }
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
            capture.requested = true;
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = image {
            let [width, height] = image.size;
            let target_w = theme::WIN_W as usize;
            let target_h = theme::WIN_H as usize;
            let (crop_w, crop_h, x0, y0) = if width >= target_w && height >= target_h {
                (
                    target_w,
                    target_h,
                    (width - target_w) / 2,
                    (height - target_h) / 2,
                )
            } else {
                eprintln!(
                    "capture is {}x{}, smaller than the {}x{} design canvas",
                    width, height, target_w, target_h
                );
                (width, height, 0, 0)
            };
            let mut bytes = Vec::with_capacity(crop_w * crop_h * 4);
            for y in y0..y0 + crop_h {
                for x in x0..x0 + crop_w {
                    let pixel = image.pixels[y * width + x];
                    bytes.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), pixel.a()]);
                }
            }
            match image::save_buffer(
                &capture.path,
                &bytes,
                crop_w as u32,
                crop_h as u32,
                image::ColorType::Rgba8,
            ) {
                Ok(()) => eprintln!(
                    "screenshot saved to {} ({}x{}, from {}x{})",
                    capture.path.display(),
                    crop_w,
                    crop_h,
                    width,
                    height
                ),
                Err(error) => eprintln!("screenshot failed: {error}"),
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
            self.capture = None;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frames += 1;
        if self.frames % 60 == 0 {
            eprintln!("frame {} size {:?}", self.frames, ctx.content_rect().size());
        }
        if !self.sized {
            if self.capture.is_some() && std::env::var("ACER_CC_FULLSCREEN").as_deref() != Ok("0") {
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
            }
            self.sized = true;
        }
        self.window_fixup(&ctx);
        self.backend.poll();
        let state = self.backend.state().clone();
        if state.error_seq != self.last_error_seq {
            self.last_error_seq = state.error_seq;
            if let Some(error) = &state.error {
                let text = error.clone();
                self.toast(text);
            }
        }
        self.controls.sync(&state);
        if let Some((id, at)) = self.optimistic_profile {
            if state.profile == id || at.elapsed() > Duration::from_secs(4) {
                self.optimistic_profile = None;
            }
        }

        let profile = self
            .mode_override
            .or_else(|| self.optimistic_profile.map(|(id, _)| id))
            .unwrap_or(state.profile);
        let capture = self.capture.is_some();
        let page = self.page;
        let toast = self.toast.clone();
        let offline = !state.online;

        self.edge_resize(&ctx);
        if !capture {
            self.drag_window(&ctx);
        }

        let mut actions = Vec::new();
        let assets = &self.assets;
        let controls = &mut self.controls;
        {
            ui.set_clip_rect(ui.max_rect());
            let area = ui.max_rect();
            let mut c = if capture {
                Canvas::with_pixel_scale(ui, assets, area, Some(1.0))
            } else {
                Canvas::new(ui, assets, area)
            };
                c.interactive = !capture;
                let baked_power = page == Page::Power
                    && profile == 3
                    && std::env::var_os("ACER_CC_DYNAMIC_POWER").is_none();
                background(&c, page, baked_power);
                header(&mut c, &mut actions, page == Page::Home, baked_power);
                match page {
                    Page::Home => pages::home(&mut c, &mut actions),
                    Page::Power => pages::power(&mut c, &mut actions, profile, baked_power),
                    Page::Fan => pages::fan(&mut c, &mut actions, controls, &state),
                    Page::Keyboard => pages::keyboard(&mut c, &mut actions, controls),
                    Page::Gpu => pages::gpu(&mut c, &mut actions, controls),
                    Page::Flexikey => pages::flexikey(&mut c, &mut actions),
                }
                if offline {
                    let pill = c.r(627.0, 181.0, 346.0, 27.0);
                    c.painter().rect_filled(
                        pill,
                        egui::CornerRadius::from(14.0),
                        Color32::from_rgba_unmultiplied(52, 12, 22, 210),
                    );
                    c.painter().rect_stroke(
                        pill,
                        egui::CornerRadius::from(14.0),
                        egui::Stroke::new(1.0, alpha(theme::RED, 130)),
                        egui::StrokeKind::Inside,
                    );
                    c.text(
                        pill.center(),
                        "HARDWARE SERVICE OFFLINE",
                        theme::sans(12.0 * c.scale),
                        theme::RED,
                    );
                }
                if let Some((text, at)) = &toast {
                    if at.elapsed() < Duration::from_secs(5) {
                        let fade = (1.0 - at.elapsed().as_secs_f32() / 5.0).clamp(0.0, 1.0);
                        let rect = c.r(530.0, 955.0, 540.0, 36.0);
                        c.painter().rect_filled(
                            rect,
                            egui::CornerRadius::from(18.0),
                            Color32::from_rgba_unmultiplied(
                                18,
                                14,
                                20,
                                (220.0 * fade) as u8,
                            ),
                        );
                        c.text(
                            rect.center(),
                            text,
                            theme::sans(13.0 * c.scale),
                            alpha(theme::TXT, (235.0 * fade) as u8),
                        );
                    }
                }
        }

        self.handle_actions(actions);
        self.handle_screenshot(&ctx);
        if self.capture.is_some() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(240));
        }
    }
}

impl App {
    /// GNOME/Mutter auto-maximizes windows that nearly fill the work area.
    /// The window starts with a temporary max-size clamp so Mutter's restore
    /// geometry cannot overshoot the design canvas. During the first moments
    /// we undo the maximize and ask for the design size again; once the window
    /// settles the clamp is removed so the user can resize freely.
    fn window_fixup(&mut self, ctx: &egui::Context) {
        if self.capture.is_some() || self.fixup_done {
            return;
        }

        let now = Instant::now();
        let design = egui::vec2(theme::WIN_W, theme::WIN_H);
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        let size = ctx.content_rect().size();
        let correct = !maximized && (size - design).length() <= 2.0;

        if now < self.fixup_until {
            // Repaint continuously so a maximize configure is undone on the
            // very next frame instead of up to a quarter second later.
            ctx.request_repaint();
            if maximized {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(design));
            } else if !correct {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(design));
            }
            return;
        }

        if correct {
            // Startup is over: drop the temporary clamp so the window can be
            // resized larger again.
            ctx.send_viewport_cmd(ViewportCommand::MaxInnerSize(egui::vec2(
                10000.0, 10000.0,
            )));
            self.fixup_done = true;
            return;
        }

        if now >= self.fixup_hard_until {
            // Leave the clamp in place rather than risk an oversized window.
            self.fixup_done = true;
            return;
        }

        ctx.request_repaint();
        if maximized {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
        }
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(design));
    }

    fn drag_window(&self, ctx: &egui::Context) {
        let Some(pointer) = ctx.pointer_hover_pos() else {
            return;
        };
        let pressed = ctx.input(|input| input.pointer.primary_pressed());
        if !pressed {
            return;
        }
        let area = ctx.content_rect();
        if pointer.y > area.top() + 120.0 * scale_for(ctx) {
            return;
        }
        let close = Rect::from_min_size(
            Pos2::new(area.right() - 120.0 * scale_for(ctx), area.top()),
            Vec2::new(120.0, 100.0) * scale_for(ctx),
        );
        let minimize = Rect::from_min_size(
            Pos2::new(area.right() - 245.0 * scale_for(ctx), area.top()),
            Vec2::new(100.0, 100.0) * scale_for(ctx),
        );
        if close.contains(pointer) || minimize.contains(pointer) {
            return;
        }
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }

    fn edge_resize(&self, ctx: &egui::Context) {
        let Some(pointer) = ctx.pointer_hover_pos() else {
            return;
        };
        let area = ctx.content_rect();
        let edge = 7.0;
        let left = pointer.x - area.left() <= edge;
        let right = area.right() - pointer.x <= edge;
        let top = pointer.y - area.top() <= edge;
        let bottom = area.bottom() - pointer.y <= edge;
        if !(left || right || top || bottom) {
            return;
        }
        let direction = match (left, right, top, bottom) {
            (true, _, true, _) => egui::ResizeDirection::NorthWest,
            (_, true, true, _) => egui::ResizeDirection::NorthEast,
            (true, _, _, true) => egui::ResizeDirection::SouthWest,
            (_, true, _, true) => egui::ResizeDirection::SouthEast,
            (true, _, _, _) => egui::ResizeDirection::West,
            (_, true, _, _) => egui::ResizeDirection::East,
            (_, _, true, _) => egui::ResizeDirection::North,
            (_, _, _, true) => egui::ResizeDirection::South,
            _ => return,
        };
        match direction {
            egui::ResizeDirection::North | egui::ResizeDirection::South => {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical)
            }
            egui::ResizeDirection::East | egui::ResizeDirection::West => {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal)
            }
            egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe)
            }
            _ => ctx.set_cursor_icon(egui::CursorIcon::ResizeNeSw),
        }
        if ctx.input(|input| input.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
    }
}

fn scale_for(ctx: &egui::Context) -> f32 {
    let area = ctx.content_rect();
    (area.width() / theme::WIN_W).min(area.height() / theme::WIN_H)
}

fn background(c: &Canvas, page: Page, baked_power: bool) {
    let design = c.r(0.0, 0.0, theme::WIN_W, theme::WIN_H);
    c.painter().rect_filled(c.ui.max_rect(), 0, Color32::from_rgb(28, 28, 28));
    if page == Page::Home {
        if !c.image("bg_full", design, Color32::WHITE) {
            c.painter().rect_filled(design, 0, theme::PANEL);
        }
        c.image_at("home_full", 0.0, 0.0, 1594.0, 1027.0, Color32::WHITE);
    } else if baked_power {
        if !c.image("bg_base", design, Color32::WHITE) {
            c.painter().rect_filled(design, 0, theme::PANEL);
        }
        c.image_at("power_full", 0.0, 0.0, 1594.0, 1027.0, Color32::WHITE);
    } else {
        if !c.image("bg_base", design, Color32::WHITE) {
            c.painter().rect_filled(design, 0, theme::PANEL);
        }
        c.image_at("bg_main", 4.0, 12.0, 1599.0, theme::WIN_H, Color32::WHITE);
        c.image_at("edge_left", 0.0, 208.0, 6.0, 819.0, Color32::WHITE);
        c.image_at("edge_right", 1592.0, 208.0, 2.0, 811.0, Color32::WHITE);
        c.image_at("edge_bottom", 0.0, 1019.0, 1594.0, 8.0, Color32::WHITE);
    }
}

fn header(c: &mut Canvas, actions: &mut Vec<Action>, home: bool, baked_power: bool) {
    let baked = home || baked_power;
    if !baked {
        c.image_at("logo", 756.3, 44.5, 91.0, 91.0, Color32::WHITE);
        c.image_at("title", 527.0, 144.0, 623.0, 20.0, Color32::WHITE);
    }

    let close = c.r(1492.5, 39.5, 48.0, 48.0);
    if icon_button(c, close, "close", baked) {
        actions.push(Action::Close);
        c.ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
    let minimize = c.r(1369.5, 39.5, 48.0, 48.0);
    if icon_button(c, minimize, "minimize", baked) {
        c.ui
            .ctx()
            .send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}
