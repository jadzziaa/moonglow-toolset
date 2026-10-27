---
title: Dense area frames and opening an area apart
author: August
date: 2026-10-27
branch: area-frames-and-load
cost: { minutes: 330, tokens: 11192355 }
touches:
  - crates/mg-render/src/pool.rs
  - crates/mg-render/src/batch.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/game.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/tests/frame_perf.rs
---

## Decided
- Meshes live in shared chunks (`mg-render/src/pool.rs`, `Gpu::pool`): a draw binds a chunk's vertex/index buffers only when the draw before was of another chunk, and draws `first_index..+count` at `base_vertex`. wgpu-core's per-call cost of `set_vertex_buffer`/`set_index_buffer` (init tracker, usage merge, registry lookups, radv binds) was ~35% of a 30k-draw frame. Chunks grow to the size of all before (up to the device's max buffer, 1<<22 vertices): a large area still spans ~10, so draws switch chunks often; fewer, larger chunks would help more.
- See-through draws are batched into **runs** by state (pipeline, material) as well as by mesh (`batch.rs`: a batch's place is (run, index); a draw joins its mesh's batch, else the end of its state's latest run, else a new run, each only where it is taken past no draw it meets). Material changes in TDoD's woods overview 7,700 -> 1,800; it is what cut wgpu's `finish` from ~7 to ~4 ms. The randomized test checks the order invariant; mutate `since` to see it fail.
- `meets_after` scans only draws placed since the batch began / the run after began (positions are not monotone in placing order); for a box on too many squares it asks whether any draw lies after the place (`top`), never "any placed since" (that never joined wide see-through boxes: render.rs batching tests caught it).
- The game's material groups are bound last (STAGE_GROUPS 2,3; draws' storage 0, bones 1): a group bound anew makes wgpu re-flush every group after it. The storage group's layout has min_binding_size None so it is one group for every pipeline and stays bound across pipeline changes.
- A game program that moves vertices (reads the clock, the wind, or samples a texture in the vertex stage: `GamePipeline::moves`) gets its box grown by `MOVED` (1 m) before it is put on the screen for batching: with the raw box, runs reordered a swaying leaf's soft edge (226 px differed in TDoD's woods close up). TDoD's grass shader also lifts by `fTilesetMeshOffsetZ` (2 m): not covered.
- An area's first reading is a plain thread in `area_view::refresh` (not `start_job`: jobs lock the window's input); textures are read apart by `Renderer::read_textures_apart` + `textures_apart` (threads in renderer.rs, `Drawn::textures_pending`, a model drawn once all its textures are read). Only with `Moonglow::background_jobs` (the desktop app); tests stay synchronous.
## Tried and ruled out
- Excluding moving draws from runs (state None): TDoD's woods programs all "move", so it fell back to ~19k pipeline switches in the turned view; growing their boxes kept the pictures identical and most of the gain.
## Watch out
- The scratchpad is shared by parallel agents: another agent overwrote `gpu.sh`. Keep scripts in a subfolder of your own.
- Compare pictures at a fixed clock: frame_perf's `FRAME_PARTS_SHOTS=<dir>` saves area_parts' last frames; area092 with the game's shaders differs by 1-5 pixels from run to run before and after (pre-existing).
- frame_perf's area_views now drops the area's view before opening it (the "on" pass used to start from the "off" pass's close camera) and clears the renderer's textures (every opening cold).
- `/proc/thread-self/schedstat` gives a thread's time on the processor without libc (frame_perf's "on the processor"), though at ~1 ms resolution.
- The app's frame loop takes `guard::take_failures()` each frame: a test of a failing model under the UI must look in the log.
