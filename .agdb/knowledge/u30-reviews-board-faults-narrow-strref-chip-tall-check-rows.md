---
type: Gotcha
title: "U30 review's board faults: narrow StrRef chip, tall check rows, a board case scrolling, blank pictures"
description: "What the U30 review's board fixes found: why a check box in add_enabled_ui made tall grid rows, why NUI's Properties picture opened scrolled, how a wholly see-through model became an empty picture, and test-harness traps met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T06:10:24Z }
touches: ["crates/mg-ui/src/blueprint/mod.rs:locstring_then", crates/mg-ui/src/blueprint/placeable.rs, "crates/mg-ui/src/model_view.rs:render_picture", crates/mg-ui/tests/screens.rs, crates/mg-ui/tests/app/dialog_pattern.rs]
asks:
  - "Why are a placeable's Useable and Has inventory rows far apart?"
  - Why does the board_nui picture show Properties scrolled down?
  - "Why is Null Human's gallery cell empty?"
  - Why does a picture test pass with the check broken?
fingerprint: v2-7fd48f3d3dcabbe1
created: 2026-11-21
author: augustaugur
---
## Found (U30's review, 2026-11-21)
- A check box wrapped in `ui.add_enabled_ui` inside a Grid cell made the row 47 to 51 px tall (the scope's own layout). Use `ui.add_enabled(enabled, widget)`: `Form::check_if` in blueprint/mod.rs.
- Narrow forms: a name field with its StrRef chip inside the field cut the name ("Plague Vi"). In a narrow form the field takes the whole width and the chip goes under it beside "…" (`Form::locstring_then`, `narrow_locstring`); the wide form is unchanged. Extra buttons (the creature's random name) pass through `after`, not a nested `row`.
- board_nui's Properties opened scrolled down because the screens case `board_nui_as_drawn` called `scroll_to_me` on the bound chip (added in U19's review), not because of the app. A chip's click leaves Properties at their top (`nui_panels::properties_open_at_their_top_on_a_chip_s_click`).
- Null Human (c_invsguy) has 17 trimeshes all `alpha 0`; the renderer skips wholly see-through meshes, so the thumbnail was an empty frame kept as a picture. `model_view::render_picture` returns Ok(None) when nothing was drawn (no meshes, no game meshes, no particles): the cell says "No picture".
## Test traps
- `blueprint_harness` has no GPU: every picture is Missing there, so a picture test passes with its check broken. Use `area_harness` (skips without an install or GPU).
- `dialog_pattern::every_dialog_is_listed` finds dialogs by a source scan for a one-line `loc_edit = Some(` keyed by function name; a multi-line assignment or a renamed caller breaks it (its SHARED_MAKERS list names callers).
