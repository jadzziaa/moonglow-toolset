---
type: Decision
title: "Alt + middle click focuses the area view's camera"
tags: []
status: stable
generated: { by: trail, at: 2026-10-28T00:00:00Z }
touches: ["crates/mg-ui/src/area_view.rs#focus_at", "crates/mg-ui/src/area_view.rs#camera_input", crates/mg-ui/tests/app/area_view.rs]
created: 2026-10-28
author: August
description: "Alt + middle click sets the area view's `Orbit::target` to `AreaView::focus_at` (walkmesh hit, else tile floor plane), leaving distance, yaw and pitch unchanged; plain middle click was dropped, and tests must send Alt via `h.event`."
asks:
  - "How do I recenter the area view camera on a point under the mouse cursor?"
  - "Why does a plain middle click not focus the camera in the area view?"
  - "My kittest Alt + middle drag turns the view instead of panning; why does the Alt modifier get lost?"
  - "Why not use `ground_at(pos, 0.0)` to pick the focus point on the ground?"
---
## Decided
- Alt + middle click (Blender's, El Grillo's ask) sets `Orbit::target` to `AreaView::focus_at`: the walkmesh hit (`Ground::hit`), else the nearest tile floor plane (`AreaTile::position.z`) the ray meets inside that tile's square. Distance, yaw and pitch untouched; camera moves are plain `view.orbit` mutations (no undo), like Go to Start Location.
- Alt only, the user's decision: a plain middle click was tried and dropped as disruptive (egui counts a middle drag under ~6 px as a click, so it would jump the view). Not with Ctrl or Shift, nor with the left or right button down. The middle button's drags: turn; Alt: pan; Ctrl+Shift: raise.
- Mouse gestures stay out of Options › Keyboard (settled in deferred.md, 'Not every key').

## Ruled out
- `ground_at(pos, 0.0)` for the pick: its z=0 plane fallback answers for any downward ray, past the area's edge too, and at the wrong height on a raised tile.

## Watch out
- kittest's `event_modifiers` queues ModifiersChanged(NONE) right after the event: a drag driven with it loses Alt on the next frame (an Alt + middle drag turned instead of panning). Send ModifiersChanged(alt) and the button events with `h.event`.
- The overview looks straight down: no sky on screen. A sky test sets a low pitch first.
- The model viewer has the same orbit fields but nothing to pick (no ground, no ray against the model).
