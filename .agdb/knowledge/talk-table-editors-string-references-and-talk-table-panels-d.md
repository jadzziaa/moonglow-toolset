---
type: Decision
title: "Talk table editor's String, References and Talk Table panels, dialog.tlk in its own tab (M4 U17)"
description: "How U17 rebuilt the talk table editor on the kit header and three Right Dock panels, why dialog.tlk is a tab of its own (Tab::GameTalkTable), how Find is sliced across frames, and the traps found on the way."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-17T12:04:39Z }
touches: [crates/mg-ui/src/talk_view.rs, crates/mg-ui/src/talk_view/panels.rs, crates/mg-ui/src/shell.rs, crates/mg-ui/src/browser.rs, crates/mg-ui/src/quick_open.rs, "crates/mg-module/src/talk.rs:Table::of_game", "crates/mg-module/src/rename.rs:UsageIndex::strref_usages_until"]
asks:
  - Why does dialog.tlk open in its own tab instead of the Talk Table tab?
  - "The window never comes to rest after Find or Find References in the talk table: where is the work followed?"
  - "Typed text in the String panel lands on another line: how are its field ids made?"
  - "Why does a talk table test's Undo remove a line it never added?"
fingerprint: v2-f7217d5233161133
created: 2026-11-17
---
## Decided
- `Tab::GameTalkTable` ("game-talk-table", title "dialog.tlk") holds the game's dialog.tlk read-only; `app.game_talk: Option<GameTalk>` = `Arc<Table>` made once from `GameData::tlk()` (`Table::of_game`, a copy at opening only) plus its own `TalkView`. Its key `of` is (Arc ptr of GameData, entries ptr, len), NOT a Weak: a Weak makes `Arc::get_mut` (`exclusive`) fail and the window mutates GameData in place. Dropped when its tab is closed (`talk_view::follow`), which stops its References thread.
- The module's table stays `app.talk` + `app.talk_view` (`Which::Module`); `talk_view::parts(app, which)` gives (&Table, &mut TalkView) for either. `Which::strref(row)`: module CUSTOM+row, game row.
- The Resources panel lists a synthetic row (`dialog.tlk`, layer `lang:en`, `browser::is_game_talk`), left out of Export as Files; Quick Open adds a "dialog.tlk" hit in the load order's group; `browser::tab_for` maps the key to the tab.
- Panels: `Panel::String`, `Panel::TalkFile` ("Talk Table"), and the shared `Panel::References` dispatching on `shell::talk_of(editor)`. Shares 700/306/994. String's copy button is a header tool (`panels::copy_tool`, `Panel::tools` = 1).
- String's fields are ids of (which, field, row), Length's through `push_id`, and set at each keystroke (`set_line` with `typing` Some(0..3) merges a burst into one Undo step), so nothing is pending and text never lands on another line; a test of it must type AFTER the selection moves, or one id for every line passes it.
- Find: `talk_view::sweep` looks through `SWEEP_BUDGET` (3 ms) a frame, the clock asked every `SWEEP_SLICE` (64) lines (`Table::find_in`); the lines listed so far are an `Arc<Vec>` appended to with `Arc::make_mut` (shared with the list drawn, never cloned a frame), filtered by Only lines with text slice by slice; a query, revision or Only lines with text changed begins again. Only `list()` advances it; `follow` lets the sweep go where the tab is not in front, so `under_way` ends.
- Find by number takes the table's own numbering: `Table::first_strref` (CUSTOM, 0 for `of_game`), used by `Table::has` (review: dialog.tlk matched custom StrRefs).
- Undo bursts (review): `Table::set_line` joins typing only while `typing_open`; `Table::end_typing` ends it, called by `talk_view::follow` when the chosen line changes and by `panels::string` when a field loses focus; `undo`/`redo` end it too.
- Floating first size (coordinator's ruling at the review): editors that bring window panes (talk table, 2DA, conversation) open clipped to the main window less 24 a side (`Moonglow::open_window`), not 0.85 of the room between the docks; a talk table's window gives String `TALK_WINDOW_SHARES` (1100/300/600). The app tests' unthemed harness lays String out taller than the themed pictures: check first-size layouts in the pictures too.
- A hak's own `dialog.tlk` is a resource like any (the game reads dialog.tlk from lang/ only): the Resources panel lists both; Quick Open's hit is `Target::GameTalk`.
## Watch out
- Test fixtures built with `Table::create` + `add_line` carry those adds in the table's undo history: the Top Bar's Undo removes a line. Re-open the saved file (`Table::open`) for a clean history.
- A kit popover (More) left open by a toggle row takes the next click: `common::talk_click` presses Escape after it.
- `Response::widget_info` on a multiline TextEdit replaces its role and value: name it with `ctx.accesskit_node_builder(id, |n| n.set_label(..))` instead.
- Enter or a double click in the list asks String's Text for the keyboard: String is brought to the front (`bring_string`: `ShowPanel` docked, `window_rights` floating) and the request is dropped after 3 passes if never drawn.
- `mg refs` takes no StrRef: the References panel was checked against `mg unpack` + `mg gff` dumps of the fixture by hand.
