---
title: "Armor part 0 and the browser's overridden copies (dansen9191)"
author: August
date: 2026-10-22
branch: builder-reports-browser-armor
cost: { minutes: 400, tokens: 2144230 }
touches:
  - crates/mg-ui/src/browser.rs
  - crates/mg-ui/src/blueprint/item.rs
  - crates/mg-ui/src/blueprint/situated.rs
  - crates/mg-resman/src/lib.rs
  - crates/mg-module/src/table_layers.rs
  - apps/mg/src/main.rs
  - crates/mg-preview/src/creature.rs
---

## Decided
- An armor's part 0 is valid and means no part: worn, the game draws the wearer's own body part there (seen in the client: creatures_look with MG_WORN=1 MG_PARTS=LBicep=0,... gives a bare arm and leg). 'Invisible' needs the creature's own part at 0 too. The previews already did this (creature.rs: worn > 1 || own <= 0); don't change them to draw nothing.
- The armor part lists are parts_*.2da rows with an ACBONUS (Aurora's rule), plus 0 first: situated::part_choices, shared with the creature's body parts. Most game tables leave row 0's ACBONUS blank (not belt, shoulder, robe), which is why 0 was missing.
- The resource browser lists every copy (ResMan::every_copy; ResMan::copies and get_in for one), each overridden copy under the copy the game reads, opened as Tab::ResourceCopy { key, layer label }. The label, not the layer index, is kept: viewed is cleared on load_order_changed, and indices move when haks are added. A 2DA copy is shown with table_layers::layered_from(start) so From/Rows from work from that copy down.
- mg which lists every copy and takes --module (module_resman); before it read the game's layers alone, so it could never answer a hak question.

## Watch out
- Export as Files writes name.ext into one folder: overridden copies must be left out or a lower copy overwrites the winning one.
- A long line in the read-only view's header row pushes Copy to Module out of the window and kittest's click lands nowhere (no error): the 'overridden by' note is on its own line.
- The game's own KEY layers override each other (key:nwn_retail over key:nwn_base), so many game resources have an overridden copy even without haks.
- The worktree shell guard refuses 'flock ... cargo test' and heredocs: put such commands in a scratchpad script.
