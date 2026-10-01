#!/bin/sh
# Builds Moonglow-<version>-<arch>.AppImage in target/dist/.
#
# Needs appimagetool (https://github.com/AppImage/appimagetool/releases):
# on PATH, or named by $APPIMAGETOOL. Build on the oldest distribution the
# image should run on (CI uses Ubuntu 22.04): the image runs where the
# build machine's glibc and libstdc++ are no newer than the system's.
# Without appimagetool, the AppDir is left in target/dist/AppDir.
set -eu
cd "$(dirname "$0")/../.."
ID=io.github.moonglow_toolset.Moonglow
ARCH=$(uname -m)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
OUT=target/dist
APPDIR=$OUT/AppDir

cargo build --profile dist --locked -p moonglow -p mg
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
    "$APPDIR/usr/share/metainfo" "$APPDIR/usr/share/mime/packages" \
    "$APPDIR/usr/share/icons/hicolor/scalable/apps" "$APPDIR/usr/share/doc/moonglow"
install -m 755 target/dist/moonglow target/dist/mg "$APPDIR/usr/bin/"
install -m 644 packaging/linux/$ID.desktop "$APPDIR/usr/share/applications/"
install -m 644 packaging/linux/$ID.metainfo.xml "$APPDIR/usr/share/metainfo/$ID.appdata.xml"
install -m 644 packaging/linux/moonglow-mime.xml "$APPDIR/usr/share/mime/packages/$ID.xml"
install -m 644 packaging/icons/moonglow.svg "$APPDIR/usr/share/icons/hicolor/scalable/apps/$ID.svg"
for s in 16 24 32 48 64 128 256 512; do
    d="$APPDIR/usr/share/icons/hicolor/${s}x$s/apps"
    mkdir -p "$d"
    install -m 644 packaging/icons/moonglow-$s.png "$d/$ID.png"
done
install -m 644 LICENSE "$APPDIR/usr/share/doc/moonglow/"
python3 packaging/third_party_licenses.py > "$APPDIR/usr/share/doc/moonglow/THIRD-PARTY-LICENSES.txt"
cp -r docs/manual "$APPDIR/usr/share/doc/moonglow/manual" 2>/dev/null || true

# The AppImage's top: the desktop entry, its icon and the start script.
ln -s usr/share/applications/$ID.desktop "$APPDIR/$ID.desktop"
ln -s usr/share/icons/hicolor/256x256/apps/$ID.png "$APPDIR/$ID.png"
ln -s $ID.png "$APPDIR/.DirIcon"
cat > "$APPDIR/AppRun" <<'RUN'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
exec "$HERE/usr/bin/moonglow" "$@"
RUN
chmod 755 "$APPDIR/AppRun"

TOOL=${APPIMAGETOOL:-$(command -v appimagetool || true)}
if [ -z "$TOOL" ]; then
    echo "appimagetool not found: the AppDir is in $APPDIR" >&2
    exit 1
fi
# --no-appstream: the metainfo has no homepage yet, which appstreamcli
# reports as a warning and appimagetool as a failure.
ARCH=$ARCH "$TOOL" --no-appstream "$APPDIR" "$OUT/Moonglow-$VERSION-$ARCH.AppImage"
echo "$OUT/Moonglow-$VERSION-$ARCH.AppImage"
