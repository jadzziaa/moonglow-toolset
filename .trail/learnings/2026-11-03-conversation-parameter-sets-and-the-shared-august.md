---
title: Conversation parameter sets and the shared sets window
author: August
date: 2026-11-03
branch: worktree-agent-ad3ab1e9ca7386506
cost: { minutes: 7, tokens: 165686 }
touches:
  - crates/mg-ui/src/param_sets.rs
  - "crates/mg-ui/src/widgets.rs#sets_window"
  - "crates/mg-ui/src/dialog_view.rs#script_with_params"
  - crates/mg-ui/src/saved.rs
---

decided: Saved sets of a kind (variable sets, parameter sets) share one Add Set menu (widgets::sets_menu), one Save Set name row (set_name_row) and one Manage Sets window (sets_window), driven by a SetKind const (VARIABLE_SETS, PARAMETER_SETS: shelf, title, load, describe); the files' read cache is Shelf::read_all with saved::Read<T>. A new kind of set adds a SetKind, not a copy of the window. decided: Adding a parameter set is one Command of two SetFields (the Script/Active field and the params list) so one Undo takes both back; it merges into the frame's pairs (the typed buffer included), then drops the buffer so a focused field can't put the old pairs back. watch out: In conversation UI tests the editor's toolbar has its own Delete and the editor its own Cancel: get_all_by_label("Delete").next() hits the toolbar's (the blueprint tests' pattern doesn't carry over); pick the button in the set's row by its y. watch out: Clicking Add Set takes focus from a parameter field, so its buffer commits as its own step before the set is added: Undo after adding takes back the set only.
