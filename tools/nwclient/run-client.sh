#!/bin/sh
# Runs the NWN:EE game client for tests, sandboxed with bubblewrap so that it
# cannot touch anything but a scratch directory:
#
#   tools/nwclient/run-client.sh SCRATCH [client arguments...]
#
# - The whole system is read-only; only SCRATCH is writable. The client's
#   user directory is SCRATCH/user (-userdirectory).
# - Steam is hidden (~/.steam and the Steam folder, except the game install),
#   so the client runs without the Steam API: no presence, no cloud sync.
# - No network, no D-Bus session, no audio (the runtime directory is empty).
# - It draws on $DISPLAY (default :1, the off-screen display from
#   tools/aurora/headless.sh), never on the desktop.
#
# The game install comes from $NWN_ROOT (default: the Steam path).
set -e
SCRATCH=$(realpath "$1")
shift
NWN=${NWN_ROOT:-"$HOME/.local/share/Steam/steamapps/common/Neverwinter Nights"}
DISP=${DISPLAY:-:1}
[ "$DISP" = ":0" ] && { echo "refusing to run on the desktop display :0" >&2; exit 1; }
SOCKET=/tmp/.X11-unix/X${DISP#:}
mkdir -p "$SCRATCH/home" "$SCRATCH/user" "$SCRATCH/run"
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
  --chdir "$NWN/bin/linux-x86" \
  ./nwmain-linux -userdirectory "$SCRATCH/user" "$@"
