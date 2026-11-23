---
type: Gotcha
title: "egui popup sized to its content: Area remembers last frame, ScrollArea's 64 px least height"
description: "Why the F2 completion list wrapped names and showed a needless scroll bar, and how completion_size fixes it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T13:43:40Z }
touches: ["crates/mg-ui/src/script_view.rs:completion_size"]
asks:
  - why does the completion popup wrap names
  - why does a scroll area show a scroll bar with few rows
  - how do I size an egui Area popup to its widest row
fingerprint: v2-cc53713b4bfed7bd
created: 2026-11-23
author: augustaugur
---
- An egui `Area` lays out in the size it had last frame, so content-sized popups shrink-wrap wrongly (names wrapped "Ge/tIs/In/C") and a stale height made 3 rows 66 px in 64 available: a scroll bar. Measure once per list (cache keyed by a hash of names + font in ctx data) and set `ui.set_width`/`set_height` every frame.
- `ScrollArea`'s default `min_scrolled_height` is 64: a one-name list was three rows tall. Pass `.min_scrolled_height(h).max_height(h)`.
- Width = widest name by `fonts_mut(layout_no_wrap)` in TextStyle::Button + 2 x row `button_padding.x` + the bar's room only when rows exceed the max height; clamp to [least, room to the window edge], shift x left when even the least has no room; rows `Button::selectable(..).truncate()` in `top_down_justified` (the tip gives the whole name).
