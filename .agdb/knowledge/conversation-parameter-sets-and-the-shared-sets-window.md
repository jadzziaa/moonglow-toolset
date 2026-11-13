---
type: Decision
title: Conversation parameter sets and the shared sets window
tags: []
status: stable
generated: { by: trail, at: 2026-11-03T00:00:00Z }
touches: [crates/mg-ui/src/param_sets.rs, "crates/mg-ui/src/widgets.rs#sets_window", "crates/mg-ui/src/dialog_view.rs#script_with_params", crates/mg-ui/src/saved.rs]
created: 2026-11-03
author: August
description: "Variable and parameter sets share one Add Set menu, name row and Manage Sets window via a SetKind const; adding a parameter set is one two-field Undo step, with UI-test pitfalls around Delete buttons and focus-commit."
asks:
  - "How do I add a new kind of saved set without copying the Manage Sets window?"
  - "Why does Undo after clicking Add Set remove only the set and leave my typed parameter value?"
  - "My conversation UI test clicks the wrong Delete button; how do I target the one in the set's row?"
  - "How is adding a parameter set made a single undoable command that merges into the frame's pairs?"
---
decided: Saved sets of a kind (variable sets, parameter sets) share one Add Set menu (widgets::sets_menu), one Save Set name row (set_name_row) and one Manage Sets window (sets_window), driven by a SetKind const (VARIABLE_SETS, PARAMETER_SETS: shelf, title, load, describe); the files' read cache is Shelf::read_all with saved::Read<T>. A new kind of set adds a SetKind, not a copy of the window. decided: Adding a parameter set is one Command of two SetFields (the Script/Active field and the params list) so one Undo takes both back; it merges into the frame's pairs (the typed buffer included), then drops the buffer so a focused field can't put the old pairs back. watch out: In conversation UI tests the editor's toolbar has its own Delete and the editor its own Cancel: get_all_by_label("Delete").next() hits the toolbar's (the blueprint tests' pattern doesn't carry over); pick the button in the set's row by its y. watch out: Clicking Add Set takes focus from a parameter field, so its buffer commits as its own step before the set is added: Undo after adding takes back the set only.
