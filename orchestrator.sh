#!/bin/sh
# Launch the native UI on Wayland when a live socket is available, otherwise
# let winit fall back to X11 via DISPLAY. winit 0.30+ no longer reads
# WINIT_UNIX_BACKEND, so clearing a stale WAYLAND_DISPLAY is the reliable path.
if [ -n "$WAYLAND_DISPLAY" ]; then
    runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    if [ ! -S "$WAYLAND_DISPLAY" ] && [ ! -S "$runtime/$WAYLAND_DISPLAY" ]; then
        unset WAYLAND_DISPLAY
    fi
fi
exec /usr/local/bin/acer-control-center-gui-bin "$@"
