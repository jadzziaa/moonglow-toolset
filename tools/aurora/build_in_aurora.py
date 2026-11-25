#!/usr/bin/env python3
"""Build modules' scripts in Aurora and save them, unattended: open each
module, Build > Build Module with only Compile > Scripts (Advanced
Controls), Done, File > Save, and copy the saved module out.

    tools/locks.sh hold display -- tools/aurora/build_in_aurora.sh SHOTS NAME=OUT.mod...

(build_in_aurora.sh starts the off-screen display and Aurora, runs this,
and stops both.) NAME is a module in the oracle's `userdir/modules/`
(without `.mod`); OUT is where the saved module is copied. SHOTS is a
folder for a picture of each step and the log, to see where a run
went wrong. Dialogs are found by their titles and their controls by the
positions in Aurora's forms (uiinv/forms.json: TDLGWELCOME,
TDLGMODULESELECT, TDLGVERIFYMODULE), checked with OCR (tesseract).
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(__file__))
import xdrive  # noqa: E402

ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))
MODULES = os.path.join(ORACLE, "userdir", "modules")
SHOTS = None
STEP = [0]


def log(*a):
    line = time.strftime("%H:%M:%S ") + " ".join(str(x) for x in a)
    print(line, flush=True)
    with open(os.path.join(SHOTS, "log.txt"), "a") as f:
        f.write(line + "\n")


def shot(label):
    STEP[0] += 1
    path = os.path.join(SHOTS, f"{STEP[0]:02d}-{label}.png")
    xdrive.shot(path)
    log("shot", path, "windows:", [(n, x, y, w, h) for _w, n, x, y, w, h in windows()])
    return path


def windows():
    return xdrive.viewable_windows()


def new_window(before):
    """A dialog that was not among `before` (by title)."""
    names = {w[1] for w in before}
    for w in reversed(windows()):
        if w[1] and w[1] not in names and w[4] > 10:
            return w
    return None


def find(title):
    for w in reversed(windows()):
        if title in w[1]:
            return w
    return None


def wait_for(pred, seconds, what):
    end = time.time() + seconds
    while time.time() < end:
        r = pred()
        if r:
            return r
        time.sleep(0.5)
    shot("timeout")
    raise SystemExit(f"timed out waiting for {what}")


def ocr(win=None):
    """Words on the display (or in window `win`): (text, cx, cy) in display pixels."""
    with tempfile.TemporaryDirectory() as t:
        p = os.path.join(t, "s.png")
        xdrive.shot(p, scale=2.0)
        out = subprocess.run(["tesseract", p, "-", "tsv"], capture_output=True, text=True).stdout
    words = []
    for line in out.splitlines()[1:]:
        f = line.split("\t")
        if len(f) < 12 or not f[11].strip():
            continue
        x, y, w, h = (int(v) // 2 for v in f[6:10])
        if win is not None:
            _w, _n, wx, wy, ww, wh = win
            if not (wx <= x < wx + ww and wy <= y < wy + wh):
                continue
        words.append((f[11].strip(), x + w // 2, y + h // 2, x, y))
    return words


def word(win, text, nth=0):
    hits = [w for w in ocr(win) if w[0].rstrip(":") == text]
    return hits[nth] if len(hits) > nth else None


def click_at(win, cx, cy, label):
    _w, _n, wx, wy, _ww, _wh = win
    xdrive.click(wx + cx, wy + cy)
    time.sleep(0.4)
    log("click", label, wx + cx, wy + cy)


def main_window():
    tops = [w for w in windows() if "Toolset" in w[1]]
    return max(tops, key=lambda w: w[4] * w[5]) if tops else None


def start():
    """Waits for Aurora, closes the welcome screen."""
    welcome = wait_for(lambda: find("Toolset"), 240, "Aurora")
    time.sleep(8)
    shot("started")
    # The welcome screen is the dialog of that title over the main window,
    # if any: Cancel it (Escape).
    # "The toolset did not close properly last time... recover it?" (an
    # earlier run was stopped, leaving temp0): No.
    end = time.time() + 30
    while time.time() < end:
        conf = find("Confirmation")
        if conf:
            click_at(conf, 167, 93, "recover? No")
            time.sleep(3)
            continue
        tops = [w for w in windows() if "Toolset" in w[1] and w[4] > 10 and w != main_window()]
        if tops:
            click_at(tops[0], 284 + 37, 364 + 12, "welcome Cancel")
            time.sleep(2)
            continue
        if word(None, "Welcome!") is None:
            break
        time.sleep(1)
    shot("welcome-closed")
    return welcome


def open_module(name):
    main = main_window()
    _w, _n, x, y, w, h = main
    xdrive.click(x + w // 2, y + h // 2 + 200)  # focus the main window (an empty part)
    time.sleep(0.5)
    before = windows()
    xdrive.key("ctrl+o")
    dlg = wait_for(lambda: new_window(before), 30, "Open Module")
    log("open dialog:", dlg[1])
    time.sleep(1)
    shot("open-dialog")
    click_at(dlg, 104 + 116, 360 + 10, "Module Name")
    xdrive.key("End")
    xdrive.key("shift+Home")
    xdrive.key("BackSpace")
    xdrive.type_text(name)
    time.sleep(0.5)
    shot("open-typed")
    xdrive.key("Return")
    # Loaded: the main window's title names the module, or its area list
    # shows; wait until the display stops changing.
    time.sleep(5)
    settle(120)
    for _ in range(5):  # popups (a missing hak, a warning): OK them
        extra = [w for w in windows() if w[1] and w[4] > 10 and "Toolset" not in w[1]]
        if not extra:
            break
        log("popup", [w[1] for w in extra])
        xdrive.key("Return")
        time.sleep(2)
    shot("opened")


def settle(seconds, quiet=6):
    """Waits until the display has not changed for `quiet` seconds."""
    end = time.time() + seconds
    last, same_since = None, time.time()
    with tempfile.TemporaryDirectory() as t:
        p = os.path.join(t, "s.png")
        while time.time() < end:
            xdrive.shot(p, scale=0.25)
            data = open(p, "rb").read()
            if data != last:
                last, same_since = data, time.time()
            elif time.time() - same_since >= quiet:
                return True
            time.sleep(5)
    return False


def build():
    xdrive.key("alt+b")
    time.sleep(1)
    shot("build-menu")
    before = windows()
    # Build > Build Module, the menu's second item.
    hit = [w for w in ocr() if w[0] == "Module" and w[2] < 120]
    if hit:
        xdrive.click(hit[0][1], hit[0][2])
    else:
        click_at(main_window(), 241, 55, "Build Module (menu)")
    dlg = wait_for(lambda: new_window(before), 30, "Build Module")
    title = dlg[1]
    log("build dialog:", title)
    time.sleep(1)
    shot("build-dialog")
    # Aurora keeps these settings from run to run: each box is set as
    # wanted, read from the picture (a tick is dark pixels in the box, left
    # of its label).
    def checked(cx, cy):
        with tempfile.TemporaryDirectory() as t:
            p = os.path.join(t, "s.png")
            xdrive.shot(p)
            from PIL import Image
            img = Image.open(p).convert("L")
            dark = sum(1 for dx in range(-4, 5) for dy in range(-4, 5)
                       if img.getpixel((cx + dx, cy + dy)) < 90)
        return dark > 4

    def set_box(label, want, column=None):
        # (The dialog grows and moves as Advanced Controls opens: found
        # again each time.)
        words = ocr(find(title))
        hits = [w for w in words if w[0] == label]
        if column is not None:
            hits = [w for w in hits if abs(w[3] - column) < 30]
        if not hits:
            log("not found:", label)
            return None
        w = hits[0]
        cx, cy = w[3] - 13, w[2]
        if checked(cx, cy) != want:
            xdrive.click(cx, cy)
            time.sleep(0.8)
        log("box", label, "wanted", want, "now", checked(cx, cy))
        return w

    set_box("Advanced", True)
    time.sleep(1)
    set_box("Unused", False)
    set_box("Missing", False)
    set_box("Spell", False)
    comp = set_box("Compile", True)
    col = comp[3] + 16 if comp else None
    set_box("Scripts", True, col)
    set_box("Creature", False, col)
    set_box("Encounters", False, col)
    set_box("Palettes", False, col)
    shot("filters")
    dlg = find(title)
    b = [w for w in ocr(dlg) if w[0] == "Build"]
    b = max(b, key=lambda w: w[2]) if b else None
    if b:
        xdrive.click(b[1], b[2])
    else:
        _w, _n, x, y, ww, hh = dlg
        xdrive.click(x + ww - 130, y + hh - 18)
    log("Build pressed")
    # The main window's log says "Building Module..." at once, or the
    # click missed: stop rather than hold the display for nothing.
    wait_for(lambda: any(w[0] == "Building" for w in ocr()) or (time.sleep(2) and False),
             30, "Building Module (the Build click missed?)")
    log("building")
    # Done when the display has stopped changing for a while.
    settle(7200, quiet=30)
    # (and the main window's log says "Finished Building Module")
    wait_for(lambda: any(w[0] == "Finished" for w in ocr()) or (time.sleep(10) and False),
             7200, "Finished Building Module")
    shot("built")
    dlg = find(title)
    d = [w for w in ocr(dlg) if w[0] == "Done"]
    if d:
        xdrive.click(d[0][1], d[0][2])
    else:
        _w, _n, x, y, ww, hh = dlg
        xdrive.click(x + ww - 46, y + hh - 18)
    time.sleep(2)
    shot("done")


def save(name, out):
    path = os.path.join(MODULES, name + ".mod")
    before = os.path.getmtime(path)
    xdrive.key("ctrl+s")
    wait_for(lambda: os.path.getmtime(path) != before, 600, "the save")
    time.sleep(3)
    settle(120)
    wait_for(lambda: os.path.getmtime(path) == before or (time.time() - os.path.getmtime(path)) > 5,
             120, "the save to end")
    shot("saved")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    shutil.copy(path, out)
    log("saved", path, "->", out)


def main(argv):
    global SHOTS
    SHOTS = argv[1]
    os.makedirs(SHOTS, exist_ok=True)
    start()
    for job in argv[2:]:
        name, out = job.split("=", 1)
        log("module", name)
        open_module(name)
        build()
        save(name, out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
