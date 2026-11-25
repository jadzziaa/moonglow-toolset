---
type: Decision
title: "Several inventory items carried at once: the rules"
description: "How a drag, Copy, Cut and Paste of several selected inventory items work (order, slot drops, step names, selection in chests and stores) and the pitfalls met building it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T20:12:42Z }
touches: [crates/mg-ui/src/blueprint/inventory.rs, crates/mg-ui/src/blueprint/creature_lists.rs, crates/mg-ui/tests/app/inventories.rs]
asks:
  - "What happens when several selected inventory items are dropped on a creature's equipment slot?"
  - Why does a Dragged inventory payload have a more field and how are removals ordered?
  - Why did menu_of in the inventory tests stop finding Equip after Cut was added to the row menu?
  - Does Aurora let you drag several inventory items at once?
fingerprint: v2-21dd6ab68a714704
created: 2026-11-25
author: augustaugur
---
## Decided (2026-11-25)
- Aurora selects one inventory item (meSelectedName): no multi-carry to match; Moonglow's is its own extension.
- A drag, Copy or Cut of a row carries the whole selection only where that row is selected (else the row alone). Order: a creature's slots (Equip_ItemList) first, then the list, each by index (`Carried::order`). Payload: `Dragged::Listed/Equipped` keep the lead item and gain `more: Vec<Carried>`; `carried()` gives all in order, `removals()` sorts by list then descending index so sequential RemoveItem edits stay valid (and `after_taken` too).
- Placeable contents and store pages now keep a selection too (`inventory::selected_rows/select_rows`, cleared on a same-length reorder). The creature passes its equipped selection as `ItemLook::also`.
- Onto a list: all in, `add_items` placing each after those before. Own rows onto an own row: move before it together. Own items with slots' ones onto own backpack: those slots' items stowed (`stow_all`).
- Onto a slot (count > 1 only; the single path is untouched): the lead if it fits the slot under the pointer, else the first that fits; then the existing secondary-to-primary bump and feat question run. From another object the rest go to the backpack after the displaced slot item (`Moving::along`); the creature's own others stay. None fits: log error "None of the N items goes in that slot", nothing changes.
- `Moving` got `gone` (the item each removal takes) so `still_there` checks each removal against its own item; with one item compared to all, the feat question vanished the next frame.
- Step names: one item keeps its old name ("Add item", "Move item", "Add store item", "Equip"); several say how many: "Move 3 items", "Paste 3 items", "Add 3 items", "Cut 3 items", "Unequip 2 items to backpack" (`inventory::adding_name`).
- The count label ("3 items") is a non-interactable Order::Tooltip Area offset from the pointer, drawn once per pass from `item_list` (`show_count`); non-interactable areas are skipped by egui's layer_id_at, so drops still land.

## Watch out
- Adding Cut made row menus taller; near a window's foot a menu opens above the row, and the old test helper (nearest label below the row) then picked nothing or a slot's Equip button. `menu_of_at` now picks the label that was not there before the menu opened.
