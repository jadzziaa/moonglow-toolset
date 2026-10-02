# The EE data Aurora has no fields for

October 2026. Item 9 of `community_pain_points.md`. Enhanced Edition
added things to the game's objects that its toolset never got fields for.
Builders set them with a GFF editor or from scripts at load time, and
Aurora dropped some of them on save (nwn-issues #528, #370, #566, #816).

The map said each one is checked in the engine before it gets a field.
None of these fields occurs in any file the game or its modules ship, so
the shipped files couldn't give their names. The engine gave them:
- **Names:** a probe module applied each change by script
  (`ReplaceObjectTexture`, `ReplaceObjectAnimation`,
  `SetMaterialShaderUniform*` and so on) and logged `ObjectToJson`.
- **What the game reads back:** the same fields were written into
  blueprints and placed objects, and the game's getters were compared.

`crates/mg-corpus-tests/tests/engine_ee_fields.rs` keeps the checks.

## What the game stores and reads

### Object visuals

| Field | Holds | Script calls |
| --- | --- | --- |
| `TextureReplace` (struct 9) › `TextureReplaceLi` [`OldTexture`, `NewTexture` resrefs] | textures drawn as others | `ReplaceObjectTexture` |
| `AnimationReplace` (struct 11) › `AnimationReplace` [`OldAnimation`, `NewAnimation` resrefs] | animations played as others | `ReplaceObjectAnimation` |
| `Material` (struct 7) › `ShaderParams` [`Material`, `Param` strings, `Type` byte (1 integer, 2 vector), `Int` or `Float1`–`Float4`] | shader uniforms | `SetMaterialShaderUniformInt`, `…Vec4` |
| `MiscVisuals` (struct 8): `HiliteColorR/G/B` floats (-1: the game's), `MouseCursor` int (-1), `TextBubbleText`, `TextBubbleType` (0–3), `UiDiscoverMask` (bits 1, 2, 4, 8), `VisibleDistance` (45 m) | the rest | `SetObjectHiliteColor`, `SetObjectMouseCursor`, `SetObjectTextBubbleOverride`, `SetObjectUiDiscoveryMask`, `SetObjectVisibleDistance` |

**Which objects:**
- **All four** are read on placed placeables, doors, creatures and items,
  and on blueprints created by script.
- **`MiscVisuals` alone** is read on triggers.
- **None of them** are read on waypoints, sounds, stores or encounters.

A partial `MiscVisuals` (only `VisibleDistance`) leaves the other fields
at the game's defaults.

**In Moonglow:**
- **Editors:** a Visuals page in those editors. A changed `MiscVisuals`
  field keeps the struct's others.
- **Drawing:** the area view and model viewer draw texture replacements
  (`mg_preview::replaced`). PLT textures keep their coloring, since the
  game doesn't replace those. An idle animation that's replaced plays
  the new one.

### Areas and tiles

**Area flags:** the game keeps `Flags` bits past interior (1),
underground (2) and natural (4). An area with `0x109` still reads as
interior, and `ObjectToJson` returns 265. Custom shaders read them from
their `areaFlags` uniform (nwn.wiki *Shaders and Area Flags*). Area
Properties › Advanced has checkboxes for bits 8 to 32768, and keeps any
higher bits.

**Tile replacement texture:** the game keeps a tile's `Tile_ReplaceTex`
byte, a `replacetexture.2da` row. The tile model's texture named
`replace_tex` is drawn with that row's `TEXTURENAME`. One game model has
such a texture (`tms01_c08_01`). Tile Properties sets it, and the area
view draws it.

### Creatures

**Familiars:** `FamiliarType` (int, a `hen_familiar.2da` row) and
`FamiliarName` are read only when a class has a familiar. With a wizard
class they're read; without one they read as 0 and "".

**Animal companions:** `CompanionType` and `CompanionName` are read only
when a class has a companion. A druid, or a ranger at level 3 or at
level 6, has them read; a cleric doesn't.

**Which classes:** `classes.2da` gives the rule. A familiar comes with an
arcane class (`Arcane` 1) whose `MinAssociateLevel` isn't 255, a
companion with a divine class (`Arcane` 0) likewise. The level in
`MinAssociateLevel` doesn't gate reading the fields: a level-3 ranger's
companion is read.

**Domains and school:** `Domain1`, `Domain2` and `School` (bytes) on a
class's `ClassList` entry are read for classes with `PickDomains` or
`PickSchool` (`GetDomain`, `GetSpecialization`). A cleric blueprint
without them gets domains from the game (nw_mumcleric: 0 and 1).
Moonglow shows a missing one as "Not set" instead of guessing.

### Placeables and items

**Placeables:** a placeable with `HasInventory` 1 and `Useable` 0 keeps
its inventory, and scripts can walk it. Moonglow already allowed this, as
long as the placeable isn't static. The editor now says what it means.

**Item stacks:** a stack of 5,000 arrows (the base item stacks to 99)
stays 5,000. The item editor now allows stacks up to 65,535 and notes
when one is past the base item's limit.

**Charges:** 255 charges read as 250, so Aurora's 250 limit is the
game's. It stays.

## Not done

- **Item costs:** `AddCost` past Aurora's limit wasn't changed. The game
  uses the stored `Cost`, which Moonglow computes (`engine_item_cost.rs`).
- **Custom shader effects:** custom shaders reading the shader parameters
  or area flags aren't drawn. Moonglow's renderer doesn't run custom
  shaders.
