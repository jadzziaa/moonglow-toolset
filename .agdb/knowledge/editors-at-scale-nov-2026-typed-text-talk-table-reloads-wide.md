---
type: Gotcha
title: "Editors at scale (Nov 2026): typed text, talk table reloads, wide grids, claims"
description: "Where the editors' frames went at a world's sizes (frame_perf::editors_at_scale) and the traps found building that view."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T03:27:42Z }
touches: ["crates/mg-rules/src/lib.rs:update_custom_tlk", "crates/mg-ui/src/talk_view.rs:stamps_now", "crates/mg-ui/src/table_view.rs:Unseen", "crates/mg-ui/src/claims.rs:still_whose", "crates/mg-ui/tests/frame_perf.rs:editors_at_scale"]
asks:
  - Why did a typed name in the creature editor cost a frame of 11 ms?
  - Which kept lists does a changed custom talk-table line forget?
  - Why did saving a hak take a frame of 11 ms with a large module open?
  - How do I write variables.toml for a variable whose value is a 2DA row in a test?
fingerprint: v2-3a81fc935d8ffc66
created: 2026-11-26
author: augustaugur
---
Found by frame_perf::editors_at_scale (FRAME_VIEW picks a group: "typed line", "hak saved", "2DA editor", "Claims Table"...).

- A typed text (talk_typed: the line goes into the module's TLK) made GameData::update_custom_tlk clear every choice list, named list and property type list; the Creature editor draws ALL its pages each frame, so the next frame named appearance, spells, soundsets... again (Special Abilities alone 5.7 ms). Now each kept list records the custom lines it read (thread-local recorder in GameData::string, `reading_lines` around a build, hits inside a build pass their lines up), and only lists that read a changed line (or a line past the old end: one asked for and missing counts) are made again. `marked` (appearance lists) is keyed by the list's texts, so it is not cleared. GameData::generation() has no callers; the revision bump is harmless.
- Comparing 50,000 TLK entries took 4.8 ms on one thread (memory-bound, ~100 ns a line): `differing` splits it over up to 8 threads above 8,192 lines. available_parallelism() reads cgroup files (~60 us): ask it only for big tables.
- talk_view::read_again with stamps None read the whole talk table to see if it changed; stamps were None from open until the first save, so the first Reload Resources after opening (a hak's save does one) cost 6.4 ms. stamps_now() is taken at every open now.
- A 2DA grid's out-of-sight cells were each allocate_exact_size (a widget, with interact): a run of them is one allocate_space now, ids skipped 2 per cell to keep later ids. Pixel-identical (screens table_* and talk_*).
- claims::listing cloned the whole claims table every frame to compare (whose()): still_whose compares in place.
- Left: the raw fields view's per-item open check (an id and egui memory lookup for each of 3,000 closed items, about 27 ns each); block-skipping needs to know which items are open without looking.

Test-building traps: variables.toml takes `table = "placeables"` (not `rows =`); a plugin menu command is offered on Advanced Search hits only when every ticked hit has a plugin target (module/area variables have none: restrict to placed objects); the Right Dock starts folded, push Action::ShowPanel(Panel::Cell) before looking for Choose Model…; Advanced Search hits are Labels containing " › ", not links.
