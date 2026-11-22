---
type: Gotcha
title: "Focus Selection Group::Area key command and Outliner dot shortcut handling"
description: "Explains why Cmd::FocusSelection must stay in Group::Area without a commands::Id and how pressed_but plus typed_row let '.' work straight from the Outliner without breaking letter type-ahead."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T16:03:00Z }
touches: [crates/mg-ui/src/keys.rs, crates/mg-ui/src/tree.rs, crates/mg-ui/src/area_view.rs]
asks:
  - "Why is Cmd::FocusSelection Group::Area with no entry in commands::Id?"
  - "How do F and '.' behave differently when focus or keys are in the Outliner or Palette?"
  - "Why does typed_row skip leading '.' characters when no multi-character search is active?"
fingerprint: v2-554fdf9d4615456e
created: 2026-11-22
author: august
---
commands::tests::the_keys_that_work_anywhere_are_all_commands asserts that every Group::General Cmd has a matching commands::Id and no non-General Cmd shares an Id; therefore area-only FocusSelection (bound to F and '.') stays Group::Area without a commands::Id (invoked in camera_input and the area context menu). In Keymap::pressed_but, plain single ASCII-alphanumeric keys (F) yield while Taken.letters is active and typed_row consumes letters for list type-ahead, whereas Key::Period ('.') is exempt and typed_row ignores leading '.' unless a multi-character type-ahead string is already in progress. camera_input evaluates focus_active with is_none_or(|a| a == view.area) so pressing '.' right after clicking an Outliner row focuses even before hovering the 3D viewport.
