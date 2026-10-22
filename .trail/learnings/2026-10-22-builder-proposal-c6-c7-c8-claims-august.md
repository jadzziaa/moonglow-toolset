---
title: "Builder proposal C6, C7, C8: claims, protected names, project commands"
author: August
date: 2026-10-22
branch: builder-c6-c7-c8
touches:
  - crates/mg-module/src/claims.rs
  - crates/mg-module/src/project_file.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-ui/src/claims.rs
  - crates/mg-ui/src/project_commands.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/tree.rs
  - crates/mg-ui/src/outside.rs
  - apps/mg/src/claims.rs
  - docs/manual/03-modules.md
---

## Decided
- C6: the claims table is a 2DA named by `[claims] file` (the hand-back proposal's own choice; TDOD had no file yet): `Area`, `Builder` needed; `Status` (claimed/free/merged), `Date`, `Tag` read where present; other columns kept. Parsed and edited in `crates/mg-module/src/claims.rs` with its own line-aware `Table` (cells split as `mg_2da` splits them; a test checks they agree). Claim/Release rewrite one row's line laid out under the header's column starts, or add a row after the last: never `TwoDa::to_bytes` (re-aligning every line would make two builders' claims conflict in git).
- The table is the project file's fourth stamped file (`ProjectFile.stamps[3]`), so `changed()` and the folder watch reread it; a missing table is a warning and an empty table.
- `ProjectFile::access(area)` (Edit / Warn(why) / ReadOnly(why)) and `protected(area)` are the rules; `[claims] others` defaults to warn, `unclaimed` to edit. No builder code: every claim is another's.
- Enforcement is one gate: `claims::gate` called from `Moonglow::apply` (every edit of every window goes through it). Read-only refuses with an `EditError::Invalid`; a protected area's Tag change, rename (an ARE removed while another ARE is set) or delete is held: `ProtectedAsk { then: Action }`, `claims.held` tells `Action::Apply` not to log it. Rename…/Delete… are asked about before their windows (`claims::asks_first` at the top of `Moonglow::run`); "anyway" sets `let_through` for one pass.
- Warn mode asks at save: `claims::hold_save(app, then)` in `Action::Save` and `SaveThen`, comparing `module.get` with `module.as_read` for ARE/GIT/GIC after a flush; Save Anyway sets `cleared` and pushes `then`.
- C8: `[commands]` (string, or `{ run, windows, unix, label }`); allowed by name+text in user.toml `[allowed_commands]` (`ProjectFile::set_allowed_command`, via `with_value`, the generalised `with_code`). Runs as a job (modal; synchronous in tests, so UI tests see the whole run): `pause_outside()` (drops watch and look), `release_haks()` (hak layers removed, `sync_haks` puts them back: haks are memory-mapped, the only project-adjacent files Moonglow holds), `sh -c`/`cmd /C` + `raw_arg`, output lines through an `Arc<Mutex<Vec>>` drained into the log each frame, then `look_outside()` (or `reload_project_files()` without background jobs). `take_outside` now calls `sync_haks` when module.ifo changed.

## Watch out
- In tree.rs the row closure must not take `app` whole (it would capture `*app` uniquely while `ws` is borrowed): pass `claims::of(&app.ws)` and `&mut app.actions` (disjoint field captures).
- egui_kittest: "Cancel" matches Module Properties' too; use `get_all_by_label("Cancel").last()`.
- A YAML description with ": " breaks okf lint (use a comma).
- The tree mark at `.small()` was unreadable in the picture; normal size reads.
