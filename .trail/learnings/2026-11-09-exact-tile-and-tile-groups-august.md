---
title: Exact Tile and tile groups
author: August
date: 2026-11-09
branch: develop
touches:
  - crates/mg-tiles/src/paint.rs
  - crates/mg-ui/src/terrain_mode.rs
  - crates/mg-ui/src/tile_select.rs
---

## Decided

- With the Exact Tile switch on, a Groups or Features picture goes in as it is (`Grid::place_group_exact`), and `Grid::set_exact` refuses only where `groups_broken` says a larger group would be left part changed. A group of one tile (Fort Interior's staircases, `twc03` tiles 40 and 41 among others) is replaced or turned like any tile. Larger groups keep the lock unless "Allow editing tiles of groups and features" (`free_groups`) is on. This was the owner's choice, made for vicky.bat's "Exact tile doesn't work with the staircase" report.
- `set_exact` also backs Tile Properties' All Tiles and Turn (`tile_select.rs`), so those follow the same rule.

## Tried and ruled out

- A regression: the group lock (v2.2.0) came before Exact Tile (v2.4.0), and `TileIndex::tiles_where` has always left grouped tiles out of the Gallery. The staircase case never worked, so there was no last good version to look for.

## Watch out

- The Gallery labels a group by the palette's name for it (`TilesetPalette::read`: "Stairs Up"), not the `.set` group name ("Stairway_up"). In a UI test, set `settings.gallery_tile` small: a picture out of sight is not laid out and can't be found by its label.
- The mg-tiles test helper `apply` checks that the tiles rebuild the lattice, which an exact tile on purpose doesn't. Apply exact strokes with `Grid::apply` directly.
