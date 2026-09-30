#!/usr/bin/env python3
"""Drive X11 programs on the oracle's headless display (see headless.sh).

    xdrive.py windows                 # viewable top-level windows: id, title, geometry
    xdrive.py shot OUT.png [--scale S] # screenshot of the whole display
    xdrive.py winshot TITLE OUT.png   # screenshot of one window (title contains TITLE)
    xdrive.py click X Y [--right|--double]
    xdrive.py key KEY [KEY...]        # e.g. Return, Tab, alt+f, ctrl+s
    xdrive.py type TEXT               # types printable ASCII text
    xdrive.py wait TITLE [SECONDS]    # waits until a window title contains TITLE

The display comes from $DISPLAY (headless.sh prints it). Input goes through
XTEST, so it reaches whatever window is under the pointer or focused; nothing
here touches the user's own session.
"""

import sys
import time

from PIL import Image
from Xlib import X, XK, display
from Xlib.ext import xtest

d = display.Display()
root = d.screen().root


def viewable_windows(titled_only=True):
    """Viewable top-level windows, bottom to top: those with a title, or
    also untitled ones (menus, popups) when `titled_only` is false."""
    out = []
    for w in root.query_tree().children:
        try:
            attrs = w.get_attributes()
            if attrs.map_state != X.IsViewable:
                continue
            name = w.get_wm_name() or ""
            if titled_only and not name:
                continue
            g = w.get_geometry()
            pos = w.translate_coords(root, 0, 0)
        except Exception:
            continue
        out.append((w, name, -pos.x, -pos.y, g.width, g.height))
    return out


def shot(path, scale=1.0):
    g = root.get_geometry()
    canvas = Image.new("RGB", (g.width, g.height), (40, 40, 40))
    for w, _name, x, y, width, height in viewable_windows(titled_only=False):
        if width < 2 or height < 2:
            continue
        try:
            raw = w.get_image(0, 0, width, height, X.ZPixmap, 0xFFFFFFFF)
        except Exception:
            continue
        img = Image.frombytes("RGB", (width, height), raw.data, "raw", "BGRX")
        canvas.paste(img, (x, y))
    if scale != 1.0:
        canvas = canvas.resize((int(g.width * scale), int(g.height * scale)))
    canvas.save(path)


def winshot(title, path):
    """The contents of the first window whose title contains `title` (also
    where other windows cover it)."""
    for w, name, _x, _y, width, height in viewable_windows():
        if title in name:
            raw = w.get_image(0, 0, width, height, X.ZPixmap, 0xFFFFFFFF)
            Image.frombytes("RGB", (width, height), raw.data, "raw", "BGRX").save(path)
            return 0
    return 1


def move(x, y):
    xtest.fake_input(d, X.MotionNotify, x=x, y=y)
    d.sync()


def click(x, y, button=1, count=1):
    move(x, y)
    time.sleep(0.05)
    for _ in range(count):
        xtest.fake_input(d, X.ButtonPress, button)
        d.sync()
        time.sleep(0.08)
        xtest.fake_input(d, X.ButtonRelease, button)
        d.sync()
        time.sleep(0.08)


def keycode(name):
    sym = XK.string_to_keysym(name)
    if sym == 0 and len(name) == 1:
        sym = ord(name)
    code = d.keysym_to_keycode(sym)
    if code == 0:
        raise SystemExit(f"unknown key {name!r}")
    return code


MODS = {"ctrl": "Control_L", "alt": "Alt_L", "shift": "Shift_L"}


def key(combo):
    parts = combo.split("+")
    mods = [keycode(MODS[p.lower()]) for p in parts[:-1]]
    code = keycode(parts[-1])
    for m in mods:
        xtest.fake_input(d, X.KeyPress, m)
    xtest.fake_input(d, X.KeyPress, code)
    d.sync()
    time.sleep(0.03)
    xtest.fake_input(d, X.KeyRelease, code)
    for m in reversed(mods):
        xtest.fake_input(d, X.KeyRelease, m)
    d.sync()
    time.sleep(0.05)


SHIFTED = {'~': '`', '!': '1', '@': '2', '#': '3', '$': '4', '%': '5', '^': '6', '&': '7',
           '*': '8', '(': '9', ')': '0', '_': 'minus', '+': 'equal', '{': 'bracketleft',
           '}': 'bracketright', '|': 'backslash', ':': 'semicolon', '"': 'apostrophe',
           '<': 'comma', '>': 'period', '?': 'slash'}
NAMES = {' ': 'space', '-': 'minus', '=': 'equal', '[': 'bracketleft', ']': 'bracketright',
         '\\': 'backslash', ';': 'semicolon', "'": 'apostrophe', ',': 'comma', '.': 'period',
         '/': 'slash', '`': 'grave', '\n': 'Return', '\t': 'Tab'}


def type_text(text):
    for ch in text:
        if ch.isupper():
            key("shift+" + ch.lower())
        elif ch in SHIFTED:
            key("shift+" + SHIFTED[ch])
        else:
            key(NAMES.get(ch, ch))


def main(argv):
    cmd, args = argv[1], argv[2:]
    if cmd == "windows":
        for w, name, x, y, width, height in viewable_windows():
            print(f"{w.id:#x}\t{x},{y} {width}x{height}\t{name}")
    elif cmd == "shot":
        scale = float(args[args.index("--scale") + 1]) if "--scale" in args else 1.0
        shot(args[0], scale)
    elif cmd == "winshot":
        return winshot(args[0], args[1])
    elif cmd == "click":
        x, y = int(args[0]), int(args[1])
        click(x, y, 3 if "--right" in args else 1, 2 if "--double" in args else 1)
    elif cmd == "key":
        for k in args:
            key(k)
    elif cmd == "type":
        type_text(" ".join(args))
    elif cmd == "wait":
        deadline = time.time() + (float(args[1]) if len(args) > 1 else 30)
        while time.time() < deadline:
            if any(args[0] in name for _, name, *_ in viewable_windows()):
                return 0
            time.sleep(0.25)
        return 1
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
