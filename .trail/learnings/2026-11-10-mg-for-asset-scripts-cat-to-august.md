---
title: "mg for asset scripts: cat --to, --hak, ls --sha256, model info, 2da-set"
author: August
date: 2026-11-10
branch: tools-consolidation
touches:
  - apps/mg/src/added.rs
---

## Decided
- `--hak PATH` (apps/mg/src/added.rs, `Added`, flattened into which, cat, model-files and model info) takes archives and folders in one ordered list, the first given highest, at priority HAK_USER + 1 (over every hak, a module's too, under development/). Layers are labeled `<ext>:PATH` / `folder:PATH` as given, so a report names the very file. mgv has separate `--hak` and `--folder`; mg keeps one list so the order between a hak and a folder is the user's.
- `mg cat --to FOLDER` is the bulk read (not a new command): names from arguments and `--list FILE|-`, deduplicated case-blind; a name found nowhere is a report entry and fails the run after the rest are written (as single `mg cat` fails). 3,000 entries of a 1.5 GB hak: 1.3 s.
- `mg model info` computes `mg_mdl::facts::Facts` in mg-mdl (no GPU), the box from rest places composed with node scale as mg-render's `anim::compose`; skins' bones counted from weights > 0, faces via `Mesh::triangles` (Model::read's count).
- `mg 2da-set` checks column, row and value before touching the table: `TwoDa::set` silently passes over a cell the table lacks.
## Watch out
- `cargo build -p mg` makes an unoptimized target/debug/mg; `cargo test -p mg --no-run` (test profile is optimized) overwrites it with a fast one. Time with the latter (ls --sha256 of 1.5 GB: 33 s vs 4 s).
- The game's data/txpk/*.erf are empty stubs: time bulk reads on a builder's hak.
- trail check flags these files on word matches (map name key, text model face, one written two) that are unrelated learnings.
