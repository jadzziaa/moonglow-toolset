---
title: "Module tree keys at placed objects, one filter matcher and search field"
author: August
date: 2026-10-27
branch: tree-keys-and-filters
cost: { minutes: 708, tokens: 8090952 }
touches:
  - crates/mg-ui/src/tree.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/src/keys.rs
  - crates/mg-core/src/text.rs
  - crates/mg-ui/src/palette_view.rs
  - crates/mg-ui/src/terrain_mode.rs
  - crates/mg-ui/src/browser.rs
  - crates/mg-ui/src/commands.rs
  - crates/mg-module/src/areas.rs
  - crates/mg-ui/tests/app.rs
  - docs/manual/02-main-window.md
  - docs/deferred.md
---

## Decided
- One matcher: `mg_core::text::Needle` holds the folded words; `finds`/`finds_folded` need every word in one text, `finds_in(&[fields])` every word in one of a row's fields (each folded by the caller), `letters_in_order()` is the palettes' fallback alone. Changing `Needle` reached every user at once (~25: 2DA filter, wizards, Open Script, dropdown filters, `mg areas --match`); lists that ORed `finds` per field were moved to `finds_in` (tree, palettes, picker, tileset editor, 2DA rows, Area chooser, Options › Keyboard, `mg_module::areas::Filter`). Hand copies gone: the palettes' `Find`, the Tiles tab's, the hak editor's, the browser's, the picker's, item properties', the Command Palette's.
- One field: `widgets::SearchField` (hint, ×, `.find_when(bool)` or `.find_over()` for Find's keys, `.down()` for lists with keys of their own, `.width`). Find's keys come from the keymap's `Cmd::Find` (grouped under Script Editor), shared each frame through ctx temp data (`Keymap::share_find` in `Moonglow::shortcuts`, read by `keys::find_pressed`): `Form`, `table_view::header` and the NUI creator have no `Keymap` at hand.
- Module tree: `seek` goes by `key_rows(shown, outs)`, the rows in sight, texts from `row_text` (kind label without its count, an object's `ObjectRow.text`, `(no name)`). Down from the Filter sets `tree_to_match`, resolved in `tree_rows` after the rows are laid out (in `module_tree` they don't exist yet); first `Resource` found by its own name/ResRef (`matched`) or any `Object`. Delete at an object = `Asked::Delete`; `past_deleted` moves the cursor to the next sibling row (indices after it shift down), and `Asked::Delete` clears `tree_object`, else `follow_selection` wiped the cursor when the selection was cleared. F2 at an object does nothing (no Rename in our menu nor Aurora's object menu).

## Tried and ruled out
- Holding Down back from `filter_keys` with an event-stash hack: a `down` flag on the field's keys (`field_keys`) is all it takes.
- Driving Aurora under Wine for F2 on a placed object: not needed for the task's fallback (menu has no Rename); left in deferred.md.

## Watch out
- Fields moved off `pane_filter_keys` keep Down in the field now (they had surrendered focus with nothing taking it).
- `hak_editor_and_attach` in screens.rs fails at 48f54c6b too (`get_by_label("Custom Content")`, line 2393): not this work's.
- Before/after pictures: build the base with `git checkout BASE -- crates/<changed src>` in the worktree, render, then `git checkout HEAD -- crates`; only mg-ui and mg-core rebuild.
- `flock` with cargo must go in a scratchpad script (`sh locked.sh cargo test ...`); heredocs and sed with variables are refused in a worktree session: use Edit/Write.
