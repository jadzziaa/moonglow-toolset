---
title: "Builder proposal C10, C11, C13: hak usage, export stamp, locks out of projects"
author: August
date: 2026-10-22
branch: builder-c10-c11-c13
cost: { minutes: 165, tokens: 1247837 }
touches:
  - apps/mg/src/usage.rs
  - crates/mg-module/src/stamp.rs
  - crates/mg-module/src/locks.rs
  - crates/mg-preview/src/files.rs
  - crates/mg-module/src/nasher/project.rs
---

decided: `mg haks --usage` lives in `apps/mg/src/usage.rs`, not a library: it needs mg-module (refs, Module) and mg-preview (previews, `files::Gatherer`), and neither crate depends on the other (the viewer pins mg-preview by tag). Move it to a crate when the window wants it.
decided: the usage walk resolves against the game a player has: `GameInstall { user_dir: None, workshop: false }` for the base, haks added with the real install (`add_haks` takes it separately, so user `hak/` still counts), then the module. The user's override would otherwise shadow hak copies and lie about the release.
decided: a hak resource is used / shadowed (`rm.origin` is another layer) / not judged (2DA, GUI, fonts, SHD, ITP…, or a replacement of a base resource) / unused. One shared `Gatherer` for the whole module, so each model is parsed once.
watch out: `ResMan::texture` READS the bytes; use `ResMan::texture_type` (added) to learn DDS vs TGA. Over a 28 GB hak stack the difference is the run.
watch out: an armor item's preview (`mg_preview::item`) is a human male body WEARING it: its head, hands etc. are not the item's. Read the `ArmorPart_*` numbers and expand `p{g}{r}{ph}_{part}{nnn}` over appearance.2da's part-based race letters and phenotype.2da's rows.
decided: export stamp goes in the ERF description after the builder's text (`Moonglow-Export:` / `Base: git <sha> [with uncommitted changes]` / `Module: name (tag)`), `mg_module::stamp`. An extra resource would be imported into the module by Aurora and Moonglow.
decided: MG_Locked stays in memory (undo, area model, Unlock All unchanged) and is stripped only where a project GIT becomes JSON (`Project::source_bytes`). Locks live in data_dir/locks/<root>-<hash>.json, written by the window after a save, restored by `Module::restore_locks` on open and after take_outside. restore_locks must also set the project's `Source.resource` (`stands_for`), or every git pull of that area is a false "changed outside" conflict.
watch out: the "file already had MG_Locked" check must parse `src.file` (the JSON text), not `src.resource`: after a save Adopts, `src.resource` holds the in-memory bytes with the locks.
ruled out: stripping in module folders (write_folder compares v == read in several places; binary GFF has no line diff anyway) — recorded in deferred.md.
