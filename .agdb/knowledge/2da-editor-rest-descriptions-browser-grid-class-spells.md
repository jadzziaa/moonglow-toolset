---
type: Decision
title: "2DA editor rest: descriptions, browser grid, class spells"
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
touches: [crates/mg-module/src/doctor.rs, docs/2da-editor-proposal.md, docs/deferred.md, docs/log.md, docs/manual/09-build-and-test.md, apps/mg/src/diff.rs, apps/mg/src/main.rs, apps/mg/tests/twoda_check.rs, crates/mg-corpus-tests/tests/twoda.rs, crates/mg-module/src/table_desc.rs]
fingerprint: v2-131a29d3ab78d127
created: 2026-11-02
author: August
description: "Design decisions for Moonglow's 2DA editor: the browser's read-only layered grid (table_view::grid_in, TableDoc), description lookup via table_desc::find_for, 2da-spells summarizing, nwscript constant limits, and test/path/UI pitfalls."
asks:
  - "How does the browser show a read-only layered 2DA table, and where are its TableDoc views stored?"
  - "How does Moonglow find the description file for a 2DA table, including family patterns like loot_*?"
  - "Why does mg 2da-check fail with no game install, and how do I avoid needing nwscript there?"
  - "My UI test fails right after opening a described 2DA table; do I need to wait for the folder walk to finish?"
---
## Decided
- The browser's read-only 2DA view is table_view::grid_in with layers: Some(&Layered) (read-only mode: no typing, column drag or edit menus; From column, changed cells, Rows from via TableDoc::within). Each view keeps a TableDoc in Browser::table_views keyed by (key, layer), rebuilt when the Layered's address or shape changes; ids count down from u32::MAX, and keys_in(read_only) acts only when keys_table == the view's id.
- Description lookup is table_desc::find_for(table location): name.2da.toml beside, else a family's (tables = ["loot_*"] in any *.2da.toml of the folder, closest pattern wins). Moonglow's own game-table descriptions (table_desc/spells.2da.toml, baseitems.2da.toml) write nwscript's constants out: a built-in needing nwscript fails with no game install (mg 2da-check --root nowhere).
- The editor walks a description's folder on a thread (TableDoc::walking, counted by under_way): UI tests settle() after opening a described table.
- 2da-spells: the game's packsp tables a hak's spells.2da reshapes are summarized once per table (TDoD: 18 findings instead of 2,919); faults the game's own tables have are dropped.
- const int X = Foo(); is refused by nwn_script_comp (INVALID VALUE ASSIGNED TO CONSTANT); a global int X = Foo(); compiles: constants() leaves it out and says so.
## Watch out
- Paths from mg_testkit::scratch_dir have ../ in them: compare a Beside's file and an open doc's Home::File by canonicalize.
- The cell panel under the 2DA grid has a fixed 96 px; a chooser opened there needs room (choosing_under) or it runs over the window's bottom.
