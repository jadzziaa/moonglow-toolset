---
title: "A command that edits a typed-in field's resource waits for egui's two-frame lost_focus"
author: August
date: 2026-10-29
branch: worktree-agent-a3a960e99c8a34aff
cost: { minutes: 45, tokens: 12738728 }
touches:
  - "crates/mg-ui/src/commands.rs#run_after"
  - "crates/mg-ui/src/outside.rs#take_looked"
  - "crates/mg-ui/src/widgets.rs#leave_text"
---

decided: A command asked for while a text field that sets its text when the keyboard leaves it holds typing (widgets::leave_text) runs a frame on (commands::AfterText). A plugin's command is then begun with the frame's actions (Action::RunPluginCommand), not at keys_pressed, and a look at files changed outside (Moonglow::take_looked, called after the dock is drawn) waits until the field has drawn and its Apply has run.
watch out: egui 0.36's Response::lost_focus is true for TWO frames (id_previous_frame or id_two_frames_ago, memory/mod.rs lost_focus). A field that set its text in frame N sees lost_focus again in N+1, and if anything changed the resource synchronously before it is drawn in N+1 (a plugin job in tests runs at once; poll_job applies a background job's edits before drawing), buf != shown and it sets what was typed AGAIN over that change. Seen as: plugin's 'Upper-case creature tags' applied, then a second 'Tag' undo step reverting it. Run such commands at the end of frame N+1 (actions) or later.
watch out: keys_pressed runs before the panels are drawn, reload_timer after; a surrender_focus at reload_timer is seen by the field only in the next frame, so wait for that frame's actions too (Looking::left_text).
