---
title: "Unused scripts: one line, 2DA scripts, hak reach, [used] marks"
author: August
date: 2026-10-28
branch: worktree-agent-ab70e71cdb3ba8ec3
cost: { minutes: 26, tokens: 19310 }
touches:
  - apps/mg/src/main.rs
  - apps/mg/src/usage.rs
  - apps/mg/tests/verify.rs
  - crates/mg-corpus-tests/tests/doctor.rs
  - crates/mg-corpus-tests/tests/modules.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-module/src/project_file.rs
  - crates/mg-module/src/project_plugins.rs
  - crates/mg-module/src/refs.rs
  - crates/mg-module/src/table_desc.rs
---

## Decided
- The Unused report is `mg_module::verify::unused_of(module, references, also)` returning `Unused { key, script: Option<ScriptFiles> }`: a script once (its .nss key where the source exists, else .ncs). `also` comes from `verify::also_used(module, refs, rm)`: `table_scripts_in(rm)` (doctor::SCRIPT_COLUMNS, resolved through the resman so module/hak 2DAs win), `described_scripts(module)` (a module 2DA column with `kind = "script"` in its name.2da.toml), and `outside_references` (BFS from the module's references and script strings into hak/game GFFs and NSS).
- String literals count for every checked kind (scripts, DLG, blueprints), computed inside unused_of from module NSS via `refs::script_strings` (also used by mg haks --usage). They are NOT References: putting them in module_references would make every literal a missing-script error. A script's own name doesn't count.
- Marks: project file `[used] scripts|conversations|blueprints` (team file only), `ProjectFile::set_used(place, key, glob, add)`; `ProjectFile::place_to_write` gives a .mod's NAME.moonglow.toml even before it exists (place_of returns None until then).
## Watch out
- Without following into haks, Kingmaker reported 1,220 unused scripts (its dlg/utc live in km_resources.hak); script literals naming hak blueprints (CreateObject) must seed the BFS, and so must the table scripts (game's x0_s0_dirge names x0_s0_dirgeen).
- toml_edit inserts a first table before a comment-only file's text: set_used appends `[used]` as text when the section is new.
- areaeffects.2da names scripts the game doesn't have: the engine doesn't read it.
