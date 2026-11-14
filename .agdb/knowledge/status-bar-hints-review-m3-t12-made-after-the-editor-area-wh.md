---
type: Gotcha
title: "Status Bar hints review (M3 T12): made after the editor area, what blocks the view's keys, testing Esc guards alone"
description: "Why the Status Bar's content is made once a frame after the editor area, which states hide the area's hints, how the Esc and Enter hints exclude lists and model windows, and how to build a test that exercises one Esc guard alone."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T23:50:50Z }
touches: ["crates/mg-ui/src/shell.rs:update_shown", "crates/mg-ui/src/shell.rs:keys_blocked", "crates/mg-ui/src/tabs.rs:Viewer::foresee_escape", "crates/mg-ui/src/palette_view.rs:a_list_has_the_keys", crates/mg-ui/tests/app/open_editors.rs]
asks:
  - Why does the Status Bar show what the area did one frame late?
  - "The bar hints Esc or Enter but pressing it does something else: where to look?"
  - Why do area hints disappear while a menu or dialog is open?
  - "My Esc-guard test passes even with the guard removed: why?"
fingerprint: v2-19f7b43c129e2081
created: 2026-11-13
---
## Decided
- What the Status Bar shows is made once a frame by `shell::update_shown`, after the editor area, and drawn by the next frame's bar (the bar is drawn first). A repaint is asked for where it changed. `Moonglow::status_said` returns what was made; tests must wait until the bar's nodes are drawn, not only until it is said.
- The area's hints are cleared (counts kept) while `shell::keys_blocked`: an egui Modal (job window), a dialog (`widgets::dialog_open`) or any popup or menu. Measured: each of the three kept Q from turning a selection with the pointer over the view.
- Esc and Enter hints exclude `palette_view::a_list_has_the_keys` (tree, Resources, log take Escape, and the tree Enter, before the editor area). A floating model window under the pointer takes Escape whatever a docked tab saw (`body`'s `model`), so it sets `model_under_pointer_next` and the docked hint is dropped.
- The old hint line read the selected object from `view.model`, so it was never live during a drag; the drag readout now reads `view.posed`.

## Watch out
- A test of one Esc guard must keep the others quiet: clicking a menu title moves the last press and the pointer away from the tab, and a dropdown takes egui focus, so both hide the hint for other reasons. Use the tab's own context menu (right click on its tab, then hover its body), and assert `focused()` is None.
- egui time in the harness advances by its step, not by wall time: a four-second state needs frames, not sleeps.
