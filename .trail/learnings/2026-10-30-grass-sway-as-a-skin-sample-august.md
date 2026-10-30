---
title: "Grass sway as a skin, sample_framebuffer late, area084 taken apart"
author: August
date: 2026-10-30
branch: worktree-agent-a17f4646e5c69b1e7
cost: { minutes: 2391, tokens: 16674667 }
touches:
  - crates/mg-area/src/grass.rs
  - crates/mg-area/src/scene.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/reach.rs
  - crates/mg-mdl/src/ascii.rs
  - crates/mg-ui/src/lib.rs
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
