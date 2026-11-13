---
type: Decision
title: "Lists edited together and Advanced Search's list changes"
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
touches: [crates/mg-edit/src/lists.rs, crates/mg-edit/src/found.rs, crates/mg-ui/src/blueprint/mod.rs, crates/mg-ui/src/blueprint/inventory_together.rs, crates/mg-ui/tests/app/lists_together.rs]
fingerprint: v2-ce09548079acfe55
created: 2026-11-02
author: August
description: "List edits across several objects go through commands in `mg_edit::lists`, shared with Advanced Search's AddEntry/RemoveEntry, applied via `Form::apply_each` past the `blueprint::partial` guard; includes UI and test pitfalls."
asks:
  - "How do I make Advanced Search's add or remove entry changes follow the same rules as editing lists on several objects at once?"
  - "Why does my kittest click on 'Give to All' in the encounter Creature List do nothing in the Edit Together window?"
  - "Why does `h.run()` exceed its steps when a grid's last column appears or disappears, and how do I fix it?"
  - "How do insert and remove indexes get chosen when editing a list across multiple objects, and which pages still use single-object code?"
---
## Decided
- Lists across several objects edited together (`docs/lists-together-proposal.md`): the commands are built in `mg_edit::lists` (Target = document, path, struct, name; `merged`, `add`, `remove`, `set`, `set_count`, and the rules `set_skill_rank`, `add_class`/`remove_class`/`change_class`, `add_property`, `place`), so Advanced Search's `found::Change::AddEntry`/`RemoveEntry` reach the same rules. Every insert/remove index comes from that target's own list.
- The guard (`blueprint::partial`) stays; a list page pushes its command through `Form::apply_each`, which records the action's index in `Form::together.each`, and `edit_many` lets those through; `together.said` becomes the window's note. `LIST_PAGES` is gone: every page is offered with several.
- Skills, feats, special abilities, classes and spells share one code path for one or several (one target behaves as before). Encounters, restrictions, item properties and inventories keep one object's page as it was (a list there may hold the same entry twice, which a merged list can't show) and branch on `f.several()`.
- Inventories of several: `inventory_together.rs`; equipping per creature reuses `creature_lists::equip_edits` by swapping the form's key/path/root (`as_target`).

## Watch out
- A "Give to All" in the last column of a wide grid (encounter's Creature List) is clipped in the Edit Together window (about 680 px wide): kittest's click lands on nothing and the test sees no change. Put such buttons under the list for the entry chosen.
- Tests set a number field with accesskit's SetValue (`common::set_number`, `number_beside`), a DragValue taking it as typed.
- A grid whose last column comes or goes takes a few frames to settle: `h.run()` exceeds its steps; `run_steps(4)` first.
- In Advanced Search's change panel, a new act's kind resets the entry's row: set the act, run, then the row.
