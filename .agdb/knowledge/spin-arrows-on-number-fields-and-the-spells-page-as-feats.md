---
type: Decision
title: Spin arrows on number fields and the Spells page as Feats
tags: []
status: stable
generated: { by: trail, at: 2026-10-26T00:00:00Z }
touches: [crates/mg-rules/src/lib.rs, crates/mg-rules/src/spell_warnings.rs, crates/mg-ui/src/area_props.rs, crates/mg-ui/src/blueprint/creature.rs, crates/mg-ui/src/blueprint/creature_lists.rs, crates/mg-ui/src/blueprint/encounter.rs, crates/mg-ui/src/blueprint/item.rs, crates/mg-ui/src/blueprint/mod.rs, crates/mg-ui/src/blueprint/situated.rs, crates/mg-ui/src/blueprint/sound.rs]
fingerprint: v2-125dd8b5a1ac6869
created: 2026-10-26
author: August
description: "Decides how number fields get Aurora-style spin arrows (widgets::spin_with, SpinRun, one undo step per run) and how the Spells page shows class spells and list entries by metamagic with per-level slot counts."
asks:
  - "How do I add Aurora-style up/down spin arrows to a number field in a blueprint editor form?"
  - "Why did holding a spin arrow stop after about 0.8 seconds with a plain egui Button?"
  - "Each click or held arrow run should be a single undo step; how is that implemented?"
  - "How does the Spells page count 'Level N: n of max' slots and remove a metamagic entry at a raised level?"
---
## Decided
- `widgets::spin_with` / `spin_number` (generic over egui Numeric) is the number field with Aurora's arrows: a click, Up/Down in the focused field and a held arrow (repeat after SPIN_DELAY 0.4 s, every SPIN_RATE 0.05 s) build a `SpinRun` kept in temp data under the field's id; the change is made when no arrow or key is held, so each click, key press or held run is one undo step. `Form::spin`, `Form::spin_float`, `Form::millis(.., step)` wrap it; module_props' `number` uses it.
- Which fields get arrows: those with `spin a…b` in docs/parity/aurora-ui-inventory.md for the blueprint editors, Module and Area Properties. Wizards, Adjust Location, Options, Script Wizard left (deferred.md).
- `GameData::spell_slots(creature, class entry)` (mg-rules) is what Aurora's spell warnings count per level; `spell_warnings` is built on it, and the Spells page shows "Level N: n of max".
- Spells page: left the class's spells (classes.2da SpellTableColumn), right the actual list entries grouped by (Spell, SpellMetaMagic) (`spell_groups`), so a metamagic entry kept at its raised level shows and can be removed. Counts change by copies of the group's first entry (`set_count`, which `set_uses` now wraps).

## Tried and ruled out
- A plain egui Button for the arrows: egui drops a press held past the click time (max_click_duration), so `is_pointer_button_down_on` turns false after ~0.8 s and the run ended early. The arrows sense click_and_drag.
- Escape in a kittest after focusing a number field closed the blueprint's page from the test's view; the test doesn't press it.

## Watch out
- egui_kittest runs one frame per queued event and advances time 0.25 s a frame: a press, `run_steps(5)`, release is a 1.25 s hold.
- nw_mumcleric's ClassList[0] is an undead class; the cleric (class 2) is second.
- Aurora's Meta-Magic combo on the Spells page was not seen (the Aurora lock was busy): the inventory reads it two ways.
