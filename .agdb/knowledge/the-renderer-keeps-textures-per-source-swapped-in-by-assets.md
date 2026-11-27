---
type: Decision
title: "The renderer keeps textures per source, swapped in by Assets::source"
description: "Why mg-render keeps one Cache per Assets source (textures, slots, materials map, surfaces, valued, particle materials) swapped in at draw_scene, keys the game's pipelines by their assembled text, and how sources are let go of."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T03:28:23Z }
touches: ["crates/mg-render/src/renderer.rs:Cache", crates/mg-render/src/renderer/kept.rs, crates/mg-render/src/assets.rs, "crates/mg-resman/src/lib.rs:Identity", "crates/mg-render/src/game.rs:Made"]
asks:
  - "why does a hak's model show another hak's texture"
  - how does the renderer tell two load orders apart
  - "when is a source's texture cache dropped"
  - "are the game's shader pipelines kept per source or by text"
fingerprint: v2-210f197d64a163e1
created: 2026-11-27
author: augustaugur
---
Textures, MTR slots and everything worked out of them were keyed by name in one Renderer, so the model viewer drawing a hak's own ResMan got the area's (or another hak's) same-named texture. Clearing on every source change was rejected: area view plus hak model would clear twice a frame and reread everything.

Design (Nov 2026): Assets::source() -> Option<&mg_resman::Identity> (ResMan has one, a process counter never reused, plus a Weak to watch it). Renderer holds cache: Cache (the frame's source), source: u64, caches: HashMap<u64, Cache>; draw_scene calls use_source() once per frame, a mem::replace when the source changes, so the hot path is one u64 compare. Per source: textures+bytes, palettes, slots, the MaterialKey->index map, surfaces(+prev), valued, particle_materials. Shared: groups, material_keys, free_materials, kept.materials (indices unique across sources), game (keyed by material index), game_pipelines (keyed by Slots address: purged when a cache is dropped, or a reused address would serve a wrong pipeline).

Game programs: GameShaders kept pipelines by PipelineKey (program names) alone, so a hak's own fswall drew with the area's. Now keyed by Made = key + hash of each program's assembled text; by_source[source][key] -> Made memoizes the assembling (once per source and key, on the UI thread). Chose text over per-source keys: a hak's ResMan reads the game's layers through, so its pairs are the same text as the area's and reuse its pipelines with no compile; per-source keys would compile everything again for each hak (and each hak edit, a new ResMan). Worker programs and value_ids are keyed by text too. forget_source drops by_source[id] and pipelines no other source's keys use; prepare_pipeline records its source so session-prepared pipelines survive.

Watch out: a wrapper Assets must forward source() (mg-ui's GameAssets does: the area view draws with game.resman but reads apart through GameAssets; a mismatch silently stops reading apart). TextureReader records its source; texture_read asks it only for that source, take_textures files results under it. Don't drop an idle cache on a timer: the area view stops drawing while a builder sits in the model viewer; dead sources go by their Weak, idle ones only by the shared stale-bytes budget (kept.rs scans every cache).
