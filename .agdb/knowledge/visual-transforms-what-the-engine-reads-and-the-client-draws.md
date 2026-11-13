---
type: Decision
title: "Visual transforms: what the engine reads and the client draws"
tags: []
status: stable
generated: { by: trail, at: 2026-10-25T00:00:00Z }
touches: [crates/mg-ui/src/area_tools.rs, crates/mg-ui/tests/app.rs, docs/deferred.md, docs/findings.md, docs/log.md, docs/manual/04-areas.md, docs/manual/09-build-and-test.md, docs/manual/12-differences.md, crates/mg-area/src/edit.rs, crates/mg-area/src/lib.rs]
created: 2026-10-25
author: August
description: "The engine reads all visual transforms written by Adjust Location, but the client ignores them on static placeables; rotation maps RotateX/Y/Z to Z/X/Y axes (T*Rz*Rx*Ry*S), and only ScaleX is used."
asks:
  - "Why does my placeable's Adjust Location rotation or scale not show up in the client even though it saved correctly?"
  - "Which axis does RotateX, RotateY or RotateZ rotate around in the client, and in what order is the matrix composed?"
  - "Does ScaleY or ScaleZ have any effect on an object's visual transform, and what does SetObjectVisualTransform(SCALE) write?"
  - "Why does client_screenshot return None during a test, and why are my earlier test pictures missing from scratch_dir?"
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
