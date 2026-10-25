#!/bin/sh
# Orchestrator: coba Wayland dulu, kalau gagal fallback X11 otomatis.
# Log lengkap tersimpan di /tmp/acer-cc-debug.log
BIN=/usr/local/bin/acer-control-center-gui
LOG=/tmp/acer-cc-debug.log

run() {
    # $1 = backend
    : > "$LOG"
    WINIT_UNIX_BACKEND="$1" WINIT_DEBUG=1 RUST_LOG=debug \
        timeout 6 "$BIN" --page home 2>&1 | tee -a "$LOG" >/dev/null
}

backend="x11"
if [ -n "$WAYLAND_DISPLAY" ]; then
    backend="wayland"
fi

run "$backend"

# Deteksi gagal Wayland: log error init / fallback web
if grep -qiE "wayland.*(fail|error|No such|NoWayland|unable)" "$LOG" \
   || grep -qiE "fallback|web.?view|open.*browser" "$LOG"; then
    echo "[orchestrator] Wayland gagal, fallback ke X11. Log: $LOG" >&2
    backend="x11"
    run "$backend"
fi

exec env WINIT_UNIX_BACKEND="$backend" RUST_LOG=debug WINIT_DEBUG=1 "$BIN" "$@"
