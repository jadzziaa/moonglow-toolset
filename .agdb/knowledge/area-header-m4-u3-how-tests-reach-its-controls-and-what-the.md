---
type: Decision
title: "Area header (M4 U3): how tests reach its controls, and what the toolbar's removal moved"
description: "The area's toolbar became the kit header 'Area header' in U3: where each control lives by width, how common::area_tool/area_click reach popover rows, the Escape wiring through Popover::any_open, and the pitfalls (accesskit clicks blocked under a window with Cancel, view height moving camera-dependent tests)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-15T05:19:43Z }
touches: [crates/mg-ui/src/area_view/header.rs, "crates/mg-ui/tests/app/common.rs:area_tool", crates/mg-uikit/src/header.rs, crates/mg-uikit/src/popover.rs]
asks:
  - How do I click an area toolbar button in a UI test now?
  - Why does a test that clicks the area header do nothing while a floating window is open?
  - "Where did Select Tiles, Paint, Reorient Camera and Go to Start Location go?"
  - Why did a tiles or sky test break when the area header changed height?
fingerprint: v2-792646731f22291d
created: 2026-11-15
---
- The area's controls are the kit `EditorHeader` named **"Area header"** (`area_view/header.rs`). Yields come from `Yields::of(window, ui.available_width())` — the header row's own width, not `app.yields`. Compact (<990): Show popover ("Show N of 10" via `Item::shown_words`, rows named "Show Creatures"…), moon toggle "Night", popovers `icon_only`. Full: joined group "Show" (its parts are the nodes; the group has none) + custom "Show All or None" arrow (egui menu). <550 Snap/Turn compact whatever the room; <490 Together/Onto Objects `in_more`. Wide: popovers unfold (kit re-unfolds early-folded ones from the left).
- Until U4: **Select Tiles, Paint, Reorient Camera, Go to Start Location are `in_more`** (boards put them on the shelf/compass). U4 removes them from the header.
- Tests: `common::area_tool(&mut h, name)` opens what holds a control (by `header::records` + the `AREA_HOLDERS` table: Show, Show All or None, Lighting, Overlays, Audio, and the wide row's held Shadows/Walkmesh menus) and returns the node; `area_click` clicks and closes (`close_area_popover`); `area_tool_shown` reads without opening. A popover left open blocks keys (`keys_blocked` asks `Popover::any_open`), so prefer `area_click`.
- **Accesskit clicks don't reach the area header while a floating window with Cancel is open** (and that window may cover it): click by pointer (`press`) on a header part clear of the window (More at the right end).
- Renames: "Lighting" toggle → switch "Use the area's lighting" (button "Lighting" is the popover); "Shadows" → rows "Sun and moon shadows"/"Creature shadows"/"Light shadows" (wide row: "Shadows" toggle); "Walkmesh…" → "Ground walkmesh"/"Object walkmeshes" (wide: "Walkmesh"); sentence case "Tile border", "Sound ranges". `hidden_area_tools` keeps the old 29 names.
- The kit header used to leave the parent's cursor 8 px inside its row (fixed with `advance_cursor_after_rect`); the area view is now taller than under the wrapped toolbar, which moved camera-geometry tests: the sky test keeps a 782-high window; a tiles test's ray ran exactly down the tiles' diagonal (their ground faces' split) and is now taken from 130°.
- Overlays switch = `AreaView::hide_overlays`; read overlays through `AreaView::overlay(on)`.
