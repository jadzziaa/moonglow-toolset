---
type: Wiki Edit
title: "feat.2da: CRValue is read as an integer"
description: Ready-to-paste edit for the CRValue column of the nwn.wiki feat.2da page, saying the toolset's challenge rating reads it as an integer and giving the feats' share of the rating.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38175102/feat.2da
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38175102/feat.2da
    title: "nwn.wiki: feat.2da, version 14"
---

# feat.2da

https://nwn.wiki/spaces/NWN1/pages/38175102/feat.2da, edit written against version 14. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

CRValue: the toolset reads it as an integer (0.5 counts 0, `****` counts 1); the feats' share of CR.

## Change 1

**Find** (table "2da Contents", row CRValue):

> Float value. Recommended 0 instead of `****` if it has no relevance to CR.

**Replace with:**

> Written as a float, but the toolset's challenge rating reads it as an integer: 0.5 and 0.2 count as 0, and `****` counts as 1. So use 0, not `****`, if it has no relevance to CR. The feats' share of the rating is 0.1 × L × F / (L/2 + 7), where L is the creature's total level and F the sum of its feats' CRValue.

## How it was checked

`aurora_creatures.rs` `challenge_ratings_match_aurora` in Moonglow Toolset's `crates/mg-corpus-tests/tests/`: the challenge ratings Aurora (toolset 89.8193.37 under Wine) computed for 4,204 probe creatures (Build › Compile › Creature CR and Creature Properties), sweeping levels, feats and the other terms, compared with the formula. The tests read the tester's own game install.
