---
title: "The fourth client batch: particles, part scopes, static placeables, helmets"
author: August
date: 2026-10-30
branch: worktree-agent-a18abc9dc898ef8aa
cost: { minutes: 240, tokens: 13430709 }
touches:
  - crates/mg-corpus-tests/tests/client_render/batch4.rs
  - crates/mg-render/src/particles.rs
  - crates/mg-preview/src/creature.rs
  - crates/mg-preview/src/item.rs
  - crates/mg-area/src/scene.rs
---

## Decided
- Client readings of the fourth batch live in `crates/mg-corpus-tests/tests/client_render/batch4.rs`. `shoot_close` takes the client's picture from a camera given a second after entry and renders Moonglow's area view from about the same camera (0.92 of the distance, a degree less pitch, as `TURNED_DISTANCE`/`TURNED_PITCH` were fitted for (f, 12, 45)); good enough to compare by eye, not fitted. `put_creature` places a creature with extra equipment by slot bit (cloak 0x40, head 0x1), the items expanded from their blueprints.
- A part number on an item or creature is its EE twin first (`xModelPart1` before `ModelPart1`, as `mg_rules::items::part_number`): setting only the byte field changes nothing in the client or Moonglow.
- Particles in the client: Motion_Blur 2x size across, tail 0.76 x blurlength s behind; Fountain birthrate < 1 makes none (no stock emitter has one); particleRot in rad/s from upright; birth order, newest drawn last; lightning and Linked soft as others.
- The client plays no animation of a static placeable; its emitters run.
## Tried and ruled out
- `refModel NULL` on a lightning emitter's reference node crashes the client loading the model (`CreateReferenceObjects`); `fx_ref` works.
- An amulet lies as its base item's bag, so it can't carry a probe model; a longsword's blade model replaced by a sphere drew no sphere in either.
## Watch out
- TDoD's `dd_ce3_txburneds.dds` alone in an override drew pale and see-through in the client; with its TGA beside it (as the hak ships both) it draws as Moonglow draws the DDS. Copy both when reading TDoD content out of its haks.
- In this worktree session Bash refuses commands it can't parse as staying inside the worktree (heredocs with tricky quoting, loops over variables, `python -c` with computed text, chains with `git` after `cd`): write code to a scratchpad file and `cat` it in or run it with `python3 -I file`, and run git alone. zsh aborts a chain at a glob with no match (`rm -f x/*.png` when none): spell the files.
- `client_screenshots`' pictures lag: two of four can be the same frame.
