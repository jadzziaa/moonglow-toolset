#!/usr/bin/env bash
# Build modules' scripts in Aurora and save them, unattended
# (build_in_aurora.py): starts the off-screen display and Aurora, runs the
# driver, and stops both, whatever happens. Run it under the display lock:
#
#   tools/locks.sh hold display -- tools/aurora/build_in_aurora.sh SHOTS NAME=OUT.mod...
set -uo pipefail
here=$(dirname "$0")
[[ ",${MOONGLOW_LOCKS_HELD:-}," == *,display,* ]] || {
  echo "run under tools/locks.sh hold display --" >&2
  exit 2
}
D=$("$here/headless.sh" start) || exit 1
export DISPLAY=$D
"$here/run-aurora.sh" run >/dev/null 2>&1 &
aurora=$!
python3 "$here/build_in_aurora.py" "$@"
status=$?
"$here/run-aurora.sh" kill
wait "$aurora" 2>/dev/null
"$here/headless.sh" stop
exit $status
