---
type: Decision
title: "text typed into the talk table, LocString precedence, typed lines and undo"
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
touches: [crates/mg-ui/src/talk_typed.rs, crates/mg-module/src/talk.rs, crates/mg-module/src/talk_range.rs, crates/mg-ui/src/talk_view.rs, crates/mg-corpus-tests/tests/engine_tlk.rs]
fingerprint: v2-6ff77f0b75ae8345
created: 2026-11-02
author: August
description: "Explains how typed talk-table lines work: a string's own text in any language overrides its StrRef, typed lines have separate undo state (TypedState), and commits must be idempotent."
asks:
  - "Why does an item show its French name instead of my custom talk table text on an English server?"
  - "Why is Move to Talk Table disabled for a localized string?"
  - "Why does a loc field show empty or 'no such string' after I type a talk table line, and a second commit re-converts it?"
  - "How is undo/redo for typed talk-table lines tracked separately from the Table history, and what does Workspace::undo_depth do?"
---
- A localized string's own text, in ANY language, goes before its StrRef in the game (engine_tlk `a_typed_name_is_read_from_the_custom_talk_table`: an English server named an item by its French substring beside a custom StrRef). Moving text into the talk table must take the language's text out, and a string with another language's text can't rely on its line: typed text keeps it in the file; Move to Talk Table is disabled for it.
- Typed talk-table lines live outside the Table's own history (`Table::put_typed`/`take_back`/`put_back`, `typed_unsaved` makes it dirty) and are kept beside the module's step by `Workspace::undo_depth` (`talk_typed::TypedState`): `Moonglow::apply` prunes stale entries and clears redo links on every apply; the Undo/Redo handler calls `may_redo` before and `stepped` after. `adopt` dropping history calls `typed.forget()`.
- `Table::put(row, None)` empties a row before the last instead of truncating (typed lines and editor lines interleave); a typed line past the end pads with empty entries, which `take_back` trims again.
- The game data's custom table is the saved file until reloaded: after a typed line, `talk_typed::shown` sets `GameData::set_custom_tlk` to the open table, or the field shows empty ("no such string") and a second commit re-converts it. `apply_with` also compares with the line's text a field's StrRef names, so re-committing the same text is no change.
- egui commits a typed loc field twice in the kittest harness (Tab, then the next pass): anything converting on commit must be idempotent.
- A table kept in the module is now put in at save by `ws.derive` + `mark_modified`, not an undoable `SetResource` step: undoing that step put the old table under lines the fields named.
- Commands from typed loc fields are `Action::ApplyTyped`; blueprint windows' Cancel/partial/look_only match it beside `Action::Apply`.
