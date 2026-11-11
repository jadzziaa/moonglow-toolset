# Moonglow's icon set

`icons.json` is the one source of the suite's icons: each icon's name and
its SVG markup in a 24 by 24 box, drawn as strokes 1.5 wide with round caps
and joins and no fill, at 16 px. `build.rs` makes it into Rust (the `Icon`
enum and each icon's strokes as polylines); `src/icons.rs` paints them with
egui. Edit the set here, and only here.

## Where the icons come from

The set was drawn for Moonglow's UI redesign in the manner of
[Lucide](https://lucide.dev) (ISC license). The object-kind icons (creature,
door, encounter, item, placeable, sound, store, trigger, waypoint, start
location, area, conversation, script, journal, faction, 2DA table, talk
table, NUI window, model, texture, tile, and the like) are Moonglow's own.
Where an icon's shape matches one of Lucide's, Lucide's ISC license
applies to it: its text is in `LICENSE-lucide.txt` beside this file.

Added after the boards (M2 of the redesign): `restore` and `extract-all`,
Lucide's `minimize` and `folder-output` shapes.
