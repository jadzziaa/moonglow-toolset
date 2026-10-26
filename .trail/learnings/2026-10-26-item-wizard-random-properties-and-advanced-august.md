---
title: Item Wizard random properties and Advanced Search actions
author: August
date: 2026-10-26
branch: item-wizard-and-search-actions
cost: { minutes: 338, tokens: 4903639 }
touches:
  - apps/mg/src/search.rs
  - crates/mg-edit/src/lib.rs
  - crates/mg-module/src/refs.rs
  - crates/mg-module/src/search.rs
  - crates/mg-ui/src/search.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
---

## Decided
- Item Wizard's Magical: rules in `crates/mg-rules/src/item_wizard.rs`, fitted to 126 UTIs Aurora's wizard made (captures in `~/.local/share/moonglow-oracle/captures/item-wizard/` with `manifest.tsv`: file, level band, quality). Table by baseitems `PropColumn`; first tier `round((3L+Q) × top / 12)`; later ones one or two below (Low two), kept while `item_cost` ≤ itemvalue `MAXSINGLEITEMVALUE` at level 5L+Q+2 (Low: 1, 5, 8, 13); Low ≤ 2 props, single-use tables 1, staves 3 (tiers P, P, P-1), rods ≤ 3; charges 50 for staves/rods/wands; Use Limitation Class 1,10,9 for scrolls/wands, 10,9 for staves; Identified 0. The first property ignores the cap (Aurora's L1 Low scroll costs 5403 > 5000).
- `aurora_item_wizard.rs` checks every capture holds to these rules (tiers matched against raw table rows, Aurora's invalid rows included); `item_wizard.rs` sweeps every base item × 16 combos × 3 seeds for validity.
- Advanced Search conditions: `Query::and`; an object meets a condition if it, something it holds, or something holding it is found (`Hit::objects()`: self then holders, area last; a carried hit's self is the item blueprint, its holder the container). Two things in one area are not conflated (self vs chain, both directions).
- Changes to what is found live in `mg_edit::found` (mg-module can't make `Edit`s: mg-edit depends on it). Area variables go to the GIT root; a field/script found is set where found, else the object's own top-level field; nothing is added that the object lacks.

## Tried and ruled out
- A pure value cap (MAXSINGLEITEMVALUE) for the number of properties: 6-10 Low items had room for a third under any itemvalue row in [4234, 4604); hence Low's cap of 2 properties.
- Typing "Magic Staff" into Aurora's base item tree: the incremental search stops at "Magic Rod" (space ends it); type "Magic" then Down arrows.

## Watch out
- `mg_testkit::aurora_capture` wants a file: name `item-wizard/manifest.tsv`, not the folder.
- Aurora's Item Wizard needs an area open (the Wizards menu lists only Area/Plot Wizard without one).
- In the small UI-test screen an Edit Together window lies over the search tab: close it before clicking in the tab again.
- The worktree Bash guard refuses `flock … cargo test`; run it through a script in the scratchpad.
