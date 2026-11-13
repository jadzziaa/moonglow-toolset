---
type: Decision
title: Page Up/Down to the next surface; Adjust Location live preview
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
created: 2026-11-02
author: August
description: "Page Up/Down (Cmd::SurfaceAbove/SurfaceBelow) move the selection to the next flat surface on a vertical line, using walkmesh heights plus model upward faces via ray_crossings; includes pitfalls on heights, stall models, and UI tests."
asks:
  - "How does Page Up/Down decide which surface the selected object moves to next?"
  - "Why does moving an object up with Page Up stop at a tabletop's underside or skip a surface?"
  - "Why use ray_crossings_in instead of ray_hit_in for stepping through multiple surfaces, and what stays unchanged?"
  - "My UI screenshot test with a dialog open passes but may be wrong; what should I check about AreaView::screen_pos?"
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
