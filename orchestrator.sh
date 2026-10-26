#!/bin/sh
# Auto-select backend for Acer Control Center
# Prefer Wayland if available; otherwise fallback to X11.
if [ -n "$WAYLAND_DISPLAY" ] && [ -z "$DISPLAY" ]; then
    export WINIT_UNIX_BACKEND=wayland
elif [ -n "$DISPLAY" ]; then
    export WINIT_UNIX_BACKEND=x11
fi
exec /usr/local/bin/acer-control-center-gui-bin "$@"
