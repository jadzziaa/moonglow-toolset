# Builds Moonglow's Windows installer, target\dist\Moonglow-<version>-windows-x64-setup.exe,
# and the same files as a zip, Moonglow-<version>-windows-x64.zip (to unpack
# anywhere and run; what Scoop installs: bucket\moonglow.json).
# Needs Rust, Python 3 and Inno Setup 6 (iscc on PATH, or in its usual folder).
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..\..")
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value

cargo build --profile dist --locked -p moonglow -p mg
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
python packaging\third_party_licenses.py --target x86_64-pc-windows-msvc `
    --output target\dist\THIRD-PARTY-LICENSES.txt
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# The zip: what the installer puts in its folder.
$portable = "target\dist\portable"
if (Test-Path $portable) { Remove-Item -Recurse -Force $portable }
New-Item -ItemType Directory -Path $portable | Out-Null
Copy-Item target\dist\moonglow.exe, target\dist\mg.exe, target\dist\THIRD-PARTY-LICENSES.txt $portable
Copy-Item LICENSE (Join-Path $portable "LICENSE.txt")
Copy-Item -Recurse docs\manual (Join-Path $portable "manual")
$zip = "target\dist\Moonglow-$version-windows-x64.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path "$portable\*" -DestinationPath $zip
Remove-Item -Recurse -Force $portable

$iscc = (Get-Command iscc -ErrorAction SilentlyContinue).Source
if (-not $iscc) { $iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" }
& $iscc "/DVersion=$version" "/DSource=..\..\target\dist" packaging\windows\moonglow.iss
exit $LASTEXITCODE
