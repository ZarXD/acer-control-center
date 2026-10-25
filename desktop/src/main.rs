mod app;
mod assets;
mod backend;
mod pages;
mod theme;
mod widgets;

use app::Launch;
use std::path::PathBuf;
use widgets::Page;

fn usage() {
    eprintln!(
        "acer-control-center-gui [--page home|power|fan|keyboard|gpu|flexikey] \
[--screenshot PATH] [--mode quiet|power_saving|performance|entertainment]"
    );
}

fn parse_page(value: &str) -> Option<Page> {
    match value.to_ascii_lowercase().as_str() {
        "home" => Some(Page::Home),
        "power" | "power-modes" => Some(Page::Power),
        "fan" | "thermal" => Some(Page::Fan),
        "keyboard" | "led" => Some(Page::Keyboard),
        "gpu" | "graphics" => Some(Page::Gpu),
        "flexikey" => Some(Page::Flexikey),
        _ => None,
    }
}

fn parse_mode(value: &str) -> Option<u8> {
    match value.to_ascii_lowercase().as_str() {
        "quiet" | "0" => Some(0),
        "power_saving" | "power-saving" | "1" => Some(1),
        "performance" | "2" => Some(2),
        "entertainment" | "3" => Some(3),
        _ => None,
    }
}

fn main() -> eframe::Result<()> {
    // Choose backend: prefer Wayland when available, otherwise X11 if a display exists
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        std::env::set_var("WINIT_UNIX_BACKEND", "wayland");
    } else if std::env::var_os("DISPLAY").is_some() {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    
    let mut launch = Launch {
        page: Page::Home,
        screenshot: None,
        mode: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--page" => {
                let Some(value) = args.next() else {
                    usage();
                    std::process::exit(2);
                };
                let Some(page) = parse_page(&value) else {
                    eprintln!("unknown page: {value}");
                    usage();
                    std::process::exit(2);
                };
                launch.page = page;
            }
            "--screenshot" => {
                let Some(value) = args.next() else {
                    usage();
                    std::process::exit(2);
                };
                launch.screenshot = Some(PathBuf::from(value));
            }
            "--mode" => {
                let Some(value) = args.next() else {
                    usage();
                    std::process::exit(2);
                };
                let Some(mode) = parse_mode(&value) else {
                    eprintln!("unknown mode: {value}");
                    usage();
                    std::process::exit(2);
                };
                launch.mode = Some(mode);
            }
            "-h" | "--help" => {
                usage();
                return Ok(());
            }
            other => {
                eprintln!("unknown argument: {other}");
                usage();
                std::process::exit(2);
            }
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([theme::WIN_W, theme::WIN_H])
            .with_min_inner_size([1200.0, 770.0])
            .with_decorations(false)
            .with_resizable(true)
            .with_transparent(false)
            .with_app_id("acer-control-center")
            .with_title("Acer Control Center"),
        persist_window: false,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    eframe::run_native(
        "acer-control-center",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, launch)))),
    )
}
