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
#
# Starting a new session and stopping one take the machine's display lock
# (tools/locks.sh, docs/locks.md), unless an ancestor holds it
# (MOONGLOW_LOCKS_HELD); they wait on stderr, since start's stdout is the
# display. Starting a session that already runs, and check, take nothing.
set -euo pipefail

. "$(dirname "$0")/../locks.sh"

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
    mg_lock_display "headless.sh start"
    # (Another run may have started it while this one waited.)
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
echo "\$\$" > "$H/keeper.pid"
echo "\$DISPLAY" > "$H/display"
exec sleep infinity
EOF
    chmod +x "$H/started.sh"
    # (The session outlives this command, so it must not inherit the lock's
    # descriptor, or the lock would be held for the session's life: every
    # descriptor above 2 is closed in the subshell that starts it.)
    (
      mg_close_extra_fds
      exec env -u DISPLAY -u WAYLAND_DISPLAY XDG_CONFIG_HOME="$H/config" \
        setsid dbus-run-session -- kwin_wayland --virtual --xwayland \
        --no-lockscreen --no-global-shortcuts \
        --socket moonglow-oracle --width 1280 --height 1024 "$H/started.sh"
    ) > "$H/kwin.log" 2>&1 &
    echo $! > "$H/session.pid"
    for _ in $(seq 100); do
      [[ -s "$H/display" ]] && break
      sleep 0.1
    done
    if [[ ! -s "$H/display" ]]; then
      mg_display_history "headless.sh start: the session did not start"
      echo "headless session did not start; see $H/kwin.log" >&2
      exit 1
    fi
    mg_display_history "headless.sh start: display $(cat "$H/display"), session PID $(cat "$H/session.pid")"
    cat "$H/display"
    ;;
  check)
    # Whether DISPLAY (or the display given) is this off-screen session's,
    # running: the client and Aurora draw on no other (on this machine the
    # desktop's Xwayland holds :1 as well as :0).
    want=${2:-${DISPLAY:-}}
    if [[ -z "$want" ]]; then
      echo "DISPLAY is not set: give the off-screen display (tools/aurora/headless.sh start)" >&2
      exit 1
    fi
    if ! running || [[ ! -s "$H/display" ]] || [[ "$(cat "$H/display")" != "$want" ]]; then
      echo "refusing display $want: it is not the running off-screen session's (tools/aurora/headless.sh start)" >&2
      exit 1
    fi
    ;;
  stop)
    mg_lock_display "headless.sh stop"
    if running; then
      mg_display_history "headless.sh stop: display $(cat "$H/display" 2>/dev/null), session PID $(cat "$H/session.pid")"
    else
      mg_display_history "headless.sh stop: no session running"
    fi
    if running; then
      # setsid made the session its own process group.
      kill -- "-$(cat "$H/session.pid")" 2>/dev/null || true
    fi
    # KWin starts the session's keeper (started.sh's sleep) outside that
    # group, so it outlived every stop: end it by its own PID.
    if [[ -f "$H/keeper.pid" ]]; then
      keeper=$(cat "$H/keeper.pid")
      if [[ "$(tr '\0' ' ' < "/proc/$keeper/cmdline" 2>/dev/null)" == "sleep infinity " ]]; then
        kill "$keeper" 2>/dev/null || true
      fi
    fi
    rm -f "$H/session.pid" "$H/display" "$H/keeper.pid"
    ;;
  *)
    echo "usage: $0 [start|stop]" >&2
    exit 2
    ;;
esac
