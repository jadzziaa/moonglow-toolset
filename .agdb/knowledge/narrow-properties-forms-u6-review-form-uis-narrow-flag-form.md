---
type: Decision
title: "Narrow Properties forms (U6 review): form_ui's narrow flag, form_grid, row, held widths"
description: "How a Properties form fits the 280-400 panel since U6's review: NARROW_FORM flag set by form_ui, FormGrid halves with wrapping, row() wraps inside a vertical (grid rows), wrap_for/held_width for combos, all inactive outside narrow forms."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-15T17:34:23Z }
touches: ["crates/mg-ui/src/blueprint/mod.rs:form_grid", "crates/mg-ui/src/blueprint/mod.rs:row", "crates/mg-ui/src/widgets.rs:wrap_for", crates/mg-ui/tests/app/properties_panel.rs]
asks:
  - Why do Properties panel fields overflow the dock?
  - How do I make a blueprint page fit the narrow Properties panel?
  - Why does a wrapped row in a grid cell overlap the next row?
  - Why does a combo box not wrap in a horizontal_wrapped row?
fingerprint: v2-f9fcfe61696d354f
created: 2026-11-15
---
## Decided
- `blueprint::form_ui` sets `NARROW_ID` temp data = (its room < `NARROW_FORM` 400) for the form's drawing and removes it after; `blueprint::narrow(ui)` reads it (false outside forms: Area/Module Properties, wizards keep their look). Never decide narrow by local width inside a grid cell: a window's cell is < 400 too.
- Two-column grids: `form_grid(ui, id)` → `FormGrid` (narrow: `max_col_width(w/2)` + scoped `TextWrapMode::Wrap`; with a finite max egui wraps non-last columns at that width, safe; with infinite max a Wrap label wraps at the column's first-frame width and stays crushed).
- `row()` narrow = `ui.vertical(|ui| ui.horizontal_wrapped(add))`: a wrapping row placed alone in a grid cell does not grow the grid row; its second line was drawn over by the next row.
- egui never wraps a ComboBox in a wrapping row: `widgets::wrap_for(ui, width)` end_rows first; `held_width`/`fit` clamp widths to the room — all only in narrow forms (outside, clamping to a grid column's first width oscillated → "Harness::run exceeded max_steps").
- Multi-column page tables get explicit narrow layouts (abilities/saves 3 cols, classes level under, replacements pair on two lines, store/feats lists stacked via `beside_or_over`/`part_width`/`down_or_row` with a narrow bool asked BEFORE the parts — an overflowing widget widens max_rect for everything after it).
- Panel form ids: `Moonglow::panel_form` salts `window_state_id`; ids follow objects via `follow::window_id`, and `follow::retarget` keeps ids of selected objects.
## Testing
- `properties_panel::every_page_of_every_kind_fits_the_panel` (MG_PAGE_SHOTS=1 saves each page; MG_LIGHT for light) is the sweep; contact sheets via `target/u6/sheets.py`.
