---
type: Decision
title: "FlatList: the tileset palette's and Prefabs' arrow keys over rows as drawn"
description: "How the tileset palette and the Prefabs list got keyboard keys (2026-11-25): palette_view::FlatList, rows registered as drawn the frame before, under Keys::Palette; what it guards and the pitfalls the review found."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T15:18:14Z }
touches: ["crates/mg-ui/src/palette_view.rs:FlatList", crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/src/area_view/bar.rs]
asks:
  - "How do the tileset palette's arrow keys work?"
  - Why does the Prefabs list have keys but no Find field?
  - Where does the cursor go when its row is filtered out or folded away?
  - Why does Down from the hak or tileset Find choose the first row?
fingerprint: v2-99a496c025d6be61
created: 2026-11-25
author: augustaugur
---
## Decided
- One shared cursor, `palette_view::FlatList<T>` (`PaletteView::tile_list`, `prefab_list`), for lists drawn as nested CollapsingHeaders or plain rows: each row asks for a slot as it is drawn and says where it is; keys act on the rows of the frame before (a closed heading's children are not rows). Not the blueprint palette's `Listed`/`Step`: its row model differs; `flat_step` repeats its stepping (Page reach, Left out to the parent).
- Keys are `Keys::Palette`'s (click, F6, Down from the search). Enter = the row's click (brush chosen, heading folded, `Action::PlacePrefab`). Prefabs got no Find field (not asked).
- Down from the hak, tile table and Appearance Gallery Finds: `SearchField::down()`; the hak/tile tables take the keys at the first filtered row and scroll to it (`scrolled_to` cleared); the gallery focuses its grid with `entry = (0, count)`, dropped when the count changes.
- Hak and tile tables draw `keys_outline_when` while they have the keys.

## Watch out (the review's findings, fixed)
- A cursor whose row is no longer drawn falls back to its heading or the row in its place (`FlatList::end`), else Up/End jumped to the last row.
- A "first row" asked counts only in its frame, with the keys and rows (`ask_first`); it had lingered and grabbed the cursor later.
- `FlatList::keys` does nothing under `shell::keys_blocked` or any popover/popup: Enter places a prefab.
- The list hints go after the area bar's own; none while a tile-picture grid has the focus (its arrows move pictures).
- Tests by exact name only: hak_panels.rs and tileset_panels.rs hold GPU tests (gpu_harness, area_harness_with).
