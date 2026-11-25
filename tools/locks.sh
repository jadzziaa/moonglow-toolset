#!/usr/bin/env bash
# The machine's lock on the off-screen display (docs/locks.md): one
# exclusive flock(2) on ${MOONGLOW_ORACLE:-~/.local/share/moonglow-oracle}/locks/display.lock,
# held by every run that puts a window on the display or restarts it (the
# game client, Aurora, a session start or stop). Linux, bash, flock(1).
#
# Sourced by the scripts that take it:
#
#   . "$(dirname "$0")/../locks.sh"
#   mg_lock_display "what this run is"   # waits; the lock lives as long as
#                                        # the shell and what inherits fd
#
# or run for anything driven by hand (one client run with xdrive calls under
# it, a whole Aurora capture session):
#
#   tools/locks.sh hold display -- CMD [ARGS...]
#
# Environment:
#   MOONGLOW_LOCK_WAIT=SECONDS  how long to wait (default 1800; 0 refuses at once)
#   MOONGLOW_LOCKS_HELD         comma list of the locks an ancestor holds
#                               (`display`): a lock named there is not taken
#                               again, since a lock belongs to the open file
#                               and a child would wait for its own parent.
#
# A waiting run says so on stderr (never stdout: headless.sh start's is the
# display) and, past the wait, refuses with "the off-screen display is still
# held after N min: nothing was run", status 2. The holder writes one line
# into the file: PID, start time, command, folder.

mg_oracle_dir() {
  printf '%s\n' "${MOONGLOW_ORACLE:-$HOME/.local/share/moonglow-oracle}"
}

# Whether an ancestor holds the lock named $1 (MOONGLOW_LOCKS_HELD).
mg_lock_held_by_ancestor() {
  case ",${MOONGLOW_LOCKS_HELD:-}," in
    *",$1,"*) return 0 ;;
    *) return 1 ;;
  esac
}

# Takes the display lock for this shell and what it starts (the descriptor
# is inherited; a program that must not keep it closes it, as
# mg_close_extra_fds does), unless an ancestor holds it. $1 describes the
# run, for the holder's line (default: this script and its arguments).
mg_lock_display() {
  mg_lock_held_by_ancestor display && return 0
  local dir file wait deadline now holder chunk mins what
  dir="$(mg_oracle_dir)/locks"
  file="$dir/display.lock"
  what=${1:-"$0"}
  wait=${MOONGLOW_LOCK_WAIT:-1800}
  case "$wait" in '' | *[!0-9]*) wait=1800 ;; esac
  # (Minutes, as the default is; seconds for a short wait set by hand.)
  if ((wait == 0 || wait >= 60)); then mins="$((wait / 60)) min"; else mins="$wait s"; fi
  mkdir -p "$dir" || { echo "cannot make $dir for the display lock" >&2; exit 2; }
  # (Appending: opening the file must not empty the holder's line.)
  exec {MG_DISPLAY_LOCK_FD}>>"$file" || { echo "cannot open $file" >&2; exit 2; }
  if ! flock -x -n "$MG_DISPLAY_LOCK_FD"; then
    now=$(date +%s)
    deadline=$((now + wait))
    while true; do
      if ((wait == 0)); then break; fi
      holder=$(head -n 1 "$file" 2>/dev/null)
      echo "waiting for the off-screen display: held by ${holder:-someone}; up to $mins" >&2
      chunk=$((deadline - $(date +%s)))
      ((chunk > 300)) && chunk=300
      ((chunk < 1)) && break
      flock -x -w "$chunk" "$MG_DISPLAY_LOCK_FD" && break
      (($(date +%s) >= deadline)) && break
    done
    if ! flock -x -n "$MG_DISPLAY_LOCK_FD"; then
      echo "the off-screen display is still held after $mins: nothing was run" >&2
      exit 2
    fi
  fi
  printf 'PID %s, started %s, %s, in %s\n' "$$" "$(date +%T)" "$what" "$PWD" > "$file"
  export MOONGLOW_LOCKS_HELD="display${MOONGLOW_LOCKS_HELD:+,$MOONGLOW_LOCKS_HELD}"
}

# Appends one line to the display's history
# (${MOONGLOW_ORACLE}/locks/display.history, docs/locks.md): the UTC time,
# this PID, the event ($*), the display lock's state (taken by this run, or
# by an ancestor and its holder line, or not held), what called this
# script and the folder; keeps the last 1,000 lines. Never fails its caller.
mg_display_history() {
  local dir file lock caller line
  dir="$(mg_oracle_dir)/locks"
  file="$dir/display.history"
  mkdir -p "$dir" 2>/dev/null || return 0
  if [[ -n "${MG_DISPLAY_LOCK_FD:-}" ]]; then
    lock="held by this run"
  elif mg_lock_held_by_ancestor display; then
    lock="held by an ancestor ($(head -n 1 "$dir/display.lock" 2>/dev/null))"
  else
    lock="not held"
  fi
  caller=$(tr '\0' ' ' < "/proc/$PPID/cmdline" 2>/dev/null | cut -c1-200)
  line="$(date -u +%FT%TZ) PID $$: $*; lock $lock; called by PID $PPID ${caller% }; in $PWD"
  (
    flock -x 9 || exit 0
    printf '%s\n' "$line" >&9
    if (($(wc -l < "$file") > 1200)); then
      tail -n 1000 "$file" > "$file.tmp" && cat "$file.tmp" > "$file"
      rm -f "$file.tmp"
    fi
  ) 9>> "$file" 2>/dev/null || true
}

# Closes every descriptor above 2 in this shell: for a subshell that starts
# a long-lived program (the headless KWin session) which must not keep the
# lock's descriptor, or the lock would be held for the session's life.
mg_close_extra_fds() {
  local f n
  for f in /proc/self/fd/*; do
    n=${f##*/}
    # (255 is bash's own script descriptor, close-on-exec.)
    if [[ "$n" =~ ^[0-9]+$ ]] && ((n > 2 && n != 255)); then
      eval "exec $n>&-" 2>/dev/null || true
    fi
  done
}

# Run, not sourced: tools/locks.sh hold display -- CMD...
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  usage() {
    echo "usage: $0 hold display -- COMMAND [ARGS...]" >&2
    exit 2
  }
  [[ "${1:-}" == hold && "${2:-}" == display && "${3:-}" == -- && $# -ge 4 ]] || usage
  shift 3
  mg_lock_display "$*"
  exec "$@"
fi
