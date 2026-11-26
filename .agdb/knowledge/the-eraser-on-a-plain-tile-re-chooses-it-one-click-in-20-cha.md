---
type: Gotcha
title: "The Eraser on a plain tile re-chooses it: one click in 20 changes nothing"
description: "tiles::other_brushes_on_an_object_today flaked: an Eraser click with nothing to erase re-picks the tile at random (preview_seed) and about 1 in 20 picks the tile already there, so no change and no step; not load, not a dropped click."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T19:27:24Z }
asks:
  - "Why did other_brushes_on_an_object_today fail with the Eraser's click doing nothing?"
  - Does the Eraser drop clicks under load?
  - What does the Eraser do on a tile with no crosser?
  - "Why is a tile brush's result random in tests?"
created: 2026-11-25
author: augustaugur
---
grid.erase -> erase_only clears the cell's crossers and the stroke re-fits the cell; commit applies it with fastrand::Rng::with_seed(view.preview_seed) (a fresh random seed after every commit). On a plain tile nothing is cleared, so the cell is re-chosen among the fitting tiles; ~1 in 20 (20 quiet runs: 1; 40 runs: the drag's chest tile once) it is the same tile: tile_command gets no changes, no step, nothing said. Measured alone on an idle machine, so not load. The test now clicks (or undoes and drags) again up to five times for the Eraser until the chest's tile changes: 40/40 pass. preview_seed is pub(crate), so tests can't pin it. Product: not data loss (nothing to erase); whether an erase with nothing to erase should pick another tile is the owner's call.

Later (2026-11-25 evening): it failed once more on Engine's branch with the drag retry in place ("others" after six drags: not chance, ~1 in 60 million). Not reproduced here in 125 runs under heavy load (two copies of `tiles::` beside it). The test now takes the chest's and the drag end's screen places again after `settle` before every drag (a camera or chest settled late would make each drag start beside the chest), and prints the tiles each drag changed when the Eraser's drag misses tile 5, so a next failure says which cells it reached. No log of Engine's failure was kept.
