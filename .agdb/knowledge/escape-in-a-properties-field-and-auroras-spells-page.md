---
type: Decision
title: "Escape in a Properties field, and Aurora's Spells page"
tags: []
status: stable
generated: { by: trail, at: 2026-10-26T00:00:00Z }
touches: [crates/mg-ui/src/blueprint/creature_lists.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/tabs.rs, crates/mg-ui/src/widgets.rs, crates/mg-ui/tests/app.rs, crates/mg-ui/tests/screens.rs, docs/deferred.md, docs/findings.md, docs/log.md, docs/manual/02-main-window.md]
created: 2026-10-26
author: August
description: "Pressing Escape in a Properties field closed the window because egui clears focus in Memory::begin_pass; Moonglow now records text-edit focus in widgets::dialogs_end to keep it open, matching Aurora's Spells page behavior notes."
asks:
  - "Why does pressing Escape while typing in a Properties field close the whole window in Moonglow?"
  - "How can I detect that Escape left a text field in egui when focused() is already none?"
  - "How do I find a ComboBox in egui accessibility tests when get_by_label doesn't match the selected text?"
  - "How does Aurora's Spells page compute Casting Level and which metamagic feats show in Meta-Magic?"
---
## Decided
- egui clears the keyboard focus at Escape in `Memory::begin_pass`, before anything of the frame is drawn, so a window's "nothing is being typed" check (`m.focused().is_none()`) is always true on an Escape frame: Properties windows closed on Escape in a field. `widgets::dialogs_end` (end of `Moonglow::ui`) records `ctx.text_edit_focused()`; `escape_left_text` says the frame's Escape left a field, and tabs.rs then keeps the window. Nothing typed was lost before: numbers commit as typed and `held_text` sets a closed window's text.
- Aurora (watched): Escape in a Creature Properties field is Cancel (typed Tag gone). Moonglow keeps its own (leave the field; a second Escape closes keeping changes); recorded in deferred.md for the builders.
- Aurora's Spells page (watched): Meta-Magic = None + the creature's metamagic feats (feat.2da 11, 12, 25, 29, 33, 37); off for known-spell classes; Casting Level = level + metamagic_levels, not capped (Aura vs Alignment 10); spells.2da MetaMagic not read; counts per (spell, metamagic); Spell Level filters by base level; summary "N x Name (Empower Spell)"; domain spells outside the class column not listed.

## Tried and ruled out
- Clicking creatures in Aurora's top-down area view: the pick misses; pitch the camera down (the blue down-arrow under the view, six clicks after four zoom-ins) and right-click the body, after which the module tree lists the area's creatures (right-click one there, Properties).
- `h.get_by_label("None")` for an egui ComboBox: the combo's accessible value, not its label, is the selected text; query `Role::ComboBox` by value.

## Watch out
- Another agent's sandboxed game client can open on the same off-screen display over Aurora: check `xdrive windows` before clicking and wait for it to go.
- Aurora's Delphi tab rows swap when a tab of the other row is clicked: take a fresh window shot before each tab click.
