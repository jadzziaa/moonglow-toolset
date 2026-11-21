---
type: Gotcha
title: "Gallery cell (U25): one kit grid for every gallery, pictures in sight plus a row ahead, the enlargement beside the dock"
description: "How M4's U25 and its review built the galleries on mg_uikit::gallery (GalleryCell, GalleryGrid, show_beside), the keys, hints and screen-reader roles, and the egui and kittest traps met on the way."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T18:15:14Z }
touches: [crates/mg-uikit/src/gallery.rs, crates/mg-ui/src/palette_view.rs, "crates/mg-ui/src/widgets.rs:open_gallery_at", crates/mg-ui/src/area_view/bar.rs, crates/mg-ui/tests/app/gallery_keys.rs, crates/mg-ui/tests/app/galleries.rs]
asks:
  - why does a gallery reopen at the wrong cell
  - how do I test that a grid ignores keys while a menu is open
  - are kittest node rects in points or pixels
  - why does the overlap sweep flag a gallery list
  - which hints win when placing from the palette gallery
fingerprint: v2-463d6ac4d043ee8c
created: 2026-11-19
author: augustaugur
---
## What was built
- `mg_uikit::gallery`: `GalleryCell` (Quiet Tiles states, the name rule, `.small()`, `.model()` one-line Hack names), `GalleryGrid` (laid out by `GridLayout` arithmetic, paints only rows in sight, one Tab stop by `Sense::focusable_noninteractive`, arrows/Home/End/Page keys move a cursor scrolled into sight; `GridOutput::{moved, activated, ahead}`), `size_slider` (step 1), `beside`/`show_beside` (an enlargement beside the panel, never over it, inside the window; no room either side: above the cell, else below, else none; a new enlargement measured in a sizing pass first).
- The palette keeps its own keys (`Listed::step`, and `step_far` for Home/End/Page Up/Page Down: first/last showing row, a heading counts; pages by row tops and `Listed::page`) and visits only the rows in sight plus one ahead. `Thumbnails::shown_now()` counts pictures asked for in a frame.
- The enlargement says name, ResRef and Tag; the Tag read once per hovered blueprint (`palette_view::tag_of`, counter `tags_read_one_by_one`). It stands beside `app.right_rect`: the palettes never float (`Brings` for an Area tab brings no Palette).
- Placing's hints win over the Gallery's (`area_view/bar.rs`: the brush branch first, the Gallery's arrows appended while it has the keys).
- Wide layout with Gallery on: Right Dock 881, first column 480 (`shell::gallery_wide`).

## Watch out
- egui reports `WidgetInfo::selected` as *toggled*; a cell's selected state needs `ctx.accesskit_node_builder(id, |n| n.set_selected(..))`.
- Cells sensed with `Sense::click()` are Tab stops; use `Sense::CLICK | Sense::DRAG`.
- A grid is said as a List (`said_as_list`: role, `size_of_set`), each built cell its place (`said_in_list`), the keys' cell as `active_descendant`. The cells are the List's siblings in the AccessKit tree, not its children. The overlap sweeps (`common::overlapping`, `right_dock::dock_wrong`) must pass the List role by (it lies under its own cells).
- The kit grid's kept cursor is `Kept{cursor, given, count, pass}`: kept only while shown pass after pass with the same given cursor and count. Before, it survived a reopening, so Select Texture reopened at the last-stepped cell, not the chosen texture.
- `ui.scroll_to_rect` bubbles to every parent scroll area; open a gallery at a cell via the ScrollArea's own `State.offset` (`widgets::open_gallery_at`).
- kittest `Node::rect()` is already in points at the zoom; do not divide by `pixels_per_point`.
- To test that keys do nothing while a menu is open: `egui::Popup::open_id(&ctx, id)` before each key and step holds a menu open; `Popup::close_all` after (`tests/app/gallery_keys.rs`).
- Tests of an enlargement must measure the whole tooltip area (Order::Tooltip layers, `area_rect`), not one label in it, and pass by the measuring pass: `show_beside` lays a new enlargement out unseen (sizing pass) at the window's top-left corner, yet it is a visible layer and in the AccessKit tree; counting it let a test of where the enlargement stands pass with it never shown (`galleries.rs` `enlargements`).
- In the tiles palette, clicking a cell whose brush is already chosen lets the brush go.
- The Right Dock's least width is 280: a 200-wide dock cannot be tested.
- Do not run un-awaited `edit()`s in codemode on one file: concurrent writes emptied gallery.rs.
