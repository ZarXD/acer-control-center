#!/usr/bin/env bash
set -euo pipefail

BIN_SRC="${1:-./target/release/acer-control-center}"

if [[ ! -f "$BIN_SRC" ]]; then
    echo "binary not found: $BIN_SRC" >&2
    exit 1
fi

sudo install -m 0755 "$BIN_SRC" /usr/local/bin/acer-control-center

sudo tee /etc/systemd/system/acer-control-center.service >/dev/null <<'SERVICE'
[Unit]
Description=Acer Control Center daemon
After=multi-user.target

[Service]
Type=simple
ExecStart=/usr/local/bin/acer-control-center
Restart=on-failure
RestartSec=3

[Install]
WantedBy=multi-user.target
SERVICE

sudo systemctl daemon-reload
sudo systemctl enable --now acer-control-center.service

echo "installed. check: systemctl status acer-control-center"
