---
type: Decision
title: "Moonglow Client's mc-scene on values: the wear check confirmed, and how the port was accepted"
description: "Whether mc-scene's creatures lost their items to the wear check (yes: shields and armor past cloth under TDoD's tables), what Look::creature takes since the join, and how a client port is accepted against this worktree (three builds, model dumps, pictures)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T13:21:34Z }
touches: ["crates/mg-area/src/values.rs:Look::creature", crates/mg-area/tests/creature_values.rs, crates/mg-preview/src/look.rs]
asks:
  - "does mc-scene draw a creature's shield or armor"
  - "how do I give Look::creature a dressed creature"
  - how was the Moonglow Client port checked against the toolset
  - "why does a client's creature lose its items when read as fields"
fingerprint: v2-bed70fbb59ed57b8
created: 2026-11-19
author: augustaugur
---
- Confirmed 2026-11-19 with a scratch copy of Moonglow Client (bf99614) at its pin v3.18.1 and repointed at this tree: mc-scene wrote creatures as GIT fields with no FeatList, and AreaModel::read applies the wear check, so under The Domains of Dread's tables a tower shield (base item 57, Shield Proficiency: feat 32) and any armor of class 1 and up (torso part AC: feats 3/4/2) were not drawn; weapons were, only because TDoD's baseitems.2da asks no feat of the longsword or dagger (the base game's would drop them too). The Shard's own character wears cloth (AC 0), so its picture never showed it.
- Since the join, values::Look::creature(game, impl Into<DressedLook>) draws through mg_preview::dressed_look: everything given is worn, no wear check (a client is never told feats). A bare CreatureLook, given or borrowed, still works (From<&CreatureLook>). Look::read is unchanged: every installed area's model hashed the same before and after (1461 areas, AREA_DUMP), and tests/creature_values.rs holds values == fields with the feats, fields without them == the bare body.
- Acceptance method that isolated the port from toolset drift: (a) the client at its tag, (b) the same repointed at the worktree by path deps (workspace mg-* lines), unported, (c) ported. Model Debug dumps and pictures of fixtures/mgc-shard-idle and mgc-bar-tour: (a)/(b) models identical but pictures differ (the mist particles, renderer drift 3.18.1 to 4.2), (b)/(c) pictures pixel-identical, models identical but the Shard character's rotation -pi vs pi (Object::facing against the old cos/sin fields). Each run deterministic (a twice, c twice: 0 pixels).
- Port pitfalls: Tile::new has replace_texture None but a file's tile without Tile_ReplaceTex takes row 0 (the Shard's tms01 has the one replace_tex model): give Some(0) for parity; creatures before placeables (index by kind); a workspace-level default-features = false for mg-area (a member's alone is ignored).
