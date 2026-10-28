# Puts Microsoft's DirectX Shader Compiler (DXC) beside Moonglow's programs:
# dxcompiler.dll, x64, of the release pinned here, which Direct3D 12 compiles
# shaders with where it lies beside the program (crates/mg-render/src/dx12.rs;
# FXC, which can't compile the game's normal-mapped lit programs, elsewhere).
#
#   packaging\windows\dxc.ps1 -Destination target\dist [-Licenses DIR]
#
# Downloads the release's zip from GitHub into target\dxc\<tag> (once; kept
# there while its hash matches), checks its SHA-256 and copies
# bin\x64\dxcompiler.dll into each folder of -Destination. dxil.dll is not
# needed (since 1.8.2505 dxcompiler.dll validates and signs on its own) and
# not shipped. -Licenses also copies DXC's license texts there, for
# THIRD-PARTY-LICENSES.txt (packaging\third_party_licenses.py --dxc): the
# zip's LICENSE-LLVM.txt (University of Illinois/NCSA) and LICENCE-MIT.txt
# (Microsoft's MIT), the repository's ThirdPartyNotices.txt at the tag
# (packaging\windows\dxc\), and VERSION.txt. The zip's LICENSE-MS.txt (the
# terms of dxil.dll, Microsoft's closed validator) is left out with it.
#
# A new release: change the three lines below (the hash is the release
# asset's "digest" on GitHub, checked against the file) and the notices in
# packaging\windows\dxc\ThirdPartyNotices.txt to the new tag's.
param(
    [Parameter(Mandatory = $true)] [string[]] $Destination,
    [string] $Licenses
)
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Tag = "v1.9.2609"
$Zip = "dxc_2026_09_29.zip"
$Sha256 = "ad31b1fc8443175d204f77a611fdb3ef2ec42759bdc2f1167368de24a4a7e7f1"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$cache = Join-Path $root "target\dxc\$Tag"
New-Item -ItemType Directory -Force -Path $cache | Out-Null
$archive = Join-Path $cache $Zip
$url = "https://github.com/microsoft/DirectXShaderCompiler/releases/download/$Tag/$Zip"
if (-not (Test-Path $archive) -or (Get-FileHash -Algorithm SHA256 $archive).Hash -ne $Sha256) {
    Write-Host "Downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing
}
$hash = (Get-FileHash -Algorithm SHA256 $archive).Hash
if ($hash -ne $Sha256) {
    Remove-Item -Force $archive
    throw "$Zip has SHA-256 $hash, not ${Sha256}; not used"
}
$unpacked = Join-Path $cache "unpacked"
if (Test-Path $unpacked) { Remove-Item -Recurse -Force $unpacked }
Expand-Archive -Path $archive -DestinationPath $unpacked
$dll = Join-Path $unpacked "bin\x64\dxcompiler.dll"
if (-not (Test-Path $dll)) { throw "no bin\x64\dxcompiler.dll in $Zip" }
foreach ($d in $Destination) {
    New-Item -ItemType Directory -Force -Path $d | Out-Null
    Copy-Item -Force $dll $d
    Write-Host "dxcompiler.dll ($Tag) -> $d"
}
if ($Licenses) {
    New-Item -ItemType Directory -Force -Path $Licenses | Out-Null
    Copy-Item -Force (Join-Path $unpacked "LICENSE-LLVM.txt"), (Join-Path $unpacked "LICENCE-MIT.txt") $Licenses
    Copy-Item -Force (Join-Path $PSScriptRoot "dxc\ThirdPartyNotices.txt") $Licenses
    Set-Content -Path (Join-Path $Licenses "VERSION.txt") -Value $Tag -Encoding ascii
}
