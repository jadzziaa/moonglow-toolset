---
type: Gotcha
title: "UI caches after the adversarial review (fix U1): what their keys must hold"
description: "Why the 2DA editor's incremental caches key on a history counter not the revision, why Thumbnails::forget keeps the creatures, why outlines are smoothed below 1000 lines, and other cache keys the review C found stale."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T01:23:36Z }
touches: ["crates/mg-ui/src/table_view.rs:TableDoc", "crates/mg-ui/src/model_view.rs:Thumbnails::forget", "crates/mg-ui/src/area_view.rs:outlines", "crates/mg-ui/src/blueprint/item.rs:part_grid", "crates/mg-ui/src/lib.rs:Moonglow::load_custom_tlk"]
asks:
  - "why does the table editor's faults cache have a history field"
  - why are creature thumbnails not cleared when pictures are forgotten
  - when are area view outlines drawn as one mesh instead of line segments
  - what must a cache keyed on load_order know about custom talk tables
fingerprint: v2-b0460d5435019ebd
created: 2026-11-24
author: augustaugur
---
Found and fixed by fix writer U1 (review C of the adversarial-review branch, 2026-11-23):

- 2DA editor (`table_view.rs`): `TableDoc::changed()` bumps `revision` at EVERY edit, so an incremental cache (faults, shown rows) cannot require the same revision or it is never incremental. What breaks them is `read_again`, which makes a new `Editing` whose `changes()` restarts at 0: `TableDoc.history` counts those, and the incremental paths require the same history. `columns_cached` keys on (revision, changes): keyed on changes alone, a table never edited and read again (changes 0 before and after) kept its old columns.
- Incremental faults: fetch the game's copy (`base_faults_for`) whenever the touched rows have faults, not only when the table had faults before; otherwise the game's own wrong cells show red in an edited row.
- `Thumbnails::forget` must NOT clear `creatures`: the Appearance Gallery's `dressed_kept` keeps the creature id per (place, revision) in egui temp and never re-registers it, so every creature tile became "No picture" after the Shaders switch / Largest textures / game data reload. dressed_kept now also re-registers when `has_creature` is false (the end-of-frame prune drops creatures nothing asks for).
- SUPERSEDED (fix X, 2026-11-24): `load_custom_tlk` bumps `tlk_generation` instead and these caches key on `game_names()`; see talk-table-reloads-bump-tlk-generation-not-the-load-order-fi. Was: `load_custom_tlk` now calls `load_order_changed()`: palette Find (`fold_fields`), store base items, wizard item types and Setup Area Transition's area names are keyed on `load_order`, and saving the talk table (talk_view save -> reload_custom_tlk) did not bump it.
- Area view outlines (`area_view::outlines`): egui line segments (smoothed) up to `BOXES_MESHED` (1000) lines, one `lines_mesh` past it. Pictures at 100% (target/adv/fixU1/pictures/walk-zoom.png) showed the always-meshed walkmesh edges stair-stepped; the tile selection uses the same switch (it meshed from 16 tiles).
- Item Appearance grid (`part_grid`): the whole item's icon size must be learned per frame; kept in egui temp under ("uti-icons", key, label) it outlived a base item change (the key has no base item) and off-screen icons took the first base item's room.
- Largest textures change: model views need `redraw_picture()` (drawn = None) like area views, or the idle picture stays stale.
- Gallery Appearance panel stays an F6 stop (U26), the branch had filtered it out.
