---
title: "TDoD notes on 2.2.0: long patterns, root = false, notes, builtin checks, var-runtime hint"
author: August
date: 2026-10-22
branch: tdod-notes
cost: { minutes: 215, tokens: 1160297 }
touches:
  - apps/mg/tests/plugins.rs
  - crates/mg-plugin/tests/examples.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/03-modules.md
  - docs/manual/11-command-line.md
  - docs/manual/16-writing-plugins.md
  - docs/manual/17-plugin-api.md
  - docs/plugins/CHANGES.md
  - docs/plugins/examples/tag-conventions/tests/builtin.toml
---

## Decided
- variables.toml names: the old 64-character cap had no reason behind it. `engine_local_variables` shows the game reads names of 4000 characters from a VarTable and SetLocalInt/GetLocalInt with 5000; Aurora's Variables window's Name field has MaxLength 1024 (docs/parity inventory, TdlgVarTable eName). So `var_desc::NAME_MAX = 1024` for a literal name, and a pattern (glob, braces) is not held to any length: `*` takes any, so neither the pattern's text nor each brace expansion means anything.
- Plugin tests' no-game: `root = false` (untagged `RootSetting { At(String), Game(bool) }`), `root = true` the machine's (overrides a file's false). Internally `Root { Machine, At, No }`, part of the fixture cache key. `GameFor` unchanged: with `Root::No` the callback is not called. Not `game = "only"`: `game` is a folder path and "only" a legal folder name. With no game and no folder, `ctx.game` is still None (as without an install), not the module alone.
- Passing tests' hints: `TestResult.notes`; the lacked-resource line stays a problem on failure (existing asserts), a note on pass. `Host::game_listed()` (default no-op) is called by `ctx.game:resources()`; the hint applies only when the test has no game install.
- Built-in checks in plugin tests: `builtin = "id"`, a key of its own, not a `check` fallback (a reader can't tell which ran without the manifest, and a plugin adding a check of that id later would silently change the test). mg-plugin already depends on mg-module and mg-edit, so `run_builtin` calls `doctor::examine_groups` directly, skipping every group but the check's, filters by id, and `mg_edit::fix::attach(module, findings, &[])` gives Verify's fixes, so `fix`/`left`/`[[test.field]]` work. The fixture's project file comes with `Module::open`; its warnings are notes on builtin tests.
- var-runtime nearest name: `Variables::nearest(name, keep)` (shared with var-name's `unknown`) and `runtime_typo(name)` (only when a runtime *pattern* took the name; any literal entry other than the name itself). Message tail `: NAME? (variables.toml:LINE)`.

## Watch out
- `GameData` is not Clone and `ResMan::add` at an equal priority inserts after the existing layer (lower precedence): the builtin `left` re-run reuses the game data with the fixture as it was layered.
- The tag-conventions example now has 5 tests (`tests/builtin.toml`, fixture `tests/project/`); `apps/mg/tests/plugins.rs` counts them, and with `--module` it is "2 passed, 3 failed".
- A heredoc script for python edits is refused when it is long; write it with Write to the scratchpad and run `python3 script.py`.
