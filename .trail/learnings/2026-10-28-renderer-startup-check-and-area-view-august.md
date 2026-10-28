---
title: Renderer startup check and area view watch
author: August
date: 2026-10-28
branch: worktree-agent-a5b3c5d9a3f50adaf
touches:
  - crates/mg-render/src/probe.rs
  - crates/mg-render/src/watch.rs
  - crates/mg-render/src/backend.rs
  - crates/mg-render/src/guard.rs
  - crates/mg-ui/src/renderer_check.rs
  - crates/mg-ui/src/area_view.rs
  - apps/moonglow/src/main.rs
---

## Decided
- The window's adapter goes through `mg_render::backend::start` (`pick_adapter`, then `probe::probe` on that `wgpu::Adapter` inside egui-wgpu's `native_adapter_selector`; on Automatic a failure takes `other_backend` once). A backend asked for (Options, `WGPU_BACKEND`) is kept and gets the notice. `request_device(&self)` on the same adapter twice works (the probe's device, then egui's).
- Failures are `Settings::graphics_failures`, keyed vendor + device + `driver_of` (driver and driver_info; Direct3D 12 gives the version alone) + backend name; `pick_adapter`'s `failed` list skips them on Automatic unless every adapter is recorded.
- Watch: depth (the sky writes none; cleared to 1.0) sampled on a 64x36 grid by a one-draw R32Float pass (multisampled depth, sample 0), `map_async` read on a later frame; the expectation is rays to the frame's tile instances' 10 m ground squares (`tile && !grass && opacity >= 1`), downward rays only. The expectation is stored with the look, so a zoom can't make a false failure. Ready = `drawn.game_pending == 0 && textures_pending == 0`. Costs here: probe 88 ms (83 ms of it `Renderer::new`'s pipelines), a look 0.02 ms plus the expectation 0.05 ms, once a second.
- `guard::blank_draws` sets a 1x1 scissor in `scene_pass`: Moonglow's draws, the game's, the sky, lines and particles. Tests that set it take `gpu::alone()`.

## Watch out
- kittest: a click on a window opened in the frame before needs `h.run()`, not `run_steps(2)` (Don't Save was missed).
- In a worktree session, Bash refuses heredocs and `flock … cargo test`: wrap the command in a scratchpad script.
