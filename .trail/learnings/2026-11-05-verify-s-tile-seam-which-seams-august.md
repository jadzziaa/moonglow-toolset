---
title: "Verify's tile-seam: which seams are the tileset's"
author: August
date: 2026-11-05
branch: claude/sweet-hodgkin-80a42d
cost: { minutes: 1, tokens: 1254 }
touches:
  - crates/mg-corpus-tests/tests/tiles.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-tiles/src/lib.rs
  - docs/findings.md
  - docs/log.md
  - docs/manual/04-areas.md
  - docs/manual/09-build-and-test.md
  - docs/manual/17-plugin-api.md
  - crates/mg-tiles/src/seams.rs
---

## Decided
- `mg_tiles::seams` compares each pair of neighbors once (east, north; across a corner only where both cells between are unknown tiles or holes) and splits the result: `area` (Verify's `tile-seam`, a warning) and `groups` (what a placed group of the tileset makes, never said).
- A seam is the tileset's only when no other tile could close it: both cells in one placed group (`group_at`); on a group's outside, a lattice corner the group's own tiles disagree about where the neighbor matches one of them; or an edge whose crosser on the group's side no free (non-grouped, non-hole) tile has (`TileIndex::is_free_crosser`). Groups are NOT skipped wholesale: a builder's stamps are groups, and one stamped beside the wrong terrain must be reported.
- Shipped areas (1,462): 75 disagreeing pairs; 66 the tileset's (ttu01 group 299/300 has Floor left, Rock right in both tiles; tid01, tni01 groups disagree inside about a crosser; trs02's undeclared crosser `path` is on tile 582 only), 9 the area's: Tyrants of the Moonsea tss13 (tile 0 beside a group gangplank; tiles 392-403 are the same deck model with a gangplank edge) and Darkness over Daggerford tno01 (sandbank ends at a cliff tile). `shipped_tile_grids_are_consistent` holds them to that list.
## Tried and ruled out
- Counting tss13's gangplank seams (Tyrants of the Moonsea) as the tileset's: its free tiles 392-403 have a gangplank edge, so the free-crosser rule (which trs02 needs) leaves them the area's. Excusing them because tile 0 and 392 share a model would be a rule fitted to 8 lines; they're reported and explained instead.
## Watch out
- Verify reports no Tile_ID the tileset lacks (only the area's problems in the UI, `AreaModel::read`); `seams` passes such cells over.
- `Lattice::from_tiles`'s second value counts cells (69 shipped), not pairs (75), and returns None on any unknown tile; `seams` doesn't.
