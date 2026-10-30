#!/bin/sh
# The game's headless server and the game client joined to it, in one
# sandbox whose network is a loopback of its own (bubblewrap's
# --unshare-net brings `lo` up and nothing else): for tests of what a
# client sees of a server (the DM client, NWSync downloads).
#
# It is started as the server's program by mg_serve::Server::start
# (Options::server), so that the server gets what Server::start gives
# every server (-publicserver 0, passwords nobody was told):
#
#   tools/nwclient/run-server-and-client.sh <nwserver's arguments...>
#
# with, in the environment (Options::env):
#
# - MG_SCRATCH: the scratch directory, the only writable place. The
#   client's user directory is MG_SCRATCH/user; the server's
#   (-userdirectory, from its arguments) must be under MG_SCRATCH too.
# - MG_CLIENT: `player` (the default) joins with the player password,
#   `dm` starts the DM client (-dmc) and joins with the DM password,
#   `none` starts no client.
# - MG_WEB (optional): a folder served over HTTP on 127.0.0.1:MG_WEB_PORT
#   (default 8080) inside the sandbox, by Python's http.server: an NWSync
#   repository, the server given -nwsyncurl http://127.0.0.1:PORT.
# - MG_CLIENT_ARGS (optional): more of the client's arguments.
#
# As run-client.sh: the system read-only, Steam hidden, no D-Bus or audio,
# the client drawn on $DISPLAY (default :1, the off-screen display), never
# the desktop's :0. Killing this process (bwrap) ends everything in the
# sandbox: the server and the client are in its own PID namespace.
#
# The game install comes from $NWN_ROOT (default: the Steam path).
set -e
[ -n "$MG_SCRATCH" ] || { echo "MG_SCRATCH is not set" >&2; exit 1; }
SCRATCH=$(realpath "$MG_SCRATCH")
NWN=${NWN_ROOT:-"$HOME/.local/share/Steam/steamapps/common/Neverwinter Nights"}
DISP=${DISPLAY:-:1}
[ "$DISP" = ":0" ] && { echo "refusing to run on the desktop display :0" >&2; exit 1; }
SOCKET=/tmp/.X11-unix/X${DISP#:}

# The port and passwords the server is given, for the client.
PORT=5121
PLAYER=
DM=
prev=
for a in "$@"; do
  case "$prev" in
    -port) PORT=$a ;;
    -playerpassword) PLAYER=$a ;;
    -dmpassword) DM=$a ;;
    -userdirectory)
      case "$(realpath "$a")" in
        "$SCRATCH"/*) ;;
        *) echo "the server's user directory must be under MG_SCRATCH" >&2; exit 1 ;;
      esac ;;
  esac
  prev=$a
done

WEB_PORT=${MG_WEB_PORT:-8080}
WEB=
NWSYNC=
if [ -n "$MG_WEB" ]; then
  WEB=$(realpath "$MG_WEB")
  NWSYNC="-nwsyncurl http://127.0.0.1:$WEB_PORT"
fi

case "${MG_CLIENT:-player}" in
  player) CLIENT="+connect 127.0.0.1:$PORT +password $PLAYER" ;;
  dm) CLIENT="-dmc +connect 127.0.0.1:$PORT +password $DM" ;;
  # (The client, joined by MG_CLIENT_ARGS alone.)
  bare) CLIENT=" " ;;
  none) CLIENT= ;;
  *) echo "MG_CLIENT is player, dm, bare or none" >&2; exit 1 ;;
esac

mkdir -p "$SCRATCH/home" "$SCRATCH/user" "$SCRATCH/run"

# Inside: the web server (if any), the server, and once its port is
# taken, the client; the sandbox ends when the server does.
INNER='
cd "$NWN/bin/linux-x86"
if [ -n "$WEB" ]; then
  python3 -I -m http.server "$WEB_PORT" --bind 127.0.0.1 --directory "$WEB" \
    > "$SCRATCH/web.log" 2>&1 &
fi
./nwserver-linux "$@" $NWSYNC &
SERVER=$!
if [ -n "$CLIENT" ]; then
  # (The module loaded: the line MG_READY_LINE in the log of the
  # server, by default its "Loading Module", and three seconds more.)
  i=0
  while [ $i -lt 240 ] && ! grep -qs "${MG_READY_LINE:-Loading Module}" "$LOGS"/nwserverLog1.txt; do
    sleep 0.5; i=$((i + 1))
  done
  sleep 3
  ./nwmain-linux -userdirectory "$SCRATCH/user" $CLIENT $MG_CLIENT_ARGS \
    > "$SCRATCH/client.out" 2>&1 &
fi
wait $SERVER
'
# The server's logs: its -userdirectory's logs.0.
LOGS=
prev=
for a in "$@"; do
  [ "$prev" = "-userdirectory" ] && LOGS="$(realpath "$a")/logs.0"
  prev=$a
done

exec bwrap \
  --ro-bind / / \
  --dev-bind /dev /dev \
  --tmpfs /dev/shm \
  --proc /proc \
  --tmpfs /tmp \
  --bind "$SOCKET" "$SOCKET" \
  --tmpfs "/run/user/$(id -u)" \
  --tmpfs "$HOME/.steam" \
  --tmpfs "$HOME/.local/share/Steam" \
  --ro-bind "$NWN" "$NWN" \
  --bind "$SCRATCH" "$SCRATCH" \
  --unshare-net --unshare-ipc --unshare-pid --die-with-parent \
  --setenv HOME "$SCRATCH/home" --setenv DISPLAY "$DISP" --setenv XDG_RUNTIME_DIR "$SCRATCH/run" \
  --unsetenv DBUS_SESSION_BUS_ADDRESS --unsetenv WAYLAND_DISPLAY \
  --setenv ALSOFT_DRIVERS null --setenv SDL_AUDIODRIVER dummy \
  --setenv NWN "$NWN" --setenv SCRATCH "$SCRATCH" --setenv LOGS "$LOGS" \
  --setenv WEB "$WEB" --setenv WEB_PORT "$WEB_PORT" --setenv NWSYNC "$NWSYNC" \
  --setenv CLIENT "$CLIENT" --setenv MG_CLIENT_ARGS "${MG_CLIENT_ARGS:-}" \
  /bin/sh -c "$INNER" sh "$@"
