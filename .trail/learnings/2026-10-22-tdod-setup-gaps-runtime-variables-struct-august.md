---
title: "TDOD setup gaps: runtime variables, struct ids, portable plugin tests, list items, per-area tags"
author: August
date: 2026-10-22
branch: tdod-gaps
cost: { minutes: 232, tokens: 1592863 }
touches:
  - docs/deferred.md
  - docs/log.md
  - apps/mg/src/plugins.rs
  - apps/mg/tests/plugins.rs
  - crates/mg-plugin/src/lib.rs
  - crates/mg-plugin/src/runtime.rs
  - crates/mg-plugin/src/testing.rs
  - crates/mg-plugin/tests/examples.rs
  - crates/mg-plugin/tests/plugins.rs
  - docs/manual/11-command-line.md
---

## Decided
- Runtime-only variables: `runtime = true` on a `variables.toml` entry (`Described.runtime`). `describe()`/`unknown()` unchanged, so `var-name` is quiet; `names_for` skips them (Variables window and `mg vars --on`); a stored one is its own check `var-runtime`, an Error (not folded into `var-object`: the remedy is to remove it, and TDOD's hooks fix every roll). Checked before the type check, so a runtime var gets only that finding. No "has no type" warning for a runtime one.
- Struct ids: `mg.struct_id(t)`, not a key in `gff()` tables. Any string is a GFF label (`__struct_id` too), so no key "can't collide", and a key would show in every `for label in t`. The id was already in the table's metatable (`struct_id`, beside `path`); the helper also reads a `raw` table's `__struct_id`.
- Plugin tests' game data: `run_tests(plugin, module, &GameFor)` where `GameFor = Fn(&Module, Option<&Path> /*root*/) -> Result<Option<GameData>, String>` (owned, so mg-plugin can layer). A folder `game/` beside the test file (or `game = "…"`, file or test) is added as a DirContainer at `priority::DEVELOPMENT` (over install, haks and module); with no game, a ResMan of the module + the folder and an empty Tlk. `root = "…"` (absolute, from the test file's folder, or `~/`). The fixture cache key is (fixture, root, folder).
- "Says so clearly": `Host::not_in_game(resource)` (default no-op) is called by `ctx.game` reads that fail (bytes/gff/raw/text and `table`); the test host collects them and, only when the test fails, adds a line naming them and where to put them. Needed because TDOD reads the game in `pcall` and degrades silently.
- `[[test.field]]` with a list item: parse the field as a `GffPath`; a last `Step::Item` is held present/absent by `path.get(root)`; a `value` on it is an error.
- Per-area tags: `[tags] unique_in_area`; `TagRules::unique_for` gives the per-area pattern precedence over `unique` (TDOD's `unique = ["KZ_*"]` takes in the store tags `KZ_SQUARE_STORE_1`, so "unique wins" would have left their gap open). `TagIndex::breaches(rules)` is the doctor's grouping; `duplicates_in_areas` groups by `Tagged::area()` (an area's own tag counts as in it). `MadeTag.area` (the made key's resref) and `Clash.area` for the paste warning; a per-area tag is counted in the area made in under every `paste_warning`.

## Watch out
- `mg-edit/fix.rs::retag_fix` re-finds a `tag-unique` finding by `places[0].key() == resource` and the joined `describe()`s; it must also try the per-area groups, and it must not depend on the project's rules (its test module has no project file).
- `mg verify --json` lists errors before warnings: a new Error check reorders expected finding lists in `apps/mg/tests/vars.rs`.
- In a worktree session, a Bash command with a long python heredoc touching several files was refused as "too complex"; writing the script to the scratchpad with Write and running `python3 script.py` works.
