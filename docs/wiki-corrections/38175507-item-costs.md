---
type: Wiki Edit
title: "itemprops.2da (Item Costs): the charge table, the two dearest spells, charges, totals and creation"
description: Ready-to-paste edit for the Item Costs section of the nwn.wiki itemprops/itempropdef/iprp_costtable/iprp_paramtable page, matching how the engine prices items.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38175507/itemprops.2da+itempropdef.2da+iprp_costtable.2da+and+iprp_paramtable.2da
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38175507/itemprops.2da+itempropdef.2da+iprp_costtable.2da+and+iprp_paramtable.2da
    title: "nwn.wiki: itemprops.2da, itempropdef.2da, iprp_costtable.2da and iprp_paramtable.2da, version 25"
---

# itemprops.2da, itempropdef.2da, iprp_costtable.2da and iprp_paramtable.2da

https://nwn.wiki/spaces/NWN1/pages/38175507/itemprops.2da+itempropdef.2da+iprp_costtable.2da+and+iprp_paramtable.2da, edit written against version 25. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

Item Costs: iprp_chargecost.2da; which charge rows scale; the engine's two-highest pass; truncation, AddCost and stacks; items are priced anew when created.

## Change 1
- **Find:** "Get the Cost field value in iprp_charges.2da relatd to the item property cost table value"
- **Replace with:** "Get the Cost field value in iprp_chargecost.2da related to the item property cost table value"

## Change 2
- **Find:** "If the charges are per-use (1 - 5 charges per use) then multiply the charges left based on a default of 50."
- **Replace with:** "If the charges are per-use (iprp_chargecost.2da rows 2 - 6, 5 down to 1 charges per use) and the item has charges left, then multiply the charges left based on a default of 50. Single Use, 0 Charges/Use and the uses-per-day rows are never scaled."

## Change 3
**Add after** "With the above calculation the two highest are also added on again, the most expensive having half added on, and the second most at a quarter.", as a new sub-point:

> Note the engine finds the two highest in one pass in property order, and the pass has a bug: a cost above the current highest becomes the highest without the old highest becoming the second; otherwise a cost above the current second becomes the second. So the second's extra quarter only counts if it comes after the most expensive spell. Sixteen base game items are priced lower than a true top two would give (x0_it_mneck002 is 6526, not 7538). The example below, with three equal spells, comes out the same either way.

## Change 4
- **Find:** "For each of these values the float values are converted back to a uint32_t or a positive-only integer."
- **Replace with:** "All of this is done in single-precision floats. The positive price is truncated in two parts, trunc(Base Cost + Active Cost) + trunc(Positive Multiplier ^ 2 * 1000), and the negative price is trunc(Negative Multiplier ^ 2 * 1000)."

## Change 5
- **Find:** "Then add on the Additional Cost set on the items GFF (a UNIT32 field it can only be positive)"
- **Replace with:** "Then truncate, add on the Additional Cost set on the items GFF (a UNIT32 field it can only be positive) once per item, and multiply by the stack size: (trunc((Positive - Negative) * ItemMultiplier) + AdditionalCost) * StackSize. Scripts (GetGoldPieceValue) get 0 for plot items, creature items and gold, and otherwise at least 1."

## Change 6
**Add before** "The item was identified" (the list after "The game doesn't recalculate it dynamically...unless..."), as a new list item:

> The item is created: GetGoldPieceValue straight after CreateObject returns the calculated value, not the Cost stored in the blueprint (which is out of date in about 9% of the base game's blueprints)

## How it was checked

`engine_item_cost.rs` `item_costs_match_the_engine` in Moonglow Toolset's `crates/mg-corpus-tests/tests/`: every base game item (2,763) created with CreateObject in the game's own server (`nwserver`, started privately), its GetGoldPieceValue equal to the formula's, plus made-up wands for the charge rows. Negative costs and no charges left are covered only by unit tests (no base item has them); placed items and store stock were not probed. The tests read the tester's own game install.
