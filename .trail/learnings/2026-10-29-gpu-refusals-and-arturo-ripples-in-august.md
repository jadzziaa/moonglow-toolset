---
title: GPU refusals and arturo ripples in the renderer
author: August
date: 2026-10-29
branch: worktree-agent-aa064cc510057ad52
cost: { minutes: 90, tokens: 14100302 }
touches:
  - crates/mg-render/src/guard.rs
  - crates/mg-render/src/texture.rs
  - "crates/mg-render/src/renderer.rs#posed_box"
  - docs/deferred.md
---

wgpu 30's default uncaptured-error handler panics synchronously on the calling thread (wgpu_core.rs default_error_handler), so a guard::guarded step already caught a GPU validation error; what was not caught was the main-thread texture path and draw-time errors. Error scopes are per thread (game::checked); texture uploads now go through GpuTexture::try_upload under them. Only the app binary installs guard::note_gpu_errors on eframe's device: tests keep wgpu's panic, and probe.rs sets its own handler, so it can't live in Gpu::new. To provoke a real refusal in a UI test: egui_kittest's device takes max_texture_dimension_2d 8192 while mg-image reads up to 16384 (MAX_SIDE), so a 16384x1 TGA in override is refused; a 65535-wide one is refused by mg-image first and never reaches the GPU. GPU errors are said once per process (guard keeps every one said): a test that reopens an area won't see them again, so take gpu::alone() before the harness. Culling a skinned mesh: posed_box (each bone's copy of the bound box; the game's programs don't normalize weights, Moonglow's shader does, hence GpuSkin.weighed and unweighed). Arturo (TXI proceduretype arturo): the wiki says distort 1 ignores channelscale/channeltranslate and distortionamplitude is in texels; the client computes it on the CPU (nwmain-linux symbols: ArturoTextureController::BuildArturoImage, Control). Reading its code was left to the owner; the deferred entry lists the client readings instead.
