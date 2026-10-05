#!/usr/bin/env bash
# Point the Scoop manifest (bucket/moonglow.json) at a published release:
#
#   packaging/scoop/update-manifest.sh 1.11.0
#
# The version and the SHA-256 of the release's Windows zip (the installer's
# files, to unpack: Scoop's unpacker can't read the installer itself),
# which GitHub gives for the asset (nothing is downloaded). Run once the release is
# published, then commit the manifest: Scoop users get the release with
# `scoop update moonglow`.
set -euo pipefail

version=${1:?usage: update-manifest.sh VERSION}
repo=jadzziaa/moonglow-toolset
asset="Moonglow-$version-windows-x64.zip"
manifest="$(dirname "$0")/../../bucket/moonglow.json"

digest=$(gh release view "v$version" --repo "$repo" --json assets \
  --jq ".assets[] | select(.name == \"$asset\") | .digest")
hash=${digest#sha256:}
if [[ ! $hash =~ ^[0-9a-f]{64}$ ]]; then
  echo "no SHA-256 for $asset in release v$version" >&2
  exit 1
fi

cat > "$manifest" <<JSON
{
    "version": "$version",
    "description": "A module toolset for Neverwinter Nights: Enhanced Edition (the Aurora Toolset, rebuilt)",
    "homepage": "https://github.com/$repo",
    "license": "GPL-3.0-only",
    "notes": "Moonglow needs an installation of Neverwinter Nights: Enhanced Edition and reads the game's files from it.",
    "architecture": {
        "64bit": {
            "url": "https://github.com/$repo/releases/download/v$version/$asset",
            "hash": "$hash"
        }
    },
    "bin": "mg.exe",
    "shortcuts": [
        [
            "moonglow.exe",
            "Moonglow Toolset"
        ]
    ],
    "checkver": {
        "github": "https://github.com/$repo"
    },
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/$repo/releases/download/v\$version/Moonglow-\$version-windows-x64.zip"
            }
        }
    }
}
JSON
echo "bucket/moonglow.json: $version ($hash)"
