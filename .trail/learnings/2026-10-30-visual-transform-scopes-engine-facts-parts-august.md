---
title: "Visual transform scopes: engine facts, parts drawn about their node"
author: August
date: 2026-10-30
branch: worktree-agent-a02be1f76ba28c11f
cost: { minutes: 90, tokens: 4102845 }
touches:
  - crates/mg-preview/src/compose.rs
  - crates/mg-module/src/placement.rs
  - crates/mg-area/src/edit.rs
---

## Decided
- Engine (engine_visual_transforms.rs, the_engine_keeps_part_scopes_animation_speed_and_lerps): a VisTransformList with only a head entry (254) leaves scope 0 at 1; AnimationSpeed is read; a script lerp saves TimerType 1, ValueFrom, LerpDuration (seconds), LerpProgress (fraction 0..1), BehaviorFlags, RepeatsRemaining, and a module with that entry reads its goal and resumes from where it stood.
- placement::visual_transform no longer falls back to the list's first entry: that drew a head-scaled creature whole at the head's scale.
- Part scopes are drawn via mg_preview::Part::local (scope_matrix), set in creature.rs on head/helmet (254), tail (253), wings (252), cloak (243). Placed::hung returns (drawn, bones): a skin bound to the base skeleton must take its pose from the hung WITHOUT local, while the instance uses the hung WITH it, so the skin is transformed about the node. Applied in posed, stance and lamps_standing alike.
- Adjust Location edits a scope-0 entry axis by axis (visual_transform_edits_with): unchanged axes keep lerps/timers; AnimationSpeed is a field.
## Watch out
- nw_bandit001 is appearance 269 (one model, no head part): use CreatureLook::new(6).to_utc() fields for a part-based creature in tests.
- The client convention for part scopes is assumed (same matrix as scope 0, about the attach node); the deferred entry says what the client batch should read.
