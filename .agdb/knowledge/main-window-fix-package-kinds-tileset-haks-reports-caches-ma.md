---
type: Decision
title: "Main window fix package: kinds, tileset, haks, reports, caches, maximize, kept tabs, menus, covered arrow"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [crates/mg-ui/src/lib.rs, crates/mg-ui/tests/app/windows.rs, docs/deferred.md, docs/log.md, crates/mg-ui/src/commands.rs, crates/mg-ui/src/keys.rs, crates/mg-ui/tests/app/options_and_keys.rs, docs/manual/02-main-window.md, Cargo.lock, Cargo.toml]
fingerprint: v2-1801e50ba852e497
created: 2026-10-29
author: August
description: "Records decisions for the Moonglow main window fix package: single tileset and hak lookup, blueprint window keys, stable conversation fingerprint, maximize button, project file rereading, plus a failed focus fix and build tips."
asks:
  - "How does Moonglow decide which tileset a tile tab title and open area use, and why doesn't the title read a tileset?"
  - "Keyboard focus keeps landing on egui_dock's covered arrow and moving it after DockArea::show_inside does nothing; what works instead?"
  - "Where do a module's hak list and the pre-1.69 Mod_Hak get read, and how are hak files located and prioritized?"
  - "Why was a store project resref like tdod_vendor_other rejected with only a log warning when the project file loaded?"
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
