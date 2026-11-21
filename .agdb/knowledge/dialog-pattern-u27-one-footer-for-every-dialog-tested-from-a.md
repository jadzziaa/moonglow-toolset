---
type: Decision
title: "Dialog pattern (U27): one footer for every dialog, tested from a source scan, the pitfalls of egui's windows"
description: "How every dialog gets the footer (widgets::Foot through Dialog::show_foot / show_bare / modal_titled_foot), how dialog_pattern's table finds every dialog in the sources and holds its order and Enter, and what broke along the way: stale footer records, Escape with no press, keys under a Modal, Modals and bodies past the window, fade-in waits, Tab out of a scrolled body's sight, egui_dock's Tab order."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-20T08:15:49Z }
touches: [crates/mg-ui/src/widgets.rs, crates/mg-ui/tests/app/dialog_pattern.rs, crates/mg-ui/tests/app/dialog_pattern/table.rs, vendor/egui_dock/src/widgets/dock_area/show/leaf.rs, crates/mg-uikit/src/dialog.rs]
asks:
  - How do I add a new dialog so the dialog pattern test accepts it?
  - "every_dialog_is_listed fails with a key like file.rs:fn#2: what is that key?"
  - "A dialog test says Escape closed it but no press was recorded: why?"
  - "Why does Tab reach a floating window's collapse arrow first?"
  - "A dialog's button order or its Enter fails the table: where is the order held?"
  - "Why do Enter and Escape not reach a dialog under the job window or a plugin's question?"
fingerprint: v2-d3eb7618d615dc31
created: 2026-11-20
author: augustaugur
---
## Decided
- Every dialog is drawn through `widgets::Dialog::show_foot` (body and a `Foot`: note or one check at the left, `.main()` first in the accent fill, `.button()`s, `.cancel()`/`.leave()` last), `show_bare` (no buttons of its own) or, for an `egui::Modal`, `modal_titled_foot`. The footer owns Enter (main action while enabled, also from a one-line field) and Escape (Cancel or the leave button). The old `Dialog::show`, `modal_titled`, `always_box`, `escape_here`, `cancel_button` are gone.
- Only egui Modals dim (job window, plugin questions, the talk-table question, Before Saving): dimming was kept as the code had it.
- `tests/app/dialog_pattern.rs` scans `src/` for calls of the constructors (`widgets::dialog`, `modal`, `modal_window`) however they are imported (a `widgets::` path, a bare name its file's `use` imports, across lines or by a glob; an `as` rename fails the scan; `#[cfg(test)] mod` blocks skipped) and for egui's `Window::new`/`Modal::new` outside `widgets.rs:centered` and `modal_window`, and keys each as `file:fn#n`, n counted per function NAME across the file (not reset at each `fn`: same-named fns in one file made duplicate keys). Each key needs an `Opener` in `dialog_pattern/openers_*.rs`; `every_dialog_is_listed` names the missing ones. A shared maker (`files_already(`, `NameWindow {`, `open_picker(`, `loc_edit = Some(`, `var_edit = Some(`) lists its callers in `SHARED_MAKERS`: a new caller fails until named.
- `dialog_pattern/table.rs` holds each dialog's whole button order (`ORDERS`, a button before the main action only with a named exception: `WIZARD`, `PAGE`) and `ENTER_WITHHELD` (the orchestrator's ruling: Enter not taken by a main action that grants trust, overrides a protection, overwrites or quits; `Foot::no_enter`). A failing order prints the line to add.
- One key, one window: `modal_over` (an `egui::Modal` up on another layer) keeps Enter and Escape from every window under it; among windows, the last opened (`Dialogs.open`) takes them.
- Tests read what the footer recorded: `widgets::footers_shown` (last pass only) and `dialog_presses` (records only after its first call arms it).
- egui_dock is patched (fifth patch, `vendor/egui_dock/MOONGLOW.md`) to make the collapse arrow before the tabs, so Tab goes arrow, tabs, close all. Maximize is the app's, registered after the DockArea, so it still comes after.

## Watch out
- A wizard's step list (190 points) beside its page at the least window (420 by 320) squeezed the Creature Wizard's Add Class to a letter a line: `widgets::step_list_fits` leaves the list out unless the page keeps 320 points. Look at the least-window pictures of every wizard after a layout change.
- `footers_shown` across passes kept a closed dialog's record: "closed by Escape" passed falsely until it was filtered to the last pass.
- Dialogs made by `widgets::dialog()` closed by Escape through egui's open flag recorded no press: `dialog()` now notes a ("Close", "Escape") press.
- A Modal's label covers only its title row: the window rect is recorded in `FooterSeen.window`.
- Modals with a fixed `set_max_width` ran past the least window: give them a max width from the room and a scrolled body.
- A contrast measured on a window still fading in reads 1.00: wait for rest (`settle`, or `dialog_pattern::rested` for a job's window, which repaints while it runs: still for four frames), not a count of frames; one kittest step (0.25 s) outlasts egui's fade.
- A body `ScrollArea` scrolling one way only expands to its content the other way: a vertical-only body widened windows past the screen. The body scrolls both ways and words wrap to the room (`widgets::fit_width`, `fit_min_width`).
- `set_max_width(ui.available_width())` inside a window that grows to its content is circular (it shrank dialogs a frame at a time): `fit_width` clamps by `body_room_width`, the room the screen leaves (content width less the body's padding, the window's edges and a scroll bar's allocated width).
- egui's Tab moves the focus but not a scrolled body's view: `widgets::focus_in_sight` (after the content, in the body's and the wizards' page scrolls) scrolls to a control just focused that the view cuts.
- `ctx.read_response(id)` is not a pure read: for a focused widget, in a frame with a pointer press or click that missed it, it surrenders the focus (egui's `get_response`). `focus_in_sight` read it after a click on a project command's Hide box and took the keys from the input line (`projects::a_project_s_command_shows_its_output_and_takes_what_is_typed`): it now skips frames with a press, click or release.
- Scroll bars are not in `WidgetRects::info` outside a debug view: find them as AccessKit `Role::ScrollBar` nodes. AccessKit bounds are unclipped (a scrolled row out of sight still has them); `Node::rect()` panics without bounds.
- `Harness::key_press` steps the key's down and up as two frames and `output()` keeps the last: what the down frame copied (`CopyText`) is lost unless read between `key_down`/`key_up` steps.
- Openers share scratch folders by name: a second test opening the same dialogs at once must call `dialog_pattern::running(tag)` first (the folders get the tag), or the two empty each other's.
- Placing with the palette takes the blueprint out of the hand after one placement: a test placing twice sets `palette.selected` again.
- The area view under a dialog is kept from keys by its own `response.hovered()` gate and from clicks by its response (egui's hit test). A test of it needs something selected for Delete or the arrows to show a leak, and the blueprint taken into the hand again before every click (a placing empties the hand), or it passes whatever the code does.
