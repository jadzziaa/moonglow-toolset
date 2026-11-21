---
type: Decision
title: "Kit popover is its own Area, not an egui popup; the header measures in one frame"
description: "Why mg_uikit::popover hangs in its own egui Area with a click-outside rule, why EditorHeader items are measured data rather than drawn-then-measured closures, and why KitButton exists beside IconButton."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T23:38:00Z }
touches: [crates/mg-uikit/src/popover.rs, crates/mg-uikit/src/header.rs, crates/mg-uikit/src/button.rs]
asks:
  - Why does a combo box inside the kit popover not close it?
  - "Why is the editor header not laid out from last frame's widths?"
  - Why is there a KitButton as well as IconButton?
  - How do tests find a header control that went into More?
fingerprint: v2-692ac5e6af729e9d
created: 2026-11-14
---
- `Popover` (M4 U1) is shown in its own `egui::Area`, not `egui::Popup`: egui keeps one popup open at a time (memory), so a ComboBox opened from a popover row would close an egui popup (the NUI designer's known difference of M2). The popover closes on a click outside it and its anchor, or Escape, *unless an egui popup was open when the frame began* (`Popup::is_any_open` read before the rows run: by the click's frame the combo has already closed itself). Action rows and the header's More rows close it with `ui.close()` inside a `UiBuilder::closable()` scope; check rows leave it open.
- Because egui's popup memory does not see it, mg-ui's Escape rule (`widgets::dialogs` escape, `shell::keys_blocked`) must also ask `mg_uikit::popover::Popover::any_open(ctx)` once the app shows a kit popover (U3 wires it); otherwise an Escape closing More (as `common::header_holds` does) would also cancel a docked editor. The Area is `constrain_to(content_rect)`: moved inside the window, never flipped.
- `EditorHeader` measures every item before drawing (KitButton::width, given widths for custom items, Popover::unfolded_width), never last frame's width plus `request_discard` (that loses the click on the frame of a change and gives hysteresis: see T2's note on `three_parts_sized`). So yieldable items are data (`Item`), and a custom item must give its width.
- `KitButton` draws in tokens (disabled = text-off, accent-fill-off/on-fill-off) with `controls::box_painter` to undo egui's disabled fade. `IconButton` was NOT changed (it is egui's Button underneath, and changing it would restyle every toolbar before its editor's task); editors move onto KitButton through the header from U3.
- Tests: `mg_uikit::header::records(ctx, name)` is the record, one per header id (two docked editors' headers of one name each have theirs; `places` is the first's): places (Row/Icon/Unfolded/More/Cut) and the row's, end slot's, More's and open popover's rects. `common::header_holds` (and `header_holds_nth`) finds the real node on the row AND checks its rect lies inside the row, clear of the end slot and More, or opens More and checks the node lies inside More's popover. U1's review showed that finding the node alone passed with the left group drawn 5000 px to the right, so keep the rect checks.
- Popover/menu details from U1's review: every close path calls `Popover::forget` (else More keeps its old measured width); a header that missed a pass drops its open popover (else it reopens when its tab returns); a menu closes only when the widget clicked (`ctx.viewport(|v| v.interact_widgets.clicked)`) is an enabled, click-not-drag widget inside its rows (the Area itself and selectable labels are 'clicked' too); `box_painter` divides out the fade only in a disabled `Ui`; painter opacity shows in a text shape's color alpha, not `opacity_factor`.
- egui_kittest: a toggle button's state is `accesskit_node().toggled()`, not `is_selected()`; a ComboBox has no label (find it by role).
