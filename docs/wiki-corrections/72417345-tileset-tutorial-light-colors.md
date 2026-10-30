---
type: Wiki Edit
title: "Tileset Construction Tutorial: tile light colors come from lightcolor.2da"
description: Ready-to-paste edit for the Tile Light Colors section of the nwn.wiki Tileset Construction Tutorial, which names tilecolor.2da where the game uses lightcolor.2da, and adds the fixed light radii.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial
    title: "nwn.wiki: Tileset Construction Tutorial, version 13"
---

# Tileset Construction Tutorial

https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial, edit written against version 13. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

Tile Light Colors: main lights use lightcolor.2da (not tilecolor.2da), source lights fx_flame01; the game's fixed light radii.

## Change 1

**Find** ("Tile Light Colors"):

> Tile-oriented mainlights and sourcelights will draw their colors only from the file "tilecolor.2da".

**Replace with:**

> Tile-oriented mainlights draw their colors only from the file "lightcolor.2da" (its RED, GREEN and BLUE columns; the TOOLSETRED, TOOLSETGREEN and TOOLSETBLUE columns only color the toolset's swatches). Sourcelights draw theirs from the fx_flame01 animation their value names. The game draws main light 1 with a radius of 10 and main light 2 with a radius of 5, whatever the tile model's light node says (the model gives only the position), and source lights with a radius of 7, as Model Special Nodes describes for ml1 and ml2.

## How it was checked

`client_render.rs` `light_uniforms_match_the_client` in Moonglow Toolset's `crates/mg-corpus-tests/tests/` (seven cases): the game client, run on an off-screen display with a scratch user directory, draws a test area, and the light colors and radii it sends to its shader are read back through a debug copy of the game's own shader include placed in that user directory's override. The tests read the tester's own game install.
