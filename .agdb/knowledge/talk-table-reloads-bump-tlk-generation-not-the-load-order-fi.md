---
type: Gotcha
title: "Talk table reloads bump tlk_generation, not the load order (fix X)"
description: "Why load_custom_tlk no longer calls load_order_changed, and which caches key on Moonglow::game_names (load order plus talk table generation); this replaces the load_custom_tlk bullet of the fix U1 caches note."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T06:39:52Z }
touches: ["crates/mg-ui/src/lib.rs:Moonglow::load_custom_tlk", "crates/mg-ui/src/lib.rs:Moonglow::game_names", "crates/mg-ui/src/palette_view.rs:found"]
asks:
  - what must a cache keyed on load_order know about custom talk tables
  - "why does saving the talk table not clear the browser's 2DA views"
  - a renamed talk table line is still found by its old name in the palette
  - what is tlk_generation for
fingerprint: v2-52405a40fd09d201
created: 2026-11-24
author: augustaugur
---
Review U1 of efe958b1 found that `load_custom_tlk` calling `load_order_changed()` (the fix U1 note's bullet) emptied the browser's 2DA views (their Find, selection, hidden columns), its pictures, hak docs' resources and moved every model view's stamp at every talk table Save (`talk_view.rs` save -> `reload_custom_tlk`) and Create (`talk_view/panels.rs`).

Now `load_custom_tlk` bumps `Moonglow::tlk_generation` (both where the table changed: cleared or read), and caches that hold text read through the talk tables key on `app.game_names()` = `(load_order, tlk_generation)`:
- palette `FoldCache.names` (Find's folded names) and `palette.found`'s key (the finds themselves: keyed on the search and revision alone, a reload found by the old line until the search changed),
- store `base_items` (blueprint/store.rs), blueprint wizard item types, Setup Area Transition's area names (situated.rs `cached_areas_for`),
- the dialog's journal summary (dialog_view.rs) and the Journal view's `Seen.names`.

Still on `load_order` alone (no talk-table text, or left as they were): quick_open's game part, the wizards' category lists (names resolved when drawn; only their sort order could lag), tile_select, area_props, model views.

`open_module` still calls `load_custom_tlk` then `load_order_changed`: a new module is a load order change.
Test: `palette_view::tests::a_talk_table_read_again_is_found_by_its_new_lines_and_keeps_the_browser` (a custom tlk served from a MemContainer layer; a higher-priority layer stands in for the saved file).
