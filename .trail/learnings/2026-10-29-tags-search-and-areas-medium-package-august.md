---
title: "Tags, search and areas medium package: duplicate tags everywhere, Find and Replace fields and regex, StrRef usages, areas by what they hold"
author: August
date: 2026-10-29
branch: worktree-agent-a95782fbf157a5293
cost: { minutes: 1718, tokens: 12854923 }
touches:
  - crates/mg-ui/src/area_props.rs
  - crates/mg-ui/tests/screens.rs
  - apps/mg/src/main.rs
  - apps/mg/tests/areas.rs
  - apps/moonglow/src/main.rs
  - crates/mg-module/src/areas.rs
  - crates/mg-ui/src/area_sets.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/search.rs
  - crates/mg-ui/tests/app/areas.rs
---

## Decided
- Duplicate tags are weighed for every command applied, in one place: `Moonglow::apply` calls `tag_clash::made_by` (a thin wrapper over `mg_edit::tags::made_by`) before `ws.apply` and `tag_clash::check` after. Made = a `SetField "Tag"` (String) on a GIT list item or an ARE root whose tag changes, and an `InsertItem` into a GIT ObjectKind list in a command that removes nothing from that list (a move is not a placement). Undo/Redo go through their own action, so they ask nothing. The paste's own `Action::CheckTags` was removed (the hook covers it); an area's Copy… still calls `check` with `whole_area` (it is a `SetResource`). `what` is the command's label in quotes ("“Paste”", "“Place plc_barrel”").
- `mg set`/`mg apply` (edits::run, `made_twice`), `mg areas --set Tag=` and `mg replace --only tags` (`objects_made_twice`) warn as notes: project's `paste_warning`, else Custom with every tag the command sets taken as a builder's; an inserted object whose blueprint isn't the module's is taken as having its blueprint's tag (no game data is read there).
- A blueprint copied with another blueprint's tag is said in the Copy window, not the clash window (blueprints aren't in TagIndex): `UsageIndex::blueprints_tagged` (root `/Tag`), weighed by the same rule (Custom: only a tag typed in the window).
- Retag follows `LinkedTo` only within what was made (TagClash::made), only where exactly one made object in that GIT had the old tag. Scripts are named (UsageIndex::mentions), never rewritten. `tags::retag` keeps the number's width (Create Set's `_01`).
- Find and Replace's plain kinds live in `mg_module::text::TextKind` (Tag, ResRef, Variable, Field); `TextKind::PLAYERS` is the default (was ALL): callers meaning player text use PLAYERS. A resource's own name (blueprint's resref_field, ARE's ResRef at root) is never a ResRef hit. Regex is regex-lite (ASCII case folding only): `text::Finder` for Find and Replace (groups via Captures::expand), `query::like` with a thread-local compiled cache for Advanced Search/mg search (`Query::regex`, `Query::check`; `Kept` resets when the flag changes).
- StrRef plain-number fields: `rename::STRREF_FIELDS` = ITP `STRREF` only; `CatName` (a dword of ~20 shipped palettes' `Cat` lists) is not known to be a StrRef and was left out. Script calls: `rename::STRREF_FUNCTIONS` (9 from nwscript.nss, SendMessageToPCByStrRef's arg is the 2nd), a literal Int token only. Haks' 2DAs: `rename::hak_strrefs` over layers with priority HAK/HAK_USER, each hak's own copy (get_in).
- Edit Areas Together's Holding: `mg_module::areas::{Has, Holds, HoldsCache}` (ARE root + GIT AreaProperties fields, VarTables of GIT and ARE); the chooser flushes the workspace and recomputes only when the condition or `ws.revision()` changes (flush doesn't bump revision). Sets: `area_sets` (Shelf "area-sets", `{module, areas}`), `Moonglow::area_set_dir` set by the app like `prefab_dir`.

## Watch out
- In this worktree, Bash refuses heredocs/python/sed whose text it can't verify (often anything with GIT, `*`, `!`, or `$var` in sed): write the script with Write to the scratchpad and run `python3 path`.
- kittest pointer clicks can miss in a centered auto-sized window whose content just changed height (the window re-centers over 2–3 frames; the click lands on the widget above): use `click_accesskit()` or Enter. The chooser grew two rows and broke `several_areas_are_edited_together` until its list went from 320 to 250 points high.
- `one_gate` forbids `egui::ComboBox` in mg-ui: use `widgets::dropdown`.
- Before pictures: commit, `git checkout <old> -- files`, render with a screens test that compiles against both, `git checkout HEAD -- files`.
