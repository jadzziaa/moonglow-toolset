---
type: Fact
title: "The review's small slowdowns against 3.18.1: what each step was (6b26e702, dd8d0bc1) and the grass window frame left"
description: "What made area092, the grass field and TDoD's woods slower or quicker than the review's 3.18.1 build, which commits, and the bisect pitfalls met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T05:42:42Z }
touches: [crates/mg-ui/tests/frame_perf.rs, crates/mg-render/src/batch.rs, crates/mg-render/src/shadow.rs]
asks:
  - "Why do TDoD's woods take more GPU when turned than in 3.18.1?"
  - Why is the grass field close up slower than in 3.18.1?
  - How do I build old commits for a timing bisect without getting the same binary?
  - Why did frame_perf run every test when I asked for frames_at_144?
fingerprint: v2-ea7eec1aa0c44a83
created: 2026-11-26
author: augustaugur
---
Re-measured 2026-11-25/26 against v3.18.1 (a46ba28a, the review's build), alternated, three runs.
- Area092 turned: now quicker than 3.18.1 (encoded 1.86 vs 2.28 lowest).
- TDoD woods turned, Shaders off, GPU +1 ms: 6b26e702 (see-through runs kept to one chunk), confirmed by builds on each side (quickest tenth 7.27-7.90 vs 8.69-9.64). Kept: it saves 1.5-2.4 ms of encoding and that frame waits on the processor.
- Grass field close, GPU +0.15-0.2 ms: bisected to dd8d0bc1/03340a65; 03340a65 changes nothing that draws, so it is dd8d0bc1 (grass and see-through meshes take the shadows through a screen mask, client parity; its own figure 0.3-0.6 ms in area084). Kept.
- Grass field close, window frame: slower in every pair (0.98-1.02 -> 1.17-1.25 at load 10), but old/mid/new crossed at every bisect step at load 11-30; left in deferred for a calm machine (load < 5). The review called it waiting, not work.
- Gotchas: frame_perf's test filter is a substring (frames_at_144 runs every *frames_at_144 test): pass --exact. Two git-archive trees built into one CARGO_TARGET_DIR gave the same binary (second build "Finished in 0.32s": the archive's mtimes are older than the artifacts); give each revision its own target dir. A bisect rule "closer to old or new median" misclassifies when the gap is near the noise: confirm the last pair with a direct alternated A/B.
