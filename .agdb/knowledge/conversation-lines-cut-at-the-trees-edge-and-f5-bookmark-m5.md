---
type: Decision
title: "Conversation lines cut at the tree's edge and F5 Bookmark (M5 W4, rulings 29 and 28)"
description: "How the conversation tree cuts long lines, caps deep indents, caches cuts and where Toggle Bookmark (F5) lives, with the traps met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T15:38:10Z }
touches: [crates/mg-ui/src/dialog_view.rs, crates/mg-ui/src/keys.rs, crates/mg-ui/src/commands.rs]
asks:
  - why does the conversation tree not scroll sideways
  - how are long conversation lines truncated and when is the cut recomputed
  - which key bookmarks a conversation line
  - how does an editor command get into the Command Palette
fingerprint: v2-fb48565e250d88ba
created: 2026-11-21
author: augustaugur
---
## Decided (owner's rulings 29 and 28, 2026-11-21)
- The conversation tree is `ScrollArea::vertical()`. Each row is `line_row`: words cut with `TextWrapping::truncate_at_width` (as the kit), then ` [END DIALOGUE]`, then each script mark as its own label, drawn whole or not at all: the words are cut first down to the least room, then marks drop from the right and are added to the row's accessible name (`label  dropped`) and tip (fix round, owner's ruling). Room = available width minus `RIGHT_ROOM` (the kit's `ROW_PADDING_RIGHT`). The kit's scroll bars are solid (`mg-uikit/src/theme.rs`, `ScrollStyle::solid()`), never over the rows, so no scroll-bar allowance is needed.
- Least text width: `least_line(width, em) = max(LEAST_LINE_EMS (12) * em, width / 3)`; `tree_levels` caps the indent (and the guides) so a deep line keeps it. 12 em alone left deep lines 156 px in a 1000 px tree, hence the third.
- Cuts are cached in `DialogView::cuts` (this frame's and last frame's maps, swapped each frame) keyed by (text, font, italics, floored room); rows out of sight lay out nothing. `cuts_made()` counts layouts for tests. Breaking only the first lookup still passes: the last frame's map serves.
- The whole line: the row's AccessKit name always; its tip where the words are cut or a mark dropped.
- Toggle Bookmark = `Cmd::DialogBookmark` ("dialog-bookmark", Conversation group) on F5: the script's Toggle Bookmark key; the Script and Conversation groups never overlap. Aurora's tree popup Bookmark has no key; the script has no next/previous bookmark keys, so none here.
- Command Palette: `commands::Choice::Editor(Cmd)`, rows from `dialog_view::palette_commands` (only when a dialog is in front), run by setting `view.menu = Some(Asked::Bookmark)` (the ask is not gated by the pointer, the key is).
## Watch out
- The marks need their own AccessKit Label nodes, one per mark (also for rows out of sight): `conversations::conversation_lines_show_their_scripts...` finds them by "if c_seen" and "do a_hello".
- Least-width test expectations are by hand: rows 659 wide at 1000x700 give (659-6)/3 = 217.67; 334 in the least window give 12 em = 156.
- First round's mistake: a `right_room` branch for a floating scroll bar never ran (solid bars everywhere, app and harness); the reason given then ("egui_kittest doesn't float the bar") was wrong. The padding is held by `the_words_end_a_row_s_padding_short_of_the_scroll_bar`, which relays out the painted galley's job at the expected room and compares widths exactly, measured from `DialogView::rows_right()` (the selected row's fill runs to the clip rect, past the rows, so it is no measure of the rows' edge).
- The Status Bar's F5 hint shows where F5 works: `here` (tree has keys or pointer over editor, nothing typed) and a line chosen (`selected.is_some()`; Root is `None`).
- The deep-tree guide test at 1600 hits the level cap; it runs at 1920x1080.
