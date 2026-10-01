#!/bin/sh
# Builds target/dist/Moonglow-<version>-<arch>.flatpak, a single-file
# bundle (install it with `flatpak install --user Moonglow-*.flatpak`).
#
# Needs flatpak-builder and, from Flathub, the Freedesktop 25.08 SDK with
# its rust-stable extension (installed for the user when missing). The
# crates come from Cargo's download cache where they are, else crates.io.
set -eu
cd "$(dirname "$0")/../.."
ID=io.github.moonglow_toolset.Moonglow
ARCH=$(flatpak --default-arch)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
OUT=target/dist

python3 packaging/flatpak/cargo_sources.py --extra-sources target/flatpak-crates \
    Cargo.lock > packaging/flatpak/cargo-sources.json
flatpak-builder --user --install-deps-from=flathub --force-clean \
    --state-dir=target/flatpak-builder --repo=target/flatpak-repo \
    --extra-sources=target/flatpak-crates \
    target/flatpak-build packaging/flatpak/$ID.yml
mkdir -p "$OUT"
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    target/flatpak-repo "$OUT/Moonglow-$VERSION-$ARCH.flatpak" $ID
echo "$OUT/Moonglow-$VERSION-$ARCH.flatpak"
