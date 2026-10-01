# Levelling up by package, and the Creature Wizard (Aurora's)

Aurora's Levelup Wizard (a creature's Classes page, or its context menu in
the area viewer) and Creature Wizard (Wizards menu) level creatures up by
their classes' packages. Moonglow's: `mg_rules::levelup` (`level_up`,
`new_class_gear`) and `mg_module::blueprints::creature`. Worked out from
creatures levelled and made in Aurora 89.8193.37 and checked against them
(`aurora_levelup.rs`, `aurora_creature_wizard.rs`; captures under
`levelup/` and `creature-wizard/`).

## Each level

In class slot order, one level at a time:

- **Package**: the class's (classes.2da `Package`), not the creature's
  `StartingPackage`.
- **Hit points**: a player class's (`PlayerClass`) whole hit die at the
  character's first level; otherwise the average, (HitDie + 1) / 2, summed
  and rounded down (Fighter 1 to 10: + 49; a goblin's Humanoid 1: 4).
- **Ability**: at every fourth character level, + 1 to the package's
  `Attribute`.
- **Skills**: `SkillPointBase` + Intelligence modifier + the race's
  `ExtraSkillPointsPerLevel` (times `FirstLevelSkillPointsMultiplier` at the
  first level), spent a rank at a time on the class skills (the class's
  `SkillsTable`) down the package's `SkillPref2DA`, round after round while
  points last, up to the character level + 3. Cross-class skills get
  nothing. (Human Fighter 1: 12 points, Discipline, Parry, Heal and Craft
  Weapon 2, four more 1.)
- **Feats**, in this order: the race's `FeatsTable` (first level); a
  cleric's domain powers (the package's `Domain1`, `Domain2`; domains.2da
  `GrantedFeat`) at the class's first level; the class's own (its
  `FeatsTable`, list 3, `GrantedOnLevel`), each replacing a feat whose
  `SUCCESSOR` it is (Sneak Attack 2 replaces Sneak Attack); then the picks:
  a normal feat at the first and every `NormalFeatEveryNthLevel` (3)
  character level (plus `ExtraFeatsAtFirstLevel`), and a bonus feat where
  the class's `BonusFeatsTable` has one. A pick is the first of the
  package's `FeatPref2DA` the levelling class lists (its `FeatsTable`:
  list 0 or 1 for a normal feat, 1 or 2 for a bonus one; `ALLCLASSESCANUSE`
  does not count: a rogue does not take Weapon Finesse) whose prerequisites
  hold (`MINATTACKBONUS`, the ability minimums, `PREREQFEAT1`/`2`,
  `OrReqFeat0`–`4`, skills, `MinLevel`/`MinLevelClass`, `MaxLevel`,
  `MinFortSave`, `PreReqEpic`, `MINSPELLLVL`). The level's granted feats
  count as prerequisites (Martial Weapon Proficiency lets a new fighter
  take Weapon Focus) but its other picks do not (Weapon Focus and Weapon
  Specialization are not taken the same level).

## A class new to the creature

- **Spells**: a class that prepares them (`MemorizesSpells`) gets each
  spell level's slots (`SpellGainTable`, plus a bonus slot per four points
  of the casting ability's modifier from the spell level up) filled with a
  domain spell (the first domain's for that level, else the second's) and
  the package's spells of that level (`SpellPref2DA`) in order, the first
  repeated in what is left (a cleric's five cantrips: Inflict Minor Wounds
  five times). One that knows them gets the package's first spells of each
  level up to its `SpellKnownTable` (not checked against Aurora). Entries
  are Spell, SpellMetaMagic 0, SpellFlags 1.
- **Equipment**: the package's `Equip2DA` items in order: each to the
  first slot of its base item's `EquipableSlots` if free and a weapon,
  shield, armour, helmet or ammunition (baseitems.2da `Category` 1–8; a
  torch is carried even with the left hand free), else into the backpack
  at the first place it fits (10 wide, row by row, the base item's size).

## The Creature Wizard

Pages: welcome, racial type, classes and levels (the race's
`ToolsetDefaultClass` at level 1), gender, appearance (the race's) and
portrait (required), faction (Hostile), name (Aurora: a random one), palette
category, review, finish (Launch Creature Properties).

The creature: Aurora's fields in its order (`blueprints::creature`); the
first class's recommended abilities (classes.2da `Str`–`Cha`, stored
without the racial adjustments) and package; the race's alignment (a table
of Aurora's own: dwarf lawful good; elf and half-elf chaotic good; gnome
neutral good; half-orc and fey chaotic neutral; goblinoid neutral evil;
aberration, monstrous and orc chaotic evil; halfling, human, animal, beast,
dragon, reptilian, elemental and giant true neutral; the racial types past
the wizard's first screen not captured); a dragon's or elemental's generic
hide (`nw_it_creitemdra`, `nw_it_creitemele`; a construct's
`nw_it_creitemcon` by analogy) in the creature armour slot, which does not
count as gear in its challenge rating; body parts, head and
colours only for appearances made of parts (MODELTYPE P); the walk rate of
the appearance's `MOVERATE`; the `x2_def_*` scripts; then levelled up from
nothing as above, each class's package equipment, an empty backpack left
out, and the hit points and challenge rating derived. Aurora writes
`Interruptable` 144, `NoPermDeath` 95, `Disarmable` 16 (read as set;
Moonglow writes 1) and `SoundSetFile` 24448 (no sound set; Moonglow the
same).

Names: a random one from the race's letter tables (`NameGenTableA` +
`m`/`f`/`l`, `mg_rules::names`, following nwn.wiki's description of the
generator; random, so not compared with Aurora's).

Checked: nineteen creatures of seventeen racial types, Elf Wizard 3 among
them, match Aurora's field for field.
