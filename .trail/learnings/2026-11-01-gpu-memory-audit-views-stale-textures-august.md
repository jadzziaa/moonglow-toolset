---
title: "GPU memory audit: views, stale textures, memory hints, weaker GPUs"
author: August
date: 2026-11-01
branch: worktree-agent-a208df9cc30399cde
cost: { minutes: 150, tokens: 19728242 }
touches:
  - crates/mg-render/src/renderer/kept.rs
  - crates/mg-render/src/texture.rs
  - crates/mg-render/src/gpu.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/model_view.rs
  - crates/mg-ui/tests/frame_perf.rs
  - apps/moonglow/src/main.rs
---

## Decided
- VRAM is measured with wgpu 30's `Device::generate_allocator_report()` (Vulkan and D3D12): `frame_perf`'s `FRAME_MEMORY=1` prints allocated and reserved after each view, `=labels` by label (game textures are labelled by their names: summed as "textures by name"), plus `Renderer::texture_memory()` (uncompressed, stale). Reserved is what the card gives up; with wgpu's default `MemoryHints::Performance` it was 2-4x allocated, so the app's device now uses `MemoryUsage` (`apps/moonglow` `device_descriptor`).
- A view's targets must be freed with `Viewport3d::let_go_of_targets`: egui-wgpu's `register_native_texture` keeps a bind group holding the texture view, so dropping `Targets` alone frees nothing. Views not drawn for `model_view::UNSEEN_AFTER` (2 s) let go of targets; an area whose tab is closed `reload()`s (scene dropped, camera kept). `area_views` is otherwise only emptied on module close.
- Stale textures (`renderer/kept.rs`): materials are stamped per frame (`Kept::drawn` over the frame's draws, game draws and particle groups), textures in `texture()`; a texture's last use is the max over the materials naming it. The clock is frame time with each gap capped at 1 s, so an idle window (no frames, now that views hold still without focus) never makes what it shows stale. Eviction clears materials/groups/surfaces and `GameShaders::forget_materials` (groups, templates: keyed by material index); slots and pipelines stay.
- On the 9950X's integrated Radeon (select with `MESA_VK_DEVICE_SELECT=1002:13c0 MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE=1`) an area's picture is fill-bound (quarter pixels, quarter time; shadows nothing, MSAA ~10%): Options' Area view resolution is the lever (80 -> 20 ms at 50%).
## Tried and ruled out
- Looking for a renderer regression since 3.0.0: v2.13.3, v3.0.0 and develop give the same GPU times on the iGPU and the same memory; build old tags with `git archive <tag> | tar -x -C target/old/<tag>` and their own CARGO_TARGET_DIR (about 1.5 min each here), patching their frame_perf with the same memory report and device.
- Wall-clock staleness: after an idle hour every visible texture looked stale and would have been evicted and re-read (pop-in).
## Watch out
- `frame_perf` (and any egui_kittest render state) asks for NO device features unless told: BC textures were uploaded as RGBA8 (TDoD woods 4.4 GB instead of 870 MB), so earlier TDoD frame timings were in a full card. Its `render_state()` now mirrors the app's descriptor.
- The worktree's Bash guard refuses heredocs/sed with backticks, `$VAR` paths and loops: write scripts to the scratchpad (perf/*.sh, *.py) and call them by literal path.
- egui's tooltip timer shows as "again 56 ms" in an idle view whose pointer rests on something with a tip: not a repaint leak.
