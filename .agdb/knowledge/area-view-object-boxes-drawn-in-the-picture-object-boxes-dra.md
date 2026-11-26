---
type: Decision
title: "Area view object boxes drawn in the picture (object_boxes, Drawn.boxes)"
description: "How the area view hands its selection and marker boxes to mg_render::Scene::boxes with no frame of lag, what stayed in egui, and the stacking change."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T20:24:23Z }
touches: ["crates/mg-ui/src/area_view.rs:object_boxes", "crates/mg-ui/src/area_view.rs:viewport", crates/mg-ui/tests/app/selection_boxes.rs]
asks:
  - "where are the area view's selection boxes drawn"
  - why do selection boxes not lag behind a drag
  - why are the boxes now under trigger outlines and door arrows
  - what happened to boxes_kept and BoxesKey
fingerprint: v2-b972ab561efa8e30
created: 2026-11-26
author: augustaugur
---
## Decided (26 Nov 2026, with the Interface pipeline)
- viewport() builds the boxes (object_boxes) from `shown`, the copy the frame posed (drag, key, handle, Adjust Location), and puts them in `Drawn.boxes`, the picture's key, before `kept` is decided; frame.boxes takes them before render_targets, in the same egui pass as the Image showing the texture. So no frame of lag, and a selection change redraws the picture (nothing else keys it now). `kept` must stay ahead of vp.size_targets (it reads the old targets' size), so poses/shown/boxes/kept/paint_instances were moved above it.
- has_box(kind, selected, in_scene) is the one rule: no outline kinds; selected, or not drawn by the scene and not an arrow kind (an undrawn waypoint/store is an egui arrow). Markers first, then selected (lines in that order). Widths: points x the target's ppp (with Area view resolution's share); colors to_srgba_unmultiplied (gamma_multiply'd Color32 is premultiplied).
- boxes_kept/BoxesKey/box_builds/Marks::kept_poses removed. The egui box path stays under scene.is_none() as agreed, though viewport() returns without a scene, so the app never reaches it.
- Stacking changed: boxes now lie under every egui overlay (outlines, spawn posts, bare light's sun/ring, door arrows). The owner's call; pictures in target/selection-boxes-pictures/{before,after}.
## Watch out
- At Area view resolution 50 the 1 pt lines are 0.5 px in the target: drawn 1 px faint, stretched, so softer than egui's were.
- Tests: AreaView::picture_boxes() and shown_object(i) (posed copy of the last frame); selection_boxes::the_boxes_follow_a_drag_in_the_same_frame fails if boxes come from the unposed model.
- Timing (frame_perf area_editor_at_scale, load 10-15): drag of 5,000 +2.1..2.3 ms over one (develop +3.5..5.5); eu-stack samples put the rest in the picture's draw, not the boxes.
