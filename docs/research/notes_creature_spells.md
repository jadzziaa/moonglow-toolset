# Creature spell and inventory warnings (Aurora's)

Options › General has two creature warnings: "Show invalid creature spell
assignment warning" and "Show creature inventory warning". Worked out from
probe creatures in Aurora 89.8193.37
(`examples/spell_warning_probe_module.rs`; captures under
`spell-warnings/` and `inventory/`) and checked against them
(`aurora_spell_warnings.rs`, `aurora_inventory.rs`). Moonglow:
`mg_rules::spell_warnings`, `GameData::missing_feats`.

## Spell assignments

Aurora checks a creature's spells when its properties close with OK, even
untouched, and asks for each problem (Yes closes, No stays; "Never warn
again" turns the option off). Each spellcasting class, in class order, gets
at most one warning, the first of:

1. **Too high** (dialog.tlk 67093, "…too high for its current %s level"): a
   spell list above the highest level the class casts at its level
   (`SpellGainTable` row level − 1, `NumSpellLevels`). Wizard 1 with a
   level 2 spell; Wizard 2 still too high; Bard 1 with a level 1 spell.
2. **Ability too low** (67094, "…because its %s is too low"; the ability's
   name, dialog.tlk 131–136): a spell level whose 10 + level the casting
   ability (`SpellcastingAbil`) does not reach, the race's adjustment
   included (a half-orc's Int 12 is 10: no level 1 spells). Checked after
   too high: Wizard 1, Int 10, with a level 2 spell gets "too high".
3. **Too many** (67095, "…a maximum of %d level %d spells, but has been
   assigned %d spells"), the first level over:
   - a class that prepares its spells (`MemorizesSpells`): the
     `SpellGainTable` slots plus the bonus slots, (modifier − level) / 4 + 1
     when the modifier reaches the level, level 0 included (Wizard 1,
     Int 10: 3 + 1 = 4 cantrips; Int 18: 3 + 2 = 5; Wizard 1, Int 18: 1 + 1
     = 2 level 1 spells; Wizard 3, Int 14: 1 + 1 = 2 level 2; Cleric 1,
     Wis 12: 2 + 1 = 3 level 1);
   - one that knows them: its `SpellKnownTable` count, no ability bonus,
     but one more at level 0, the same rule with a modifier of 0
     (Sorcerer 1: 4 + 1 = 5 cantrips at Cha 10 and Cha 18, 2 level 1
     spells; Bard 1: 5 cantrips; Sorcerer 2: 6).

The lists checked are the ones the Spells page edits: `MemorizedList0`–`9`
for a class that prepares spells, `KnownList0`–`9` otherwise; every entry
counts at its list's level.

Moonglow has no OK to check on: the Spells page shows the warnings (the
talk table's text without its closing question) while the spells are
edited, with Never warn again.

## Inventory

**The notice** (dialog.tlk 67646): opening a creature's inventory, Aurora
says the game may unequip what the creature's feats or level do not allow
(standard equipment only), with Never warn again. This is what "Show
creature inventory warning" turns off. Moonglow shows it on the creature's
Inventory page.

**Equipping** (9069, not tied to the option): an item whose feats the
creature has none of brings "This creature cannot equip the selected item
because the creature is missing one or more of the following feats:" with
the feats' names, a line each, and "Do you wish to add the first feat listed
above…?". Yes adds the first feat to `FeatList` and equips the item; No
leaves the slot as it was. The feats: the base item's `ReqFeat0`–`5` (any
one will do; a longsword lists Weapon Proficiency (martial) and (Elf)), or
for armor, which lists none, the proficiency its armor class takes
(hardcoded feats: AC 1–3 Light, 4–5 Medium, 6 and up Heavy; banded mail,
AC 6: Armor Proficiency (heavy)). Moonglow asks the same on Equip, and Yes
is one undoable step.
