---
type: Manual Page
title: Blueprints
description: Blueprints - palettes, the blueprint editors and wizards, and instances.
tags: [manual, blueprints, palettes]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-07T21:30:00Z }
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

A right click on a blueprint, on a category or in the room under the
list offers **New Creature…** (or the kind shown): the wizard New… opens.
A filter typed opens every category with a match; they can be closed
again while it stands. Home and End go to the list's top and bottom.

**Categories…** (beside New…) edits the categories blueprints of the type
shown go in: **Add Category**, **Add Group** (a branch that holds
categories), **Rename** and **Remove**, in the group chosen or at the
top. The game's categories are the starting point; with a change the
module gets a list of its own, kept in the module (`placeablepal.itp` for
placeables, and so on: the palette skeleton that content packs ship in
their haks), and the blueprints' Category lists and the custom palette
follow it.
- **Drag a row** to move it: onto another's upper half to go before it,
  onto its lower half to go after it, or into it if it is a group; held
  near the list's top or bottom, it scrolls the list, as the wheel does
  while you hold it. The
  module's own categories show in the order you give them, in the
  palette and in the blueprints' Category lists (the game's show by
  name, as in Aurora).
- A renamed category keeps its blueprints; one with blueprints of the
  module in it isn't removed until they are given another.
- **Use the Game's Categories** drops the module's list.
- Aurora reads custom categories from palette files kept in a module too
  (a builder who keeps them so reports it; Moonglow's own check against
  Aurora is still to do), so the module's categories should show there
  as they are. Whether Aurora shows a category that has a name written
  out rather than a talk-table string was not tried.

Choose a blueprint to place it in an area, or drag it there. Hover over
one to see its resref, tag and, for creatures, doors, items and
placeables, a picture; for the others, what there is to say of them (a
sound's sounds, a trigger's kind, an encounter's creatures, a store's
prices, a waypoint's map note).

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
- **List** and **Gallery** (beside Categories…): Gallery shows the blueprints as pictures in a grid rather than a
  list of names, to choose by eye (creatures, doors, items and
  placeables, the types with a model, and waypoints by their flags).
  Whether it is on is kept between sessions. A picture is clicked, dragged and
  right-clicked as a name is; the pictures are made a few at a time as
  they come into sight.
- **Replace Selected with This** (a blueprint's right-click menu): the
  objects of that type selected in the area become ones of this
  blueprint, where they stand and facing as they face. One Undo puts the
  old ones back. With the Gallery, this swaps a placed object for a
  neighbouring one in a click or two.
- **The Appearance Gallery** (**Tools › Appearance Gallery…**, or **All
  Appearances…** in the palette's Placeables): **Placeables**, **Creatures**
  and **Doors** at its top choose whose. Every appearance in
  `placeables.2da`, `appearance.2da` (a plain body of each) or
  `genericdoors.2da` as a picture, those that are only an effect (flames,
  sparks, shafts of light) too. It is a tab like any other: it opens in a
  window of its own, and can be docked beside the area or maximized. Click
  a picture to give it to the objects of that kind selected in the area
  shown; the tab stays open to try another. Drag a picture into the area's view to
  place a placeable of that appearance there (it shows see-through where
  it would go): a plain, static one named
  for the appearance, of no blueprint. **Find** narrows it by name, and the
  slider sets how large the pictures are at least: they grow to fill the
  tab's width (the palette's Gallery has the same slider, and the size is
  kept). A model the palettes don't offer can be chosen here.
- **Gallery…** beside the appearance in a placeable's, creature's or
  door's Properties (a blueprint's, or a placed one's) opens the gallery
  at that object's
  own appearance, so its neighbours in the table are beside it; a click
  gives it to that placeable (**Use the Selection** goes back to the
  area's selection).
- **View**: open one of the game's blueprints to look at, page by page
  as its editor shows it (or double-click it). It can't be changed
  there: its fields are dimmed, its pages and lists can still be looked
  through, and a change tried is refused with a note that offers **Edit
  Copy…**.
- **Export…** (a custom blueprint, or the several selected): the Export
  window with them chosen, as File › Export.
- **Lists of choices** (an appearance, a race, a part) take the arrow
  keys once clicked or reached with Tab: Up and Down choose the one
  before and after, Page Up and Page Down ten away, Home and End the
  first and last. A creature's and a placeable's Appearance list shows
  each appearance's picture beside the row the pointer rests on, and the
  chosen one's with the pointer resting on the list's box: leave it there
  and the picture follows the arrow keys.
- **Edit Copy…**: copy any blueprint, standard or custom, into the module
  as a new custom one, and open it. It asks for the copy's ResRef (a free
  one is offered) and Tag first.
- **Delete**: remove a custom blueprint from the module.
- **The model in the page:** a placeable's and a door's Basic page and
  an item's Appearance page show the model beside their fields, in a
  window wide enough for both (a creature's Appearance page always has
  it). A change shows as you make it.
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
category and a name, then opens the new blueprint's properties. The Name
page shows the ResRef and Tag the blueprint gets, made from the name as
Aurora makes them: type others there to choose your own (not the Creature
Wizard, yet).

## The editors

A name or description whose text is the talk table's (a StrRef, as the
game's blueprints have) shows in blue with its StrRef beside it; typing
there gives the blueprint text of its own.

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
- **Variables…**: local variables the object starts with, in a window
  whose edges are dragged to size it. **Save Set…**
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
- **Feats** shows the feats the creature has beside the list to choose
  from, under **Assigned**; × takes one away.
- **Feats** and **Spells** are found by name (**Find**) and by category:
  the toolset's categories of feats (combat, defensive, magical…), and
  spells' talent categories (harmful ranged, beneficial healing…).
- **Descriptions:** rest the pointer on a skill, a feat, a spell or a
  special ability to read what the game says of it (Aurora's F1).
- **Special Abilities** lists each ability as the game has it: so many
  **Uses** of a spell at a **Caster Level**. Clicking a spell adds a use;
  the same spell at another caster level is another ability. **Flags**
  are the ability's own, named as the file format names them: Ready,
  Spontaneous, Unlimited. The game counts a use with any of them set as
  one the creature has, and a use with none as spent; Unlimited does not
  make the uses unlimited.

The game reads a familiar only when one of the creature's classes has
one (an arcane class whose `MinAssociateLevel` in `classes.2da` isn't
255: Wizard, Sorcerer), and an animal companion only for a divine class
(Druid, Ranger, at any level). The Classes page says when it won't.
A domain or school left **Not set** is the game's to choose.

- On the **Spells** page, a warning names a class's first problem:
  spells of a level the class cannot cast yet, a casting ability too low
  for the spell level, or more spells than the class allows.
- In the **Inventory**, equip items from the item palette into the
  equipment slots, or put them in the backpack. **Equip** under the
  palette puts the chosen item in the first free slot it goes in (a
  ring in the first free ring slot), and says so in the log when every
  such slot is taken; the Equip beside a slot puts it in that one. With
  an item selected, **Equip** takes it from the backpack to a free slot,
  **To Backpack** unequips it into the backpack, and **Properties**
  opens the item's properties: for a placed object, the item as that
  object holds it, to change there as in Aurora; for a blueprint, the
  item's blueprint (the module's to edit, the game's to look at). Items are
  dragged, too: from the palette onto a slot (equipped there, if it goes
  in it) or onto the backpack (added); from the backpack onto a slot;
  from a slot onto the backpack, or onto another slot it goes in. A
  right click on an item anywhere offers the same (**Equip**, **To
  Backpack**, **Properties**, **Remove**), and on the palette's
  items **Add to Backpack** and **Equip**. A placeable's and a store's
  items take an item dragged from the palette as well.
- **Between inventories, and within one:** drag an item from one object's
  inventory (a creature's backpack or equipment, a placeable's contents,
  a store's page) onto another's list or a creature's slot to move it
  there; hold Ctrl as you let go to copy it instead. Drag a row onto
  another row of its own list to move it to that place. **Copy** (a
  row's right-click menu, or Ctrl+C with the pointer over it) and
  **Paste** (the menu, **Paste (n)** in the list's heading, or Ctrl+V
  over the list) carry an item from any inventory to any other, in any
  module open after it. Equipping an item the
  creature lacks the feat for (a weapon proficiency, an armor weight)
  asks whether to give it the feat. Items can be marked droppable,
  pickpocketable and infinite (merchants).
- The **Creature Wizard** makes a creature from a race, classes and levels,
  gender, appearance, portrait, faction and name, leveled as the game's
  packages level it. Its appearances are pictures, narrowed by **Find**;
  **All portraits** offers every creature's portrait, not the race's
  alone; and its Name page takes a ResRef and Tag of your own.
- **Portraits…** chooses the portrait from the game's, by race and
  gender.

### Items

**General** (name, base item, cost, charges, stack size, plot, stolen,
cursed, identified; the base item's statistics), **Appearance** (by the
base item: a model, three weapon parts, or the armor's parts and colors,
with the inventory icon as the game shows it; a model and each of a
weapon's three parts are also chosen by picture, the item's icon as it
would be with each; the item's model shows in the page and follows each
change, beside the fields in a wide window and beside the icon in a
narrower one; an armor or a cloak is shown on a man or, with **Shown on
› Female**, on a woman, and its inventory icon is then the one a woman's
inventory shows), **Properties** (the item
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
