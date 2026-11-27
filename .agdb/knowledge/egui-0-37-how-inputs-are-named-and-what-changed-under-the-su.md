---
type: Decision
title: "egui 0.37: how inputs are named, and what changed under the suite"
description: "Why mg_uikit::FieldLabels names inputs by geometry (and its rules), and the egui 0.37 behavior changes the suite and its tests had to follow."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T02:27:41Z }
touches: [crates/mg-uikit/src/labels.rs, "crates/mg-ui/src/widgets.rs:labelled_by_its_label", "crates/mg-ui/src/blueprint/mod.rs:room_right", vendor/egui_dock/MOONGLOW.md]
asks:
  - "Why does a kittest test fail with 'widget(s) lack accessible name'?"
  - "How does a form's field get named by its label for a screen reader?"
  - "Why does get_by_label find the field and not the label, or find two nodes?"
  - "What changed in egui 0.37 that broke menus, scrolls or grids in the tests?"
fingerprint: v2-7766bfb13734621b
created: 2026-11-27
author: augustaugur
---
-
