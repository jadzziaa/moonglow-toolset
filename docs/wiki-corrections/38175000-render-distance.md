---
type: Wiki Edit
title: "Render Distance with Fog and Skyboxes: a skybox does not change the fog"
description: Ready-to-paste edit for the nwn.wiki Render Distance with Fog and Skyboxes page, saying that a skybox lengthens the draw distance the page describes but leaves the fog's start and end unchanged.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-19T13:57:52Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes
    title: "nwn.wiki: Render Distance with Fog and Skyboxes, version 5"
---

# Render Distance with Fog and Skyboxes

https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes, edit written against version 5. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

With Skybox: the skybox adds 90M to the tile draw distance, not to the fog, which still ends at the Fog Clip Distance.

## Change 1

**Find** ("With Skybox"):

> This increases the fog clip distance value by 90M.

**Replace with:**

> This increases the render distance by 90M, but not the fog: the fog still ends at the Fog Clip Distance and starts at the lower of (30 - Fog Amount - (20 - the camera's distance from the player)) and (Fog Clip Distance - 1) meters from the camera (30 - Fog Amount with the camera 20 m off, as a player enters; zoomed in to 5.5 m with a Fog Amount of 12, 3.5 m), the same as with no skybox. So the extra 90M of tiles is drawn fully fogged.

## How it was checked

`client_render.rs` `fog_uniforms_match_the_client` and `client_render/night_fog.rs` `fog_start_follows_the_camera` (cameras 5.5 to 25 m from the player) in Moonglow Toolset's `crates/mg-corpus-tests/tests/`: the game client, run on an off-screen display with a scratch user directory, draws test areas with and without a skybox, and the shader's `fogStart` and `fogEnd` are read back through a debug copy of the game's own shader include placed in that directory's override. The +90M draw distance is the page's own figure; it was not measured. The tests read the tester's own game install.
