---
type: Guide
title: 'Plugins: for authors'
description: Starting point for plugin authors - the example plugins, the editor type file, the API changes, where the manual covers plugins, and the mg plugin commands for checking, testing, running and packing one.
tags: [plugins, authors, examples]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-09T00:38:51Z }
---

# Plugins: for authors

Plugins are experimental: the plugin API is at 0.9, and a later release
may change it ([`CHANGES.md`](CHANGES.md) lists each change).

What a plugin author needs besides the manual:

- [`examples/`](examples): plugins that work, each a folder to copy into
  Moonglow's plugins folder. Moonglow's tests run them
  (`crates/mg-plugin/tests/examples.rs`), so they match the Moonglow they
  come with.
  - [`hello`](examples/hello): the plugin of the manual's "Writing
    plugins". A command that reads, and one that asks and edits.
  - [`tag-conventions`](examples/tag-conventions): a team's rule as a
    check for Verify Module, with a fix for each finding (API 0.4), and
    a command that applies it, sharing the rule from a second file; and
    tests of its own (`tests/tags.toml` on a fixture module, and
    `tests/builtin.toml`, Moonglow's own `tag-unique` on a small
    project, run by `mg plugin test`).
  - [`builder-tags`](examples/builder-tags): settings (API 0.4): a
    team's prefix kept in the project's `moonglow.toml`, the builder's
    code in their own part of the project, and one setting of the
    user's, read by a check (with a fix for each finding) and two
    commands, one of which sets the builder's code; tests of its own on
    a fixture project whose `moonglow.toml` gives the prefix.
  - [`house-rules`](examples/house-rules): events (API 0.4). Before a
    save, each area it would write is held to a team's rules (a changed
    tag refused, one not upper case warned of, each with a fix); a
    pasted waypoint gets a tag of its own, and a new area the team's
    land, as part of the paste or the making; tests of its own on a
    fixture project and the same project as it was before a save.
  - [`store-markup`](examples/store-markup): a batch edit with a
    form and progress.
  - [`set-local`](examples/set-local): commands in the right-click
    menus of objects and areas (API 0.4), run on what was clicked: a
    local variable set on the objects or blueprints selected, and an
    area's objects counted.
  - [`pick-tint`](examples/pick-tint): a color field (API 0.8). A
    placed placeable's right-click command that asks for its tint
    (The Domains of Dread's `tdod_tint`), drawn on the placeables while
    it is picked, and sets it.
  - [`tint-panel`](examples/tint-panel): a panel's fields (API 0.9).
    The same tint picked in a pane that stays open and follows the
    selection: a color field showing the selected placeable's
    `tdod_tint`, drawn on it while it is picked and set by the field's
    command when the picker closes (`ctx.values`), and a choice of named
    tints, each with its swatch; tests of its own.
  - [`creature-report`](examples/creature-report): reading the game's
    data (a 2DA and the talk table).
  - [`creature-sheet`](examples/creature-sheet): an exporter and an
    importer (API 0.6). File › Export writes the module's creature
    blueprints (or those selected) to a CSV file, File › Import reads one
    back, setting each creature's tag and name; tests of its own.
  - [`store-wizard`](examples/store-wizard): a wizard (API 0.6). A store
    made page by page, the pages after the first made from its answers
    (or skipped), a page's check keeping it until it is right; tests of
    its own, answering its pages.
  - [`module-overview`](examples/module-overview): a panel (API 0.6). A
    pane of what the module has and of what is selected, its areas to
    open, a button running the plugin's command; tests of its own of what
    it shows.
  - [`tileset-import`](examples/tileset-import): outside the module. A
    folder the user chooses (a tileset's files, as NWN Mapper exports
    them) goes into a hak, and an area is made with one of the
    tileset's groups placed.
- [`types/moonglow.luau`](types/moonglow.luau): the API's types, for
  editors with the Luau language server (`.luaurc` here points the
  examples at it).
- [`CHANGES.md`](CHANGES.md): what changed in each version of the plugin
  API, and what a plugin has to change.

The manual has the rest: [Plugins](../manual/15-plugins.md) (for the
people who use them), [Writing plugins](../manual/16-writing-plugins.md)
and the [Plugin API reference](../manual/17-plugin-api.md). The design
and its reasons are in the [proposal](../plugin-proposal.md); what is
planned beyond API 0.9 is in the [deferred list](../deferred.md)
(Automation).

To check a plugin, run its own tests, try a command without the window,
and pack a plugin into the archive that Plugins › Install Plugin from
File… takes:

```sh
mg plugin check docs/plugins/examples
mg plugin test docs/plugins/examples/tag-conventions
mg plugin test docs/plugins/examples/builder-tags
mg plugin test docs/plugins/examples/house-rules
mg plugin run mymodule.mod docs/plugins/examples/tag-conventions fix-tags --dry-run
mg plugin export mymodule.mod docs/plugins/examples/creature-sheet creatures-csv creatures.csv
mg plugin wizard mymodule.mod docs/plugins/examples/store-wizard new-store --answer name=Smithy
mg plugin pack docs/plugins/examples/tag-conventions
```
