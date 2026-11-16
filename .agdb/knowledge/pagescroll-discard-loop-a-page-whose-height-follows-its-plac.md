---
type: Gotcha
title: "PageScroll discard loop: a page whose height follows its place on screen (Flip hold)"
description: "Why egui warned of request_discard a thousand frames in a row from mg_uikit::pages, which page caused it, and how pages::Flip holds a page whose height will not settle."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T13:43:23Z }
touches: [crates/mg-uikit/src/pages.rs, crates/mg-ui/src/blueprint/situated.rs]
asks:
  - egui PERF WARNING request_discard called 1000 frames in a row from pages.rs
  - why does a blueprint window keep repainting
  - may a form page size itself by ui.clip_rect
  - how do I count discarded passes in a kittest test
fingerprint: v2-d00a0601afd23036
created: 2026-11-16
---
- PageScroll places pages above the held one up from it by their kept heights and discards the pass when such a page measures anew. A page whose height depends on where it stands (sized by `ui.clip_rect().bottom() - ui.cursor().top()`, as U10's `situated::beside_model` sized the model) grows or flips each time it is placed: a discard every frame, egui's PERF WARNING in the picture's corner (`armor_part_pictures` on U10's tip).
- U11 removed beside_model; `pages::Flip::hold` now holds a page whose height changes back at once, or changes two frames running, at the larger height for HELD_FRAMES (20) frames, in one column and in columns.
- Counting discards in kittest: the Harness does run extra passes; compare `ctx.cumulative_pass_nr()` with `ctx.cumulative_frame_nr()` over N `step()`s. `h.output().platform_output.requested_discard()` is not a reliable count.
- Page sizing by the room in sight is still used in a few lists (inventory.rs, picker.rs); they settle today, and the sweep test `form_pages::no_form_page_is_drawn_again_every_frame` would catch one that does not.
