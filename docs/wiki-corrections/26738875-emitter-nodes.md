---
type: Wiki Edit
title: "MDL ASCII Emitter Nodes: mass, bounce_co, m_isTinted and twosidedtex as the game draws them"
description: Ready-to-paste edit for the nwn.wiki MDL ASCII Emitter Nodes page, making mass, bounce_co, m_isTinted and twosidedtex precise from what the game client was seen to draw.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/26738875/MDL+ASCII+Emitter+Nodes
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/26738875/MDL+ASCII+Emitter+Nodes
    title: "nwn.wiki: MDL ASCII Emitter Nodes, version 27"
---

# MDL ASCII Emitter Nodes

https://nwn.wiki/spaces/NWN1/pages/26738875/MDL+ASCII+Emitter+Nodes, edit written against version 27. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

mass, bounce_co, m_isTinted, twosidedtex: what they do in game, as measured from client screenshots.

## Change 1

**Find** (row mass): "Effectively the weight of the particles - positive values make the particle fall down, negative values make it float up"

**Replace with:** "Effectively the weight of the particles - positive values make the particle fall down, negative values make it float up. In testing, gravity on the particles is mass × 9.8 m/s² (a mass of 1 falls at 9.8 m/s²)."

## Change 2

**Find** (row bounce_co): "the degree of bounce a partical will have when it hits a static walkmesh and the Bounce flag is set"

**Replace with:** "the degree of bounce a particle will have when it hits a static walkmesh and the Bounce flag is set. In testing, a particle hitting the ground keeps 0.8 of its speed along the ground and 0.8 × bounce_co of its speed off it."

## Change 3

**Find** (row m_isTinted): "Will tint the particles to the ambient colour of the scene"

**Replace with:** "Will tint the particles by the light at the emitter: in testing, the area's ambient plus diffuse colour, plus any point lights, the same for every particle of the emitter"

## Change 4

**Find** (row twosidedtex): "If the particle is visible from both sides"

**Replace with:** "Meant to make the particle visible from both sides, but in testing it has no visible effect: one-sided particles show from behind too"

## How it was checked

`client_render.rs` `particles_look` in Moonglow Toolset's `crates/mg-corpus-tests/tests/`: probe emitters drawn by the game client, run on an off-screen display with a scratch user directory, and measured from screenshots (gravity 9.80 m/s² for mass 1, 4.83 for 0.5). This is exploration with no asserted tests, the weakest evidence of these edits; the tests read the tester's own game install.
