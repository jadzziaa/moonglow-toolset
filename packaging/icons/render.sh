#!/bin/sh
# Renders the icon (moonglow.svg) into what the packages need: PNGs for
# the window and the Linux icon theme, moonglow.ico (Windows) and
# moonglow.icns (macOS). Needs rsvg-convert and Pillow (python3).
set -eu
cd "$(dirname "$0")"
for s in 16 24 32 48 64 128 256 512; do
    rsvg-convert -w $s -h $s moonglow.svg -o moonglow-$s.png
done
python3 - <<'PY'
from PIL import Image
big = Image.open("moonglow-512.png")
sizes = [16, 24, 32, 48, 64, 128, 256]
big.save("moonglow.ico", sizes=[(s, s) for s in sizes],
         append_images=[Image.open(f"moonglow-{s}.png") for s in sizes])
big.save("moonglow.icns")
PY
