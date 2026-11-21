---
type: Decision
title: "Properties panel (M4 U6): edit_many's body in the Right Dock, the lower section by tool, Open in Tab, the column seam"
description: "How the area's Properties panel works since U6: blueprint::form_ui shared with the window, brought by brings_of per tool, ShowProperties for a double click, the panel leaves fields to an open window, held_text.panel, DockLayout::column for the wide seam."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-15T15:05:03Z }
touches: [crates/mg-ui/src/properties_panel.rs, "crates/mg-ui/src/blueprint/mod.rs:form_ui", "crates/mg-ui/src/shell.rs:brings_of", "crates/mg-ui/src/shell.rs:first_column", crates/mg-uikit/src/pages.rs, crates/mg-ui/tests/app/properties_panel.rs]
asks:
  - Why does a double click on an object not open the Properties window any more?
  - "Where are a placed object's properties drawn in the Right Dock?"
  - Why does the Properties panel say the properties are open in a tab?
  - "How is the wide Right Dock's column seam kept?"
fingerprint: v2-cb08bfae738ca12d
created: 2026-11-15
---
## Decided
- `Panel::Properties` (Right Dock, `Keys::Properties`, in `KEYED`), body `properties_panel::panel`. `brings_of` for `Tab::Area` (module open): Select Tiles with Tile Properties shown → `[[Palette, Preview], [TileProperties]]` (folded Palette, Preview, Tile Properties); Select Tiles with the toggle off → palettes alone; any other tool (Select, Paint, Terrain) → `[[Palette, Preview], [Properties]]`, folded Palette, Properties, Preview (Narrow), `shares` 1160:840 (Main's 58/42, `Brings::shares`), `window_right` Properties alone (a dragged-out area's window). NoArea → palettes alone.
- The body is `blueprint::form_ui` (the old `edit_many` split: `edit_many` keeps OK/Cancel, the page row and `window_drawn`; `form_ui` is the notes, the page in its ScrollArea, refusal and several-drop, returning `pending`). The panel never calls `window_drawn`: no Cancel record, no CloseTab. Its typed text is marked `HeldText::panel` so a window drawn later that frame does not claim it for its Cancel.
- While any Instance/Instances tab is open on one of the selected objects the panel draws a note and "Show It" instead of the form (no duplicated ids, buffers or Cancel confusion). Open in Tab copies the panel's page into `blueprint_pages`.
- Pages: one at a time (Main board; WideArea draws one scroll, U9's job), kept per ResType in `Moonglow::panel_pages`; the rail is the kit's `pages::PageRail` (scrolls where short; a ScrollArea needs `min_scrolled_height(0)` and padding as allocated space, or it stops short).
- Double click (Q1): `Action::ShowProperties(area)` (pub), run after the frame: during the editor area's drawing `active_editor()` is None (the dock is out of `app`). Since U6's review it puts the double-clicked object in the selection (Ctrl too); the module tree's and Outliner's row double click shows the panel too (tree::Asked::ShowProperties).
- Wide seam: `DockLayout::column` (first column's width, per kind, skip-serialized), `shell::first_column`, `column_seam` after the sections' widgets; both columns ≥ 280.
- Narrow palette row: Standard/Custom beside the types' dropdown, name row gone (kept for tiles' tileset), Group by row always drawn there (Category alone, grayed with no groupings) with Expand/Collapse at its end.
## Watch out
- Section headers named "Properties, Placeable": group nodes have no rect in kittest; measure the rail's buttons ("Pages" › "Basic").
- `panel_body` takes `(panel, titled)`; Properties has zero margin (rail flush) and draws its own detail + Open in Tab where it shares the one bar (<1400).
- The window guard (`open_on`) reads `app.dock` AND `app.panes_tabs`: a dragged-out area's pane draws while the dock is taken out of `Moonglow` (empty), and without panes_tabs the panel drew the form beside its own window (egui id-clash assert).
- Under 1400 the one bar's front is kept separately (`DockLayout::folded`, written by `keep_front` when `yields.right_dock_narrow`), read first by `right_shown` there: the per-section fronts cannot say which section was chosen last.
- A creature's Inventory page stacks palette over slots under 440 (`creature_lists::INVENTORY_SIDE_BY_SIDE`); side by side at the panel's 320 the slots overlapped.
- A held area's fields are protected by four layers (read_only_if, typing_kept_out, look_only, claims::gate): a test break must remove all four.
