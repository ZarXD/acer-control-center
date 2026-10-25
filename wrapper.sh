#!/bin/sh
# Auto-select backend: prefer Wayland even if X11 is also available
if [ -n "$WAYLAND_DISPLAY" ]; then
    export WINIT_UNIX_BACKEND=wayland
elif [ -n "$DISPLAY" ]; then
    export WINIT_UNIX_BACKEND=x11
fi
exec /usr/local/bin/acer-control-center-gui "$@"