---
type: Decision
title: "Document tabs cut in the middle, keeping the extension (the owner's ruling)"
description: "Why a crowded tab bar shows 'at…01.nss' rather than 'at_m1q4…', what counts as an extension, and where the cut is made for every tab bar."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T14:16:53Z }
touches: ["crates/mg-uikit/src/dock.rs:doc_tab_title", "crates/mg-uikit/src/dock.rs:middle_cut", "crates/mg-ui/src/tabs.rs:prepare"]
asks:
  - Why do cut tab names show an ellipsis in the middle instead of at the end?
  - "A tab named 'x (preview)' or '1.5 notes' is cut oddly: what is an extension to the cut?"
  - "Where is a document tab's name cut, for floating windows and editor groups too?"
  - Can an emoji sequence be split by the tab cut?
fingerprint: v2-30ae437d13925bab
created: 2026-11-14
---
## Decided
- The owner's ruling, 2026-11-14: "Tab cut in the middle to keep". `mg_uikit::dock::doc_tab_title` cuts with `middle_cut`: whole where it fits; else the name's start and end around an ellipsis, the start a character longer, the extension whole while at least one character of the name before it fits; otherwise the whole name cut in the middle; the ellipsis alone where not even a character at each side fits; a one-character name never cut.
- An extension: after the last dot, the dot not first, 1 to 8 letters or digits (EXTENSION_MOST). So "nw_c2 (preview)", "1.5 notes" and ".hidden" are cut as names without one.
- One place: every pane's titles (the main area's editor groups and the floating windows' panes) are laid out by `tabs::prepare` through `doc_tab_title`, cached by name, cap, dot, font and atlas; the full name stays in the tab's tip and accessible name (`Look.name`). egui_dock itself cuts nothing.
- Widths are measured on the assembled string (a binary search over the characters kept), so a cut title is never wider than the cap `doc_tab_cap` gave.

## Watch out
- Cut by `char`, as egui lays out one glyph a char: a zero-width-joiner emoji sequence or a letter with combining marks can be parted. mg-uikit is egui alone (no segmentation crate).
- Chapter1's at_m1q4c_m1q_001 and at_m1q4e_m1q_001 still read alike when cut hard: what tells them apart is in the middle (the tip has it).
