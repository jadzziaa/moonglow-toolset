---
type: Wiki Edit
title: "Creature: the toolset validates the spells of every spellcasting class"
description: Ready-to-paste edit for the nwn.wiki Creature page, which guesses the toolset validates spells only on clerics; it checks every spellcasting class.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38174829/Creature
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38174829/Creature
    title: "nwn.wiki: Creature, version 6"
---

# Creature

https://nwn.wiki/spaces/NWN1/pages/38174829/Creature, edit written against version 6. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

Creating a Creature: the toolset checks every spellcasting class's spells on OK, not only clerics'.

## Change 1

**Find** ("Creating a Creature"):

> Note the toolset does *very* light validation of this (only on clerics?), and creature abilities and spell abilities are ignored more or less from these checks.

**Replace with:**

> Note the toolset does only light validation of this: when you press OK it checks every spellcasting class (wizards, sorcerers and bards as well as clerics) and shows at most one warning per class, the first that applies: a spell above the highest level the class can cast; a casting ability below 10 + the spell's level; or too many spells of one level. Its count of spells allowed also gives preparing casters bonus spells at level 0, which the game does not. Creature abilities and spell abilities are ignored more or less from these checks.

## How it was checked

`aurora_spell_warnings.rs` `spell_warnings_match_aurora` in Moonglow Toolset's `crates/mg-corpus-tests/tests/`: thirty creatures given spells were opened in Aurora's Creature Properties (toolset 89.8193.37 under Wine) and closed with OK; the warnings Aurora showed, in order, are compared with the rules above. The tests read the tester's own game install.
