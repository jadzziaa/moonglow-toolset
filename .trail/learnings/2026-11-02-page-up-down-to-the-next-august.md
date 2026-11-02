---
title: Page Up/Down to the next surface; Adjust Location live preview
author: August
date: 2026-11-02
branch: develop
cost: { minutes: 1, tokens: 1297 }
---

## Decided
- Page Up/Down (Cmd::SurfaceAbove/SurfaceBelow) step the selection to the next flat surface on a vertical line through the FIRST selected object: area_view::surfaces_at = Ground::flat_heights (walkmesh, every level) + every placeable/door model's upward faces (AreaScene::ray_crossings, normal.z >= walk::FLAT = 0.7). Others move by the same delta; creatures/outlines stay (lifts()).
- Faces count by their own normal (corners' cross product) turned toward the summed vertex normals; vertex normals alone lean at a box's edges. Corner order is the fallback only (skinned meshes, no normals).
- Model surfaces, not placeable walkmeshes (pwk): a stall's counter is in its model; its pwk is a footprint.
## Tried and ruled out
- Nearest-hit ray_hit_in (as Drop onto Object uses) can't step past the first surface, and is two-sided: a tabletop's underside (PLC_M05, ~0.75 m) would count. ray_hit_in stays unchanged (Moonglow Viewer builds mg-preview by tag); ray_crossings_in is added beside it.
## Watch out
- The game's Table (row 97) top is 0.849 m at its middle, not its 0.9 max vertex: check heights with a picture, not the ASCII's extremes.
- 'Market Stall 01' (row 525, tnp_market_001) is a canopy on poles, no counter. 'Trades - Market Stall 01' (row 15519, ptm_market11) has counter 0.90, shelf 1.37, awning ~3.25 at the stall's (-2,-0.8): looks::look_surface_steps_on_a_stall.
- UI tests that take pictures with a dialog open: AreaView::screen_pos doesn't clip; check the places are inside view.rect and clear of the window, or the pixel diff passes for the wrong reason (adjust_location test).
