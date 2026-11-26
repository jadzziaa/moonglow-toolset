---
type: Gotcha
title: "Soft see-through texels write depth in the client and, since 2026-11-26, in Moonglow"
description: "Why Moonglow's soft (Fringe) parts write depth and sort before the framebuffer copy's readers, what the client reading showed per texture kind, and what was left different."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T11:49:29Z }
touches: [crates/mg-render/src/renderer.rs, crates/mg-render/src/game.rs, crates/mg-corpus-tests/tests/client_render/soft.rs, crates/mg-render/tests/render.rs]
asks:
  - Do soft see-through texels write depth in the NWN client?
  - "Why does Moonglow's Fringe pass write depth?"
  - Why is shiny water hidden behind a soft decal or card?
  - Which soft-texel differences from the client are left?
fingerprint: v2-5d0f65969fcbfdce
created: 2026-11-26
author: augustaugur
---
soft::reading_soft_depth (client_render, MG_SOFT_KIND=hint|dxt5|tga|punch|mtr|alpha, MG_SOFT_GRASS=1 turns the client's grass on, MG_SOFT_TAG names the run): the client writes depth for soft texels above the 0.2 cut for a DXT5 with or without transparencyhint, an MTR transparency 1 and mesh alpha. Water (fswater) and particles behind a soft card are not drawn there; soft quads in one mesh hide by face order; grass tufts hide decals under them.
- Moonglow since 2026-11-26: Pass::Fringe and GamePass::Fringe write depth; ranks Blend 3, Fringe 4, late Blend 5, late Fringe 6, Additive 7; the copy at the first late or additive draw (renderer.rs).
- Watch out: a render test's fswater must read texFBColor or the water is not late; clip_space_assets' fslit outputs alpha 1, so a soft-texture test needs an fslit override giving the texture's alpha.
- MG_CLIENT_CAPTURED does not apply to copies::reading_copy_order: it runs the client (needs DISPLAY in the display lock).
- Left in deferred.md (Soft see-through texels): Moonglow's own water (Shaders off) is drawn with the solid meshes, before the card; TXI blending punchthrough plus the hint is cut in the client, blended in Moonglow; two meshes of one model: the client's order differs by kind (hint vs mtr), Moonglow sorts by distance.
- A 32-bit TGA's alpha on a placeable is drawn opaque by both; use a DXT5 for a soft card.
