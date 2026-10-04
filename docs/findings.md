---
type: Finding
title: Findings
description: What Moonglow's authors learned about NWN:EE that is undocumented or documented wrongly - item costs, creatures, areas and tiles, talk tables and 2DAs, haks, GFF fields, what Aurora writes - each saying how it was checked, where Moonglow implements it and what nwn.wiki says.
tags: [findings, engine, aurora, gff, 2da, nwn-wiki]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-03T21:08:34Z }
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
NWN Lexicon. BioWare's format PDFs, linked from the wiki, could not be
searched; a few facts here may be in them.

## Items and costs

### An item's value (`GetGoldPieceValue`, UTI `Cost`)

All arithmetic is single-precision float.

1. **Base cost:** `baseitems.2da BaseCost`. For armor (`ModelType` 3),
   `armor.2da COST` at the row given by `parts_chest.2da ACBONUS` of
   `ArmorPart_Torso`, rounded.
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

For a light color c (MDL `color` × `multiplier`, or lightcolor.2da
RED/GREEN/BLUE): intensity = max(1, the largest channel of c); the shader's
color is (c / intensity)^2.2; its cutoff = radius × 2.0 × intensity, and
`lightColor.a` = ±cutoff², negative for `ambientonly`. A color above 1
makes a light reach further, not brighter (BrightWhite 2.0 on a radius-10
light: color 1, cutoff 40 m). `lightMaxIntensityInv` = 1.5^−2.2 ≈ 0.4098;
`lightFalloffFactor` = 2² × (0.2^−2.2 − 1.5^−2.2) ≈ 136.33.

Tile main lights use lightcolor.2da RED/GREEN/BLUE (TOOLSETRED… are only
the toolset's swatches) with radius 10 (ml1) and 5 (ml2) whatever the model
says; source lights take their color from the `fx_flame01` animation named
by the value, radius 7.

- **Checked:** game client, `client_render.rs` `light_uniforms_match_the_client`
  (seven cases).
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
the picture's bottom.

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
`twosidedtex` change nothing visible.

- **Checked:** game client screenshots, `client_render.rs` `particles_look`
  (exploration, no assertions; numbers in `docs/research/notes_models.md`).
- **Moonglow:** `crates/mg-render/src/particles.rs`.
- **Wiki:** "MDL ASCII Emitter Nodes" describes `mass`, `bounce_co`,
  `m_isTinted` and `twosidedtex` loosely or differently.

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

### Smaller layout facts

- The GFF page swaps GIT and GIC: `.git` holds an area's instances, `.gic`
  the toolset's comments for them, one struct per instance, same order.
- Store blueprints keep their palette category in `ID`, not `PaletteID`.
- The engine reads modules the same with their GFF fields in any order and
  with unknown fields added (`engine_modules.rs`, 121,000 objects;
  `engine_locked.rs`).

## What Aurora writes

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

- **Checked:** Aurora, `aurora_new.rs`; game data, `palettes.rs`
  `custom_palettes_rebuild_like_aurora`.
- **Moonglow:** `crates/mg-module/src/palette.rs`, `new.rs`.
- **Wiki:** "Toolset Palette ITP" doesn't give these rules.

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
