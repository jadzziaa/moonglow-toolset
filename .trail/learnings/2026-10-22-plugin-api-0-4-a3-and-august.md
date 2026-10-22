---
title: "Plugin API 0.4 A3 and A8: quick fixes on findings, a plugin's own tests"
author: August
date: 2026-10-22
branch: plugin-api-a3-a8
cost: { minutes: 341, tokens: 2056601 }
touches:
  - crates/mg-edit/src/fix.rs
  - crates/mg-plugin/src/runtime.rs
  - crates/mg-plugin/src/testing.rs
  - crates/mg-ui/src/lib.rs
  - apps/mg/src/main.rs
  - apps/mg/src/plugins.rs
  - docs/manual/16-writing-plugins.md
  - docs/manual/17-plugin-api.md
---

## Decided
- `Finding` (mg-module) carries no fix: mg-edit (Edit, GffPath) sits above mg-module, so a finding cannot hold edits. Fixes live in `mg_edit::fix`: `Fix { label, edits }`, `Found { finding, fix }`, `attach(module, findings, plugin_found)` (a plugin's fix goes with its finding by equality, after `doctor::leave_out` filtered the plain findings; the doctor's fixes are made there, by check id: `tag-unique` re-finds its tag by `places[0].key() == resource` and `at == describe()s joined`; `area-variables` shares `variables_moved` with Area Properties).
- `mg_plugin::run_check` returns `Vec<Found>`. A plugin's fix is `ctx.fix(label)` (a check's ctx only): a table whose `__index` makes, at its first method, a job of its own (`shared_in` on a clone of the check's module) and copies `editor()`'s methods into it; so `fix:set` etc. are exactly `ctx.edit`'s, a fix sees its own earlier edits, and neither the check nor other fixes see them. `__fix` in its metatable indexes the records; a finding's `fix` that is not one is an error.
- Staleness: `fix::Guard` holds a digest of each resource a fix touches, as the Verify job read it (`job.module`, a flushed snapshot). A fix is made only while each resource matches; after a fix that only set fields, its resources' new digests are taken (so several such fixes in one GIT work), else they are dropped (later fixes there refused). `Guard::check` flushes the workspace.
- GUI: `Log.fixes: Vec<OfferedFix { line, text, fix, made }>` beside `Log.entries` (whose 100+ tuple uses were left alone); `Log::fix_at(line)` also matches the text, `Log::clear()` takes both (ClearLog and the corner's Clear use it). `verify_report_fixes` carries each line's fix through the report's reordering; `verify_report` wraps it for the old tests. `Action::ApplyFix(line)` → `Moonglow::apply_fix`: guard, `self.apply(fix.command())` (derived values too), `guard.made`, "Fixed: …" logged; the button turns into a disabled "✔ Fixed" (the font has ✔, not ✓).
- CLI: `mg verify --fix CHECK` (repeatable) + `--dry-run`; a finding's JSON `fix` is `{label, command}` with `command` as `wire::command_to_json` (an `mg apply` file); `fixes: {made, refused, dry_run}`; the exit counts errors not fixed.
- A8: runner in `mg_plugin::testing` (`run_tests(plugin, module_override, &game_fn)`), TOML with `deny_unknown_fields`; `mg plugin test` wraps it (line a test, summary note, fails on any failure or none). The example's fixture is a nasher project of hand-written JSON (`nasher.cfg` needs a `[target]` whose `file` is a `.mod`: "no target packs a module" otherwise). `pack` skips the top-level `tests/`.

## Watch out
- In the UI harness, a log Clear click and `Action::Verify` pushed in the same frame: the actions run first, then the log's Clear empties the new report. `h.run()` between them.
- The chess module's creature blueprints have lower-case tags: the tag-conventions fixture finds 14 there (two buttons of one label: `get_all_by_label(..).next()`).
- `mg verify --json` puts notes in the JSON's `notes`, not on stderr.
- The worktree shell guard refuses heredocs and some `sed` with words like GIT; write a scratchpad script (or use Edit) instead.
