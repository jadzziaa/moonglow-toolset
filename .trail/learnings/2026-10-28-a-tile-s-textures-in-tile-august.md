---
title: "A tile's textures in Tile Properties, the tile view and the tileset editor"
author: August
date: 2026-10-28
branch: worktree-agent-a29297f4cfc8c281f
cost: { minutes: 120, tokens: 298052 }
touches:
  - Cargo.lock
  - crates/mg-corpus-tests/Cargo.toml
  - crates/mg-corpus-tests/tests/groups/areas.rs
  - crates/mg-image/src/mtr.rs
  - crates/mg-preview/src/replaceable.rs
  - crates/mg-resman/src/lib.rs
  - crates/mg-ui/src/blueprint/replacements.rs
  - crates/mg-ui/src/images.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/nui_view/images.rs
---

## Decided
- One texture walk: `mg_preview::replaceable::uses(game, model, area_env)` lists every use (node, `Role::Slot(i)`/Environment/Particles/Flare, `Source`, the mesh's material, `drawn`), mirroring `Renderer::slots` (MTR by materialname, else named like the bitmap; MTR slots stand in for the mesh's own, slot 0 too) and `load_texture` (TXI envmaptexture, water's bumpyshinytexture; `default` = tileset EnvMap else chrome1). The Visuals page's `textures()`/`named()` is derived from it (TXI maps and mesh texture1-3 left out, bitmaps kept even when stood in for: a replacement names the model's name).
- `texture_file(game, name)` gives the layer (`ResMan::texture_layer`: class-first DDS over TGA, then PLT, then a TXI `cube 1` with faces name0-5), size, format, bytes. `origin(texture_type)` is NOT the same: override's class wins over higher-priority haks for textures.
- UI: `crates/mg-ui/src/model_textures.rs::section` (cached in ui.data by ws revision + game Arc ptr), called from tileset_view (details now in a ScrollArea), tile_select (folded, first tile's model from `props.seen`), tile_view. `origin_name` moved to images.rs.
- Doors on hooks and the .wok are not listed (door objects / not drawn).
## Watch out
- A texture row's name shows twice in accesskit (label + picture alt text): tests use query_all.
- Window content in the dock sits in a horizontal scroll area wider than the window: wrap rows at `clip_rect().right() - cursor().left()`.
- The header 'Textures of <model>' matches label searches for tile model names (fixed in tile_properties_all_tiles_search_by_model_or_number).
- TDoD's tilesets: 124 textures named and found nowhere over 40k tile models (full sweep, 35 s).
