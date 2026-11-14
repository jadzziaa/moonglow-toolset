---
type: Decision
title: "The Outliner under the Left Dock (M3 T13): one contents_ui for tree and Outliner, the upper panel always nested"
description: "How the area's Outliner shares the module tree's per-area code, why the Left Dock's upper panel is always a Panel::top, and the test traps met building it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T01:23:50Z }
touches: [crates/mg-ui/src/outliner.rs, "crates/mg-ui/src/tree.rs:contents_ui", "crates/mg-ui/src/shell.rs:left_dock", "crates/mg-uikit/src/shell.rs:PanelHeader", crates/mg-ui/tests/app/outliner.rs]
asks:
  - "Where is the Outliner drawn and how does it reuse the module tree's rows?"
  - "The module tree's groups closed and its scroll jumped when an area tab closed: why?"
  - "Rows drawn under a PanelHeader overlap its bar: where does that come from?"
  - "A UI test's click on an Outliner row lands on another row: what to wait for?"
fingerprint: v2-30c54e5717dda102
created: 2026-11-14
---
## Decided
- One copy of the per-area code: `tree::contents_ui` takes a `Look` (`Tree { hide_empty }` or `Outliner { shown, fold }`); the Outliner (`outliner.rs`) passes its own state (cursor, reveal, anchor) and shares `Moonglow::area_contents` through `tree::fresh_contents` (read again at once where the GIT's `Read` changed). What a row asks goes through `tree::act_on_object`; the keys through `area_key_rows`, `step_in`, `seek_in`, `past_deleted_in`, `typed_row`. A plain click in the Outliner is `Asked::Select` (camera stays); Enter is `Go`.
- The Left Dock's upper panel (tabs + Module/Resources) is ALWAYS drawn in `Panel::top("module-panel")`, resizable only when the Outliner shows (`exact_size(whole)` without it). Nesting changes egui ids: drawing it nested only beside an area made the tree's CollapsingState and scroll ids differ as the Outliner came and went (an area closed: kinds closed, the tree's well scrolled).
- The eye is one state with the area toolbar's Show toggle: `area_view::show_kind` goes through the toolbar's `share` (every view, `hidden_in_areas`, selection retained). Named "Show <Kinds>" like the toolbar's, found in the "Outliner" group.
- Outliner rows are named apart from the tree's rows beside them: kinds "46 creatures", objects "Name, tag TAG" / "Name, no tag".

## Watch out
- `mg_uikit::shell::PanelHeader` used `scope_builder(max_rect(bar))` after allocating the bar: a scope moves the parent's cursor to its own end, back into the bar, so the next widgets overlapped it by 9 points. It now uses `new_child`.
- Selection reveal scrolls over several frames: a test that reads a row's rect and then clicks must wait for the Outliner to rest (`rest` in tests/app/outliner.rs), else the click lands on another row.
- egui puts a Label's text in accesskit `value`, not `label`; a tree row's selected state is `toggled`, not `is_selected`.
- A row out of a scroll area's clip is still in the accesskit tree in some lists but is not laid out in the virtualized ones; tests of the module tree with an area in front needed the tree's old height (`common::tall_tree`).
- A selection the Outliner makes itself (row click, Ctrl/Shift, Enter) is noted as seen (`made_here`) so it is not revealed: revealing it recentered a row cut by the foot under the pointer, and a Ctrl+click scrolled to `selection.last()` (the BTreeSet's largest), away from the row clicked.
- The Outliner's keys block runs without a cursor too: an area with nothing placed has no row, and `a_list_has_the_keys` (no Esc hint) must match who takes Escape.
- The keys' cursor follows the view's selection only when something is selected: after a Delete clears it, the cursor must stay where `past_deleted_in` put it.
