#!/usr/bin/env bash
# An off-screen X display for the Aurora oracle, so it can be driven
# (tools/aurora/xdrive.py) without touching the user's desktop: a headless
# KWin (virtual backend) with Xwayland, in its own D-Bus session and config
# directory.
#
#   tools/aurora/headless.sh start    # prints the X display, e.g. :1
#   tools/aurora/headless.sh stop
#
#   DISPLAY=$(tools/aurora/headless.sh start) tools/aurora/run-aurora.sh
set -euo pipefail

ORACLE=${MOONGLOW_ORACLE:-"$HOME/.local/share/moonglow-oracle"}
H="$ORACLE/headless"

running() {
  [[ -f "$H/session.pid" ]] && kill -0 "$(cat "$H/session.pid")" 2>/dev/null
}

case "${1:-start}" in
  start)
    if running && [[ -s "$H/display" ]]; then
      cat "$H/display"
      exit 0
    fi
    mkdir -p "$H/config"
    # Xwayland passes XTEST input on to the compositor (EIS); allow that
    # without the interactive prompt. A locked screen would swallow the
    # input: no lock screen (--no-lockscreen) and no locking when idle.
    printf '[Xwayland]\nXwaylandEisNoPrompt=true\n' > "$H/config/kwinrc"
    printf '[Daemon]\nAutolock=false\nLockOnResume=false\n' > "$H/config/kscreenlockerrc"
    rm -f "$H/display"
    cat > "$H/started.sh" <<EOF
#!/bin/sh
echo "\$DISPLAY" > "$H/display"
exec sleep infinity
EOF
    chmod +x "$H/started.sh"
    env -u DISPLAY -u WAYLAND_DISPLAY XDG_CONFIG_HOME="$H/config" \
      setsid dbus-run-session -- kwin_wayland --virtual --xwayland \
      --no-lockscreen --no-global-shortcuts \
      --socket moonglow-oracle --width 1280 --height 1024 "$H/started.sh" \
      > "$H/kwin.log" 2>&1 &
    echo $! > "$H/session.pid"
    for _ in $(seq 100); do
      [[ -s "$H/display" ]] && break
      sleep 0.1
    done
    [[ -s "$H/display" ]] || { echo "headless session did not start; see $H/kwin.log" >&2; exit 1; }
    cat "$H/display"
    ;;
  stop)
    if running; then
      # setsid made the session its own process group.
      kill -- "-$(cat "$H/session.pid")" 2>/dev/null || true
    fi
    rm -f "$H/session.pid" "$H/display"
    ;;
  *)
    echo "usage: $0 [start|stop]" >&2
    exit 2
    ;;
esac
