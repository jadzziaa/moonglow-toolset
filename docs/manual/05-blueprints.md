---
type: Manual Page
title: Blueprints
description: Blueprints - palettes, the blueprint editors and wizards, and instances.
tags: [manual, blueprints, palettes]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-04T02:52:17Z }
---

# Blueprints

A **blueprint** is a template for an object: a creature (`.utc`), door
(`.utd`), encounter (`.ute`), item (`.uti`), merchant (`.utm`), placeable
(`.utp`), sound (`.uts`), trigger (`.utt`) or waypoint (`.utw`). Placing
one in an area makes an **instance**, a copy you can change on its own.

## Palettes

The palette pane (right) shows the blueprints by type:

- **Standard**: the game's blueprints (and those of the module's haks),
  in the game's categories.
- **Custom**: the module's own blueprints, in the categories you give
  them.

Choose a blueprint to place it in an area, or drag it there. Hover over
one to see its resref, tag and, for creatures, doors, items and
placeables, a picture.

**Find** (the box above the palette) shows the blueprints that have every
word you type in their name, resref or tag, in any order (`chest secret`).
If none has them all, it shows close matches, letters in order (`lngswd`
finds Longsword).

**Favorites** and **Recent** sit at the top of each palette. Right-click a
blueprint › **Add to Favorites**; Recent holds the last dozen you placed.
Both are kept between sessions.

Drag a custom blueprint onto another category of the custom palette to
move it there.

The palette's buttons:

- **Edit**: open a custom blueprint in its editor (or double-click it).
- **Edit Copy**: copy any blueprint, standard or custom, into the module
  as a new custom one, and open it.
- **Delete**: remove a custom blueprint from the module.
- **Preview**: show the blueprint in the model viewer. A blueprint
  without a model (a merchant, a sound, a trigger, a waypoint, an
  encounter) shows its fields instead, and a merchant what it sells, page
  by page.
- **Update Instances** (custom blueprints): remake the objects placed
  from the blueprint.

**Ctrl+click** chooses several custom blueprints of one type. With
several chosen, the right-click menu offers:
- **Edit N Together**: one editor for all of them. A change is set on
  each, as one undoable step (in Variables: what you add, change or
  delete, each blueprint keeping its other variables). Inventories, classes, skills, feats, spells
  and item properties are edited one blueprint at a time, so those pages
  aren't offered.
- **Update Instances of N**.

A custom category's right-click menu updates the instances of every
blueprint in it.

Long lists are in the order of their names: a creature's feats and
spells, and dropdowns of more than a dozen choices (appearances, base
items, sound sets…). Short ones keep the game's order, where it means
something (movement rates, difficulties). The Creature Wizard offers the
racial types that have a name, and a monster's portraits whatever its
gender.

### Update Instances

Update Instances remakes each object placed from a blueprint, as Aurora's
does. It keeps where the object stands, which way it faces, a trigger's
or encounter's outline and a visual transform. Everything else comes from
the blueprint, including the object's tag, name, scripts, local variables
and a door's transition. A window lists the objects it would change:
- **Every area**, or **only** the area shown.
- **Untick** objects to leave them as they are.
- **Update** changes the rest as one undoable step.

New blueprints come from the **Wizards** menu (or the palette's New). Each
type's wizard asks what Aurora's asks (the base item for an item, the
appearance for a placeable, the classes for a creature…), a palette
category and a name, then opens the new blueprint's properties.

## The editors

Each blueprint opens as a tab with Aurora's pages. Every change takes
effect at once and is one undoable step; there is no OK or Cancel.
Fields Moonglow does not show are kept as they are.

Shared parts:

- **Name** and other text players see: localized strings (the **…**
  button opens String Edit).
- **Tag** and **ResRef**: changing a blueprint's ResRef renames it
  everywhere, objects placed from it included (see
  [Modules](03-modules.md)); Edit Copy makes a copy under another name.
- **Scripts**: event scripts, picked from the module's and the game's
  (**Edit** opens one).
- **Variables…**: local variables the object starts with. **Save Set…**
  keeps them under a name. **Add Set** adds a saved set to any object's
  or blueprint's variables, in any module; a variable of the same name
  takes the set's value. Sets are small JSON files in Moonglow's data
  folder (`variable-sets`), easy to share.
- **Comments**: notes for the builder, not seen in the game.
- **Advanced**: the palette category, and **Update Instances** (every
  type but waypoints; see above).
- **Visuals** (creatures, items, placeables, doors; triggers have the
  last part only): what Enhanced Edition's scripts can change about an
  object's look, set from the start. Aurora has no fields for these; the
  game reads them when it loads the object.
  - **Texture replacements**: a texture of the object's model drawn with
    another (not PLT textures, which keep their colors). The area view
    and the model viewer show them.
  - **Animation replacements**: an animation played as another.
  - **Shader parameters**: an integer or four numbers a material's shader
    reads, for custom shaders.
  - The highlight color, the mouse cursor over the object, its text
    bubble (its name, or text that replaces it or goes before or after
    it), whether it is highlighted and named on mouse-over and with Tab,
    and how far away it can be seen (45 m unless set).

### Creatures

Basic (name, race, gender, portrait, appearance, faction, conversation),
**Statistics** (abilities, saves, armor class, hit points, speed),
**Appearance** (body parts, colors, wings, tail, the phenotype), **Classes**
(classes and levels, a cleric's domains and a wizard's school, the
familiar and animal companion; **Levelup Wizard…** levels the creature up
by the classes' packages), **Skills**, **Feats**, **Spells** (known or prepared by
class and level), **Special Abilities**, **Inventory**, Scripts,
Advanced, Comments.

Moonglow recomputes the creature's hit points and challenge rating with
every change, as Aurora does on OK.

- **Appearance** opens with the creature's model, as it looks now and as
  large as the window has room for; its wings, tail and colors, and its
  body parts, are to the right, so a change shows as you make it. **Pop
  Out**, among the viewer's buttons, moves the model to a window of its
  own (to keep beside other pages); **Bring Back** returns it to the page.
- **Feats** and **Spells** are found by name (**Find**) and by category:
  the toolset's categories of feats (combat, defensive, magical…), and
  spells' talent categories (harmful ranged, beneficial healing…).
- **Special Abilities** lists each ability as the game has it: so many
  **Uses** of a spell at a **Caster Level**. Clicking a spell adds a use;
  the same spell at another caster level is another ability.

The game reads a familiar only when one of the creature's classes has
one (an arcane class whose `MinAssociateLevel` in `classes.2da` isn't
255: Wizard, Sorcerer), and an animal companion only for a divine class
(Druid, Ranger, at any level). The Classes page says when it won't.
A domain or school left **Not set** is the game's to choose.

- On the **Spells** page, a warning names a class's first problem:
  spells of a level the class cannot cast yet, a casting ability too low
  for the spell level, or more spells than the class allows.
- In the **Inventory**, equip items from the item palette into the
  equipment slots, or put them in the backpack. Equipping an item the
  creature lacks the feat for (a weapon proficiency, an armor weight)
  asks whether to give it the feat. Items can be marked droppable,
  pickpocketable and infinite (merchants).
- The **Creature Wizard** makes a creature from a race, classes and levels,
  gender, appearance, portrait, faction and name, leveled as the game's
  packages level it.
- **Portraits…** chooses the portrait from the game's, by race and
  gender.

### Items

**General** (name, base item, cost, charges, stack size, plot, stolen,
cursed, identified; the base item's statistics), **Appearance** (by the
base item: a model, three weapon parts, or the armor's parts and colors,
with the inventory icon as the game shows it), **Properties** (the item
properties its base item allows, with their parameters), Visuals,
Description, Comments. Moonglow recomputes the item's cost with every
change, as the game computes it. A stack can exceed its base item's
limit (Aurora stops there; the game keeps the stack whole). Charges go up
to 250, the most the game reads.

An armor's **Part Colors** (under its colors on the Appearance page) give
one part a color of its own: choose the part, then a color for any of its
six channels. A channel left at **Default** takes the armor's color;
**Reset** returns one to it. Parts with colors of their own are marked
`*` in the list. The preview, and a creature wearing the armor, show
them. Aurora has no page for these (the game added them for scripts) and
Moonglow keeps the ones an armor already has.

### Placeables and doors

Basic (name, appearance, initial state, hit points, saves, plot, static,
usable, has inventory), **Lock** (locked, key, DCs), **Trap**, **Area
Transition** (doors: to a door or waypoint in another area), Scripts,
Advanced, Visuals, Description, Comments. A placeable with an inventory
has an **Inventory** page. It can have one without being usable: players
can't open it, but scripts can reach what it holds.

### Triggers and encounters

Triggers: Basic (generic, area transition or trap), Area Transition, Trap,
Scripts, Advanced, Visuals, Comments. Encounters: Basic (difficulty, spawn option,
respawns), **Creature List** (from the creature palette, with unique
creatures marked), Scripts, Advanced, Comments.

### Merchants

Basic (name, sell markup and buy markdown, identify price, stolen goods,
the most it pays for an item, its gold),
Advanced, **Restrictions** (base items the merchant will not buy, or will
only buy), Comments, and the **Inventory** in its store pages (Armor,
Weapons, Potions & Scrolls, Rings & Amulets, Miscellaneous), each item
optionally infinite.

Wherever items are added from the item palette (a merchant's or a
placeable's inventory, a creature's backpack), the item chosen in the
palette is shown beside it before you add it: its icon, base item, cost,
weight, damage or armor class, the level and Lore it needs, and its
properties.

### Sounds

Basic (the sounds, volume, **Play** and **Stop** to hear them),
**Positioning** (everywhere in the area, or from a place, with its
distances), **Advanced** (active, play style: once, repeating or seamlessly
looping; random or in order; interval and its variation; times of day;
pitch and volume variation), Comments.

### Waypoints

Basic (name, tag, appearance, map note), Advanced, Description, Comments.

Right-click a blueprint in a palette › **Find References** to see where
the module places or names it.

## Instances

An object placed in an area has the same pages as its blueprint
(double-click it, or right-click › **Properties**). Changing an instance
changes neither its blueprint nor other instances. **Add to Palette**
turns an instance into a new custom blueprint, and **Update Instances**
on a blueprint remakes its instances from it.
