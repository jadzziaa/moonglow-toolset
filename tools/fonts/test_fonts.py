#!/usr/bin/env python3
"""Makes the test fonts in crates/mg-ui/tests/fixtures/fonts: Moonglow's own,
glyphs of plain boxes (no font's outlines copied). `boxes.ttf` has an
ordinary line (ascent 1,900 and descent 500 in an em of 2,048, as the
game's fonts); `sliver.ttf` the same glyphs with a line one unit of
2,048 high (the review of November 2026's N17). Run with fontTools:
`python3 tools/fonts/test_fonts.py`."""
import pathlib
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

OUT = pathlib.Path(__file__).resolve().parents[2] / "crates/mg-ui/tests/fixtures/fonts"


def box(width, height):
    pen = TTGlyphPen(None)
    pen.moveTo((100, 0))
    pen.lineTo((100, height))
    pen.lineTo((width - 100, height))
    pen.lineTo((width - 100, 0))
    pen.closePath()
    return pen.glyph()


def font(name, ascent, descent):
    letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    order = [".notdef", "space"] + [f"g{ord(c)}" for c in letters]
    fb = FontBuilder(2048, isTTF=True)
    fb.setupGlyphOrder(order)
    cmap = {32: "space"} | {ord(c): f"g{ord(c)}" for c in letters}
    fb.setupCharacterMap(cmap)
    glyphs = {".notdef": box(1000, 1400), "space": TTGlyphPen(None).glyph()}
    glyphs |= {f"g{ord(c)}": box(1100, 1400) for c in letters}
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({g: (1100 if g.startswith("g") else 1000, 100) for g in order})
    fb.setupHorizontalHeader(ascent=ascent, descent=descent)
    fb.setupNameTable({"familyName": f"Moonglow Test {name}", "styleName": "Regular"})
    fb.setupOS2(sTypoAscender=ascent, sTypoDescender=descent, usWinAscent=max(ascent, 0),
                usWinDescent=max(-descent, 0))
    fb.setupPost()
    OUT.mkdir(parents=True, exist_ok=True)
    fb.save(OUT / f"{name}.ttf")


font("boxes", 1900, -500)
font("sliver", 1, 0)
