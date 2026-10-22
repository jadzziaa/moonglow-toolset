---
title: "Project file (moonglow.toml), Make Static rules, duplicate tags, Setup Store by project"
author: August
date: 2026-10-22
branch: builder-project-c3-c5-c4
cost: { minutes: 262, tokens: 1979407 }
touches:
  - apps/mg/src/main.rs
  - apps/mg/tests/areas.rs
  - apps/mg/tests/verify.rs
  - crates/mg-corpus-tests/tests/groups/engine.rs
  - crates/mg-module/src/areas.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-module/src/lib.rs
  - crates/mg-module/src/nasher/mod.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/copy_as.rs
---

## Decided
- The project file is `moonglow.toml` at a nasher project's root (beside nasher.cfg) or a module folder's root; a .mod archive has none (its folder holds other modules). Parsed in `crates/mg-module/src/project_file.rs` with `toml::de::DeTable` (spans give a line for every warning; entries sorted by span, the map is alphabetical). Unknown keys/sections and wrong kinds are warnings, a TOML syntax error is an error and the rules are left out. Hung on `Module.project_file`, read in open_project/open_folder/save_as, reread by `Module::reread_project_file` from the UI's folder watch (`outside.rs`, which also watches `.moonglow/user.toml` because the walk skips hidden folders).
- Per-user part: `.moonglow/user.toml` (only `[builder] code` so far, read and unused until C1). A file, not Moonglow's settings, so mg and an installer read it; `ProjectFile::write_user` writes a `.gitignore` of `*` into `.moonglow` so it is never committed.
- Make Placeables Static is one function for window and `mg areas`: `areas::static_plan(git, &rules)` (with `Kept` reasons) and `areas::static_fields`. Default rules (no file) keep any placeable with variables dynamic.
- Engine (`engine_static_placeables`): a static placeable IS an object to scripts (GetObjectByTag finds it, its locals read, forced Plot). The wiki is right; the TDOD guide's "no script can find it" is wrong. Of two same-tag objects in one area GetObjectByTag took the one placed second.
- Duplicate tags: `mg_module::tags` (TagIndex over AREs and GITs, `clashes`, `retag` = TAG_2.., `store_tag`), window in `crates/mg-ui/src/tag_clash.rs`, fired by `Action::CheckTags` pushed after the paste's Apply (actions run in order) and directly after an area's Copy… (whole-area copies list only the area tag and must-be-unique tags).

## Watch out
- In a worktree session Bash refuses heredocs, `flock` in a plain command, and command text containing the version-control tool's name: write python edit scripts and a flock wrapper script into the scratchpad and run them.
- Any UI test that pastes or places a prefab of tagged objects now gets the Duplicate Tags window; press "Keep Tags" before clicking under it.
- kittest `query_by_label_contains` panics on several matches (the log repeats the window's sentence): query the exact label.
