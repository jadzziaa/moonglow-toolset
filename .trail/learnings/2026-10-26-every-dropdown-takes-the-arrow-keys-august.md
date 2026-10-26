---
title: "Every dropdown takes the arrow keys, filters fold accents"
author: August
date: 2026-10-26
branch: lists-rest
cost: { minutes: 58, tokens: 46480 }
touches:
  - crates/mg-module/src/query.rs
  - crates/mg-ui/src/area_tools.rs
  - crates/mg-ui/src/model_view.rs
  - crates/mg-ui/src/widgets.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/03-modules.md
  - docs/manual/04-areas.md
  - docs/manual/05-blueprints.md
---

## Decided
- Every dropdown goes through `widgets::dropdown(id)` (or `dropdown_labelled(id, label)`, the model view's): a builder over `egui::ComboBox` with `selected_text`, `width`, `height`, `stays_open`; `show(ui, &mut current, &options, text)` (selectable_value rows), `show_with(ui, &mut current, || options, |ui, current| rows)` (own rows; the options closure runs only while the box was clicked or has the focus, for lists read only while used), `show_ui` for what is not a choice (Add hak…, Setup Area Transition…, the 2DA editor's flags). `tests/one_gate.rs::every_dropdown_goes_through_the_widgets` fails on any line with `ComboBox` outside widgets.rs. 61 boxes moved (57 outside widgets.rs, 4 in it); 3 take no arrows.
- Callers that pushed an action from a click inside the popup now keep a local `pick` copy, choose into it, and push after the box when `pick != current` (one path for click and keys). Where the options and the rows both need `&mut app` (the speaker tags), `show_ui` plus an explicit `arrow_step` after.
- Lighting Scheme grid: `widgets::grid_arrow_pick(ui, r, at, count, down)`: Up/Down ±1, Left/Right ±down (the grid fills a column at a time), Home/End; focus is moved to the new label the next frame (`schemes-focus` temp id), as the label ids are positional.
- `arrow_pick`/`grid_arrow_pick` record the focused list's id (`claim_arrows`); `widgets::choice_has_keys(ctx)` = that id is `memory.focused()` now. The conversation tree's key gate checks it (the tree takes the arrows whenever no text field has the keyboard, and surrenders other focus).
- Name filters: `mg_core::text::Needle::new(typed)` (folded and trimmed once) and `.finds(text)`; or `fold` on both sides where a folded field is cached (palette `fields`, the tree's `sought`). `fold` has an ASCII fast path. Content search (`mg_module::text::matches`, `script_tools::find`) compares char by char with `same_letter` when Match case is off, so found ranges keep the text's length; script find now scans chars (non-overlapping from the start, as Replace All) instead of lowercasing the haystack (whose length change fell back to exact case).
- Latin Extended Additional folds by position (`extended_additional`: runs of capital/small pairs by letter; checked against Python's unicodedata NFD for all of U+1E00..1EFF); Extended-B and Greek tonos by hand table.

## Tried and ruled out
- A second "in use" flag mechanism (as `RowChoice::in_use`) for lazily read options: one mechanism, the options closure called only when clicked or focused, is enough.

## Watch out
- egui's `set_focus_lock_filter` only takes effect from the second frame a widget has focus (`had_focus_last_frame`); an arrow pressed the frame after a click can move egui's focus instead.
- A dialog just opened is still laying out for a few frames: kittest's `click()` hits the node's centre from the last frame, so a test clicking a combo right after opening a window may hit the one below (`run_steps(6)` after opening).
- Tab from the app's start walks the menu, toolbar and module tree first: in a test, `node.focus()` a widget near the target and Tab from there.
- `LocString::from_text` takes bytes as they are (UTF-8 then reads as cp1252 mojibake); a test's accented name needs `set_text` (the language's codepage).
- In this worktree Bash refuses long heredocs and loops with computed sed ranges: write Python scripts in the scratchpad and run them with `python3 -I`.
