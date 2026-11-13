---
type: Decision
title: "Punch-through edges white by the game's programs"
tags: []
status: stable
generated: { by: trail, at: 2026-10-23T00:00:00Z }
touches: [crates/mg-render/src/game.rs, crates/mg-render/tests/render.rs, crates/mg-ui/tests/screens.rs]
fingerprint: v2-cd5a6cd0d01bdc22
created: 2026-10-23
author: August
description: "White lines on punch-through cut edges were the render's leftover alpha, not color: opaque game passes wrote texture alpha; GamePass::write_mask() now restricts non-blending passes to ColorWrites::COLOR, keeping alpha at 1."
asks:
  - "Why do cut-out edges show a white line with the game's shaders on, even though the RGB output matches Moonglow's?"
  - "How do I fix see-through semi-transparent texels on punch-through (TXI blending punchthrough) models in the opaque pass?"
  - "How can I tell whether a white artifact in a rendered PNG is real color or just alpha composited over white?"
  - "Why does my model-view screens test show the WGSL fallback instead of the game shaders, and how long should it wait?"
---
## Decided
- The 'white line' along cut edges (TXI blending punchthrough, game's shaders on) was the PICTURE'S ALPHA, not a color: GamePass::Opaque drew unblended with ColorWrites::ALL, and fslit writes gl_FragColor = FragmentColor, alpha = texture alpha; texels kept above fAlphaDiscardValue (0.2) but below 1 left the target see-through. mgv's PNG shows it over the viewer's white; egui composites the view's texture premultiplied (lighter rim). Moonglow's WGSL sets color.a = 1 for cut-outs (extra.w 3), so it was clean.
- Fix: GamePass::write_mask(): passes that don't blend (Opaque, Sky, PunchThrough particles) write ColorWrites::COLOR; the scene clears alpha 1 and blended passes use alpha OVER, so the picture stays opaque.
- Client verdict (creatures_look with MG_OVERRIDE = model files + TDoD appearance.2da, MG_APPEARANCES=31002,31002): no lines; a marked inc_standard (rgb magenta where FragmentColor.a < 0.99) shows the band drawn solid, unblended.
## Tried and ruled out
- Texture, MTR, env map, uniforms, blend factors: the RGB Moonglow drew was right; only alpha differed. Count pixels with alpha < 255 in the render (game 7937, own 0) before reading anything into colors.
- 'decal 1 removes the lines': it doesn't (7940 pixels); the model is lit brighter, so the white rim shows less.
## Watch out
- The model view builds game pipelines in the background: a screens test must step for a few seconds (model_view_game_shaders) or it shows the WGSL fallback.
- SetCameraFacing from OnClientEnter: wrap in DelayCommand(1.0, ...) for MG_CLIENT_CAMERA.
- The Read tool / image viewers composite PNG alpha over white: check alpha before trusting a 'white' artifact.
