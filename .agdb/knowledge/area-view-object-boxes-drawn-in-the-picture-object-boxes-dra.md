---
type: Decision
title: "Area view object boxes drawn in the picture (object_boxes, Drawn.boxes)"
description: "How the area view hands its selection and marker boxes to mg_render::Scene::boxes with no frame of lag, how the list is kept at rest, the least width, and the stacking ruling."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T21:09:54Z }
touches: ["crates/mg-ui/src/area_view.rs:object_boxes", "crates/mg-ui/src/area_view.rs:viewport", crates/mg-ui/tests/app/selection_boxes.rs]
asks:
  - "where are the area view's selection boxes drawn"
  - why do selection boxes not lag behind a drag
  - why are the boxes now under trigger outlines and door arrows
  - why are boxes at least a pixel wide at low Area view resolution
fingerprint: v2-7797c558877cf076
created: 2026-11-26
author: augustaugur
---
## Decided (26 Nov 2026, with Interface and the owner)
- viewport() builds the boxes from `shown`, the copy the frame posed, before `kept` is decided (kept must stay ahead of vp.size_targets), and puts them in Drawn.boxes (PictureBoxes: Arc, equal by pointer, else by content). frame.boxes gets them before render_targets in the same pass: no lag; a selection change redraws the picture.
- Kept list: AreaView::boxes_kept keyed by BoxesKey (revision, overlay_generation, selection, show, colors, ppp, merchant_signs, Adjust Location's Previewed); no key (rebuilt every frame) while a drag/key/handle poses (kept_poses None). Uncached it cost 70-116 us/frame at rest with 5,000 (iterating every object, even with 1 selected); kept 0.4-4 us. Counters box_builds, posed_builds for tests.
- has_box(kind, selected, in_scene): no outline kinds; selected, or undrawn and not an arrow kind. Markers first, then selected.
- Widths: max(points x target ppp, 1.0) and the selected edge >= line + 2 px (owner's ruling: at resolution 50 the renderer's sub-pixel fade made boxes faint). Colors to_srgba_unmultiplied.
- Boxes stay under egui overlays (outlines, sun, door arrows): owner's ruling. The no-scene egui path and box_edges_on_screen were deleted (unreachable).
- Non-finite transforms are skipped (a NaN key would redraw every frame).
## Watch out
- view.posed.take() happens before the boxes; no return between take and put-back (checked); posed_builds proves the copy is kept through a drag.
