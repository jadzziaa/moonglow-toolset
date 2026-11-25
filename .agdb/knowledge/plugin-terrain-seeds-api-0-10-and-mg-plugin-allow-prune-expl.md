---
type: Decision
title: "Plugin terrain seeds (API 0.10) and mg plugin allow --prune: explicit, never automatic"
description: "Why ctx.terrain has a job seed and a per-area seed, and why stale project-plugin answers are pruned only by --prune."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T14:40:08Z }
touches: [crates/mg-plugin/src/terrain.rs, "crates/mg-plugin/src/project.rs:prune", "apps/mg/src/plugins.rs:prune"]
asks:
  - "how do I make a plugin's terrain painting reproducible"
  - why does project-plugins.toml keep answers for deleted worktrees
  - "why isn't mg pruning stale plugin allow entries automatically"
  - what did plugin API 0.10 add
fingerprint: v2-202fb3649ee172cf
created: 2026-11-25
author: augustaugur
---
## Decided (2026-11-25, builder request)
- `ctx.terrain:seed(n)` reseeds the job rng (`Shared.rng`), which `new_area` and unseeded areas draw from: new_area fills its tiles at random too, so a per-area seed alone could not remake an area made in the same run.
- `ctx.terrain:open(area, { seed = n })` gives the `Area` its own rng (`Area.rng`), so one area remakes the same whatever else the job draws. Seeds are whole numbers only. Without a seed, behavior is unchanged.
- Stale answers in `project-plugins.toml` (a project path gone) are removed only by `mg plugin allow --prune`, never on load: a project on an unmounted drive is "gone" too, and a yes is content-wide (it also allows the same files elsewhere), so a quiet prune could make other checkouts ask again. Paths that cannot be checked (`try_exists` errors) stay.
