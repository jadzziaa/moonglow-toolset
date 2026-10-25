---
title: Custom palettes kept as Aurora keeps them
author: August
date: 2026-10-25
branch: palettes-kept
cost: { minutes: 723, tokens: 5669132 }
touches:
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/05-blueprints.md
  - docs/manual/09-build-and-test.md
  - crates/mg-corpus-tests/tests/aurora_palettes.rs
  - crates/mg-corpus-tests/tests/groups/aurora.rs
  - crates/mg-edit/src/lib.rs
  - crates/mg-module/src/palette.rs
  - crates/mg-module/src/palette/kept_tests.rs
---

## Decided
- Aurora 89.8193.37 never rebuilds `<type>palcus.itp` on save: its custom palette IS the file as read (hak, override and missing blueprints listed; module blueprints not in the file not listed; no file: empty categories). Save writes it back but sorts each category by shown name and drops unknown fields. Edit+OK rewrites an entry only when shown text or category differs (NAME text); Edit Copy / Add to Palette add NAME text even for a talk-table name; Delete removes the entry. Only Build › Palettes and Refresh Palette rebuild: module + hak blueprints (NOT override), STRREF for talk-table names, by name then resref. Captures: `~/.local/share/moonglow-oracle/captures/palettes/`, made per `tools/aurora/capture_palettes.py`.
- Moonglow: a save calls `palette::update_custom_palettes(module, ws.saved(), game)`; `Workspace::saved()` is the resources as opened/saved (adopt() updates it too). Untouched kind => palette bytes not re-serialized. New/changed entries use STRREF for talk-table names (Build parity, not Aurora's Edit Copy) and never sort or drop fields. `rebuild_custom_palette(s)` = Aurora Build. The palette view shows `update_custom_palette` (falls back to the file); rows whose blueprint the module lacks get View, not Edit/Delete (`Tree::own`).
- `rename::rename` renames a blueprint's palcus entry in place (`palette::rename_entries`); the palcus RESREF is not a reference `refs` knows.

## Watch out
- Aurora's OK writes the blueprint again (new bytes) even with nothing changed; Moonglow only sees byte changes, so stale entries from outside stay until the blueprint changes or Build.
- On the oracle display :2 other sessions run the game client (`client_render`); its window pops over Aurora and eats clicks: wait until no "Neverwinter Nights: Enhanced Edition" window is up before each click. Never run `headless.sh stop` while another session uses it.
- Aurora's menus sometimes ignore a click on the menu bar; Escape twice, then click again. Build > Test Module launches the game: avoid.
- kittest: the menu bar has its own View and Edit labels; count labels before/after a right-click, with `query_all_by_label` (get_all panics on none). The app harness needs `run_steps(4)` (timed repaints).
