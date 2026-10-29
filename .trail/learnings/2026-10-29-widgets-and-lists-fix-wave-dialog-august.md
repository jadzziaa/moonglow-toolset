---
title: "Widgets and lists fix wave: dialog Escape, menu keys, icons only, pane keys"
author: August
date: 2026-10-29
branch: worktree-agent-a41ec740c9e45dff5
cost: { minutes: 924, tokens: 7048998 }
touches:
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/palette_view.rs
  - crates/mg-ui/src/table_view.rs
  - crates/mg-ui/tests/app/palettes.rs
  - crates/mg-ui/tests/app/tables.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/05-blueprints.md
  - crates/mg-ui/src/keys.rs
---

## Decided
- A dialog's Escape (`widgets::escape_on`, every `dialog`/`open_unless_escape` window) stays open while it clears a focused search field's text: `search_field_shown(ctx, id, typed)` records each SearchField with whether text was in it after it was drawn; `dialogs_end` keeps `searching_text` for the focused one; `dialogs_begin` sets `clears_search`. Properties windows keep their stricter rule through `escape_left_search` (clear, leave, then Cancel). The Command Palette (egui Modal) and a long dropdown's Filter (egui Popup) close on `FilterLeft::Escape` with `ui.close()`: both read Escape after their contents, and the field consumed it.
- `Dropdown::show_menu` gives lists of commands and ticks the keys: its state (`MenuState`: the box's id from the last frame, its rows when last open, a cursor) is kept under `make_persistent_id(("menu-keys", Id::new(&salt)))`, since the combo's own id is known only after `show_ui`. Rows go through `MenuKeys::row(&response)` (highlight at the cursor, true on Enter/Space). `Dropdown::show_with` reads its lazy options only on an arrow key press; `hold_arrows` does the focus, claim and lock filter every frame.
- `icons::Tools::named` gives a button its name for accesskit with Icons only (`Response::widget_info` overwrites the label; toggles `WidgetInfo::selected` → accesskit `toggled`, not `is_selected`). The editors' toolbars use `Tools::word` (words alone, or the glyph): their labels and tests stay as they were with Icons and text.
- Expand All / Collapse All keys live in a new `keys::Group::Panes`: a General `Cmd` must also be a `commands::Id` (`the_keys_that_work_anywhere_are_all_commands`).

## Watch out
- egui's drag starts only past 6 points (`max_click_dist`): a test drag of six steps under that per step loses its first step. `alt_and_a_drag_off_the_selection_moves_the_camera` passed or failed as the area toolbar wrapped (the view's size changed the points per meter).
- egui's Escape drops every widget's focus in `begin_pass`: after closing a list with Escape the box no longer has the keys (a test opens and closes it with two clicks instead).
- `Settings` restored in the palette's first frame overwrote a filter a test had set: restore the search only where the settings have one.
- A palette's categories have no accesskit expanded state from `CollapsingHeader`; a test tells open by rows drawn under the header (not too far below: the log's buttons are there too).
- Glyphs not in egui's fonts (checked by `uv_rect` against ◻): 🖨 ✍ ⇄ ＋ ⊕ ⊖ ⏯ ⬚ ⧉ 🧲 🗂 🧪 🗃 🗒 ▼ ▲ ⇔ 🧩 🖇 🛈 ⓘ 🖫 🖪 🎚 🎛 🧭 🧰 🧱 🗜.
