---
title: "Deferred sprint m08: Verify's skip and cache, outside changes told by the system, described variables"
author: August
date: 2026-10-29
branch: worktree-agent-ad71fbafc0ba5c468
cost: { minutes: 1878, tokens: 13758295 }
touches:
  - apps/mg/tests/vars.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-module/src/var_desc.rs
  - crates/mg-plugin/src/runtime.rs
  - crates/mg-plugin/tests/plugins.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/tests/app/projects.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/03-modules.md
---

## Decided
- Verify's skip is `mg_module::doctor::Skip`, read from words (`Skip::parse` for `mg verify --skip`, `Skip::read` lenient for settings): group ids, doctor check ids (`Check::from_id`), anything with `/` (a plugin check id), `references:CATEGORY` (`verify::Category::id`), `hak:NAME` / `NAME.hak` (matched against the layer label `hak:name`, lower case), `override`, `development`, and `name.ext` resources. `Doctor::push` drops what is skipped; the costly parts are gated before they run (custom resources filtered by layer label and key, tile models not read with tile-faces and mdl-read skipped, shaders not compiled with shader-compile skipped). Options › Verify keeps checkbox words in `verify_skip` and the rest in `verify_more` (a text field), merged on apply.
- Verify keeps each archive's models' facts (mdl-text findings, faces_over result) in the cache folder's `verify/`, one JSON file per layer named `label-fingerprint.json` with the release inside (`verify_cache::Memo`). Only `LayerClass::Erf` with fingerprint != 0 (a module layer is a MemContainer, fingerprint 0: never cached). Learned facts ride back from the parallel workers in `Doctor::learned` and are saved once at the end (written in parallel). `examine_groups` takes `Option<&doctor::Cache>` (`Cache::new(root)` has `shaders/` and `verify/`; `Cache::user()`); the UI passes the parent of its `shader_cache_dir`.
- TDoD (145 haks, load 18-37 on 32 threads): cold 2.3 s wall / 23.5 s user, warm 0.87-0.90 s / 8.1-8.3 s; `--json` byte-identical. The warm critical path is appearance.2da (0.43 s) in `two_da`.
- Files changed outside: `Watch` (mg-ui/src/outside.rs) is told by `notify` (a file is watched by its parent folder, so a write-then-rename is heard), debounces 300 ms quiet (10 s cap), and hands `Heard { all, paths }`; the window looks at once (`heard_outside` checked every frame, the watch asks for a repaint). `Module::outside_scan_of(paths)` snapshots only those files (+ known entries of their keys, for moves); scans with `only` skip no file by its stamp and walk no folder. Walk fallback every 3 s when the system can't watch; a told watch still walks every 30 s.
- A `.mod` is watched too: `Module::archive` (`archive_file::ArchiveFile`) keeps the as-read Arcs and the stamp; the scan reads the whole ERF when the stamp changed, changes are `castle.mod/name.ext` virtual paths. The stamp is taken as read only when every change found at it is taken (`waiting`), and `save_as` refuses (ChangedOnDisk) only while changes found wait for an answer — not when nobody looked (reloading off, `mg`), which keeps the old overwrite.
- Scripts read again compile 4 ms a frame (`compile_outside_step`, `script_view::compile_edits` / `put_compiled`), applied once at the end (one undo step, or derived when the module is clean), dropping scripts whose source Arc changed meanwhile.

## Watch out
- The worktree's Bash guard refuses many heredocs and long python -c: write scripts to the scratchpad with Write and run them with python3 PATH.
- `sed -i 'Nd;Md'` deletes by original line numbers in one pass; I deleted the wrong line once (a `self.outside.content = None;`) — check with sed -n after.
- frame_perf `reload_frames_at_144` with FRAME_RELOAD_AREAS=10000 FRAME_RELOAD_BLUEPRINTS=2000 (2,870 files) varies 6.7-15 ms longest frame between runs; the look's start was 3.8 ms until `outside_scan_of` stopped asking each path `is_dir()`.
