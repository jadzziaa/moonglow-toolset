---
type: Gotcha
title: NUI verified_clean skips diffing on quiet frames after deferred ctx mutations
description: "nui_view::end skips work diffs when verified_clean is true unless deferred ctx mutations call mark_mutated."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T23:34:33Z }
touches: ["crates/mg-ui/src/nui_view.rs:mark_mutated", "crates/mg-ui/src/nui_view.rs:end", "crates/mg-ui/src/nui_view/images.rs:answer_from_disk", "crates/mg-ui/src/nui_view/panels.rs:bindings"]
asks:
  - Why does a picture loaded from disk or a newly created NUI binding disappear until the next click?
  - Why are deferred ctx temp edits in NUI Creator silently dropped on quiet frames?
  - "When should nui_view::mark_mutated or verified_clean = false be called?"
fingerprint: v2-849376a21fa6e79c
created: 2026-11-23
author: augustaugur
---
`nui_view::end` skips the O(N) `work.window != *was` and `work.settings != *was_settings` comparisons on quiet frames when `work.verified_clean` is true (`can_skip_diff`). Any deferred `ctx` temp consumer or callback that mutates `work` without pointer or key input (such as `images::answer_from_disk` applying a file-picker result or `panels::bindings` consuming `create_asked`) must call `nui_view::mark_mutated(ctx)` or clear `work.verified_clean = false`; otherwise edits are silently dropped until the next user input.
