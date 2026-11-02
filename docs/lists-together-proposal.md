---
type: Proposal
title: 'Lists across several blueprints: a design'
description: How Edit Together edits the pages of lists (skills, feats, special abilities, an encounter's creatures, a store's restrictions, classes, spells, item properties, inventories) on several blueprints or placed objects at once - a list shown merged with an entry some have marked ≠ and how many, what adding, removing and changing an entry does to each, the guard that dropped a change reaching some of them and how it becomes explicit, one Undo step, the rules' limits (room for a class, levels and spells per class, item properties matched by type, subtype and value), and Advanced Search's changes to what it found through the same commands.
tags: [proposal, bulk-editing, edit-together, blueprints, advanced-search]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-02T09:50:11Z }
sources:
  - id: deferred
    resource: deferred.md
    title: 'The deferred list: "Lists across several blueprints" (Bulk editing), the 6th large entry after 3.0.0, and the first item of "Advanced Search, what it does not do"'
  - id: code
    resource: crates/mg-ui/src/blueprint/mod.rs (edit_many, partial, LIST_PAGES), creature.rs, creature_lists.rs, encounter.rs, store.rs, item.rs, inventory.rs, placeable.rs; crates/mg-edit/src/found.rs (read 2026-11-02, at 3.2.0)
    title: The code this is built on
---

# Lists across several blueprints: a design

Edit Together (the palette's **Edit N Together**, the area's several
objects of one kind, Advanced Search's **Edit the N ticked together**)
sets a field on each blueprint or placed object it edits, and marks a
field where they differ with **≠**. It left out the pages that edit
lists of the object's own (`LIST_PAGES`): each built its commands for
the first object alone, and `blueprint::partial` dropped a command that
reached some of the objects but not all, saying "That change can only be
made to one blueprint at a time".[^code] The deferred list kept what
each page needs.[^deferred] Aurora has no Edit Together, so this is
Moonglow's own design. Blueprints and placed objects are edited alike:
each is a document and a path in it (placed objects share their area's
GIT).

## The rules every page follows

- **A list shown merged.** The page lists every entry any of them has,
  in the first one's order and then the others' (an entry is told apart
  by its key, per page below). An entry all of them have shows as it
  does for one. An entry some have is marked **≠** with how many: "≠ 2
  of 3", and its tip says which part it is ("2 of the 3 have it"). A
  value that differs among those that have the entry (a rank, a level,
  uses) is marked ≠ as a field is, and shows the first one's.
- **Adding** an entry adds it to each that lacks it, at the end of its
  own list (a skill at its index), and leaves those that have it alone.
  A **≠** entry offers the same as **Give to All**: added where it
  lacks.
- **Removing** an entry takes it from each that has it (all its copies:
  a spell prepared three times, a feat listed twice).
- **Changing** an entry's value sets it on each that has it.
- **One step.** What a click does to all of them is one command, named
  as for one ("Add feat"), which Undo takes back whole.
- **The rules' limits.** Where a rule refuses an entry to one of them
  (no room for a class), that one is left as it is, and the window's
  note says so: "Added to 2; left: mg_b (8 classes already)".
- **Indices are each one's own.** Every insert and removal is built from
  that object's list, not the first's; removals from one list go from
  the last.

## The guard

`partial` stays: it still catches a command a page builds for the first
object alone (an item one placed object holds, its Properties). A change
that reaches some of the objects but not all is now the point of a list
page (a feat added to the two that lack it), so it is made explicit,
not dropped: a list page pushes its command through
`Form::apply_each`, which notes it as built for each, and the guard
passes the commands so noted. Any other command that reaches some but
not all is dropped as before, with its note. The pages are taken out of
`LIST_PAGES` as each lands; the note under the window lists those left.

## Page by page

| Page | Entry's key | Add | Remove | Values set on each |
| --- | --- | --- | --- | --- |
| Skills | the skill's row (`SkillList` is indexed by skill) | — | — | the rank, entries up to it added where the list is short |
| Feats | `Feat` | where it lacks | from each having it | — |
| Special Abilities | spell, caster level, ready (as one creature groups its uses) | one use, at each creature's own level (its classes' levels) | from each having it | uses, caster level, ready |
| Creature List (encounter) | the creature blueprint (`ResRef`) | where it lacks | from each having it | Unique (`SingleSpawn`) |
| Restrictions (store) | the base item | to the list of the mode shown | from that list | the mode (below) |
| Classes | the class row | at level 1, where there is room | from each having it, but not its only class | the level; a cleric's domains, a wizard's school; the class itself changed (not where it has that class already) |
| Spells | per class (by its row, not its place in `ClassList`), then spell and metamagic in a level's list | known: where it lacks; prepared: one more on each | from each having it | how many prepared |
| Properties (item) | type, subtype, cost value and parameter value | where it lacks and its base item takes the type | from each having it | the parameters, on each one's matching property |
| Inventory | the item blueprint | one on each, at its own first free place (a store: the page of its kind) | every one of it, from each | Droppable, Pickpocketable, a store's Infinite |

Details the table leaves out:

- **Feats.** A feat's box is ticked where all have it and half-ticked
  (≠) where some do; a click on a half-ticked one gives it to the rest,
  on a ticked one takes it from all. The Assigned column lists every
  feat any has, with ≠ and how many, × taking it from those that have
  it.
- **Special Abilities.** The uses shown are the first one's that has
  it, ≠ where the others' differ; a number typed sets that many uses on
  each that has the ability (more are copies of its first use).
- **Restrictions.** A store either will not buy the base items listed
  (`WillNotBuy`) or will only buy them (`WillOnlyBuy`). Stores in
  different modes show the first one's mode, marked ≠; adding goes to
  that mode's list on each store in that mode or with none (a store in
  the other mode is left, and said: its list there would be the one its
  mode ignores), removing takes from that list on each. Clicking a mode
  moves each store's items to it where it is in the other mode, as one
  store's do.
- **Classes.** A creature takes up to 8 classes; one with 8 is left
  without the class added. A creature keeps one class at least: Remove
  leaves the class of one that has no other. The Levelup Wizard levels
  one creature at a time and is grayed with several. Alignment, the
  package and the familiar and companion are fields, set on each.
- **Spells.** The class buttons are the caster classes any of them has,
  ≠ and how many where some have it; the page edits that class's lists
  on each that has it. A known spell's box is half-ticked where some
  know it. The count of a prepared spell is the first one's, ≠ where
  they differ; typed, it is set on each (those that had none prepare
  that many). A level's heading counts the first one's and is marked ≠
  where the others have other counts; the spell warnings are each
  creature's, named. Meta-Magic offers the metamagic feats all of them
  have. Clear and Load Class Spell List do each one's lists; Save
  saves one creature's and is grayed with several.
- **Properties.** Properties are matched by type, subtype and value: an
  Enhancement Bonus +1 and +2 are two entries, and adding +1 to an item
  with +2 gives it both, as adding it to one item would. The properties
  offered are those the first one's base item takes; an item whose base
  item doesn't take the type is left, and said. Total Cost is the
  first's.
- **Encounters, restrictions, item properties and inventories** keep
  one object's page
  as it was for one: a list there can hold the same entry twice, which
  a merged list can't show.
- **Inventory.** The list is of item blueprints, each with how many of
  them hold it; dragging rows, Copy and a held item's own Properties are
  for one object at a time (dropping an item from the palette, and
  pasting, add it to each). A creature's equipment: a slot shows the
  item all of them wear there, or ≠; an item equipped there from the
  palette is worn by each (what each wore there goes to its backpack, as
  for one), To Backpack takes each one's to its backpack and Remove
  takes it away. The question one creature is asked when it lacks the
  feat an item needs is not asked of several.

## Advanced Search: lists changed on what it found

Advanced Search changes what it found in one step (a variable, a
script, a field). Its changes gain list entries, built by the same
functions as the pages (`mg_edit::lists`), so the rules above hold
there too:

- **Add a feat** and **Remove a feat** (creatures found);
- **Add a class** at a level, **Remove a class** (creatures found:
  where there is room; not a creature's only class);
- **Add an item property** and **Remove an item property** (items
  found: the property's type, subtype and value chosen as a condition's
  are; added where the base item takes it and the item lacks it;
  removed where it has it so);
- **Add an item** and **Remove an item** (creatures, placeables and
  stores found: one of the item blueprint at its first free place;
  every one of it).

What each leaves, and why, is said as the other changes say it ("mg_b:
has it already"). Objects of another kind found are left and said
("door DOOR_01: no feats").

## What was built

All nine pages, in the order the deferred list gave (2026-11-02):
`mg_edit::lists` (the merged list, add, remove, set, a count, and the
rules: a skill's rank, feats, classes, item properties, an item's
place), `Form::apply_each` and the guard in `blueprint::edit_many`,
the pages in `creature.rs`, `creature_lists.rs`, `encounter.rs`,
`store.rs`, `item.rs` and `inventory_together.rs`; Advanced Search's
`found::Change::AddEntry` and `RemoveEntry`. Tested in
`tests/app/lists_together.rs` (a page each: several blueprints,
entries some have, one Undo) and `found.rs`'s unit tests; pictured by
`screens.rs`'s `lists_together`.

## What is not in it

Edit Together still takes blueprints of one type, and objects of one
kind placed in one area. Moving items between the objects edited
together, and opening one of their held items, are for one object at a
time: which one's item would be meant.

[^deferred]: The deferred list's "Lists across several blueprints" and "Advanced Search, what it doesn't do".
[^code]: The code this is built on.
