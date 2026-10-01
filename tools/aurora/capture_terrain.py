#!/usr/bin/env python3
"""Captures how Aurora paints terrain, for tests/aurora_terrain.rs.

    D=$(tools/aurora/headless.sh start)
    DISPLAY=$D tools/aurora/capture_terrain.py [AREA...]

Writes a probe module (examples/terrain_probe_module.rs: a 10x10 area per
scenario of crates/mg-corpus-tests/tests/data/terrain_scenarios.json) into
the oracle's module folder, starts Aurora on the off-screen display, opens
the module and plays each scenario's steps in its area with the palette's
brushes, saving the module after every step: captures go to
~/.local/share/moonglow-oracle/captures/terrain/<area>/NN.mod (00 before the
first step). Aurora is stopped at the end.

Only for the oracle's own Wine prefix and display (see run-aurora.sh).
"""

import json
import os
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import xdrive as x  # noqa: E402

ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))
MODULE = "zz_terrain"
MOD = os.path.join(ORACLE, "userdir", "modules", MODULE + ".mod")
CAPTURES = os.path.join(ORACLE, "captures", "terrain")
SCENARIOS = os.environ.get("TERRAIN_SCENARIOS", os.path.join(ROOT, "crates/mg-corpus-tests/tests/data/terrain_scenarios.json"))

# Aurora's main window at the headless display's size (1280x1024).
VIEW = (215, 110, 1035, 890)  # the area view
VIEW_CENTRE = (626, 500)
ZOOM_OUT = (485, 920)
PALETTE_TERRAIN = (1098, 210)  # the Terrain node, Features and Groups closed
PALETTE_ROW = 16


def wait_window(title, seconds=30):
    deadline = time.time() + seconds
    while time.time() < deadline:
        if any(title in name for _, name, *_ in x.viewable_windows()):
            return True
        time.sleep(0.25)
    return False


def start():
    subprocess.Popen([os.path.join(HERE, "run-aurora.sh")], stdout=subprocess.DEVNULL,
                     stderr=subprocess.DEVNULL)
    if not wait_window("BioWare Aurora", 90):
        raise SystemExit("Aurora did not start")
    time.sleep(8)
    # A module left open by an earlier run: don't recover it.
    if wait_window("Confirmation", 5):
        x.click(642, 561)
        time.sleep(2)


def open_module(areas):
    x.key("ctrl+o")
    if not wait_window("BioWare Aurora Neverwinter Nights Toolset", 10):
        raise SystemExit("no Open Module dialog")
    time.sleep(2)
    # The module list: the last name, selected by typing it.
    x.click(545, 520)
    x.type_text(MODULE)
    time.sleep(0.5)
    x.click(695, 703)
    if not wait_window(MODULE + ".mod", 60):
        raise SystemExit("the module did not open")
    time.sleep(5)
    x.click(12, 88)  # expand Areas
    time.sleep(1)


# The status bar's digits (Tahoma 8 under Wine): 5 by 8 pixels in 6-pixel
# slots, learnt from a sweep across an area.
DIGITS = [
    "####.#..#.#..###..###..###..###..#.####.", "###...##....#....#....#....#....#..####.",
    "####....##...##...#...##..##..##...#####", "####....#....#.####....#....#....#.###..",
    "..##..###.####.#..#.#####..##...##...##.", "####.#....#....####....#....##...#.####.",
    ".###.#....#....####.#..###..###..######.", "#####...#....#...##...#...##...#....#...",
    "####.#..#.#..#.####.#..#.#..###..######.", "####.#..###..#######....#...###..#.###..",
]


def mouse():
    """Where the pointer is in the area, whole metres (truncated), from the
    status bar's `Mouse(x:X y:Y)`; None off the area."""
    from PIL import Image
    path = os.path.join(CAPTURES, "_status.png")
    x.shot(path)
    im = Image.open(path).convert("RGB").crop((0, 1012, 140, 1020))
    os.remove(path)
    bits = [["#" if sum(im.getpixel((c, r))) < 450 else "." for c in range(140)] for r in range(8)]

    def number(c):
        out = ""
        while c + 5 <= 140:
            glyph = "".join("".join(row[c:c + 5]) for row in bits)
            errors = [sum(a != b for a, b in zip(t, glyph)) for t in DIGITS]
            if min(errors) > 3:
                break
            out += str(errors.index(min(errors)))
            c += 6
        return out

    mx = number(48)
    my = number(48 + 6 * len(mx) + 12) if mx else ""
    return (int(mx), int(my)) if mx and my else None


class Pointer:
    """Puts the pointer over a point of the area, steering by the status
    bar (which reads what the pointer is over, raised ground included)."""

    def __init__(self, n):
        self.n = n
        a = self.read(VIEW_CENTRE)
        b = self.read((VIEW_CENTRE[0] + 120, VIEW_CENTRE[1] + 120))
        if a is None or b is None or a[0] == b[0] or a[1] == b[1]:
            raise SystemExit(f"cannot read the pointer's position ({a}, {b})")
        self.px_per_m = (120 / (b[0] - a[0]), 120 / (a[1] - b[1]))
        self.centre = a

    @staticmethod
    def read(p):
        x.move(*p)
        time.sleep(0.35)
        return mouse()

    def aim(self, wx, wy, tolerance):
        """Screen position where the status bar reads (wx, wy) (metres;
        within the area: its far edges read a metre short), or failing
        that, the nearest within `tolerance` metres seen on the way (water
        reads unevenly)."""
        wx, wy = (min(max(v, 0), 10 * self.n - 1) for v in (wx, wy))
        sx, sy = self.px_per_m
        p = (VIEW_CENTRE[0] + (wx + 0.5 - self.centre[0]) * sx,
             VIEW_CENTRE[1] - (wy + 0.5 - self.centre[1]) * sy)
        best = None
        for _ in range(10):
            q = (int(round(p[0])), int(round(p[1])))
            r = self.read(q)
            if r is None:
                p = ((p[0] + VIEW_CENTRE[0]) / 2, (p[1] + VIEW_CENTRE[1]) / 2)
                continue
            off = max(abs(wx - r[0]), abs(wy - r[1]))
            if best is None or off < best[0]:
                best = (off, q)
            if off == 0:
                return q
            p = (p[0] + (wx - r[0]) * sx, p[1] - (wy - r[1]) * sy)
        if best and best[0] <= tolerance:
            return best[1]
        raise SystemExit(f"cannot put the pointer at ({wx}, {wy})")

    def corner(self, i, j):
        return self.aim(10 * i, 10 * j, 2)

    def cell(self, i, j):
        return self.aim(10 * i + 5, 10 * j + 5, 3)


def brush(palette, label):
    i = palette.index(label)
    # Near the label's start: tree items select only on their text.
    x.click(PALETTE_TERRAIN[0] + 7, PALETTE_TERRAIN[1] + PALETTE_ROW * (i + 1))
    time.sleep(0.5)


def press(button):
    x.xtest.fake_input(x.d, x.X.ButtonPress, button)
    x.d.sync()


def release(button):
    x.xtest.fake_input(x.d, x.X.ButtonRelease, button)
    x.d.sync()


def glide(to):
    q = x.root.query_pointer()
    sx, sy = q.root_x, q.root_y
    for k in range(1, 11):
        x.move(int(sx + (to[0] - sx) * k / 10), int(sy + (to[1] - sy) * k / 10))
        time.sleep(0.03)
    time.sleep(0.2)


def approach(p, towards=None):
    """Glides onto `p`, from the side of `towards`: the area view follows
    the pointer only as it moves (a crosser brush's cursor stays where it
    was after a jump, and a drag would start there), and a crosser drag
    starts from the quarter of the cell the pointer was last in."""
    if towards is None:
        x.move(p[0] - 15, p[1] - 15)
    else:
        x.move(int(p[0] + 0.3 * (towards[0] - p[0])), int(p[1] + 0.3 * (towards[1] - p[1])))
    time.sleep(0.2)
    glide(p)
    time.sleep(0.4)


def play(pointer, palette, step):
    brush(palette, step["brush"])
    if "at" in step or "cell" in step:
        p = pointer.corner(*step["at"]) if "at" in step else pointer.cell(*step["cell"])
        approach(p)
        x.click(*p, 3 if step.get("lower") else 1)
    else:
        cells = [pointer.cell(*c) for c in step["path"]]
        approach(cells[0], cells[1])
        press(1)
        time.sleep(0.3)
        for c in cells[1:]:
            glide(c)
        release(1)
    time.sleep(1)


def save_to(path):
    before = os.path.getmtime(MOD)
    x.key("ctrl+s")
    deadline = time.time() + 15
    while time.time() < deadline and os.path.getmtime(MOD) == before:
        time.sleep(0.25)
    time.sleep(1)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    shutil.copy(MOD, path)


def main(only):
    spec = json.load(open(SCENARIOS))
    scenarios = [s for s in spec["scenarios"] if not only or s["area"] in only]
    n = spec["size"]
    areas = sorted(s["area"] for s in spec["scenarios"])
    subprocess.run(["cargo", "run", "-q", "--release", "-p", "mg-corpus-tests", "--example",
                    "terrain_probe_module", "--", MOD]
                   + [f"{a}:{n}x{n}" for a in areas], cwd=ROOT, check=True)
    start()
    try:
        open_module(areas)
        # The tree lists START, then the areas by name; an opened area
        # expands below itself, so the last is opened first.
        tree = ["start"] + areas
        for s in sorted(scenarios, key=lambda s: s["area"], reverse=True):
            area = s["area"]
            out = os.path.join(CAPTURES, area)
            shutil.rmtree(out, ignore_errors=True)
            x.click(59, 103 + 16 * tree.index(area), 1, 2)
            time.sleep(8)
            x.click(*PALETTE_TERRAIN, 1, 2)
            time.sleep(1)
            brush(s["palette"], "Eraser")
            # Out until the whole area shows.
            for _ in range(12):
                pointer = Pointer(n)
                if n * 10 * max(pointer.px_per_m) < 0.85 * (VIEW[3] - VIEW[1]):
                    break
                x.click(*ZOOM_OUT)
                time.sleep(1)
            print(f"{area}: {pointer.px_per_m[0]:.2f} px a metre")
            save_to(os.path.join(out, "00.mod"))
            for i, step in enumerate(s["steps"], 1):
                play(pointer, s["palette"], step)
                save_to(os.path.join(out, f"{i:02}.mod"))
                print(f"  {i:02} {step}")
    finally:
        subprocess.run([os.path.join(HERE, "run-aurora.sh"), "kill"])


if __name__ == "__main__":
    main(sys.argv[1:])
