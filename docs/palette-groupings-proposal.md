---
type: Proposal
title: 'Palettes grouped more than one way: a design'
description: How the palette pane could show the same blueprints grouped by something other than their palette category (TDoD's placeables by theme, and also by what each one is - tables, seats, food, containers) without copying a blueprint or listing it twice in a palette file - what holds today (one PaletteID per blueprint, standard palettes written by a builder's tool), three ways to get it (groupings of Moonglow's own from rules in the project file; a second branch in a standard palette; names and the search), measurements on TDoD's 13,212 placeables, a recommendation, milestones and the owner's questions.
tags: [proposal, palettes, blueprints, project-file, tdod]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-03T00:28:49Z }
sources:
  - id: owner
    resource: human:august
    title: 'The owner''s ask, 2026-11-02: TDoD''s custom palette sorted by theme (City, Street, Ruin...) and also by what a placeable literally is (Table, Chairs, Containers, Food...), in the same palette, avoiding duplication'
  - id: code
    resource: crates/mg-module/src/palette.rs (Places, updated, rebuild_custom_palette), crates/mg-ui/src/palette_view.rs (rows keyed by their path, the search) (read 2026-11-02, at 3.4.0)
  - id: manual
    resource: manual/05-blueprints.md
    title: 'Blueprints: "Custom", "The custom palettes change only where you change a blueprint", "Categories..."'
  - id: tdod-pal
    resource: ~/Projects/tdod-module/build/hak/tdod_blueprints.hak (placeablepalstd.itp), tdod_top.hak (placeables.2da)
    title: TDoD's standard placeable palette and appearance table, as built on 2026-11-02
  - id: tdod-build
    resource: ~/Projects/tdod-modulework/docs/assets/library-build.md
    title: 'TDoD''s library build: themes.toml, the palettes it writes, labels by rules'
---

# Palettes grouped more than one way: a design

The owner wants TDoD's placeables sorted in the palette both by theme (City, Street, Ruin,
as now) and by what each placeable is (Table, Chairs, Containers, Food), in one palette, with
no blueprint copied to get it.[^owner] This proposal sets out what holds today, three ways to
get there, and a recommendation. Nothing is built until the owner chooses.

## What holds today

- **A blueprint is in one category.** Its `PaletteID` (one byte) names the category, in the
  game, in Aurora and in Moonglow. The module's custom palette (`placeablepalcus.itp`) is kept
  to it: a save moves an edited blueprint's entry to its category, and Refresh Palette and
  Build Module's Palettes rebuild the file from every blueprint's `PaletteID`.[^code][^manual]
  An extra entry for the same blueprint, written into a custom palette by hand, isn't kept:
  a rebuild drops it, and an edit looks after one entry only.
- **A standard palette is a plain tree.** `placeablepalstd.itp` lists blueprints by ResRef under
  categories, whatever their `PaletteID` says, and whatever tool wrote it. TDoD's is written by
  its library build, in its blueprint hak: a "Domains of Dread" branch with 24 categories from
  `themes.toml`, 14,665 entries, 2.2 MB.[^tdod-pal][^tdod-build]
- **TDoD's categories already mix both axes.** Most are themes (Autumn, Castle, Manor, Tavern,
  Ruin), a few are kinds (Container, Light, Clutter, Nature).[^tdod-pal]
- **What a placeable is shows in its name, not in its appearance.** The names read
  `Theme: Name (source) [triangles]`: "Tavern: Table 2 (N2E)", "Market: Food Stand 02a
  (HFG)". Most of TDoD's `placeables.2da` rows have no label (`****`), so the appearance can't
  say what a model is.[^tdod-pal]
- **The palette's search already finds every word.** `table ruin` lists the tables under Ruin;
  `table` lists them across all themes. It needs a name to say so, and it shows one list, not
  groups.

### Measured: kinds from names

Ten rough word rules over TDoD's 13,212 placeable names (Plants, Walls and fences, Rocks,
Containers, Food, Dishes, Lights, Seats, Tables, Beds), with the name's theme, source and
triangle count left out, measured 2026-11-03:

| | Blueprints |
|---|---:|
| Matched by at least one rule | 5,861 (44%) |
| In two groups ("Food box 2": Containers and Food; "Ivy Wall": Plants and Walls; "Flower Pot": Plants and Dishes) | 223 |
| Matched by none | 7,351 |

Plants 2,039, Walls and fences 759, Rocks 711, Containers 673, Food 507, Dishes 438, Lights
416, Seats 254, Tables 235, Beds 52. Rules carry much of it; a list that names blueprints
directly has to carry the rest. TDoD's library build already labels each asset by rules of its
own, so it could write that list.[^tdod-build]

A first try that matched the whole name put 150 bushes and ferns under Food: "after HAM", the
pack they were made after, matched the rule for "ham". Rules have to match whole words of the
part of a name that says what the object is. With TDoD's `Theme: Name (source)` names, that's
the part after the colon, without the brackets.

## Three ways

### A. Groupings of Moonglow's own (recommended)

The palette files stay as they are: one category per blueprint, which is what the game and the
DM Creator show. Moonglow's palette pane gets a **Group by** choice above the list:
**Category** (as now) or a grouping the project file defines. Grouped by **Kind**, the list
shows Tables, Seats, Food… each with every blueprint the rules put there. A blueprint in two
kinds shows under both (it's a view: nothing is copied). Those no rule takes are under
**Other**. **Then by Category** nests the categories inside each kind (Tables › Tavern, Tables
› Manor), or the other way round.

The rules go in `moonglow.toml`, shared by the team as the project's other rules are:

```toml
[palette.kind]            # a grouping: shown as "Kind" in Group by
title = "Kind"
types = ["utp"]           # the palettes it applies to (any blueprint type)
words_of = '^.*?:\s*(.*?)\s*(\(|$)'   # optional: the part of a name its words come from
from = "palette_kinds.toml"   # optional: a generated file, as below

[[palette.kind.group]]
name = "Tables"
names = ["table", "desk", "counter"]    # whole words of the blueprint's name, case ignored
resrefs = ["dd_*_tbl*"]                  # or ResRef patterns

[[palette.kind.group]]
name = "Food"
names = ["cheese", "bread", "fruit", "vegetable", "meat", "fish", "pie"]
```

`from` names a file the library build writes: `"Tables" = ["dd_n2e_table2", ...]`, the
blueprints its own labels put in each group, added to what the rules find. A blueprint's
Comment could carry it instead (`kind: Table`), but TDoD's comments already hold the source and
licence, and a file is easier to rewrite in one go.

What else it touches:
- The search works within the grouping; the Gallery shows the same groups; each grouping
  remembers which groups are open.
- Placing, previewing, Edit, Edit Copy and the right-click menus work on a row as they do now.
  The blueprint's Category list and its `PaletteID` don't change.
- The module tree is unchanged.
- `mg find --palette-group Kind/Tables` lists a group's blueprints; Verify notes a group no
  blueprint falls in, and a `from` file naming blueprints that don't exist.
- The groups are worked out once when the palettes are read, and again when a blueprint or the
  rules change. 14,665 entries against a few dozen rules is milliseconds, not a cost per frame.

Costs: builders see the kinds in Moonglow only. In Aurora, and in the game's DM Creator, the
palette is the theme tree it is now.

### B. A second branch in TDoD's standard palette

The library build writes a "By kind" branch beside "Domains of Dread", listing every blueprint
a second time under its kind. That's a second entry, not a second blueprint. Aurora and the DM
Creator would show it too, if they cope with a ResRef listed twice: that's unverified, and would
be checked with the Aurora oracle and the client before relying on it. Moonglow keys palette
rows by their place in the tree, so both rows would draw. Selecting one may also mark the other
(to check).

Costs: the file roughly doubles (about 4.5 MB, read by the game for the DM Creator and by every
toolset). It works for the standard palette only: a module's own blueprints, in the custom
palette, keep one category. Every change to the kinds needs a library build.

### C. Names and the search (works today)

Keep the theme categories, and make sure each name says what the placeable is. TDoD's mostly
do. `table` then finds all the tables, `table ruin` those of Ruin. It needs no work, but there
are no groups to browse, and it relies on the names alone.

## Recommendation

**A**, with TDoD's kinds from a few rules in `moonglow.toml` and the rest from a list its
library build writes. It needs no change to any palette file, nothing duplicated, and nothing
in the game. Add **B** only if DMs in the game should browse by kind as well, and only once the
oracles show that Aurora and the DM Creator take a blueprint listed twice.

## Milestones (A)

1. **M1, the groupings** (`mg-module`): reading `[palette.*]` and its `from` files, the match
   (words, ResRef patterns, lists), and a blueprint in several groups or none. Unit tests, plus a
   corpus test over TDoD's hak when `MOONGLOW_HAKS` has it.
2. **M2, the pane**: Group by, Then by Category, Other, search and Gallery within a grouping, its
   open groups remembered. UI tests, and a picture of the pane on TDoD's palette.
3. **M3, the rest**: `mg find --palette-group`, Verify's two notes, the manual (Blueprints, and
   the project file's keys), the plugin API (`ctx.palette`: maybe later, not in M3).

## The owner's answers (2026-11-03)

**A**, built; B and C aren't.[^answers] What the answers settle:

1. **A blueprint shows under every group it falls in**, by a rule or by the list.
2. **Kinds come from rules in `moonglow.toml` and from a generated list** (`from`), together. The
   Comment isn't read.
3. **Other is shown**, last, with the blueprints no group takes.
4. **Nested by default:** grouped by a grouping, each group holds the palette's categories
   (Tables › Tavern); **Then by Category** turned off lists a group's blueprints flat.
5. **Every blueprint type from the start:** creatures, doors, encounters, items, placeables,
   sounds, stores, triggers and waypoints.

### The keys, as settled

```toml
[palette.kind]                 # a grouping, its key the id ("kind"), any number of them
title = "Kind"                 # shown in Group by
types = ["utp", "uti"]         # the palettes it is offered in; left out, every one
words_of = '^.*?:\s*(.*?)\s*(\(|$)'   # optional: the part of a name its words come from
                               # (the first capture group; left out, the whole name)
from = "palette_kinds.toml"    # optional: lists a tool writes, beside moonglow.toml

[[palette.kind.group]]         # in the order shown
name = "Tables"
types = ["utp"]                # optional: only in these of the grouping's palettes
names = ["table", "desk", "counter"]   # whole words, case and accents ignored; "dining table"
                                       # (two words) matches them in that order
resrefs = ["dd_*_tbl*"]        # ResRef patterns, `*` any run, case ignored
```

The `from` file holds groups by name, each a list of ResRefs:

```toml
# palette_kinds.toml: written by TDoD's library build; edit the rules, not this file
"Tables" = ["dd_n2e_table2", "dd_vil_tbl01"]   # a ResRef: that blueprint in every palette
                                              # the group is offered in
"Food" = ["dd_din_fruitpapa.utp"]             # with its type: that one only
```

A group the file names that `moonglow.toml` doesn't is a group too, after those
`moonglow.toml` names, in the file's order. Blueprints are matched by name in the language the
palette shows them in. A name from the talk table is matched as shown. A missing `from` file,
or a bad key, is said in the log and in Verify, and the grouping works without it.

## Questions for the owner (answered above)

1. **A, B or both?** The recommendation is A, and B only for the DM Creator.
2. **A blueprint in several groups:** under each one (the default here), or only under the first
   rule it matches?
3. **Where kinds come from:** name words and ResRef patterns in `moonglow.toml`, plus a list the
   library build writes? Or the blueprint's Comment instead of the list?
4. **The groups no rule fills:** shown as **Other** (the default here), or hidden?
5. **Nesting:** Kind › Category by default when grouped by kind, or flat?
6. **Other blueprint types** (items, creatures): the same mechanism for all from the start, or
   placeables first?

[^owner]: The owner, 2026-11-02, in the session that wrote this proposal.
[^answers]: The owner, 2026-11-03, in the same session.
[^code]: `crates/mg-module/src/palette.rs`: `updated` looks after a changed blueprint's first entry; `rebuild_custom_palette` writes one entry per blueprint, by `PaletteID`.
[^manual]: [Blueprints](manual/05-blueprints.md), "The custom palettes change only where you change a blueprint" and Refresh Palette.
[^tdod-pal]: `placeablepalstd.itp` and `placeables.2da` in TDoD's built haks, read 2026-11-02.
[^tdod-build]: TDoD's `docs/assets/library-build.md`: `themes.toml`, the palettes `build` writes, `relabel`.
