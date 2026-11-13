---
title: "Open editors switch, stored places and docked Escape (M3 T10)"
author: August
date: 2026-11-13
branch: ui-redesign
cost: { minutes: 28, tokens: 72812 }
id: l-fb1de37a
scope: repo
status: active
src: agent
touches:
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/tabs.rs
  - crates/mg-ui/src/settings.rs
---

## Decided
- `Moonglow::place_for` returns `Option<Place>`: a kept place first, else `default_place` (None = the focused editor group in As Tabs; a window in As Floating Windows). `Tab::floats` (Tile Properties, model views) stay windows in both modes: Tile Properties is opened by `Moonglow::ui` as tiles are chosen and a tab would cover the area.
- In As Tabs, `remember_window_places` keeps a docked place only for a tab in `moved_by_hand` (egui_dock's dragged tab, Split Editor): a place is measured from `middle_pane`, which moves when the area is split off or a group closes, and that stored `script: split left 500` with no move of the script's own.
- Docked Escape (ruling R1) is seen in `tabs::Viewer::body` (`escape_docked_seen`), kept after the DockArea only if no floating window took it (`escape_taken`) and `key_pressed(Escape)` still holds, then acted on the next frame (`escape_docked`). Where a widget had the keyboard as the last frame ended (`keyboard_had`, a bool read at `dialogs_end`), the Escape is the tab the last press (`last_press`) was in, wherever the pointer is (a field typed in another group must not cancel the tab under the pointer); otherwise the tab under the pointer, or the last pressed with the pointer on no other group or window.

## Tried and ruled out
- `egui::Context::read_response(focused_id)` at the frame's end to know where the keyboard was: it changed the next frame's widgets (24 app tests failed, scripts' Replace did nothing). Use the last press instead.
- The dock's focused leaf as "keyboard in it": a click in the palette's prefab rename field leaves Module Properties' group focused, and its Escape would cancel Module Properties.

## Watch out
- Module Properties is docked at module open in both modes, so R1 makes Escape in its fields its Cancel for every user: tests that left a description field by Escape now click away (`fields.rs` `leave_field`).
- Options dropdowns sit below the new Editors section: tests scroll them into sight before clicking.
