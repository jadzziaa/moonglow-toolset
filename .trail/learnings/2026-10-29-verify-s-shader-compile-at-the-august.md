---
title: "Verify's shader-compile at the players' settings"
author: August
date: 2026-10-29
branch: worktree-agent-af3546853ed119650
cost: { minutes: 1753, tokens: 12946064 }
touches:
  - "crates/mg-module/src/doctor.rs#settings"
  - "crates/mg-shader/src/cache.rs#check"
  - "crates/mg-shader/src/lib.rs#check_assembled"
---

Verify's shader-compile (mg_module::doctor::shaders) compiles each program at six preambles (doctor::settings: High, then Performance, Low with FRAGMENT_LIGHTING 0, enhanced lighting off, NO_DISCARD 1, 3 lights) and reports a program at the first it fails at, its label carrying the setting. Only glslang judges (mg_shader::check_assembled / ProgramCache::check): Moonglow's own SPIR-V passes (split_samplers etc.) and naga are not the game's driver, so their refusals (ShaderError::Spirv/Naga) must never be reported as 'drawn magenta'. ProgramCache::check shares the renderer's cache entries (keyed on the assembled text, preamble first) and writes an entry only when the passes take the words; failures are never kept, so they are compiled again each run. The cache folder is mg_module::doctor::shader_cache_dir() (the user's cache folder's shaders/, the window's shader_cache_dir too); CLI tests set MOONGLOW_CACHE_DIR, library tests pass None. TDoD with build/hak as a scratch user dir's hak: no program fails at any setting; 0.26 s cold, 0.05 s warm. To run mg verify on TDoD, its haks are not in the real user folder: make a scratch --user-dir whose hak links to ~/Projects/tdod-module/build/hak.
