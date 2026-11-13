---
type: Decision
title: "Script editor line ends, indentation, Help as written"
tags: []
status: stable
generated: { by: trail, at: 2026-10-21T00:00:00Z }
touches: [docs/deferred.md, docs/log.md, docs/manual/07-scripts.md, docs/manual/10-options.md, crates/mg-script/src/analysis.rs, crates/mg-script/src/outline.rs, crates/mg-script/src/spec.rs, crates/mg-ui/src/script_nav.rs, crates/mg-ui/src/script_tools.rs, crates/mg-ui/src/script_view.rs]
fingerprint: v2-5b580d33b80f0851
created: 2026-10-21
author: August
description: "Records decisions for the script editor: LF buffer with line ends restored on save via mark_saved, indentation settings in ScriptStyle, Help showing doc_source, jiff for local time, plus underline, Tab and paste pitfalls."
asks:
  - "How does the script editor keep a file's CRLF line endings when saving, given its buffer is LF text?"
  - "Why did my saved script write the old contents after I set saved = text and called bytes()?"
  - "Tab inserts a literal tab in the egui multiline TextEdit instead of my indent setting; how do I intercept it?"
  - "Why are lines around an error underlined too, and how did underline_line fix the LayoutJob section merging?"
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
