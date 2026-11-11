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
# - It draws on $DISPLAY, the off-screen display from
#   tools/aurora/headless.sh, never on the desktop: on any other display,
#   or none, it does not start (a default once sent clients to the
#   desktop's Xwayland, which holds :1 here, when the off-screen display
#   failed to start).
#
# - MG_CLIENT_PRELOAD (optional): a library preloaded into the client
#   alone (LD_PRELOAD inside the sandbox; outside /tmp, which it hides),
#   as tools/nwclient/timeshift.c and execlog.c are.
# - MG_CLIENT_AUDIO_OUT (optional): a file under SCRATCH that receives
#   everything the client plays, as a WAV file: OpenAL Soft's `wave`
#   backend (ALSOFT_DRIVERS=wave), configured by SCRATCH/alsoft.conf,
#   which this writes, for 44100 Hz, stereo, 16-bit. Still no audio
#   device: the mixer writes the file in real time, and a killed client
#   leaves its header's sizes unset (0xFFFFFFFF). A path outside SCRATCH
#   is refused. Unset, the client gets the null backend, as before.
#
# The game install comes from $NWN_ROOT (default: the Steam path).
set -e
SCRATCH=$(realpath "$1")
shift
NWN=${NWN_ROOT:-"$HOME/.local/share/Steam/steamapps/common/Neverwinter Nights"}
DISP=${DISPLAY:-}
# Only the running off-screen session's display (headless.sh check).
"$(dirname "$0")/../aurora/headless.sh" check "$DISP"
SOCKET=/tmp/.X11-unix/X${DISP#:}
mkdir -p "$SCRATCH/home" "$SCRATCH/user" "$SCRATCH/run"
AUDIO=null
if [ -n "${MG_CLIENT_AUDIO_OUT:-}" ]; then
  OUT=$(realpath -m "$MG_CLIENT_AUDIO_OUT")
  case "$OUT" in
    "$SCRATCH"/*) ;;
    *) echo "run-client.sh: MG_CLIENT_AUDIO_OUT must be under $SCRATCH: $OUT" >&2; exit 2 ;;
  esac
  mkdir -p "$(dirname "$OUT")"
  printf '[general]\ndrivers = wave\nfrequency = 44100\nchannels = stereo\nsample-type = int16\n\n[wave]\nfile = %s\n' \
    "$OUT" > "$SCRATCH/alsoft.conf"
  AUDIO=wave
fi
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
  --setenv ALSOFT_DRIVERS "$AUDIO" --setenv SDL_AUDIODRIVER dummy \
  ${MG_CLIENT_AUDIO_OUT:+--setenv ALSOFT_CONF "$SCRATCH/alsoft.conf"} \
  ${MG_CLIENT_PRELOAD:+--setenv LD_PRELOAD "$MG_CLIENT_PRELOAD"} \
  --chdir "$NWN/bin/linux-x86" \
  ./nwmain-linux -userdirectory "$SCRATCH/user" "$@"
