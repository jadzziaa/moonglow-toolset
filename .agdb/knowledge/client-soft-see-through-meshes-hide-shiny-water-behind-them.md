---
type: Gotcha
title: "Client: soft see-through meshes hide shiny water behind them; Moonglow's soft parts don't"
description: "In the client a soft see-through mesh in front of shiny water hides the water (as if it wrote depth); Moonglow draws soft (Fringe) parts after the water without depth, and moving them before the copy laid the water over them"
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T07:39:28Z }
asks:
  - "Why not draw the see-through meshes' soft parts before the framebuffer copy?"
  - "Does the client's water draw over an alpha-blended mesh in front of it?"
  - Why does a 32-bit TGA with alpha 0.5 on a placeable draw opaque in the client?
created: 2026-11-26
author: augustaugur
---
copies::reading_copy_order MG_COPY_ORDER=front (2026-11-26): fswater patched to paint copy.r, 1.0, copy.b. A standard DXT5 of alpha-0.5 texels with transparencyhint 1 in front of a pool: client green 39 there (no water behind it), Moonglow as built 132 (soft part laid over the water), with soft parts sorted before the copy (rank Fringe 4 before late) 255 (water over the card, since Fringe writes no depth). The latter variant is saved nowhere in the tree; the render test's soft wall case only passes with it once fslit gives the texture's alpha (clip_space_assets' fslit outputs alpha 1, which hides the difference). Fixing it means soft parts writing depth for every see-through mesh: filed in deferred.md. A 32-bit TGA of alpha 0.5 texels on a placeable mesh draws opaque in both the client and Moonglow, hint or not; use a standard DXT5 for a soft card.
