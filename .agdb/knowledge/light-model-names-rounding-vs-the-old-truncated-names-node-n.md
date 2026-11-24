---
type: Decision
title: "Light model names: rounding vs the old truncated names; node-name limit 31 vs 32"
description: "Why lights::plan also looks up a light's pre-rounding model name and how it avoids a neighbour's row, and the unproven 31/32 node-name limit."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T01:44:51Z }
touches: [crates/mg-module/src/lights.rs, crates/mg-mdl/src/lint.rs]
asks:
  - why does lights plan look up two model names?
  - could re-placing a light add a duplicate placeables.2da row?
  - is a model node name limited to 31 or 32 characters?
  - do 0.52 and 0.53 m lights share a row?
fingerprint: v2-b7cf95ee2c9b6d32
created: 2026-11-24
author: augustaugur
---
Light placeables (mg_module::lights): a light's model name is a hash of its settings in hundredths. Before the adversarial-review branch the hundredths were truncated: in f32, 0.53*100 = 52.999996, so 0.53 m and 0.52 m (and 0.59/0.58, 1.05/1.04 ...) hashed to the same name and shared one placeables.2da row. The branch rounds now (distinct names), which would have made every existing such light miss its row and add a duplicate on re-place.

Kept: plan() looks for the row by the new name, then by name_before_rounding(), but only if that row is really this light's: its model in the hak has this radius and multiplier (model_has), or with the model missing, the row's Label is this light's label. So an old 0.53 row is reused, and a 0.53 light never takes a genuine 0.52 row. Write uses plan.model as the model's name.

Related, not settled: mg-mdl lint MAX_NODE_NAME is 32 (nwn.wiki's MDL page) but the binary writer cuts a name to 31 without a word (it keeps a NUL); since 2026-11-24 the lint warns of a name of exactly 32. No game model has a node name over 28 characters (scanned all 32,832 .mdl of the install on 2026-11-23), so the engine's handling of an unterminated 32-byte name is unproven; an engine/client test would settle it.
