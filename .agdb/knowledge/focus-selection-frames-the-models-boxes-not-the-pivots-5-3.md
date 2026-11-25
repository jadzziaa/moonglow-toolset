---
type: Decision
title: "Focus Selection frames the models' boxes, not the pivots (5.3)"
description: "Where Focus Selection (F, '.', the Outliner/tree double click) gets an object's bounds, what falls back to the position, and the distance rule."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T00:25:51Z }
touches: ["crates/mg-ui/src/area_view.rs:object_bounds_for_focus", "crates/mg-ui/src/area_view.rs:focus_bounds", "crates/mg-area/src/scene.rs:model_bounds", crates/mg-ui/tests/app/focus_frames.rs]
asks:
  - Why does F focus on the middle of a placeable and not its base?
  - "Where does Focus Selection get an object's box from?"
  - Why does focusing a waypoint not land exactly on its position?
  - How far does Focus Selection pull the camera back for a small or tall object?
fingerprint: v2-a63d5726b60d2c63
created: 2026-11-25
author: augustaugur
---
## Decided
- `object_bounds_for_focus(scene, model, i)` gives the world box: an outline's box for triggers/encounters; else `AreaScene::model_bounds(model, i)` (the drawn models' local box, the same `Shown::bounds` the selection box and picking use, but `None` where `AreaScene::bounds` would give an arrow's or marker's box, a bare light, or a model not loaded yet) transformed by `o.model_transform()` (8 corners, min/max); else the position. Used by `focus_selection` (F, '.', the context menu) and by the `app.area_focus` path (Outliner/module tree double click, Find Instance), so they agree.
- A waypoint whose model is drawn (nw_waypoint001's flag) frames its model's middle, about a meter up, not its position: the tests in area_view.rs (`focus_selection_with_f_and_period_keys`) compare loosely for that. Sounds and arrows still frame their position.
- Distance: unchanged above a half-extent of 8 m (2.4x, 12..200); below, `min(current, 25)` as before but never nearer than `half_extent * 2.4`, so a tall tree seen from close is drawn back to show it whole; it only ever pulls back.
- Cmd::FocusSelection stays Group::Area with no commands::Id (see the Focus Selection key note).

## Watch out
- Find Instance opening an area for the first time: its scene's models may not be loaded on the frame `area_focus` is read, so that first focus uses the position.
