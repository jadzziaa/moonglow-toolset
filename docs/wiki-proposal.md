---
type: Proposal
title: Proposed corrections to nwn.wiki
description: Proposed corrections to fifteen nwn.wiki pages that say something the game or Aurora does differently - for each, what the page says, what it should say and how that was checked - in order of how much trouble the current text causes.
tags: [nwn-wiki, corrections, proposal]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-03T21:08:32Z }
sources:
  - id: findings
    resource: findings.md
    title: Moonglow's findings (the complete list, with the tests)
  - id: nwn-wiki
    resource: https://nwn.wiki
    title: nwn.wiki, page versions of 2026-10-03
---

# Proposed corrections to nwn.wiki

Fifteen pages of the [NWN wiki](https://nwn.wiki) say something that the
game or the Aurora toolset does differently. We found these while building
Moonglow Toolset, a reimplementation of Aurora, and checked each one in the
game (a private `nwserver`, or the client read back by screenshot or shader
uniform) or against files Aurora itself wrote (toolset 89.8193.37, EE 1.89).
This is a proposal for the wiki's editors: for each page, what it says now,
what we suggest it say, and how the claim was checked.

Page versions are those of 2026-10-03. The tests named are in Moonglow's
`crates/mg-corpus-tests/tests/`; they read the tester's own game install,
so anyone with the game can run them. `docs/findings.md` has the complete
findings, including many the wiki does not cover at all (not proposed
here: only contradictions are).

Entries are in order of how much trouble the current text can cause.

## Wrong, and likely to mislead

### 1. TLK: talk tables in haks do work

[TLK](https://nwn.wiki/spaces/NWN1/pages/38176005/TLK) (version 22),
"Adding a Custom TLK file to a module".

- **Says:** "Tlk files placed into a hakpak won't be recognized by the
  game". (The wiki's own Content Load Order page says they are.) The
  introduction also calls TLK "a GFF file".
- **Proposed:** The custom talk table named in the module properties
  (`Mod_CustomTlk`, stored without extension) is looked up in the module's
  haks first, then in the module, then in the user's `tlk` folder. The
  feminine table (`<name>f.tlk`) is looked up separately in the same order,
  so a hak's table can pair with a feminine one in the `tlk` folder;
  without a feminine table, female lookups return the masculine text. The
  module fails to load when the table is missing, when the stored name
  ends in `.tlk`, or, on Linux, when its case differs from the file's. In
  the introduction: TLK is its own binary format, not GFF.
- **Also worth adding:** an entry's text counts only when its text flag
  (`0x1`) is set; an entry with text but no flag reads as an empty string.
- **Checked:** engine, `engine_tlk.rs`
  `where_the_game_reads_custom_talk_tables` (8 cases: hak, module and
  folder, masculine and feminine, the failures).

### 2. GFF: GIT and GIC are swapped

[GFF](https://nwn.wiki/spaces/NWN1/pages/38175366/GFF) (version 5), the
table of GFF file types.

- **Says:** `.gic` is "Area Instanced Contents" and `.git` "Area Instanced
  Comments".
- **Proposed:** swap the two descriptions. `.git` holds the area's placed
  objects; `.gic` the toolset's comments for them, one struct per
  instance, in the same order.
- **Checked:** any module; Aurora's output (`aurora_instances.rs`).

### 3. Hakpacks over 2 GB: the game cannot read them either

[Resource Limits](https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits)
(version 7) and [HAK](https://nwn.wiki/spaces/NWN1/pages/53670235/HAK)
(version 4).

- **Says:** haks should stay under 2 GB because "the Toolset (and other
  tools like nwsync) doesn't read past 2GB".
- **Proposed:** The limit is the game's as well. A resource whose data
  *starts* at 2³¹ bytes or later in a hak is missing to the game: a 2DA
  reads as empty, a script does not run. A resource that starts before
  that point is read whole, even if it ends after it. A copy of the
  resource in a lower hak does not take over: the game finds the name in
  the higher hak, fails to read it, and stops.
- **Checked:** engine, sparse 2.1 GiB haks, `engine_big_hak.rs`
  (`the_game_reads_nothing_that_starts_past_2_gib_in_a_hak`,
  `past_2_gib_a_lower_haks_copy_doesnt_count`).

### 4. Item costs

[itemprops.2da, itempropdef.2da, iprp_costtable.2da and iprp_paramtable.2da](https://nwn.wiki/spaces/NWN1/pages/38175507/itemprops.2da+itempropdef.2da+iprp_costtable.2da+and+iprp_paramtable.2da)
(version 25), "Item Costs".

- **Says:** (a) the charge multiplier is in "iprp_charges.2da"; (b) of the
  cast-spell costs "the two highest are also added on again", half and a
  quarter; (c) the game does not recalculate an item's cost except when it
  is identified, uses charges, or has properties added or removed.
- **Proposed:**
  - (a) The table is `iprp_chargecost.2da`.
  - (b) The two are found in one pass in property order, and the pass has
    a bug: a cost above the current highest becomes the highest *without
    the old highest becoming the second*; otherwise a cost above the
    current second becomes the second. So the runner-up's extra quarter
    counts only when it comes after the most expensive spell. Sixteen
    base-game items are priced lower than a true top two would give
    (`x0_it_mneck002`: 6526, not 7538).
  - The charges ÷ 50 scaling applies only to `iprp_chargecost` rows 2–6 (5
    down to 1 charges per use), and only when the item has charges left.
    Single Use, 0 Charges/Use and the uses-per-day rows are never scaled.
  - The final sum, in single-precision float: plus = trunc(base + spells)
    + trunc(P² × 1000); minus = trunc(N² × 1000), with P and N the totals
    of the positive and the negative property costs; unit = trunc(max(plus
    − minus, 0) × ItemMultiplier); value = (unit + AddCost) × stack size.
    The additional cost counts once per item in the stack, and the
    per-item value is truncated before multiplying by the stack.
  - Scripts get 0 for plot items, creature items and gold, and otherwise
    at least 1.
  - (c) Add to the list: an item is also priced anew when it is created.
    `GetGoldPieceValue` straight after `CreateObject` returns the computed
    value, not the `Cost` stored in the blueprint (which is stale in about
    9% of the base game's blueprints).
- **Checked:** engine, `engine_item_cost.rs` `item_costs_match_the_engine`:
  all 2,763 base-game items created in `nwserver`, every value equal to
  the formula's; made-up wands for the charge rows. Negative costs and the
  no-charges-left case are covered only by unit tests (no base item has
  them). Items placed in areas and store stock were not probed for (c).

### 5. SET File Format: primary rules are in use

[SET File Format](https://nwn.wiki/spaces/NWN1/pages/195362833/SET+File+Format)
(version 2), "[PRIMARY RULES] / [SECONDARY RULES]".

- **Says:** "Not used in practice for custom content… Set `Count=0`."
- **Proposed:** 23 of the base game's tilesets have primary rules (up to
  192), and the toolset's terrain brush applies them: painting a corner
  looks up the rule for (Placed, placed height − base, Adjacent, adjacent
  height − base), where base is the lower of the two heights, for each of
  the eight corners around it, diagonals included; `Changed` and
  `ChangedHeight` give the neighbour its new terrain and base + height.
  Rules are not applied to the corners they change, and corners more than
  one step apart in height match no rule.
- **Checked:** Aurora, `aurora_terrain.rs` `painting_matches_aurora` (54
  scripted painting steps in four tilesets, compared tile by tile with
  what Aurora saved).
- **Note for the same page:** `Doors=` values and `[GROUPS] Count=` in
  custom tilesets in use on a live server are not always counts (we found
  `Doors=1869573190`, which is the bytes `Floo`, and a `Count=270` with no
  `[GROUP266]`). Readers should take the sections that exist rather than
  loop to the declared count.

### 6. Area Editor: Resize Area does not copy the edge tiles

[Area Editor](https://nwn.wiki/spaces/NWN1/pages/60982635/Area+Editor)
(version 17), "Resize Area", and "Update Instances" further down.

- **Says:** enlarging an area "will copy the same tiles that were on those
  edge areas"; Update Instances updates objects "to match the blueprint".
- **Proposed:**
  - Resize: the new part continues the edge's *terrain*, not its tiles.
    New corners take the terrain and height of the nearest old corner, new
    edges the crosser (road, stream, wall) of the nearest old edge, and
    the toolset then picks tiles at random among those that fit. Shrinking
    replaces what is left of a group cut by the new edge with plain terrain
    tiles and removes its doors.
  - Rotate: counter-clockwise by a quarter turn.
  - Update Instances remakes every placed copy as if it were placed anew.
    Only its position, facing, visual transform, a trigger's or
    encounter's outline and an encounter's spawn points are kept; the
    instance's own tag, name, scripts, variables, Plot and Locked flags, a
    door's transition and a sound's volume are lost.
- **Checked:** Aurora, `aurora_terrain.rs` `resize_and_rotate_match_aurora`
  and `aurora_update_instances.rs`.

## Wrong in a detail

### 7. Tile lights: lightcolor.2da and fixed radii

[Tileset Construction Tutorial](https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial)
(version 13), "Tile Light Colors", and
[Area Lighting](https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting)
(version 7), "Main Lights 1 / 2".

- **Says:** the tutorial, that tile lights take their colors from
  "tilecolor.2da"; Area Lighting, that a main light's "Strength (radius)"
  is "set in the MDL".
- **Proposed:** Main lights take their color from `lightcolor.2da`
  (`RED`, `GREEN`, `BLUE`; the `TOOLSETRED`… columns are only the
  toolset's swatches). The game draws main light 1 with radius 10 and main
  light 2 with radius 5 whatever the tile's model says; the model gives
  only the position. Source lights take their color from the `fx_flame01`
  animation their value names, with radius 7. This agrees with what the
  wiki's Model Special Nodes page already says for `ml1`/`ml2`.
- **Checked:** game client, `client_render.rs`
  `light_uniforms_match_the_client`: the light uniforms the client sends to
  its shader, read back through a debug copy of the game's own shader
  include (seven cases).

### 8. Fog and skyboxes

[Area Lighting](https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting)
(version 7), "Fog Clip Distance" and "Sky Box", and
[Render Distance with Fog and Skyboxes](https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes)
(version 5).

- **Says:** a skybox extends, or increases, the fog clip distance by 90 m.
- **Proposed:** A skybox does not change the fog. The fog's end is the
  Fog Clip Distance and its start is min(30 − fog amount, end − 1) meters
  from the camera, with or without a skybox; fog is on even at amount 0.
  If a skybox makes the game draw tiles 90 m further, that is the draw
  distance, and those tiles are fully fogged.
- **Checked:** game client, `client_render.rs`
  `fog_uniforms_match_the_client` (the shader's `fogStart`/`fogEnd`).
  **Not checked:** how far geometry is drawn with a skybox; the 90 m may
  well be right for that.

### 9. feat.2da: CRValue is read as an integer

[feat.2da](https://nwn.wiki/spaces/NWN1/pages/38175102/feat.2da)
(version 14), column `CRValue`.

- **Says:** "Float value. Recommended 0 instead of `****`".
- **Proposed:** The toolset's challenge rating reads the column as an
  integer: 0.5 and 0.2 count as 0, and `****` counts as 1. (So 0 rather
  than `****` is not only tidier, it changes the result.) The feats' share
  of the rating is 0.1 × L × F ÷ (L/2 + 7), with L the creature's total
  level and F the sum over its feats.
- **Checked:** Aurora, `aurora_creatures.rs`
  `challenge_ratings_match_aurora` (4,204 creatures recalculated by
  Aurora). The whole formula is in `docs/findings.md`; the wiki has none
  and it could make a page of its own.

### 10. Creature: the toolset validates all casters' spells

[Creature](https://nwn.wiki/spaces/NWN1/pages/38174829/Creature)
(version 6).

- **Says:** the toolset does very light validation of spells "(only on
  clerics?)".
- **Proposed:** On OK the toolset checks every spellcasting class
  (wizards, sorcerers and bards too) and shows one warning per class, the
  first that applies: a spell above the highest level the class can cast;
  a casting ability below 10 + the spell's level; too many spells of a
  level. Its slot count gives preparing casters bonus spells at level 0
  as well, which the game does not.
- **Checked:** Aurora, `aurora_spell_warnings.rs` (30 creatures).

### 11. Content Load Order: where a hak is beats its place in the list

[Content Load Order](https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order)
(version 18), "Order of Hakpacks".

- **Says:** "the highest (first loaded) hakpack takes precedence".
- **Proposed, to add:** This holds among haks in the same folder. A hak in
  the user's `hak` folder outranks one in the install's `data/hk`, even
  when it is listed lower in the module's hak list.
- **Checked:** engine, `engine_resman.rs` `resolution_matches_engine`.

## Imprecise

### 12. MDL ASCII Emitter Nodes

[MDL ASCII Emitter Nodes](https://nwn.wiki/spaces/NWN1/pages/26738875/MDL+ASCII+Emitter+Nodes)
(version 27).

- **Says:** `m_isTinted` tints particles "to the ambient colour of the
  scene"; `twosidedtex` decides "if the particle is visible from both
  sides".
- **Proposed:** `m_isTinted` multiplies the particles by the light at the
  emitter: the area's ambient plus its diffuse color, plus point lights.
  `twosidedtex` has no visible effect (one-sided particles show from
  behind too). `mass`: gravity is mass × 9.8 m/s². `bounce_co`: a particle
  hitting the ground keeps 0.8 of its speed along the ground and 0.8 ×
  `bounce_co` of its speed off it.
- **Checked:** game client screenshots of probe emitters (`client_render.rs`
  `particles_look`). This was exploration, measured from pictures, without
  asserted tests: the weakest evidence in this list.

### 13. Creature JSON: MaxHitPoints

[Creature JSON](https://nwn.wiki/spaces/NWN1/pages/38176577/Creature+JSON)
(version 10), field `MaxHitPoints`.

- **Says:** "This appears to be HitPoints + Constitution bonuses/penalties."
- **Proposed:** As the toolset writes it: HitPoints + Constitution
  modifier (base Constitution plus the racial adjustment) × total level,
  plus the total level again with the Toughness feat; never below 1. Epic
  Toughness is not included.
- **Checked:** Aurora, `aurora_creatures.rs` `max_hit_points_match_aurora`;
  1,520 of the 1,582 base-game creatures store this value.
