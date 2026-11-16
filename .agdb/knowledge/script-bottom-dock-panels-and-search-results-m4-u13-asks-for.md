---
type: Decision
title: "Script Bottom Dock panels and Search Results (M4 U13): asks forward, redraw stamps, live error as message and wavy underline"
description: "How a script's Compiler and Bookmarks and the global Search Results live in the Bottom Dock and a floating window's bottom pane since U13, how a script brings one forward, why the default front is not Search Results, the redraw stamp rule, and the live error's new homes."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T20:23:09Z }
touches: [crates/mg-ui/src/script_panels.rs, crates/mg-ui/src/search_results.rs, "crates/mg-ui/src/shell.rs:bottom_forward", "crates/mg-ui/src/shell.rs:follow_bottom", "crates/mg-ui/src/shell.rs:script_note", "crates/mg-ui/src/script_view.rs:underlined_line", crates/mg-ui/tests/app/script_bottom.rs]
asks:
  - How does a compile or a search bring a Bottom Dock panel to the front?
  - Why does the Bottom Dock open on the Log rather than Search Results?
  - "Where did the script's live error row go?"
  - How do I feed Search Results from another dialog (U28)?
fingerprint: v2-972ea61965f845c1
created: 2026-11-16
---
- Compiler and Bookmarks are `Panel`s a script's tab brings to the Bottom Dock (`Brings::bottom`, `SCRIPT_BOTTOM`); Search Results is global (`Panel::BOTTOM_TABS = [SearchResults, Log]`, `SWITCHES_AFTER = [SearchResults]`, always available, empty state) and follows a floating script into its window's bottom pane (`window_bottom`). Bodies: `script_panels::{compiler, bookmarks}`, `search_results::body`, all drawn BEFORE the editor.
- Bottom Dock default front (`shell::bottom_shown`): run choice, else kept for the kind, else the editor's own first, else the Log — NOT `held.first()` (that is Search Results, an empty panel beside every other editor).
- A script asks a panel forward with `ScriptTools::ask_forward(from, InfoTab, open)` (compile, bookmark, search); `shell::bottom_forward` acts at frame start (the editor, drawn inside the dock area, can't see `active_editor`): floating → `Moonglow::window_bottoms` for EVERY tab of that window (the pane is drawn for the tab in front, which may be another script); docked → only if `from` is the active editor (searches whatever is in front); dock opened only if `open` (searches, failed compiles). Nothing written to settings (`focus_bottom`).
- Redraw rule (same lesson as U12's A1): each body stamps `(pass, least ScriptTools::made)` in `script_panels::begin`; `shell::follow_bottom` after the frame asks for one repaint only if a panel drawn THIS pass showed a `made` that changed since, or a forward is pending. Every change to messages/bookmarks/hits must bump `made` (ask_forward does; restoring kept marks does).
- Search hits live on `Moonglow::search_results` (`what` + `Vec<Hit>`), not on `ScriptTools::search` any more; `Hit` is an enum for U28 to extend (Find Instance, Find and Replace Text). `show_search_results(what, hits, from)` feeds it.
- The live error: no row under the header. Docked in front it is the Status Bar's message (`Shown::note`, in the log's last line's slot, the compiler's own words, click → jump + keys back); in either host a wavy underline (`mg_uikit::paint_wavy_line`, token `error`) painted over the galley via `pos_from_cursor`, its tip the error. The underline left `script_layout`'s cache (no relayout when the error moves).
- "Line N, column M" is a `Count` from `ScriptBuffer::place`, read from the editor's galley (`layout_from_cursor`: one row per line, never wrapped). Do NOT compute it with `line_of`/`line_start` over the text: that cost ~0.3 ms a frame in editor_perf's cursor cases on nwscript.nss.
- A floating window's bottom pane is held to a third of its height (`shell::window_bottom_range`, default 140), its id `("window-bottom", owner tab)`; it is clipped to the room the right pane leaves (its drag edge reached over the right pane otherwise).
- Tests: kittest Label text is in `value()`, group nodes (`name_group`, `DockTabs` names) have NO bounding box — `rect()` panics; take the union of children's boxes (`script_bottom::place`). The "Bottom Dock" label is its TAB BAR's group, not the body: check bodies by place inside `bottom_dock_rect()`. Live-error tests need the game (`nwscript.nss`), else "FILE NOT FOUND" on line 1.
- Review (2026-11-16): the underline falls back (`script_view::underlined_line`): blank/EOF → nearest line with text above; include error (`Live::include`) → its `#include` line. `shell::script_note` builds the message (docked: Status Bar; floating: the window bottom pane's tab bar end, icon alone under 80). Asks are a `Vec<Forward>` per frame; a bookmark or passing compile never takes the Log's place; empty searches don't open. Status Bar yield order: ≥1400 folder → hints → switch words (`bare`), <1400 T14's.
