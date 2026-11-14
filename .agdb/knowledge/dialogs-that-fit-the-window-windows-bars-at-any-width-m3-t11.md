---
type: Decision
title: "Dialogs that fit the window, windows' bars at any width (M3 T11 review)"
description: "How widgets::Dialog keeps a dialog inside the main window (scroll area, fold arrow via title_row_with) and how floating windows' tabs and least width (window_end_room, window_least_width) are kept at any width."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T20:58:08Z }
touches: [crates/mg-ui/src/widgets.rs, crates/mg-ui/src/tabs.rs, vendor/egui_dock/src/dock_state/window_state.rs, vendor/egui_dock/MOONGLOW.md]
asks:
  - "Why does a folded dialog still show a fold arrow at its title row's left?"
  - How does a Dialog stay no taller or wider than the main window?
  - "Why is egui_dock's Shift mode turned off with secondary_button_on_modifier(false)?"
  - "Why does Window::min_width after fixed_size undo the fixed size?"
fingerprint: v2-a40e9be2a4a2ea4c
created: 2026-11-13
---
## Decided
- A dialog is never taller or wider than the main window: `widgets::Dialog` keeps the size its body took last frame and, past the room under its title row, shows the body in a scroll area of that room; egui then keeps the window inside. A dialog that egui folded keeps a fold arrow at its title row's left (`mg_uikit::title_row_with`'s `fold`).
- A floating window's tabs end before Maximize (the vendored egui_dock's `DockArea::window_end_room`) and a window is at least `tabs::window_least_width()` (211 points) wide (`window_min_width`). Moonglow turns egui_dock's Shift mode off (`secondary_button_on_modifier(false)`): in it egui_dock's own close all drew a "close window" symbol and a click did nothing.

## Tried and ruled out
- Folding a dialog with `Window::fixed_size` or min/max heights: egui sized the folded window to its content anyway, an empty body's margin included (50 points). The folded row is laid out at the width kept and the body skipped.

## Watch out
- In egui 0.36 `Sense::drag()` and `Sense::click()` are focusable: a catcher behind a body made with them is an unnamed stop of Tab. Use `Sense::DRAG` / `Sense::CLICK`.
- `egui::Window::min_width` after `fixed_size` undoes the fixed size (a NUI window put at 1100 opened at 720): set the least first.
- A test's break restored with the backup's old mtime leaves cargo's broken build in place: touch the file after restoring.
