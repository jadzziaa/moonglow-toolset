---
type: Manual Page
title: Journal and factions
description: The Journal Editor and the Faction Editor.
tags: [manual, journal, factions]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-25T08:44:45Z }
---

# Journal and factions

## The Journal Editor

**Tools › Journal Editor** (Ctrl+Alt+J), or a double click on
`module.jrl` under **Journal and Factions** in the module tree, opens the
module's journal in a tab of its own: the quests (categories) players
see in their journal, and each quest's entries.

The editor's header holds **Add** with its word, then **Copy**, **Cut**,
**Paste** and **Delete**, **Expand All** and **Collapse All** as icons
(each named in its tip), with **OK** and **Cancel** at its end: OK keeps
what was done and closes the editor, Cancel sets back everything done
since it was opened (what was changed in the Right Dock's panels too)
and closes it. With a quest chosen, Add adds an entry to it, and with
an entry chosen, an entry to its quest, at the end; with nothing chosen
(or Root), a quest. The new one is chosen. Delete chooses what took the
deleted one's place, the last where it was the last: an entry of the
same quest (the quest itself where it has none left), or a quest.
Paste puts a copied quest at the end, a copied entry into the quest
chosen. The tree lists the quests under **Root**, each with its tag in
brackets and its count of entries, closed to begin with as in Aurora:
the arrow before one (or a double click) opens it out to its entries,
each with its ID (`[0030]`) and the start of its text, and Expand All
and Collapse All do so for all. An entry that ends its quest carries an
**End** badge. A quest's name or an entry's text that is a talk-table
string (as the original campaign's are) shows that string.

Choose a node to edit it in the Right Dock:

- **Entry** (an entry chosen; its title says "[0030] of M1Q5_Bertrand",
  the entry's ID and its quest's tag): its ID (what scripts set),
  **Finish category** (the entry ends the quest) and its Text, in the
  language edited, under a heading naming that language (with the
  string's number where the text is the talk table's, and the **…**
  button for the other languages).
- **Category** (a quest, or the quest of the entry chosen; its title
  says the quest's tag): its Name, Tag (how scripts and conversations
  refer to it), Priority, XP (the experience it gives) and Comments.

With nothing for it to show, a panel says what to choose. Under 1400
points of window the two are tabs of one bar. A journal in a window of
its own has its Entry and Category in the window's right pane.

A number (ID, XP) is set as you type it. A text is set when you leave
its field: Name and Tag also by Enter, while in Text and Comments Enter
starts a new line (leave the field, or choose another node, to set
them). What you type stays with its quest or entry: if the journal
changes meanwhile (an entry added, cut or deleted, an Undo, the module
saved), it is set on the one you typed into. Where the whole journal is
written by something else while you type (a plugin, a module-wide Find
and Replace, a rename of what it refers to), the quest is found again by
its tag and the entry by its ID; where that can't tell which it was (two
entries with one ID, as a pasted copy has), what you typed is dropped
rather than set on another. While a text waits to be set, **Redo** is
not offered (the Top Bar, the Edit menu and the key alike): leave the
field first. Each change is one step of **Edit › Undo**. The Status Bar
counts the quests and entries, and says what a click and a double click
do over the tree.

Conversations set journal entries from the **Journal** row of a line's
**Other Actions** (the Node section's Data tab: see
[a line's Data](06-conversations.md#a-lines-data)); scripts with
`AddJournalQuestEntry`.

## The Faction Editor

**Tools › Faction Editor** (Ctrl+Alt+F), or a double click on
`repute.fac` under **Journal and Factions** in the module tree, opens the
module's factions in a tab of its own: the five standard ones (PC,
Hostile, Commoner, Merchant, Defender) and the module's own.

The header holds the switch **Basic | Advanced**, **Add Faction…**,
**Remove Faction** and **Change Name…** (the last two off for a standard
faction), with **OK** and **Cancel** at its end (Cancel sets back
everything done since the editor was opened, a removal's renumbering of
the module's objects too).

**Advanced** shows the matrix: how each faction (a row) regards every
other (a column), from 0 (hostile) to 100 (friendly); 11 to 89 is
neutral, and a hostile cell is framed red, a friendly one blue. The row
and column headings stay in place as it scrolls. Click a cell to choose
it; then the arrow keys move, a number typed and Enter set the value,
Escape stops typing (nothing is set), and Tab sets what was typed as
Enter does. The matrix keeps the keys (Tab does not leave it): **F6**
goes on to the next pane. Drag a cell sideways to change it, or
double-click it to type. **Basic** shows a chart of how the chosen
faction regards each of the others, a slider for each; its **Full
detail** at the foot shows how each of them regards it too. In Basic
the faction is chosen in the Outliner (with the Left Dock folded to its
strip, the strip's Outliner button opens it); a click on a name in the
chart chooses the other faction, whom the Reaction panel speaks of.

The Right Dock follows the choice:

- **Faction**: the chosen faction's Name and Parent (read-only: Change
  Name… renames a faction of the module's own) and **Global effect**,
  which makes a faction act as one: when a player angers one member, all
  the others turn too (off for a standard faction, whose setting is the
  game's).
- **Reaction**: the chosen cell in words ("Guards is hostile toward
  Commoner."), a slider and the value, and the scale: Hostile 0 to 10,
  Neutral 11 to 89, Friendly 90 to 100.
- **Reactions**, in Advanced only (Basic's chart shows the same): how
  the chosen faction regards each of the others (a row clicked chooses
  its cell); with **Full detail**, how each of them regards it too.

Under 1400 points of window the panels are tabs of one bar. A Faction
Editor in a window of its own has its factions in an Outliner at the
window's left and its panels in the window's right pane.

The Outliner lists the factions: a click chooses one, the arrow keys
move, and a letter goes to a faction by its name. PC can be chosen
there in Advanced too: it has no row in the matrix (its own feelings are
not stored), so none is ringed; the Reaction panel says so, and
Reactions shows, with Full detail, how the others regard PCs. Each
change (a value typed, a drag let go, a slider let go) is one step of
**Edit › Undo**. The Status Bar says what the pointer can do where it
is: over a cell Click, Drag and Double Click; over a name in Basic's
chart Click; over its slider Drag. At its right it counts the factions
and those the module added ("49 factions", "44 added by the module").

**Add Faction…** opens the Add Faction window: a Name, **Global effect**
and the Parent whose attitudes the new faction starts with.
**Change Name…** (the Change Faction Name window) and **Remove Faction**
change the module's own (the five standard factions can't be renamed or
removed). A faction some object of the module still belongs to is not
removed: the log says which objects do. The Change Faction Name window
closes when its faction is removed behind it. A `repute.fac` that can't be read, or that has fewer
than the five standard factions, is not edited: the editor says so in
place of the matrix, and the file stays as it is.

Creatures, doors, placeables, triggers and encounters belong to a faction
(their Advanced page; a creature's in the Palette section of its Basic
page, with **Edit Factions** beside it, which opens this editor). The PC
faction isn't offered there: it is the
players' alone, and the game puts a creature of it in Hostile as it
loads, where it fights the players (a placeable, door or trigger keeps
it, and is regarded as players are; it regards a player as hostile, a
player it as neutral), and an encounter of it spawns nothing when a
player enters it. An object whose file has it keeps it, shown as "PC (not for
objects)", until another is picked, and Verify Module warns of it
(except on a trigger: Aurora writes faction 0 for one whose blueprint
has none, so ordinary triggers have it).
