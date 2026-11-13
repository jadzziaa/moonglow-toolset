---
type: Decision
title: Placeable brush and see-through ghosts in test pictures
tags: []
status: stable
generated: { by: trail, at: 2026-10-16T00:00:00Z }
touches: [crates/mg-ui/src/paint_mode.rs, crates/mg-ui/tests/app.rs, docs/deferred.md, docs/manual/04-areas.md, crates/mg-ui/src/area_view.rs, crates/mg-ui/tests/screens.rs, docs/log.md, crates/mg-area/src/edit.rs, crates/mg-area/src/walk.rs, crates/mg-ui/src/icons.rs]
created: 2026-10-16
author: August
description: "The area view's Paint brush (paint_mode.rs, key B) has Scatter and Row modes, stores integer settings, and places only on walkable ground; semi-transparent ghosts don't render in egui_kittest wgpu pictures, so test them via counts."
asks:
  - "Where is the placeable paint brush implemented and what modes does it offer in the area view?"
  - "Why are my placement ghosts invisible in egui_kittest screenshots, and how can I check them instead?"
  - "Why does the brush circle climb up walls and towers next to the pointer?"
  - "Why are paint settings stored as whole centimeters, percents and degrees instead of f32 values?"
---
## Decided
- The area view's placeable brush lives in crates/mg-ui/src/paint_mode.rs (Paint on the toolbar, key B): Scatter and Row, one `Paint Placeables` command per stroke. Its settings are `settings.paint`, in whole centimeters, percents and degrees, because `Settings` derives `Eq` and can't hold f32.
- Dabs land only on walkable walkmesh faces (`Ground::height_on` with surfacemat.2da `Walk`), at the level nearest the pointer's. The circle is drawn on the same ground, so it doesn't climb towers.
- A row's piece length comes from the model's box (`AreaView::bounds_of`, through `AreaScene::ghost`), plus a Gap that may be negative.

## Tried and ruled out
- Drawing the brush circle with `on_ground` (nearest tile level): it ran up walls and towers beside the pointer.

## Watch out
- See-through ghosts (GHOST_OPACITY 0.6) don't show in the egui_kittest wgpu harness pictures at all. On develop, `area_ghost` in screens.rs shows only the statue's cyan box. Set the opacity to 1.0 to see them in a test picture, or shoot after the stroke is laid. Hold ghosts with counts (`paint.shown`), not pictures.
