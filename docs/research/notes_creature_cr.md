---
type: Research Note
title: Creature challenge rating (Aurora's calculation)
description: How Aurora computes a creature's challenge rating (the engine only returns the stored value), worked out from probe creatures and checked against Aurora; implemented in mg_rules::challenge.
tags: [creatures, challenge-rating, aurora, rules]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-01T04:34:01Z }
---

# Creature challenge rating (Aurora's calculation)

Aurora computes a creature's `ChallengeRating` when its Creature Properties
open (Basic › Challenge Rating) and stores it on OK, and recomputes every
blueprint's in Build › Build Module › Compile › Creature CR (on by default).
The engine does not: `GetChallengeRating` returns the stored value (a
creature spawned with 42 stored reports 42), so Aurora is the only oracle.
Moonglow's implementation is `mg_rules::challenge` (`GameData::challenge`).

## Where it comes from

Black-box fitting on probe creatures (below) got the structure (hit points
per class hit die, natural AC, feats, racial modifiers, rounding above 1,
fractions below) but not a formula that matched every probe. The
calculation was then read from `nwtoolset.exe` (89.8193.37): the function
that reads `CRValue` (feat.2da), `CRModifier` (racialtypes.2da), `HitDie`,
`SavingThrowTable`, `Innate` (spells.2da), `WALKRATE`/`MOVERATE`
(creaturespeed.2da, appearance.2da) and fractionalcr.2da `Min` and
`Denominator`, at 0x52EDE0–0x531874 with the fraction step at 0x531C58.
Only the arithmetic was taken from it; the result is checked against
Aurora's own output.

## The calculation

L is the sum of the class levels (classes with a level above 0). Each term
is computed and added to a single-precision running sum:

| term | value |
|---|---|
| level | 0.15 L |
| natural AC | 0.1 × NaturalAC |
| gear | 0.2 L × V / (20000 L + 100000), V the gear's value in gold |
| hit points | 0.2 L × HitPoints / E × walk / PC walk |
| abilities | 0.1 L × (Str + Dex + Con + Int + Wis + Cha) / (L + 50) |
| special abilities | 0.15 L × S / (L(L + 1) + 5L), when S > 0 |
| spells | 0.15 L × P / (L(L + 1)), when P > 0 |
| saves | 0.15 L × (B + fortbonus + refbonus + willbonus) / B |
| feats | 0.1 L × F / (L/2 + 7) |

- **E**, the expected hit points: for each class, level × (HitDie + 1) / 2
  added to the total and truncated (Fighter 5: 27). `HitPoints` is the base
  (rolled) hit points; Constitution's do not count.
- **walk**: creaturespeed.2da `WALKRATE` of the row whose `2DAName` is the
  appearance's `MOVERATE` (a human's NORM: 1.75). **PC walk** is row 0's
  (2.00). The code looks at a movement rate first and goes to the
  appearance for the `Default` row, but the creature's `MovementRate` makes
  no difference: rates 0 to 8 give the same ratings, a deer (FAST) the
  same with rates 0, 4 and 7.
- **V**: the value of the equipped items (`Equip_ItemList`); carried items
  (`ItemList`) do not count. Moonglow values them with its item cost
  (`GameData::item_cost`).
- **S**: the spells.2da `Innate` levels of `SpecAbilityList`.
- **P**: the `Innate` levels of the spells each class casts from:
  `MemorizedList0`–`9` for classes that memorize (classes.2da
  `MemorizesSpells`), `KnownList0`–`9` for the others (a wizard's known
  spells count nothing, a sorcerer's do); plus the special abilities
  again, which so count in both terms. Cantrips add nothing (Aurora adds
  0.5 and truncates).
- **B**: the base saves (Fortitude + Reflex + Will) of every class's
  SavingThrowTable at its level; 1 if they add up to 0.
- **F**: feat.2da `CRValue` of each feat, read as an integer (so `0.5` and
  `0.2`, most feats, count 0 and `1` counts 1); a feat with no CRValue
  (`****`) counts 1.

Then the sum is multiplied by racialtypes.2da `CRModifier` of the race (if
it has one), and rounded:

- above 0.75: if below 1.5, 0.25 less; then to the nearest whole number,
  halves down (the fraction must exceed 0.5 to round up);
- at most 0.75: 0.35 less, then rounded the same way.

`CRAdjust` is added to both the value and the rounded number. If the value
is then above 0.75, the rating is the rounded number; otherwise it is a
fraction: the first fractionalcr.2da row whose `Min` the value reaches
(0.40 → 1/2, 0.30 → 1/3, 0.20 → 1/4, 0.15 → 1/6, else 1/8). A creature
without class levels is rated 1/8.

Not settled: the hit die is the larger of the class's `HitDie` and a value
Aurora takes from a list on the creature that no probe filled (0 for
them); special abilities with several classes (counted once here).

## Checked against Aurora

`crates/mg-corpus-tests/tests/aurora_creatures.rs`
(`challenge_ratings_match_aurora`) rates the 4182 probe creatures of four
captures and compares with what Aurora's Build stored; all match. The
creatures are the bandit
`nw_bandit001` made plain (no feats, abilities 10, natural AC 0, Fighter 5,
base HP 25) with one thing changed, made by
`crates/mg-corpus-tests/examples/cr_probe2_module.rs`:

- `creature-cr-build.mod` (default mode): levels 1–40, hit points, natural
  AC 1–30, feats (CRValue 1, 0.5, 0.2), special abilities 1–10, each
  ability, every class at levels 1, 5, 10, every race, save bonuses,
  skills, sizes, several classes;
- `creature-cr-hp.mod` (`hp`): hit points 1–300 against levels 1–20 for
  hit dice d4–d12;
- `creature-cr-fine.mod` (`fine`): the same in single steps, and levels
  1–40 at 1 hit point;
- `creature-cr-terms.mod` (`terms`): movement rates 0–8, a fast appearance,
  single special abilities, wizard and sorcerer spell lists, equipped and
  carried items, feats without a CRValue, each over hit points 1–90 (where
  the rating steps up shows the change's size).

## Earlier probes

`creature-probes.mod` (`examples/cr_probe_module.rs`, the bandit with base
HP 10, Str 15, Dex 13, Con 12 and its 11 feats) holds 22 creatures opened
and confirmed in Creature Properties; `aurora_creatures.rs` checks their
maximum hit points.
