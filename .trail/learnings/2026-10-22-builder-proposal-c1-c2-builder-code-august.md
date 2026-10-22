---
title: "Builder proposal C1, C2: builder code in ResRefs, described variables"
author: August
date: 2026-10-22
branch: builder-c1-c2
cost: { minutes: 325, tokens: 2053888 }
touches:
  - crates/mg-module/src/project_file.rs
  - crates/mg-module/src/var_desc.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/src/builder_code.rs
  - apps/mg/src/main.rs
  - crates/mg-corpus-tests/tests/engine_local_variables.rs
---

## Decided
- C1: the team's pattern is `[resrefs] pattern` in moonglow.toml (`{code}` and `{name}` once each), not `[builder] resref`: `[builder]` stays the builder's own section of `.moonglow/user.toml` (the team's file warns of it). One helper, `project_file::Naming` (`ProjectFile::naming()`; `fit(name, number)`, `strip`), is threaded through every proposer as a `*_named` twin of the old function, which keeps Aurora's behavior byte for byte with `Naming::default()` (the Aurora corpus tests still pass): `blueprints::resref_named`, `new::resref_for_named` (inside `add_area_of`, so plugins' new areas get the code too), `palette_add::new_resref_named` (and `add_to_palette` takes `&Naming`), `palette_view::copy_resref_named`, `store_setup::next_name` (reads the module's), `script_wizard::default_name_named`. The name is cut, never the code; the builder's own code is stripped first so it is never doubled.
- The setter is Edit › Builder Code… (`crates/mg-ui/src/builder_code.rs`) and `mg builder-code`; `ProjectFile::set_builder_code` edits user.toml by DeTable spans (replace the value, insert under `[builder]`, append a section) so the rest stays as written. It rereads the project file at once (the 3 s watch is too late for the next wizard).
- C2: `variables.toml` beside moonglow.toml, read by `ProjectFile::read` as a third file (stamps, so the folder watch rereads it); parsed in `crates/mg-module/src/var_desc.rs` with project_file's `Reader`/`in_order`/`line_of` (made `pub(crate)`). Top-level tables are variables (globs quoted), top-level `undescribed = [...]` the typo switch. Values resolve lazily (`Described::choices(&dyn Lookup)`; `ModuleLookup` = project files via `table_desc::Beside`, the module, the game's resman) because ProjectFile has no game; the constants reader was split out of table_desc (`table_desc::constants`).
- Engine (`engine_local_variables`): GetLocalInt on a name in another case reads 0 (names are case-sensitive), an Int read as String is "" and as Float 0.0, a String read as Int 0, and one name may hold an Int and a String at once. So described names and globs match in their case only; a case-only match is `var-name`.
- Verify: `var-type` is an Error (fails `mg verify`), `var-value`, `var-object`, `var-name` Warnings, all in the Objects group. Owners walk the module's IFO, GITs (lists by `ObjectKind::from_list`; "StoreList" in a GIT is stores, in a store its pages) and blueprints, items held included (`var_desc::variables_of`, `Owner::of_place` for the window's target path).
- The Variables window never blocks OK on a description: notes in warn color under the list (truncated, one line each, room reserved), the type set only when a name changes to a described one.

## Watch out
- egui ComboBox's selected text is its accessible *value*: kittest finds it with `get_by_value`, not `get_by_label`.
- `get_all_by_role(TextInput).next()` is the tree's filter field, not a window's: give the field a hint and find it by `placeholder()`.
- A closure `|text, width| TextEdit::singleline(text)` used twice ties both borrows to one lifetime; a nested `fn` doesn't.
- The log said "moonglow.toml is gone" whenever the user file changed in a project without one: `log_project_file(again, had)` now takes whether it was there before.
