---
title: "talk tables in the module, toml order, rows by name"
author: August
date: 2026-10-29
branch: worktree-agent-a3584c5eba937b3ac
cost: { minutes: 1486, tokens: 11569189 }
touches:
  - apps/mg/src/main.rs
  - apps/mg/tests/groups/cli.rs
  - apps/mg/tests/row_names.rs
  - crates/mg-module/src/areas.rs
  - crates/mg-module/src/query.rs
  - crates/mg-rules/src/rows.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/11-command-line.md
---

- A talk table kept in the module is `talk::Source::Module` (found by the layer's priority, `priority::MODULE`, not its label); `Table::save_with_module` hands its bytes to a callback. `talk_view::save` applies the module step inside that callback (the table taken out of `app.talk` meanwhile), so the table is marked saved only once the module took it; the module's own Save writes it.
- `install.tlk_dirs()` includes the game's `data/tlk` (premium campaigns): a File source outside the user folder is turned into a read-only Layer in `talk_view::open`, or its Save would write into the install.
- The `toml` crate's tables are sorted (no `preserve_order` feature in the workspace): anything needing a TOML file's own key order (mg tlk-alloc's names) reads it with `toml_edit` (`tlk_alloc::toml_in_order`); serde_json has `preserve_order`, so JSON keeps it.
- Rows by name: `mg_rules::rows::row_table(file_type, path, label)` is the one map from a field to its 2DA; `GameData::row_of` is its inverse (label, then name, ambiguity said). `mg find --where` needs a value per object type (`Query::typed_fields`), since `Appearance` is a different table per kind.
- `mg_gff::from_json` skips struct keys starting `__` and reads only `type`/`value` of a field object, so extra `row`/`__row` annotations read back unchanged.
- In a worktree session, Bash refuses heredocs mixed with other commands; `python -c` with backslash-newline inside double quotes drops the backslash (Rust string continuations lost): edit such strings with Edit.
