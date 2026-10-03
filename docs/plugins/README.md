# Plugins: for authors

What a plugin author needs besides the manual:

- [`examples/`](examples): plugins that work, each a folder to copy into
  Moonglow's plugins folder. Moonglow's tests run them
  (`crates/mg-plugin/tests/examples.rs`), so they match the Moonglow they
  come with.
  - [`hello`](examples/hello): the plugin of the manual's "Writing
    plugins". A command that reads, and one that asks and edits.
  - [`tag-conventions`](examples/tag-conventions): a team's rule as a
    check for Verify Module and a command that applies it, sharing the
    rule from a second file.
  - [`merchant-markup`](examples/merchant-markup): a batch edit with a
    form and progress.
  - [`creature-report`](examples/creature-report): reading the game's
    data (a 2DA and the talk table).
- [`types/moonglow.luau`](types/moonglow.luau): the API's types, for
  editors with the Luau language server (`.luaurc` here points the
  examples at it).
- [`CHANGES.md`](CHANGES.md): what changed in each version of the plugin
  API, and what a plugin has to change.

The manual has the rest: [Plugins](../manual/15-plugins.md) (for the
people who use them), [Writing plugins](../manual/16-writing-plugins.md)
and the [Plugin API reference](../manual/17-plugin-api.md). The design,
and what is planned beyond API 0.1, is in the
[proposal](../plugin-proposal.md).

To check a plugin and try a command without the window:

```sh
mg plugin check docs/plugins/examples
mg plugin run mymodule.mod docs/plugins/examples/tag-conventions fix-tags --dry-run
```
