---
type: Decision
title: "Area Right Dock layout, ultrawide auto-hide, and docked_instance_tabs snapshot"
description: "Explains how AreaRightLayout rearranges Palette/Preview/Properties, how ultrawide auto-hide reclaims column width, and why docked_instance_tabs snapshots main-surface tabs before DockArea draw."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T16:03:00Z }
touches: [crates/mg-ui/src/shell.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/settings.rs, crates/mg-ui/src/properties_panel.rs]
asks:
  - "How does Moonglow rearrange Palette, Preview, and Properties in the Right Dock?"
  - "Why does Moonglow::docked_instance_tabs snapshot Tab::Instance tabs before drawing DockArea?"
  - How does ultrawide auto-hide reclaim Right Dock column width when nothing editable is selected?
  - Why is the Hide Properties close button placed beside Open in Tab while Fold the Right Dock stays at the outer right column?
fingerprint: v2-bf9dd52a6114b5d8
created: 2026-11-22
author: august
---
Moonglow stores Right Dock pane arrangement in Settings::area_right_layout (AreaRightLayout with Presets TwoColumns, Stacked, OneTabBar, per-pane moves, and Reset) and renders via draggable DockTabs and PanelHeader in egui/mg-uikit. On ultrawide windows (>= 2560), area_properties_shown hides docked Properties whenever properties_bar_shown is false (nothing editable selected, popped out into a tab/window, or hidden via View > Properties Panel / Hide Properties button), causing arrange_area to drop the second column and shrink right_dock_width to Settings::right_dock_width_wide_one (or RIGHT_DOCK_WIDE_SINGLE). During frame draw, lib.rs moves self.dock out via std::mem::replace so widgets inside docked tabs cannot call app.dock.find_tab directly; docked_instance_tabs snapshots main-surface Instance/Instances tabs before DockArea::show_inside so ok_cancel_header_with can offer Pop In.
