use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

const SERVICE: &str = "com.acer.ControlCenter";
const PATH: &str = "/com/acer/ControlCenter";

#[derive(Clone, Copy, Debug, Default)]
pub struct Fan {
    pub speed: u8,
    pub temp: i16,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub online: bool,
    pub profile: u8,
    pub backlight: u8,
    pub battery: u8,
    pub battery_status: String,
    pub charge_limit: u8,
    pub charge_start: u8,
    pub color: [u8; 3],
    pub fans: [Fan; 3],
    pub error: Option<String>,
    pub error_seq: u64,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            online: false,
            profile: 2,
            backlight: 0,
            battery: 0,
            battery_status: String::new(),
            charge_limit: 80,
            charge_start: 70,
            color: [255, 255, 255],
            fans: [Fan::default(); 3],
            error: None,
            error_seq: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Profile(u8),
    Backlight(u8),
    FanSpeed(u8, u8),
    FanAuto(bool),
    Color([u8; 3]),
    ChargeLimit(u8),
    Refresh,
}

pub struct Backend {
    tx: Sender<Command>,
    rx: Receiver<Snapshot>,
    latest: Snapshot,
}

impl Backend {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (state_tx, state_rx) = channel();
        std::thread::Builder::new()
            .name("acer-cc-hardware".into())
            .spawn(move || worker(cmd_rx, state_tx))
            .expect("hardware worker");
        Self {
            tx: cmd_tx,
            rx: state_rx,
            latest: Snapshot::default(),
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        while let Ok(state) = self.rx.try_recv() {
            self.latest = state;
            updated = true;
        }
        updated
    }

    pub fn state(&self) -> &Snapshot {
        &self.latest
    }

    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }
}

#[zbus::proxy(
    interface = "com.acer.ControlCenter",
    default_service = "com.acer.ControlCenter",
    default_path = "/com/acer/ControlCenter"
)]
trait ControlCenter {
    fn get_profile(&self) -> zbus::Result<u8>;
    fn set_profile(&self, id: u8) -> zbus::Result<bool>;
    fn get_backlight(&self) -> zbus::Result<u8>;
    fn set_backlight(&self, value: u8) -> zbus::Result<bool>;
    fn get_fan_speed(&self, fan: u8) -> zbus::Result<u8>;
    fn get_fan_temp(&self, fan: u8) -> zbus::Result<i16>;
    fn set_fan_speed(&self, fan: u8, percent: u8) -> zbus::Result<bool>;
    fn set_fan_auto(&self) -> zbus::Result<bool>;
    fn set_fan_auto_mode(&self, enabled: bool) -> zbus::Result<bool>;
    fn get_color(&self) -> zbus::Result<String>;
    fn set_color(&self, red: u8, green: u8, blue: u8) -> zbus::Result<bool>;
    fn get_charge_limit(&self) -> zbus::Result<u8>;
    fn set_charge_limit(&self, value: u8) -> zbus::Result<bool>;
}

fn connect() -> Option<ControlCenterProxyBlocking<'static>> {
    let connection = zbus::blocking::Connection::system().ok()?;
    ControlCenterProxyBlocking::new(&connection).ok()
}

fn worker(cmd_rx: Receiver<Command>, state_tx: Sender<Snapshot>) {
    let mut snapshot = Snapshot::default();
    let mut proxy = connect();
    let mut next_reconnect = std::time::Instant::now() + Duration::from_secs(2);
    let mut seq = 0u64;

    loop {
        while let Ok(command) = cmd_rx.try_recv() {
            run_command(&mut proxy, &mut snapshot, &mut seq, command);
        }

        // Values that do not require the daemon.
        snapshot.battery = read_u8("/sys/class/power_supply/BAT0/capacity").unwrap_or(snapshot.battery);
        snapshot.battery_status = read_string("/sys/class/power_supply/BAT0/status")
            .unwrap_or_else(|| snapshot.battery_status.clone());
        snapshot.charge_limit = read_u8("/sys/class/power_supply/BAT0/charge_control_end_threshold")
            .unwrap_or(snapshot.charge_limit);
        snapshot.charge_start = read_u8("/sys/class/power_supply/BAT0/charge_control_start_threshold")
            .unwrap_or(snapshot.charge_start);
        snapshot.backlight = read_u8("/sys/class/leds/rgb:kbd_backlight/brightness")
            .unwrap_or(snapshot.backlight);
        if let Some(color) = read_rgb("/sys/class/leds/rgb:kbd_backlight/multi_intensity") {
            snapshot.color = color;
        }

        if proxy.is_none() && std::time::Instant::now() >= next_reconnect {
            proxy = connect();
            next_reconnect = std::time::Instant::now() + Duration::from_secs(2);
        }

        let mut lost_service = false;
        if let Some(p) = proxy.as_ref() {
            match p.get_profile() {
                Ok(value) => snapshot.profile = value,
                Err(error) => {
                    let text = error.to_string();
                    if text.contains("ServiceUnknown")
                        || text.contains("NameHasNoOwner")
                        || text.contains("not provided by any .service files")
                    {
                        lost_service = true;
                    }
                }
            }
            if !lost_service {
                if let Ok(value) = p.get_backlight() {
                    snapshot.backlight = value;
                }
                for i in 0..3u8 {
                    if let Ok(speed) = p.get_fan_speed(i) {
                        snapshot.fans[i as usize].speed = speed;
                    }
                    if let Ok(temp) = p.get_fan_temp(i) {
                        snapshot.fans[i as usize].temp = temp;
                    }
                }
                // Newer daemon methods: keep failures silent during polling.
                if let Ok(value) = p.get_charge_limit() {
                    if value > 0 {
                        snapshot.charge_limit = value;
                    }
                }
                if let Ok(text) = p.get_color() {
                    let parts: Vec<u8> = text
                        .split_whitespace()
                        .filter_map(|v| v.parse().ok())
                        .collect();
                    if parts.len() == 3 {
                        snapshot.color = [parts[0], parts[1], parts[2]];
                    }
                }
            }
        } else {
            lost_service = true;
        }
        if lost_service {
            proxy = None;
            snapshot.online = false;
        } else {
            snapshot.online = true;
        }

        if state_tx.send(snapshot.clone()).is_err() {
            break;
        }
        match cmd_rx.recv_timeout(Duration::from_millis(650)) {
            Ok(command) => run_command(&mut proxy, &mut snapshot, &mut seq, command),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn record_error(snapshot: &mut Snapshot, seq: &mut u64, text: String) {
    *seq += 1;
    snapshot.error = Some(text);
    snapshot.error_seq = *seq;
}

fn run_command(
    proxy: &mut Option<ControlCenterProxyBlocking<'static>>,
    snapshot: &mut Snapshot,
    seq: &mut u64,
    command: Command,
) {
    let result = {
        let Some(p) = proxy.as_ref() else {
            snapshot.online = false;
            record_error(
                snapshot,
                seq,
                "Hardware service is offline. Run the install script and retry.".into(),
            );
            return;
        };
        let result: Result<bool, zbus::Error> = match command {
            Command::Profile(id) => p.set_profile(id),
            Command::Backlight(value) => p.set_backlight(value),
            Command::FanSpeed(fan, percent) => p.set_fan_speed(fan, percent),
            Command::FanAuto(enabled) => p.set_fan_auto_mode(enabled),
            Command::Color([r, g, b]) => p.set_color(r, g, b),
            Command::ChargeLimit(value) => p.set_charge_limit(value),
            Command::Refresh => Ok(true),
        };
        result
    };
    match result {
        Ok(true) => {}
        Ok(false) => record_error(snapshot, seq, "The hardware rejected that change.".into()),
        Err(error) => {
            let text = error.to_string();
            if text.contains("UnknownMethod") || text.contains("Unknown method") {
                record_error(
                    snapshot,
                    seq,
                    "This control needs the updated service. Re-run the install step.".into(),
                );
            } else if text.contains("ServiceUnknown")
                || text.contains("NameHasNoOwner")
                || text.contains("not provided by any .service files")
            {
                snapshot.online = false;
                *proxy = None;
                record_error(
                    snapshot,
                    seq,
                    "Hardware service is offline. Start acer-control-center.service.".into(),
                );
            } else {
                record_error(snapshot, seq, text);
            }
        }
    }
}

fn read_string(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
}

fn read_u8(path: &str) -> Option<u8> {
    read_string(path)?.parse().ok()
}

fn read_rgb(path: &str) -> Option<[u8; 3]> {
    let text = read_string(path)?;
    let values: Vec<u8> = text
        .split_whitespace()
        .filter_map(|value| value.parse().ok())
        .collect();
    if values.len() == 3 {
        Some([values[0], values[1], values[2]])
    } else {
        None
    }
}

pub fn service_name() -> &'static str {
    SERVICE
}

pub fn object_path() -> &'static str {
    PATH
}
