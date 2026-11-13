---
title: "Dialogs' title rows and floating windows' bars (M3 T11)"
author: August
date: 2026-11-13
branch: ui-redesign
cost: { minutes: 30, tokens: 43960 }
id: l-7ec449c2
scope: repo
status: active
src: agent
touches:
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/src/tabs.rs
  - crates/mg-ui/src/settings.rs
  - vendor/egui_dock/MOONGLOW.md
---

## Decided
- Dialogs draw their own title row (`widgets::Dialog`, `mg_uikit::title_row`): egui's title bar off, the frame's margin moved inside, the row's room kept first and the row painted after the body across the width the body made. The boards decide where they differ from the plan: dialogs open in the middle of the main window (`widgets::dialog_center`; `src/Wizard.html` puts the Area Wizard at the window's middle, not the editor area's); a Close only on dialogs Escape closes (`widgets::dialog`) and on the two pickers (`Dialog::close_as_cancel`, read by `cancel_button` the next frame).
- A floating window's collapse arrow and close all are painted and named by Moonglow through the vendored egui_dock's third addition, `TabViewer::window_bar_button`; the close all is named "Close Window" ("Close Pane" in a split window), not "Close", because dialogs' Close and tabs' own Close buttons can be in sight with it (and NUI's Insert › Close Window).
- Remembered window sizes from before the theme are scaled once by `Tab::taller_of_kind` behind `Settings::window_sizes_scaled` (false when absent), run by `Moonglow::with_settings`; the desktop app saves the marker with the sizes because its `saved_over` compares with the settings as read, before the migration.

## Tried and ruled out
- Keeping a dialog's row at least as wide as last frame's body for every dialog: a dialog resized by its edges could no longer be made narrower. egui does that only for windows that grow to their content; `titled`'s `grows` follows `Dialog`'s `resizes`.
- Leaving the row at its title's width: egui's own title bar kept a dialog at least as wide as `egui::Resize`'s first 320 points, and without it a dialog with a grid (Rotate Area) opened a frame narrower, moved about its middle, and a test's click on OK landed on Cancel (`DIALOG_LEAST_BODY`).

## Watch out
- `ui.interact` with `Sense::drag` behind the body, from last frame's body rect, keeps a dialog from moving by its body (egui moves a window with no title bar by any part); registered after the body it would steal text fields' drags.
- egui's resize corner is drawn in the window frame's stroke color only; Moonglow sets `resize_corner_size` to 0 after the theme and paints the boards' grip itself.
- Under load a dialog's Close (laid out after the body) was missing from one accesskit frame of the Select Portrait picker; tests wait for it with a deadline.
- `scratch_dir(name)` empties the test-output folder of that name: `dialog_windows`' light run empties the dark pictures' `screens-dialogs`; copy them between runs.
