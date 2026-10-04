---
type: Research Note
title: 'Persistent-world scale: budgets, and haks over 2 GiB'
description: Persistent-world scale - where builders report Aurora slows or fails, Moonglow's measurements on a large world and what they found, and haks over 2 GiB.
tags: [performance, scale, persistent-worlds, haks]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-02T05:53:59Z }
---

# Persistent-world scale: budgets, and haks over 2 GiB

October 2026. Item 8 of `community_pain_points.md`: builders report that
Aurora becomes slow or fails at persistent-world sizes:
- Inventories take 20–30 s to open with 2,500–8,000 item blueprints
  (nwn-issues #368).
- Area Properties freezes with 200+ areas (#374).
- The toolset crashes at the 152nd area.
- Haks over 2 GB can't be used.

The research map said Moonglow should measure these sizes, not assume
them.

## The world

`crates/mg-ui/tests/world/mod.rs` builds a synthetic persistent world from
the user's install. Game data isn't committed: the world is built in
`target/test-output/pw-world`, in under a second.

**The module:** Tyrants of the Moonsea grown to:
- 300 areas (the campaign's, copied);
- 8,000 item, 2,000 creature and 3,000 placeable blueprints, copied from
  the game's, the creatures and placeables on rows the haks add;
- 4,000 scripts and 1,500 conversations;
- a store holding 1,000 items.

That's 25,028 resources and 150 MB.

**Haks:** 50 of CEP's scale, plus the campaign's own four, so 71 resource
layers in all:
- 150,000 models and textures;
- `placeables.2da` and `appearance.2da` grown by 4,000 and 2,000 rows;
- the standard placeable palette grown by 10,000 blueprints;
- one hak over 2 GiB (a sparse file).

With the game's own resources, that's 300,146 in the resource browser.

## Measurements

From `crates/mg-ui/tests/world_perf.rs`, run in release on the
development machine (Ryzen 7 5800X3D, Radeon RX 9070). Budgets are three
to five times the measurement and are only tightened (`PLAN.md` §7).

| Step | Measured | Budget |
| --- | --- | --- |
| Open the module and its 54 haks | 0.09 s | 1 s |
| Frame with the module open (300 areas in the tree) | 1.2 ms | 33 ms |
| Custom item palette (8,000), first frames | 47 ms | 0.5 s |
| …with every one of the 8,000 shown | 13 ms a frame | 33 ms |
| Standard placeable palette, 10,000 hak blueprints shown | 16 ms a frame | 33 ms |
| A store's inventory, 1,000 items (449 on its first page), first frames | 0.68 s | 2 s |
| …afterwards | 6 ms a frame | 50 ms |
| A placeable's 20,500 appearances, listed | 7 ms a frame | 50 ms |
| A creature's 17,100 appearances, listed | 5 ms a frame | 50 ms |
| Area Properties, first frames | 8 ms | 0.5 s |
| The 300th area in the 3D view | 0.14 s | 1 s |
| The resource browser (300,146 resources), first frames | 0.14 s | 1 s |
| Verify | 2.3 s | 6 s |
| The content doctor | 0.9 s | 3 s |
| Where-used of an item blueprint | 0.8 s | 2 s |
| Looking for changed haks (Reload Resources) | 0.6 ms | 100 ms |
| Script references of `GetObjectByTag`, all 4,000 scripts | 78 ms | 1 s |
| Compile all 4,000 scripts | 1.4 s | 15 s |
| Write the module | 27 ms | 1 s |

Each view was checked by a screenshot (`WORLD_SHOTS=1`) to be the one
timed: the store on its Inventory page, the palettes open, the
appearance lists dropped down.

The store's 0.68 s is its first look at 449 item icons. Each is colored
from its PLT once and kept, so the page shows at once after that.

## What the measurements found

### Haks over 2 GiB: the game doesn't read them either

The wiki says to keep haks under 2 GB because Aurora and nwsync don't read
past it. It doesn't say what the game does.
`crates/mg-corpus-tests/tests/engine_big_hak.rs` builds haks of 2.1 GiB
(sparse files, so they cost no disk space) and runs them in `nwserver`:

- **What starts past 2 GiB is missing to the game.** A resource whose
  data starts 2³¹ bytes or more into the hak can't be read: a 2DA reads
  empty and a script doesn't run.
- **What starts before the mark is read whole,** even if it ends past
  the mark.
- **A lower hak's copy doesn't take over.** The game finds the resource
  in the big hak, fails to read it, and doesn't look in the haks below.
  A 2DA there reads empty even when a lower hak has a good copy.

The offset is evidently read as a signed 32-bit number. Moonglow matches
the game, and tells the builder:
- **The resource manager** (`ERF_READ_LIMIT` in `mg-resman`) lists those
  resources, and reading them fails with "starts past 2 GiB, where the game
  stops reading". Previews and checks see what the game sees.
- **The content doctor's `erf-size` check** names the hak and the
  resources the game can't read, as an error.
- **`mg pack`** warns when it writes an archive with resources past the
  mark, and streams the archive to disk instead of building it in memory.

### Find didn't open palette categories

In the palettes and the module tree, Find (Filter) opened only the
categories you hadn't looked at yet. egui's `default_open` applies only
the first time a header is shown, so a category seen closed stayed closed,
its matches hidden. Filtering now opens every category with a match.
The blueprint pickers already did.

## Not covered

- **Real CEP haks.** The haks here are synthetic: tiny models and
  textures. Their resource counts are CEP's, but the models' sizes aren't.
  Loading real ones is covered by the campaign budgets
  (`campaign_perf.rs`) and the model corpus tests.
- **Other systems.** Only the development machine was measured. The test
  needs the game and a GPU, so CI doesn't run it.
- **Item icons:** the store and inventory lists still lay out every row,
  so a page of thousands of items would build thousands of icons the first
  time.
