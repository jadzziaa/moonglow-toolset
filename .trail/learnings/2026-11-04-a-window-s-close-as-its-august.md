---
title: "A window's close as its Cancel (Options)"
author: August
date: 2026-11-04
branch: worktree-agent-a5cbeee3524e7472b
cost: { minutes: 60, tokens: 1407086 }
touches:
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/tests/app/common.rs
---

decided: Options › General close_cancels (off by default): every builder's close of a tab goes through Moonglow::may_close (egui_dock's on_close for the ×, its tab-menu Close and a window's close-all button; Action::CloseTabs for Close Others/Right/All, middle click, Ctrl+W), so blueprint::closing hooks there; OK, Cancel and Escape close by Action::CloseTab and module close / deletion by close_tabs, neither of which calls may_close, so they are never a close-cancel. The window's entry is moved to Moonglow::closing_cancels and cancel_closed runs it at the end of run_actions (after the frame's Applies, also those queued after a CloseTabs in the same pass), with all=false. A held text field of the closing window is dropped. Cancel's per-window extras (NUI event script, factions' numbers) are blueprint::cancel_more by tab, shared with window_drawn_but (which lost its more closure).
watch out: egui_dock calls on_close after every surface is drawn, so the closed window's body has drawn (and noted) that frame; run_actions takes the actions once, so an Action pushed while it runs waits a frame.
watch out: egui_dock's tab × has no accesskit name: tests click it with common::click_tab_close (the click-and-drag widget in the pane's tab bar, the square click widget at its right end). A 24x24 widget left of the first tab is not its close button.
watch out: egui_dock's window and pane close-all buttons call on_close on every tab before removing any, and keep them all when one says Focus (a side editor with unsaved changes): may_close (and closed) run for tabs that stay open, so cancel_closed puts back and skips an entry whose tab is still in the dock.
