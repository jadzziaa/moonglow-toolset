---
type: Decision
title: A .mod with its module folder beside it
tags: []
status: stable
generated: { by: trail, at: 2026-10-26T00:00:00Z }
touches: [docs/deferred.md, docs/findings.md, docs/log.md, docs/manual/03-modules.md, docs/manual/10-options.md, docs/manual/11-command-line.md, docs/manual/12-differences.md, docs/manual/14-coming-from-aurora.md, docs/parity/checklists.md, apps/moonglow/src/main.rs]
fingerprint: v2-e0ad99db56fd2f1a
created: 2026-10-26
author: August
description: "When a .mod has a same-named module folder beside it, Action::OpenModule and Open Module… ask which to open (like Aurora), while Moonglow::open_module stays a direct opener; Settings::module_folders controls the behavior."
asks:
  - "Why does opening a module now prompt me to choose between a .mod file and a folder?"
  - "How do I make the editor always open the module folder, or always the .mod, without asking?"
  - "Why doesn't Moonglow::open_module ask which to open, and where was the question placed instead?"
  - "Does nwserver load a module folder or the .mod when both name.mod and name/ exist?"
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
