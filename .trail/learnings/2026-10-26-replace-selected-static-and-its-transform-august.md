---
title: "Replace Selected, Static and its transform, lighting schemes in mg areas"
author: August
date: 2026-10-26
branch: area-objects-and-schemes
cost: { minutes: 3, tokens: 34928 }
touches:
  - crates/mg-corpus-tests/tests/groups/aurora.rs
  - crates/mg-module/src/areas.rs
  - crates/mg-module/src/new.rs
  - crates/mg-ui/src/area_props.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/04-areas.md
  - docs/manual/05-blueprints.md
  - docs/manual/11-command-line.md
---

## Decided
- Aurora has no Replace Selected (its DFM inventory's palette and viewer menus have none); its Update Instances is the equivalent and is captured (`update-aurora-*.mod`). Replace Selected therefore remakes through `mg_module::instances::remade` (Update Instances' own path): place, facing, outline (`Geometry`), `VisTransformList`/`VisualTransform` and `SpawnPointList` kept; tag, name, scripts, variables from the blueprint. `replaced` adds one rule: a placeable the blueprint makes static loses its transform. Triggers and encounters are replaced now.
- Static ticked in a placeable's Properties (`Form::check_taking`, so multi-object Properties and Edit Together too) removes `VisTransformList`/`VisualTransform` in the same command; unticking gives nothing back, Undo does. Make Static and `mg areas --static-placeables` already did.
- Aurora's Area Properties › Visual scheme, on an existing area (captured: `lighting-scheme.mod` from `lighting-scheme-probe.mod`, test `aurora_lighting_scheme.rs`): the Area Wizard's fields from the row, plus FogClipDist 45.0 and SkyBox 0, and every tile's lights picked anew (Src2 = Src1). `Scheme::applied_fields` is that; `Scheme::tile_fields` a tile's four.
- `mg areas --lighting-scheme` (row, LABEL or name via `Scheme::find`) is applied before `--set`. Give Every Area This Page's Visual Settings re-picks tile lights only for areas whose scheme differs from the first's.
## Tried and ruled out
- Copying the first area's tile lights tile by tile to the others: areas differ in size and tiles.
## Watch out
- `Scheme::read` now refuses a row past environment.2da (it used to give zeros); callers that `.ok()` it (terrain, reshape, plugin terrain) get None and lights 0, as before.
- A test placing custom blueprints in a module made by `new_module` must call `palette::rebuild_custom_palettes`, or the custom palette shows none.
