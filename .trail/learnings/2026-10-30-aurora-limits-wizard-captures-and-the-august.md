---
title: "Aurora limits, wizard captures and the picker, read offline"
author: August
date: 2026-10-30
branch: worktree-agent-a46d68247cce27a96
cost: { minutes: 2213, tokens: 14127633 }
touches:
  - crates/mg-ui/src/blueprint/encounter.rs
  - crates/mg-rules/src/item_wizard.rs
  - crates/mg-corpus-tests/tests/aurora_item_wizard.rs
  - crates/mg-ui/tests/frame_perf.rs
  - crates/mg-ui/src/blueprint/picker.rs
---

ruled out: reading an Aurora field's limits from its form (~/.local/share/moonglow-oracle/uiinv/forms.json, a TUpDown's Min/Max): the forms are not the code. The encounter's up-downs say Min 0 where Aurora keeps 1 to 8; the fog clip's say 25..999 where it keeps 30..9999. A limit is measured by typing past both ends in Aurora (the item property's Chance of Appearing still waits on that).
decided: the Item Wizard's captures (captures/item-wizard/) include base items Aurora does not make magical (the box can't be ticked): they come out Identified 1 with no properties. Magical is offered exactly where Aurora's item is Identified 0 (gems and gauntlets too, which get no properties); aurora_item_wizard.rs holds item_wizard::offered to that.
watch out: frame_perf's drag of a selection (drag_selection) presses one selected object: with several selected, the press at an object's feet can land on another's turning ring and Rotate instead of Move; the test asserts the Move. area092 has no pickable object in sight close up.
watch out: the blueprint picker's search is kept in the palettes' Kept by (kind, custom) with the palette's Arc and the search text; a palette rebuilt is a new Arc, so it is found again.
watch out: a worktree agent's Bash refuses heredocs, loops over a variable, a variable as a sed file operand and $(...) in a cargo argument; use Edit/Write, or a Python script in the scratchpad that calls subprocess.
