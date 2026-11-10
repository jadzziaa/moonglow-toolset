---
title: "mg for asset scripts: cat --to, --hak, ls --sha256, model info, 2da-set"
author: August
date: 2026-11-10
branch: tools-consolidation
touches:
  - apps/mg/src/added.rs
  - apps/mg/src/texture.rs
  - crates/mg-image/src/encode.rs
  - crates/mg-image/src/adjust.rs
---

## Decided
- `--hak PATH` (apps/mg/src/added.rs, `Added`, flattened into which, cat, model-files and model info) takes archives and folders in one ordered list, the first given highest, at priority HAK_USER + 1 (over every hak, a module's too, under development/). Layers are labeled `<ext>:PATH` / `folder:PATH` as given, so a report names the very file. mgv has separate `--hak` and `--folder`; mg keeps one list so the order between a hak and a folder is the user's.
- `mg cat --to FOLDER` is the bulk read (not a new command): names from arguments and `--list FILE|-`, deduplicated case-blind; a name found nowhere is a report entry and fails the run after the rest are written (as single `mg cat` fails). 3,000 entries of a 1.5 GB hak: 1.3 s.
- `mg model info` computes `mg_mdl::facts::Facts` in mg-mdl (no GPU), the box from rest places composed with node scale as mg-render's `anim::compose`; skins' bones counted from weights > 0, faces via `Mesh::triangles` (Model::read's count).
- `mg 2da-set` checks column, row and value before touching the table: `TwoDa::set` silently passes over a cell the table lacks.
- Texture encoding (crates/mg-image/src/encode.rs) scores every choice with bc.rs's own palettes (`color_palette`, `channel_palette`); alpha is scored against the rounded-down AND the exact values, or it loses to Neverblender's encoder under an exact decoder (measured: docs/research/notes_texture_encoder.md). Opaque DXT1 is four-color only (c0 > c1, or c0 == c1 with index 0): index 3 is transparent black, which the game draws black.
- BioWare DDS alpha mean = the largest level's mean alpha (checked on 603 game DXT5 files), 1.0 for 3-channel. Both DDS containers hold the same rows (bottom first); PNG is turned over at the edge only (`png::read_picture(..).top_down()`, `write_rgba(top_down)`).
- `mg texture convert` copies a DDS's blocks when its format and size are kept (`reencoded: false`), encoding only the levels it lacks; `Texture::to_rgba` forces DXT1 opaque (what the game draws), so conversion reads `stored_rgba`.
## Watch out
- `cargo build -p mg` makes an unoptimized target/debug/mg; `cargo test -p mg --no-run` (test profile is optimized) overwrites it with a fast one. Time with the latter (ls --sha256 of 1.5 GB: 33 s vs 4 s).
- The game's data/txpk/*.erf are empty stubs: time bulk reads on a builder's hak.
- trail check flags these files on word matches (map name key, text model face, one written two) that are unrelated learnings.
