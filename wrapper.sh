#!/bin/sh
# Require Wayland session; no X11 fallback.
if [ -n "$WAYLAND_DISPLAY" ]; then
    export WINIT_UNIX_BACKEND=wayland
else
    echo "Acer Control Center GUI requires a Wayland session (WAYLAND_DISPLAY not set)" >&2
    exit 1
fi
exec /usr/local/bin/acer-control-center-gui "$@"