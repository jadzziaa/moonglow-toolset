---
type: Gotcha
title: "dialog_pattern::SHARED_MAKERS enforces shared dialog makers and callers by source scan"
description: "dialog_pattern tests scan mg-ui source text against SHARED_MAKERS, so refactoring shared dialog helpers must update SHARED_MAKERS."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T23:34:33Z }
touches: ["crates/mg-ui/tests/app/dialog_pattern.rs:SHARED_MAKERS", crates/mg-ui/src/blueprint/mod.rs]
asks:
  - Why do dialog_pattern tests fail after extracting or renaming a shared dialog helper in blueprint or editor views?
  - What does SHARED_MAKERS check in mg-ui tests?
  - Where must callers be registered when consolidating shared dialog-opening code?
fingerprint: v2-224e64856704f93b
created: 2026-11-23
author: augustaugur
---
`tests/app/dialog_pattern.rs` scans `crates/mg-ui/src/` source text to verify every shared dialog maker and its caller list in `SHARED_MAKERS`. Whenever extracting, renaming, or moving shared dialog-opening helpers (for example `open_loc_edit_on_click` in `blueprint/mod.rs`), update `SHARED_MAKERS` in `dialog_pattern.rs` to match, or the dialog-pattern tests fail on unrecognized or missing callers.
