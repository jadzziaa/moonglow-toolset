---
type: Decision
title: Open dropdown widening to the window edge
tags: []
status: stable
generated: { by: trail, at: 2026-10-28T00:00:00Z }
touches: ["crates/mg-ui/src/widgets.rs#RowChoice", crates/mg-ui/tests/app/fields.rs]
created: 2026-10-28
author: August
description: "egui dropdown popups keep any one-frame extra width forever because Area max_rect, combo set_min_width and ScrollArea feed back; RowChoice::show_optional fixes it by computing width once on open and setting it every frame."
asks:
  - "Why does my egui combo box dropdown keep getting wider until it reaches the window edge?"
  - "How do I stop an egui popup menu from growing after a scroll bar appears for one frame?"
  - "Where was the dropdown widening bug fixed and how does RowChoice::show_optional set the popup width?"
  - "How should I measure frame-by-frame popup width in egui tests, h.step() or h.run(), and why?"
---
egui's popup (Popup::menu -> Area) takes its max_rect from last frame's content min size, and egui's combo calls set_min_width(available_width) inside it; a vertical ScrollArea follows its content's width on x and adds its bar width, and selectable rows fill the width. So any extra width in one frame (a scroll bar appearing, a row wider than the box, sub-pixel rounding) is kept for good: the Appearance list measured 571 then 581 on its first two frames. Fixed in RowChoice::show_optional by computing the popup's width once when it opens (box width, or widest row galley + 2*button_padding + scroll bar) and ui.set_width(it) every frame. Measure such tests from h.step() right after the click: h.run() steps past the frame that grew and hides it. Tried and ruled out for reproducing the builder's gradual growth (Windows): pixels_per_point 1.1-1.75, ui_scale 125, step_dt 1/144, fractional window x, hovering and wheel scrolling, a taller inner list: none grew beyond the one-off jump on Linux.
