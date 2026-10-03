# Plugin API changes

A plugin's manifest names the API it was written for (`api = "0.1"`).
Moonglow runs a plugin only if it has that API; while the API is at 0.x,
each version may change what the one before had. This file lists each
version's changes and what a plugin written for the version before has
to change.

## 0.1

The first version.

- A manifest, `plugin.cfg`: `[plugin]`, and a `[command]` or `[check]`
  for each thing added.
- `mg.command` and `mg.check` register handlers; typed values
  (`mg.int`…) and `mg.path`.
- A handler's `ctx`: `ctx.module` and `ctx.game` to read, `ctx.edit` for
  a command's changes (fields, list items, whole resources),
  `ctx.log`, `ctx.progress`, `ctx.ui` (message, confirm, form) and
  `ctx.plugin`.
- `require` for the plugin's own files.
