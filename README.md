# Acer Control Center (Linux daemon)

Small DBus daemon that exposes hardware controls from the patched Clevo/TUXEDO kernel driver on Acer laptops (Clevo ODM).

## Features

- Keyboard backlight brightness
- Fan speed
- Power profile
- Battery capacity

## Requirements

- Linux with the patched `tuxedo-drivers` DKMS modules loaded
- Rust (only for building from source)

## Build

```bash
cargo build --release
```

Or use the prebuilt binary from GitHub Actions / Releases.

## Install

```bash
sudo ./install.sh ./target/release/acer-control-center
```

The installer copies the binary to `/usr/local/bin` and registers a systemd system service.

## DBus API

Service: `com.acer.ControlCenter`
Object: `/com/acer/ControlCenter`
Interface: `com.acer.ControlCenter`

Methods:

- `SetBacklight(u8)`
- `GetBacklight() -> u8`
- `SetFan(u16)`
- `GetFan() -> u16`
- `SetProfile(u8)`
- `GetProfile() -> u8`
- `GetBattery() -> u8`

Example:

```bash
gdbus call --system --dest com.acer.ControlCenter \
  --object-path /com/acer/ControlCenter \
  --method com.acer.ControlCenter.GetBattery
```
