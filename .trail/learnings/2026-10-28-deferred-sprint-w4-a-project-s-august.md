---
title: "Deferred sprint w4: a project's rules and commands"
author: August
date: 2026-10-28
branch: worktree-agent-a913041ccdb94e95e
cost: { minutes: 1428, tokens: 12249215 }
touches:
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/03-modules.md
  - docs/manual/04-areas.md
  - docs/manual/09-build-and-test.md
  - docs/manual/10-options.md
  - docs/manual/11-command-line.md
  - apps/mg/src/main.rs
  - apps/mg/tests/vars.rs
---

## Decided
- `var-runtime`/`var-name` nearest name takes builders' globs: `var_desc::glob_distance` is an edit distance where `*` matches any run for free (each `{a,b}` alternative tried); a glob is near only when the distance is at most a third of its fixed places (`*_x` is near nothing) and never a `runtime` glob.
- A String described by `table` with no `column` takes the row's label column (Label/LABEL/Name) as values; an Int keeps row numbers.
- Verify fixes: `var-runtime` is `Edit::RemoveItem` on the owner's `VarTable` (re-found by `rename::describe(...) == at`, the nth for several findings of one place: `fix::Seen.said`); `static-transform` takes `VisTransformList`/`VisualTransform` off (`areas::static_fields`). `Guard::check` now returns the fix to make, its paths and indices following the items earlier fixes took out (`Guard.removed`, in Verify's coordinates); a SetField of a whole list that lost an item is refused. Callers apply the returned fix but pass the original to `made`.
- `[resrefs] pattern`: `{name}` once, `{code}` and `{kind}` at most once; no `{code}` is the team's prefix for everyone (applies without a builder code); `{kind}` from `[resrefs] kinds` by `project_file::KINDS` words; `ProjectFile::naming_for(ResType)` at each proposer, `naming()` leaves `{kind}` empty.
- A `.mod`'s project file: `project_file::Place` (root + module name), `ProjectFile::place_of(location)` (a .mod's only when `NAME.moonglow.toml` or `NAME.variables.toml` exists), `read_at(place)`. `root_of` stays None for a .mod: commands, claims, plugins and the builder's own file are a folder's; `[claims]`/`[commands]`/`[plugins]`/`[builder]` in a .mod's file are warned of. The UI rereads it from `reload_changed_files`' no-folder branch.
- Project commands: `ProjectCommand.writes`/`key`; key ids `project:NAME` (a plugin id has no colon), `keys::Group::Project`, the keymap rebuilt when the project's (name, key) list changes (`project_keys_from`). Cancel: `process_group(0)` + rustix `kill_process_group` (TERM, 2 s, KILL), Windows `taskkill /PID /T /F` before `child.kill()`.

## Watch out
- `ErfContainer` keeps an open `File`, not a mapping: "is the hak open" is `/proc/$PPID/fd`, not `maps`.
- The Windows test can run under Wine: `cargo test --target x86_64-pc-windows-gnu -p mg-ui --lib --no-run` builds (mingw is installed), then run the exe with a scratch `WINEPREFIX`, `DISPLAY`/`WAYLAND_DISPLAY` unset, `WINEDLLOVERRIDES="mscoree,mshtml="` (else wineboot waits on an installer window) and `WINEPATH=Z:\usr\x86_64-w64-mingw32\bin` (libstdc++ for glslang).
- A UI harness on a sample project has Module Properties open: two "OK" buttons with Options open (`get_all_by_label("OK").last()`). A Button with `shortcut_text` has the key in its accessible label: `get_by_label_contains`.
