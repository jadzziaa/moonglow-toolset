---
type: Finding
title: Findings
description: What Moonglow's authors learned about NWN:EE that is undocumented or documented wrongly - item costs, creatures, areas and tiles, talk tables and 2DAs, haks, GFF fields, what Aurora writes - each saying how it was checked, where Moonglow implements it and what nwn.wiki says.
tags: [findings, engine, aurora, gff, 2da, nwn-wiki]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-29T15:26:52Z }
sources:
  - id: engine
    resource: The game itself (NWN:EE 89.8193.37) - a private nwserver, and the game client read back by screenshot; tests engine_* and client_* in crates/mg-corpus-tests/tests
    title: Checks in the game
  - id: aurora
    resource: The Aurora toolset 89.8193.37 (EE 1.89) under Wine, its output compared field by field; tests aurora_* in crates/mg-corpus-tests/tests
    title: Checks against Aurora
  - id: neverwinter-nim
    resource: https://github.com/niv/neverwinter.nim
    title: neverwinter.nim's tools (differential checks)
  - id: nwn-wiki
    resource: https://nwn.wiki
    title: nwn.wiki, searched through the local mirror synced 2026-09-29
---

# Findings

What Moonglow's authors learned about Neverwinter Nights: Enhanced Edition
while building it, that we could not find documented elsewhere, or found
documented wrongly. Each finding was checked against the game (a private
`nwserver`, or the game client read back by screenshot), against Aurora
(the original toolset, 89.8193.37 / EE 1.89, run under Wine, its output
compared field by field), or against neverwinter.nim's tools. Each says how
it was checked, where Moonglow implements it, and what the
[NWN wiki](https://nwn.wiki) says, so others can verify it and use it.

Tests are in `crates/mg-corpus-tests/tests/` unless a path is given:
`engine_*` run the game's server, `client_*` the game client, `aurora_*`
compare with files Aurora wrote. Aurora's captures and the game's data are
not in the repository; the tests read them from the tester's machine.

The wiki was searched through a local mirror (synced 2026-09-29) and the
NWN Lexicon. Where a finding contradicts the wiki, `docs/wiki-proposal.md`
proposes the correction, page by page. BioWare's format PDFs, linked from the wiki, could not be
searched; a few facts here may be in them.

## Items and costs

### An item's value (`GetGoldPieceValue`, UTI `Cost`)

All arithmetic is single-precision float.

1. **Base cost:** `baseitems.2da BaseCost`. For armor (`ModelType` 3),
   `armor.2da COST` at the row given by `parts_chest.2da ACBONUS` of
   `ArmorPart_Torso`, rounded (a blank one, row 0: below).
2. **Passive properties** (all but Cast Spell, `itempropdef` row 15): each
   costs (`itempropdef Cost`, or if empty the subtype table's `Cost` at the
   property's `Subtype`) × (the cost table's `Cost` at `CostValue`; the
   table is the `iprp_costtable.Name` row named by `CostTableResRef`, factor
   1 if that is 0 or empty). Positive costs sum into P, negative ones (as
   magnitudes) into N.
3. **Each spell:** `iprp_spells.Cost[Subtype]` × `iprp_chargecost.Cost[CostValue]`.
   Only for `CostValue` 2–6 (5 down to 1 charges per use), and only if
   `Charges` > 0, multiply by Charges / 50. Single Use (row 1), 0
   Charges/Use (row 7) and the per-day rows are never scaled.
4. **Spell total:** half of every spell's cost, plus first/2, plus
   second/4, where first and second come from one pass in property order
   that is buggy: a cost above first becomes first *without demoting the
   old first to second*; otherwise a cost above second becomes second. So
   the runner-up's quarter counts only if it comes after the dearest spell.
5. **Totals:** plus = trunc(base + spell total) + trunc(P² × 1000); minus =
   trunc(N² × 1000); unit = trunc(max(plus − minus, 0) × `ItemMultiplier`);
   value = (unit + `AddCost`) × `StackSize`. AddCost counts once per item,
   and the per-item value is truncated before the stack multiplies it.
6. **What scripts see:** 0 for plot items, creature-slot items (claws,
   bites, hides) and gold (base item 76); otherwise at least 1 (99 plain
   arrows compute 0 but report 1). An unidentified item reports
   trunc(`BaseCost` × `ItemMultiplier`) × stack, without properties or
   AddCost, and can be 0.

- **Checked:** engine, `engine_item_cost.rs` `item_costs_match_the_engine`:
  every base-game item (2,763) created in `nwserver`, values compared, all
  equal; made-up wands pin the charge rules (Aid: single use 600; 3
  charges/use with 10 charges 585; 0 charges/use 13,500). 16 base items
  show the top-two bug (`x0_it_mneck002`: 6526, against 7538 for a true top
  two). Stacked magic ammunition values are exact multiples of the stack
  (1881 = 19 × 99). Negative costs and the zero-charges case are checked
  only by unit tests (`crates/mg-rules/src/items.rs`); no base item has them.
- **Moonglow:** `crates/mg-rules/src/items.rs` (`GameData::item_cost`,
  `property_cost`, `spell_cost`).
- **Wiki:** incomplete and partly wrong. The "Item Costs" section of the
  itemprops/itempropdef/iprp_costtable page says "the two highest are also
  added on again" (wrong when the cheaper spell comes first), names the
  table "iprp_charges.2da" (it is `iprp_chargecost`), and omits which rows
  are scaled, the separate truncations, AddCost per item and the minimum
  of 1.

### A torso with no `ACBONUS` is clothing

An armor whose `ArmorPart_Torso` names a parts_chest.2da row with a blank
`ACBONUS` (row 0 and row 2 of the game's table) or a row the table lacks
(250) has armor class 0, as a torso of `ACBONUS` 0.00 has: `GetItemACValue`
0, `GetWeight` 10 (1.0 lb) and `GetGoldPieceValue` 1, its `Cost` 1, which
are armor.2da row 0's `WEIGHT` and `COST`. A plain armor with torso 4
(4.00) gives 4, 300 and 100. Moonglow had read a blank as no armor class:
no weight on the General page and a base cost of 0 (a builder's report on
2.0.0); it now reads a blank or missing row as 0.

- **Checked:** engine, `engine_armor_torso.rs`
  `an_armor_whose_torso_has_no_armor_class_is_clothing` (`nw_aarcl001`
  with no properties, torsos 0, 1, 2, 4 and 250; 2026-10-26, EE
  89.8193.37).
- **Moonglow:** `crates/mg-rules/src/items.rs` (`GameData::armor_class`),
  `crates/mg-ui/src/blueprint/item.rs` (`statistics`).
- **Wiki:** the armor.2da page says parts_chest.2da's `ACBONUS`, rounded,
  is the row of armor.2da the armor takes; what a blank cell or a missing
  row gives is missing.

### Created items are priced anew

`GetGoldPieceValue` straight after `CreateObject` already equals the value
above, not the blueprint's stored `Cost`, which is stale in about 9% of
shipped blueprints (`nw_it_mbelt016` stores 14440 and reports 14441;
`nw_wammar002` stores 1940, from multiplying the stack before truncating,
and reports 1881).

- **Checked:** engine, the log of `engine_item_cost.rs` (logged, not
  asserted). Placed instances and store stock were not probed.
- **Wiki:** the Item Costs section says the game doesn't recalculate the
  value "dynamically… unless…" and does not list object creation.

### Stacks and charges

A 5,000-arrow stack stays 5,000 though the base item stacks to 99; a UTI
with `Charges` 255 reads back as 250.

- **Checked:** engine, `engine_ee_fields.rs`
  `the_engine_reads_the_ee_fields_aurora_has_none_for`.
- **Wiki:** the Item page says large stacks "can be defined"; the load-time
  clamp to 250 is not documented.

### The Item Wizard's random properties (Magical)

Aurora's Item Wizard has **Magical** on its Name page; ticked, it enables
**Item Level** (1-5, 6-10, 11-15, 16-20) and **Item Quality** (Low,
Medium, High, Godly), and Next waits for both. The item gets random
properties from the toolset's `itmwiz*.2da` (columns `Prop SubProp
CostTable CostVal ParamTable Param Value`; `Value` is a tier, 0 to 8 for
weapons and armor, 0 to 12 for potions, scrolls, wands and trap kits, 0
to 15 for staves). 127 items Aurora made (27 base items: longswords
three times at every level and quality, the rest at two to eleven of
them) show:

- **The table** follows baseitems.2da `PropColumn`: 0 melee, 1 ranged
  (also the unenchanted wand), 2 throw, 3 staves, 4 rods, 5 ammo, 6
  armor (and shields), 7 helmet, 8 potion, 9 scroll, 10 wands, 12 trap;
  16 (amulets, belts, boots, bracers, cloaks, rings) gets the armor
  or helmet table's properties (they differ in one row, the armor's
  Weight Reduction, not drawn in the captures; Moonglow uses the armor
  table). 11, 13–15 and 18–20 (thieves' tools, creature
  items, books and other unequippables, containers, healer's kits,
  torches) can't be made magical; 17 (gems) and 21 (gauntlets) can, and
  get no properties.
- **The first property's tier** is `round(step × top / 12)` (halves
  up), the step `3 × level + quality` (0 to 12), `top` the table's
  highest tier: 2L + {0,1,1,2} on the 9-tier tables, 3L + Q on the
  13-tier ones, 13 for a 16-20 Medium staff.
- **The rest** are one or two tiers lower (Low: two lower only), as long
  as the item's value stays within itemvalue.2da `MAXSINGLEITEMVALUE` at
  character level 5L + Q + 2 (Medium to Godly: the band's third to fifth
  level). Low items have at most two properties, and stay within the
  values of levels 1, 5, 8 and 13 (fitted: 6-10's Low items cost up to
  4234 and none took a third property under 4604; 11-15's and 16-20's
  dropped a second worth more than 12000 and 35000). The first is kept
  whatever it is worth (a 6-10 Low scroll costs 5403). No property is
  repeated (a +2 sword got no +1); one row's property may be in several
  tiers (Enhancement +1 is in three of the melee table's), weighting it.
  Up to four were seen (16-20 Godly throwing weapons and unenchanted
  wands); the tiers of the rest vary between one and two lower.
- **Potions, scrolls, wands and trap kits** get one property; **magic
  staves** three spells, two of the first's tier and one a tier lower
  (clamped at 0); **rods** up to three. Staves, rods and wands have 50
  `Charges`. Scrolls and wands also get Use Limitation: Class for Bard,
  Wizard and Sorcerer (classes 1, 10, 9, in that order), staves for
  Wizard and Sorcerer, whatever the spell.
- The item is written with `Identified` 0, and its `Cost` with the
  properties (Aurora's `Cost` for a +2 longsword was 7248 where the
  engine's is 7250).
- Aurora's tables hold properties itemprops.2da doesn't allow: Vampiric
  Regeneration (row 67) on ranged weapons, Freedom of Movement (75) on
  thrown ones.

The draws themselves are Aurora's random number generator's; Moonglow
makes its own (seeded) with these rules, and only properties the base
item can have.

- **Checked:** Aurora, `aurora_item_wizard.rs` (every captured item holds
  to the rules), and `item_wizard.rs` (every base item, level and
  quality of the install gives properties its base item can have, with
  subtypes, values and parameters from its tables); implemented in
  `crates/mg-rules/src/item_wizard.rs`.
- **Wiki:** the 2da Files page lists the `itmwiz*.2da` as "Toolset only;
  Item Wizard 2da files" and says nothing of how they are used.

## Creatures

### Challenge rating, as Aurora computes it

L is the total class level (classes above level 0, at most 8). The terms
are added in order to a single-precision sum, with f32 weights:

| Term | Value |
| --- | --- |
| Level | 0.15 L |
| Natural AC | 0.1 × `NaturalAC` |
| Gear | 0.2 L × V / (20000 L + 100000) |
| Hit points | (0.2 L × `HitPoints` / E) × walk / PC walk |
| Abilities | 0.1 L × (the six base scores, no racial adjustment) / (L + 50) |
| Special abilities (if S > 0) | 0.15 L × S / (L² + 6L) |
| Spells (if P > 0) | 0.15 L × P / (L² + L) |
| Saves | 0.15 L × (B + `fortbonus` + `refbonus` + `willbonus`) / B |
| Feats | 0.1 L × F / (L/2 + 7) |

- **V:** the value of equipped items only (`Equip_ItemList`, creature slots
  excluded).
- **E:** for each class, level × (`HitDie` + 1) / 2, truncated per class
  (Fighter 5: 27). `HitPoints` is the rolled base, without Constitution.
  The term is skipped if E = 0.
- **walk:** `creaturespeed.2da WALKRATE` of the row whose `2DAName` is the
  appearance's `MOVERATE`; PC walk is row 0 (2.00). The creature's own
  `MovementRate` field has no effect.
- **S:** the sum of `spells.2da Innate` over `SpecAbilityList`. **P:** the
  sum of max(0, `Innate`) over the special abilities again, plus each
  class's `MemorizedList0–9` (classes with `MemorizesSpells` 1) or
  `KnownList0–9` (others). A wizard's known spells count nothing.
- **B:** Fort + Ref + Will from each class's `SavingThrowTable` at its
  level; 1 if that is 0.
- **F:** the sum of `feat.2da CRValue` read as an integer (0.5 and 0.2
  count 0, `****` counts 1).

Then multiply by `racialtypes.2da CRModifier`; above 0.75 subtract 0.25
when below 1.5, at 0.75 or less subtract 0.35; round with halves down
(1.5 gives 1); add `CRAdjust` to the value and to the rounded number. Above
0.75 the rating is rounded + `CRAdjust`; otherwise 1/`Denominator` of the
first `fractionalcr.2da` row whose `Min` the value reaches (0.40: 1/2,
0.30: 1/3, 0.20: 1/4, 0.15: 1/6, else 1/8). No class levels: 1/8. The game
never recomputes it.

- **Checked:** Aurora, `aurora_creatures.rs` `challenge_ratings_match_aurora`:
  4,182 probe creatures from Build › Compile › Creature CR plus 22 from
  Creature Properties, sweeping levels 1–40, hit points, natural AC, feats,
  special abilities, every class and race, saves, movement rates, spells
  and gear. The structure was read from `nwtoolset.exe` and confirmed
  against its output (`docs/research/notes_creature_cr.md`).
- **Moonglow:** `crates/mg-rules/src/challenge.rs`.
- **Wiki:** missing (no formula). The feat.2da page calls `CRValue` a float
  weight; it is read as an integer.

### Maximum hit points, as Aurora computes them

`MaxHitPoints` = `HitPoints` + (Constitution modifier, base Con plus the
racial adjustment) × total level + total level with Toughness (feat 40); at
least 1. Epic Toughness is not included (the game adds it).

- **Checked:** Aurora, `aurora_creatures.rs` `max_hit_points_match_aurora`;
  `creature_stats.rs` `max_hit_points_match_the_stored_ones` (1,520 of
  1,582 base creatures).
- **Moonglow:** `crates/mg-rules/src/creatures.rs` (`creature_stats`).
- **Wiki:** the Creature JSON page says only "HitPoints + Constitution
  bonuses/penalties".

### Saving throws past level 20, and the Great ability feats

A creature's base saves come from its classes' tables for its first 20
levels, taken in the order of its classes (the first class's levels,
then the next's, until 20 are counted); every two levels past 20 add a
point to each save, whatever the classes. A fighter 20 / wizard 20 has
the fighter's saves at 20 and ten more on each, not the two tables'
rows added. A single class past 20 comes to the same either way (its
table's epic rows are that rule). Aurora adds each class's table at its
full level (nwn-issues 816), and so did Moonglow through 1.21.3.

The epic Great Strength, Dexterity, Constitution, Intelligence, Wisdom
and Charisma feats (feat.2da rows 764 to 823, ten of each) each add a
point to the ability's score itself: `GetAbilityScore` with the base
asked for has them, and so the modifiers and the saves.

- **Checked:** in the game's server, `engine_saves.rs`
  `saving_throws_match_the_engine`: thirteen creatures of the Creature
  Wizard's making, of one to three classes and 10 to 40 levels, each
  save equal to the Statistics page's.
- **Moonglow:** `crates/mg-rules/src/creatures.rs` (`creature_stats`).
  The stored maximum hit points stay Aurora's (the Constitution the
  blueprint has, without the Great Constitution feats); whether the
  game adds those feats' hit points itself, as it adds Epic
  Toughness's, was not measured.
- **Wiki:** missing.

### Aurora's Levelup Wizard

Levels are applied one at a time, in class-slot order:

- **Package:** the levelling class's `classes.2da Package`, not the
  creature's `StartingPackage`.
- **Hit points:** a player class (`PlayerClass` 1) at character level 1
  gets its whole hit die; otherwise (`HitDie` + 1)/2 a level, the total
  floored at the end (Fighter 1 to 10 gains 49); a monster class's first
  level gets the average.
- **Ability:** +1 to the package's `Attribute` every 4th character level.
- **Skills:** `SkillPointBase` + Int modifier + the race's
  `ExtraSkillPointsPerLevel`, × `FirstLevelSkillPointsMultiplier` at level
  1; spent a rank at a time, round-robin down `SkillPref2DA`, class skills
  only, capped at level + 3; leftovers go to `SkillPoints`.
- **Feats, in order:** the race's `FeatsTable` at level 1; domain feats
  (`domains.2da GrantedFeat`) at class level 1; the class's list-3 feats at
  `GrantedOnLevel`, each replacing the feat it is the `SUCCESSOR` of; normal
  picks at level 1 and every `NormalFeatEveryNthLevel`, plus
  `ExtraFeatsAtFirstLevel`; bonus picks where `BonusFeatsTable` has one. A
  pick is the first feat in `FeatPref2DA` that the levelling class's
  cls_feat table lists (lists 0/1 for normal, 1/2 for bonus;
  `ALLCLASSESCANUSE` ignored) whose prerequisites are met; feats granted
  earlier in the level count, the level's other picks do not.
- **A new preparing caster's spells:** each level's slots filled with the
  first domain's spell, then the package's spells of that level, then the
  first of those repeated (a Cleric 5 gets Inflict Minor Wounds five times).
- **Package gear:** each item goes to its first `EquipableSlots` slot if
  free and it is `Category` 1–8 or a creature item; otherwise into the
  10-wide backpack at the first place it fits, row by row.

- **Checked:** Aurora, `aurora_levelup.rs` `levelling_up_matches_aurora` and
  `aurora_creature_wizard.rs` (19 creatures of 17 racial types). Known-spell
  classes were not checked.
- **Moonglow:** `crates/mg-rules/src/levelup.rs`.
- **Wiki:** missing (the wiki describes the game's levelup, not Aurora's).

### Aurora's spell-assignment warnings

On OK, each spellcasting class gets at most one warning, the first that
applies: spells above the highest level it casts (dialog.tlk 67093); a
casting ability (with racial adjustment) below 10 + the spell level
(67094); too many spells of a level (67095). The limit for preparing
casters is the `SpellGainTable` slots + ((modifier − level)/4 + 1), also at
level 0 (Wizard 1 with Int 10: 4 cantrips), which the game does not do;
for known-spell casters `SpellKnownTable`, +1 at level 0 only. Preparing
classes are checked on `MemorizedList*`, the others on `KnownList*`.

- **Checked:** Aurora, `aurora_spell_warnings.rs` (30 creatures).
- **Moonglow:** `crates/mg-rules/src/spell_warnings.rs`.
- **Wiki:** the Creature page says the toolset validates very lightly
  ("only on clerics?"); it checks wizards, sorcerers and bards too.

### What the Creature Wizard writes

- Abilities: the first class's `classes.2da Str`–`Cha`, without racial
  adjustments; `StartingPackage` its `Package`.
- Alignment from a per-race table in the toolset, not a 2DA: dwarf lawful
  good; elf and half-elf chaotic good; gnome neutral good; half-orc and fey
  chaotic neutral; goblinoid neutral evil; aberration, monstrous and orc
  chaotic evil; others neutral.
- Hides: dragons `nw_it_creitemdra`, elementals `nw_it_creitemele`.
- `SoundSetFile` 24448 (no soundset.2da row: silent); `Tag` the first name;
  `x2_def_*` event scripts; `DecayTime` 5000, `PerceptionRange` 11;
  `WalkRate` the creaturespeed.2da row matching the appearance's `MOVERATE`.
- `Interruptable`, `NoPermDeath` and `Disarmable` hold uninitialized bytes
  (144, 95, 16).

- **Checked:** Aurora, `aurora_creature_wizard.rs`
  `creature_wizard_matches_aurora`. The construct hide was not captured.
- **Moonglow:** `crates/mg-module/src/blueprints.rs`.
- **Wiki:** missing.

### Familiar and companion fields are read only for classes that have them

`FamiliarType`/`FamiliarName` are read only with an arcane class whose
`MinAssociateLevel` isn't 255 (otherwise 0 and ""); `CompanionType`/
`CompanionName` likewise for divine classes. The level doesn't gate the
reading (a level-3 ranger's companion is read). `Domain1`/`Domain2` and
`School` on a `ClassList` entry are read for classes with `PickDomains`/
`PickSchool`; a cleric without them gets domains 0 and 1.

- **Checked:** engine, `engine_ee_fields.rs`.
- **Wiki:** the fields are documented; the class gating is not.

### A special ability's flags are one yes or no

A `SpecAbilityList` entry with any of the flags' three bits set (Ready,
Spontaneous, Unlimited) is a use the creature has; with none it is spent.
Unlimited does not make the uses unlimited: each entry is used up.

- **Checked:** engine, `engine_special_abilities.rs`.
- **Moonglow:** one switch, Ready, on a creature's Special Abilities
  page (three until 1.19.5).
- **Wiki:** not compared.

### Aurora's Spells page

Aurora's Creature Properties › Spells, watched for a cleric (level 1 and,
with domains Air and Animal, level 9), a cleric given Empower Spell, and a
sorcerer:

- **The list** is every spell of the class's column of `spells.2da`, all
  levels 0 to 9 whatever the class's level, with Prepared (Known for a
  class that knows its spells), Name, Spell Level and Casting Level.
  **Spell Level** (All Levels, Level 0 to 9) narrows it by the spell's
  own level.
- **Meta-Magic** lists None and the metamagic feats the creature has
  (Empower Spell, once the feat was ticked; None alone before); it is
  off for a class that knows its spells. With one chosen, each spell's
  Casting Level is its level raised by the metamagic (Empower: +2; Aura
  versus Alignment, level 8, shows 10), every spell is still listed
  (`spells.2da`'s `MetaMagic` column is not read: Animate Dead, which
  lacks Empower's bit, is there), and Prepared counts the spell with
  that metamagic: one Bless prepared so went in at level 3 and reads 0
  again with None chosen.
- **Spell Selection Summary** lists "Level 0" to "Level 9", each
  followed by its spells as "5 x Cure Minor Wounds", a metamagic one as
  "1 x Bless (Empower Spell)".
- **Domain spells** outside the cleric's column are not listed: with
  domains Air and Animal, no Call Lightning.
- **Escape** in a field of Creature Properties (the Tag, typed into)
  closes the window as Cancel does: what was typed is gone.

- **Checked:** Aurora, by hand on the oracle (2026-10-26), with the
  creatures of `spell-warnings/set4.mod`.
- **Moonglow:** the Spells page has the Meta-Magic list, the casting
  level beside the spell level with a metamagic chosen (a spell cast
  above level 9 can't be chosen: no list keeps it), counts by metamagic,
  and names a metamagic spell as the summary does; its levels show how
  many the class may have rather than Aurora's bare "Level 1". Escape in
  a field leaves the field; Escape with no field typed in closes the
  window keeping what was changed (Cancel sets it back).
- **Wiki:** not compared.

## Areas and tiles

### The tile grid

Tile (x, y) is `Tile_List[y × Width + x]`, row-major from the south-west;
its center is at (10x + 5, 10y + 5). `Tile_Orientation` o is o quarter
turns counter-clockwise: with corners [TL, TR, BR, BL] and edges [T, R, B,
L], one turn gives new[i] = old[(i + 1) mod 4]. A corner's absolute height
is the SET corner height + `Tile_Height`, and neighbours agree on it; the
floor is at `Tile_Height` × `Transition`.

- **Checked:** game data (161,000 shared corner and edge checks in 439
  areas, `tiles.rs` `shipped_tile_grids_are_consistent`); engine
  (`engine_terrain.rs`: a corner raised twice in Rural, Transition 5,
  stands at 10.00 m).
- **A tile's own height:** the engine stands each tile at its own
  `Tile_Height` × `Transition`, whatever its neighbors' heights, with
  its walkmesh: a flat Rural tile at -1, -3, 2, 19, 50, 255 and 1000
  steps had its ground at -5, -15, 10, 95, 250, 1275 and 5000 m, grass
  underfoot (engine, `engine_terrain.rs`
  `a_tile_s_own_height_stands_where_the_engine_puts_it`). The game's own
  areas reach 19; below 0 is not in them but loads. Tile Properties'
  Height (issue #29) sets it.
- **Moonglow:** `crates/mg-area/src/lib.rs`, `crates/mg-tiles`.
- **Wiki:** missing (no page documents the ARE `Tile_*` fields).

### Door hooks, and the doors Aurora places itself

A door on a hook stands at tile center + Rz(90° × o)·(X, Y) + (0, 0,
`Tile_Height` × `Transition` + Z), with `Bearing` = radians(hook
Orientation + 90° × o), plus π when reversed. Aurora places a door itself
only on hooks with `Type` ≠ 0, using `doortypes.2da TemplateResRef` and
`Appearance` = Type; replacing a tile removes the doors on its typed hooks.

- **Checked:** game data (1,854 doors within 1e-5 m); Aurora,
  `aurora_terrain.rs` `group_doors_match_aurora`; engine,
  `engine_terrain.rs`.
- **Moonglow:** `crates/mg-area/src/terrain.rs` (`door_edits`).
- **Wiki:** "Adding Doors to a Tile" explains the offsets but not the
  rotation or the Type ≠ 0 rule.

### Custom tilesets' counts are not to be trusted

Custom tilesets in use on a persistent world carry `Doors=` values that
are no counts. Of 65 tilesets in one server's content, 9 had such tiles
(350 in all), beside tiles with true counts: `Doors=1869573190`,
`1953393015`, `1634169902`, which as bytes read `Floo`, `wint`, `.tga`:
whatever was in the memory of the editor that wrote them. One tileset
had a hole among its groups (`[GROUPS] Count=270`, no `[GROUP266]`). A
reader that loops up to a declared count never ends, and runs out of
memory if it notes each missing section. Read the `[TILE<n>DOOR<d>]`
sections that exist (numbered below the count), and leave a missing
group out.

- **Checked:** game data (65 tilesets served by a live server, so the
  game takes them; not run in the engine here); `mg-set`'s
  `counts_that_are_no_counts_cost_nothing`, `small_formats.rs`.
- **Moonglow:** `crates/mg-set/src/lib.rs` (`Sections`).
- **Wiki:** missing.

### A tileset with holes is read tile by tile

Old tilesets merged by hand from many can have holes: a `[TILEn]`
section the count calls for missing, a tile without `Model=`, a
terrain's, crosser's or rule's section missing. The game reads the
rest. A tile without its section or a model has no walkmesh, as a tile
whose model doesn't exist has none (`GetSurfaceMaterial` at its centre
reads what it reads for those), and the tiles after it keep their
numbers; a model named in capitals is the same model; missing terrain,
crosser and rule sections change nothing the server reads. (A
builder's tileset drew no tiles at all in Moonglow, which failed such a
file whole.)

A tileset whose lines end in a bare LF, not CRLF, is another matter:
with one in a hak the server never gets past "Loading Module" (the same
file with CRLF loads in a second). Every shipped tileset's lines end in
CRLF. Moonglow keeps a file's line ends and writes new ones with CRLF.
What lines ending each way in one file do was not measured.

- **Checked:** engine (`engine_tileset_holes`: ttr01.set with holes cut
  in it, in a hak, and with its CRLF turned to LF); `small_formats.rs`
  (no shipped tileset has a hole or a bare LF).
- **Moonglow:** `crates/mg-set/src/lib.rs` (`Sections::items`,
  `Tile::is_hole`), the area's problems (`crates/mg-area/src/lib.rs`),
  the doctor's `set-hole` and `set-line-ends`.
- **Wiki:** missing.

### Aurora's terrain painting

- **Terrain brush:** sets one corner's terrain, keeping its height.
  Primary rules apply to all eight neighbours, diagonals included, by
  relative height: with base = min(h_placed, h_adj), the rule for (Placed,
  h_placed − base, Adjacent, h_adj − base) applies, and `Changed@k` sets
  the neighbour to base + k. Pairs more than a step apart match no rule,
  and rules don't chain. Only cells touching a changed corner, and the four
  around the painted one, get a new random tile (with new random lights).
  If any of them fits no tile, the stroke does nothing.
- **Raise/Lower:** one corner one step; the eight neighbours follow,
  recursively, so no two adjacent corners differ by more than a step
  (raising three times makes rings at 3, 2, 1); rules then apply as if the
  corner were painted at its new height. Lowering at 0 changes nothing but
  re-picks the tiles.
- **Crossers:** each cell is split by its diagonals into four quarters, one
  per edge; the crosser goes on the edge of every quarter the drag passes
  through, the first included. A click without moving re-picks the cell.
- **Eraser:** acts on the tile, not a corner, and does not paint the
  Default terrain: it clears the tile's crossers and re-picks it and its
  neighbours across the cleared edges; a tile that then fits nothing loses
  its other crossers, outward.
- **Groups:** Tile0 under the pointer; each right-click turns the group a
  quarter counter-clockwise about Tile0 (offset (c, r) → (−r, c), every
  tile the same orientation); placed at the height of the ground under
  Tile0 (its lowest corner), overwriting the terrain and re-picking the
  tiles around.

- **Checked:** Aurora, `aurora_terrain.rs` `painting_matches_aurora` (54
  scripted steps in four tilesets, and eight group placements); game data
  (770 group placements).
- **Moonglow:** `crates/mg-tiles/src/paint.rs`.
- **Wiki:** the Tilesets page says only that the toolset "will perform a
  check on adjacent tiles". "SET File Format" says primary rules are "not
  used in practice… set Count=0": 23 base tilesets use them, up to 192
  rules. BioWare's ITP document says the Eraser paints the Default
  terrain; it doesn't.

### Aurora's Tile Properties: the model's lights and loops

Tile Properties always shows its seven controls, and enables those the
tile's model has: Main Light 1 and 2 where it has a light node
`<model>ml1`/`ml2`, Source Light 1 and 2 where it has a node
`<model>sl1`/`sl2`, Animation Loop 1 to 3 where it has an animation
`animloop01` to `03`. The others are greyed out: a swatch drawn blank,
not in its stored color, and a checkbox unchecked even where the file has
1. The tileset's `MainLight1`..`AnimLoop3` flags can't be what it reads:
the game's sets flag all seven on every tile. With several tiles selected
(a dragged box), a control is enabled where any of them has it. Its
title bar's ✕ discards a change without asking; OK applies it.

- **Checked:** Aurora on the oracle, by hand: a tin01 area with tiles 18
  (`tin01_d11_01`: ml1 only), 0 (ml1, ml2, sl1, sl2), 134 (ml1, ml2,
  loops 1 to 3) and 45 (ml1, ml2, loops 1 and 2), each tile's Tile
  Properties, then 18 with 0 and 0 with 134 together (October 2026).
- **Moonglow:** `mg_area::TileFeatures`, `crates/mg-ui/src/tile_select.rs`
  (which asks before its ✕ discards, a builder's ask).
- **Wiki:** "Model Special Nodes" names the `ml`/`sl` nodes; that the
  toolset greys out the controls follows BioWare's ARE document.

### Resize Area and Rotate Area

Resize keeps the south-west corner. New corners copy the nearest old
corner and new edges the nearest old edge across; new tiles are random
among those that fit (edge tiles are not copied as tiles). Shrinking
replaces what is left of groups cut by the new edge with terrain tiles and
removes their doors. Rotate (counter-clockwise, an area H tiles high): tile
(x, y) → (H − 1 − y, x), orientation + 1; a point (x, y) → (10H − y, x);
door bearings + 90°, wrapped into [−π, π).

- **Checked:** Aurora, `aurora_terrain.rs` `resize_and_rotate_match_aurora`.
- **Moonglow:** `crates/mg-area/src/reshape.rs`.
- **Wiki:** "Area Editor" says enlarging "will copy the same tiles that
  were on those edge areas"; it doesn't. Rotate is not documented.

### What the Area Wizard writes

- **Terrain:** corners get the SET's `Default`, the outer ring `Border`,
  and the corners within half a cell of the center `Floor` (one for even
  sizes, 2×2 for odd), all at height 0; random tiles that fit. If none
  fits, Aurora raises an access violation (Lizardfolk Interior at 5×5
  always does).
- **Names:** resref = the name's letters, digits and `_`, lowercased, at
  most 16 ("Area 001" → `area001`); tag the same with case kept, at most
  32.
- **Lighting and weather:** the areag.ini `EnvScheme` row of
  environment.2da: `LIGHT_*` → `Sun*`, `DARK_*` → `Moon*` (ambient,
  diffuse, fog color, fog amount, shadows), `DAYNIGHT` → `DayNightCycle`/
  `IsNight`, `ShadowOpacity` = round(`SHADOW_ALPHA` × 100), `RAIN`/`SNOW`/
  `LIGHTNING`/`WIND` → `ChanceRain`/`ChanceSnow`/`ChanceLightning`/
  `WindPower`; `FogClipDist` 45.0, `SkyBox` 0. Each tile's
  `Tile_MainLight1`/`2` and `Tile_SrcLight1` is a random pick from
  `MAIN1_COLOR1-4`, `MAIN2_COLOR1-4` and `SECONDARY_COLOR1-4`;
  `Tile_SrcLight2` equals `Tile_SrcLight1`.
- **Module:** the first area becomes `Mod_Entry_Area`, entered at its
  center facing north.

- **Checked:** Aurora, `aurora_new.rs` `new_modules_and_areas_match_aurora`
  (over 40 areas across all 33 tilesets); engine, `engine_new_module.rs`.
- **Moonglow:** `crates/mg-module/src/new.rs`, `crates/mg-tiles/src/lib.rs`.
- **Wiki:** the environment.2da page lists the columns without
  descriptions; the mapping, the Floor patch and the crash are missing.

### A lighting scheme chosen for an area

Area Properties › Visual's scheme list, on an existing area, writes what
the Area Wizard writes from the row (above), and more:

- **Fog and sky set back:** `FogClipDist` becomes 45.0 and `SkyBox` 0
  whatever they were (77.0 and 3 in the probe); the other fields of the
  area are kept.
- **Every tile's lights picked anew:** each tile's `Tile_MainLight1`/`2`
  and `Tile_SrcLight1` from the row's colors at random, `Tile_SrcLight2`
  equal to `Tile_SrcLight1` (all 16 of the probe's, set to 31, changed;
  nothing else of a tile). The page says so: choosing a scheme loses
  "custom tile lighting".
- **`Version`** counts up, as whenever Aurora saves an area.

- **Checked:** Aurora, `aurora_lighting_scheme.rs` (Interior Evil, row 16,
  on a 4×4 Rural area whose tiles were lit 31).
- **Moonglow:** `Scheme::applied_fields` and `tile_fields` in
  `crates/mg-module/src/new.rs`; Area Properties, Give Every Area This
  Page's Settings and `mg areas --lighting-scheme` use them.
- **Wiki:** not described.

### ARE colors and tile lights, stored and read

The ARE color fields (`Sun/Moon` `AmbientColor`, `DiffuseColor`,
`FogColor`) are DWORD `0x00BBGGRR`; `GetAreaLightColor` and `GetFogColor`
return `0xRRGGBB`. `Tile_SrcLight1/2` stores the
`TILE_SOURCE_LIGHT_COLOR_*` constant + 1, 0 meaning off, so
`GetTileSourceLight*Color` returns the stored value − 1 (a stored 0 reads
255); `Tile_MainLight*` stores the constant as is.

- **Checked:** engine, `engine_new_module.rs` `new_module_runs_in_the_engine`
  (every tileset).
- **Wiki:** missing (no ARE field page; the source-light offset is not in
  the Lexicon).

### Fog

`fogEnd` = `FogClipDist`; `fogStart` = min(30 − `Sun/MoonFogAmount`,
`fogEnd` − 1), in meters. Fog is always on, even at amount 0; amounts above
30 start it behind the camera. A skybox changes neither value. The fog
color is the stored color, in gamma space.

- **Checked:** game client, `client_render.rs` `fog_uniforms_match_the_client`
  (the shader's uniforms read back by a debug copy of the game's own shader
  include in a scratch `override`).
- **Moonglow:** `crates/mg-area/src/scene.rs` (`fog`).
- **Wiki:** no formula. "Area Lighting" and "Render Distance with Fog and
  Skyboxes" say a skybox extends the fog clip distance; the fog end stays
  at `FogClipDist` (the draw distance was not measured).

### Point lights

For a light color c (MDL `color`, or lightcolor.2da RED/GREEN/BLUE):
intensity = max(1, the largest channel of c); the shader's color is
(c / intensity)^2.2; its cutoff = radius × 2.0 × intensity × the MDL
`multiplier`, and `lightColor.a` = ±cutoff², negative for `ambientonly`.
The multiplier lengthens the reach and leaves the color (a gray 0.3 light
with multiplier 2 is as gray, reaching twice as far). A color above 1
makes a light reach further, not brighter (BrightWhite 2.0 on a radius-10
light: color 1, cutoff 40 m). `lightMaxIntensityInv` = 1.5^−2.2 ≈ 0.4098;
`lightFalloffFactor` = 2² × (0.2^−2.2 − 1.5^−2.2) ≈ 136.33.

Tile main lights use lightcolor.2da RED/GREEN/BLUE (TOOLSETRED… are only
the toolset's swatches) with radius 10 (ml1) and 5 (ml2) whatever the model
says, and in place of the node's own color, even a black one (`tni01`'s
smithy: nodes of radius 14 and 9, three of them black, light as row 26
on every tile); source lights take their color from the `fx_flame01`
animation named by the value, radius 7. A tile whose model has no
`ml1`, `ml2`, `sl1` or `sl2` node is lit by none of its settings (the
black filler of `tni01`, whatever main and source lights it is given).

The light nodes of a placeable's own model are lights like any other,
static or not; **Illuminated** (`LightState` 0) turns off only the
appearance's light (placeables.2da `LightColor`), not them. A draw keeps
32 of the lights that reach it, the most important first (by
`lightpriority`, then nearness).

- **Checked:** game client, `client_render.rs` `light_uniforms_match_the_client`
  (seven cases); `model_light_uniforms` (an authored placeable model's
  light node: multipliers 0.5 to 10, colors above and below 1, ambient
  only, static, `LightState` 0, 36 lights at once; the "Light, White"
  placeable with `LightState` 0); `heed_armourer_look` (the `tni01`
  smithy, pictures within 1.6 of 255).
- **Moonglow:** `crates/mg-render/src/scene.rs`, `renderer.rs`
  (`attenuation_params`), `anim.rs`.
- **Wiki:** "Shader Engine Support" lists the uniforms without values.
  "Area Lighting" says the main lights' radius is "set in the MDL" (the
  forced 10/5 is in "Model Special Nodes"); "Tileset Construction Tutorial"
  says tile lights take their colors from tilecolor.2da.

### Colors in the shader

Area ambient and diffuse are uploaded as (byte/255)^2.2, and so are MDL
`ambient`, `diffuse` and `selfillumcolor` (tic01's floor diffuse 0.659 →
0.400).

- **Checked:** game client, `client_render.rs` (`light_uniforms_match_the_client`;
  material values read back, then region means within 1/255 in
  `reference_scenes_match_the_client`).
- **Wiki:** missing (only "fog color is always gamma").

### An area's ambient and diffuse beside its tile lights

The area's ambient and diffuse colors are light, added to the tile and
object lights, not a tint over them: each channel is (byte/255)^2.2 of
white light, so a gray of 60 is 4% of white ambient light everywhere and
170 is 41% of white from the sun's (or moon's) side, the default
direction (towards 4000, 4500, 7000) by day and night alike; an Always
Bright area takes the sun's colors. A white ambient is full white light
on every surface, which drowns the tile lights in the client as well: a
flat, bright picture. Fog of 5 with black fog darkens the far part only
(from 25 m). The client's default settings (bloom, high contrast,
vibrance, sharpening, ambient occlusion on, Performance) make such an
interior more saturated and warmer, not darker (HSV saturation 0.50
against 0.43, red 6 of 255 more, blue 3 less).

A material's custom pair is lit as its fragment program says: a program
that defines `NORMAL_MAP 1` (the game's `fslit_nm`, and Heed's
`fhss_skintile`, which is it with keyholing off) lights as the
normal-mapped programs do, with no light softened past the terminator
and none from a light the surface faces away from, whether or not the
mesh has a render hint or a normal map; one that defines it 0 lights as
`fslit`. In a tile-lit interior drawn by Heed's pair the client's dark
corners are about a tenth darker than `fslit` lights them.

- **Checked:** game client, `heed_interior_look` in `client_render.rs`
  (2026-10-27): a stock 4×4 `tin01` interior (`map_m1q1k002` of Chapter
  1, its tiles' lights) set as Heed's smithy (Always Bright, the
  Interior Bright scheme, black fog of 5, no shadows), with ambient and
  diffuse 60/170, 60/150, 255/170 and 0/0. Moonglow's area view with
  the Shaders switch off and on drew the same picture (0.00 of 255
  apart), within 0.8, 0.7, 1.5 and 0.7 of 255 of the client's mean
  color, and within 8.5 in each of a 3×3 grid of regions (the most in
  the top right, by 6 with no area light too). With Heed's pair on every
  texture of the tiles (`MG_HEED_SHADERS`): Moonglow's own shader was
  0.2 to 0.8 of 255 from the game's programs, brighter in the dark
  corners (17.8 against the client's 15.4 with tile lights alone), its
  mean color 1.1 to 1.7 from the client's; after, 0.00 apart and 0.6 to
  1.5 from the client's. `a_custom_pair_is_lit_as_its_fragment_program_lights`
  in `crates/mg-render/tests/render.rs` holds Moonglow's own shader to the
  game's `fslit_nm` and `fslit` under custom names (before, 99% within
  7 of 255 on the `_nm` one; after, 0).
  `light_uniforms_match_the_client` reads the uniforms themselves.
- **Checked:** game client, `heed_armourer_look` in
  `client_render/armourer.rs` (2026-10-28): Heed's armourer
  (`hss_armourer`, the area he sent; the test builds it of stock
  content): a 4×4 `tni01` of the smithy group in the tileset's black
  filler, his tiles' lights (row 26 on the smithy's main lights, the
  filler's inert), 60/170 by day, black fog of 5, the innkeeper, from
  four cameras (the forge among them). Moonglow's area view with the
  Shaders switch off and on drew the same picture (0.02 of 255 apart at
  most), within 0.1 to 1.6 of 255 of the client's mean color and 1.6 to
  3.1 in any region of the 3×3 grid; his own file and the built one
  drew alike. The area's Lighting Scheme (12) is Aurora's preset, which
  set the colors when it was chosen: neither the client nor Moonglow
  reads it. `a_builders_area_in_the_view_draws_as_the_comparison`
  (`crates/mg-ui/tests/app/drawing.rs`) holds the window's own picture
  of his area to what the comparison draws (within 0.9 of 255). Where
  the client leaves rooms out, that is the visibility of the tile the
  player stands on (on the smithy's doorway tile it drew nothing but
  the innkeeper), which the area view doesn't follow, as Aurora doesn't.
- **Moonglow:** `crates/mg-render/src/scene.rs` (`AreaLight::from_are`),
  `crates/mg-area/src/scene.rs` (`area_light`), `crates/mg-render/src/renderer.rs`
  (`Slots::lit_normal_mapped`).
- **Wiki:** "Area Lighting" gives no formula; that a custom pair is lit
  by its own defines is missing.

### Facing

`GetFacing` is in degrees, 0 east, counter-clockwise, rounded to whole
degrees. Doors and placeables store `Bearing` in radians, facing − 90°;
creatures, waypoints, items and stores store `XOrientation`,
`YOrientation` = (cos f, sin f); module.ifo stores the start's facing as
`Mod_Entry_Dir_X`/`Y` = (cos, sin). The game turns a model by facing − 90°:
a model's front is its +Y axis.

- **Checked:** engine, `engine_facings.rs`
  `bearings_and_orientations_face_as_the_engine_reports` (Bearing 0.5 rad
  reads 119), `engine_area_edits.rs`, `engine_test_start.rs`; client,
  `reference_scenes_match_the_client`.
- **Moonglow:** `crates/mg-module/src/instances.rs`.
- **Wiki:** missing.

### Minimaps

The map draws each tile's `ImageMap2D` turned a quarter counter-clockwise
per `Tile_Orientation` step, Tile_List row 0 at the bottom (south); a tile
without a picture is black. In TGA and DDS alike the first stored row is
the picture's bottom, also in a TGA that declares a top-left origin
(descriptor bit 5): the game ignores it, so a picture saved top row
first, as Krita saves it, shows upside down.

- **Checked:** game client, `client_minimap.rs`
  `minimaps_are_laid_out_as_the_client_draws_them`.
- **Moonglow:** `crates/mg-module/src/minimap.rs`.
- **Wiki:** the layout and rotation are missing (flipping DDS is
  documented).

### Particles

Gravity is `mass` × 9.8 m/s²; `velocity` is in m/s. On hitting the ground a
particle keeps 0.8 of its speed along it and 0.8 × `bounce_co` off it.
`m_isTinted` multiplies a particle by the area's ambient plus diffuse
(added in gamma space) plus point lights. The three-stop values
(`colorMid`, `alphaMid`, `sizeMid`, `percentStart/Mid/End`) and
`twosidedtex` change nothing visible. A tile's own emitters (the
`tni01` smithy's forge fire, which lives 5 seconds) burn in the client's
picture taken some seconds after the player enters; whether the client
runs them ahead when the area loads was not measured (Moonglow runs an
emitter new to its view ahead, a tile's as an object's).

- **Checked:** game client screenshots, `client_render.rs` `particles_look`
  (exploration, no assertions; numbers in `docs/research/notes_models.md`);
  `heed_armourer_look` (the forge's fire).
- **Moonglow:** `crates/mg-render/src/particles.rs`; `crates/mg-area/src/scene.rs`
  (`AreaScene::particles`: an emitter new to the view, an object's or a
  tile's, is run ahead).
- **Wiki:** "MDL ASCII Emitter Nodes" describes `mass`, `bounce_co`,
  `m_isTinted` and `twosidedtex` loosely or differently.

### A static placeable is an object to scripts

The server keeps a static placeable (`Static` 1) as an object: a script
finds it by its tag (`GetObjectByTag`) and among the area's objects
(`GetFirstObjectInArea`), and reads its local variables. It is Plot,
whatever its file's `Plot` says. (The client merges it into the area's
tiles, so what scripts do to it is not seen.) Of two objects with one
tag in one area, `GetObjectByTag` took the one placed second, and `nNth`
1 the first.

- **Checked:** engine (`engine_static_placeables.rs`: a static and a
  dynamic barrel with a variable each, and two with one tag).
- **Moonglow:** `crates/mg-module/src/areas.rs` (`static_plan`: a
  placeable with variables stays dynamic), `crates/mg-module/src/tags.rs`
  (static placeables' tags count).
- **Wiki:** agrees (Placeable, "Static Placeables": found by
  `GetObjectByTag`, Plot by default); a builders' guide that says no
  script finds one is wrong. Which of two tags is found is missing.

## Drawing, seen in the game client

All checked by screenshots of the game client beside Moonglow's picture,
by eye (`client_render.rs`, the test named in each; exploration, no
assertions).

### A body part is drawn in its own texture

A creature's part (`p<g><r><pheno>_<part>NNN`) is drawn with the texture
of its own name, whatever texture its mesh names: the game's thigh 3
names thigh 2's texture and chest 32 chest 13's (598 such meshes in the
base game). A mesh that names something that is no part's texture keeps
it.

- **Checked:** `creatures_look` with `MG_PARTS`.
- **Moonglow:** `crates/mg-preview`.
- **Wiki:** missing.

### A skin mesh in a body part or a held item

A body part (`pmh0_chest001`) and a held item (the small shield,
`ashsw_011`) were put in the override with their meshes made `skin`
nodes, each vertex weighted to one bone, and looked at on a creature in
the client (`creatures_look` with `MG_OVERRIDE`, and `MG_HELD` for the
shield; the body part's vertices raised 0.6 m, so that the override's
copy could not be taken for the game's).

| The model | The skin's bone | In the client |
| --- | --- | --- |
| Body part, compiled | the skeleton's (`torso_g`, no node of the part) | not drawn |
| Body part, compiled against the skeleton (Beamdog's HD bodies) | the skeleton's | drawn, following the skeleton's bones |
| Body part, text | the same | drawn as a plain mesh, where the part hangs |
| Body part, compiled | the part's own dummy | drawn at the creature's feet, not where the part hangs |
| Body part, text | the part's own dummy | drawn out of shape, above its place |
| Held item, compiled or text | the skeleton's (`lforearm_g`) | not drawn |

A builder's report (a body part exported as a skin shows in Moonglow
and not in the game) is the first row. Another's (an armor showing only
its head and pelvis, in Moonglow alone) is the second: Beamdog's HD
bodies (the human male's `pmh0_*` parts) are skins compiled against the
creature's skeleton. Such a skin's bind pose has one entry per node of
the skeleton (56 for `pmh0`), not of the part, and its bones are the
skeleton's nodes by their place in a depth-first walk of its tree
(`pmh0_neck001`: 4, 5 and 2, `neck_g`, `head_g` and `torso_g`; as
nwn-mdl-spec's MDL-SKIN-011 and -012 have it, from the engine's code:
[the comparison](research/notes_mdl_spec.md));
the head and pelvis are plain meshes. A compiled skin that is not so
(the first row's, compiled by `mgv compile`, keeps a bind pose for the
part's own nodes) is not drawn. Moonglow draws the first, second, third
and last rows as the client does: the skeleton's skin posed by the base's
nodes (`mg_preview::compose::on_skeleton`); a skin weighted to nodes of
its own part is not drawn at all (`mg_preview::compose::without_skins`).

- **Checked:** by eye, in the client's pictures (2026-10-11); the rule
  on a model read as text and compiled, in `mg-preview`'s
  `a_body_part_s_skin_is_drawn_as_the_game_draws_it`. The compiled
  models were made by `mgv compile`, not the game's own compiler; one
  body part and one item were tried. The second row: the HD pack's
  `pmh0_*` parts in the override of `creatures_look` (2026-10-19), the
  client's armored human whole and its picture beside Moonglow's alike,
  and different from the stock parts'; in Moonglow,
  `a_body_part_bound_to_the_skeleton_follows_its_bones` (a part made so
  by hand).

### `setfillumcolor` is not self-illumination

2,365 meshes of the game's own ASCII models carry `setfillumcolor`, a
misspelling of `selfillumcolor` that nwnmdlcomp compiled and our notes
said to accept. The game does not read it: an armoire's model replaced
(in a scratch override) by a grey panel whose only glow is
`setfillumcolor 1 0 0` is drawn dark in the client, and the same panel
with `selfillumcolor 1 0 0` red (`placeables_look` with `MG_OVERRIDE`,
2026-10-19). nwn-mdl-spec finds the same in the engine's compile of its
probe `zspec_setfill` (MDL-ASCII-070). Of the game's models, 28 meshes
glow by the misspelling alone, most of them lanterns of `tni01` tiles;
Moonglow drew them lit until 1.34.4, and its reader now gives the word no
meaning (the linter notes it; the decompiler still writes it, for
nwnmdlcomp, before `selfillumcolor`).

- **Checked:** by eye, in the client's pictures beside Moonglow's
  (2026-10-19).

### A part-based creature with no body part fields

A creature made with a single model (the game's `nw_bandit001`,
appearance 269) keeps no `BodyPart_*` field, no `ArmorPart_RFoot` and no
`Appearance_Head`. Given a part-based appearance (Human, row 6) and
nothing else, the client draws no body: a field that is not there is
part 0, which is nothing; and no head, for the same reason (below). An
armor's parts cover that while it is worn, so it shows when the armor is
taken off (a builder's report, "unequipping an armor sets all body parts
to 0"). Moonglow draws the same, and since 1.22.1 sets the parts (1, and
no belt or shoulders, as a creature new in Aurora has them, and head 1)
in the step that gives such a creature a part-based appearance
(`blueprint::with_a_body`).

- **Checked:** by eye, in the client's picture beside Moonglow's
  (2026-10-12; `client_render.rs`'s `creatures_look` with `MG_NO_BODY=1
  MG_WORN=0 MG_APPEARANCES=6,6`): the creature without the fields beside
  one with them. (A head showed there because `MG_NO_BODY` keeps
  `Appearance_Head`; without it there is none: `heads_look`, below.) The
  step, in `a_part_based_appearance_brings_a_body`.

### A head of 0 is no head

A part-based creature whose `Appearance_Head` is 0, or that has no
`Appearance_Head` at all, has no head in the client: the neck is drawn
alone, as for a head number with no model (254). The engine holds 0 for
both (`GetCreatureBodyPart(CREATURE_PART_HEAD)`). It is not drawn as head
1. Moonglow's previews, and so its area view, had drawn head 1 for
either (a builder's report on 2.0.0); they now draw none.

- **Checked:** game client, `client_render.rs`'s `heads_look`
  (2026-10-26, EE 89.8193.37): a human man wearing nothing, one client
  run each for head 0, 1, 2, 254 and no field (the EE twin
  `xAppearance_Head` set with the byte); over the head, the pictures of
  0 and of no field are within 0.4 of 255 of 254's and 2.8 or more from
  head 1's. Moonglow's area view of the same areas is held to the same in
  the test, and its previews in `crates/mg-preview/tests/previews.rs`
  `a_head_of_0_is_no_head`.
- **Moonglow:** `crates/mg-preview/src/creature.rs` (`creature`).
- **Wiki:** the Phenotype Quick Overview calls part 000 "invalid
  (nothing)", which the client's drawing bears out for heads.

### An armor's part 0 leaves its wearer's own part

An armor part 0 is no part: there is no `p<g><r><p>_<part>000` model.
Worn, the client draws the wearer's own body part where the armor's is 0
(a human with body parts 1, under armor whose left bicep, forearm, hand,
shoulder, thigh, shin and foot, neck and belt are 0: a bare left arm and
leg beside the armor's right ones). So 0 is "not visible" only where the
wearer's own part is 0 too. nwn.wiki's Phenotype Quick Overview calls
part 000 "invalid (nothing)", which holds for the model, not for what is
drawn over a body. Most of the game's parts_*.2da leave row 0's
`ACBONUS` blank (all but the belt, shoulder and robe tables), so lists
made of the rows with an `ACBONUS` lacked 0. Moonglow offers 0 for every
armor part (since a builder's report on 2.0.0), and its previews draw
the wearer's part there, as they did.

- **Checked:** by eye, in the client's picture beside Moonglow's
  (2026-10-22; `client_render.rs`'s `creatures_look` with `MG_WORN=1
  MG_PARTS=LBicep=0,LFArm=0,LHand=0,LShoul=0,LThigh=0,LShin=0,LFoot=0,Neck=0,Belt=0`,
  which sets each `ArmorPart_*` and its EE twin). The previews, in
  `an_armor_s_part_0_leaves_its_wearer_s_own`.

### A worn part moves at its wearer's animation scale

A cloak or robe playing its wearer's skeleton's animation moves at the
wearer's `setanimationscale`, not its own model's (a cloak's has none):
on an elf it sits on the shoulders as on a human, and the wings a
cloak's row hides are hidden.

- **Checked:** `creatures_look`; Moonglow's own test
  `a_cloak_sits_on_a_smaller_body_s_shoulders`.
- **Wiki:** missing.

### A shader that switches the environment map off

With an MTR naming a shader whose source has `#define ENVIRONMENT_MAP 0`
(`fslit_nm`), the appearance.2da row's environment map is not reflected:
the texture's alpha is see-through with the MTR's `transparency` and
drawn solid without it. With no shader named, or no MTR, the map shows
where the texture is see-through, `transparency` or not.

- **Checked:** `creatures_look` with `MG_OVERRIDE`.
- **Moonglow:** `crates/mg-render/src/renderer.rs` (`env_switched_off`:
  the shader's source is read for that one line, and not run).
- **Wiki:** missing.

### A static placeable reflects no appearance map

A placeable whose placeables.2da row names an environment map
(`Reflection`: `default` or a named one) reflects it only if it isn't
static: a static one's textures take their alpha for opacity, as without
a map (an alpha-0 card isn't drawn). A texture's own TXI
`envmaptexture` is reflected either way, `default` being the tileset's
`EnvMap`.

- **Checked:** `area_env_map_look` (authored cards in `tin01` and
  `tic01` interiors).
- **Moonglow:** `crates/mg-preview/src/object.rs` (`placeable`).
- **Wiki:** missing.

### A placeable's `Reflection` `default` is `chrome1`

`default` means three things in the game. In placeables.2da's
`Reflection` it is the game's own map, `chrome1`, wherever the
placeable stands; in a texture's TXI `envmaptexture` and in
appearance.2da's `ENVMAP` (a creature's) it is the area's, its tileset's
`EnvMap`. In a rural interior (`tin01`), whose map is a copper brown, a
mirror that isn't static is silver; the armoire's texture with
`envmaptexture default`, and a bone golem, are copper. (Up to 2.5.2
Moonglow drew the mirror copper too: a builder's armor stand, Heed's
report.)

- **Checked:** `area_env_map_look` with `MG_ENV_SHAPE=sphere`,
  `MG_ENV_PAINT` and `MG_ENV_CREATURE=24` (2026-10-25): spheres of a gray
  texture reflecting wholly, as the models of the big mirror, a mirror,
  the dwarven mirror, the armoire and the bone golem, each painted with
  the map bound to it by the game's `inc_standard` in the scratch
  override; in `tin01` and `tic01`, static and not. Flat cards facing the
  camera, as first tried, reflect one dark spot of any map: they can't
  tell the maps apart.
- **Moonglow:** `crates/mg-preview/src/object.rs`
  (`PLACEABLE_DEFAULT_ENV_MAP`); a texture's and a creature's `default`
  in `crates/mg-render/src/renderer.rs`.
- **Wiki:** missing.

### The game's shaders and Moonglow's own in stock interiors

With the game's stock shaders, the area view draws an interior the same
with its Shaders switch on and off, and as the client does: in seven
stock interiors (`tin01`, `tic01`, `tdc01`, `tdm01`; 2 to 111 lights,
torches and placeables) the two pictures were 130 pixels of 786,432
apart, by at most 4 of 255, and within 2.5 to 15 of 255 of the client's
(where it was furthest, the client leaves out rooms the player's tile
doesn't see, which Moonglow draws, as Aurora does). The client's post
effects (bloom, high contrast, vibrance, sharpening, ambient occlusion:
on by default) make its picture a little more saturated (0.02 to 0.07 of
the HSV saturation) and no darker; High Quality and Performance differ
by a level or two. What does change the switch's side alone is a hak's
or the override's shader include: The Domains of Dread's
`inc_lighting` (Tjured's diffuse wrap) draws an interior a tenth darker
with the game's shaders, and the client draws it so too.

- **Checked:** `area_lighting_look` (with `MG_CLIENT_SETTINGS=owner` and
  `default`, `MG_AREA_HAKS=tdod_shaders`, `MG_MUTATE` showing the game's
  programs draw), and the area view itself:
  `a_stock_interior_looks_alike_with_the_shaders_on_and_off` in
  `crates/mg-ui/tests/app/drawing.rs`.
- **Moonglow:** `crates/mg-render/src/game.rs`.
- **Wiki:** missing.

### The lit check's pink on Heed's Vulkan

The lit check (`mg_render::probe::probe_lit`) draws a red BC1-textured
quad, seen from straight above in the default area light. On this
machine (Radeon RX 9070 XT, Mesa RADV) the stock `vslit`/`fslit` and
Moonglow's own shader both draw its middle [204, 56, 56]: the green and
blue are the stock program's white specular light (its specular color
is white for a red albedo at the default specularity 0.04, metallicness
0). Heed's Radeon RX 580 through AMD's Vulkan driver 25.5.1 drew it
[212, 133, 133] with the game's programs (2.13.1). That is exactly the
stock program with `fNdotV` (the normal against the view) zero, as if
the surface were seen edge on: the Fresnel term at a grazing angle
raises the environment specularity from 0.04 to 0.29 (`mix(1, 0.04,
sqrt(0.55))`), which takes the red light down by the same factor (0.6
of linear light times 0.71, against 0.96) while the diffuse light is
unchanged. Forcing `fNdotV = 0.0` in `inc_lighting` here draws [202,
116, 116]; with the view vector also lying in the surface's plane,
[208, 127, 127].

Nothing Moonglow feeds the program differs by driver: every uniform
block is zeroed on the CPU and then written (the members the contract
leaves zero among them; filled with 1.0 instead they draw [255, 220,
220]), the lights past a draw's count read the table's zeros and the
check has none, naga zero-initializes private and function variables and
reads `OpUndef` as zero, wgpu zero-fills buffers and textures, the
environment, noise and missing slots are bound to white 1×1 textures,
the zero vertex inputs (`vWeight`, `vIndex`) are read only for skinned
meshes, every read of the storage buffers is in bounds, and nothing in
the program's path divides by zero or normalizes a zero vector for this
view. What does change the program is the load order: the check read
`vslit`, `fslit` and their includes as the area view does, from
`override/`, `development/`, a Workshop item and the module's haks
first, and The Domains of Dread's `inc_lighting` shows builders do
replace it. Whether Heed's machine had such a copy or its driver
computes the view vector wrong is not known. The check now draws the
game's own copies (`Assets::game_shader`, the keys' copy) and names any
copy the load order puts over them in its debug-log line, with the
colors each shader drew.

- **Checked:** `the_lit_check_draws_with_the_game_s_programs` and
  `a_builder_s_include_leaves_the_lit_check_to_the_game_s_own`
  (`crates/mg-corpus-tests/tests/hlsl.rs`, pictures in
  `target/test-output/lit-check/`), 2026-10-29. Heed's machine: not
  reproduced (waiting on him: `docs/deferred.md`, "Heed's interiors'
  lighting").
- **Moonglow:** `crates/mg-render/src/probe.rs`, `assets.rs`.
- **Wiki:** missing.

### What a custom shader gets of its mesh

A material's custom vertex program reads each of the model's texture
coordinate sets by the engine's names: `tverts` as `vTcIn`, `tverts1`,
`tverts2` and `tverts3` as `vTcIn1`, `vTcIn2` and `vTcIn3`. A set the
model doesn't have reads zeros (not `tverts`). The MTR's `parameter int`
and `parameter float` lines fill the uniforms of their names; a map the
MTR doesn't name is not bound (its `textureNBound` 0), for a custom pair
as for the stock ones, and the stock include's own defaults follow from
that (specularity 0.04, roughness 0.55, no height or glow). A mesh gets
tangents only where its model has them or a render hint is given (the
model's, or its MTR's `renderhint`); without, `vTangent` and
`fHandedness` are zeros, and the stock include's normal mapping in a
custom pair draws the mesh black (the tangent frame of a zero vector).
A static placeable is drawn the same as one that isn't. (Up to 2.5.3
Moonglow fed every mesh zeros for `vTcIn2` and `vTcIn3`, which drew a
builder's walls one flat, grayish color where his shader picked
`tverts2` or `tverts3`, and made tangents for every mesh: Heed's
report.)

- **Checked:** `custom_shader_uvs_look` (2026-10-25): cards in a `tni01`
  interior whose four sets each cover one stripe of a texture of gray,
  red, green and blue stripes, drawn by a custom pair that picks the set
  by `parameter int nUvmap` and moves it by float parameters (Moonglow's
  own pair, and the builder's), the normal map alone, every map, with a
  render hint and without, static and not. The mean color of each card
  of one stripe in the client and in Moonglow's area view agree within
  0.6 of 255 (set 2: client (18.5, 77.3, 17.7), Moonglow (18.6, 77.4,
  17.7); before, Moonglow (33.7, 33.5, 63.5), what (0, 0) samples,
  saturation 0.47 against 0.77), the cards without a render hint black
  in both; the cards of the whole texture (set 0, and the stock pair's
  for comparison) within 3.4, the stripes' edges filtered a little
  otherwise. `MG_UV_PAINT=bound` painted each card with its bound maps:
  the client's and Moonglow's flags the same on every card, the whole
  picture within 0.6 of 255. Seen on Linux (radv); the black of a
  normal map without tangents (the stock include normalizing a zero
  vector) was not seen on Windows or macOS.
- **Moonglow:** `crates/mg-render/src/model.rs` (`Vertex`: `uv1` to
  `uv3`), `crates/mg-render/src/game.rs` (`attribute`).
- **Wiki:** the MTR page and the 1.74.8159 notes say the sets link to
  `vTcIn1`…`vTcIn3`; what a set the model lacks reads, and that a mesh
  without a render hint has no tangents, are missing.

### Static placeables stand upright

A placeable turned about all three axes stands as its file says; a
static one is drawn upright and at full size whatever its file says
(the server still holds the transform: `GetObjectVisualTransform`
reads it back, see "Visual transforms, as the game reads and draws
them").

- **Checked:** `placeables_look`, `visual_transforms_look`.
- **Moonglow:** `crates/mg-area/src/lib.rs` (`takes_visual_transform`);
  Verify's `static-transform` warning (`crates/mg-module/src/doctor.rs`).
- **Wiki:** missing.

### What a visual transform's rotations turn

The rotations of a visual transform (`RotateX`, `RotateY`, `RotateZ`;
Aurora's and Moonglow's Adjust Location call them X, Y and Z Rotation)
are not about the axes they are named for. X Rotation turns the model
about its upright axis (as Bearing does, counter-clockwise seen from
above), Y Rotation tips it about its own X axis (east at Bearing 0; a
positive angle brings its top south), and Z Rotation leans it about its
own Y axis (a positive angle brings its top east). The game makes them
in that order from the outside in: the model is leaned, then tipped,
then turned. It is scaled by `ScaleX` alone, the same every way
(`ScaleY` and `ScaleZ` change nothing; the script's `SCALE` is one
number), and the translation moves it along the object's own axes,
turned by its Bearing but not by the rotations. A builder's mirror at
Bearing 89.63 with X Rotation -50.8 stands upright, turned on its stand.

- **Checked:** `visual_transforms_look` (mirrors, and chairs whose
  backs show the way they turn, each axis alone and in pairs, in the
  client beside Moonglow's view).
- **Moonglow:** `crates/mg-area/src/lib.rs` (`VisualTransform::matrix`);
  the area view's tilt rings set Y and Z Rotation
  (`crates/mg-ui/src/area_view.rs`). Up to 2.5.3 Moonglow turned the
  model about X for X Rotation, about Y for Y and about Z for Z, and
  scaled each axis by its own field: it laid such a mirror flat.
- **Wiki:** missing; the Lexicon's constant table says only "X-axis
  Rotation".

### A TGA's origin bits are ignored

The game takes a TGA's rows and columns as stored: neither the top-left
origin bit (5) nor the right-to-left bit (4) changes the picture.

- **Checked:** `placeables_look` with `MG_TGA=16`.
- **Moonglow:** `crates/mg-image`.
- **Wiki:** missing.

### Two-sided faces, cut-outs and decals

A face is drawn from its front only, whatever its texture's TXI says
(`blending punchthrough`, `decal 1`, both or none). An MTR with
`twosided 1` shows it from behind too, lit as its front is, and the
texture's `blending punchthrough` still cuts it out. `decal 1` is drawn
unlit: as bright in an area lit 0x101010 as by day.

A texel is cut out where its alpha is 0.2 or less (`fAlphaDiscardValue`),
whatever the TXI says and with an MTR or without. What is left of a
`blending punchthrough` texture is drawn solid; of any other, blended.

- **Checked:** `cutouts_look`; the cut with `MG_DISCARD=1`, which paints
  each surface with the value through a patched `inc_standard.shd`.
- **Moonglow:** `crates/mg-render/src/renderer.rs` (`Slots::twosided`).
- **Wiki:** not compared.

### Particles: kept, flip-books and ribbons

Particles an emitter keeps in its own space (`inherit`) fall the
world's way down, whatever way the emitter is turned. A flip-book's
first frame is the picture's top left, and its frames go along a row
and down the rows. A `Linked` ribbon is twice its particles' size
across, each piece showing its particle's frame whole.

- **Checked:** `particles_look` with `MG_PARTICLES=inherit`, `linked`,
  `linked2` (and `MG_GRID`) and `linked4`.
- **Moonglow:** `crates/mg-render/src/particles.rs`.
- **Wiki:** not compared.

### An upright particle turns to the eye

An `Aligned_to_World_Z` particle stands upright and is turned about the
world's Z to face the eye: seen from the north, north-east, east, south
and west, a still 1 m quad is as wide and the same way round each time
(103 px at the test's distance), and so is one whose emitter is turned
a quarter about Z. It keeps no fixed side, neither the world's nor its
emitter's. (A card fixed in the world beside it went from edge on to
face on, so the view did turn.) Moonglow drew it so already; its area
view matches the client's picture within 1 px each way.

- **Checked:** game client, `client_render.rs`'s
  `upright_particles_look` (2026-10-26, EE 89.8193.37), held to it there
  and in `particles.rs`'s `an_upright_particle_turns_to_the_eye`. The
  client's scripted camera takes effect only when set a second after
  the player enters: `SetCameraFacing` in the module's OnClientEnter
  itself left the client's own camera (so `particles_look`'s closer
  camera was likely never the one drawn).
- **Moonglow:** `crates/mg-render/src/particles.rs` (`batches`).
- **Wiki:** MDL ASCII Emitter Nodes says the particles "face sideways
  from ground plane"; that they turn to the camera is missing.

## Talk tables and 2DAs

### Where the game finds a custom talk table

`Mod_CustomTlk` names it without extension. The game looks in the module's
haks first, then the module, then the user's `tlk/` folder; the feminine
table (`<name>f`) is looked up separately in the same order, so a hak's
masculine table can pair with a `tlk/` folder's feminine one. Without a
feminine table, female lookups return the masculine text. The module does
not load at all when the table is missing, when the name ends in `.tlk`,
or when its case differs from the file's on Linux.

- **Checked:** engine, `engine_tlk.rs` `where_the_game_reads_custom_talk_tables`
  (8 cases).
- **Moonglow:** `crates/mg-module/src/talk.rs`, `crates/mg-resman`.
- **Wiki:** wrong: the TLK page says "Tlk files placed into a hakpak won't
  be recognized by the game" ("Content Load Order" says they are). The
  feminine order, the `.tlk` and case failures are missing.

### A talk-table entry's text counts only with its text flag

An entry with text but without flag `0x1` reads as ""; so does a
sound-only entry (`0x2`) and a row past the end. Custom StrRefs are
`0x01000000` + row.

- **Checked:** engine, `engine_tlk.rs`.
- **Moonglow:** `crates/mg-tlk/src/lib.rs`.
- **Wiki:** missing (the TLK page also calls TLK "a GFF file").

### Line ends: plain in talk tables and localized text, CRLF in comments

Every talk table of the install is plain LF: of the strings with a line
end, English `dialog.tlk` has 2,408, German 3,714 (3,731 in
`dialogf.tlk`), French 2,056, Spanish 2,012, Italian 2,006, Polish 2,267,
and not one `\r` among them. GFF text in the game's data is mostly
CRLF: of the 15,116 CExoString and CExoLocString texts with a line end
in the base game's blueprints (BIFs) and the official campaigns' `.nwm`
modules, 13,925 are CRLF and 1,191 LF (conversations 3,721 to 248,
`.gic` comments 9,129 to 0; item descriptions 140 to 256 and module
descriptions 0 to 14 the other way). No one text mixes the two, and
none has a lone `\r`. The game shows both kinds.

What Aurora (EE) writes for a line typed into a text of one line goes by
the kind of field, not by that data: a plain LF in a localized string
(CExoLocString: the module's description, a placeable blueprint's
Description, a conversation line's Text), CRLF in a CExoString (a
conversation line's Comment, a blueprint's Comment). Saving a blueprint
also makes every line end of its comment CRLF, typed in or not (one
stored with LF came back CRLF). Both kinds of field are the same
control in Aurora's forms (a VCL `TMemo`, a Windows edit control, which
gives CRLF), so the plain line end is Aurora's own doing, not Wine's.
Aurora writes no talk table.

- **Checked:** the install (89.8193.37), each string's bytes counted, by
  a script reading the TLK entries and the GFF fields of every
  extracted blueprint and `.nwm` resource. Aurora (89.8193.37, on the
  oracle, 2026-10-26): a line typed into each of those fields of a
  module and saved (capture `line-ends/typed.mod`); test
  `crates/mg-ui/src/text.rs` `a_new_line_takes_the_line_end_aurora_writes`.
  That the fields are `TMemo`s was read from Aurora's decoded form
  resources (the oracle's `uiinv/forms.json`); that an edit control's
  text has CRLF line ends is Windows' documentation, not measured here.
- **Moonglow:** text fields show plain line ends and store the text's
  own, line by line (`crates/mg-ui/src/text.rs` `with_line_ends_or`); a
  line typed into a text of one line gets LF in a localized string and
  a talk table (`with_loc_line_ends`), CRLF in a CExoString and a
  script (`with_line_ends`). What is not typed in keeps its line ends,
  a blueprint's comment too.
- **Wiki:** missing. The 1.87.8193.35 notes say carriage returns had
  been drawn as question marks (among the NUI fixes).

### NUI text fields have no page keys

The game's NUI text field of several lines (`NuiTextEdit` with
`bMultiline`) takes neither Page Down nor Page Up: its caret stays where
it is and its text doesn't scroll, while the arrow keys move the caret
a line.

- **Checked:** the game client (89.8193.37, sandboxed on the off-screen
  display, 2026-10-26): a window with a text field of 60 lines, its
  value watched; a letter typed after each key, the bind read back in
  the module's NUI event script, showed where the caret was.
- **Moonglow:** the NUI Creator's preview (Interact) draws a NUI text
  field as the game's, without page keys
  (`crates/mg-ui/src/nui_view/preview/text_controls.rs`); Moonglow's own
  text fields of several lines have them (`widgets::page_keys`).
- **Wiki:** missing (the NUI pages don't name the keys a text field
  takes).

### An empty quoted 2DA cell keeps its place

`""` is an empty cell in its column, not skipped (Beamdog's
`random_hostile.2da` row 10 relies on it).

- **Checked:** engine, `engine_2da.rs` `engine_reads_quoted_empty_2da_cells_in_place`.
- **Moonglow:** `crates/mg-2da/src/lib.rs`.
- **Wiki:** the 2da page says a blank quoted field is blank, not that it
  keeps its place. `nwn_twoda` gets this wrong (see below).

## Haks and resources

### Haks over 2 GiB

A resource whose data *starts* at 2³¹ bytes or later in a hak is missing to
the game (a 2DA reads empty, a script doesn't run); one that starts before
is read whole, even if it ends past the mark. A lower hak's copy does not
take over: the game finds the name in the higher hak, fails, and stops.

- **Checked:** engine, sparse 2.1 GiB haks, `engine_big_hak.rs`
  (`the_game_reads_nothing_that_starts_past_2_gib_in_a_hak`,
  `past_2_gib_a_lower_haks_copy_doesnt_count`).
- **Moonglow:** `crates/mg-resman/src/container.rs`, `crates/mg-erf`, the
  content doctor's `erf-size` check.
- **Wiki:** "Resource Limits" and "HAK" give only a 2 GB size guideline
  because "the Toolset… doesn't read past 2GB".

### A hak's folder outranks its place in the list

A hak in the user's `hak/` folder outranks one in the install's `data/hk/`,
even when listed later in `Mod_HakList`. `ResManGetAliasFor` returns
`DEVELOPMENT:`, `OVERRIDE:`, `HD0INSTALL:data/<key>` for the keys and
`CURRENTGAME:<module>` for the module's resources.

- **Checked:** engine, `engine_resman.rs` `resolution_matches_engine`.
- **Moonglow:** `crates/mg-resman/src/lib.rs`, `install.rs`.
- **Wiki:** "Content Load Order" lists user haks above install haks, but
  its hak-order section says only that the first listed wins. The alias
  strings are not documented.

### A woman's inventory icons: armor's, not cloaks'

The base game has a woman's inventory icon of every armor part a man's
has: 182 `ipf_` PLTs beside 182 `ipm_` (belt 17, chest 63, pelvis 39,
robe 11, left and right shoulders 26 each). Of cloaks it has the man's
alone, `icloak_m_001` to `icloak_m_016`, and no `icloak_f_`: a cloak's
icon is the man's whoever wears it.

- **Checked:** the resources, counted by name in the install's load order
  (`nwn_resman_extract -p ipf_`, `-p ipm_`, `-p icloak`), 2026-10-26.
- **Moonglow:** `crates/mg-ui/src/images.rs` (`item_icon_names`,
  `item_icon_on`: a layer without a woman's picture is the man's), shown
  so with Options › General's Show armor and cloaks on a woman.
- **Wiki:** "cloakmodel.2da" names the icon `icloak_m_NNN` and says
  nothing of a woman's; the armor parts' `ipf_` and `ipm_` icons are not
  named in the mirror.

## GFF fields

### EE object visual fields

- `TextureReplace` (struct 9): list `TextureReplaceLi` of `OldTexture`/
  `NewTexture`.
- `AnimationReplace` (struct 11): list `AnimationReplace` of
  `OldAnimation`/`NewAnimation`.
- `Material` (struct 7): list `ShaderParams` of `Material`, `Param` and
  `Type` (1 int, 2 vec4), then `Int` or `Float1`–`Float4`.
- `MiscVisuals` (struct 8): `HiliteColorR/G/B` (default −1), `MouseCursor`
  (−1), `TextBubbleText`, `TextBubbleType` (0–3), `UiDiscoverMask` (15),
  `VisibleDistance` (45).

All four are read on placed placeables, doors, creatures and items (and
blueprints created by script); triggers read only `MiscVisuals`;
waypoints, sounds, stores and encounters none. A partial `MiscVisuals`
keeps the defaults for what it lacks.

- **Checked:** engine, `engine_ee_fields.rs` (names from `ObjectToJson`).
- **Moonglow:** `docs/research/notes_ee_fields.md`; the editors' Visuals
  pages.

### Visual transforms, as the game reads and draws them

A placed object's visual transform is `VisTransformList` (struct 6 per
entry): `Scope` (int, 0 for the whole model), then `AnimationSpeed`,
`ScaleX`…`ScaleZ`, `RotateX`…`RotateZ` and `TranslateX`…`TranslateZ`,
each a struct (0) of `TimerType` (int), `ValueTo` (float) and `LerpType`
(int), as Aurora's Adjust Location writes it and Moonglow's does. The
server reads it on placed creatures, doors, items and placeables, static
or not and Useable or not, and the older single `VisualTransform` struct
(floats `ScaleX`… `RotateX`… directly) too: `GetObjectVisualTransform`
returns what the file says for each. Its own save writes a transform a
script set the same way, with `ScaleX` holding the script's one scale and
`ScaleY` and `ScaleZ` left 1. The client draws it on every one of them
but a static placeable, which it draws upright and at full size; what
the rotations turn is in "What a visual transform's rotations turn".
Saved as a `.mod`, as a folder or through a nasher project (Moonglow's
JSON, packed by neverwinter.nim's `nwn_gff`), the fields are the same.

- **Checked:** engine, `engine_visual_transforms.rs`; client,
  `visual_transforms_look`.
- **Moonglow:** `crates/mg-area/src/edit.rs` (`visual_transform_edits`),
  `crates/mg-ui/src/area_tools.rs` (Adjust Location).
- **Wiki:** "Placeables and the Toolset" says transforms are for "a
  usable placeable or a creature"; the engine applies them to placeables
  that aren't Useable as well, and to doors and items.
- **Wiki:** missing.

### EE "x" twin part fields

Each BYTE part field has a WORD twin labelled "x" + its label, cut to 16
characters, written right after it: `xBodyPart_*`, `xArmorPart_LBice`,
`_LShou`, `_LThig`, `_Pelvi`…, `xAppearance_Head`, `xModelPart1`–`3`.
Creatures get part fields only if appearance.2da `MODELTYPE` is `P`. A
creature's right-foot body part is stored as `ArmorPart_RFoot`. How the
game reads parts above 255 was not tested.

- **Checked:** Aurora, `aurora_instances.rs`, `aurora_creature_wizard.rs`;
  game data (Doom of Icewind Dale).
- **Moonglow:** `crates/mg-rules/src/items.rs` (`wide_label`).
- **Wiki:** missing.

### An armor's per-part colors

An armor (UTI) may give each part its own color per channel, over the
item-wide `Cloth1Color`…`Metal2Color`: a BYTE field
`APart_<part>_Col_<channel>`, with `part` an `ITEM_APPR_ARMOR_MODEL_*`
number (0 right foot … 17 left hand, 18 robe) and `channel` an
`ITEM_APPR_ARMOR_COLOR_*` number (0 leather 1, 1 leather 2, 2 cloth 1,
3 cloth 2, 4 metal 1, 5 metal 2: not the order of a PLT's layers). The
game reads the fields from a blueprint (`GetItemAppearance` with index
`6 + part × 6 + channel` returns them, and 255 for a part without one),
and writes exactly these for a color set with `CopyItemAndModify`; a part
without a color of its own has no field. No base-game item has one, and
Aurora has no page for them.

- **Checked:** engine, `engine_armor_colors.rs` (names and types from
  `ObjectToJson`). How the client draws them was not compared; whether
  Aurora drops them on save was not captured.
- **Moonglow:** `crates/mg-rules/src/items.rs` (`armor_part_color`); the
  item editor's Part Colors; `crates/mg-preview/src/creature.rs`.
- **Wiki:** missing.

### A placeable's illumination (`LightState`)

`GetPlaceableIllumination` and `SetPlaceableIllumination` keep a placeable's
light in a BYTE field, `LightState`, which the game reads when it loads the
placeable: from a placed placeable's GIT entry and from a blueprint
created by script. 0 is off, any other value on, and without the field
(as every Aurora file is: Aurora neither shows nor writes it) the light is
on. A placeable whose appearance has no light (placeables.2da `LightColor`
empty) is never lit: the engine reports it off whatever the field says,
and setting it on does nothing. No base-game blueprint has the field.

- **Checked:** engine, `engine_light_state.rs` (a brazier, row 57, and a
  chest, placed and created, with 0, 1, 2 and no field; the field's name
  and type from `ObjectToJson`). Whether the client draws a placeable
  loaded with 0 without its light was not looked at.
- **Moonglow:** `crates/mg-rules/src/placeables.rs`; a placeable's
  Advanced page, **Illuminated**. Source: Jasperre's report (October
  2026).
- **Wiki:** placeables.2da and Area Lighting say the light is controlled
  with `SetPlaceableIllumination`; the field is missing.

### A creature's items: `Dropable` and `Pickpocketable`

What `GetDroppableFlag` and `GetPickpocketableFlag` report for a creature's
item is read from two BYTE fields of its entry, spelled `Dropable` (one
p, unlike the script functions) and `Pickpocketable`: in a blueprint's
`ItemList` (beside `InventoryRes`) and `Equip_ItemList` (beside
`EquippedRes`), and in a placed creature's whole items in the GIT. 1 sets
them; 0 and a field left out do not (an item neither drops nor can be
stolen unless set). Aurora writes 1 for a ticked box and leaves the field
out otherwise (its inventory's Options, `cbDropable` and
`cbPickpocketable`); an item it equips has neither. The game's own
blueprints and modules have only 1 or no field (one base-game blueprint
has a 0); placeables' and stores' items never have them.

- **Checked:** engine, `engine_item_flags.rs` (four backpack items and two
  equipped, set, left out and 0, on a placed creature and on a blueprint
  created by script); Aurora, `aurora_inventory.rs` (items equipped in
  its inventory); the base game's UTCs and Kingmaker's and Witch's Wake's
  GITs counted by hand (October 2026).
- **Moonglow:** `crates/mg-ui/src/blueprint/creature_lists.rs` (the
  Inventory page's Options) and `inventory.rs` (`flag_edit`);
  `crates/mg-module/src/instances.rs` keeps an entry's flags when it
  places a creature (an equipped item's were dropped up to 2.5.4).
  Source: Sacha's report (October 2026).
- **Wiki:** missing; the NWN Lexicon has the functions only.

### A creature's item moved between its backpack and its equipment

A placed creature's items are whole items in the GIT, its backpack's in
`ItemList` and its equipment's in `Equip_ItemList`, and the two differ
only in where the item is held: a backpack item's struct id is its place
in the list and it has `Repos_PosX` and `Repos_Posy` (WORD, its cell in
the inventory grid); an equipped item's struct id is its slot's bit
(0x200 the amulet, 0x100 the right ring) and it has neither. Every other
field is the same in both, `Dropable` and `Pickpocketable` included. A
blueprint's entries differ the same way, with the item named
`InventoryRes` in the backpack and `EquippedRes` in a slot. Aurora moves
the item itself: an item of the creature's own (its own name, tag,
description, flags) dragged from the backpack onto its slot, or from a
slot into the backpack, keeps all of that; the struct id and the grid
fields change, and an item put in the backpack goes at the list's end, at
the first free cell (beside a necklace at 0, 0: 1, 0). Aurora asks
nothing on either move, and a right click on a placed creature's item, in
the backpack or a slot, opens that item's own Item Properties. As with
every item it writes, Aurora adds `xModelPart1` and drops fields it does
not know.

- **Checked:** Aurora, `aurora_inventory.rs`
  (`an_item_aurora_moves_is_kept_as_it_is`: `inventory-moves/before.mod`
  and `after.mod`, an amulet equipped from SPW0's backpack and a ring
  taken off SPW1 into its backpack, each with its own tag and name,
  dropable, pickpocketable and with an unknown field); the campaign
  modules' GITs counted (1663 equipped items, none with `Repos_PosX` or
  `Repos_Posy`; 883 backpack items, all with both; the other fields
  differ only with the kind of item: armor is mostly worn) and the base
  game's and campaigns' UTCs (319 `EquippedRes` entries, none with grid
  fields; 95 `InventoryRes`, all with them) (October 2026).
- **Moonglow:** `crates/mg-module/src/instances.rs` (`listed_item`,
  `equipped_item`) and `crates/mg-ui/src/blueprint/inventory.rs`
  (`carried`), `creature_lists.rs` (Equip, To Backpack, drags) and
  `follow.rs` (an item's own window following it). Up to 2.5.6 a move
  made the item again from its blueprint. Moonglow keeps the fields
  Aurora drops (lossless editing).
- **Wiki:** missing.

### An item put in an equipment slot that is taken

Aurora's inventory never refuses a slot that holds an item, and asks
nothing about it: the item dropped takes the slot, and the one the slot
held goes to the backpack as it is (every field kept; its struct id its
place in the list, at its end, and `Repos_PosX`/`Repos_Posy` the first
free cell, which is where a backpack item just taken out left). This is
so for an item dragged from the backpack, from the palette, and from
another slot: a ring dragged onto the other ring slot leaves its own
slot empty and sends the ring that was there to the backpack (no swap).
The question about a missing feat is asked when a weapon that needs one
is dropped on a weapon slot, before anything moves; No leaves it where
it was, Yes adds the first feat. A longsword dropped on the secondary
weapon slot while the primary is empty goes in the primary slot, so a
drag of the creature's own longsword from the primary to the secondary
slot leaves it in the primary (after the question, and Yes still adds
the feat).

- **Checked:** Aurora, `aurora_inventory.rs`
  (`an_item_aurora_drops_on_a_taken_slot_sends_its_item_to_the_backpack`:
  `inventory-slots/before.mod`, `inventory/before.mod` with items of the
  bandits' own; `after-a.mod`: SPW2's backpack amulet onto its worn one,
  a palette amulet onto SPW3's, SPW4's left ring onto its right ring's
  slot, SPW5's longsword from the primary slot onto the secondary with
  No; `after-b.mod` the same with Yes; `after-c.mod` the sword through
  the backpack onto the secondary slot) (October 2026).
- **Moonglow:** `crates/mg-ui/src/blueprint/creature_lists.rs`
  (`equip_edits`: what the slot held goes to the backpack as it is, in
  the same step; up to 2.5.6 it was deleted). A weapon dropped on the
  secondary slot stays there (recorded in deferred.md).
- **Wiki:** missing.

### Conversation script parameters

Entries and replies have `ActionParams`; links (and `StartingList` entries)
`ConditionParams`; each a list of structs (id = index) with `Key` and
`Value` strings. In the game, greetings' conditions run in order until one
passes; each condition gets only its own link's parameters, and the chosen
node's action its `ActionParams`.

- **Checked:** Aurora, `aurora_dialog.rs`; engine, `engine_dialog.rs`.
- **Moonglow:** `crates/mg-module/src/dialog.rs`.
- **Wiki:** the layout is missing.

### `repute.fac`

A `RepList` entry (`FactionID1`, `FactionID2`, `FactionRep`) is how much
faction 2 likes faction 1, as `GetReputation(member of 2, member of 1)`.
Aurora's Add Faction copies the parent's reputations both ways; any missing
pair Aurora looks up is written as 100; removing a faction shifts later ids
down.

- **Checked:** engine, `engine_factions.rs`; Aurora, `aurora_factions.rs`.
- **Moonglow:** `crates/mg-module/src/factions.rs`.
- **Wiki:** missing (only BioWare's Faction PDF is linked).

### Objects of the PC faction

Faction 0, PC, is the players' (Jasperre: "an invalid faction ID" for
anything else). A creature of it, placed or created by script, is put in
faction 1, Hostile, as it loads: it is of one faction with a hostile
creature (`GetFactionEqual`), a commoner regards it at 0, and it is
saved with `FactionID` 1. A placeable, door or trigger keeps faction 0,
belongs to no creature's faction, and creatures regard it as
`repute.fac` has them regard the PC faction (a commoner at 50, a hostile
creature at 0). No base-game blueprint and no object of the shipped
modules has faction 0; Aurora writes it for a trigger whose blueprint has
no `Faction`.

- **Checked:** engine, `engine_factions.rs`
  (`objects_of_the_pc_faction_in_the_engine`). How a player regards such
  an object was not measured: no player joins a headless server.
- **Moonglow:** the faction lists leave it out
  (`blueprint/situated.rs`, the Store Wizard), and Verify warns of it
  (`pc-faction`, `crates/mg-module/src/doctor.rs`), but not of a
  trigger's: Aurora writes 0 for a trigger whose blueprint has none (the
  owner's decision, 2026-10-21).
- **Wiki:** missing ("repadjust.2da (and Reputation in general)" says a
  PC is not in an NPC faction).

### Smaller layout facts

- The GFF page swaps GIT and GIC: `.git` holds an area's instances, `.gic`
  the toolset's comments for them, one struct per instance, same order.
- Store blueprints keep their palette category in `ID`, not `PaletteID`.
- The engine reads modules the same with their GFF fields in any order and
  with unknown fields added (`engine_modules.rs`, 121,000 objects;
  `engine_locked.rs`).

## What Aurora writes

### The module tree and the window in front

Aurora's module tree doesn't follow the window brought to the front: a
click on an area's tab, or Ctrl+Tab between areas, leaves the tree's
selection, open folders and scroll as they were, and only turns the
area in front's row blue (a mark of its own, apart from the selection).
Areas are the only windows to choose between: the script and
conversation editors and Properties are modal dialogs. A script opened
from an area's Events (Edit) isn't selected in the tree; a double click
on an area selects the Areas folder, not the area. Ctrl+F6 does
nothing, and there is no Window menu.

- **Checked:** Aurora under Wine on *The Dark Ranger's Treasure*
  (2026-10-26), by screenshots.
- **Moonglow:** follows the tab chosen by default (a builders' request);
  Options › General › Show the tab chosen in the module tree off is
  Aurora's way (`crates/mg-ui/src/tree.rs`, `reveal_tab`).
- **Wiki:** missing.

### Placing a blueprint

Palette fields (`PaletteID`, `Comment`, a store's `ID`) are dropped; old
fields become their EE forms (`Tail` → `Tail_New`, `Wings` → `Wings_New`,
`GenericType` → `GenericType_New`); a creature's `SkillList` is padded to
skills.2da's length. Inventories expand from `InventoryRes`/`EquippedRes`
into whole item structs at (−1, −1, −1), struct id the list index
(equipment: the slot bit), property struct ids 0, a `Cost` computed if
missing. A store gets its five pages; a sound's `Priority` is recomputed; a
placeable without `Static` gets !`Useable`; an encounter's `CreatureList`
is sorted by CR, then resref. Triggers and encounters stand at the
outline's first point, with relative X/Y and absolute heights (ground +
0.025 m). Everything faces north.

- **Checked:** Aurora, `aurora_instances.rs` `placed_objects_match_aurora`;
  engine, `engine_blueprints.rs`.
- **Moonglow:** `crates/mg-module/src/instances.rs`.
- **Wiki:** missing.

### Update Instances

It remakes every placed object of the blueprint as if placed anew, keeping
only its position, facing, visual transform, a trigger's or encounter's
`Geometry` and an encounter's `SpawnPointList`. The instance's own `Tag`,
name, scripts, variables, `Plot`, `Locked`, a door's transition and a
sound's volume are lost.

- **Checked:** Aurora, `aurora_update_instances.rs`.
- **Moonglow:** `crates/mg-module/src/instances.rs` (`update`).
- **Wiki:** says only that it updates objects "to match the blueprint".

### Update Instances and held items

Asked "even those inside a container (creature, placeable object,
store)?", Aurora's Update Instances makes the items placed objects hold
again from their blueprint too: name, charges and cost are the
blueprint's, and so is `StackSize` (a chest's three and a creature's
two became one); the place in the grid, `Dropable`, `Pickpocketable`
and a store's `Infinite` stay.

- **Checked:** Aurora, the captures `wave1/update-held.mod` and
  `wave1/pal2-update.mod` (`tools/aurora/capture_wave1.py`).
- **Moonglow:** `crates/mg-module/src/instances.rs` (`update_held`).
- **Wiki:** missing.

### What Aurora drops when it saves an area or a blueprint

An area's own grass (`GrassDefDisabled`, `GrassOverrides` in the GIT's
`AreaProperties`) is gone from an area Aurora saves; so is the visual
transform (`VisTransformList`) of a static placed placeable, while a
dynamic one's stays. A blueprint written by its Edit and OK has no
`VisTransformList`, static or not (Aurora's blueprints have none).

- **Checked:** Aurora, the captures `wave1/area-saved.mod` (the probe
  `wave1/before.mod`), `wave1/static-bp.mod`, `wave1/pal2-vtdyn.mod`.
- **Moonglow:** keeps what the files have; Verify warns of a static
  object with a visual transform. The manual warns of the grass.
- **Wiki:** missing.

### Add to Palette naming

The new resref is the template without `nw_` and its trailing number; from
the template's number Aurora finds the first unused number, then takes the
next free one after it, skipping one (`nw_wswls001` → `wswls003`).

- **Checked:** Aurora, `aurora_palette_add.rs` (three cases).
- **Moonglow:** `crates/mg-module/src/palette_add.rs`.
- **Wiki:** missing.

### Blueprint wizards

Resref: the name's letters and digits, lowercased, cut to keep a trailing
number within 16 characters ("Area Transition 001" → `areatransitio001`);
any character other than letters, digits, spaces and `_` gives
`blueprintNNN` instead. Sound play styles: once = `Looping` 0,
`Continuous` 0; repeating = 0, 1; seamless = 1, 0 (one sound).
`Priority` (prioritygroups.2da): looping area-wide 2, looping positional 3,
single area-wide 19, single positional 20.

- **Checked:** Aurora, `aurora_blueprints.rs`, `aurora_sounds.rs`.
- **Moonglow:** `crates/mg-module/src/blueprints.rs`.
- **Wiki:** missing.

### Custom palettes (`*palcus.itp`)

Built from the `*pal.itp` skeleton without `TYPE` 0 and 1 nodes, each level
sorted by name with Windows word sort (case-insensitive, `-` and `'`
ignored but for ties); each blueprint a leaf in its `PaletteID`'s category
(stores: `ID`), sorted by name then resref; creatures add `CR` and
`FACTION`.

That is how Aurora makes a palette **anew**, and it does so only in Build
(Compile › Palettes) and the palette's Refresh Palette: from the module's
**and its haks'** blueprints (not the override folder's), a name that is a
talk-table string written as its `STRREF`, a name of the blueprint's own as
`NAME`; entries whose blueprint is in neither go. Otherwise Aurora's custom
palette **is the module's `palcus.itp` as read** (none: the categories,
empty, until a Refresh): it lists the entries of hak blueprints, of the
override folder's and of blueprints there are no more, and leaves out
module blueprints the file doesn't list. A save writes the file back with
every entry as it was (a `STRREF` stays one, a stale name stays stale) but
sorts each category's entries by name and drops fields it doesn't know.
Edit then OK on a blueprint rewrites its entry only when the name shown or
the category differs (as `NAME` text, at its place by name; a talk-table
name shown the same keeps its `STRREF`); Edit Copy and Add to Palette add
an entry as `NAME` text, even for a talk-table name, after an entry of the
same name; Delete takes out the entry alone.

- **Checked:** Aurora, `aurora_new.rs`, `aurora_palettes.rs` (the
  captures `palettes/`, made by `tools/aurora/capture_palettes.py`);
  game data, `palettes.rs` `custom_palettes_rebuild_like_aurora`.
- **Moonglow:** `crates/mg-module/src/palette.rs` (`rebuild_custom_palette`
  for Build, `update_custom_palette` for a save), `new.rs`. A save does not
  sort or drop fields, and a new entry for a talk-table name is its
  `STRREF` (as Aurora's Build writes it), so that a palette changes only
  where the builder changed something.
- **Wiki:** "Toolset Palette ITP" says the toolset makes the `palcus.itp`
  from the module's blueprints at every save; Aurora 89.8193.37 doesn't.

A module's own skeleton (`<type>pal.itp`) is what Select Category and
Refresh Palette read: its new and renamed categories, a node's name its
`STRREF` before a `NAME` written out, `DELETE_ME` where there is
neither, written into the `palcus.itp` as `NAME` (or the `STRREF`
alone). The custom palette shown is the `palcus.itp`'s own nodes and
names, each level by name, whatever order the file has; Save As writes
it back in that order, and gives every creature entry its blueprint's
`CR` and `FACTION`, stale or not. A blueprint whose category the
palette lacks (a `PaletteID` of 200, 255, a category the `palcus.itp`
hasn't) is left out by Refresh Palette, and put in the first category
(Battlefield) by Edit and OK.

- **Checked:** Aurora, the captures `wave1/saveas.mod`,
  `wave1/refresh-items.mod`, `wave1/pal2-refresh2.mod`,
  `wave1/pal2-newcat.mod` and `wave1/pal2-saveas.mod`.

### Build › Compile › Encounters

Each `CreatureList` entry's `CR` and `Appearance` are refreshed from its
creature blueprint (after the creatures' CRs); entries whose blueprint is
gone are deleted.

- **Checked:** Aurora, `aurora_encounters.rs`.
- **Moonglow:** `crates/mg-module/src/build.rs`.
- **Wiki:** missing.

### Script Wizard output that does not compile in EE

An "Actions Taken" script that opens a store with appraise checks and gives
a party reward includes both `nw_i0_tool` and `nw_i0_plot`, which both
define `HasItem`: the EE compiler fails with "DUPLICATE FUNCTION
IMPLEMENTATION".

- **Checked:** Aurora, `aurora_script_wizard.rs` (Aurora's own script fails
  with the built-in official compiler).
- **Wiki:** missing.

### The conversation tree's letters and Escape

Aurora's conversation tree goes to a line by its row's text as drawn,
and an NPC line's row begins with its speaker (`[OWNER] - Over here.`):
from Root, `o` went past that row to the player's `Off I go.`, `h` past
`[OWNER] - Hello there.` to `Hmm.`, and `[` to the first NPC row, the next
one when typed again. Letters typed in quick succession go by all of them
(`gol` to `Gold, please.`, past `Goodbye.`); a letter after a pause of two
seconds starts over. Escape with a line selected does nothing: the window
stays, and Down goes on from the line.

Under Wine the tree is Wine's own tree control, so two things are Wine's
and not necessarily Windows': the same letter typed again quickly (`gg`,
`ggg`) stayed on the first row beginning with it, and how long a pause
starts over.

- **Checked:** Aurora under Wine, driven on the off-screen display
  (2026-10-26), on a conversation made with `mg dialog-import` from a
  Twine story; watched, not captured (no test reads it).
- **Moonglow:** `crates/mg-ui/src/dialog_view.rs` (`row_text`,
  `type_ahead`), test `the_conversation_tree_s_letters_typed_quickly`.
  Moonglow goes round the rows beginning with a letter typed again and
  starts over after a second; its Escape hands the tree's keys back
  (`docs/deferred.md`, the Settled section).
- **Wiki:** missing.

### A module directory beside its .mod

Opening `modules/name.mod` with a folder `modules/name/` beside it,
Aurora asks, in a window titled "Open directory as a module": "A
directory with the same name has been found. Do you want to open this
directory as a module?", then "YES - open directory" and "NO - open
.mod file", with Yes and No (no Cancel). It asks from the welcome
dialog's list and from File's recent modules alike, every time (the
recent list keeps `name.mod` even when the folder was opened). Yes
loads the folder (its own `module.ifo`, not the `.mod`'s) and unpacks
nothing; No unpacks the `.mod` into `modules/temp0/` and loads that.
Saving the folder writes the changed files into it (the module's
`module.ifo` and the palettes; untouched files keep their times) and
packs the folder into `name.mod`, keeping the `.mod` that was there as
`name.BackupMod`. Aurora's Open lists `.mod` files only: a folder with
no `.mod` is not offered. Its Always open module directories (ini
`Always Open Module Directory`) opens the folder without asking.
Whether Aurora also asks for a folder of the name that holds no module
(no `module.ifo`) was not tried; Moonglow asks only for one that does.

The server does not read module directories: `nwserver -module name`
loads `modules/name.mod` with a folder `modules/name/` beside it (one
holding no module) as if the folder were not there, and with only a
folder of the name (no `.mod`) it fails: "LoadModule() Failed:
Unrecognized file type", "Unable to load module". So the game needs the
`.mod` that saving a module directory writes.

- **Checked:** Aurora 89.8193.37 under Wine, driven on the off-screen
  display (2026-10-26), on `dirprobe.mod` and `dirprobe/` told apart by
  their module names; the question captured as a picture
  (`captures/module-folder/prompt.png` in the oracle's folder), the
  rest watched (no test reads it). The server through `mg serve`
  (private), on a scratch user folder.
- **Moonglow:** `crates/mg-ui/src/module_folder.rs` (the question, in
  Aurora's words, and Options › General's A .mod with its folder beside
  it: ask, the folder, or the `.mod`), `write_mod_beside_folder` in
  `crates/mg-ui/src/lib.rs`; test
  `a_mod_with_its_folder_beside_it_asks_which_to_open`. GitHub issue 30.
- **Wiki:** [Open Module Folder](https://nwn.wiki/spaces/NWN1/pages/38176075/Open+Module+Folder)
  has the question's first two lines; the rest is missing.

## Files and tools

### NWSync repositories

A manifest is `NSYM`, version 3, entry count, mapping count; each entry a
20-byte SHA-1, u32 size, 16-byte name (lowercase, NUL-padded) and u16
type, sorted by SHA-1, name, type; resources repeating earlier bytes go in
the mapping section. Data files are `data/sha1/ab/cd/<sha1>`, each an
`NSYC` compressed buffer (magic, 3, algorithm 0/1/2 for none/zlib/zstd,
uncompressed size; zstd adds 1 and dictionary 0). Zstd frames must declare
their content size: neverwinter.nim decompresses in one shot. With the
module included, its `module.ifo` drops `Mod_HakList`, keeps `Mod_Hak` and
gains `Mod_UUID`; `nss`, `ndb` and `gic` are left out.

- **Checked:** against `nwn_nwsync_write`, `nwn_nwsync_print` and
  `nwn_compressedbuf` (manifests byte-identical), `nwsync.rs`. Not tested
  against the game.
- **Moonglow:** `crates/mg-module/src/nwsync.rs`, `crates/mg-erf`.
- **Wiki:** "NWSync technical guide" shows only the folders.

### Talk tables as `nwn_tlk`'s JSON (nasher's)

`nwn_tlk` writes `{"language":N,"entries":[{"id":…,"text":…,"sound":…,"soundLen":…}]}`,
compact with no line end after it (`-p`: two-space indents, `"key": value`,
still no line end; nasher passes neither unless `tlkFlags` does). An entry
is written when its text or sound is not empty, whatever its flags say;
`text` always, `sound` up to its last non-NUL byte, `soundLen` when the
length rounded to four places (in 64 bits) is above 0, as the 32-bit float
that rounding gives, printed as Nim prints a 64-bit float: shortest
digits, ties to the even one (2.4235000610351562, not …563), fixed while
the point is at most 17 digits in, else `d.ddde+NN`, `.0` after a whole
number. Strings are escaped as Nim's `escapeJson`: `\b \t \n \f \r`,
`\u000b` (lower case), `\u0000`–`\u0007`, `\u000E`–`\u001F` (upper case);
DEL and `/` as they are. Text is read and written in `--nwn-encoding`
(windows-1252 by default), never by the table's language: a byte the
codepage has no character for goes out as the raw byte (so the file isn't
UTF-8), a character it can't write comes in as its UTF-8 bytes, C1
controls included. Read back: flags are text 1, sound 2|4 (with or
without a length), a length alone none; the table has max `id` + 1
entries (one when there are none); a repeated `id` takes the later entry
whole; `language`, `entries`, `id` and `text` are required, `soundLen`
must be a float literal and not negative (otherwise an assertion). An
entry with flags and no text points at the running string offset, one
without flags at 0. A tlk target packs `<file's name>.tlk.json` from its
sources and fails on any other name.

- **Checked:** every shipped `dialog.tlk`/`dialogf.tlk` (ten tables, five
  languages), both ways and pretty, byte for byte
  (`tlk::shipped_talk_tables_as_json_as_nwn_tlk_has_them`); `nasher unpack`
  and `nasher pack` of a tlk target (`nasher::a_talk_table_kept_as_nasher_keeps_it`);
  probes of the edges with `-k debug`.
- **Moonglow:** `crates/mg-tlk/src/json.rs`, `crates/mg-module/src/nasher/tlk.rs`.
- **Wiki:** missing.

### The compiler's debug file (`.ndb`)

`NDB V1.0`, then a line of counts (files, structs, functions, variables,
lines); then `N00 <file>`, `s <fields> <name>` with `sf <type> <name>`
fields (vector is struct 0), `f <start> <end> <params> <ret> <name>` with
`fp <type>`, `v <start> <end> <stackoffset> <type> <name>`, and
`l<file> <line> <start> <end>`. Types `i f s o v`, `tNNNN` a struct, `eN`
an engine structure. Prototypes and global constants appear as functions
at `ffffffff`.

- **Checked:** against the compiler's output, `script_frontend.rs`,
  `script_analysis.rs`.
- **Wiki:** missing.

### Binary models from the game's own compiler

`nwmain compilemodel` stores `materialname` in a mesh's texture3 slot
(+0x1A8, dropping a `texture3` given with it) and `renderhint` as a u32 at
+0xE4 (1 None, 2 NormalAndSpecMapped, 3 NormalTangents); with a render hint
it stores tangents at +0x258 and handedness at +0x260. Bézier keys get the
column byte 0x10 | columns (`nwnmdlcomp` writes 0x19, and cuts
`orientationbezierkey` to 9 values). The game's IDs for the three-stop
emitter values are alphaMid 448, colorMid 452, percentStart/Mid/End
464–466, sizeMid 468, sizeMid_y 472, where `nwnmdlcomp` uses 464, 468 and
480–488. `nwnmdlcomp` drops `materialname` and `renderhint`.

- **Checked:** the game's compiler on authored probe models, in the
  client's sandbox; fixtures in `crates/mg-mdl/src/binary.rs` and `ctrl.rs`.
- **Wiki:** missing.

### Where tools differ from the engine

- `nwn_twoda` drops `""` cells, shifting the row, and crashes on a table
  with no non-empty rows (retail `iprp_base1.2da`).
- `nwn_gff` keeps fields in hash order and cannot write back JSON with an
  empty label (BioWare's own palettes have one).
- `nwn_script_comp --erfs` puts later archives above earlier ones.
- nasher drops `Mod_ID` and an area's `Version` when unpacking, and rounds
  floats to 4 places (−π becomes +π in `Bearing`): packed modules' objects
  move up to 0.011 m and turn up to 1° in the game.
- nwn.py's script-spec reader misses constants with lowercase letters
  (`DAMAGE_BONUS_1d4`).

- **Checked:** `twoda.rs`, `nwn_gff_diff.rs`, `scripts.rs`, `nasher.rs`,
  `engine_modules.rs`, `nwscript_spec.rs`.
