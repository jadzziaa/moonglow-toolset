---
title: "What a module's open tabs keep, the rest"
author: August
date: 2026-10-26
branch: open-tabs-rest
cost: { minutes: 848, tokens: 8748488 }
touches:
  - crates/mg-ui/src/open_tabs.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/03-modules.md
  - docs/manual/10-options.md
  - crates/mg-ui/src/area_props.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/blueprint/mod.rs
---

## Decided
- The record (`open-tabs/<name>-<hash>.json`) gained optional fields only, so older records read as before and an older Moonglow ignores the new ones: `views` (by tab text: `camera` [x,y,z,yaw,pitch,distance] + `night`, `scroll`/`cursor` for scripts, `page` by its name, and a conversation's `content` fingerprint, `selected` row `root 0`/`entry 3 1`, `bookmarks`/`folded` as `reply 5`), `objects` (`area:path` -> tag, TemplateResRef, position) and a window pane's `split` (a tree: `tabs`/`active` or `way` left-right/above-below, `share` in thousandths, two `parts`; the flat `tabs` stays for old readers).
- New tab texts: `hak:<path>`, `table:<key>` (the module's), `table-file:<path>`, `table-hak:<key>:<hak path>`, `tileset:<path>`, `options`, `references[:resource:k|:tag:t|:strref:n]`, `placeable-gallery`, `tile-properties:<area>:<indices>`, `tile:<tileset>:<n>`, `resource-copy:<key>:<layer>`. Paths are the rest of the text (Windows' `C:` is fine). `parse_tab` returns a `Want`; side editors are opened (by file) in `open_kept` at restore, and the `Action::OpenTab` their open functions push is truncated off `actions`, so `place_pane` puts them where kept.
- View state is gathered only when the record is written (`view_of`), from what the views already hold: `AreaView.orbit/night`, `ScriptBuffer.seen` (scroll offset from the ScrollArea's output and the TextEdit state's cursor, stored as it is drawn), `DialogView::content()` (= `seen.1`, `fingerprint(g)` = hash of `g.to_bytes()`), `blueprint_pages`, `module_page`, the Options draft's page. Restored lazily: `OpenTabs::views` (by Tab) is taken by the area view at creation, the script editor at its first draw, the Properties pages by `OpenTabs::page`. A pending view wins at write time (the view was never drawn since), except a conversation once drawn.
- A conversation's marks are restored at restore time (pre-seeding `dialog_views`, as references.rs does) only if `fingerprint(ws.doc(key))` equals the kept `content`: both sides are hashes of `to_bytes(parsed)`, never of the module's raw bytes (lossless files keep their own bytes). `follow` with `seen == None` just records the first fingerprint, so pre-seeded marks survive the first frame.
- A placed object is found again in `read_open_tabs` (`found_again`): the same tag+resref+position (0.01) nearest its old index, else the only one of its tag+resref (moved), else the tab is passed over; no identity kept (older records, or deeper paths) means by index. Views of a renamed tab move to its new text.
- Window places: `open_tabs_frame` hashes the windows' `now` rects; a change sets `moved` (egui's `input.time`), and a second without change writes; until then each frame asks for a repaint at the time left (a frame a little early would otherwise leave the write to the next input). kittest's step is 0.25 s, so 6 steps pass it.
- A split window's flat `tabs` is the split's depth-first order (`Split::flat`), not `iter_leaves`' order: egui_dock's tree is a Vec by level, so past two panes (root split, its first child split again: leaves 3, 4, 2) the two differ. After `Laid::build`, the window's `WindowTrack` is re-keyed to the first tab of the leaf `iter_leaves` lists first, the one `window_panes` follows it by.
- A script's `seen` keeps the scroll even before the editor has a cursor (`TextEditState::cursor.char_range()` is `None` until it has had one).
- Tile Properties waits (`OpenTabs::tiles`) until its area's view has `model` and `terrain`, then `tile_select::properties_of` and the tab goes beside a tab it was kept with, else to its kept place.
- Recover Unsaved Work: `recover` calls `read_open_tabs` after `use_module` (the module's path is the note's).
- Talk table split: `Settings::talk_splits` (name, `+f` with a feminine table; thousandths), not egui memory.

## Tried and ruled out
- Constructing `DialogView { .., ..Default::default() }` from open_tabs.rs: it has private fields; set the pub ones on a default.
- Options restored by pushing the tab alone: `lib.rs` opens the Options tab whenever a draft exists, so the draft is made (`Action::OptionsDialog` via `run_now`) unless one is there, and its page set.

## Watch out
- Reopening tdod.mod (3 areas, 224 resources, haks) with 12 tabs (3 areas, 4 scripts, Factions, Area Properties, Resources, Module Properties, the palettes): open_module 1.2 ms either way; open + 3 frames 77-82 ms kept against 75-77 ms opening the same tabs by hand (the tabs' own first draw); close (writing the record) 0.3 ms.
- In tests, a second session is a new `Moonglow` (and Harness) with the same `open_tabs_dir`: reopening in the same app keeps side docs and the Options draft around and hides what isn't restored.
- `open_some_tabs` leaves two windows (Factions, and the scripts): pick the window by its tabs, not the first non-main leaf.
- The worktree guard refuses commands whose text says "git" (even a Python variable): use Edit, or a script file.
