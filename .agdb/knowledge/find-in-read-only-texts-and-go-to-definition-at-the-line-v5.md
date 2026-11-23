---
type: Gotcha
title: Find in read-only texts and Go to Definition at the line (v5.2 W3)
description: "How a read-only text tab (browser::LongText) got its Find bar, Select All and a marked line, and how Go to Definition places a game script, with the traps met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T12:48:25Z }
touches: [crates/mg-ui/src/browser.rs, crates/mg-ui/src/script_nav.rs]
asks:
  - "Why does Ctrl+F do nothing in a read-only tab: front_tab is None while it draws?"
  - Where does a game script opened by Go to Definition go in As Floating Windows mode?
  - "Why does a scroll_offset sent to a floating window's first frame not scroll?"
  - How is a read-only text searched without holding a frame?
fingerprint: v2-0c28cf4240fa53b8
created: 2026-11-23
author: augustaugur
---
- State: `Browser::finds` by (key, layer) holds a `TextFind` per read-only text tab (bar, query, matches, current, marked line, reveal, Select All). `LongText::ui` (plain, no Find) stays for `plain_ui`/hak Preview; `ui_finding` is the tab's. `Viewed` is a shared Arc and `LongText::ui` takes &self, so Find state cannot live in LongText.
- While the editor area draws, `app.dock` is swapped out (lib.rs `std::mem::replace(&mut self.dock, ...)`): `front_tab()`/`active_editor()` read an empty dock then. Use `table_view::active_tab(app)` (`active_tab_now`, taken as the frame began). This was why Ctrl+F first never reached the view.
- Keys: the view takes Find only when its tab is that front tab (or the pointer is over it) and no list has the keys (`palette_view::a_list_has_the_keys`): the module tree's Ctrl+F is its Filter's. A press in the text gives the keys to the editor area (`give_keys(Keys::Area)`).
- Search: `TextFind::sweep` looks 64 lines between clock checks for 3 ms a frame (as `talk_view::sweep`), per line through `script_tools::find` so case/whole-word match the script editor's Find exactly; it requests a repaint while unfinished, so `settle` (try_run) waits for it; no `under_way` entry (an unfinished sweep behind another tab would never end).
- Scroll: `show_rows` draws only rows in sight, so reveal sets `ScrollArea::scroll_offset` from line x row height. A floating window's first frame clamps it (no room yet): the reveal is sent again up to 5 frames until the offset is reached.
- The same swap trap for cleanup: dropping closed tabs' finds by `dock.find_tab` while drawing dropped every other tab's (the dock is empty then); `text_ui` checks `app.panes_tabs` (taken before the swap). Floating windows just opened are not the focused leaf: Ctrl+F reaches them with the pointer over the text or after a click.
- Highlights take x from the line's galley (`layout_job`, cached), not columns x glyph width (tabs).
- Go to Definition of a game script: `script_nav::open_definition` is the one place deciding where (owner's ruling 2026-11-23, q6 a): As Floating Windows, pushed into the leaf of the script asked from (`Nav::asked_from`, set by `declaration_at`, which both script_view callers call first; `front_tab` is ModuleProperties after a floating script opens), then `Action::OpenTab` brings it forward; As Tabs, plain OpenTab. Already open: OpenTab fronts it. The line comes from `script_source` text (same decoding as Content::Script) via `mg_core::text::line_of`.
- Tests: `tests/app/read_only.rs` (fixture include via `GameData::new` over a MemContainer, no install); `Moonglow::tab_rect` is the tab button, not its content: tests use `read_only_view_rect_for_test`.
