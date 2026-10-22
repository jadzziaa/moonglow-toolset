---
title: "Script editor: Save key, Open Script selection, To Scratch"
author: August
date: 2026-10-22
branch: script-editor-save-open-scratch
cost: { minutes: 198, tokens: 1359694 }
touches:
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/keys.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/script_tools.rs
  - crates/mg-ui/src/script_view.rs
  - crates/mg-ui/src/transfer.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
---

## Decided
- The Save key (`Cmd::Save`, any key it is given) in a script editor whose text has the keyboard (`script_view::with_keys`, the focus rule its Undo and Redo use) is that script's Save: `commands::keys_pressed` turns `Id::Save` into `Action::SaveScript(key)` there. Only the key: File › Save, the toolbar's 💾 and the Command Palette run `Id::run` and save the module. A new `Cmd` on Ctrl+S would conflict with Save (General overlaps every group).
- A script editor's Save, Compile and To Scratch are one function, `script_view::save_script(app, key, ScriptSave)`; a Save with nothing unsaved does nothing (the button is grayed then).
- To Scratch copies the `.nss` whatever the compiler says, the `.ncs`/`.ndb` only if it compiled just now (`Action::ScriptToScratch { key, compiled }`, `Moonglow::script_to_scratch`), never the module's older `.ncs`. `Moonglow::scratch_folder` is the one place the scratch folder is asked for.
- Open Script's selection is by `ResKey` (stays as the name narrows the list), through `widgets::select_in_list` (the hak editor's click rule: Shift from the anchor, Ctrl toggles, Ctrl+Shift adds a range).
- A script's Save writes no file for any kind of module: `Edit::SetResource` only sets the module in memory; File › Save writes the `.mod`, a folder's file or a nasher project's source.

## Watch out
- `consume_key(Modifiers::NONE, k)` also takes Shift+k (`matches_logically` ignores Shift and Alt not asked for): test the Shift variant first.
- kittest gives each queued event a frame of its own (`step_dt`: 1/4 s by default, 1/60 in `area_harness`), so four queued pointer events are never a double click at 1/4 s. Push them into `h.input_mut().events` and run one step (`double_click_now` in tests/app.rs); at 1/60 s, wait a second after any earlier click, or egui counts a triple click.
