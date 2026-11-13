---
type: Decision
title: Blueprints read again from outside follow into the custom palettes
tags: []
status: stable
generated: { by: trail, at: 2026-10-27T00:00:00Z }
touches: [crates/mg-edit/src/lib.rs, crates/mg-module/src/palette.rs, crates/mg-ui/src/outside.rs, crates/mg-ui/src/palette_view.rs, crates/mg-ui/tests/app.rs]
created: 2026-10-27
author: August
description: "Blueprints reread from outside were missed in custom palettes because `Workspace::adopt` put them in `saved`; adopt now skips blueprints, palette change is detected per frame, and `update_custom_palette` was made faster."
asks:
  - "Why did new blueprints pulled in from outside not show up in the custom palette after the module reloaded them?"
  - "How does Moonglow decide the palette changed and mark the module as having unsaved work?"
  - "Why does frame_perf `reload_frames_at_144` fail intermittently with 'not read again', and how was it fixed?"
  - "What does Refresh Palette in the palette More menu rebuild, and can it be undone?"
---
## Decided
- Why new blueprints were missed: `Workspace::adopt` wrote files read again into both `module` and `saved`, and the palette view and the save compute `update_custom_palette(module, saved)`, which only sees differences. Fix: adopt no longer takes blueprints (`ObjectKind::from_restype`) into `saved`; everything else still is (a pulled palcus or skeleton stays taken in).
- Writing the updated palcus into the module at take time (`derive`) was rejected: the module's palcus would then differ from its as-read bytes, and a teammate's palcus pulled later would land in Changed Outside Moonglow as a conflict the builder never caused.
- 'The palette changed is unsaved work' is decided off the take, one type a frame (`Moonglow::look_at_palettes`, from `reload_timer`), only when the module isn't modified already, with `update_custom_palette_for` (the keys read again alone). Done in the take, the frame_perf pull (400 blueprints) went from 2.8 ms to 12-68 ms.
- `update_custom_palette` walks the palette once (`Places`: entries by ResRef, categories by id) and again only after a change of shape; compares blueprints by Arc first; returns before `to_bytes` when nothing moved. Before: two walks per changed blueprint, 44 ms for 250 items in the persistent world.
- Refresh Palette (palette More menu) rebuilds the shown type only: Aurora's refresh.mod capture differs from edit-copy.mod in itempalcus.itp alone. Undoable command.
- Unknown/missing PaletteID: still left out (no Aurora capture); recorded in deferred.md.

## Watch out
- frame_perf `reload_frames_at_144` used to fail now and then ('not read again'): it waited for each kind's last file only, and when the watch's first look caught those, the 2 s tail ended before its next look (3 s). Fixed 2026-10-27: it checks every file, four times a second; the honest 'all in' is ~3.6 s, not the 1.5 s the old check printed.
- The app harness's palette search opens categories only when the filter text changes: a blueprint arriving later stays in a closed category; tests clear and retype the filter.
- A test holding `app.game.clone()` makes the app fail to add the module's haks (it is in use): drop it before open_module.
