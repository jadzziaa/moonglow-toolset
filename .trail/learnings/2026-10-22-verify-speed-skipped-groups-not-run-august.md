---
title: "Verify speed: skipped groups not run, checks in parallel"
author: August
date: 2026-10-22
branch: verify-speed
touches:
  - crates/mg-module/src/doctor.rs
  - apps/mg/src/main.rs
  - crates/mg-ui/src/lib.rs
---

## Decided
- A Verify group left out (mg verify --skip, Options › Verify) is not checked at all: doctor::examine_groups skips its checks, Verify skips missing references and plugins. This replaces the 2026-10-20 decision that a left-out group is still computed and counted: the counting made --skip useless for speed (the Domains of Dread's 144 haks: 73 s with --skip tilesets,models,2das, now 0.5 s). leave_out now returns the groups only; the JSON's skipped has null for each, the summary line says 'left out, not checked'.
- examine_groups runs each custom SET/2DA/MTR/MDL's checks in parallel (rayon map_init, a per-worker 2DA cache) and a tileset's tile models too; results are collected in key order and the final sort is stable, so findings are byte-identical to the sequential run (checked on the tdod module, 17,401 findings). Keep it that way: never push findings from inside a parallel closure into shared state.
- mg verify --timings prints each part's time to stderr (never into Output, so --json stays the same), with each type's work added up and its longest resource.

## Watch out
- doctor::shaders read every custom resource (textures included) before matching the type; it now reads MTR and SHD only.
- Measuring tdod: copy the project into target/ (rsync without build/.git) and make a scratch user dir whose hak/ links to ~/Projects/tdod-module/build/hak/*.hak (as their bin/env.zsh does); the real user folder's hak/ is empty.
- What is left of a full Verify (9 s) is ASCII model lint (~4 MB/s) and tile-faces parsing each ASCII tile model again: in docs/deferred.md.
