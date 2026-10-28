#!/usr/bin/env bash
# Compiles HLSL with Microsoft's DXC (dxcompiler.dll, through its dxc.exe)
# under Wine, as wgpu's Direct3D 12 backend compiles it on a Windows
# machine with the dxcompiler.dll Moonglow ships beside its programs: the
# arguments wgpu 30 gives IDxcCompiler3::Compile (wgpu-hal's
# dx12/shader_compilation.rs: -E, -T, -HV 2018, -no-warnings, -Ges).
#
#   tools/dx12/dxc.sh DXC_DIR MANIFEST
#
# DXC_DIR holds the release's dxc.exe and dxcompiler.dll (bin/x64 of
# github.com/microsoft/DirectXShaderCompiler's zip); they alone are copied,
# not dxil.dll, which Moonglow doesn't ship: a shader compiled but left
# unsigned (its container's hash zero) fails, as Direct3D 12 refuses it.
# MANIFEST is a file of lines "<file>\t<entry point>\t<profile>", the
# files' paths as Windows sees them under Wine (Z:\home\...); the HLSL
# tests write one and run this. Prints a line for each shader (OK or FAIL,
# the milliseconds it took, and DXC's message); exits 1 when one failed.
# (dxc.exe runs once a shader: the milliseconds are mostly Wine's start,
# about 4 s here, not DXC's.)
#
# Run in the Wine prefix $WINEPREFIX (default the Aurora oracle's,
# ~/.local/share/moonglow-oracle/wineprefix) with no display: nothing
# reaches the desktop. The copies go to $MOONGLOW_DXC_DIR (default
# target/test-output/dxc), the compiled objects beside them.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
dxc=${1:?the folder of dxc.exe and dxcompiler.dll}
manifest=${2:?a manifest of shaders}
out=${MOONGLOW_DXC_DIR:-"$root/target/test-output/dxc"}
mkdir -p "$out/objects"
cp -f "$dxc/dxc.exe" "$dxc/dxcompiler.dll" "$out/"
rm -f "$out/dxil.dll"
export WINEPREFIX=${WINEPREFIX:-"$HOME/.local/share/moonglow-oracle/wineprefix"}
export WINEDEBUG=-all
# (The DLLs beside the program; no Mono or Gecko asked for.)
export WINEDLLOVERRIDES="dxcompiler=n;dxil=n;mscoree=;mshtml="
unset DISPLAY WAYLAND_DISPLAY

failed=0
n=0
while IFS=$'\t' read -r file entry profile; do
  [[ -n "$file" && -n "$profile" ]] || continue
  n=$((n + 1))
  object="$out/objects/$n.dxil"
  rm -f "$object"
  start=$(date +%s%N)
  set +e
  said=$(wine "$out/dxc.exe" -nologo -E "$entry" -T "$profile" -HV 2018 -no-warnings -Ges \
    -Fo "Z:${object//\//\\}" "$file" 2>&1)
  status=$?
  set -e
  ms=$((($(date +%s%N) - start) / 1000000))
  said=$(printf '%s' "$said" | tr '\r\n' '  ')
  if [[ $status -ne 0 || ! -s "$object" ]]; then
    echo "FAIL $file $ms: exit $status: $said"
    failed=$((failed + 1))
    continue
  fi
  # The container's digest (bytes 4 to 19 after "DXBC"): zero when unsigned.
  digest=$(od -An -tx1 -j4 -N16 "$object" | tr -d ' \n')
  if [[ "$digest" == "00000000000000000000000000000000" ]]; then
    echo "FAIL $file $ms: compiled but not signed (no validator): $said"
    failed=$((failed + 1))
  else
    echo "OK $file $ms"
  fi
done <"$manifest"
[[ $failed -eq 0 ]]
