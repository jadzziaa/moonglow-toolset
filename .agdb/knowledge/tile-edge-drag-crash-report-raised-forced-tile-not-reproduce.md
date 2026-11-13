---
type: Decision
title: "Tile edge drag crash report (raised forced tile): not reproduced"
tags: []
status: stable
generated: { by: trail, at: 2026-11-12T00:00:00Z }
touches: [crates/mg-ui/tests/app/main.rs, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-f068852f/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1109878-ec1a9131-2/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1109878-ec1a9131/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1109878-fec79a17/.mg-lock, apps/mg/target/tmp/moonglow-test/a1098591-0667e805/.mg-lock, apps/mg/target/tmp/moonglow-test/a1109878-ec1a9131/.mg-lock, apps/mg/target/tmp/moonglow-test/chess-26a2bc7b/.mg-lock]
fingerprint: v2-12af14d50dd0d8dc
created: 2026-11-12
author: August
description: "A reported toolset crash when dragging a placeable across a height step with a forced tile was not reproduced in 63 tests in tile_edges.rs; no code changed, and the reporter's crash-<time>.txt is needed next."
asks:
  - "Does dragging an object over the edge between tiles at different heights crash Moonglow, and has anyone reproduced it?"
  - "Where do I find the crash-<time>.txt file, and what does it tell me if Recover Unsaved Work wasn't offered?"
  - "Why does a dragged object end up off the raised tile in the area even though it stays under the pointer on screen?"
  - "Which tests cover drags across height steps, and why are two of them marked #[ignore]?"
---
## Decided
- A builder ([Haze] Wolf, Discord, 2026-11-11) reported that dragging a placeable over the divide between two tiles at altered heights, one of them forced where it does not fit, crashes the toolset. It was NOT reproduced: 63 cases in `crates/mg-ui/tests/app/tile_edges.rs` (pointer events through rendered frames, the installed game's tiles with real walkmeshes, 8 GB cap and timeout), the first 59 also on the v4.1.0 tag. No panic, hang, growth, guard-caught panic or wgpu validation error. No product code was changed.
- The next step is the reporter's `crash-<time>.txt` (Moonglow's data folder: `~/.local/share/moonglow`, `%APPDATA%\Moonglow`, `~/Library/Application Support/Moonglow`; written by `crash_reports` in `apps/moonglow/src/main.rs`) and whether Recover Unsaved Work was offered: a frame panic gives both, so neither means a freeze or a driver's end, not a panic in a frame.
- The test file stays as the regression net for drags across height steps. Two cases are `#[ignore]` awaiting the owner's ruling (see Watch out).

## Tried and ruled out
- Forced tile (Exact Tile and Tile Properties' All Tiles) at equal, raised (1, 3), lowered heights; both tiles altered (2/5, 5/2, +1/-2); each direction; four-tile corner; the area's outer edge; let go just before, just past and on the edge.
- Camera top-down, tilted both ways, nearly flat; pointer a few pixels a frame and held over the open gap; snap; X/Y arrow drags; Onto Objects; a lifted placeable; two selected; walkmesh overlays; Fade Geometry; the game's shaders.
- Placing by click, palette drag-and-drop (`drop_blueprint`) and paste; every object kind, trigger and encounter outlines, the start location's and a spawn point's drag; Adjust Location, Drop to Ground, Page Up/Down; undo and redo.
- The state forcing a tile leaves: Exact Tile brush still chosen (a press-drag on a placeable paints the tile, the placeable stays), tiles selected (box-selects tiles), Tile Properties open.
- Reading `Ground::height/flat_heights/height_on/hit` (mg-area `walk.rs`), `AreaView::ground_at/ground_height/moved/put_down_at/snapped/surface_below/surfaces_at/dragged`: no panic site.
- Continuity loops (bisection, converge-on-a-height) around a dragged object: none found, and nothing timed out.

## Watch out
- Beside a raised tile with no cliff face the view shows through the gap. `AreaView::ground_at` (`crates/mg-ui/src/area_view.rs`) then picks the ground seen behind, or, with nothing behind, the plane at the press's height, which `kept_in_area` clamps to the area's edge. On screen the object stays under the pointer; in the area it is not on the raised tile. Whether that is wanted is undecided: don't "fix" it without the owner's ruling.
- An X or Y arrow drag and Adjust Location keep the object's height by design, so across a height step the object ends under the raised ground.
- A test's exit status does not show a panic the renderer's guards caught (`mg_render::guard`): grep the log for `panicked at`, `Validation Error`, `left it out`.
- Tests that aim at an edge or a gap aim from area points and check the ray, not from screen pixels: the view's place in the window moves with the UI redesign.
- There is no keyboard nudge for objects: arrows and WASD move the camera.
