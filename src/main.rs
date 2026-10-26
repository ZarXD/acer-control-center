use std::fs;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicU8, Ordering};
use zbus::{dbus_interface, ConnectionBuilder};

const KBD_PATH: &str = "/sys/class/leds/rgb:kbd_backlight/brightness";
const KBD_MAX_PATH: &str = "/sys/class/leds/rgb:kbd_backlight/max_brightness";
const KBD_RGB_PATH: &str = "/sys/class/leds/rgb:kbd_backlight/multi_intensity";
const BAT_PATH: &str = "/sys/class/power_supply/BAT0/capacity";
const BAT_STATUS_PATH: &str = "/sys/class/power_supply/BAT0/status";
const CHARGE_LIMIT_PATH: &str = "/sys/class/power_supply/BAT0/charge_control_end_threshold";
const CHARGE_START_PATH: &str = "/sys/class/power_supply/BAT0/charge_control_start_threshold";
const CHARGE_START_VALUES_PATH: &str = "/sys/class/power_supply/BAT0/charge_control_start_available_thresholds";
const CHARGE_TYPE_PATH: &str = "/sys/class/power_supply/BAT0/charge_type";
const STATE_DIR: &str = "/var/lib/acer-control-center";
const PROFILE_PATH: &str = "/var/lib/acer-control-center/profile";

// Linux ioctl encoding (x86_64): _IOC(dir, type, nr, size)
const IOC_READ: u64 = 2;
const IOC_WRITE: u64 = 1;
const PTR_SIZE: u64 = 8;
const MAGIC_READ_CL: u64 = 0xEC + 1;
const MAGIC_WRITE_CL: u64 = 0xEC + 2;

const fn ioc(dir: u64, ty: u64, nr: u64, size: u64) -> u64 {
    (dir << 30) | (ty << 8) | nr | (size << 16)
}

const R_CL_FANINFO1: u64 = ioc(IOC_READ, MAGIC_READ_CL, 0x10, PTR_SIZE);
const R_CL_FANINFO2: u64 = ioc(IOC_READ, MAGIC_READ_CL, 0x11, PTR_SIZE);
const R_CL_FANINFO3: u64 = ioc(IOC_READ, MAGIC_READ_CL, 0x12, PTR_SIZE);
const W_CL_FANSPEED: u64 = ioc(IOC_WRITE, MAGIC_WRITE_CL, 0x10, PTR_SIZE);
const W_CL_FANAUTO: u64 = ioc(IOC_WRITE, MAGIC_WRITE_CL, 0x11, PTR_SIZE);
const W_CL_PERF_PROFILE: u64 = ioc(IOC_WRITE, MAGIC_WRITE_CL, 0x15, PTR_SIZE);

const FAN_COUNT: u8 = 3;

static PROFILE: AtomicU8 = AtomicU8::new(0);

fn read_u8(p: &str) -> u8 {
    fs::read_to_string(p)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn read_string(p: &str) -> String {
    fs::read_to_string(p)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string())
}

fn write_u8(p: &str, v: u8) -> bool {
    fs::write(p, v.to_string()).is_ok()
}

fn load_profile() -> u8 {
    fs::read_to_string(PROFILE_PATH)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|v| *v <= 3)
        .unwrap_or(0)
}

fn save_profile(id: u8) {
    let _ = fs::create_dir_all(STATE_DIR);
    let _ = fs::write(PROFILE_PATH, id.to_string());
}

fn tuxedo_io() -> Option<std::fs::File> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tuxedo_io")
        .ok()
}

fn fan_info(fan: u8) -> Option<i32> {
    let fd = tuxedo_io()?;
    let req = match fan {
        0 => R_CL_FANINFO1,
        1 => R_CL_FANINFO2,
        2 => R_CL_FANINFO3,
        _ => return None,
    };
    let mut val: i32 = 0;
    let ret = unsafe { libc::ioctl(fd.as_raw_fd(), req as libc::c_ulong, &mut val) };
    if ret < 0 { None } else { Some(val) }
}

fn fan_percent(fan: u8) -> u8 {
    match fan_info(fan) {
        Some(info) => {
            let raw = (info & 0xff) as f32;
            ((raw / 255.0) * 100.0).round().clamp(0.0, 100.0) as u8
        }
        None => 0,
    }
}

/// Temperature reported by the EC. Values <= 1 mean "fan not present".
fn fan_temp(fan: u8) -> Option<i16> {
    let info = fan_info(fan)?;
    let temp = (((info >> 16) & 0xff) as u8 as i8) as i16;
    if temp <= 1 { None } else { Some(temp) }
}

fn set_fan_percent(fan: u8, percent: u8) -> bool {
    if fan >= FAN_COUNT || percent > 100 {
        return false;
    }
    let mut raw = [0i32; 3];
    for (i, slot) in raw.iter_mut().enumerate() {
        match fan_info(i as u8) {
            Some(info) => *slot = info & 0xff,
            None => return false,
        }
    }
    raw[fan as usize] = ((percent as f32) * 255.0 / 100.0).round() as i32;
    let mut argument = raw[0] | (raw[1] << 8) | (raw[2] << 16);
    let fd = match tuxedo_io() {
        Some(fd) => fd,
        None => return false,
    };
    let ret = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            W_CL_FANSPEED as libc::c_ulong,
            &mut argument,
        )
    };
    ret >= 0
}

/// Hand fan control back to the embedded controller firmware, or take it back.
fn set_fan_auto_mode(enabled: bool) -> bool {
    let fd = match tuxedo_io() {
        Some(fd) => fd,
        None => return false,
    };
    let mut argument: i32 = if enabled { 0x0f } else { 0x00 };
    let ret = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            W_CL_FANAUTO as libc::c_ulong,
            &mut argument,
        )
    };
    ret >= 0
}

fn set_fan_auto() -> bool {
    set_fan_auto_mode(true)
}

fn get_color() -> String {
    fs::read_to_string(KBD_RGB_PATH)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn set_color(red: u8, green: u8, blue: u8) -> bool {
    fs::write(KBD_RGB_PATH, format!("{red} {green} {blue}")).is_ok()
}

fn set_charge_limit(value: u8) -> bool {
    if !(50..=100).contains(&value) {
        return false;
    }
    if value >= 100 {
        let _ = fs::write(CHARGE_TYPE_PATH, "Standard");
        let start_ok = write_u8(CHARGE_START_PATH, 95);
        let end_ok = write_u8(CHARGE_LIMIT_PATH, 100);
        return start_ok && end_ok;
    }

    // The EC expects a start threshold below the end threshold.
    let mut start = 40u8;
    if let Ok(text) = fs::read_to_string(CHARGE_START_VALUES_PATH) {
        for candidate in text.split_whitespace().filter_map(|v| v.parse::<u8>().ok()) {
            if candidate < value && candidate > start {
                start = candidate;
            }
        }
    }
    if start >= value {
        start = value.saturating_sub(10).max(1);
    }

    let custom_ok = fs::write(CHARGE_TYPE_PATH, "Custom").is_ok();
    let start_ok = write_u8(CHARGE_START_PATH, start);
    let end_ok = write_u8(CHARGE_LIMIT_PATH, value);
    custom_ok && start_ok && end_ok
}

fn set_profile(id: u8) -> bool {
    if id > 3 {
        return false;
    }
    let fd = match tuxedo_io() {
        Some(fd) => fd,
        None => return false,
    };
    let mut argument: i32 = id as i32;
    let ret = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            W_CL_PERF_PROFILE as libc::c_ulong,
            &mut argument,
        )
    };
    if ret >= 0 {
        PROFILE.store(id, Ordering::Relaxed);
        save_profile(id);
        true
    } else {
        false
    }
}

struct ControlImpl;

#[dbus_interface(name = "com.acer.ControlCenter")]
impl ControlImpl {
    fn set_backlight(&self, val: u8) -> bool {
        write_u8(KBD_PATH, val)
    }
    fn get_backlight(&self) -> u8 {
        read_u8(KBD_PATH)
    }

    /// Backwards-compatible single-fan API, now in percent.
    fn set_fan(&self, percent: u16) -> bool {
        set_fan_percent(0, percent.min(100) as u8)
    }
    fn get_fan(&self) -> u16 {
        fan_percent(0) as u16
    }

    fn set_fan_speed(&self, fan: u8, percent: u8) -> bool {
        set_fan_percent(fan, percent)
    }
    fn get_fan_speed(&self, fan: u8) -> u8 {
        fan_percent(fan)
    }
    fn get_fan_temp(&self, fan: u8) -> i16 {
        fan_temp(fan).unwrap_or(0)
    }
    fn set_fan_auto(&self) -> bool {
        set_fan_auto()
    }

    fn set_fan_auto_mode(&self, enabled: bool) -> bool {
        set_fan_auto_mode(enabled)
    }

    fn get_color(&self) -> String {
        get_color()
    }

    fn set_color(&self, red: u8, green: u8, blue: u8) -> bool {
        set_color(red, green, blue)
    }

    fn get_charge_limit(&self) -> u8 {
        read_u8(CHARGE_LIMIT_PATH)
    }

    fn set_charge_limit(&self, value: u8) -> bool {
        set_charge_limit(value)
    }

    fn set_profile(&self, id: u8) -> bool {
        set_profile(id)
    }
    fn get_profile(&self) -> u8 {
        PROFILE.load(Ordering::Relaxed)
    }

    fn get_battery(&self) -> u8 {
        read_u8(BAT_PATH)
    }
}

// ---- Web UI ----
use axum::{
    extract::Json,
    response::Html,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct FanStatus {
    speed: u8,
    temp: Option<i16>,
}

#[derive(Serialize)]
struct Status {
    battery: u8,
    battery_status: String,
    backlight: u8,
    backlight_max: u8,
    fans: Vec<FanStatus>,
    profile: u8,
}

async fn ui_index() -> Html<&'static str> {
    Html(include_str!("ui.html"))
}

async fn api_status() -> Json<Status> {
    Json(Status {
        battery: read_u8(BAT_PATH),
        battery_status: read_string(BAT_STATUS_PATH),
        backlight: read_u8(KBD_PATH),
        backlight_max: read_u8(KBD_MAX_PATH).max(1),
        fans: (0..FAN_COUNT)
            .map(|i| FanStatus {
                speed: fan_percent(i),
                temp: fan_temp(i),
            })
            .collect(),
        profile: PROFILE.load(Ordering::Relaxed),
    })
}

#[derive(Deserialize)]
struct BacklightReq {
    value: u8,
}

async fn api_backlight(Json(req): Json<BacklightReq>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": write_u8(KBD_PATH, req.value), "value": req.value}))
}

#[derive(Deserialize)]
struct FanReq {
    fan: u8,
    percent: u8,
}

async fn api_fan(Json(req): Json<FanReq>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": set_fan_percent(req.fan, req.percent),
        "fan": req.fan,
        "percent": req.percent,
    }))
}

async fn api_fan_auto() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": set_fan_auto()}))
}

#[derive(Deserialize)]
struct ProfileReq {
    id: u8,
}

async fn api_profile(Json(req): Json<ProfileReq>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": set_profile(req.id), "id": req.id}))
}

async fn start_web() {
    let addr = std::env::var("ACC_WEB_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let app = Router::new()
        .route("/", get(ui_index))
        .route("/api/status", get(api_status))
        .route("/api/backlight", post(api_backlight))
        .route("/api/fan", post(api_fan))
        .route("/api/fan/auto", post(api_fan_auto))
        .route("/api/profile", post(api_profile));

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("web ui disabled ({addr}): {e}");
            return;
        }
    };
    println!("Acer Control Center UI: http://{addr}");
    let _ = axum::serve(listener, app).await;
}

#[tokio::main]
async fn main() {
    PROFILE.store(load_profile(), Ordering::Relaxed);
    tokio::spawn(start_web());

    if std::env::var("ACC_WEB_ONLY").is_ok() {
        eprintln!("running in web-only mode (no D-Bus)");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    match ConnectionBuilder::system()
        .and_then(|b| b.name("com.acer.ControlCenter"))
        .and_then(|b| b.serve_at("/com/acer/ControlCenter", ControlImpl))
    {
        Ok(builder) => match builder.build().await {
            Ok(_conn) => loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            },
            Err(e) => {
                eprintln!("D-Bus connection failed: {e}");
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            }
        },
        Err(e) => {
            eprintln!("D-Bus setup failed: {e}");
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        }
    }
}
