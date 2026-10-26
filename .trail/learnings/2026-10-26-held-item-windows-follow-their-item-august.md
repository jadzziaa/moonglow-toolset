---
title: Held item windows follow their item; Equipment in a narrow window
author: August
date: 2026-10-26
branch: inventory-window-held-items
cost: { minutes: 871, tokens: 9379390 }
touches:
  - crates/mg-edit/src/lib.rs
  - crates/mg-ui/src/blueprint/creature_lists.rs
  - crates/mg-ui/src/blueprint/inventory.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/05-blueprints.md
---

## Decided
- A held item has no identity of its own (its struct id is its list position), so its Properties follow it by translating the window's path through the edits about to run: blueprint::follow::follow (pure; insert/remove before it shift the index, a removal of it followed in the same command by an insert of an equal struct into the same list is a move). Worked out before the edits run (followed: needs the document as it was, for the struct a removal takes) and applied after (retarget) in Moonglow::apply and in the Undo/Redo arm, which reads the next command with the new Workspace::peek_undo/peek_redo.
- retarget rewrites Tab::Instance/Instances/InstanceModel paths in place, moves the blueprint_pages entry and the properties_changed entry (its key, its tab and its Was::Field paths, else Cancel sets back the item now at the old place), and closes windows whose object is gone. It covers top-level placed objects too.
- The creature Inventory's Equipment is rows, not an egui::Grid: one line a slot (name column, icon, name, Equip, Remove) when that fits the width measured with layout_no_wrap (SlotLayout), else the slot's name over the item's beside a 40-point icon with Equip over Remove. The palette column is 40% of the page, 200 to 240 points.

## Tried and ruled out
- Truncating the item name alone: at 480 points the side panel has about 200 points and a one-line slot needs about 360; the shape has to change.
- Finding the window's item by content alone: two identical items (two torches) can't be told apart; the path through the edits can.

## Watch out
- A dock tab's title is not in the accesskit tree: tests check what a Properties window shows by a TextInput's value (its Tag field).
- Equip, To Backpack and drags between objects make the item again from its blueprint (inventory_item, held_item): the held item's own changes are lost and its window closes (recorded in docs/deferred.md).
- side_panel's right margin already clears its scroll bar: rows inside it take ui.available_width(); item_list subtracts a gutter for its own nested ScrollArea.
