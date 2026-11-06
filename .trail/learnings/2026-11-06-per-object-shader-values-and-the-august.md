---
title: Per-object shader values and the TDoD tint
author: August
date: 2026-11-06
branch: develop
cost: { minutes: 37, tokens: 435864 }
touches:
  - crates/mg-area/src/lib.rs
  - crates/mg-area/src/scene.rs
  - crates/mg-module/src/areas.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-preview/src/compose.rs
  - crates/mg-preview/src/lib.rs
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/lib.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/scene.rs
---

## Decided
- The renderer is generic: `Instance::material_values` (an object's values for its MTRs' parameters, in order, later wins; `material: None` = every MTR declaring the parameter) and `Environment::scriptable` (`scriptableVec1..16`, Key::ScriptableVec, a frame value). The TDoD convention (locals `tdod_tint`, `tdod_tint_<mtr>`, area GIT `tdod_wallpaper`) lives in `mg_rules::tint` alone; mg-preview's `of_object` turns locals and the object's saved `Material` › `ShaderParams` into values (skipped for a static placeable: the client bakes it as made).
- A picked tint is set when the picker closes (one undo step), as Form::color does; the area view redraws on the workspace revision.

## Watch out
- The game path keeps a material's uniform template by `d.params.as_ptr()`: valued Slots must be kept (`Renderer::valued`, cleared with `slots`), never rebuilt per frame, or templates alias or grow.
- The renderer reads MTRs from game.resman (base, haks, override), not from the open module's own files: a UI test's MTR goes in a scratch user folder's override (`blueprint_harness_with`).
- `plc_chest1` is model `plc_a08`; its textures are plc_woodb256, plc_rck, plc_wood256, plc_rugdmtlbrk.
- Moonglow's WGSL fallback draws no custom shader's parameters, so no tint with Shaders off.
