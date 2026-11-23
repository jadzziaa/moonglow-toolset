---
type: Gotcha
title: Adjust Location selection tracking and post-body auto-save ordering
description: Explains why adjust_window stores AdjustWinState.seen_selection and runs selection-change and click-away auto-save after drawing the dialog body.
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T16:03:00Z }
touches: [crates/mg-ui/src/area_tools.rs]
asks:
  - How does Adjust Location auto-save on click-away and follow selection changes without breaking direct app.adjust tests?
  - Why does adjust_window run selection-change and click-away commits after show_foot instead of before?
  - How does adjust_window distinguish a real selection change from objects_moved re-indexing after deletion?
fingerprint: v2-8957066e0631edb4
created: 2026-11-22
author: august
---
adjust_window keeps transient per-window state AdjustWinState { rect, seen_selection } in egui ctx temp data (cleaned up on Cancel/Escape/OK/close) rather than adding fields to public struct AdjustLocation, which several tests construct with struct literals. Selection-change and click-away checks run AFTER show_foot so text_begin + spin_draft lost_focus() commit any typed edit into AdjustLocation before apply_location runs. Checking *cur != prev.seen_selection && (cur.0 != a.start || cur.1 != a.objects) triggers auto-save and retargeting when view selection actually changes while ignoring both tests that set app.adjust without setting view.selection and objects_moved re-indexing both selection and a.objects on deletion.
