---
type: Decision
title: "egui 0.37: how inputs are named, and what changed under the suite"
description: "Why mg_uikit::FieldLabels names inputs by geometry (and its rules), and the egui 0.37 behavior changes the suite and its tests had to follow."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T03:01:04Z }
touches: [crates/mg-uikit/src/labels.rs, "crates/mg-ui/src/widgets.rs:labelled_by_its_label", "crates/mg-ui/src/blueprint/mod.rs:room_right", "crates/mg-ui/src/search_results.rs:form_frame", vendor/egui_dock/MOONGLOW.md]
asks:
  - "Why does a kittest test fail with 'widget(s) lack accessible name'?"
  - "How does a form's field get named by its label for a screen reader?"
  - "Why does get_by_label find the field and not the label, or find two nodes?"
  - "What changed in egui 0.37 that broke menus, scrolls, combo boxes or grids?"
fingerprint: v2-3b9fb87480044c4f
created: 2026-11-27
author: augustaugur
---
egui 0.37 (2026-11-26, owner-approved for accesskit_unix 0.24, which watches at-spi's IsEnabled) names a widget only by what it is told (labelled_by, accessible_name). kittest 0.37 runs an a11y check on every step and fails on any visible, non-read-only input with no label, labelled_by or placeholder; it also fails on glyphs no font has.

Decision: name inputs by geometry in one place, mg_uikit::FieldLabels (crates/mg-uikit/src/labels.rs), an egui Plugin that theme::apply installs. Tests building their own Harness add .with_context_setup(mg_uikit::FieldLabels::install). It runs at on_end_pass only while the AccessKit tree is read (has_accesskit_node(root)), so it costs nothing otherwise. Rules, each from a failure seen:
- leftward along the row through other fields (BETWEEN 60) to a Label with words (BESIDE 200); else upward along the column at the field's left edge (UNDER 12, UNDER_SIDE 8) to a label: a narrow form's label, a table's heading.
- "beside" judged by the shown (interact, clipped) rects: a row scrolled under a bar is not beside the bar's buttons; where either is out of sight, both must be clipped. The column walk has no visibility filter (a field scrolled out under a label still in sight).
- kept: a field keeps last pass's label while it is drawn. A Grid lays rows out by the previous pass's heights, so a label stands a pass off its field when the form changes.
- new: a field new in the pass may take a label one field height below its row. A new Grid's first pass is an invisible, discarded sizing pass, so its rows sit at those sizes the pass after.
- invisible widgets (WidgetRect.visible false) are skipped; duplicate labelled_by entries are deduplicated (Response::labelled_by pushes, so a field could be read "Into Into").
- ctx.has_accesskit_node inside ctx.viewport(..) re-enters the lock (SIGABRT): collect first, then query.
mg-ui's side: widgets::field_label records the last label with words; labelled_by_its_label links the first field after it (commit_field, Dropdown::named, RowChoice, spin_with, NUI scalar). name_number_field names a number field without a visible label (its arrows "X: One More"). name_node(ui, id from ui.next_auto_id() taken before drawing, name) names a field whose place alone says what it is (table cells, a group's tile grid, door numbers, array items).
Test consequences: get_by_label drops a Label node that labels another, so get_by_label("Hit Points") now finds the field; use common::label_node / label_nodes (include_labels) for the label itself.

Other egui 0.37 changes the suite followed:
- a menu hangs under the widget it opened from in the tree: find items by a Menu ancestor (common::in_menu, screens.rs popup_item), not by depth.
- Id::new deprecated (Id::unique or IdSalt::new; AsId -> AsIdSalt); AtomLayout -> WidgetAtom.
- kinetic scroll rubber band turned off in theme (scrolls placed by code sprang back).
- ui.max_rect() can reach past a window's visible edge: blueprint::room_right caps it at clip_rect (the Sound set row ran out of a narrow window).
- a ComboBox grows to fill a justified layout (a ui.columns column): widgets::combo_looks draws it inside ui.horizontal there, as 0.36 sized it.
- a ScrollArea whose bar takes room rewraps its content; where that height sizes the scroll's container (the Bottom Dock's least height from search_results::form_frame) it looped every frame: the form is laid out as wide as with the bar.
- eframe's system_fonts gives a system fallback font (fontconfig dlopen); egui needs default_fonts + monochrome_emoji_fonts.
- egui_dock 0.22 is vendored with a sixth patch naming its separators (AccessibilityTranslations::separator).
