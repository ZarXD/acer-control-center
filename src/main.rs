use std::fs;
use zbus::{dbus_interface, ConnectionBuilder};

const KBD_PATH: &str = "/sys/class/leds/rgb:kbd_backlight/brightness";
const FAN_PATH: &str = "/sys/devices/platform/clevo_fan/fan_speed";
const PROF_PATH: &str = "/sys/devices/platform/clevo_power/profile";
const BAT_PATH: &str = "/sys/class/power_supply/BAT0/capacity";

fn read_u8(p: &str) -> u8 {
    fs::read_to_string(p).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}
fn write_u8(p: &str, v: u8) { let _ = fs::write(p, v.to_string()); }
fn read_u16(p: &str) -> u16 {
    fs::read_to_string(p).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}
fn write_u16(p: &str, v: u16) { let _ = fs::write(p, v.to_string()); }

struct ControlImpl;

#[dbus_interface(name = "com.acer.ControlCenter")]
impl ControlImpl {
    fn set_backlight(&self, val: u8) { write_u8(KBD_PATH, val); }
    fn get_backlight(&self) -> u8 { read_u8(KBD_PATH) }
    fn set_fan(&self, rpm: u16) { write_u16(FAN_PATH, rpm); }
    fn get_fan(&self) -> u16 { read_u16(FAN_PATH) }
    fn set_profile(&self, id: u8) { write_u8(PROF_PATH, id); }
    fn get_profile(&self) -> u8 { read_u8(PROF_PATH) }
    fn get_battery(&self) -> u8 { read_u8(BAT_PATH) }
}

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let _conn = ConnectionBuilder::system()?
        .name("com.acer.ControlCenter")?
        .serve_at("/com/acer/ControlCenter", ControlImpl)?
        .build()
        .await?;
    loop { tokio::time::sleep(std::time::Duration::from_secs(60)).await; }
}
