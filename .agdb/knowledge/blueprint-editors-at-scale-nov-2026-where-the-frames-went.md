---
type: Gotcha
title: "Blueprint editors at scale (Nov 2026): where the frames went"
description: "What frame_perf::blueprint_editors_at_scale found in the blueprint editors and how each cost was found and cut: per-frame work over many targets, egui temp-state clones, resman walks, the module tree's out-of-sight rows, and GPU drops on the UI thread."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T02:03:28Z }
touches: [crates/mg-ui/tests/frame_perf.rs, crates/mg-ui/src/blueprint/inventory_together.rs, crates/mg-ui/src/tree.rs, crates/mg-ui/src/model_view.rs, crates/mg-gff/src/read.rs, crates/mg-ui/src/blueprint/situated.rs]
asks:
  - Why is the window slower with nothing open after some views ran?
  - Why does Setup Area Transition or Select Texture take tens of milliseconds to open?
  - How do I find which part of a frame grew in frame_perf?
  - Why is the Instances window of many objects slow?
  - How are 2DA tables grown for a perf test without touching the install?
fingerprint: v2-6b0d9076db874045
created: 2026-11-26
author: augustaugur
---
Method: Instant timers with a static accumulator printed every N frames, wrapped around Moonglow::ui's sections (left/right/bottom dock, central, dock show) and then narrower (each page of form_ui by name, inside a page). Revert with git checkout; keep the timer patch in the scratchpad to reapply. Whole-frame A/B under load 10-17 spreads +-0.5 ms, so time the parts.

Found (all fixed unless said):
- Many targets (Instances of 100 creatures): per-frame merge/count/flag loops over targets, and SipHash lookups per row (held(r) over 100 maps). Cache per list_targets() Arc with blueprint::kept_for (the Arc changes each revision, so ptr_eq is a correct key). Tab::Instances cloned/hashed by the dock each frame copied 100 GffPaths: now Arc<[...]>.
- ui.data get_temp/insert_temp CLONES the value every frame: a dialog state holding thousands of entries (Setup Area Transition's destinations) must hold them in an Arc.
- ResMan::list(type) walks every key of every layer and builds a HashSet: each call ~13-18 ms with a world's haks. Walk game.resman.layers() keys once, filter by type and prefix. Still the floor for first openings (Select Texture 26-30 ms, Select Portrait, icons): a per-type index in mg-resman would be the real fix (deferred).
- Parsing whole GITs to find doors: Gff::read_lists(data, &[lists]) leaves other root lists empty and unread.
- The "control drift" (nothing open, 0.23 -> 1.0 ms after the tables views): an item opened revealed its module-tree group of 8,000 rows; out-of-sight rows were each checked. Now skipped in blocks (tree_rows: rows above the clip in one allocate_space, the rest once the cursor passes clip bottom), only when nothing in the group is to be revealed or keyed to. Test: tests/app/tree.rs a_group_of_thousands_draws_the_rows_in_sight_as_one_by_one (fails if the block math is off).
- Thumbnails: ready pictures no longer asked for were dropped on the UI thread (GPU buffers/textures, 57 ms worst frame in a gallery drag): extract_if and drop on a spawned thread. Left: one picture drawn per frame however long (~5 ms).

Test fixtures: grow tables by writing 2DAs and TGAs into the scratch user folder's development/ (top of the resman, above haks), then Action::ReloadResources and wait for the row count. Pointer::Clicks(pos) and Pointer::Types(text) in frame_perf time the event frame itself.
