---
title: "Script editor: module save note, log times, block comments, indentation, scratch rule"
author: August
date: 2026-10-26
branch: script-editor-rest
cost: { minutes: 3, tokens: 34928 }
touches:
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/03-modules.md
  - docs/manual/07-scripts.md
  - docs/manual/10-options.md
---

## Decided
- Log times: `Log.times` is a Vec kept beside `entries` (same index), filled by `Log::said` with `script_tools::clock()`; `Log::shown(i)` is the line as drawn (`HH:MM:SS msg`). `entries` stayed `(Level, String)` because ~130 test sites destructure it. Every write must go through `said` (Verify's plugin lines were `entries.extend`ed; now looped through `said`), and `clear` clears both.
- "Saved, not compiled" is `ScriptTools::saved_not_compiled(key)`, called by a script's Save and by `Moonglow::save` for each key `store_script_text` returned, when `auto_compile` is off.
- Block comments: `outline::doc_before` and `spec::parse` take `BlockComment` tokens as `LineComment`s (adjacent, no empty line). `comment_text` returns String: a block loses `/*`, `*/`, each line's leading `*`s and spaces, and its empty first/last lines. `doc_source` keeps the block as written, `\r` removed. BioWare's includes put `/* … */` between `//::` lines above the definition, while the prototype is documented by `//` lines or not at all, so `script_symbols` now gives a function the documentation of whichever declaration has some.
- Indentation: `Indent::guess(text) -> Option<Guess>` (Tabs or Spaces(n): majority of indented lines; n = the most frequent positive difference between consecutive space-indented lines, ties to the narrower, else the least indentation; lines starting `*` skipped). Cached on `ScriptBuffer.indent`, set in `read()` and `follow()` and by a conversion; `Indent::of_script(guess, style)` falls back to Options.
- Convert Indentation: `Id::IndentToTabs`/`IndentToSpaces` (Edit menu) and the editor's context menu set `script_tools.convert_indent`; the editor applies it when drawn. "The script in front" (`commands::script_in_front`): `front_tab()` if a Script, else `script_tools.last_typed` (set whenever an editor has focus), else the only open one. Script editors usually open in a window of their own (another dock surface), where `front_tab()` is not the script.
- Scratch rule: `transfer::remove_stale_compiled(dir, resrefs)` removes `name.ncs` and `name.ndb`. To Scratch calls it when not compiled; `run_export_files` with `scratch` drops a broken script's NCS/NDB from what is written and removes the folder's for broken and uncompiled scripts. Export as Files to a chosen folder is unchanged.

## Tried and ruled out
- Adding both the before and after states to egui's `TextEditUndoer`: the TextEdit normalises the cursor (`prefer_next_row: true`), so the stored after-state never equals the live one and Ctrl+Z only moved the cursor. Only the before-state is added (`add_undo`); the field records the after-state itself, and one Ctrl+Z returns the old text either way.
- `request_focus` from the conversion: a menu click takes focus away again that frame; tests re-focus the editor with `script_tools.jump`.

## Watch out
- `cargo test` stops after the first failing test binary: use `--no-fail-fast` to see every binary's result.
- A log line found by exact label now has its time in front: use `get_all_by_label_contains`, or clear the log before looking a Compiler tab line up by `"{at} {text}"` (the log's line is the same text).
- In screens tests with game data, a custom function in the side list needs `script_tools.filter` set or it is out of sight (kittest only finds what is drawn).
