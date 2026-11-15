---
type: Decision
title: "Tile Properties panel (M4 U5): diff-written, read again by revision, the area's window_right"
description: "How Tile Properties works since it became the Right Dock's panel: no OK/Cancel, each frame writes what differs from what was read as one command, props re-read when area/cells/revision/rewrites change, height drag merged via withdraw, Brings::window_right for a dragged-out area, old tile-properties records."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-15T10:26:10Z }
touches: [crates/mg-ui/src/tile_select.rs, crates/mg-ui/src/shell.rs, crates/mg-ui/src/area_view/header.rs, crates/mg-ui/src/open_tabs.rs, crates/mg-ui/tests/app/tile_properties.rs]
asks:
  - Why does a change to app.tile_props in a test get lost?
  - How is a dragged Height one undo step?
  - Where does Tile Properties go when the area is dragged out into a window?
  - What happens to an old open-tabs record naming tile-properties?
fingerprint: v2-a1590d93040b2a5b
created: 2026-11-15
---
## Decided
- `Panel::TileProperties` (Right Dock), brought by `shell::brings_of` for `Tab::Area` while Select Tiles is on and `Settings::hide_tile_properties` is false (`tile_properties_shown`): `AREA_TILES_RIGHT = [[Palette, Preview], [TileProperties]]`. The view is out of `area_views` while the area draws, so `app.tile_mode_areas` (written at the end of `area_view::ui`) answers then.
- A dragged-out area brings its Palette/Preview to the MAIN dock (`floats: false`) but Tile Properties into its window: `Brings::window_right` (sections that follow a non-floating editor into its window); `window_panes` draws `window_right` when `!floats`.
- A one-panel section with `Panel::detail` (Tile Properties: "6 tiles", "Tile (4, 4)", "none selected") gets the kit `PanelHeader`, named "Tile Properties, 6 tiles" (`shell::section_bar`). Its group node has no rect in kittest: measure a control instead.
- `tile_select::panel` = `props_now` (reuse `app.tile_props` while area, cells, ws revision and ARE rewrites match, else `read_props` again keeping All tiles/Turn/find/picking/live) + draw + `write_changes`: the draft (controls differing from `opened`, or `touched`) is ONE command "Tile Properties" per frame, then `props.revision = u64::MAX` forces a re-read next frame. No OK, Cancel, closing question, `take_back`, `reread`, `carry_draft` (all removed).
- Height drag: `height_row` returns `r.dragged()`; while dragging, if the last command is our own (`props.live == ws.revision()`), `ws.withdraw()` first, so the drag is one undo step. withdraw+apply moves the revision by 2: never count steps by revision deltas, count by undo.
- Old open-tabs records with `tile-properties:AREA:INDICES` parse to `Want::TileProperties`; `tiles_again` turns Select Tiles on with those cells (in range) once the view has tiles; still waiting at write time it is kept as a window pane. `Settings::shell_migrate` drops `tile-properties` window places and sizes.
- Header Tiles state (`header::tiles_items`/`tiles_asked`): Refine Tile, joined Previous/`Joined::words("Appearance")`/Next (kit add), Rotate Tile (words "Rotate"), joined Cut/Copy/Paste Tiles, Delete Tiles, Tile Properties toggle; the Show popover in More (as Paint). The palette's tool rows stay (brushes without a selection): tests click them with `apart`, the header ones with `area_click`.
## Watch out
- A test that mutates `app.tile_props` right after a write must run 2 frames between changes: a mutation on props whose revision is stale is discarded by the re-read.
- The panel only exists where the Right Dock draws it: under 1400 it is a folded tab (Palette in front unless chosen), so tests use `wide_window` or the tile menu's "Tile Properties…" (which `ShowPanel`s it).
- (U5 review) The panel has its own `Keys::TileProperties` (in `Panel::KEYED`); F6 there focuses its first control (`Moonglow::tile_props_focus`). `keys_leave_panels_not_drawn` counts `window_bars` as drawn. The area view used to take the arrow keys back EVERY frame the pointer rested over it (so F6 after a tile click stayed on Area, panel or not); it now does so only as the pointer comes in (`AreaView::pointer_was_in`), moves or presses in the view (`area_view::camera_input`).
- Moonglow::apply can return Ok without applying (a viewed resource): `write_changes` keeps `live` only when the revision moved and withdraws only its own "Tile Properties" command.
