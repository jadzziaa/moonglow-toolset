---
title: "Visual transforms: what the engine reads and the client draws"
author: August
date: 2026-10-25
branch: visual-transforms-saved
cost: { minutes: 180, tokens: 6979769 }
touches:
  - crates/mg-ui/src/area_tools.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/04-areas.md
  - docs/manual/09-build-and-test.md
  - docs/manual/12-differences.md
  - crates/mg-area/src/edit.rs
  - crates/mg-area/src/lib.rs
---

## Decided
- The engine reads every visual transform Adjust Location writes (VisTransformList scope 0, axes {TimerType, ValueTo, LerpType}; the old VisualTransform struct too), on Useable and non-Useable placeables, STATIC ones (server-side), creatures, doors, items: engine_visual_transforms (GetObjectVisualTransform). Saving was never the bug.
- The client's rotation convention (visual_transforms_look, chairs with MG_APPEARANCE=179): RotateX = about the model's upright Z, RotateY = about its X, RotateZ = about its Y; matrix T * Rz(rx) * Rx(ry) * Ry(rz) * S(ScaleX). ScaleY/ScaleZ are ignored; translation is in the object frame (not turned by the rotation). VisualTransform::matrix does this; tilt rings: red ring (about X) sets field 1 (RotateY), green (about Y) field 2 (RotateZ).
- The engine's own save of SetObjectVisualTransform(SCALE) writes ScaleX only (Y, Z = 1).
- The client draws a static placeable untransformed (already known); Adjust Location now says so (static_note), Verify has static-transform (warning, no fix).
## Tried and ruled out
- Eyeballing rotation order: six candidate orders rendered by Moonglow and measured against the client picture settled it (zxy outermost first); single-axis pictures alone can't tell order.
## Watch out
- scratch_dir wipes its folder on every call: pictures from a previous run of the same test are gone; copy them out.
- Another agent may stop the shared headless display (tools/aurora/headless.sh): client_screenshot then returns None (client false). Start it again and rerun.
- nwn_gff keeps fields in hash order: compare GFF structs with fields sorted.
