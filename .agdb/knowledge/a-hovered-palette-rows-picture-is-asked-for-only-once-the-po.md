---
type: Gotcha
title: "A hovered palette row's picture is asked for only once the pointer rests (M3 T14 review): the 8,000-row wheel cost"
description: "The palette asked for the hovered row's thumbnail every frame, so a wheel scrolling under a resting pointer made a picture a frame (8 to 12 ms with 8,000 rows); now only after a short rest on one row (`HOVER_REST`, 0.12 s since T14's second review)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T09:01:15Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-14T11:52:53Z }
touches: [crates/mg-ui/src/palette_view.rs, "crates/mg-ui/tests/frame_perf.rs:palette_rect", "crates/mg-ui/src/model_view.rs:Thumbnails"]
asks:
  - Why was the palette slow under the wheel in a narrow Right Dock?
  - "When does the palette ask for a hovered blueprint's picture?"
  - Where does frame_perf put the pointer over the palette?
supersedes: [the-palettes-8-000-rows-under-the-wheel-cost-8-8-ms-in-a-280]
fingerprint: v2-d36661bc126bb059
created: 2026-11-14
---
## Found (the review of M3 T14, superseding the note that blamed the dock's width)
- `palette_view` asked `app.thumbnails.get(Pictured::Resource(key))` for the hovered row every frame. While the wheel scrolls under a resting pointer the row under it changes every frame, so a picture was made every frame: 8.8 to 11.9 ms with the custom item palette's 8,000 rows. It looked like the dock's width only because frame_perf turned the wheel at the dock's middle, which in a 280 dock is on the rows' words and in a 320 dock past their end (a row's hover is its words).
- Fix: `PaletteView::hover_since`: the picture and the `about` lines are asked for only once the pointer has rested on one row for `palette_view::HOVER_REST` (a repaint asked at its end). T14's review had used egui's whole tooltip delay, so every tooltip opened without its picture and grew; the second review made it 0.12 s, still never reached under a turning wheel (`right_dock::a_rested_row_s_picture_is_asked_for_before_its_tooltip_opens`). `right_dock::a_wheel_over_the_palette_asks_for_no_pictures_of_the_rows_it_passes` (hidden `Thumbnails::asked`).
- frame_perf's `palette_rect` is 120 points wide from the panel's left, so the wheel turns on the rows' words at every dock width (and in the wide layout's first column).
- The tree, Resources, the Outliner and the galleries ask for no picture of a hovered row (checked: only `palette_view` did).
