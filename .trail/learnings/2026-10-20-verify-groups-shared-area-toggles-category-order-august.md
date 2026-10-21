---
title: "Verify groups, shared area-view toggles, a clearable log, category order"
author: August
date: 2026-10-20
branch: develop
touches:
  - crates/mg-module/src/doctor.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-ui/src/settings.rs
  - apps/mg/src/main.rs
---

## Decided
- Verify's report is filtered by group, not by check: `mg_module::doctor::Group` (references, tilesets, models, 2das, objects, module, plugins; ids are kept in settings, so they are fixed), `Check::group`, `Group::of(&Finding)` (an unknown check id is a plugin's) and `doctor::leave_out`, shared by the GUI's Verify Module (Options › Verify, `Settings::verify_skip`) and `mg verify --skip`. A group that is left out is still computed and counted; the summary line names it, so a filtered run isn't read as a clean module. When plugins are left out, what they wrote to the log goes with them.
- Verify Module is one click and has no window, so its choices are an Options page. Build Module is a different dialog (Aurora's TdlgVerifyModule) and doesn't run the doctor.
- The area view's object-kind toggles are shared state: `Settings::hidden_in_areas` (kind plurals plus "Start"). A new view reads it, and a change in the toolbar (or Set Start Location turning the marker on) goes to every open view through `share_shown`. Views keep their own `show` array, so tests can still set one view directly.
- Category list order: the dropdown deliberately follows the order the builder gives in Palette › Categories (an older decision, held by a test). A builder wanted it alphabetical, so the owner chose an option, Options › General › List a blueprint's categories alphabetically (`categories_by_name`), off by default.
- Clear Log is a command (`commands::Id::ClearLog`, View menu and command palette), plus a Clear button in the log's corner named "Clear Log" for tests.

## Watch out
- A blueprint editor's Category combo carries the category as its accesskit *value*, not its label: find it with `get_all_by_value`.
- Sessions started from a "New session" card may share this checkout instead of getting worktrees: check `git worktree list` and `git status` before a whole-file rewrite, `cargo fmt` or a commit. Format only your own hunks, and stage only your own changes.
