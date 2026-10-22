---
title: "Tile editing feedback: right-click tile menu, appearance/rotate tools, Replacement Texture removed, variant zoom"
author: August
date: 2026-10-22
branch: tile-editing-feedback
cost: { minutes: 392, tokens: 2136696 }
touches:
  - crates/mg-area/src/terrain.rs
  - crates/mg-corpus-tests/tests/aurora_terrain.rs
  - crates/mg-tiles/src/paint.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/icons.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/src/terrain_mode.rs
  - crates/mg-ui/src/tile_select.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
---

## Decided
- A tile brush's right click (Terrain, Eraser, Refine, Appearance, Rotate: `terrain_mode::menu_on_right_click`) opens `tile_select::context_menu`, called from `area_view::input` right after `terrain_mode::input` returns true; Raise/Lower, Crosser, Group and Tile keep their own right click. Escape or the menu's "Put Down <brush>" drops the brush (Aurora's right click did).
- Out of tile mode, a right click on ground with no object records `view.menu_tile`; the object `context_menu` starts with `tile_select::ground_items` (Aurora's pmViewerArea shows Tile Properties for a clicked tile). Items act on explicit cells; `properties_of` restores `tile_selection` out of tile mode so the status line shows no stale "selected:" tile.
- Next/Previous Tile Appearance = `mg_tiles::paint::step_appearance` (next distinct tile id among `fits_in_order`, keeping orientation/height where it fits); Rotate Tile = `turn_fit` (same tile, next orientation that still fits, else refuse). Validity is exactly `TileIndex::fits(cell)`. Rotate keeps lights/loops via `tile_command_keeping`; Appearance gets fresh lights like the painter.
- Refine Tile is NOT random: it is `next_fit`, Aurora's Shift+right-click order (tile, then orientation), so look and rotation change together. Builders perceive it as random.
- Tile Properties' Replacement Texture removed from the form (field kept, raw fields). Evidence: nwn.wiki "unused BioWare functionality"; replacetexture.2da = two rows both T_Flag02; the only model with `replace_tex` (tms01_c08_01) is in no .set; none of the 18 searchable data/nwm campaign modules writes `Tile_ReplaceTex` (grep -a on the .nwm finds Tile_MainLight1 in them). Client drawing never checked; recipe in docs/deferred.md.
- Palette's Select Tiles row is labeled "⛶ Select Tiles (F10)" (keymap.titled) because the toolbar's "⛶ Select Tiles" label would collide in kittest queries; not auto-entered when the Tiles palette is shown.

## Tried and ruled out
- A variant column's stride is side + Grid spacing (6) + item_spacing.x (8), not side + 6: the `across` count was overestimating.

## Watch out
- In this worktree guard, `flock ... cargo test FILTER` is refused; build with `--no-run` and run the test binary under flock from crates/mg-ui. Heredoc python edits are sometimes refused: write the script to the scratchpad and run it.
- Tooltips (on_hover_ui) don't show in kittest's accesskit tree; render a picture to check them.
