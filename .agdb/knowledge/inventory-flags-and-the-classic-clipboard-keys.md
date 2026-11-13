---
type: Decision
title: Inventory flags and the classic clipboard keys
tags: []
status: stable
generated: { by: trail, at: 2026-10-25T00:00:00Z }
touches: ["crates/mg-ui/src/widgets.rs#clipboard_keys", "crates/mg-ui/src/blueprint/creature_lists.rs#item_options", "crates/mg-module/src/instances.rs#instance", "crates/mg-ui/tests/app.rs#press_now"]
created: 2026-10-25
author: August
description: "Dropable and Pickpocketable flags are now always-visible Options in the creature Inventory (grayed with no selection, multi-select as one Command), and Ctrl+Insert, Shift+Delete, Shift+Insert are hard-coded as copy, cut, paste via widgets::clipboard_keys, with test pitfalls."
asks:
  - "Why can't I find the Dropable and Pickpocketable checkboxes in the creature inventory, and where do they appear now?"
  - "How do I make Shift+Delete cut instead of deleting in the area view or module tree?"
  - "My kittest copy or paste test loses CopyText or RequestPaste after key_press_modifiers; how do I send the keys properly?"
  - "Why were Dropable and Pickpocketable flags missing from the GIT for equipped items, and what fixed it?"
---
## Decided
- Dropable and Pickpocketable were already in the creature Inventory (since 0.1.0) but drawn only once an item was clicked: Sacha could not find them. They are now Aurora's Options under the palette (inside palette_picker_over's below closure, so the tree leaves room), always drawn, grayed with nothing selected. The label stays Aurora's one-p `Dropable` (its caption and the GFF field; the script functions spell Droppable).
- Selection is a BTreeSet of (list, index) through widgets::select_in_list (listed: equipment in slot order, then backpack), let go when either list's length changes. A box over several items is one Command; inventory::flag_edit returns None for an item already so, keeping its bytes (an explicit 0 stays).
- Ctrl+Insert, Shift+Delete, Shift+Insert: widgets::clipboard_keys at the top of Moonglow::ui replaces the Key events with Event::Copy/Cut (and RequestPaste for Shift+Insert), so every handler that reads egui's events gets them and Shift+Delete never reaches key_pressed(Delete) (which ignores modifiers: area view, module tree). Hard-coded, not keymap entries: keys.rs leaves the platform's Copy/Cut/Paste out by design.

## Watch out
- egui-winit 0.36 maps these keys to Copy/Cut/Paste on Windows only (cfg!(target_os = "windows")); elsewhere, and always in kittest, they come as Key events.
- kittest gives each queued event its own frame and h.output() is the last frame's, so a copy's CopyText or a RequestPaste is lost after key_press_modifiers: push ModifiersChanged and the Key into h.input_mut().events and h.step() once (press_now in tests/app.rs). kittest ignores RequestPaste: send the Event::Paste yourself.
- instances::instance expanded an equipped item from EquippedRes and dropped the entry's other fields (Dropable, Pickpocketable never reached the GIT); add_to_palette dropped them the other way. engine_item_flags caught it. Aurora appends the flags after the item's fields, as Struct::set does.
