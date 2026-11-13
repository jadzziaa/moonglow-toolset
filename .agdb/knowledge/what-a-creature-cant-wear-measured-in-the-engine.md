---
type: Decision
title: "What a creature can't wear, measured in the engine"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: ["crates/mg-rules/src/items.rs#wear_problem", crates/mg-corpus-tests/tests/engine_equipped_items.rs, "crates/mg-preview/src/creature.rs#dressed"]
created: 2026-10-29
author: August
description: "Measured in nwserver: equipped items are moved to the backpack if the creature lacks a required feat in its own FeatList, or fits no entry in a Use Limitation kind (class, race, alignment); cost and level are irrelevant."
asks:
  - "Why does a creature's equipped item end up in its backpack when I spawn it or call ActionEquipItem?"
  - "Do item level restrictions or ILR stop a low-level commoner from wearing a +5 cloak?"
  - "How do class, race and alignment Use Limitations combine when deciding whether a creature can equip an item?"
  - "Why do my OnModuleLoad test cases fetch the wrong creature with GetObjectByTag, and how do I preview armor with no feats?"
---
The engine (nwserver, OnModuleLoad probe) takes an equipped item off into the backpack, for a placed creature, one made from its blueprint and one given ActionEquipItem alike, when: the creature lacks every feat the item needs (only its own FeatList counts: nw_commale has no Simple Weapon Proficiency, so even a club comes off); or of a kind of Use Limitation (63 class, 64 race, 62 alignment group + 65 specific alignment as ONE kind) none fits. Within a kind any listed one fits. Good/lawful from 70, evil/chaotic to 30; group Neutral = neutral on either axis. Item cost/ILR and level do not matter (a +5 cloak stays on a level-1 commoner). The stock waypoint blueprints have no Appearance field (no flag): tests of a waypoint's flag must set one. Gotcha in the probe: case names became tags, and two cases with one name made GetObjectByTag read the wrong copy; the test asserts unique names. mg_preview::creature leaves such items off; an armor's own preview goes through creature::dressed(.., false), since its synthetic body has no feats.
