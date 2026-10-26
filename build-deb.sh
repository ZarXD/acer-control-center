#!/bin/bash
set -euo pipefail

GUI_BIN=desktop/target/release/acer-control-center-gui
DAEMON_BIN=target/release/acer-control-center

for bin in "$GUI_BIN" "$DAEMON_BIN"; do
    if [ ! -x "$bin" ]; then
        echo "missing release binary: $bin" >&2
        echo "run: cargo build --release && (cd desktop && cargo build --release)" >&2
        exit 1
    fi
done

rm -rf build
mkdir -p build/DEBIAN
mkdir -p build/usr/local/bin
mkdir -p build/usr/local/share/acer-control-center/assets/ui
mkdir -p build/usr/local/share/acer-control-center/assets/fonts
mkdir -p build/lib/systemd/system
mkdir -p build/etc/dbus-1/system.d
mkdir -p build/usr/share/applications
mkdir -p build/usr/share/icons/hicolor/scalable/apps

# Extract version from root Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

install -m 0755 orchestrator.sh build/usr/local/bin/acer-control-center
install -m 0755 "$GUI_BIN" build/usr/local/bin/acer-control-center-gui-bin
install -m 0755 "$DAEMON_BIN" build/usr/local/bin/acer-control-center-daemon

cp -r desktop/assets/ui/. build/usr/local/share/acer-control-center/assets/ui/
cp -r desktop/assets/fonts/. build/usr/local/share/acer-control-center/assets/fonts/

install -m 0644 packaging/acer-control-center.service build/lib/systemd/system/
install -m 0644 packaging/com.acer.ControlCenter.conf build/etc/dbus-1/system.d/
install -m 0644 packaging/acer-control-center.desktop build/usr/share/applications/
install -m 0644 icon.svg build/usr/share/icons/hicolor/scalable/apps/acer-control-center.svg

cp -r debian/DEBIAN/. build/DEBIAN/
sed -i "s/^Version: .*/Version: $VERSION/" build/DEBIAN/control
chmod 0755 build/DEBIAN/postinst build/DEBIAN/prerm

dpkg-deb --root-owner-group --build build "acer-control-center_${VERSION}_amd64.deb"
echo "created acer-control-center_${VERSION}_amd64.deb"
