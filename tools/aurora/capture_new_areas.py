#!/usr/bin/env python3
"""Create areas with Aurora's Area Wizard, to capture its defaults.

Needs Aurora running on the headless display with a module open and the main
window idle (tools/aurora/headless.sh, run-aurora.sh). Each job picks the
tileset at a position in the wizard's list (sorted by display name) and a
size; the wizard's default name is kept. Save the module afterwards.

    DISPLAY=:1 capture_new_areas.py INDEX:WIDTHxHEIGHT [...]
    DISPLAY=:1 capture_new_areas.py 0-32:4x4    # a range of list positions

Coordinates are those of Aurora 89.8193.37 at its default window layout.
Aurora sometimes fails generating an area (an access violation); the job is
retried. It always fails where no tile fits (e.g. Lizardfolk Interior at
5×5), so such jobs give up after five tries.

The capture the tests use (`captures/new-module-and-areas.mod`, see
crates/mg-corpus-tests/tests/aurora_new.rs) was made from Aurora's Welcome
dialog: Create a new Module, name "My Test Module", the Module Wizard's Area
Wizard with Castle Interior at Small (area001), Finish, File > Save; then

    3:3x3 3:2x2 3:2x2 3:5x2 3:7x7 3:8x8 3:6x3 0-32:4x4 24:8x8 24:8x8 5:8x8

(area002 to area044; the first 2×2 was asked for as 1×1, below the minimum)
and the saved module copied from the oracle's userdir/modules.
"""

import os
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
import xdrive  # noqa: E402

FILE_MENU = (18, 41)
SAVE_ITEM = (49, 135)
WIZARDS_MENU = (284, 41)
AREA_WIZARD_ITEM = (322, 63)
FIRST_TILESET = (322, 223)
NEXT = (367, 461)
FINISH = (448, 461)
HEIGHT_BOX = (259, 219)
WIDTH_BOX = (339, 339)
OPEN_IN_VIEWER = (233, 401)
PROPERTIES_DIALOG = (233, 377)
ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))


def window(title):
    return any(title == name for _, name, *_ in xdrive.viewable_windows())


def wait_for(pred, seconds, what):
    deadline = time.time() + seconds
    while time.time() < deadline:
        if pred():
            return
        time.sleep(0.2)
    raise SystemExit(f"timed out waiting for {what}")


def checked(x, y):
    """Whether the checkbox at (x, y) shows a check mark (dark pixels)."""
    path = "/tmp/claude-1000/aurora-check.png" if os.path.isdir("/tmp/claude-1000") else "/tmp/aurora-check.png"
    xdrive.shot(path)
    from PIL import Image

    img = Image.open(path).convert("L")
    dark = sum(1 for dx in range(-4, 5) for dy in range(-4, 5) if img.getpixel((x + dx, y + dy)) < 80)
    return dark > 6


def set_box(xy, value):
    xdrive.click(*xy)
    xdrive.key("End")
    xdrive.key("shift+Home")
    xdrive.type_text(str(value))


def area_count():
    temp = os.path.join(ORACLE, "userdir", "modules", "temp0")
    return sum(1 for n in os.listdir(temp) if n.lower().endswith(".are"))


def crash_dialog():
    """Aurora's access-violation message box, if shown: (x, y) of its OK button."""
    for _, name, x, y, w, h in xdrive.viewable_windows():
        if name.startswith("BioWare Aurora") and (w, h) == (317, 82):
            return x + 160, y + 61
    return None


def save():
    xdrive.click(*FILE_MENU)
    time.sleep(0.5)
    xdrive.click(*SAVE_ITEM)
    wait_for(lambda: not any(n.endswith("*") for _, n, *_ in xdrive.viewable_windows()), 60, "the save")


def create(index, width, height):
    """Runs the wizard once; returns whether Aurora added the area. Aurora
    sometimes fails while generating it (access violation at 005AF8E8)."""
    before = area_count()
    xdrive.click(*WIZARDS_MENU)
    time.sleep(0.5)
    xdrive.click(*AREA_WIZARD_ITEM)
    wait_for(lambda: window("Area Wizard"), 20, "the Area Wizard")
    time.sleep(0.5)
    xdrive.click(*FIRST_TILESET)
    xdrive.key("Home")
    for _ in range(index):
        xdrive.key("Down")
    xdrive.click(*NEXT)
    time.sleep(0.8)
    set_box(HEIGHT_BOX, height)
    set_box(WIDTH_BOX, width)
    xdrive.click(*NEXT)
    time.sleep(0.8)
    for box in (OPEN_IN_VIEWER, PROPERTIES_DIALOG):
        if checked(*box):
            xdrive.click(*box)
            time.sleep(0.3)
    xdrive.click(*FINISH)
    wait_for(lambda: not window("Area Wizard"), 120, "the wizard to close")
    wait_for(lambda: area_count() > before or crash_dialog(), 120, "the new area")
    time.sleep(1)
    ok = crash_dialog()
    if ok:
        xdrive.click(*ok)
        time.sleep(1)
        return False
    return True


def main(args):
    jobs = []
    for a in args:
        which, size = a.split(":")
        w, h = (int(n) for n in size.lower().split("x"))
        indices = range(int(which.split("-")[0]), int(which.split("-")[1]) + 1) if "-" in which else [int(which)]
        jobs += [(i, w, h) for i in indices]
    for i, w, h in jobs:
        for attempt in range(5):
            print(f"tileset #{i} {w}x{h}" + (f" (retry {attempt})" if attempt else ""), flush=True)
            if create(i, w, h):
                break
        else:
            raise SystemExit(f"tileset #{i} {w}x{h}: Aurora kept failing")
        save()


if __name__ == "__main__":
    main(sys.argv[1:])
