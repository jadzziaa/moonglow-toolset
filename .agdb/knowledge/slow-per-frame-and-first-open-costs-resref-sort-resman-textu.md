---
type: Gotcha
title: "Slow per-frame and first-open costs: ResRef sort, resman.texture reads, multi-object panel"
description: "Where the Properties panel of 5,000 objects, Select Texture's and Select Portrait's openings spent their time (Nov 2026), and the traps behind each."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T12:08:17Z }
touches: [crates/mg-ui/src/properties_panel.rs, crates/mg-ui/src/blueprint/inventory_together.rs, crates/mg-ui/src/blueprint/replacements.rs, crates/mg-ui/src/images.rs, crates/mg-ui/src/narrowing.rs, crates/mg-core/src/resref.rs]
asks:
  - Why is sorting ResRefs slow?
  - Why was the Properties panel slow with thousands of objects selected?
  - "Does ResMan::texture read the file?"
  - Why was Select Portrait slow to open?
fingerprint: v2-de5517686bd481c8
created: 2026-11-26
author: augustaugur
---
- ResRef's Ord folds both names byte by byte at every comparison: sorting 135,000 took 19 ms. Sort already-lowercase names by bytes, or as u128 (lowercase bytes zero-padded, big-endian: same order, shorter first): `replacements::sorted_names`.
- `ResMan::texture` READS the bytes. To ask whether a texture exists use `texture_type`/`texture_layer`. But per-row lookups across ~30 layers × 2 types × 5 sizes are themselves the cost (Select Portrait, 32,000 rows): build a name set in one walk of `layers()[..].container.keys()` (`images::Portraits`). The walk of every layer's keys is only ~2 ms for the whole install plus a world's haks.
- Multi-object Properties panel (5,000 selected): the profile, not guesses, found it. Two thirds was `inventory_together::page` gathering every object's ItemList each frame although only its `kept_for` closure used it; a quarter was `Form::list_targets` hashing the `also` Arc's 5,000 paths for its HashMap key (now a last-hit slot checked by `Arc::ptr_eq`, which only works because `properties_panel::kept_paths` keeps the same `also` Arc while the selection is unchanged). `differs` stops past MIXED_LIMIT (64), so mixed marks were not the cost.
- Drag of 5,000 selected: what is left (~3 ms) is the boxes' 480k vertices (12 edges × dark edge + line, 8 vertices each) built and appended; the renderer's redraw dominates the frame (Engine's).
- Find over a large list: `narrowing::Narrowing` (one for Choose Image and Select Texture) looks among what the last filter found when `Needle::narrows`.
- Profiling method: scratchpad `sample2.py MARK -- <frame_perf binary> <test> --ignored --exact` with SAVE=pickle; tally stacks under the frame_perf closure of the view (the drag view is `area_editor_at_scale::{closure#7}`).
