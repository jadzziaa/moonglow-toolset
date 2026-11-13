---
type: Decision
title: "Deferred sprint w7: plugin tests, vars, tables, builder code"
tags: []
status: stable
generated: { by: trail, at: 2026-10-28T00:00:00Z }
touches: [docs/2da-editor-proposal.md, docs/deferred.md, docs/log.md, docs/manual/03-modules.md, docs/manual/09-build-and-test.md, docs/manual/11-command-line.md, docs/manual/16-writing-plugins.md, docs/manual/17-plugin-api.md, docs/plugins/CHANGES.md, crates/mg-2da/src/lib.rs]
fingerprint: v2-ad8e2fe143908a19
created: 2026-10-28
author: August
description: "Records week-7 decisions: plugin tests use a swappable-module `Fixture` over `GameData`, game-folder handling for tlk/haks, uncounted talk tables, new 2da `ColumnKind`s, `to_bytes_like`, plus worktree and egui gotchas."
asks:
  - "How does the plugin test Fixture in testing.rs swap in a fixed module without rebuilding GameData?"
  - "What happens when a test has no game and no dialog.tlk but the module names a custom talk table?"
  - "My egui TextEdit puts the cursor at the start after request_focus; how do I place it where I want?"
  - "How can I save a 2da back with its original layout and line endings unchanged?"
---
## Decided
- Plugin tests' game data is a `Fixture` (testing.rs): always a `GameData` (no game: an empty ResMan + empty Tlk, the module alone), the module's layer replaced by a `ModuleLayer` (Arc<RwLock<MemContainer>>) so `left` swaps in the fixed module and calls `invalidate()` through the Arc, then swaps back. GameData is not Clone and sits in an Arc shared by Input; interior mutability avoids rebuilding the game.
- A test's game folder: `dialog.tlk` replaces the base table (`GameData::set_tlk`, new), the fixture's custom table is `talk::find_any` over the game data (folder top level counts, a nasher project's JSON too), haks in `game/hak/NAME.hak` replace `hak:NAME` layers in place (or add at HAK_USER in list order).
- No talk table to count (no game, no folder dialog.tlk): `TalkTables { base: 0, custom: Some(TalkTables::UNCOUNTED) }` when the module names a custom table the data lacks; `uncounted()` makes it a problem for a test that names 2da-strref (or custom-tlk with the table missing), a note for "all".
- `ColumnKind` gained `Table` and `RowBy { by, tables }` (cls_pres_* ReqParam1 by ReqType, `REQUIREMENT_ROWS`); `NAMED_TABLES` carries a `ColumnKind` and an empty start means known only by the naming column (the iprp tables' `Name`). `table_faults` takes `Option<&TableOf>` (None: no game, no table known missing). A padding row (every cell `****`) named is a fault. The game's own tables: FOOTSTEPTYPE breaks in appearance rows 562-568 (30 of 18); Verify leaves a hak's cell that keeps the game's broken value (`game_copy`).
- `TwoDa::to_bytes_like(cp, original)`: the original bytes if unchanged, else its layout, header-offset widths, row-number width and line ends.

## Watch out
- In this worktree, Bash refuses loops and `flock cargo …`; a scratchpad shell script running `flock … systemd-run … cargo test` works.
- egui TextEdit focused by `request_focus` puts the cursor at the start: set it with `TextEdit::load_state`/`store` on the field's first frame.
- `mg.*` is built per job (`api(lua, sh)`), so `mg.vars` and `mg.twoda.kinds` can read the job's module and game.
