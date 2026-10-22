---
type: Guide
title: 'Plugins: for authors'
description: Starting point for plugin authors - the example plugins, the editor type file, the API changes, where the manual covers plugins, and the mg plugin commands for checking, testing, running and packing one.
tags: [plugins, authors, examples]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-22T09:14:30Z }
---

# Plugins: for authors

Plugins are experimental: the plugin API is at 0.4, and a later release
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
    tests of its own (`tests/tags.toml` on a fixture module, run by `mg
    plugin test`).
  - [`store-markup`](examples/store-markup): a batch edit with a
    form and progress.
  - [`set-local`](examples/set-local): commands in the right-click
    menus of objects and areas (API 0.4), run on what was clicked: a
    local variable set on the objects or blueprints selected, and an
    area's objects counted.
  - [`creature-report`](examples/creature-report): reading the game's
    data (a 2DA and the talk table).
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
planned beyond API 0.4 is in the [deferred list](../deferred.md)
(Automation).

To check a plugin, run its own tests, try a command without the window,
and pack a plugin into the archive that Plugins › Install Plugin from
File… takes:

```sh
mg plugin check docs/plugins/examples
mg plugin test docs/plugins/examples/tag-conventions
mg plugin run mymodule.mod docs/plugins/examples/tag-conventions fix-tags --dry-run
mg plugin pack docs/plugins/examples/tag-conventions
```
