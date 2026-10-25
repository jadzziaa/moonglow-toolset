---
title: Inventory flags and the classic clipboard keys
author: August
date: 2026-10-25
branch: inventory-flags-insert-keys
cost: { minutes: 837, tokens: 7777757 }
touches:
  - "crates/mg-ui/src/widgets.rs#clipboard_keys"
  - "crates/mg-ui/src/blueprint/creature_lists.rs#item_options"
  - "crates/mg-module/src/instances.rs#instance"
  - "crates/mg-ui/tests/app.rs#press_now"
---

## Decided
- Dropable and Pickpocketable were already in the creature Inventory (since 0.1.0) but drawn only once an item was clicked: Sacha could not find them. They are now Aurora's Options under the palette (inside palette_picker_over's below closure, so the tree leaves room), always drawn, grayed with nothing selected. The label stays Aurora's one-p `Dropable` (its caption and the GFF field; the script functions spell Droppable).
- Selection is a BTreeSet of (list, index) through widgets::select_in_list (listed: equipment in slot order, then backpack), let go when either list's length changes. A box over several items is one Command; inventory::flag_edit returns None for an item already so, keeping its bytes (an explicit 0 stays).
- Ctrl+Insert, Shift+Delete, Shift+Insert: widgets::clipboard_keys at the top of Moonglow::ui replaces the Key events with Event::Copy/Cut (and RequestPaste for Shift+Insert), so every handler that reads egui's events gets them and Shift+Delete never reaches key_pressed(Delete) (which ignores modifiers: area view, module tree). Hard-coded, not keymap entries: keys.rs leaves the platform's Copy/Cut/Paste out by design.

## Watch out
- egui-winit 0.36 maps these keys to Copy/Cut/Paste on Windows only (cfg!(target_os = "windows")); elsewhere, and always in kittest, they come as Key events.
- kittest gives each queued event its own frame and h.output() is the last frame's, so a copy's CopyText or a RequestPaste is lost after key_press_modifiers: push ModifiersChanged and the Key into h.input_mut().events and h.step() once (press_now in tests/app.rs). kittest ignores RequestPaste: send the Event::Paste yourself.
- instances::instance expanded an equipped item from EquippedRes and dropped the entry's other fields (Dropable, Pickpocketable never reached the GIT); add_to_palette dropped them the other way. engine_item_flags caught it. Aurora appends the flags after the item's fields, as Struct::set does.
