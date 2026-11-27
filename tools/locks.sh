#!/usr/bin/env bash
# The machine's locks (docs/locks.md): the quiet lock, and the display.
#
# The quiet lock: a run that needs the machine to itself (a timing run, a
# soak) holds it alone; the ordinary heavy runs (the oracle scripts, an
# orchestrator's suites) share it, and wait while a quiet run holds it.
# Two flock(2) files in ${XDG_STATE_HOME:-~/.local/state}/moonglow/locks:
# quiet.lock (shared by ordinary runs, exclusive for a quiet one) and
# quiet.gate (an ordinary run passes through it shared; a quiet run holds it
# exclusive while it waits and runs, so new ordinary runs can't keep it
# waiting for ever).
#
#   tools/locks.sh hold quiet -- CMD [ARGS...]    # the machine alone
#   tools/locks.sh share quiet -- CMD [ARGS...]   # an ordinary heavy run
#
# The order is the quiet lock, then the display, then a GPU slot; nobody
# upgrades. Taking the display shares the quiet lock first.
#
# The display: one
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
#                               (`display`, `quiet`): a lock named there is not taken
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

# The folder of the machine's quiet lock (beside the GPU slots, not moved
# by MOONGLOW_ORACLE).
mg_state_locks_dir() {
  printf '%s\n' "${XDG_STATE_HOME:-$HOME/.local/state}/moonglow/locks"
}

# How long to wait (MOONGLOW_LOCK_WAIT, default 1800 s) and how to say it.
mg_lock_wait() {
  local wait=${MOONGLOW_LOCK_WAIT:-1800}
  case "$wait" in '' | *[!0-9]*) wait=1800 ;; esac
  printf '%s\n' "$wait"
}
mg_lock_wait_words() {
  if (($1 == 0 || $1 >= 60)); then echo "$(($1 / 60)) min"; else echo "$1 s"; fi
}

# Takes flock mode $2 (-s or -x) on descriptor $1 by the epoch second $3,
# saying `$4` (evaluated: it may name the holder) on stderr at once and
# every 5 minutes while it waits. Fails past the deadline.
mg_flock_by() {
  local fd=$1 mode=$2 deadline=$3 line=$4 chunk
  flock "$mode" -n "$fd" && return 0
  while true; do
    chunk=$((deadline - $(date +%s)))
    ((chunk < 1)) && break
    eval "echo \"$line\"" >&2
    ((chunk > 300)) && chunk=300
    flock "$mode" -w "$chunk" "$fd" && return 0
  done
  flock "$mode" -n "$fd"
}

# Who holds the quiet lock, from /proc/locks (matched by inode): "1 run
# holds it: PID 1234", "2 runs hold it: PIDs 1234, 5678", or "other runs
# hold it" where it can't be read.
mg_quiet_holders() {
  local ino pids n
  ino=$(stat -c %i "$(mg_state_locks_dir)/quiet.lock" 2>/dev/null)
  pids=$(awk -v ino="$ino" '$2 == "FLOCK" { n = split($6, p, ":"); if (p[n] == ino) print $5 }' \
    /proc/locks 2>/dev/null | sort -un)
  n=$(printf '%s' "$pids" | grep -c .)
  case "$n" in
    0) echo "other runs hold it" ;;
    1) echo "1 run holds it: PID $pids" ;;
    *) echo "$n runs hold it: PIDs $(printf '%s' "$pids" | paste -sd, - | sed 's/,/, /g')" ;;
  esac
}

# Shares the quiet lock for this shell and what it starts (an ordinary heavy
# run), unless an ancestor holds it: waits while a quiet run holds it or
# waits for it, and past MOONGLOW_LOCK_WAIT refuses with "the machine is
# still kept quiet after N min: nothing was run", status 2.
mg_lock_quiet_shared() {
  mg_lock_held_by_ancestor quiet && return 0
  local dir wait mins deadline
  dir="$(mg_state_locks_dir)"
  wait=$(mg_lock_wait)
  mins=$(mg_lock_wait_words "$wait")
  mkdir -p "$dir" || { echo "cannot make $dir for the quiet lock" >&2; exit 2; }
  exec {MG_QUIET_GATE_FD}>>"$dir/quiet.gate" || { echo "cannot open $dir/quiet.gate" >&2; exit 2; }
  exec {MG_QUIET_LOCK_FD}>>"$dir/quiet.lock" || { echo "cannot open $dir/quiet.lock" >&2; exit 2; }
  deadline=$(($(date +%s) + wait))
  # (Through the gate, shared; then the lock, shared; the gate given back.
  # Past the gate no quiet run holds the lock, but one just ending may
  # still: a moment's wait.)
  local line='waiting for a quiet run to end: held by $(head -n 1 "$dir/quiet.lock" 2>/dev/null || true); up to $mins'
  if ! mg_flock_by "$MG_QUIET_GATE_FD" -s "$deadline" "$line" ||
    ! mg_flock_by "$MG_QUIET_LOCK_FD" -s "$((deadline > $(date +%s) + 5 ? deadline : $(date +%s) + 5))" "$line"; then
    echo "the machine is still kept quiet after $mins: nothing was run" >&2
    exit 2
  fi
  flock -u "$MG_QUIET_GATE_FD"
  exec {MG_QUIET_GATE_FD}>&-
  export MOONGLOW_LOCKS_HELD="quiet${MOONGLOW_LOCKS_HELD:+,$MOONGLOW_LOCKS_HELD}"
}

# Takes the machine alone for this shell and what it starts (a timing run, a
# soak), unless an ancestor holds the quiet lock: the gate, then the lock,
# both exclusive; waits for another quiet run and for the ordinary runs to
# end, and past MOONGLOW_LOCK_WAIT refuses with "the machine is still busy
# after N min: nothing was run", status 2. $1 describes the run, for the
# holder's line in quiet.lock.
mg_lock_quiet() {
  mg_lock_held_by_ancestor quiet && return 0
  local dir wait mins deadline what
  dir="$(mg_state_locks_dir)"
  what=${1:-"$0"}
  wait=$(mg_lock_wait)
  mins=$(mg_lock_wait_words "$wait")
  mkdir -p "$dir" || { echo "cannot make $dir for the quiet lock" >&2; exit 2; }
  exec {MG_QUIET_GATE_FD}>>"$dir/quiet.gate" || { echo "cannot open $dir/quiet.gate" >&2; exit 2; }
  exec {MG_QUIET_LOCK_FD}>>"$dir/quiet.lock" || { echo "cannot open $dir/quiet.lock" >&2; exit 2; }
  deadline=$(($(date +%s) + wait))
  if ! mg_flock_by "$MG_QUIET_GATE_FD" -x "$deadline" \
    'waiting for a quiet run to end: held by $(head -n 1 "$dir/quiet.lock" 2>/dev/null || true); up to $mins' ||
    ! mg_flock_by "$MG_QUIET_LOCK_FD" -x "$deadline" \
      'waiting for the machine to be quiet ($(mg_quiet_holders)); up to $mins'; then
    echo "the machine is still busy after $mins: nothing was run" >&2
    exit 2
  fi
  printf 'PID %s, started %s, %s, in %s\n' "$$" "$(date +%T)" "$what" "$PWD" > "$dir/quiet.lock"
  export MOONGLOW_LOCKS_HELD="quiet${MOONGLOW_LOCKS_HELD:+,$MOONGLOW_LOCKS_HELD}"
}

# Takes the display lock for this shell and what it starts (the descriptor
# is inherited; a program that must not keep it closes it, as
# mg_close_extra_fds does), unless an ancestor holds it. $1 describes the
# run, for the holder's line (default: this script and its arguments).
mg_lock_display() {
  mg_lock_held_by_ancestor display && return 0
  # (The quiet lock first, shared: the order is quiet, display, GPU.)
  mg_lock_quiet_shared
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
    echo "usage: $0 hold display|quiet -- COMMAND [ARGS...]" >&2
    echo "       $0 share quiet -- COMMAND [ARGS...]" >&2
    exit 2
  }
  [[ "${3:-}" == -- && $# -ge 4 ]] || usage
  case "$1 $2" in
    "hold display") shift 3 && mg_lock_display "$*" ;;
    "hold quiet") shift 3 && mg_lock_quiet "$*" ;;
    "share quiet") shift 3 && mg_lock_quiet_shared ;;
    *) usage ;;
  esac
  exec "$@"
fi
