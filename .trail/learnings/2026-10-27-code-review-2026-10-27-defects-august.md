---
title: "Code review 2026-10-27: defects fixed and where the costs are"
author: August
date: 2026-10-27
branch: develop
cost: { minutes: 37, tokens: 493666 }
touches:
  - CLAUDE.md
  - apps/mg/src/main.rs
  - apps/mg/tests/diff.rs
  - apps/mg/tests/groups/cli.rs
  - apps/mg/tests/json.rs
  - apps/mg/tests/merge_gff.rs
  - apps/mg/tests/models.rs
  - apps/mg/tests/nui.rs
  - apps/mg/tests/plugins.rs
  - apps/mg/tests/serve.rs
---

## Decided
- A file named after a resource goes through `ResKey::path_in(dir)` (refuses `ResRef::is_file_name() == false`: `/`, `\`, `:`, control chars). ResRef itself still accepts such names (readers are lossless); only writers to disk refuse. Nasher's `new_file` returns None for them.
- Plugin code loads with `set_mode(ChunkMode::Text)` (`runtime.rs` const `TEXT`): mlua with no mode passes "bt", and Luau runs bytecode unverified.
- Undo of `SetField { value: None }` restores the containing struct whole when the field was not last or had duplicate labels (`Workspace::removed_in_place`: list item -> RemoveItem+InsertItem, struct field -> SetField of the parent, root -> SetResource). `run_edit` returns `Vec<Edit>`. InsertItem that created its list undoes with SetField None.
- Save As into a folder that is not the module's and holds resource files not in the module is refused (ModuleError::Archive), nothing deleted.
- Script editor caches (`ScriptTools::symbols`, `script_nav` live checks) are dropped when another script's text or the workspace revision changes (`ScriptTools::seen`/`revision`, `Nav::check_again`); `close()` drops `spec`.

## Tried and ruled out
- Fixing per-frame costs without numbers: `frame_perf` showed every view inside 144 fps, so the per-frame findings (selection `contains`, camera rebuilt per projected point, 2DA editor filtering twice, log lines x fixes...) went to docs/deferred.md for large-input cases rather than being changed blind.

## Watch out
- `frame_perf reload_frames_at_144` now checks every file of the pull four times a second; "all in" is ~3.6 s.
- A refused tile Delete is computed on a clone: copy the clone's `blocked()` back (done in `Grid::delete`).
- `GameData::choices` copies; per-frame callers use `choice_list` (Arc). `Form::choices` returns the Arc.

## Second pass (the deferred list worked through)
- Measure before changing a per-frame cost: `frame_perf`'s persistent-world test now has views for a filtered 2DA editor, a 10,000-line log, the Export window, a store in the Preview window and an item's Properties page; the Tyrants test an area with everything selected and its walkmesh. A cache that measured no gain (blueprint category list, 0.88 to 0.87 ms) was reverted.
- Lists drawn row by row skip rows out of sight with `ui.is_rect_visible` and `add_space` (log, Find References, store preview). For the log, keep measured heights across width changes: clearing them made the scroll bar toggle each frame (`Harness::run exceeded max_steps`).
- Script symbols: stale only on frames with a key event (`symbols_for(.., typing, ..)`); a timer-based settle broke kittest's `run()` (repaint requests).
- Readers charge copies to `mg_core::bin::CopyBudget`; external programs go through `process::output_within`; script stamps changed, so the first Compile after upgrading recompiles everything.
