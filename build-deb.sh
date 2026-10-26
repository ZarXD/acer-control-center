#!/bin/bash
set -e
rm -rf build
mkdir -p build/DEBIAN
mkdir -p build/usr/local/bin
mkdir -p build/usr/local/share/acer-control-center/assets/ui

# Extract version from root Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

# Include orchestrator script
cp orchestrator.sh build/usr/local/bin/acer-control-center

cp -r debian/DEBIAN/* build/DEBIAN/
chmod +x build/DEBIAN/postinst build/DEBIAN/prerm

cp desktop/target/release/acer-control-center-gui build/usr/local/bin/acer-control-center-gui-bin
cp -r desktop/assets/ui/* build/usr/local/share/acer-control-center/assets/ui/

chmod +x build/usr/local/bin/acer-control-center

dpkg-deb --root-owner-group --build build acer-control-center_${VERSION}_amd64.deb
echo "created acer-control-center_${VERSION}_amd64.deb"
