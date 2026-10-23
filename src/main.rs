use std::fs;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicU8, Ordering};
use zbus::{dbus_interface, ConnectionBuilder};

const KBD_PATH: &str = "/sys/class/leds/rgb:kbd_backlight/brightness";
const BAT_PATH: &str = "/sys/class/power_supply/BAT0/capacity";

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

static PROFILE: AtomicU8 = AtomicU8::new(0);

fn read_u8(p: &str) -> u8 {
    fs::read_to_string(p)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn write_u8(p: &str, v: u8) {
    let _ = fs::write(p, v.to_string());
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

fn fan_temp(fan: u8) -> i16 {
    match fan_info(fan) {
        Some(info) => (((info >> 16) & 0xff) as u8 as i8) as i16,
        None => 0,
    }
}

fn set_fan_percent(fan: u8, percent: u8) -> bool {
    if fan > 2 || percent > 100 {
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

fn set_fan_auto() -> bool {
    let fd = match tuxedo_io() {
        Some(fd) => fd,
        None => return false,
    };
    let mut argument: i32 = 0x0f;
    let ret = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            W_CL_FANAUTO as libc::c_ulong,
            &mut argument,
        )
    };
    ret >= 0
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
        true
    } else {
        false
    }
}

struct ControlImpl;

#[dbus_interface(name = "com.acer.ControlCenter")]
impl ControlImpl {
    fn set_backlight(&self, val: u8) {
        write_u8(KBD_PATH, val);
    }
    fn get_backlight(&self) -> u8 {
        read_u8(KBD_PATH)
    }

    // Backwards-compatible single-fan API, now in percent.
    fn set_fan(&self, percent: u16) {
        set_fan_percent(0, percent.min(100) as u8);
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
        fan_temp(fan)
    }
    fn set_fan_auto(&self) -> bool {
        set_fan_auto()
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

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let _conn = ConnectionBuilder::system()?
        .name("com.acer.ControlCenter")?
        .serve_at("/com/acer/ControlCenter", ControlImpl)?
        .build()
        .await?;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
