---
title: "Copy Set and Paste Set: a Paste button's paste"
author: August
date: 2026-11-04
branch: worktree-agent-a3342425f38aec0b7
cost: { minutes: 88, tokens: 931957 }
touches:
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/src/dialog_view.rs
  - crates/mg-ui/src/color_button.rs
---

A Paste button (Paste Set, Paste Color) sends RequestPaste and its text comes as the next frame's Event::Paste. Taking it where the button is drawn is too late in the conversation editor: its toolbar reads Event::Paste first (copy_cut_paste) and pastes copied lines. So widgets::ask_paste records (id, pass), widgets::pastes_begin (lib.rs, first thing each frame, before clipboard_keys) takes the Paste event for that id, and asked_paste hands it to the button; an ask is dropped on a key or mouse press in a later pass (an empty clipboard sends no event, and a later Ctrl+V would be stolen) or after 10 passes. Pass numbers, not frames: egui's second pass of a frame gets no events. Two layout findings: in the dialog editor's columns a plain ui.horizontal row wider than its column (the line's tabs) widens the column's max_rect, so every horizontal_wrapped row below wraps at the widened width and runs past the pane's clip; buttons there are drawn clipped and clicks past the clip don't register (accesskit rects still look fine). Wrap the wide row. And egui's Response::context_menu id is response.id.with("popup"), the same as ui.auto_id_with("popup") taken before the response is allocated: color_button's picker had that id, so its right-click menu never opened; give the context menu its own id (Popup::context_menu(..).id(..)). kittest: click() is hover+press+release in one step, so the click's frame output holds CopyText / RequestPaste; read it after one h.step(), not after settle.
