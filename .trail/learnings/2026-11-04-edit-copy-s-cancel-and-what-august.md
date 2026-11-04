---
title: "Edit Copy's Cancel and what Aurora's windows drop"
author: August
date: 2026-11-04
branch: worktree-agent-ab2dd028664103b2d
cost: { minutes: 30, tokens: 220769 }
touches:
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/copy_as.rs
  - crates/mg-ui/tests/app/blueprints.rs
  - crates/mg-ui/tests/app/fields.rs
  - crates/mg-ui/tests/app/palettes.rs
  - docs/log.md
  - docs/manual/04-areas.md
  - docs/manual/05-blueprints.md
---

decided: Aurora 89.8193.37 (captures/edit-copy-cancel/, README.txt): Edit Copy and Add to Palette open the new blueprint's Properties and write it only at OK; Cancel, Escape and the window's close button (WM_DELETE_WINDOW, wmclose.py there: KWin's virtual session draws no title bars on Aurora's dialogs) leave nothing, the placed object keeping its template. A wizard's blueprint is written at Finish and stays when its Properties are cancelled. An item's added property shows Total Cost and Lore at once; Cancel, Escape and the close button all drop it. Moonglow keeps making the copy at once and notes it for the window's Cancel (blueprint::note_made: Was::Resource(key, None), SetResource None removes); its tab x still keeps changes (owner's call).
watch out: Moonglow::ui drops properties_changed entries whose tab isn't in the dock BEFORE run_actions, so an entry noted in the frame that pushes OpenTab is lost; WindowChanges.opening keeps it while an OpenTab is queued, until the window draws. Keeping every entry with a queued OpenTab broke nui.rs cancel_after_a_build_leaves_a_window_that_builds (a stale OpenTab from NewNui kept an older entry).
watch out: Add to Palette now sets the placed object's changed fields in place (not remove + insert), so Cancel sets back that object alone, not the area's whole list.
