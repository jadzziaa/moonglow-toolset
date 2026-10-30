---
type: Wiki Edit
title: "Creature JSON: MaxHitPoints as the toolset writes it"
description: Ready-to-paste edit for the MaxHitPoints field of the nwn.wiki Creature JSON page, giving the formula the toolset uses in place of a guess.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38176577/Creature+JSON
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38176577/Creature+JSON
    title: "nwn.wiki: Creature JSON, version 10"
---

# Creature JSON

https://nwn.wiki/spaces/NWN1/pages/38176577/Creature+JSON, edit written against version 10. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

MaxHitPoints: the formula the toolset writes (HitPoints + Con modifier × level, + level with Toughness, at least 1).

## Change 1

**Find** (row MaxHitPoints):

> This appears to be HitPoints + Constitution bonuses/penalties.

**Replace with:**

> As the toolset writes it: HitPoints + the Constitution modifier (from the base Constitution plus the racial adjustment) × the total level, + the total level again if the creature has the Toughness feat; never below 1. Epic Toughness is not included.

## How it was checked

In Moonglow Toolset's `crates/mg-corpus-tests/tests/`: `aurora_creatures.rs` `max_hit_points_match_aurora` compares the formula with the MaxHitPoints Aurora (toolset 89.8193.37 under Wine) stored for 22 probe creatures opened and confirmed in its Creature Properties, and `creature_stats.rs` `max_hit_points_match_the_stored_ones` with the values stored in the base game's own creature blueprints: 1,520 of the 1,582 agree (the rest were stored before the blueprint's abilities or feats last changed). The tests read the tester's own game install.
