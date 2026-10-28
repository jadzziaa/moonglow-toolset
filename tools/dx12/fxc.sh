#!/usr/bin/env bash
# Compiles HLSL with Microsoft's FXC (d3dcompiler_47.dll) under Wine, as
# wgpu's Direct3D 12 backend compiles it on a Windows machine without
# dxcompiler.dll (Moonglow ships none): shader model 5.1, strictness on.
#
#   tools/dx12/fxc.sh D3DCOMPILER_DLL MANIFEST
#
# D3DCOMPILER_DLL is Microsoft's own d3dcompiler_47.dll (Wine's builtin
# one is another compiler, vkd3d's: no oracle); MANIFEST is a file of
# lines "<file>\t<entry point>\t<profile>", the files' paths as Windows
# sees them under Wine (Z:\home\...). The HLSL test of the game's programs
# (crates/mg-corpus-tests/tests/hlsl.rs) writes one and runs this. Prints
# a line for each shader (OK or FAIL, the milliseconds FXC took, and its
# message); exits 1 when one failed.
#
# The program is built with mingw-w64 (x86_64-w64-mingw32-gcc) into
# $MOONGLOW_FXC_DIR (default target/test-output/fxc), with the DLL copied
# beside it, and run in the Wine prefix $WINEPREFIX (default the Aurora
# oracle's, ~/.local/share/moonglow-oracle/wineprefix) with no display:
# nothing reaches the desktop.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
dll=${1:?the d3dcompiler_47.dll to use}
manifest=${2:?a manifest of shaders}
out=${MOONGLOW_FXC_DIR:-"$root/target/test-output/fxc"}
mkdir -p "$out"
if [[ ! -x "$out/fxc.exe" || "$here/fxc.c" -nt "$out/fxc.exe" ]]; then
  x86_64-w64-mingw32-gcc -O2 -o "$out/fxc.exe" "$here/fxc.c"
fi
cp -f "$dll" "$out/d3dcompiler_47.dll"
export WINEPREFIX=${WINEPREFIX:-"$HOME/.local/share/moonglow-oracle/wineprefix"}
export WINEDEBUG=-all
# (The DLL beside the program, not Wine's own; no Mono or Gecko asked for.)
export WINEDLLOVERRIDES="d3dcompiler_47=n;mscoree=;mshtml="
unset DISPLAY WAYLAND_DISPLAY
exec wine "$out/fxc.exe" "$manifest"
