---
type: Decision
title: Palettes CR/FACTION at save; 2DA empty last rows
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
touches: [docs/deferred.md, crates/mg-ui/src/table_view.rs, crates/mg-ui/tests/app/tables.rs, docs/log.md, docs/manual/03-modules.md, crates/mg-module/src/table_edit.rs, crates/mg-ui/src/browser.rs, crates/mg-ui/tests/editor_perf.rs, apps/mg/src/diff.rs, crates/mg-2da/src/lib.rs]
fingerprint: v2-0446189ed1ac2197
created: 2026-11-02
author: August
description: "Records how palette saves set CR/FACTION and sort entries (stable by name) in Aurora and Moonglow, how 2DA parsing treats a labeled empty last row, and two gotchas: shared scratchpad commit files and a squeezed chooser."
asks:
  - "Why do palette creature entries get their blueprint's CR and FACTION after Save As, even when the old values were stale?"
  - "In what order does a palette save arrange entries with the same name, and does it break ties by ResRef?"
  - "Why does my 2DA lose or keep its last empty row depending on whether TwoDA::parse or nwn_twoda reads it?"
  - "My chooser in the cell panel shows up as a tiny sliver; how do I fix the layout?"
---
## Decided
- Aurora's Save As gives every creature palcus entry its blueprint's CR and FACTION, stale or not (captures/wave1/saveas.mod); Moonglow's update_custom_palettes does it through creature_entries_made_right (module blueprint, else resman's).
- A save puts in order only the category an entry went into, a STABLE sort by name (no ResRef tie-break): Aurora's captures keep same-name entries in file order (mgp_uti before md_strname).
- The engine counts a labeled empty last 2DA row (Get2DARowCount, engine_counts_an_empty_last_row); TwoDa::parse keeps it now, nwn_twoda drops it (compared without them). Pickers drop such rows anyway (choice_list's named_row).
- to_bytes_like keeps row labels by position and tabs; the editor saves like the bytes it read (TableDoc::read), and rereads when the source changes.
## Watch out
- The scratchpad is shared between packages: a commit -F msgN.txt can pick up another session's file; write the message file just before committing.
- A chooser drawn inside the cell panel's horizontal_wrapped gets squeezed to a sliver: end_row() and a vertical of its own.
