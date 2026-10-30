---
type: Wiki Edit
title: "Area Lighting: fixed main light radii, and a skybox does not change the fog"
description: Ready-to-paste edit for the nwn.wiki Area Lighting page, saying the game draws tile main lights with fixed radii colored from lightcolor.2da, and that the fog is the same with or without a skybox.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting
    title: "nwn.wiki: Area Lighting, version 7"
---

# Area Lighting

https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting, edit written against version 7. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

Main lights: radius fixed at 10/5 by the game, colour from lightcolor.2da; a skybox does not change the fog (start and end formula).

## Change 1
**Find** ("Main Lights 1 / 2"): "the positions and strength of the light determined by the MDL."

**Replace with:** "the positions of the lights determined by the MDL."

## Change 2
**Find** (the same section's list): "Strength (radius) set in the MDL" and "Color set by tile properties"

**Replace with:** "Radius fixed by the game: 10 for main light 1 and 5 for main light 2, whatever the MDL's light node says (see Model Special Nodes)" and "Colour set by tile properties, from the RED, GREEN and BLUE columns of lightcolor.2da"

## Change 3
**Find** ("Source Lights 1 / 2"): "coloured to what is set on the tile."

**Replace with:** "coloured to what is set on the tile, with a radius of 7."

## Change 4
**Find** (table row "Fog Clip Distance"): "but is extended by the Sky Box setting and fades into the bottom of the skybox in more wide open areas by another 90M on top of this setting."

**Replace with:** "A Sky Box does not change the fog: with or without one, the fog ends at the Fog Clip Distance and starts at the lower of (30 - Fog Amount) and (Fog Clip Distance - 1) meters from the camera. Fog is on even at Fog Amount 0. See Render Distance with Fog and Skyboxes for how far geometry is drawn with a skybox."

## Change 5
**Find** (table row "Sky Box"): "If added this extends the fog clip distance (90M), and can change"

**Replace with:** "If added this can change"; then add at the end of the cell: "It does not change the fog (see Fog Clip Distance)."

## How it was checked

In Moonglow Toolset's `crates/mg-corpus-tests/tests/`, `client_render.rs` `light_uniforms_match_the_client` (seven cases) and `fog_uniforms_match_the_client`: the game client, run on an off-screen display with a scratch user directory, draws test areas with and without a skybox, and the light radii and colours and the shader's `fogStart`/`fogEnd` are read back through a debug copy of the game's own shader include placed in that directory's override. How far geometry is drawn with a skybox was not measured. The tests read the tester's own game install.
