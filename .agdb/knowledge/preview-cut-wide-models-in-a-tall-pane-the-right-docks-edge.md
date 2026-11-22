---
type: Gotcha
title: "Preview cut wide models in a tall pane; the Right Dock's edge seam (M5 W8)"
description: "Why the owner's build cut wide placeables (carts) in the tall Preview, what W8 fixed in ModelView, and why the Right Dock's and a floating form's Preview pane's edges got their own seam."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T19:17:31Z }
touches: ["crates/mg-ui/src/model_view.rs:frame", "crates/mg-ui/src/model_view.rs:refresh", "crates/mg-ui/src/shell.rs:edge_seam", crates/mg-ui/src/lib.rs, crates/mg-ui/src/tabs.rs]
asks:
  - why does the Preview cut off a wide cart at the sides
  - "is the Right Dock's edge draggable and how wide is its grab"
  - "why does egui's panel resize handle not show the cursor beside a form"
  - "does the Preview refit when the blueprint's appearance changes"
fingerprint: v2-cde4b9ee9869f5c6
created: 2026-11-21
author: augustaugur
---
- The owner's installed Moonglow 4.2.1+753023de (19 Nov) framed by a sphere fitted to the vertical 20 deg alone: `radius / sin(20 deg) * 1.1`. In a tall, narrow Preview (wide layout, or the 280-320 dock beside a form) a wide model (TDoD's dd_bca_cart*, a long table) was cut at the sides. U11 (3add227e) already replaced it with `fitting_distance` (each point's across and up, aspect-aware, ROOM 1.08), the same as mgv's `frame_tight`. Break 1 in target/m5/W8/breaks.txt puts the old fit back.
- A real fault remained: `ModelView::refresh` rebuilt the model for a changed appearance (Appearance changed in its form) and kept the distance framed for the old one. Now it calls `frame()` again when `framed` and the source is a blueprint (`!model`): a camera moved by hand stays, and a model's own file read again keeps its camera (gpu_memory's test holds that).
- Thumbnails, galleries and pickers frame through `mg_preview::compose::framed_on` at aspect 1.0 in square cells: a different path and correct. W6's shared picture is letterboxed into the second pane, keeping it whole.
- egui's Panel resize handle is 3 points either side, but widgets drawn later (the editor area: a form's scroll area) take the hover over the outer side, so beside a blueprint form only 2.5 points showed the cursor; a floating form's Preview pane (`window_panes`' right panel) had 3. Fix: `shell::edge_seam`, an interact made after the central area (`right_edge_seam` from lib.rs; `window_edge_seam` from tabs.rs) by the pattern of the dock's other seams (`column_seam`, `section_seams`, mg-ui's own in shell.rs, not the kit's): 6 points, ResizeHorizontal, a lit line. It senses `Sense::DRAG` alone: in egui 0.36 `Sense::drag()` is DRAG | FOCUSABLE, and the seam, made last in the window, joined the Tab order and scrolled Module Properties away (table_panels' grid-keys test). `column_seam` and `section_seams` in shell.rs had the same fault and got `Sense::DRAG` in W8's fix round (test `preview_panel::tab_passes_the_right_dock_s_seams`). Any new seam: `Sense::DRAG`, never `Sense::drag()`. Its id `right-dock-edge`.with("seam") is excluded in right_dock.rs `dock_wrong` like egui's edge.
- A floating form's Preview pane: 280 to 60% of the window, kept for the run only (not in settings); the Right Dock: 280 to two fifths, saved; under 1400 held at 280; folded, a strip.
- Turning (W8's fix round, review C's W8-1, the owner's ruling "preview turn: a"): `fitting_distance` is now yaw-free. Each point becomes the ring it sweeps about the target's up line ((horizontal distance, height) pairs); the need per yaw is convex in those, so only the convex hull's corners (`hull_of`) are fitted, each ring at 72 points (5 deg). A turn by drag or arrow keys never carries a framed model out, and the distance stays the same while turning (test `preview_panel::a_framed_model_stays_whole_as_it_turns`: plank and long table framed at yaws 60, 0, -30, 90 in a 319x864 Preview, a whole turn in 15 deg drags). Before, the plank framed end-on reached -1.3..2.3 of the width when turned. Long models show about 10-15% smaller at the default yaw (accepted by the owner). Moonglow Viewer's `frame_tight` still fits the current yaw only and centers in the view's axes, so mgv frames a long model tighter and a turn there can still cut it.
- Thumbnails and gallery cells (`mg_preview::compose::framed_on`) don't turn and keep the per-yaw fit.
- Tests can find a seam as a widget in `ctx.viewport(|v| v.prev_pass.widgets)` with `sense == Sense::DRAG` (egui's panel edge senses `click_and_drag`; a `Window`'s own edges are `Id::new(layer).with("edge_drag").with(side)`, also DRAG alone, so exclude them).
