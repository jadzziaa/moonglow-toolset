---
type: Wiki Edit
title: "Area Editor: what Resize Area, Rotate Area and Update Instances do"
description: Ready-to-paste edit for the nwn.wiki Area Editor page, saying that Resize Area continues the edge's terrain rather than copying its tiles, that Rotate Area turns counter-clockwise, and what Update Instances keeps and loses.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T04:20:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/60982635/Area+Editor
sources:
  - id: proposal
    resource: ../wiki-proposal.md
  - id: findings
    resource: ../findings.md
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/60982635/Area+Editor
    title: "nwn.wiki: Area Editor, version 17"
---

# Area Editor

https://nwn.wiki/spaces/NWN1/pages/60982635/Area+Editor, edit written against version 17. Unchanged on nwn.wiki as of the mirror's sync of 2026-10-26.

## Edit summary

Resize Area continues the edge terrain with random fitting tiles; Rotate Area turns counter-clockwise; Update Instances overwrites the instances' own changes.

## Change 1

**Find** (table row "Resize Area"): "If adding some to an area (much safer) it will copy the same tiles that were on those edge areas."

**Replace with:** "If adding some to an area (much safer) it continues the terrain of the edge, not its tiles: new corners take the terrain and height of the nearest old corner, new edges the crosser (road, stream, wall) of the nearest old edge, and the toolset then picks tiles at random among those that fit. If removing some, what is left of a group cut by the new edge is replaced with plain terrain tiles and its doors are removed."

## Change 2

**Find** (table row "Rotate Area"): "Rotations are of course not destructive"

**Replace with:** "Each Rotate Area turns the area a quarter turn counter-clockwise. Rotations are of course not destructive"

## Change 3

**Find** ("Object Palettes Right Hand Sidebar"): "Finally you can make changes to a blueprint, and use Update Instances to find and update those objects to match the blueprint."

**Replace with:** "Finally you can make changes to a blueprint, and use Update Instances to find and update those objects to match the blueprint. Be careful: it remakes every placed copy as if it were placed anew. Only its position, facing, visual transform, a trigger's or encounter's shape and an encounter's spawn points are kept; any changes made on the instance itself (its tag, name, scripts, variables, Plot and Locked flags, a door's transition, a sound's volume) are lost."

## How it was checked

In Moonglow Toolset's `crates/mg-corpus-tests/tests/`: `aurora_terrain.rs` `resize_and_rotate_match_aurora` (areas resized and rotated by Aurora, toolset 89.8193.37 under Wine, compared tile by tile with what it saved) and `aurora_update_instances.rs` `update_instances_matches_aurora` (instances given their own values, Update Instances run in Aurora, the saved areas compared field by field). The tests read the tester's own game install.
