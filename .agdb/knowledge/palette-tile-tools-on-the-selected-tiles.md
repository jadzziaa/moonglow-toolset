---
type: Decision
title: Palette tile tools on the selected tiles
tags: []
status: stable
generated: { by: trail, at: 2026-10-23T00:00:00Z }
touches: [crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/src/tile_select.rs, crates/mg-ui/tests/app.rs, docs/manual/04-areas.md, docs/deferred.md]
created: 2026-10-23
author: August
description: "With tiles selected in Select Tiles, the palette's Next/Previous Tile Appearance and Rotate Tile act as push buttons calling tile_select::step_tiles via Place::stepped, keeping tool mode and selection; otherwise they are brushes."
asks:
  - "Why does clicking Rotate Tile in the Tiles palette rotate my selected tiles instead of switching to a brush?"
  - "How does palette_ui apply step_tiles to the selection without borrow conflicts on app.area_views?"
  - "Do Next/Previous Tile Appearance keep the selection when Select Tiles is active, and what happens with no selection?"
  - "Which tile tools have keyboard shortcuts, and does Shift on Rotate Tile turn clockwise?"
---
## Decided
- With tiles selected in Select Tiles, the Tiles palette's Next/Previous Tile Appearance and Rotate Tile are push buttons on the selection: `brush_row` records `Place::stepped` (a `TileStep`; Shift on Rotate: clockwise) and does NOT set `chosen`, so `palette_ui` neither turns tile mode off nor clears the selection. After the scroll area (`at` dropped), `palette_ui` takes the view out of `app.area_views`, calls `tile_select::step_tiles` (the tile menu's: one command, each tile on its own, refusal text and red flash) and puts it back. Chosen over 'switch to the tool, keep the selection': the area click stays 'select', and another click on the button steps again.
- With no selection (or Select Tiles off) the three are chosen as brushes, as before. Refine Tile and the Eraser are unchanged (still drop the selection): in docs/deferred.md.
- None of the tile tools has a key (keys.rs); the palette is their only entry besides the tile menu.

## Watch out
- `Place` borrows `view.terrain` from `app.area_views`: anything needing `&mut app` + `&mut view` must wait until after the palette's ScrollArea; record it in `Place` and act after.
- Select Tiles' status line has the refusal at its end; at 1100 px the area view cuts it short (the test's `query_by_label_contains` still finds it).
