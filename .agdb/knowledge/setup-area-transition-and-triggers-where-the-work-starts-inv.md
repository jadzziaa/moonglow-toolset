---
type: Decision
title: "Setup Area Transition and triggers: where the work starts (investigation under way)"
tags: []
status: stable
generated: { by: trail, at: 2026-11-12T00:00:00Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/tests/app/hak_closing.rs, crates/mg-ui/tests/app/main.rs, crates/mg-ui/tests/app/tiles.rs, docs/log.md, docs/manual/04-areas.md, docs/manual/12-differences.md, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock]
fingerprint: v2-dae18d92c296cfe1
created: 2026-11-12
author: August
description: "Starting-state note for adding triggers as Setup Area Transition targets: the tool in situated.rs lists only doors and waypoints and writes one object, so trigger targets and Both Ways need new work, unmeasured so far."
asks:
  - "Why doesn't Setup Area Transition list triggers as destinations, and where in the code is that decided?"
  - "How do I let a door or trigger link to a trigger target in Moonglow's transition window?"
  - "Where is the existing Aurora capture data for door-to-door area transitions that I can compare against?"
  - "Why might Both Ways fail to write the link when the target is a trigger in another area?"
---
## Decided
- Nothing about the fix or the feature yet: this is the starting state, read from the code and documents on 2026-11-12, before Aurora and the game client were measured. A later learning records the findings.
- The tool is in `crates/mg-ui/src/blueprint/situated.rs`: `transition` (the Area Transition page), `setup_transition` (the window), `destinations_in` (what it lists). `destinations_in` gathers only `Door List` and `WaypointList` entries with a non-empty tag, from every area's GIT; triggers are never listed. OK writes `LinkedTo` and `LinkedToFlags` on the current object alone (`f.set_fields`, one object).
- `docs/deferred.md`, entry "Setup Area Transition's Connection Type", already describes the missing half (Connection Type, Target Type Trigger, the target written back) with the 2026-10-30 door-to-door measurements. Work on triggers as targets should close that entry rather than add one.
- Aurora captures of door-to-door transitions exist: `~/.local/share/moonglow-oracle/captures/wave2/transitions.mod`, `transitions2.mod`, `before.mod`. A trigger as the target was not captured then.

## Tried and ruled out
- Nothing yet.

## Watch out
- One driver on the off-screen display at a time: Aurora, then the game client. Two agents sharing it would take each other's focus and screenshots.
- A builder's "invisible trigger" needs a control picture first: get an Aurora-made transition visibly highlighted by the same capture method before calling a Moonglow-made one invisible.
- Both Ways with a trigger target touches a second object, perhaps in another area's GIT that is not open or is read-only: `f.set_fields` cannot do it.
- Other sessions' uncommitted work is in this checkout: no workspace-wide `cargo fmt`, and a `MOONGLOW_TEST_OUTPUT` of one's own.
