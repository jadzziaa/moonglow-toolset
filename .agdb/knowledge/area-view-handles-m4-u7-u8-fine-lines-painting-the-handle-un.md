---
type: Gotcha
title: "Area view handles (M4 U7/U8): Fine Lines painting, the handle under the pointer, testing them by drawn shapes"
description: "How the area view's handles are painted (handles.rs Canvas layers), why handle_under mirrors the press's order instead of the press calling it, and the traps met testing them: Status Bar glyphs in the same colors, kittest's press dropping Shift, rings hit-tested at 48 pieces but drawn at 96."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-15T19:20:44Z }
touches: [crates/mg-ui/src/area_view/handles.rs, "crates/mg-ui/src/area_view.rs:handle_under", "crates/mg-ui/src/area_view.rs:overlays", crates/mg-ui/tests/app/handles.rs, "crates/mg-ui/tests/screens.rs:board_gizmo_states"]
asks:
  - "Where are the area view's rings, arrows and scale handle painted, and in what order?"
  - Why is a handle lit or named under the pointer but a press there does something else?
  - "My drawn-shape test of a handle finds two of everything: why?"
  - "Shift is held in my UI test but the drag turns instead of tilting: why?"
fingerprint: v2-af11f1dafad22165
created: 2026-11-15
---
## Decided
- The handles are painted by `area_view/handles.rs` (proposal section 11, "Fine Lines"): every shape goes into a `Canvas` of layers (box, fill, back edge/line, edge/line, top edge/top, chip) painted once at the end of `overlays`, so no dark edge is drawn over another handle's colored line. The selection boxes stay in `overlays`' own batched `lines_mesh` path (edges then lines, under the canvas).
- Colors are the kit's `handle-*` tokens, one value in both themes (the 3D picture is the same); only the chips (`chip`, `chip-rim`, `text`) follow the theme. `the_themes_differ_but_for_the_thumbnails` exempts `handle-*`.
- `AreaView::handle_under` finds the one handle a press would take (spawn tip/foot without Shift, then with Shift scale, arrow, tilt; without, the ring; none with Ctrl or Alt; else the start marker where nothing is picked). Since U7's review the press's drag start matches on it and `object_takes_press` asks it: one copy. Where the press goes at all is `PressTo::of(app, view)`, which `input` dispatches its placing states by (paste, redraw, start, appearance, light, blueprint) and the overlays ask (`reaches_handles`) before lighting, naming or giving the hand: a new mode that takes the press goes into `PressTo`, not into a second list.
- A handle is known by its object and kind (`Handle::same`, the `of` field of `TurnRing`, `TiltRing`, `AxisArrow`; `Handle::Scale` holds the object), never by its geometry: `handle_under` reads the area's model, the drawing reads the posed copy (Q and E before the keys rest), so comparing whole structs dimmed the whole set. `handle_under` asks `start_grab` before `pick` (a pick each hover frame otherwise).
- `Lit` says what is lit: the handle under the pointer, or what a drag holds (a turn every ring, a tilt every ring of its axis, a slide every arrow of its axis, a scale every square); everything else in the hovered object's own set draws at 50% (the coordinator's ruling: other objects' sets stay whole; a drag dims every set but what it holds).
- Rings are drawn as 96 projected points (`RING_DRAWN`) cut into runs at the near plane (`handles::runs`); hit tests, `tilt_grip`, `start_grab` and `handle_points` keep `RING_SEGMENTS` (48), so reach is unchanged. A tilt ring's halves split a quarter turn either side of the point nearest the eye.
- Snap ticks sit at the facings a snapped turn lands on: rotation multiples, a quarter turn from facing-angle multiples (they coincide at 15 degrees, not at 20).

## Watch out
- The Status Bar draws glyphs (ring, axes) in the handle colors: a test reading `drawn(h)` must keep to shapes inside the view's rect (`handles.rs`' `over_view`). A guide line runs past the view, so read it from all shapes.
- `common::press` uses `event_modifiers`, which puts back the harness's own modifiers (none) after the press: a Shift drag started with it becomes a plain one. Send the `PointerButton` with `h.event` after `ModifiersChanged`.
- egui 0.36's `RawInput` has no `modifiers` field: frame_perf holds Shift with a `ModifiersChanged` event before the view.
- `git apply` run in a subfolder of the repo (a `git archive` copy under `target/`) silently skips every path outside that folder and still exits 0: use `patch -p1` there.
