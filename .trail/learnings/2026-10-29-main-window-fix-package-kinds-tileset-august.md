---
title: "Main window fix package: kinds, tileset, haks, reports, caches, maximize, kept tabs, menus, covered arrow"
author: August
date: 2026-10-29
branch: worktree-agent-a733642b3919959af
cost: { minutes: 1761, tokens: 13512975 }
touches:
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/tests/app/windows.rs
  - docs/deferred.md
  - docs/log.md
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/keys.rs
  - crates/mg-ui/tests/app/options_and_keys.rs
  - docs/manual/02-main-window.md
  - Cargo.lock
  - Cargo.toml
---

## Decided
- One `Moonglow::tileset` (an open area view's `terrain.set`, else `mg_area::tileset` from the game) and `Moonglow::open_tileset` (the view's tools, with its `TileIndex`). A tile tab's title uses `open_tileset` only: a title never reads a tileset.
- A module's haks: `mg_module::haks_of(&ifo_root)` (Mod_HakList, else the single pre-1.69 Mod_Hak where the list field is absent; trimmed). `Module::haks` and `mg_ui::listed_haks(ws)` use it. Module Properties still reads its rows itself (it edits them in place, empty rows included) and says when only Mod_Hak names a hak. `GameInstall::find_hak` / `user_hak_dir` are the one search; `add_haks` gives `priority::HAK` where the file's parent is `root/data/hk`, else `HAK_USER`.
- Blueprint windows are kept by `ObjectKind::name()` for all nine types; `Tab::kind_before()` = "blueprint" for UTE/UTS/UTT/UTW so sizes and places kept under the old shared name still apply (`kept_by_kind` in lib.rs).
- A conversation's fingerprint (kept in open-tabs records) is `siphasher::sip::SipHasher13::new()`: equal to today's `DefaultHasher` output, so no record loses its marks; pinned by `a_conversation_s_fingerprint_is_fixed`.
- Maximize: a 🗖/🗗 button drawn over the window bar beside the tabs (registered after the bar's own interact, so it takes the clicks), `Id::MaximizeWindow`/`Cmd::MaximizeWindow` (Ctrl+Shift+M) on `front_window_tab`; maximized windows are re-put over `maximize_room()` in `remember_window_sizes` whenever their `put` differs from it.
- The project file is re-read with reloading off too (`reload_timer` → `reread_project_file`, every 3 s as frames are drawn, no repaint asked); Setup Store refreshes `w.rules` from the module each frame.

## Tried and ruled out
- Moving keyboard focus off egui_dock's covered arrow after the dock is drawn (`move_focus` in the same pass): egui hands focus on only as later widgets ask (`interested_in_focus`), so nothing moved. Do it before `DockArea::show_inside` (`pass_covered_arrow`), with the rect kept from the last frame; and remember the last Tab's Shift (kittest resets modifiers the next frame, so `i.modifiers.shift` there is always false). The cover's own `ui.interact` must be `Sense::CLICK`, not `Sense::click()` (which is focusable).

## Watch out
- Before-pictures: `git archive <commit> | tar -x` into the scratchpad and build there with `CARGO_TARGET_DIR=<worktree>/target/before` (on disk, not tmpfs); `mg_testkit::output_dir` writes under that tree's own `target/test-output`.
- A screens case wanting a window: Module Properties opened with the module is docked in the middle; close and open it again (`OpenTab`) to have it in a window.
- Store project resrefs are 16 bytes at most: `tdod_vendor_other` is refused by the project file's reader with a log warning, not an error.
