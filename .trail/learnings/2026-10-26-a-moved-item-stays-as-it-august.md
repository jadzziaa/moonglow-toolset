---
title: A moved item stays as it is; its window follows by what it is
author: August
date: 2026-10-26
branch: inventory-moves-intact
cost: { minutes: 718, tokens: 8033447 }
touches:
  - crates/mg-corpus-tests/tests/aurora_inventory.rs
  - crates/mg-module/src/instances.rs
  - crates/mg-ui/src/blueprint/creature_lists.rs
  - crates/mg-ui/src/blueprint/follow.rs
  - crates/mg-ui/src/blueprint/inventory.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/05-blueprints.md
---

## Decided
- A moved item is the item itself (Aurora's capture `inventory-moves/`: its own name, tag, flags kept): `mg_module::instances::{listed_item, equipped_item}` change only where it is held (struct id; `Repos_PosX`/`Repos_Posy` in a backpack, none in a slot; a blueprint entry's `InventoryRes` renamed `EquippedRes` in place). `Form::carried` keeps the struct when the kind matches (whole item into a placed object, entry into a blueprint), else makes it from its blueprint with its flags.
- Every move is one Command: the removal and the insert together (Equip, To Backpack, slot to slot, drags between objects, a store's page too). The creature inventory carries a `Moving { item, taken }` into `equip_edits`; the feat question stores it with the slot, so No leaves the item where it was (before, the backpack item was already taken out).
- `follow::follow` simulates the edits on copies of the GITs touched and tracks each open window: an item taken out is found again by `same_item` (all fields but struct id, the grid fields and `Infinite`) in a later insert (any list, any area; nested in an inserted store page) or, at the end, in a list set whole (Cancel), else where its list was set back if the item there has its `TemplateResRef`. `Followed` carries the area, so a window can change area.
- A placed object's window keeps a stable egui id (`Moonglow::window_ids`, given on first draw, moved by `retarget`): `TabViewer::id` and `Form::id` use it, so the scroll, a TextEdit's buffer (`app.buffers` by id) and the focus survive a move.

## Tried and ruled out
- Matching a moved item by exact struct equality (the old `follow`): equipping changes the struct id and drops the grid fields, so the window closed.
- Unique ids derived from the old path at a move: a new window opened later at the old place got the same id (a clash); a counter-made id per window avoids it.

## Watch out
- A new held item's Properties opens as a tab in the creature's window, hiding it: tests that need both in sight move it into its own window (`own_window` in tests/app.rs).
- `git checkout develop -- crates/...` to check a test fails without the fix takes develop's newer files when develop has moved; check out the branch's base commit instead.
- The backpack's rows sit below the Equipment: a test window must be tall (1320 points) or the rows are out of view.
