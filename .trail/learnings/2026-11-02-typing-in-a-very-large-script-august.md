---
title: "Typing in a very large script: lines laid out one at a time"
author: August
date: 2026-11-02
branch: worktree-agent-a8d8f9633fcdce17f
cost: { minutes: 3062, tokens: 20321316 }
touches:
  - crates/mg-ui/src/script_layout.rs
  - crates/mg-ui/src/script_view.rs
  - crates/mg-ui/src/script_tools.rs
  - crates/mg-ui/tests/editor_perf.rs
---

## Decided
- The script editor keeps egui's `TextEdit` and hands it a layouter of its own, `script_layout::LaidOut` (in `Moonglow::laid_out`): a galley per line, kept while the line's fingerprint (its text and its tokens' element/span pieces) and its underline stay the same; a change lexes from the first line it touched with `lex_line` (each line from what the line before left open: `Within::{Code, Comment, RawString}`) until a line after the change begins in the same state as the old line at that place (then the rest are reused, starts shifted by the length delta); the whole galley is `Galley::concat` of the lines' (its job: the full text, one section, so `galley.text()` and `is_empty()` work). The gutter's numbers are cached per line the same way.
- Measured before choosing (eu-stack sampling): the 13 ms was whole-text highlighting, the job's hash, egui's paragraph split and concat, and `underline_line` cloning every section; at a typist's pace (frames between keys) it was ~100 ms a key.
- The editor's symbols are still read again the frame after a key (6.3 ms on nwscript.nss); a debounce was considered and not done (it would change when the side lists and completion see a new declaration).

## Tried and ruled out
- Relying on egui's own paragraph cache (`GalleyCache` splits a job at `\n`): it keeps entries only while they are asked for every frame (`flush_cache`), so the old `LaidOut` cache (which skipped `layout_job` on unchanged frames) made every key after an idle frame a full relayout.
- Whole-text tokenize + per-line fingerprints on every change: correct but ~1.5 ms of O(n) per key; per-line lexing from a stored state took a key to 1.8 ms.

## Watch out
- Kept galleys go stale when egui makes its fonts again (atlas > 80% full or text options changed): `script_layout::watch_fonts` (called once a frame in `Moonglow::ui`) lays out a tiny probe and counts when its `Arc` changes; `LaidOut::begin` drops everything then, and on palette, error color, weak color or pixels-per-point changes.
- Whole-text vs per-line layout must match row for row: an empty line needs a section (its font's height), but an empty text has none (as the whole layout gave); the tests compare rows' pos, size, glyphs and mesh vertices.
- `Galley`'s `intrinsic_size` is `pub(crate)`: a galley can't be built field by field outside epaint; use `Galley::concat`.
- egui's `ImeEvent::Enabled`/`Disabled` are deprecated (unused by egui); compose with `Preedit` then `Commit`.
- In kittest the script undo key is `Modifiers::COMMAND` + Z (the app's keymap), as the other script undo tests use.
- The development machine is shared with other sessions' builds: perf medians move by a tenth to a third; compare before/after runs taken back to back (a baseline tree via `git archive <rev> | tar -x` into `target/` and its own `CARGO_TARGET_DIR`).
