---
type: Decision
title: "Edit Copy's Cancel and what Aurora's windows drop"
tags: []
status: stable
generated: { by: trail, at: 2026-11-04T00:00:00Z }
touches: [crates/mg-ui/src/blueprint/mod.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/area_view.rs, crates/mg-ui/src/copy_as.rs, crates/mg-ui/tests/app/blueprints.rs, crates/mg-ui/tests/app/fields.rs, crates/mg-ui/tests/app/palettes.rs, docs/log.md, docs/manual/04-areas.md, docs/manual/05-blueprints.md]
fingerprint: v2-53ffe3ab84fede9c
created: 2026-11-04
author: August
description: "In Aurora 89.8193.37, Edit Copy and Add to Palette write the blueprint only on OK, so Cancel, Escape or close leaves nothing; Moonglow makes the copy immediately and undoes it on Cancel, with a pitfall around properties_changed and OpenTab."
asks:
  - "What happens in Aurora if I cancel the Properties window after choosing Edit Copy or Add to Palette?"
  - "Why did my cancelled Moonglow copy stay behind, and how does blueprint::note_made undo it on Cancel?"
  - "Why is a properties_changed entry lost when it is noted in the same frame that pushes OpenTab?"
  - "Does cancelling Add to Palette reset the whole area's object list or only the placed object?"
---
decided: Aurora 89.8193.37 (captures/edit-copy-cancel/, README.txt): Edit Copy and Add to Palette open the new blueprint's Properties and write it only at OK; Cancel, Escape and the window's close button (WM_DELETE_WINDOW, wmclose.py there: KWin's virtual session draws no title bars on Aurora's dialogs) leave nothing, the placed object keeping its template. A wizard's blueprint is written at Finish and stays when its Properties are cancelled. An item's added property shows Total Cost and Lore at once; Cancel, Escape and the close button all drop it. Moonglow keeps making the copy at once and notes it for the window's Cancel (blueprint::note_made: Was::Resource(key, None), SetResource None removes); its tab x still keeps changes (owner's call).
watch out: Moonglow::ui drops properties_changed entries whose tab isn't in the dock BEFORE run_actions, so an entry noted in the frame that pushes OpenTab is lost; WindowChanges.opening keeps it while an OpenTab is queued, until the window draws. Keeping every entry with a queued OpenTab broke nui.rs cancel_after_a_build_leaves_a_window_that_builds (a stale OpenTab from NewNui kept an older entry).
watch out: Add to Palette now sets the placed object's changed fields in place (not remove + insert), so Cancel sets back that object alone, not the area's whole list.
