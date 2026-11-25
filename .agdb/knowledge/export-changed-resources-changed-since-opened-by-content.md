---
type: Decision
title: "Export Changed Resources: changed since opened, by content"
description: "How File › Export Changed Resources finds what changed in the session: Workspace keeps the opened resources and compares content, not touched keys; why, and the edge cases settled (undo, save, module.ifo, areas, compiled scripts, unsaved editors)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T23:56:44Z }
touches: ["crates/mg-edit/src/lib.rs:Workspace::changed_since_opened", "crates/mg-ui/src/transfer.rs:changed_selection", "crates/mg-ui/src/transfer.rs:Moonglow::open_export_changed"]
asks:
  - how does Moonglow know which resources changed since the module was opened
  - why does an undone edit not count as changed in Export Changed Resources
  - why is module.ifo unticked in Export Changed Resources
  - does a save reset what Export Changed Resources lists
fingerprint: v2-1fac0c0362d25eef
created: 2026-11-24
author: augustaugur
---
Twohand's ask (5.3.0): export only what a builder changed this session.

Settled: `Workspace` holds `opened` (the resources at `Workspace::new`, sharing
their Arcs; never moved by save or adopt), and `changed_since_opened()` compares
the module with it by content. Not a set of touched keys: `last_touched` and
`touched` don't cover `derive`/`adopt` per key and would count an edit that was
undone. The only non-test `Workspace::new` in mg-ui is `use_module`
(lib.rs), so the session begins at open/new and ends at close or the next open
for free.

- Edited, unflushed documents are compared by what they hold (the doc
  against the parsed opened copy). Others by data: Arc ptr, then bytes, then
  for GFF the parsed documents (an edit undone after a save is written back in
  Moonglow's layout, because `write_back` only restores the bytes of `saved`,
  which the save moved).
- Cached by revision (every change of content moves it; a flush writes what the
  docs held already), plus a per-key verdict kept with the Arc it was told from.
- Counts every route: commands, import, plugins, derive (compiled scripts,
  custom palettes at save), adopt (files changed outside and read again).
- The UI ticks changed + companions (an area's ARE/GIT/GIC, a script's NCS)
  with "Include the resources they use" off; module.ifo is never ticked (an
  ERF import of it replaces the other module's own; refused_type only guards
  loose files), and an ifo-only change leaves the command grayed. Removed
  resources are counted in the note.
- Unsaved script buffers / module 2DAs are not in the workspace until saved,
  so not in the export (as a normal Export); the dialog warns of them.
- Menus show no why-disabled tips in general (Quick Open does); this command
  adds one for itself in `commands::button`.
- A module without custom palettes gets them at its first save with game
  data: they count as new (changed). A module that has them sees no change
  from a bare save (tested with the game's data).
