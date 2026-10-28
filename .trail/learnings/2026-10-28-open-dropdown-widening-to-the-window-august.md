---
title: Open dropdown widening to the window edge
author: August
date: 2026-10-28
branch: worktree-agent-aa952e33f6c356d37
cost: { minutes: 90, tokens: 11015621 }
touches:
  - "crates/mg-ui/src/widgets.rs#RowChoice"
  - crates/mg-ui/tests/app/fields.rs
---

egui's popup (Popup::menu -> Area) takes its max_rect from last frame's content min size, and egui's combo calls set_min_width(available_width) inside it; a vertical ScrollArea follows its content's width on x and adds its bar width, and selectable rows fill the width. So any extra width in one frame (a scroll bar appearing, a row wider than the box, sub-pixel rounding) is kept for good: the Appearance list measured 571 then 581 on its first two frames. Fixed in RowChoice::show_optional by computing the popup's width once when it opens (box width, or widest row galley + 2*button_padding + scroll bar) and ui.set_width(it) every frame. Measure such tests from h.step() right after the click: h.run() steps past the frame that grew and hides it. Tried and ruled out for reproducing the builder's gradual growth (Windows): pixels_per_point 1.1-1.75, ui_scale 125, step_dt 1/144, fractional window x, hovering and wheel scrolling, a taller inner list: none grew beyond the one-off jump on Linux.
