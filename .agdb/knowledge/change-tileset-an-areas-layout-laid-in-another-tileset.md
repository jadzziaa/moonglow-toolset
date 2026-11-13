---
type: Decision
title: "Change Tileset: an area's layout laid in another tileset"
tags: []
status: stable
generated: { by: trail, at: 2026-11-06T00:00:00Z }
touches: [crates/mg-area/src/lib.rs, crates/mg-corpus-tests/tests/tiles.rs, crates/mg-ui/src/area_reshape.rs, crates/mg-ui/src/commands.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/tests/app/tiles.rs, docs/log.md, docs/manual/04-areas.md, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock]
fingerprint: v2-503dee972f976ae0
created: 2026-11-06
author: August
description: "Change Tileset is implemented by `mg_area::retile`, which reads the old layout via `terrain::grid`, maps terrains and crossers by name, repaints with the new tileset's `Grid::paint`, raises heights, lays crosser edges, and swaps doors; it has no `mg` command."
asks:
  - "How does retile convert an area's layout when I switch it to a different tileset?"
  - "My area's terrain turned into Default after changing tileset; how are terrains and crossers matched to the new tileset?"
  - "Why can't Frozen Wastes (tti01) lay the edges of a plateau after Change Tileset?"
  - "Is there an mg command to change an area's tileset like Resize or Rotate Area?"
---
## Decided
- `mg_area::retile`: the old tiles' lattice (`terrain::grid`), terrains and crossers mapped by name (else the new Default terrain, no crosser), painted into an all-Default area with the new tileset's `Grid::paint` (two passes), `raise` a level at a time (only where `has_height_transition`), then each crosser's edges with `lay_crosser`. Corners and edges not laid are counted, not forced. Tiles keep their old lights; doors on the old tiles' typed hooks go and `door_edits` (with a sentinel "before") brings the new tiles' doors.
- No `mg` command: Resize and Rotate Area have none either.

## Watch out
- `Grid::paint` can be refused next to a raised corner (a corner painted beside a plateau); the test paints its trees away from it.
- Every shipped tileset takes a rural layout without seams; Frozen Wastes (tti01) can't lay a plateau's edges.
