---
type: Guide
title: Interface text style guide
description: How Moonglow's interface text is written - which text takes title case and which sentence case, how tooltips are kept short, and the spelling - for every change that adds or changes a label, a heading, a tooltip or a message.
tags: [style, interface, text, guide]
status: stable
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-08T18:00:00Z }
sources:
  - id: owner
    resource: conversation with the owner, 2026-11-08
    title: "The owner's observations: headings in title case, in American English; tooltips concise as industry tooltips are, few semicolons; a style guide for future agents"
---

# Interface text style guide

What the window says is read in passing, while the builder works on
something else. It is short, plain and the same from one window to the
next. Code comments, commit messages, the log and the manual are not
interface text: they keep their own, longer style.

## Title case and sentence case

**Title case** for what names a place or an action:

- window and dialog titles (**Setup Area Transition**, **Area Wizard**);
- section headings (`widgets::section_heading`: **Saving and Exporting**);
- tabs and pages (**Basic**, **Visual Transform**);
- menus and their items (**Edit › Change Tileset…**);
- buttons, toolbar buttons and their names in tips (**Give to the
  Selection**, **Show All**, **Go to Start Location**);
- the column headings of tables.

**Sentence case** for everything that is read rather than chosen:
field labels (`widgets::field_label`), checkboxes and radio buttons
(**Hide empty groups in the module tree**), tooltips, the status line,
notes under a field, the log, and messages.

In title case, every word is capitalized except, when they are not the
first or the last word:

- articles: a, an, the;
- coordinating conjunctions: and, but, or, nor, for, so, yet;
- short prepositions: as, at, by, for, from, in, into, of, off, on,
  onto, out, over, per, to, up, via, with;
- `to` before a verb (**Go to Start Location**, **Add to Palette**).

A word after a colon or a dash starts with a capital. Words that are
written a fixed way keep it: ResRef, 2DA, NWSync, NUI, TLK, GFF, MTR,
Tag. A file name, a resref, a variable or a key stays as it is
(`tdod_tint`, `module.ifo`, Ctrl+Z).

## Tooltips

A tooltip says what the control does, or what it is, in one short
sentence: about twelve words, never more than two sentences.

- Begin with what it does (**Pick the color**; **Shows the area at
  night**), not with the control's name again.
- No semicolons. Split the thought into two sentences, or keep the
  first half.
- No history: who asked for it, what it did before, and why it changed
  belong in the log and the code's comments.
- No lists of every case. The common one, and where to read more
  (the manual) if the rest matters.
- Keys in the form the menus show them: **Ctrl+Shift+S**.
- Units and limits where they help: **Tiles, 10 m each**; **0 to 255**.

A long explanation that a builder needs belongs under the field as a
weak note, or in the manual, not in a tooltip.

## Words

- **American English**: color, gray, center, meter, license, armor,
  behavior, -ize (organize, normalize), traveled, canceled.
- The game's own names, as the game and Aurora write them: Placeable,
  Waypoint, Area Transition, Static, Plot, Faction.
- Say what the builder sees, not how Moonglow does it: **Not found in
  the module or its haks**, not **ResMan lookup failed**.
- A count with its noun, singular for one: **1 placeable**,
  **3 placeables**.
- An ellipsis (…, one character) after a command that asks something
  more before it acts (**Save As…**, **Tint…**), and nowhere else.

## For changes

A change that adds or changes interface text follows this guide, and a
test that finds a control by its label (`get_by_label`) is changed with
it. `CLAUDE.md` points here.
