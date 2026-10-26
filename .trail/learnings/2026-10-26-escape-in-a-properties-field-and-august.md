---
title: "Escape in a Properties field, and Aurora's Spells page"
author: August
date: 2026-10-26
branch: spells-escape-followup
cost: { minutes: 27, tokens: 38432 }
touches:
  - crates/mg-ui/src/blueprint/creature_lists.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/tabs.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/02-main-window.md
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
