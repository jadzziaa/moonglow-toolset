---
type: Decision
title: "Typed resrefs: Aurora's characters, filtered as typed"
tags: []
status: stable
generated: { by: trail, at: 2026-11-06T00:00:00Z }
touches: [apps/mg/src/main.rs, apps/mg/tests/places.rs, crates/mg-core/src/lib.rs, crates/mg-core/src/resref.rs, crates/mg-corpus-tests/tests/tiles.rs, crates/mg-ui/src/blueprint/mod.rs, crates/mg-ui/src/blueprint/replacements.rs, crates/mg-ui/src/blueprint_wizard.rs, crates/mg-ui/src/copy_as.rs, crates/mg-ui/src/creature_wizard.rs]
fingerprint: v2-4e74d8d293eb4f7d
created: 2026-11-06
author: August
description: "Typed resrefs accept only letters, digits and underscore, lowercased, via ResRef::typed, while NewName keeps allowing '-'; the UI field filters input through ResRefText, and raising many tiles already works via Shift+drag."
asks:
  - "Why does the resref text field silently drop characters like '<' or '-' as I type instead of showing an error?"
  - "Which characters are allowed in a typed resref, and how does that differ from NewName for hak file names?"
  - "Why not use TextEdit::char_limit(16) with the filtering buffer for resref fields?"
  - "Do I need a new Grid::raise_corners to raise many tiles at once, or does Shift+drag already do it?"
---
## Decided
- A resref a builder types takes letters, digits and `_` alone, lowered (`ResRef::typable`, `ResRef::typed`, `Typed::RULE` in mg-core), as Aurora's ResRef fields and its names made from text. `NewName` (`new_name`) stays the wider rule (also `-`) for files Moonglow names from file names: a hak's `my-tex.tga` added must keep working.
- In the UI the field filters, not refuses: `widgets::ResRefText` is an egui `TextBuffer` whose `insert_text` drops other characters and lowers the rest; `widgets::resref_edit` and `commit_resref_in` use it. A value already there (a hak's `my-tex`, picked) is untouched; only inserted text is filtered. Parsing at commit stays `ResRef::from_str`.
- `mg rename` checks NEW with `ResRef::typed`. Plugins keep `from_str` (code, not typing).
- Qlippoth's "raise many tiles": Raise/Lower Shift + drag already raises a rectangle; corner by corner equals raising it whole on all 12 shipped tilesets with heights (tti01 Frozen Wastes has no plateau edge tiles on its open ground either way). Test `a_rectangle_raised_corner_by_corner_is_a_plateau` in mg-corpus-tests tiles.rs.
## Tried and ruled out
- `TextEdit::char_limit(16)` with the filtering buffer: egui cuts the raw typed text to the room left before `insert_text` sees it, so `<>` used up the room. The buffer cuts to 16 itself; no `char_limit`.
- A `Grid::raise_corners` (all marked corners in one stroke): fits exactly where the corner-by-corner drag does, so not added.
- Right-drag to lower many corners: right drag turns the camera (Aurora's binding).
## Watch out
- The tile-seam merge needed its nine shipped seams in `KNOWN` of crates/mg-corpus-tests/tests/doctor.rs; the side session ran only the sample.
- `engine_grass` can fail under load (server save timeout); passes alone.
