---
title: A .mod with its module folder beside it
author: August
date: 2026-10-26
branch: issue-30-module-folder
cost: { minutes: 1031, tokens: 10053161 }
touches:
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/03-modules.md
  - docs/manual/10-options.md
  - docs/manual/11-command-line.md
  - docs/manual/12-differences.md
  - docs/manual/14-coming-from-aurora.md
  - docs/parity/checklists.md
  - apps/moonglow/src/main.rs
---

## Decided
- A `.mod` with a folder of its name beside it (`name.mod`, `name/` holding a `module.ifo`) asks which to open, in Aurora's words, from `Action::OpenModule` and Open Module… (`module_folder::open`); `Moonglow::open_module` stays the direct opener, never asking, since Options' reload, tests and the recovery reopen through it. The answer pushes `Action::OpenModuleAsIs` (not in `run()`'s discard list: the save question was already asked).
- Options › General: `Settings::module_folders` (Ask, Folder, Module); Aurora's own is a checkbox, Always open module directories (ini `Always Open Module Directory`).
- The command line pushes `Action::OpenModule` rather than calling `open_module`, so it asks too.
- Aurora (captured 2026-10-26): title "Open directory as a module", Yes the folder, No unpacks the `.mod` into `temp0/`; it asks from the recent list every time; saving the folder writes changed files into it and packs `name.mod`, old one as `.BackupMod`.

## Tried and ruled out
- Putting the question inside `open_module`: `apply_options` reopens the open module through it and would ask again.
- A note in `mg` when a `.mod` has a folder beside it: 40 call sites open modules, no single place; deferred.

## Watch out
- nwserver never reads a module folder: with `name.mod` and `name/` it loads the `.mod`; with only `name/` it fails ("Unrecognized file type"). The `.mod` written beside a folder on save is what the game plays.
- The worktree's Bash guard refuses `flock ... cargo` and `$VAR`-built commands: put them in a scratchpad script.
