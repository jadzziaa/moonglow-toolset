---
type: Gotcha
title: "Inside the DockArea pass read snapshots, never app.dock: window_tabs_maximized, panes_tabs, fronts"
description: "Why a tab's menu, close button or body must read the frame-start snapshots instead of app.dock, and how to test a floating tab's own menu in kittest."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T08:44:14Z }
touches: [crates/mg-ui/src/tabs.rs, "crates/mg-ui/src/lib.rs:surface_maximized", crates/mg-ui/tests/app/windows.rs]
asks:
  - "Why does double-clicking a floating window's tab no longer maximize it?"
  - "Why does a floating tab's menu always say Maximize, never Restore?"
  - "Can TabViewer::context_menu or on_tab_button look at app.dock?"
  - "Why does click_secondary on a floating window's tab open the window bar's menu in a test?"
fingerprint: v2-4976c39d0aed0f21
created: 2026-11-25
author: augustaugur
---
While egui_dock draws (`DockArea`), Moonglow's dock is swapped out of `app.dock`: it is empty for the whole pass. Code that runs inside that pass (`TabViewer::context_menu`, `on_tab_button`, `on_close`, a tab's body) and asks `app.dock` (`find_tab`, `iter_leaves`, `surface_maximized`) gets nothing, silently.

## Decided
- Read the snapshots taken before the swap: `Moonglow::window_tabs_maximized` (tab -> its floating window is maximized; only tabs off the main surface are keys, so `contains_key` also means "in a floating window"), `panes_tabs`, `fronts`.
- `surface_maximized` is for the window bar (drawn outside the pass) only.
- Wave 2 regression fixed 2026-11-25: `context_menu` and the double click had switched to `surface_maximized`, so a floating tab's double click did nothing and its menu always said Maximize.

## Watch out (tests)
- In kittest a floating window's bar has its own right-click menu over its tab: `click_secondary` on a tab of most windows opens the bar's menu, not the tab's. A test of the tab menu needs `doc_tab(..).click_secondary()` on a window that opens the real tab menu: a script window does.
