#!/bin/sh
# Builds "Moonglow Toolset.app" and Moonglow-<version>-macos.dmg in
# target/dist/, for Apple silicon and Intel Macs (one universal binary).
# Run on macOS with both Rust targets installed:
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
# The app is signed ad hoc; to pass Gatekeeper without a right-click > Open,
# sign it with a Developer ID (CODESIGN_IDENTITY) and notarize the dmg.
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
OUT=target/dist
APP="$OUT/Moonglow Toolset.app"
export MACOSX_DEPLOYMENT_TARGET=11.0

for t in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --profile dist --locked -p moonglow -p mg --target $t
done
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
for b in moonglow mg; do
    lipo -create -output "$APP/Contents/MacOS/$b" \
        target/aarch64-apple-darwin/dist/$b target/x86_64-apple-darwin/dist/$b
done
sed "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
cp packaging/icons/moonglow.icns "$APP/Contents/Resources/"
cp LICENSE "$APP/Contents/Resources/"
python3 packaging/third_party_licenses.py --target aarch64-apple-darwin \
    > "$APP/Contents/Resources/THIRD-PARTY-LICENSES.txt"
[ -d docs/manual ] && cp -R docs/manual "$APP/Contents/Resources/manual"
codesign --force --deep --options runtime --sign "${CODESIGN_IDENTITY:--}" "$APP"

DMG="$OUT/Moonglow-$VERSION-macos.dmg"
STAGE="$OUT/dmg"
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Moonglow Toolset" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
rm -rf "$STAGE"
echo "$DMG"
