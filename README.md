# Acer Control Center (Linux)

Linux replacement for Acer Control Center on Acer laptops with a Clevo ODM
board (tested on Aspire A715-59G / `Kelilu_RTH`).

It is a small root daemon that talks to the patched Clevo/TUXEDO kernel
driver and exposes the hardware through D-Bus and a local web UI.

## Features

- Keyboard backlight brightness (`rgb:kbd_backlight`)
- Fan speed per fan, with automatic (EC) mode
- Fan temperature readout
- Clevo ODM performance profiles: quiet, power saving, performance,
  entertainment
- Battery capacity and charging state
- Local web panel at <http://127.0.0.1:8787> with a desktop launcher

## Requirements

- Linux with the patched `tuxedo-drivers` DKMS modules loaded
  (`clevo_acpi`, `clevo_wmi`, `tuxedo_keyboard`, `tuxedo_io`)
- `/dev/tuxedo_io` present
- Rust (only when building from source)

The matching driver patch is
[ZarXD/tuxedo-driver-patch](https://github.com/ZarXD/tuxedo-driver-patch).

## Build

```bash
cargo build --release
```

## Install

```bash
sudo ./install.sh ./target/release/acer-control-center
```

The installer copies the binary to `/usr/local/bin`, registers the systemd
service, installs the desktop entry/icon and restarts the daemon.

Open the UI at <http://127.0.0.1:8787> or from the application menu
("Acer Control Center"). The web server binds to localhost only.

## Web API

| Method | Path | Body |
| --- | --- | --- |
| `GET` | `/api/status` | – |
| `POST` | `/api/backlight` | `{"value": 0-255}` |
| `POST` | `/api/fan` | `{"fan": 0-2, "percent": 0-100}` |
| `POST` | `/api/fan/auto` | – |
| `POST` | `/api/profile` | `{"id": 0-3}` |

`GET /api/status` returns battery, battery status, backlight range, all fan
speeds/temperatures and the active profile.

## D-Bus API

Service: `com.acer.ControlCenter`
Object: `/com/acer/ControlCenter`
Interface: `com.acer.ControlCenter`

Methods:

- `SetBacklight(u8) -> bool`
- `GetBacklight() -> u8`
- `SetFan(u16) -> bool` (fan 0, percent)
- `GetFan() -> u16` (fan 0, percent)
- `SetFanSpeed(u8 fan, u8 percent) -> bool`
- `GetFanSpeed(u8 fan) -> u8`
- `GetFanTemp(u8 fan) -> i16`
- `SetFanAuto() -> bool`
- `SetProfile(u8) -> bool`
- `GetProfile() -> u8`
- `GetBattery() -> u8`

Example:

```bash
gdbus call --system --dest com.acer.ControlCenter \
  --object-path /com/acer/ControlCenter \
  --method com.acer.ControlCenter.GetFanSpeed 0
```

## Notes

- Fan writes go straight to the embedded controller, like TUXEDO Control
  Center. Use **Auto Fan** to return control to the firmware.
- The active performance profile is remembered in
  `/var/lib/acer-control-center/profile`.
- The daemon logs its HTTP address to the journal; set `ACC_WEB_ADDR` to
  change the listen address and `ACC_WEB_ONLY=1` to run the UI without D-Bus
  (useful for local development).
