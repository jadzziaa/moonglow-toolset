# Blueprints

A **blueprint** is a template for an object: a creature (`.utc`), door
(`.utd`), encounter (`.ute`), item (`.uti`), merchant (`.utm`), placeable
(`.utp`), sound (`.uts`), trigger (`.utt`) or waypoint (`.utw`). Placing
one in an area makes an **instance**, a copy that can then be changed on
its own.

## Palettes

The palette pane (right) shows the blueprints by type:

- **Standard**: the game's blueprints (and those of the module's haks),
  in the game's categories.
- **Custom**: the module's own blueprints, in the categories you give
  them.

Choose a blueprint to place it in an area. The palette's buttons:

- **Edit**: open a custom blueprint in its editor (a double click does
  the same).
- **Edit Copy**: copy any blueprint, standard or custom, into the module
  as a new custom one, and open it.
- **Delete**: remove a custom blueprint from the module.
- **Preview**: show the blueprint in the model viewer.

New blueprints come from the **Wizards** menu (or the palette's New): a
wizard for each type asks what Aurora's asks (the base item for an item,
the appearance for a placeable, the classes for a creature…), a palette
category and a name, then opens the new blueprint's properties.

## The editors

Each blueprint opens as a tab with Aurora's pages. Every change takes
effect as you make it and is one undoable step; there is no OK or Cancel.
Fields Moonglow does not show are kept as they are.

Shared parts:

- **Name** and other text players see: localized strings (the **…**
  button opens String Edit).
- **Tag** and **ResRef**: changing a blueprint's ResRef renames it
  everywhere, objects placed from it included (see
  [Modules](03-modules.md)); Edit Copy makes a copy under another name.
- **Scripts**: event scripts, picked from the module's and the game's
  (**Edit** opens one).
- **Variables…**: local variables the object starts with.
- **Comments**: notes for the builder, not seen in the game.
- **Advanced**: the palette category, and **Update Instances** (every
  type but waypoints): every object placed from this blueprint, in every
  area, made again from it, where it stands and facing as it faces.
- **Visuals** (creatures, items, placeables, doors; triggers have the
  last part only): what Enhanced Edition's scripts can change about how
  an object looks, set from the start. Aurora has no fields for these;
  the game reads them when it loads the object.
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

The game reads a familiar only when one of the creature's classes has
one (an arcane class whose `MinAssociateLevel` in `classes.2da` isn't
255: Wizard, Sorcerer), and an animal companion likewise for a divine
class (Druid, Ranger, at any level). The Classes page says when it won't.
A domain or school left **Not set** is the game's to choose.

- On the **Spells** page, a warning names the first problem a class has:
  spells of a level the class cannot cast yet, a casting ability too low
  for the spell level, or more spells than the class allows.
- In the **Inventory**, equip items into the equipment slots and put
  others in the backpack, from the item palette. Equipping an item the
  creature lacks the feat for (a weapon proficiency, an armor weight)
  asks whether to give it the feat. Items can be marked droppable,
  pickpocketable and infinite (merchants).
- The **Creature Wizard** makes a creature from a race, classes and levels,
  gender, appearance, portrait, faction and name, levelled as the game's
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
change, as the game computes it. A stack can be larger than its base
item's limit (Aurora stops there; the game keeps the stack whole), and
charges go up to 250, the most the game reads.

### Placeables and doors

Basic (name, appearance, initial state, hit points, saves, plot, static,
usable, has inventory), **Lock** (locked, key, DCs), **Trap**, **Area
Transition** (doors: to a door or waypoint in another area), Scripts,
Advanced, Visuals, Description, Comments. A placeable with an inventory
has an **Inventory** page. A placeable can have an inventory without being
usable: players can't open it, but scripts can reach what it holds.

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

### Sounds

Basic (the sounds, volume, **Play** and **Stop** to hear them),
**Positioning** (everywhere in the area, or from a place, with its
distances), **Advanced** (active, play style: once, repeating or seamlessly
looping; random or in order; interval and its variation; times of day;
pitch and volume variation), Comments.

### Waypoints

Basic (name, tag, appearance, map note), Advanced, Description, Comments.

Right-click a blueprint in a palette and choose **Find References** to see
where the module places it or names it.

## Instances

An object placed in an area has the same pages as its blueprint (open
them with a double click, or right-click › **Properties**). Changing an
instance does not change its blueprint, nor other instances; **Add to
Palette** turns an instance into a new custom blueprint, and **Update
Instances** on a blueprint replaces its instances with it again.
