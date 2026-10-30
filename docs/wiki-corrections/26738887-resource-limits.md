---
type: Wiki Edit
title: "Resource Limits: the 2 GB hakpack limit is the game's too"
description: Ready-to-paste edit for the nwn.wiki Resource Limits page, saying that the game itself cannot read a resource that starts past 2 GiB in a hak, not only the toolset and nwsync.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits
    title: "nwn.wiki: Resource Limits, version 7"
---

# Resource Limits

https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits, edit written against version 7. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

The 2GB hakpack limit applies to the game too: resources starting past 2GB in a hak are missing.

## Change 1

**Find** ("16K per ERF File", under NWN:EE 1.80.8193.14 and later):

> Note: Hakpack files should stay under 2GB since the Toolset (and other tools like nwsync) doesn't read past 2GB.

**Replace with:**

> Note: Hakpack files should stay under 2GB. The game doesn't read past 2GB either, not only the Toolset (and other tools like nwsync): a resource whose data starts at 2GB (2^31 bytes) or later in a hakpack is missing to the game, so a 2da reads as empty and a script doesn't run. A resource that starts before that point is read whole, even if it ends after it. A copy of the same resource in a lower hakpack doesn't take over: the game finds the name in the higher hakpack, fails to read it, and stops there.

## How it was checked

`engine_big_hak.rs` in Moonglow Toolset's `crates/mg-corpus-tests/tests/`: `the_game_reads_nothing_that_starts_past_2_gib_in_a_hak` and `past_2_gib_a_lower_haks_copy_doesnt_count` build sparse 2.1 GiB hakpacks with a 2da and a script before and after the 2 GiB mark, and run a module using them in the game's own server (`nwserver`, started privately), logging what a script reads and runs. The tests read the tester's own game install.
