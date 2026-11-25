---
type: Fact
title: "An item's light is the creature's effect at headconjure + (0, 0.75, 0.6); the torch raises the arm by torchl"
description: "Where the client puts an equipped item's Light property, and how the torch's raised arm (torchl) is played."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T19:59:03Z }
touches: ["crates/mg-preview/src/creature.rs:item_light", crates/mg-preview/src/compose.rs, crates/mg-preview/src/look.rs]
asks:
  - "Where does the client put an item's light on a creature?"
  - Why does a torch-bearer raise his arm and how is torchl layered?
  - "Which visualeffects row does an item's Light property use?"
  - "Does appearance.2da HEIGHT move an item's light?"
fingerprint: v2-c2fcc31a597d4044
created: 2026-11-25
author: augustaugur
---
## Decided
- Client readings 2026-11-25 (part_emitters_torch_light, 15 runs): an equipped item's Light property (torch, cloak, ring alike, the same as EffectVisualEffect(160) on the creature) is a creature effect, not a light on the item's model.
- Chain: visualeffects row [153,157,161,165,177,169,173][Param1Value color] + CostValue - 1 -> progfx row of type 4 (Param1 animation, Param3 shadows, Param7 fx_light_clr; AuroraLight01 read at the animation's end, t == length included).
- Place: the creature model's headconjure node + (0, 0.75 forward, 0.6 up) in the model's frame; turns with it, follows human, badger, ogre. appearance HEIGHT, PERSPACE, CREPERSPACE change nothing. The brightest item's light is taken (a tie is unread).
- torchl (a_ba_non_combat, left arm only) is the torch's overlay (base item 15, left hand): layered over the base's animation and the animated parts' bones (robe, cloak), not at rest. Its keys differ by about 4e-6, so a bearer is not idle_static.
- Code: mg_preview creature.rs item_light and held_overlay, look.rs DressedLook::brightest_light and light, compose.rs effect_lamps and Placed::pose_over; mg_render anim::pose_layers_scaled and pose_worn_layers. The rms log ratio against the client is 0.018 to 0.027.
## Tried and ruled out
- The light at the left hand: 0.65 of the client's far off, and the cloak and ring runs light as the torch does.
