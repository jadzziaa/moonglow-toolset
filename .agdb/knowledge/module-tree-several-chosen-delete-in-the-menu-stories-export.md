---
type: Decision
title: "Module tree several chosen: Delete in the menu, stories exported"
tags: []
status: stable
generated: { by: trail, at: 2026-11-03T00:00:00Z }
touches: ["crates/mg-ui/src/tree.rs#follow_front_tab", crates/mg-ui/src/dialog_export.rs, "apps/mg/src/main.rs#dialog_export_into"]
created: 2026-11-03
author: August
description: "A report that the multi-select module tree menu lacks Delete did not reproduce, since several_menu has had Delete since 3.0.0; testing found and fixed a focus bug resetting tree_selected via Moonglow::tree_went, plus kittest tips."
asks:
  - "Does the context menu for several selected modules in the tree have Delete, and since which version?"
  - "Why did my multi-selection in the module tree reset to a single tab after I clicked inside an opened tab?"
  - "How do I simulate shift or command clicks and right-clicks in kittest for the module tree without double-click or header problems?"
  - "Which writer exports several conversations together, and does it include unsaved edits?"
---
A builder's 'the menu of several has no Delete' (2026-11-03, on 3.6.0's code) did not reproduce: several_menu has had Delete… since 3.0.0 (d9d78567); 2.13.x's menu of several had Export… and Export as Files… only, which is what the report describes. Driven through real gestures (kittest click_modifiers SHIFT/COMMAND, click_secondary after a hover) for every group it works. What the gestures did find: follow_front_tab took a focus change to a pane as a tab chosen, and a tab opened by OpenTab comes to the front without the dock's focus, so the first click in it (egui_dock sets focus on any click in a leaf body) called reveal_tab and reset tree_selected to that tab's key. Fixed with Moonglow::tree_went (the tab reveal_tab went to last): focus moving to it is no choice. Kittest notes: two node.click() in one frame are no double click (use Action::OpenResource), editors open in windows that dock.iter_all_tabs may not list (use dock.find_tab), clicking two group headers in one frame registers one, Areas is open by default and a tab opened opens its group (check a row before clicking a header), and a module's Module Properties window adds a second Cancel (take the last). Conversations exported together go through dialog_export::text_of, the editor's Export's own writer, from ws.doc (edits not saved included).
