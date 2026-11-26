---
type: Fact
title: "Aurora's creature Properties OK sorts the FeatList by name (word sort)"
description: "Aurora's creature Properties window rewrites FeatList in Windows word-sort order of feat names at every OK; Moonglow copies it via Action::SortFeats."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T20:22:01Z }
touches: ["crates/mg-edit/src/lists.rs:feats_by_name", "crates/mg-ui/src/blueprint/mod.rs:sort_feats", crates/mg-corpus-tests/tests/aurora_levelup.rs]
asks:
  - "why does Moonglow reorder a creature's feats when I press OK"
  - what order does Aurora write FeatList in
  - why did the levelup spells capture have feats in name order but the six capture not
  - does Add to Palette sort feats
fingerprint: v2-266c05e625b074ed
created: 2026-11-26
author: augustaugur
---
Captured on the Aurora oracle 2026-11-26 (captures/levelup/featsort-before/-after/-palette.mod): the creature Properties window's OK writes FeatList sorted by the feats' names even when nothing changed and the Feats page was never opened; a creature not opened keeps its order. Add To Palette opens the new blueprint's Properties window (Cancel disabled) and its OK sorts the blueprint; the placed instance keeps its order. The palette's Level Up (six-after, bandit-fighter5) appends without sorting; spells-after was levelled from the Classes page inside Properties, hence sorted. Collation is Windows word sort (hyphens and apostrophes ignored: Bullheaded < Bull's Strength, Halflings < Half-Orcs) = mg_module::new::word_sort_key. Moonglow: window_drawn_but pushes Action::SortFeats before CloseTab for Tab::Blueprint/Tab::Instance on OK, so it runs after the frame's own Apply actions (computing it at OK time would overwrite a FeatList change asked in the same frame); its own undo step 'Feats Sorted by Name'. Edit Together windows (Blueprints/Instances tabs) don't sort: Aurora has none.
