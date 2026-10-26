---
title: "A module's open tabs kept and opened again"
author: August
date: 2026-10-26
branch: remember-open-tabs
cost: { minutes: 862, tokens: 9341710 }
touches:
  - apps/moonglow/src/main.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/03-modules.md
  - docs/manual/10-options.md
---

## Decided
- A module's open tabs are kept in Moonglow's data folder (`open-tabs/<name>-<hash>.json`, named by `mg_module::locks::file`, i.e. by the canonical module path; a folder or nasher project by its folder), never in the module. `Moonglow::open_tabs_dir` is `None` unless the app sets it, so tests never write there unless they give a scratch dir.
- Option on by default (Options › General, `Settings::no_reopen_tabs`). Off: neither written nor read; an old record is left alone. A record that restores at least one tab replaces the "area opened last" step of `open_module`; none restorable falls back to it.
- Tabs are kept as text (`script:hello.nss`, `area:field`, `instance:area:Creature List#3`), not by `Tab::kind()` (which merges variants) nor egui_dock's serde (surface indices shift when an empty surface is dropped, and Moonglow's own window tracking would be bypassed).
- Places are `Place` (middle, palettes, split side share, window) from `place_of`/`split_of`; a window's panes are flattened into one pane; its rect comes from `Moonglow::windows[..].now` (or the pre-maximize rect).
- Restoring waits until the end of the module's first frame (`open_tabs_frame` after `run_actions`): `open_palette` opens the palettes' pane in that frame's actions, with the tree's width known. Restoring earlier gave the palettes a 0.72 split at startup and nowhere to put "palettes" tabs. A module opened from a menu is opened inside `run_actions`, so `open_tabs_frame` runs in that same frame with `open_palette` still set: the restore waits while `open_palette` is true (`a_tab_kept_in_the_palettes_pane_opens_there_again` catches it; tests that call `open_module` from outside a frame do not).
- Written when the dock's shape hash (leaves, their tabs and front, the focused leaf) changes, in `close()` (which `use_module` calls, so switching modules keeps the old one's) and on `Action::Quit`; written via `.json.partial` + rename, skipped when the text is unchanged.

## Tried and ruled out
- Pushing `OpenTab` for every kept tab: places them by their kind's remembered place and regroups windows by the gather settings, not where they were.

## Watch out
- The restore pushes a `NoArea` into the middle first, so `split_middle` has a middle to split and closing Module Properties doesn't collapse the middle; `keep_middle` removes it the next frame.
- Area views are lazy (made on first draw; egui_dock draws only each leaf's active tab): ten restored areas make one view. On tdod.mod (224 resources, 3 areas, haks linked) reopening with 10 tabs: open_module about 1 ms, open plus 3 frames about 4.5 ms against 2.7 ms with the option off.
- Tests that delete the record file behind the app's back see nothing written: the last written text is cached in `OpenTabs::written`.
