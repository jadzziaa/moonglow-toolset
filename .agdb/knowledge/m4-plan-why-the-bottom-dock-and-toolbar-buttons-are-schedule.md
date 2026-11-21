---
type: Decision
title: "M4 plan: why the Bottom Dock and Toolbar buttons are scheduled where they are"
description: "Ordering constraints of the UI redesign's M4 plan (U0-U30) that the code and the proposal's order paragraph do not show: Problems/Search Results panels are needed before the dialogs-to-panels step, Toolbar buttons can only retire after its last reader, and floating-mode tests keep most editor tests valid."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T22:11:19Z }
touches: [docs/ui-redesign-proposal.md, crates/mg-ui/src/shell.rs, crates/mg-ui/src/icons.rs]
asks:
  - "Why does the M4 plan build the Bottom Dock's multi-panel support in U2 instead of with the dialogs that become panels?"
  - When can the toolbar_icons_only setting be removed?
  - Why do most editor UI tests survive moving editor panes into the docks?
  - "Is a placed object's Properties panel a new way of editing?"
fingerprint: v2-16b25bb5d1b2da36
created: 2026-11-14
---
## Decided (planning M4, 2026-11-14; docs/ui-redesign-proposal.md, M4 "Planned in detail")
- The proposal's order paragraph puts "the dialogs that become panels" last, but the script editor's Search Results (U13) and the tileset editor's Check -> Problems (U20) need those Bottom Dock panels earlier. So U2 generalizes the registry (Dock::Bottom, sections, N Status Bar switches with badges) with no visible change, each global panel arrives with its first feeder (no switch without a panel, M3's S3), and U28 only moves the dialogs into them.
- `toolbar_icons_only` is read by six toolbars (Tools::of in script_view, dialog_view, model_view, nui_view, hak_view; Tools::of_area in area_view). Each editor task stops reading it for its own toolbar; the field, its Options control and draft go in U23, after the hak editor (U21), the last reader. A retired field in an old app.ron reads silently (test at settings.rs ~2000-2017), so no migration code is needed.
- Tests run As Floating Windows (Settings::default) and a floating editor keeps its panels in its own window (M3 decision 3), so each panel body is one function with two hosts (dock when docked and in front, the editor's own pane when floating); most editor tests keep passing, each task adds As Tabs tests.
- A placed object's Properties already apply at once: blueprint::ok_cancel's contract says changes are the module's already and Cancel undoes them as one step. The Right Dock Properties panel (S2) is edit_many's body without OK/Cancel.
## Watch out
- One scroll for blueprint pages (U9) puts every page name in the page list and the page bars: about 130 test lines click a page by name; draw only pages in sight and give tests a `common::page` helper.
