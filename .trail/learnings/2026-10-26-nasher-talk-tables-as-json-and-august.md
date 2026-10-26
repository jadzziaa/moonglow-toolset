---
title: nasher talk tables as JSON and the tree following tabs
author: August
date: 2026-10-26
branch: nasher-tlk-and-tree-follows
cost: { minutes: 1085, tokens: 10120909 }
touches:
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/03-modules.md
  - docs/manual/10-options.md
  - docs/manual/11-command-line.md
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-ui/src/settings.rs
---

## Decided
- nwn_tlk's JSON lives in `mg-tlk/src/json.rs` (`to_json`, `from_json`, `JsonStyle { codepage, pretty }`), written from the entries' bytes, not through `talk::Line`: nwn_tlk ignores flags when writing and sets them from content when reading. `Tlk::to_bytes_as_nwn_tlk` points a flagged entry with no text at the running offset (nwn_tlk does; `to_bytes` writes 0). Every shipped dialog.tlk matches byte for byte both ways (`tlk::shipped_talk_tables_as_json_as_nwn_tlk_has_them`).
- A nasher tlk target packs `<file stem>.tlk.json` from its sources (nasher errors on any other name). Several targets of one stem = one table in several languages (`talk_tables` + `in_language`, the game's language first). The codepage is tlkFlags' `--nwn-encoding` (default windows-1252), never the table's language.
- `talk::find_for(module, rm, dirs, name, language)`: haks/module, then the project's JSON (`Source::Json`), then the tlk folders. `Found::tlk()` / `tlk_bytes()` parse either kind; use them instead of `Tlk::read(&found.data)`.
- Tree following: `tree::follow_front_tab` each frame compares the focused leaf's front tab and every leaf's front tab with the last frame's (`tree_front`): a newly focused or newly front tab is revealed. `tree_reveal` is now `(ResKey, bool)`; `tree_marked` is the row drawn in hand. Aurora's tree never moves (only the active area's row turns blue): `settings.tree_stays` is that mode.

## Tried and ruled out
- Following only `dock.focused_leaf()`: OpenTab doesn't move the focus to the area opened, so the area wasn't marked.
- Rust's `{:e}` alone for Nim floats: Rust breaks a tie to the higher digit, Nim to the even one (2.4235000610351562); round to the shortest length with `{:.N$e}`.
- Rounding a sound length to 4 places in f32: nwn_tlk rounds in f64, then stores f32.

## Watch out
- A group opened by a reveal animates over several frames; the reveal is kept (`opening`) until the group is fully open, or the row scrolls to the wrong place. UI tests need ~12 steps after OpenTab.
- egui_dock tab titles have no accessibility labels: click a tab by the leaf's rect (`leaf.rect` top to `leaf.viewport` top).
- The scratchpad is shared by the batch's agents: name scripts uniquely (another agent's `gpu-test.sh` was overwritten once).
- nwn_tlk writes bytes the codepage lacks as raw bytes (not UTF-8): `from_json` maps stray bytes to U+10FF00+b and back.
