---
type: Decision
title: "Appearance Gallery tab and pickers (U26): a well counts its border, a nested grid's height and bar from its own layout, the keys as the tab comes to the front"
description: "What U26 and its review round found: layout traps that grew windows, cut grids or repainted forever, how the gallery takes the keys without stealing them, Select Portrait's and Select Texture's lists, and the test-harness facts (zoom, label values, focus, tooltips) its tests rest on."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-20T00:49:13Z }
touches: ["crates/mg-ui/src/widgets.rs:picture_well", "crates/mg-ui/src/blueprint/item.rs:armor_part_gallery", "crates/mg-ui/src/widgets.rs:Dialog::show_parts", crates/mg-ui/src/appearance_gallery.rs, crates/mg-ui/src/blueprint/situated.rs, crates/mg-ui/src/blueprint/replacements.rs, crates/mg-ui/tests/app/appearance_tab.rs]
asks:
  - "The armor part pictures cut the second row's label, or the armor page repaints forever at some widths: why?"
  - How does the Appearance Gallery take the keys without stealing them from Find?
  - Why does a zoom set with set_zoom_factor not take in a Moonglow UI test?
  - "Why can't a test find a label's text or a tooltip in the AccessKit tree?"
fingerprint: v2-8380f78c8a5b7632
created: 2026-11-19
author: augustaugur
---
## Decided
- Pickers (Select Portrait, Select Texture) draw through `Dialog::show_footed` with the kit's `dialog::Footer` (`widgets::picker_footer`); grids sit in `widgets::picture_well`. Portraits are `GalleryGrid::justified(54x84, 8).row_gap(3)` so rows reach the scroll bar.
- Portrait cells keep the accessible name `po_<base>`; the tooltip is the base resref, as the board.
- Select Portrait's rows are keyed by mode, Race, Gender and the tables alone; the current portrait, if not listed (unpictured, plot), shows in the preview only. Keying by the chosen one shifted the list under the keys.
- Select Texture's grid has one line of name (`label_height(false, 1, false)`), rows 10 by 14 as the board; the default two lines left 16 points more per row.
- Select Texture: the texture replaced shows in the preview with its name; OK stays off until another is chosen (the tip's rule, the orchestrator's ruling); Enter on the grid is OK only with one chosen.
- The gallery grid takes the keys only as its tab comes to the front (`Gallery::shown = (pass last drawn, wants)`: a pass missed = the tab was behind), never each frame, so Find, another tab or a floating window keep them. Tracking `active_editor()` failed for floating galleries.
- The Status Bar count is the header's own words (`count_words`), one source.
- PlaceGive's Status Bar hint is "Right Click: Menu" (the orchestrator's ruling).

## Watch out
- egui counts a Frame's stroke in its size: a well of `size` must set its inner size to `size - 2*(margin + stroke)`, or a well as wide as its room grows the window each frame (`the_pickers_come_to_rest`).
- A nested `ScrollArea` low on a page gets down to `min_scrolled_height` (64): give a fixed-height grid `.min_scrolled_height` too. Take its height from the grid's own `layout(room)` with the room `GalleryGrid::show` takes (`available_width().min(clip.right - cursor.left)`); a height from SIDE cut row 2's label. And decide its scroll bar from the count (`AlwaysHidden` where all fit in two rows at the whole room, else `AlwaysVisible` and the bar's room off): with a bar shown only when needed, the bar's room shrank the cells, two rows fitted, the bar went, the cells grew past the height, and it came back each frame (endless repaint at 1300 and 1600 wide for a part of few choices). The armor gallery re-opens at the armor's part when its columns change (`placed` keyed by them).
- `Dialog`'s body room must take off the item spacing under the title row and above a footer.
- Tests: zoom through the app's `settings.ui_scale`, not `ctx.set_zoom_factor` (the app overrides it); a Label's text is its AccessKit `value()`, not `label()` (`query_by_label_contains` matches both); a grid's focus shows on its active-descendant cell, not the List; tooltips have no role, find them with common's `tip_at`; the Status Bar follows the floating window last pressed or under the pointer; an area view in sight animates tiles and never rests (rest tests use the blueprint harness).
- egui keeps a window's width after the app window shrinks while a dialog is open: open a picker after resizing in a test.
- The hint strip must truncate its words (a sweep from least width found them past the tab's edge).
- The tab's 56 ms idle at the tip was egui's tooltip delay over a resting pointer, not the gallery's repaint.
- A color change on an armor's Appearance page remakes the part gallery's thumbnails a few a frame: a test waits after it with `settle`, not `h.run()` (4 steps ran out under load in the workspace run).
