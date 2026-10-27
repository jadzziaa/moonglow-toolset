---
title: "Inventory lists in sight, Advanced Search kept"
author: August
date: 2026-10-27
branch: lists-and-search-index
cost: { minutes: 1106, tokens: 10411600 }
touches:
  - crates/mg-module/src/search.rs
  - crates/mg-module/src/rename.rs
  - crates/mg-ui/src/search.rs
  - crates/mg-ui/src/blueprint/inventory.rs
  - crates/mg-ui/src/blueprint/picker.rs
  - crates/mg-ui/tests/world_perf.rs
  - crates/mg-ui/tests/campaign_perf.rs
---

## Decided
- Advanced Search keeps per-query results, not parsed GFFs: `mg_module::search::Kept` holds, per GFF resource, its bytes (Arc) and what each condition found there (`Found`: hits per condition, the item blueprints a blueprint carries); asked again with the same conditions, only resources whose Arc isn't ptr_eq (and bytes differ) are read, as `rename::UsageIndex::refresh` does. A new question reads every GFF once for all its conditions, in parallel (rayon), each tree dropped once walked. The cross-resource parts (carried items, the AND join, types/placed) are recomputed from the kept `Found`s every time, so they never go stale. `search::search` (mg CLI, tests) is `Kept::default().search`. The window's Kept lives on Moonglow (`search_kept`) and is reset where `palette.forget_game_data()` is called (its text is named with the game data and codepage).
- Ruled out: keeping parsed GFF trees in `UsageIndex` so any question walks without parsing. It was faster for new questions (0.1 s vs 0.2 s in the persistent world) but cost ~0.6 GB more max RSS on the 149 MB world (two indexes alive: 2.7 GB vs 2.1 GB), and Find References users would pay it without searching. `UsageIndex::refresh` now reads on every core (1.4 s to 0.2 s), unchanged otherwise.
- Item lists (`inventory::item_list`): rows out of sight take their room only (`is_rect_visible` on an estimated rect, `allocate_space`), as palette_view's rows do. Rows are not uniform (each as tall as its icon's drawn part, which needs the decoded picture), so `show_rows` would change the look; heights of rows drawn are kept in egui temp data per list (key + path, so per store page), unknown rows take the median of those drawn (68 before any). Callers pass a `row` closure building icon and name (and a store's price) for rows in sight only.
- `Form::kept_blueprint` and `blueprint_names` keep their results in `PaletteView::kept` (`picker::Kept`): blueprints until the workspace revision moves, names while the two palettes' Arcs are the same; cleared by `forget_game_data` and when the custom TLK changes.

## Watch out
- The equivalence of the new search was checked by pasting the old `search`/`search_one` into a temporary test module of search.rs (private helpers are reachable there) and comparing on six shipped modules x 140 queries, before and after removing/adding/changing resources; not kept in the repo. Do the same for further changes to the join.
- `found::events` still reads the resources of what was found (0.1 s for 8,785 hits in the world): most of what a question asked again costs there.
- Item list pictures before/after: only the scroll bar's thumb differs (its size goes by guessed heights); rows in sight are pixel-identical. Log timestamps differ in every screens.rs picture.
