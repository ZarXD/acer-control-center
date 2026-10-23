#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_SRC="${1:-./target/release/acer-control-center}"

if [[ ! -f "$BIN_SRC" ]]; then
    echo "binary not found: $BIN_SRC" >&2
    exit 1
fi

sudo install -m 0755 "$BIN_SRC" /usr/local/bin/acer-control-center

sudo tee /etc/dbus-1/system.d/com.acer.ControlCenter.conf >/dev/null <<'POLICY'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="root">
    <allow own="com.acer.ControlCenter"/>
  </policy>
  <policy context="default">
    <allow send_destination="com.acer.ControlCenter"/>
  </policy>
</busconfig>
POLICY

sudo tee /etc/systemd/system/acer-control-center.service >/dev/null <<'SERVICE'
[Unit]
Description=Acer Control Center daemon
After=dbus.service
Requires=dbus.service

[Service]
Type=simple
ExecStart=/usr/local/bin/acer-control-center
Restart=on-failure
RestartSec=3

[Install]
WantedBy=multi-user.target
SERVICE

if [[ -f "$SCRIPT_DIR/icon.svg" ]]; then
    sudo install -Dm 0644 "$SCRIPT_DIR/icon.svg" \
        /usr/share/icons/hicolor/scalable/apps/acer-control-center.svg
fi

sudo tee /usr/share/applications/acer-control-center.desktop >/dev/null <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Acer Control Center
Comment=Fan, performance profile and keyboard backlight control
Exec=xdg-open http://127.0.0.1:8787
Icon=acer-control-center
Terminal=false
Categories=System;Settings;HardwareSettings;
Keywords=acer;fan;backlight;performance;profile;
DESKTOP

sudo systemctl daemon-reload
sudo systemctl enable acer-control-center.service
sudo systemctl restart acer-control-center.service

echo "installed. UI: http://127.0.0.1:8787"
echo "check: systemctl status acer-control-center"
