---
title: "Verify speed: model lint, tile faces, compiled models' first bytes"
author: August
date: 2026-10-27
branch: verify-speed
cost: { minutes: 1138, tokens: 10510422 }
touches:
  - crates/mg-mdl/src/lint.rs
  - crates/mg-mdl/src/lib.rs
  - crates/mg-mdl/src/ascii.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-resman/src/container.rs
---

## Decided
- mg_mdl::lint splits every line into words once (lint.rs `Lines`) and finds whether a duplicate node name is parented by a map of each parent name's last line: the old pass scanned every line after each duplicate, which made a 6 MB tile with 203 duplicate names (twcy1_s02_93) take 1.4 s; now 15 ms. All-ASCII text is split by its bytes (tab, LF, VT, FF, CR, space: char::is_whitespace's ASCII set, which split_ascii_whitespace is not: it leaves out VT); other text keeps split_whitespace. check_p2p looks for a `p2p` in the text before it outlines the model.
- tile-faces uses mg_mdl::faces_over(data, 10_000): an ASCII model whose `faces` lines' counts (plus words on those lines) add up to no more than the limit, and whose first `node` line is outside an animation (so ascii::read cannot fail with Empty), is not read. Only ascii::read's Empty can fail an ASCII read. The rest are read with Model::read as before.
- model_text tells a compiled model by ResMan::get_start(key, 12) (Container::read_start: an ERF reads only those bytes; other containers read all and cut); the tdod haks hold 8.5 GB of compiled models that were read whole for one byte.
- doctor::shaders reads/parses MTR and SHD files and compiles the wanted programs in parallel, then folds the results in key order (or_insert order and the findings' push order unchanged).
- Result on the Domains of Dread (145 haks, 32 threads, machine shared): 4 s to 1.4 s wall, 70 s to 25 s user; `mg --json verify` output byte-identical in every round.

## Watch out
- Measure the tdod nasher project folder (src/module.ifo.json lists 145 haks), not build/tdod.mod: the built module lists 4 haks and verifies in 0.2 s.
- To prove lint/faces unchanged, a scratch crate depended on the old mg-mdl (git archive of the base commit, package renamed mg-mdl-old) and compared check() and the face count over every MDL of every hak: 43,369 text models, 96,469 face counts, 0 differ.
- Rust string continuations (`\` at a line's end) drop the next line's leading spaces: lint's GOOD fixture has `parent NULL\norientationkey`, not `\n  orientationkey`, so a test's replace() on it can silently match nothing; assert_ne! the replaced text.

## Ruled out
- Counting a text model's faces exactly without ascii::read: the reader drops a face whose corners merge into one render vertex, and vertices merge by position, normal (computed from smoothing groups), UVs, colour, tangent and weights, so an exact count needs the de-indexing. Only the upper bound is used.
