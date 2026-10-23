---
title: "Punch-through edges white by the game's programs"
author: August
date: 2026-10-23
branch: punchthrough-edges
cost: { minutes: 601, tokens: 3292645 }
touches:
  - crates/mg-render/src/game.rs
  - crates/mg-render/tests/render.rs
  - crates/mg-ui/tests/screens.rs
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
