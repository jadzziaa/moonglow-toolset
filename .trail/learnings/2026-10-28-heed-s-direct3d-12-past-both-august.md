---
title: "Heed's Direct3D 12 past both checks, HLSL and FXC"
author: August
date: 2026-10-28
branch: worktree-agent-af8f9f88e4f8accce
cost: { minutes: 1493, tokens: 12630100 }
touches:
  - docs/deferred.md
  - docs/log.md
  - docs/manual/13-troubleshooting.md
  - docs/manual/index.md
  - Cargo.lock
  - crates/mg-corpus-tests/Cargo.toml
  - crates/mg-corpus-tests/tests/groups/areas.rs
  - crates/mg-corpus-tests/tests/hlsl.rs
  - crates/mg-render/Cargo.toml
  - crates/mg-render/src/framebuffer.rs
---

## Decided
- Heed's 2.13.0 Direct3D 12 failure had several ways past the checks: textures never read leave tiles out and keep `ready` false (now judged after `watch::PATIENCE` 20 s with `Renderer::progress()` unchanged); tiles whose GpuModel panicked were left out of the expectation (`AreaScene::failed_tiles`, `Observation::failed`); a look never mapped stopped the watch (`Sampler::give_up`, `LOST` 5 s); `game::checked` took Validation only, and Direct3D 12's FXC/DXC failures are `CreateRenderPipelineError::Internal` (hal `PipelineError::Linkage`), which went to wgpu's default handler: a panic (no app sets `on_uncaptured_error` on the window's device; egui-wgpu doesn't either), caught by the worker's catch_unwind, with a `crash-*.txt` named "crashed".
- So on the window's device, any wgpu error on the UI thread is a crash: a builder reporting "draws nothing" with no crash had no wgpu errors there.
- The start ring and door arrows are egui painter overlays: no depth, never measured.
- HLSL check: `mg_testkit::hlsl` (feature `hlsl`) repeats wgpu-hal 30 dx12 `create_pipeline_layout`'s binding map (space 0, registers across groups, samplers via the sampler heap + per-group index SRV, special constants always: wgpu-core always sets FIRST_VERTEX_INSTANCE). FXC = Microsoft's d3dcompiler_47.dll at `~/.local/share/lutris/runtime/d3d_extras/v2/x64/` (10.0.15063.674) via `tools/dx12/fxc.sh` (mingw driver under Wine, `MOONGLOW_D3DCOMPILER`). The oracle prefix's d3dcompiler_47 is Wine's builtin (vkd3d): no oracle.
- Result: all Moonglow WGSL compiles; of the game's 38 distinct stages (full corpus) `fslit_nm`/`fslit_sm_nm` fail X3511 (height-map loop with implicit-derivative sample, varying count). Heed's module's `fhss_skintile` is a copy of fslit_nm. Deferred: a DX12-only explicit-level/gradient rewrite or shipping DXC.
- Lit check `probe::probe_lit` runs on a thread when the game's data and viewport exist and `background_jobs` (desktop app; tests that set it run it too): game's alone wrong → `RendererCheck::game_shaders_off`, read through `Moonglow::game_shaders()`; the Shaders button (`set_game_shaders`) clears it.

## Watch out
- Window watch tests drive the harness clock (0.25 s a step) much faster than real time: a wait judged on progress must hold the worker (`guard::hold_programs`) or pace the clock, and looks are read with `watch::wait_for_looks(true)` (set in `watched()`), or verdicts depend on load.
- With `background_jobs` the area opens on a thread: the first look comes several harness seconds in.
- CI's 2.13.0 Windows (WARP) and macOS (Metal) runs do run mg-render's render tests: textures apart, authored game programs, framebuffer copies.
