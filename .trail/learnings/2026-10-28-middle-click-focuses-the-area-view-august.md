---
title: "Alt + middle click focuses the area view's camera"
author: August
date: 2026-10-28
branch: worktree-agent-aa319f1f1213d7e0c
cost: { minutes: 1496, tokens: 12685058 }
touches:
  - "crates/mg-ui/src/area_view.rs#focus_at"
  - "crates/mg-ui/src/area_view.rs#camera_input"
  - crates/mg-ui/tests/app/area_view.rs
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
