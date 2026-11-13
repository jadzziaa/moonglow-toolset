---
type: Decision
title: "Unused scripts: one line, 2DA scripts, hak reach, [used] marks"
tags: []
status: stable
generated: { by: trail, at: 2026-10-28T00:00:00Z }
touches: [apps/mg/src/main.rs, apps/mg/src/usage.rs, apps/mg/tests/verify.rs, crates/mg-corpus-tests/tests/doctor.rs, crates/mg-corpus-tests/tests/modules.rs, crates/mg-module/src/doctor.rs, crates/mg-module/src/project_file.rs, crates/mg-module/src/project_plugins.rs, crates/mg-module/src/refs.rs, crates/mg-module/src/table_desc.rs]
fingerprint: v2-5d12c91460481ec2
created: 2026-10-28
author: August
description: "Unused-script reporting uses verify::unused_of with extra references from 2DA script columns, described 2DA columns, and BFS into hak/game GFFs and NSS, counts string literals, and supports [used] marks in the project file."
asks:
  - "How does mg_module's unused_of decide a script is unused, and where do the extra 'also' references come from?"
  - "Why did Kingmaker report 1,220 unused scripts, and how do I stop scripts used only through haks being flagged?"
  - "How do I mark a script, conversation, or blueprint as used so it stops appearing in the Unused report?"
  - "Why does set_used append the [used] section as text instead of using toml_edit when the project file only has comments?"
---
## Decided
- The Unused report is `mg_module::verify::unused_of(module, references, also)` returning `Unused { key, script: Option<ScriptFiles> }`: a script once (its .nss key where the source exists, else .ncs). `also` comes from `verify::also_used(module, refs, rm)`: `table_scripts_in(rm)` (doctor::SCRIPT_COLUMNS, resolved through the resman so module/hak 2DAs win), `described_scripts(module)` (a module 2DA column with `kind = "script"` in its name.2da.toml), and `outside_references` (BFS from the module's references and script strings into hak/game GFFs and NSS).
- String literals count for every checked kind (scripts, DLG, blueprints), computed inside unused_of from module NSS via `refs::script_strings` (also used by mg haks --usage). They are NOT References: putting them in module_references would make every literal a missing-script error. A script's own name doesn't count.
- Marks: project file `[used] scripts|conversations|blueprints` (team file only), `ProjectFile::set_used(place, key, glob, add)`; `ProjectFile::place_to_write` gives a .mod's NAME.moonglow.toml even before it exists (place_of returns None until then).
## Watch out
- Without following into haks, Kingmaker reported 1,220 unused scripts (its dlg/utc live in km_resources.hak); script literals naming hak blueprints (CreateObject) must seed the BFS, and so must the table scripts (game's x0_s0_dirge names x0_s0_dirgeen).
- toml_edit inserts a first table before a comment-only file's text: set_used appends `[used]` as text when the section is new.
- areaeffects.2da names scripts the game doesn't have: the engine doesn't read it.
