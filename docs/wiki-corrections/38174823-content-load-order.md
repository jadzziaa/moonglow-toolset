---
type: Wiki Edit
title: "Content Load Order: a hak's folder outranks its place in the hak list"
description: Ready-to-paste edit for the nwn.wiki Content Load Order page, adding that a hak in the user's hak folder outranks one in the install's data/hk folder whatever the list order, and where the custom TLK is looked up.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order
    title: "nwn.wiki: Content Load Order, version 18"
---

# Content Load Order

https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order, edit written against version 18. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

Order of Hakpacks: the list order holds within one folder; a user hak outranks an install hak. TLK Files: the lookup order.

## Change 1

**Add after** ("Order of Hakpacks"):

> A note on the order of hakpacks and duplicate files; the highest (first loaded) hakpack takes precedence.

**Add:**

> This holds among hakpacks in the same folder. A hakpack in the user's hak folder outranks one in the install's data/hk folder, even when it is listed lower in the module's hakpack list (see Content Order below).

## Change 2

**Add after** ("TLK Files"):

> TLK files can be placed into HAK files and use the same priority system as all other overridable files.

**Add:**

> The module's custom TLK is looked up in the module's hakpacks first, then in the module, then in the user's tlk folder. The female variant (xxxf.tlk) is looked up separately in the same order.

## How it was checked

In Moonglow Toolset's `crates/mg-corpus-tests/tests/`: `engine_resman.rs` `resolution_matches_engine` (a user hak listed after an install hak, among other precedence boundaries, resolved by the game's own server, `nwserver`, started privately, through ResManGetAliasFor) and `engine_tlk.rs` `where_the_game_reads_custom_talk_tables` (custom TLKs in a hak, the module and the tlk folder, read back in the server). The tests read the tester's own game install.
