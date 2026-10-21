---
title: "Script editor line ends, indentation, Help as written"
author: August
date: 2026-10-21
branch: script-editor-feedback
cost: { minutes: 161, tokens: 869374 }
touches:
  - docs/deferred.md
  - docs/log.md
  - docs/manual/07-scripts.md
  - docs/manual/10-options.md
  - crates/mg-script/src/analysis.rs
  - crates/mg-script/src/outline.rs
  - crates/mg-script/src/spec.rs
  - crates/mg-ui/src/script_nav.rs
  - crates/mg-ui/src/script_tools.rs
  - crates/mg-ui/src/script_view.rs
---

## Decided
- The script editor's buffer (`ScriptBuffer.text`, `saved`) is LF text; `raw` keeps the file's text with its own line ends. `bytes()` writes `raw` when untouched (byte for byte), else `text::with_line_ends(text, raw)`: lines before the first and after the last change keep their ends, the rest take the file's majority (CRLF when there are none). Save through `mark_saved()`, never `saved = text` then `bytes()` (that order wrote the old raw).
- `script_source` returns LF text too, so every position (index, references, jumps, Rename Symbol's spans) is in the editor's form; `rename_symbol` applies spans to `to_editor(raw)` and writes back with `with_line_ends`.
- Indentation (`indent_tabs`, `indent_width`) and the editor background live in `ScriptStyle` (struct-level `#[serde(default)]`: old settings load the `Default` impl's values, so a plain `u8` default of 4 works); the live check switch is `Settings::no_live_check`.
- Help shows `doc_source` (comment lines as written, from `outline::comment_as_written`) beside the stripped `doc` that `mg lsp` hover keeps. `tools::Symbol.doc` now holds the source form (only Help reads it).
- Local time of day comes from `jiff` (workspace dependency): `unsafe_code = "deny"` rules out libc's localtime, and std has no time zone.

## Tried and ruled out
- Underlining an error line by flagging whole `LayoutJob` sections: `LayoutJob::append` merges adjacent tokens of the same format, so one section spans several lines and the lines around the error were underlined. `underline_line` now cuts sections at the line's first and last non-space byte.
- Reserving the error line even with the live check off: the option off means no line at all (the test asserts the text moves up).

## Watch out
- egui's multiline TextEdit inserts `\t` on Tab itself; consume `Key::Tab` before `TextEdit::show` (after the completion list, which takes Tab first).
- Paste arrives as `Event::Paste`; rewrite it in `ui.input_mut` before the TextEdit runs, or the cursor lands wrong.
- In a worktree session, shell heredocs, `flock CMD` and sed on a `$var` path are refused by the guard: write scripts to the scratchpad (a `gpu.sh` wrapper for the shared GPU lock) and use Edit.
- kittest: a Label's text is in `accesskit_node().value()`, not `label()`.
