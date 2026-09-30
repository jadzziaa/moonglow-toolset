#!/usr/bin/env bash
# Launch the real Aurora Toolset (nwtoolset.exe) under Wine as a test oracle.
#
# It runs in its own Wine prefix with its own NWN user directory, so it never
# touches the real NWN user folder (~/.local/share/Neverwinter Nights).
#
#   tools/aurora/run-aurora.sh            # start the toolset
#   tools/aurora/run-aurora.sh kill       # stop it (and the prefix's wineserver)
#   tools/aurora/run-aurora.sh extract-forms OUTDIR   # decode its VCL forms to text
#
# Environment:
#   NWN_ROOT        game install (default: Steam's "Neverwinter Nights")
#   MOONGLOW_ORACLE oracle state dir (default: ~/.local/share/moonglow-oracle)
set -euo pipefail

NWN_ROOT=${NWN_ROOT:-"$HOME/.local/share/Steam/steamapps/common/Neverwinter Nights"}
ORACLE=${MOONGLOW_ORACLE:-"$HOME/.local/share/moonglow-oracle"}
export WINEPREFIX="$ORACLE/wineprefix"
export WINEDLLOVERRIDES="mscoree,mshtml="
export WINEDEBUG=-all
EXE="$NWN_ROOT/bin/win32/nwtoolset.exe"

case "${1:-run}" in
  run)
    mkdir -p "$ORACLE/userdir" "$WINEPREFIX"
    [[ -f "$WINEPREFIX/system.reg" ]] || wineboot -i
    cd "$NWN_ROOT/bin/win32"
    exec wine nwtoolset.exe -userdirectory "Z:$ORACLE/userdir"
    ;;
  kill)
    wineserver -k
    ;;
  extract-forms)
    out=${2:?output directory}
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    7z x -y "$EXE" -o"$tmp" '.rsrc/0/RCDATA/*' >/dev/null
    python3 "$(dirname "$0")/dfm2txt.py" "$tmp/.rsrc/0/RCDATA" "$out"
    ;;
  *)
    echo "usage: $0 [run|kill|extract-forms OUTDIR]" >&2
    exit 2
    ;;
esac
