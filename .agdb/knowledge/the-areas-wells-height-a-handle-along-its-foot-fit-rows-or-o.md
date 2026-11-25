---
type: Gotcha
title: "The Areas well's height: a handle along its foot, Fit, Rows or Open Out (5.3 builders' request)"
description: "How the Module panel's Areas well is resized or opened out (Settings::areas_height, tree::well_handle) and the layout traps met doing it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T00:12:54Z }
touches: ["crates/mg-ui/src/tree.rs:well_handle", "crates/mg-ui/src/tree.rs:areas_well", "crates/mg-ui/src/tree.rs:in_well", "crates/mg-ui/src/settings.rs:AreasHeight"]
asks:
  - how is the areas well in the module tree resized or opened out
  - "why is the well's handle drawn in module_tree and not in areas_well"
  - where is the areas group height kept and how is it reset
  - "why does dragging the well's foot only open it out on release"
fingerprint: v2-300e5a079ba03203
created: 2026-11-25
author: augustaugur
---
`Settings::areas_height` (`AreasHeight::{Fit, Rows(u16), Open}`, not written while Fit, hand-written tolerant Deserialize, reset by View › Reset Layout) says how high the Module panel's Areas group is. Fit is ruling 10's well (amended 2026-11-24 after El Grillo's and silentrocks' asks). Rows(n) is a well n rows high (at least WELL_LEAST_ROWS), possibly taller than the tree's view. Open is no well: `in_well` lays the rows out with plain `add(ui)`, setting neither `well_view` nor `tree-well-rect`, so bring-into-view, Home/End and the wheel fall to the tree's ScrollArea as before 5.1.

Traps:
- The handle takes no layout height. Tests assert `well_rows == 14.0/5.0/3.0` and `row.bottom() == next_group.top()` exactly. It is painted over the boundary (grip 3 pt either side).
- It is registered in `module_tree` AFTER the tree's ScrollArea (from the `WellFoot` temp kept by `areas_well`/`in_well`). Registered inside `areas_well`, the Conversations row laid out later is on top of its lower half in egui's hit test. `Sense::CLICK | Sense::DRAG` without FOCUSABLE, as the docks' seams.
- Rows while dragged = the rows shown when pressed + whole rows the pointer moved since `pointer.press_origin()`. Two failed forms: (1) offset taken from the pointer at `drag_started` (egui starts a drag only past its threshold: the first step, 27 px in kittest's 8-step `drag`, was added, one row too many); (2) rows from the pointer's distance to the well's top as drawn: with the tree scrolled to its end, a shorter well scrolls the tree back, the well moves down under the pointer and shrank frame by frame to its least.
- While held (`ctx.dragged_id()` / `drag_stopped_id()` is the handle's), the handle is interacted with even with its foot out of the tree's view or with nothing to give, or the release (where Open is set) was lost.
- Open is set on `drag_stopped`, not mid-drag. While held, the well is clamped to all its rows, so the handle stays under the pointer. Never opened out while the Filter is typed (it shows only some areas).
- A well taller than the tree's view: `scroll_to_rect(whole well)` no longer shows the row. `areas_well` then scrolls the tree to where the row will be after the well's own scroll (`WellTarget::at` minus the offset change).
- No handle for a group mid-open/close animation (`header.openness < 1`: the foot is clipped), nor while the areas fit, nor when they number at most WELL_LEAST_ROWS.
- Known: kinds lists opened under an area take ui-derived ids (`TreeGroup` -> `make_persistent_id`), so they fall back to their defaults once when switching between a well and Open Out, as they already did when the Filter's well id changed.
Tests: crates/mg-ui/tests/app/areas_height.rs (no game needed, `n_areas` modules). Pictures: screens.rs `shell_tree_well_heights`.
