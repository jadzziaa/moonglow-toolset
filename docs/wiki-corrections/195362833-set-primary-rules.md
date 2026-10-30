---
type: Wiki Edit
title: "SET File Format: primary rules are in use, and declared counts are not always counts"
description: Ready-to-paste edit for the nwn.wiki SET File Format page, saying that base-game tilesets use primary rules and how the toolset's terrain brush applies them, and that declared counts in custom tilesets are not always reliable.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/195362833/SET+File+Format
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/195362833/SET+File+Format
    title: "nwn.wiki: SET File Format, version 2"
---

# SET File Format

https://nwn.wiki/spaces/NWN1/pages/195362833/SET+File+Format, edit written against version 2. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

[PRIMARY RULES] are used by 23 base game tilesets and applied by the toolset's terrain brush; declared counts are not always counts.

## Change 1

**Find** ("[PRIMARY RULES] / [SECONDARY RULES]"):

> Not used in practice for custom content. Difficult to implement and generally should be ignored. Set `Count=0`.

**Replace with:**

> Primary rules are used: 23 of the base game's tilesets have them (up to 192 rules), and the toolset's terrain brush applies them. Painting a corner looks up, for each of the eight corners around it (diagonals included), the rule for (`Placed`, placed height − base, `Adjacent`, adjacent height − base), where base is the lower of the two heights; `Changed` and `ChangedHeight` give that neighbour its new terrain and a height of base + `ChangedHeight`. Rules are not applied again to the corners they change, and corners more than one step apart in height match no rule. A custom tileset with no terrain that needs them can set `Count=0`.

## Change 2

**Add after** ("Door Sub-Sections", after its example):

> Door header index must match parent tile index.

**Add:**

> Note: in custom tilesets in use, `Doors=` and `[GROUPS] Count=` are not always real counts (seen: `Doors=1869573190`, which is the bytes of the text "Floo", and a `Count=270` with no `[GROUP266]`). Tools reading a .set should read the sections that exist rather than loop to the declared count.

## How it was checked

`aurora_terrain.rs` `painting_matches_aurora` in Moonglow Toolset's `crates/mg-corpus-tests/tests/`: 54 scripted painting steps in four base game tilesets, compared tile by tile with the areas Aurora saved (toolset 89.8193.37 under Wine). The rule counts are from the base game's .set files. The tests read the tester's own game install. The odd counts were seen in custom tilesets on a live server.
