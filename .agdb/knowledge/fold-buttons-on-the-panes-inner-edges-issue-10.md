---
type: Decision
title: "Fold buttons on the panes' inner edges (issue 10)"
tags: []
status: stable
generated: { by: trail, at: 2026-10-23T00:00:00Z }
touches: [crates/mg-ui/src/lib.rs, crates/mg-ui/src/widgets.rs, crates/mg-ui/src/tabs.rs]
fingerprint: v2-d112f06728cb834a
created: 2026-10-23
author: August
description: "Each side pane folds via a chevron in a PANE_EDGE strip (widgets::pane_fold_button): tree and log use an always-present egui Panel, palettes a dock-leaf overlay, with egui and egui_dock 0.21 workarounds for sizing and the collapse arrow."
asks:
  - "How is the fold chevron for the module tree and log panes implemented in egui?"
  - "My egui Panel::exact_size strip is narrower than the overlay button area; why don't the sizes match?"
  - "How can I hide egui_dock 0.21's leaf collapse arrow on a main-surface leaf when only a global setting exists?"
  - "Why not draw the tree's fold chevron inside the tree panel, and what went wrong with that approach?"
---
decided: each pane beside the middle folds by one chevron in a strip along its inner edge (widgets::pane_fold_button, PANE_EDGE 16). For the module tree and the log the strip is an always-present egui Panel of its own, shown right after the content panel (Moonglow::pane_edge): folded and shown are the same code path and the same rect, and the button, registered after the content panel's resize handle, wins the clicks where they overlap. The palettes are a dock leaf, not a panel: their button is drawn after DockArea::show_inside on the leaf's edge (palettes_edge), and the leaf's tabs widen their margin on that side (palette_edge, read in Viewer::ui). Folded, their strip is a Panel on the side Place::Split says (left for Split(Left), else right).
watch out: egui Panel::exact_size counts the separator line (reserved in the outer margin), so a strip whose content is PANE_EDGE wide needs PANE_EDGE + noninteractive.bg_stroke.width.round(); the overlay and the strip then measure the same.
watch out: egui_dock 0.21 has only a global show_leaf_collapse_buttons; windows fold to their bar by that arrow (manual, remember_window_sizes reads leaf.collapsed), so it stays on. A main-surface leaf's arrow is removed by painting extreme_bg_color over the leftmost 24 pt of its tab bar (Style::TAB_COLLAPSE_BUTTON_SIZE is pub(crate)) and ui.interact(Sense::click) there after the dock: the later widget in the same layer takes the click. Keyboard focus still reaches the covered arrow.
ruled out: drawing the tree's chevron inside the tree panel (over its scroll bar, and under the resize handle, which egui interacts after the contents).
