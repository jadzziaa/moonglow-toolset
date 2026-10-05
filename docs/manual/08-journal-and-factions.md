---
type: Manual Page
title: Journal and factions
description: The Journal Editor and the Faction Editor.
tags: [manual, journal, factions]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-05T19:05:31Z }
---

# Journal and factions

## The Journal Editor

**Tools › Journal Editor** (Ctrl+Alt+J) edits the module's journal
(`module.jrl`): the quests (categories) players see in their journal, and
each quest's entries.

The tree lists the quests, closed to begin with as in Aurora: the arrow
before one (or a double click) opens it out to its entries, and **Expand
All** and **Collapse All** do so for all. **Add**, **Copy**,
**Cut**, **Paste** and **Delete** work as in Aurora: Add under a quest
adds an entry, elsewhere a quest. Select a node to edit it:

- A quest: its name, tag (how scripts and conversations refer to it),
  priority, the experience it gives, and a comment.
- An entry: its ID (what scripts set), its text, and **Finish Category**
  (the entry ends the quest).

Conversations set journal entries from a line's **Other Actions** tab;
scripts with `AddJournalQuestEntry`.

## The Faction Editor

**Tools › Faction Editor** (Ctrl+Alt+F) edits the module's factions
(`repute.fac`): the five standard ones (PC, Hostile, Commoner, Merchant,
Defender) and the module's own.

The table shows how each faction regards every other, from 0 (hostile) to
100 (friendly); 11 to 89 is neutral. Edit a value to change it. A
column is as wide as its numbers: a long name is cut short in its
heading and shown whole when the pointer rests on it. **Add
Faction…** makes a faction that starts with a parent's attitudes;
**Change Name…** and **Remove Faction** change the module's own.
**Global Effect** makes a faction act as one: when a player angers one
member, all the others turn too.

Creatures, doors, placeables, triggers and encounters belong to a faction
(their Basic page).
