---
type: Decision
title: "Grass sway as a skin, sample_framebuffer late, area084 taken apart"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [crates/mg-area/src/grass.rs, crates/mg-area/src/scene.rs, crates/mg-render/src/renderer.rs, crates/mg-render/src/reach.rs, crates/mg-mdl/src/ascii.rs, crates/mg-ui/src/lib.rs]
fingerprint: v2-81c6fea9544b70ee
created: 2026-10-30
author: August
description: "Grass sway is done as a two-bone skin (mg_area::grass::tufts, SWAY_NODE) posed per tile, so existing skinning shaders work unchanged; also records late sample_framebuffer draws, Thumbnails reset, ASCII skin weight parsing, area084 perf findings."
asks:
  - "How is grass sway implemented without changing mg-render or adding a vertex pass, and why do vsgrass and vs_skinned work unchanged?"
  - "Why does a material using sample_framebuffer draw after the copy and shadows, and why is it not treated as receiving shadows?"
  - "Thumbnails look stale after switching game shaders; where are they cleared when the effective shaders change?"
  - "area084 renders slowly at about 10k draws; which optimizations were tried and rejected, and which one actually saved time?"
---
## Decided
- Grass sway is a skin, not a vertex pass: `mg_area::grass::tufts` makes each mesh a two-bone skin (root = feet, `SWAY_NODE` = tops, weights by `v % TUFT_VERTICES >= 2`) and the scene poses the sway node per tile (the world offset turned into the tile's space, poses shared per turn). The game's `inc_standard`/`inc_transform` skins any program through `skinmesh`, so `vsgrass` moves it unchanged; Moonglow's `vs_skinned` too. No mg-render change. Far chunks (merged, unskinned) stay still.
- A material's `sample_framebuffer` sets `Slots::late`: the draw goes after the copy and the shadows (`draw.late`), and is not `receives`.
- `set_game_shaders` (and the per-frame sync after the lit check) forgets `Thumbnails` when the effective shaders change.
- ASCII skin weights: blank lines are vertices only when the non-blank rows before the next keyword (`keywords::TOP_LEVEL`, `known_anywhere`, `endlist`) are fewer than the count.
## Tried and ruled out
- area084 seen whole (8 ms encode, ~10k draws): culling by screen size (632 of 12k meshes under 2 px), merging far tiles (per-mesh 32-light choice changes the picture), caching draw values while the camera moves (`m_mv`/`m_mvp` per draw; lights in view space). Only `ReachCache` in one map + light_sets' room kept: 8.0 -> 7.5 ms.
## Watch out
- Profile a render alone with `render_area`'s `MG_TIME_RENDER` (one thread): eu-stack on frame_perf took ~3 s a sample (many threads) and got 3 samples in 10 s.
- Never leave an `std::env::var_os` in the per-mesh loop, even for a temporary measurement: it cost ~1.5 ms a frame there and skewed the baseline.
- frame_perf fails "over by more than the margin" at load averages over ~15 (other agents' builds): compare by alternating two copied binaries (`target/release/deps/frame_perf-*`), not against old numbers.
