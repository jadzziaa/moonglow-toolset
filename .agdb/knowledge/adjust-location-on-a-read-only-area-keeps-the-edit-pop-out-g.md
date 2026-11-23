---
type: Decision
title: "Adjust Location on a read-only area keeps the edit; Pop Out goes through properties_panel::pop_out"
description: "Why Adjust Location never auto-saves in a read-only area, when it refreshes its numbers, and the one function every way of opening an object's Properties in a tab goes through to close the docked pane."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T02:19:59Z }
touches: [crates/mg-ui/src/area_tools.rs, "crates/mg-ui/src/properties_panel.rs:pop_out"]
asks:
  - Why did Adjust Location drop an edit silently in a claimed area?
  - Why are OK and Apply off in Adjust Location?
  - Which actions close the docked Properties pane?
  - When does Adjust Location refresh its numbers from the area view?
fingerprint: v2-7c7aa599ac3d318a
created: 2026-11-23
author: augustaugur
---
Adjust Location's auto-save (selection change, click away) and OK/Apply push Action::Apply, which claims::gate refuses in an area another builder holds (project with others = "read-only"); the window then cleared its changes and sync_live put the old numbers back, so the edit vanished without a word. adjust_window now reads claims::read_only(app, a.area): nothing is applied there, OK and Apply are off, a warning note says "<why>: read-only here, so what you change is not saved.", and a selection change while an edit is pending keeps the dialog on the object it was editing (the edit kept until Cancel). The Module Tree's and Outliner's object menus leave Adjust Location… out there (tree::contents_ui's read_only), as the area view's read-only menu does. Ruling of F1 (5.1.0 review): not retargeting was chosen over dropping the edit on a selection change.

sync_live (the clean dialog following the object) runs only when the area view's model revision (AreaView::revision) differs from AdjustWinState::synced: a drag is drawn from the view's posed copy (AreaView::posed) and reaches the model only once let go and written, so refreshing per model revision loses no live update (test adjust_location_follows_a_drag_in_the_view). It used to rebuild adjust(view) every frame.

properties_panel::pop_out(app, tab) opens an Instance/Instances tab and marks its area in props_popped_out (the docked Properties pane closes while it is open; sync_props_popped_out drops the mark when the tab closes, at the end of the action batch). Open in Tab, area_view::open_properties (the view's right-click › Properties and the Select Tiles double click) and tree::act_on_object's Asked::Properties (the Module Tree's and Outliner's menus and the triple click on a row) all go through it. Pop In no longer removes the mark itself: sync_props_popped_out does, before the next frame where show_dock_panel's request is met.
