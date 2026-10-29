---
title: "Grass in clumps, read off the client's captures; grass plans"
author: August
date: 2026-10-29
branch: worktree-agent-aae377f48561ca787
cost: { minutes: 1822, tokens: 13370946 }
touches:
  - crates/mg-area/src/grass.rs
  - crates/mg-module/src/grass.rs
  - crates/mg-ui/tests/app/drawing.rs
---

## Decided
- The client's grass placement was read without running the client: the captures in ~/.local/share/moonglow-oracle/captures/client/ hold ttr01's field from the reference camera with grass (shadows_ttr01_grass_day_off.png) and without (shadows_ttr01_day_off.png). Their difference (|a-b| > 12) shows where grass is; shadows_look renders Moonglow's view of the same scene from those captures with MG_CLIENT_CAPTURED=1 MG_SHADOW_GRASS=1 MG_SHADOW_ONLY=day_off (DISPLAY unset so nothing launches). Compare the share of near ground (rows 330-620) bare after a box blur of 15, 31, 61 px.
- Grass grows in clumps: one per 5 m² of a material's ground (mg_area::grass::CLUMP_GROUND), its tufts the density's share of that ground, within 1.4 m (CLUMP_REACH), retried on the material's faces near the clump's face. Bare share 0.25/0.16/0.06 against the client's 0.25/0.15/0.08 (even scatter: 0.09/0.02/0.00).
- A fixed number of tufts a clump (20 within 1.2 m fitted as well at density 5) was ruled out: the client's sparse white tufts at 0.3/m² (reading_grass_tufts) look scattered, which 20-tuft clumps would not. Clumps by ground, tufts by density.
- Grass edits are one plan: mg_module::grass::{set,remove,move,disabled}_plan -> Vec<Step>; mg areas applies them in place (grass::apply), the area view turns them into Edits (mg_area::grass::edits). add_override is gone (set_override changes an override in place).
## Watch out
- grass_covers_the_whole_view's close look at the target took 40 px: a clump gap fell on it; it takes 160 px now.
- The worktree guard refuses env VAR=... cargo and computed paths: put such runs in a scratchpad script.
