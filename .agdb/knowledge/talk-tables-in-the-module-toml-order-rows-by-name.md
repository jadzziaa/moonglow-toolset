---
type: Decision
title: "talk tables in the module, toml order, rows by name"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [apps/mg/src/main.rs, apps/mg/tests/groups/cli.rs, apps/mg/tests/row_names.rs, crates/mg-module/src/areas.rs, crates/mg-module/src/query.rs, crates/mg-rules/src/rows.rs, docs/deferred.md, docs/log.md, docs/manual/02-main-window.md, docs/manual/11-command-line.md]
fingerprint: v2-2e98e263457cdbb7
created: 2026-10-29
author: August
description: "Developer notes on talk tables: Module source saving via save_with_module, read-only layers for install tlk dirs, toml key-order via toml_edit, name-to-2DA row mapping, GFF JSON annotation tolerance, and worktree shell quirks."
asks:
  - "How does saving a talk table kept in the module work, and when is it marked saved?"
  - "Why does my tlk file from the game's data/tlk install folder open read-only instead of saving?"
  - "Why are TOML keys coming back sorted, and how do I read the file's original key order?"
  - "How do I map a field or row name to its 2DA table, and why does mg find --where need a type per object?"
---
- A talk table kept in the module is `talk::Source::Module` (found by the layer's priority, `priority::MODULE`, not its label); `Table::save_with_module` hands its bytes to a callback. `talk_view::save` applies the module step inside that callback (the table taken out of `app.talk` meanwhile), so the table is marked saved only once the module took it; the module's own Save writes it.
- `install.tlk_dirs()` includes the game's `data/tlk` (premium campaigns): a File source outside the user folder is turned into a read-only Layer in `talk_view::open`, or its Save would write into the install.
- The `toml` crate's tables are sorted (no `preserve_order` feature in the workspace): anything needing a TOML file's own key order (mg tlk-alloc's names) reads it with `toml_edit` (`tlk_alloc::toml_in_order`); serde_json has `preserve_order`, so JSON keeps it.
- Rows by name: `mg_rules::rows::row_table(file_type, path, label)` is the one map from a field to its 2DA; `GameData::row_of` is its inverse (label, then name, ambiguity said). `mg find --where` needs a value per object type (`Query::typed_fields`), since `Appearance` is a different table per kind.
- `mg_gff::from_json` skips struct keys starting `__` and reads only `type`/`value` of a field object, so extra `row`/`__row` annotations read back unchanged.
- In a worktree session, Bash refuses heredocs mixed with other commands; `python -c` with backslash-newline inside double quotes drops the backslash (Rust string continuations lost): edit such strings with Edit.
