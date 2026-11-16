---
type: Decision
title: "A blueprint's own Preview panel (M4 U11): brings, palette_dock, widths, framing, records"
description: "How a form's tab brings its own Preview, when a floating form shows its pane (972 rule), the test switch that keeps it away, the wide-dock width per kind, how Frame fits, and the test gotchas found."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T13:02:20Z }
touches: ["crates/mg-ui/src/shell.rs:brings_of", "crates/mg-ui/src/shell.rs:window_editor_least", "crates/mg-ui/src/area_tools.rs:own_preview_body", "crates/mg-ui/src/model_view.rs:fitting_distance", "crates/mg-ui/src/lib.rs:open_window"]
asks:
  - "where does a blueprint's model show now that the Appearance page has none"
  - why has my floating blueprint window no Preview pane
  - why does the Right Dock lose its Right Dock name beside a blueprint
  - "why does header::records still list a header after its tab closed"
fingerprint: v2-7c4f3143ee474a52
created: 2026-11-16
---
- A form's tab (Blueprint, Blueprints, Instance, Instances) whose object has a model brings `[[Preview]]` with `floats`; `shell::own_preview(app, editor)` names its source; `panel_body` draws it in place (not the late slot) because the form no longer draws the model. Pop Out opens the model's tab; while open the panel says POPPED_OUT with Bring Back. Deferred 362 still covers the palette's Preview (late) vs a floating blueprint window's own pane, and two forms of one blueprint.
- Ruling R1 (U11 review): a floating form shows its pane only where the form keeps 972 beside it (`WINDOW_FORM_LEAST`, `window_editor_least`); a new window asks for 980 + 281 only where a pane will be drawn (`Moonglow::open_window`), so at 1280-1920 screens (window held to 85% of the main pane) or at a remembered 980 there is no pane and the Preview button opens the model's tab. Floating-pane tests need a 2200-2600 wide harness and `settings.window_sizes` cleared.
- `Moonglow::palette_dock = false` (the tests' switch) keeps a form's own Preview away too; many layout tests rely on it.
- Wide layout: a dock of one panel alone is 549 (`RIGHT_DOCK_WIDE_ONE`; 560 gives four columns of 480 at 3440), its dragged width kept per kind in `DockLayout::wide_width`.
- Frame fits `Composed::reached_in` (per-mesh box corners, skinned vertices) projected across and up apart (`fitting_distance`), and reframes on an aspect change while `framed` (cleared by a pan or zoom). The one box round everything was far too wide for a tall narrow Preview.
- The header's More order: `model_view::show` forces Export Files… then Look into More by measured room, because the kit yields the left group first.
- The single-bar Right Dock draws a `PanelHeader` only for a lone own Preview (named "Preview, <info>"); the group named "Right Dock" is then absent. Find dock contents by `right_dock_rect`.
- `mg_uikit::header::records` keeps the last pass that drew a header: to check a header is gone, query the a11y group by `MODEL_HEADER`.
- The kit's PanelHeader draws no detail before its tools are measured (pass redrawn); test with `max_passes = 1` to see a first frame.
