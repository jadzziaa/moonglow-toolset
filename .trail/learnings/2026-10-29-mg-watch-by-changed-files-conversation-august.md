---
title: "mg watch by changed files, conversation tags, tileset rules and doors"
author: August
date: 2026-10-29
branch: worktree-agent-ad0524d4fbd438f51
cost: { minutes: 2030, tokens: 14021777 }
touches:
  - apps/mg/src/watch.rs
  - apps/mg/src/test.rs
  - apps/mg/src/main.rs
  - crates/mg-module/src/watch.rs
  - crates/mg-ui/src/outside.rs
  - crates/mg-module/src/dialog_io.rs
  - crates/mg-set/src/edit.rs
  - crates/mg-ui/src/tileset_view.rs
  - apps/mg/tests/watch.rs
---

## Decided
- `mg watch` reads the project once, then only what the system says changed: `mg_module::watch::Watch` (moved out of mg-ui/src/outside.rs so the CLI and the window share it; the repaint is an `OnChange` callback) gives the paths, `Module::outside_scan_of` + `OutsideScan::run` + `outside_found` read them, and the watcher applies each `Outside` (set/remove, then `Module::took`). It reads the project whole again (`Watcher::reread`) only for `nasher.cfg` changed (its stamp, compared each step), an ARE added or removed, or module.ifo changed: `Project::settle_area_list` runs only at open, so an incremental module.ifo or new area would miss nasher's area-list fill.
- A change whose bytes equal the module's (a touched file, JSON re-formatted) is taken (`took`) and counts as nothing; `Step::is_empty` makes `look` return None for it.
- `mg watch --test` runs `test::run_on` (the test run on an already-read Module, split from `test::run`) in the step callback, on `w.module().clone()`. Test scripts include `mg_test`, which only `testing::inject` puts into mg test's copy: the watch adds Moonglow's include as a lowest-priority MemContainer layer of its ResMan (priority 0), so tests compile in the watch without being put into the served module, and `inject` still sees the module as having no own include.
- Conversation tags `anim:`/`delay:`/`comment:` are written only when not the default (Animation 0, Delay u32::MAX, empty Comment), so exports of conversations without them are byte-identical; `play_outline` uses `tags()`, so the corpus round trip (2,356 conversations, MOONGLOW_FULL_CORPUS=1) now also checks they survive. Comments use `percent()` (spaces too in Twine passage tags; spaces at the ends always, as tag readers trim).
- Tileset door sections go after the tile's own last door (else the tile), via `SetFile::add_section_after`: `add_section` places by family, which for `TILEnDOORm` means after the last door of any tile. A rule or door section that exists past its count is reused (keys set), not added twice.

## Watch out
- `a_project_is_served_and_started_again_when_it_is_built_again` read `/proc/PID/cmdline` right at the `started` event; under load (other server tests at once, `timed_turn` allows 3) the process was still `mg`'s, before exec. It now reads until the server's arguments are there.
- In kittest, a `widgets::dropdown`'s shown text is the ComboBox node's value, not a label: find it with `by().role(Role::ComboBox)` and `accesskit_node().value()`.
- The tileset editor's side window doesn't clip a page that is taller than it: a long page (the rules) needs its own ScrollArea.
