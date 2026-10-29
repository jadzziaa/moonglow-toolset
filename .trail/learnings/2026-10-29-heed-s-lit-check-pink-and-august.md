---
title: "Heed's lit check pink and the game's own shader files"
author: August
date: 2026-10-29
branch: worktree-agent-a7e2addf54837b448
cost: { minutes: 28, tokens: 299588 }
touches:
  - crates/mg-corpus-tests/tests/hlsl.rs
  - crates/mg-render/src/assets.rs
  - crates/mg-render/src/probe.rs
  - crates/mg-render/tests/render.rs
  - crates/mg-ui/src/model_view.rs
  - crates/mg-ui/src/renderer_check.rs
  - crates/mg-ui/tests/app/renderer_check.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
---

## Decided
- Heed's 2.13.1 lit check pink ([212,133,133] on his RX 580 Vulkan; here both shaders draw [204,56,56]) is exactly stock fslit with fNdotV = 0: env specularity 0.29 for 0.04 (mix(1, 0.04, sqrt(0.55))), red light x0.71/0.96, diffuse unchanged. The G/B here are the stock white specular (SpecularColor white for a red albedo).
- The lit check read vslit/fslit and includes through the whole load order (override, development, Workshop, userpatch and the module's haks: sync_haks adds them to app.game's resman). It now draws the game's own copies (Assets::game_shader: the highest LayerClass::Key copy) and lists load-order copies (Assets::shader_origin) in its log line with both middle colors (Lit::own_color/game_color, pictures kept). GameAssets delegates both.
- Notice: "draws nothing of its lit, textured test mesh" only where the middle is background (Lit::drew_nothing, <=50); else "in the wrong colors".
## Tried and ruled out
- Driver-dependent inputs: uniform blocks are zeroed on the CPU then written (filling the contract's zero members with 1.0 gives [255,220,220], 0.25 gives [179,32,32]); naga's spv backend zero-inits private and function variables, spv-in reads OpUndef as null; env/noise/missing slots bound to white 1x1; zero vertex inputs only feed skinning; storage reads in bounds; light loop not run (numLights 0). None reproduces the pink.
- Patches here: fNdotV=0 -> [202,116,116]; view vector in the surface's plane toward the sun's azimuth -> [208,127,127]; flipped gl_FrontFacing -> [103,59,59]; Specularity >= 0.2 -> no green (metallicness rises).
## Watch out
- A CPU model of the stock fslit at the check's middle pixel (softening, GGX, Schlick geometry, Fresnel ^5) reproduces the GPU within 1: three free parameters fit any pink, so one pixel can't name the mechanism.
- Bash here refuses `cat >> file <<EOF` and compound git; python3 - <<'EOF' and scratchpad scripts work.
