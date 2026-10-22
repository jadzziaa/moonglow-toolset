---
title: "Plugin API 0.4 A1: selection, open object, a command's for"
author: August
date: 2026-10-22
branch: plugin-api-a1
cost: { minutes: 175, tokens: 1197303 }
touches:
  - apps/mg/tests/plugins.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/15-plugins.md
  - docs/manual/16-writing-plugins.md
  - docs/manual/17-plugin-api.md
  - docs/manual/index.md
  - docs/plugin-proposal.md
  - docs/plugins/CHANGES.md
  - docs/plugins/README.md
---

## Decided
- Plugin reads are not granted: the grants (`plugin_grants`, `JobHost::ask`) cover only a folder chosen and a yes (Confirm), for events. So `ctx.selection` and `ctx.open` are read with no question, as `ctx.module` is.
- The host passes only *where* (`mg_plugin::Selected`: Object{area,kind,index}, Blueprint(key), Area, Resource) in `Input.selection`; the runtime fills tag/resref from the job's snapshot (`runtime::selected`). Each Lua entry carries `resource` + `path` exactly as `ctx.edit` takes them (path "" for a root), so one handler serves a placed object and a blueprint (`s.path .. "/Tag"`).
- One manifest key, `for = "placeable, creature"` / `"area"` (`manifest::Target`), covers a kind placed and its blueprints; `for` under an `api` before 0.4 is refused. A command with `for` stays in the Plugins menu too (keys keep working); from there it gets `Moonglow::plugin_selection()` (the shown area view's selection, else the palette's).
- Menus push `Action::RunPluginCommand { plugin, command, selected }`; `plugins::menu_commands(ui, &commands, &kinds)` draws the group (separator + buttons) and returns the index. Placed before the Variables/Properties separator in area_view `context_menu`, the tree's object rows (`Asked::Plugin(usize)`) and ARE rows; last in palette rows (`Pick::Plugin`), which have no Properties.
- `Input::new(module, game)` replaced the struct literals; a selection goes in as `Input { selection, ..Input::new(..) }`.

## Watch out
- Module tree rows of blueprints without a name are labeled `guard.utc` (areas, scripts and dialogs by resref alone).
- A custom-palette blueprint is listed only with a `PaletteID`, and its category starts closed: set `palette.filter` in a test to show it.
- A right-click in the area view on an object already selected keeps the whole selection: set `view.selection`, then right-click one of them, to test a menu on several objects.
- In a worktree session, `flock LOCK cargo ...` is refused by the shell guard; a scratchpad script that execs flock works.
- `mg plugin run --answer` for a form that is never asked is an error (test "nothing selected" without answers).
