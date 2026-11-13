---
type: Decision
title: "A taken slot's item goes to the backpack, as Aurora does"
tags: []
status: stable
generated: { by: trail, at: 2026-10-26T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/aurora_inventory.rs, docs/deferred.md, docs/findings.md, docs/log.md, docs/manual/05-blueprints.md, crates/mg-ui/src/blueprint/creature_lists.rs, crates/mg-ui/tests/app.rs, crates/mg-module/src/instances.rs, crates/mg-ui/src/blueprint/follow.rs, crates/mg-ui/src/blueprint/inventory.rs]
created: 2026-10-26
author: August
description: "Putting an item into an occupied equipment slot now sends the displaced item to the backpack in the same command, matching Aurora, with no prompt or swap; only slot-to-slot weapon moves still ask the feat question."
asks:
  - "What happens to the item already in a slot when I drop another item onto it?"
  - "Why did my bandit's primary weapon end up in the backpack after equipping a new one?"
  - "When does the feat prompt still appear when equipping a weapon, and when is it skipped?"
  - "How do `equip_edits` and `after_taken` determine the displaced item's id and grid cell in the backpack?"
---
## Decided
- An item put in a taken slot (a drop from the backpack, the palette or another slot, or a slot's Equip) sends the slot's item to the backpack as it is, in the same command, as Aurora does (capture `inventory-slots/`, held to it by `aurora_inventory.rs`): no question, no refusal, no swap between two rings. `equip_edits` takes the backpack too; `after_taken(f, items, list, taken)` gives either list as it is once the moved item's removal has run, so the displaced item's id and grid cell are those after it (a backpack item just taken out leaves its cell, which the displaced one takes, as in Aurora).
- The feat question stays on a slot-to-slot move: Aurora asks it when a weapon needing a feat is dropped on a weapon slot, before anything moves.

## Tried and ruled out
- A slot-to-slot capture with a longsword (primary to secondary): Aurora puts a longsword dropped on the secondary slot in the primary while that is empty, so the sword never moved; it can't show whether a true slot-to-slot move asks. Rings move between slots but need no feat.

## Watch out
- Aurora moves a weapon dropped on the secondary slot to the primary when the primary is empty; Moonglow doesn't (recorded in docs/deferred.md, seen with one longsword only).
- The blueprint test `equipping_without_the_feat_asks_to_add_it` equips over the bandit's own primary weapon: that weapon now goes to the backpack.
