---
type: Gotcha
title: "Area editor at 5,000 selected: where the frame went (Nov 2026)"
description: "What area_editor_at_scale measured with 5,000 placeables selected, what was cut (kept boxes, Adjust Location keyed picture, set lookups) and what is left (Properties panel form 2.7 ms, drag boxes)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T23:26:08Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-25T23:54:25Z }
touches: ["crates/mg-ui/src/area_view.rs:overlays", "crates/mg-ui/src/tile_select.rs:overlay", "crates/mg-ui/tests/frame_perf.rs:area_editor_at_scale", "crates/mg-ui/src/properties_panel.rs:panel"]
asks:
  - Why is the area view slow with thousands of objects selected?
  - Why did Adjust Location make the area redraw every frame?
  - How do I time the area editor at large sizes in frame_perf?
  - What does the Properties panel cost with a huge selection?
fingerprint: v2-d93d0f95378f53cb
created: 2026-11-25
author: augustaugur
---
frame_perf::area_editor_at_scale (FRAME_VIEW=<group or a view's whole name>) builds a scratch module: 32x32 Rural area of 5,000 placeables, an empty area, Rural hills (corners raised 2x2 in every 4x4, filled with mg_tiles::fill), 30,000 placed in six areas, placeables.2da +30,000 non-light rows in the scratch user's override, a 2,000-tile ttr01.set copy. Each view is set against the same view at rest timed just before (UI_LINE 70 us); compare within one run, since other agents' builds on the machine move whole-frame figures by 30% while the in-run delta holds.
Not obvious: (1) with 5,000 selected the biggest single cost outside the area view is the Right Dock's Properties panel multi-edit form (properties_panel::panel: form_ui ~1.7 ms, follow::window_id per path ~0.65 ms, paths/also ~0.3 ms) = 2.7 ms a frame; left, needs the blueprint form to keep its values. (2) Adjust Location was in `follows`, so the picture was drawn again every frame even with values at rest (8 ms on 5,000 objects); now the Previewed poses are part of Drawn. (3) The selection boxes (24 lines/object, meshed) cost 3 ms; kept in AreaView::boxes_kept keyed by OverlayKey+revision+selection+show+colors+Adjust poses (Marks::kept_poses None while a drag/key/handle poses), rebuilt every drag frame. The 8-corner fast path (box_edges_on_screen) gives identical points but saved little: the per-edge cut was not the cost. (4) context_menu's placeables filter used selection.contains (O(objects x selection), 2.8 ms). (5) Dragging even one object of 5,000 is 10 ms: the picture redraw (renderer), not UI. (6) boxes_kept's key also holds merchant_signs (scene.draws/bounds read it); a variable preview (update_objects without an edit) bumps overlay_generation so kept boxes and Adjust's kept previewed follow. (7) Adjust Location's previewed() was 132 us/frame on 5,000 (a HashMap of the area's objects each frame); kept in AreaView::previewed_kept keyed by (revision, overlay_generation) and the AdjustLocation itself: 8 us. (8) Update Instances counted 30,000 ticks each frame (44 us); UpdateDraft::ticked keeps it until the window changes a tick. (9) Set views against a rest timed just before them: after a drag of 5,000 and its undo the at-rest view runs 0.4-0.7 ms higher than before it (cause not found), which once made Adjust Location and the context menu look +0.9 ms over; against a rest re-timed after the drag they are within. combo_looks' style copies with a popup open are only 7 combos x 4 us. (10) With a light in hand, bar::hints resolves light_of_row every frame (confirmed with a print; the row's name is "White"), but no "Placing a light" label was found in the harness's accesskit tree; the test asserts placing_light and the status counts ("32 by 32 tiles") instead.
