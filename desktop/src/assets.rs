use egui::{ColorImage, Context, TextureFilter, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Assets {
    map: HashMap<String, TextureHandle>,
}

impl Assets {
    pub fn load(ctx: &Context) -> Self {
        let opts = TextureOptions {
            magnification: TextureFilter::Linear,
            minification: TextureFilter::Linear,
            wrap_mode: egui::TextureWrapMode::ClampToEdge,
            mipmap_mode: Some(TextureFilter::Linear),
        };
        let mut map = HashMap::new();
        if let Some(dir) = asset_dir() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("png") {
                        continue;
                    }
                    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                        continue;
                    };
                    let Ok(bytes) = std::fs::read(&path) else {
                        continue;
                    };
                    let Ok(image) = image::load_from_memory(&bytes) else {
                        continue;
                    };
                    let image = image.to_rgba8();
                    let size = [image.width() as usize, image.height() as usize];
                    let color = ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                    let texture =
                        ctx.load_texture(format!("acer_cc_{stem}"), color, opts);
                    map.insert(stem.to_string(), texture);
                }
            }
        }
        Self { map }
    }

    pub fn get(&self, name: &str) -> Option<&TextureHandle> {
        self.map.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }
}

pub fn asset_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("ACER_CC_ASSETS") {
        candidates.push(PathBuf::from(dir));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("assets/ui"));
            candidates.push(dir.join("../assets/ui"));
            candidates.push(dir.join("../../assets/ui"));
        }
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/ui"));
    if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
        candidates.push(PathBuf::from(data).join("acer-control-center/assets/ui"));
    } else if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/share/acer-control-center/assets/ui"));
    }
    candidates.push(PathBuf::from(
        "/usr/local/share/acer-control-center/assets/ui",
    ));
    candidates.into_iter().find(|p| p.is_dir())
}

pub fn font_dir() -> Option<PathBuf> {
    asset_dir()?.parent().map(|p| p.join("fonts"))
}
