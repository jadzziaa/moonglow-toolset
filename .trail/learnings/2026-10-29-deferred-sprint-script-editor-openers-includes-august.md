---
title: "Deferred sprint: script editor openers, includes, codepage, To Scratch, indentation, running game"
author: August
date: 2026-10-29
branch: worktree-agent-afa8e88166ff6e009
cost: { minutes: 1661, tokens: 13268025 }
touches:
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/test_module.rs
  - crates/mg-ui/tests/app/modules.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/09-build-and-test.md
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/script_tools.rs
  - crates/mg-ui/src/script_view.rs
  - crates/mg-ui/tests/app/scripts.rs
---

## Decided
- A compile reads the script compiled as given (its editor's text) and every other script as the module has it, then the game: `script_view::sources` is the one resolver, for Compile, the check as you type and the wizards. Not the editors' unsaved text: the `.ncs` goes into the module beside the sources the build, an export and the test loop compile again from. `compile_one` says when an include open in an editor had unsaved changes (`compile_reading` returns the module scripts read).
- Scripts opened by Open Script, New Script, Save As, the Script Wizard and the NUI Creator go through `Action::OpenResource` (Options' external editor), not `OpenTab(Tab::Script)`. Jumps to a line (compiler messages, Definition, References) stay `OpenTab`. `open_only_external` declines a script whose file differs from the module's bytes (New Script and Save As in a folder or project have no file until File › Save).
- Script lines shown (`rename::mentions`) take a codepage: the toolset's `script_codepage()`, `mg`'s `scripts_codepage` (a module's `encoding.2da` table, else the client's settings.tml as `mg lsp` reads, else 1252).
- Other running game clients: `test_module::running(names)` (read only: `/proc` exe or comm, macOS `ps -ax -o pid=,ucomm=,args=`, Windows `tasklist /FO CSV /NH`); Moonglow's own are known by pid (`GAMES`). Same module on the command line stops the test; otherwise a warning.

## Watch out
- egui's TextEdit takes any pointer button's press as a click that moves the cursor (`text_cursor_state.rs`, `pointer.any_pressed()`): a right click lost the selection before the context menu opened. The script editor now restores a non-empty selection after `show` when the secondary button was pressed over it.
- A selection left in `TextEditState` stays when the editor loses focus but isn't painted (`has_focus` gates the selection). Edit › Convert Indentation uses the selection only when `app.undo_script == Some(key)` (the editor had the keys as the menu opened).
- macOS `ps` command lines are space-joined: a module name with spaces can't be told from separate arguments; `Running::plays` takes an exact argument pair first, else the name ending the line or followed by ` +`/` -`.
- A stand-in "client" for UI tests: copy `sh` as `nwmain-linux` with `-c 'sleep 30; true'` (two commands so the shell stays and its cmdline keeps the arguments), and `true` as the install's client; never the real client.
