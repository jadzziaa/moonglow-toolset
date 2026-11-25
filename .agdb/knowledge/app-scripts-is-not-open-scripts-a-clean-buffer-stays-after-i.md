---
type: Gotcha
title: "app.scripts is not 'open scripts': a clean buffer stays after its tab closes"
description: "Moonglow::scripts keeps a clean script's editor state for the session after its tab closes (Close, the question's Save, the header's OK); use script_open for 'open', and know when kept buffers are dropped."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T10:40:45Z }
touches: [crates/mg-ui/src/script_view.rs, crates/mg-ui/src/commands.rs, "crates/mg-ui/src/side_doc.rs:script_kept_on_close"]
asks:
  - Why did Replace All change a script whose tab was closed?
  - Does app.scripts.contains_key mean the script is open?
  - "When is a closed script's editor state dropped, and does the closing question's Save keep it?"
fingerprint: v2-e177c1cd089d252d
created: 2026-11-25
author: augustaugur
---
## Decided
`app.scripts` holds editor state, not open scripts: a clean script closed (Close, the closing question's Save, the header's OK) keeps its buffer for the session (bookmarks, Undo, codepage, External Editor link). "Open" is `Moonglow::script_open(key)` (`dock.find_tab(&Tab::Script(key))`). Use it in Find/Replace in open scripts, Edit > Undo, `script_in_front`. Not usable inside the DockArea pass, where `app.dock` is empty. Invariant: a buffer with no tab is never dirty.

## Dropped when
- Rename moves the buffer with the script.
- Module Undo drops every clean buffer.
- `forget_closed` drops clean buffers of scripts the module no longer has.
- Don't Save drops it at close (the question's Save keeps it, by the owner's ruling of 2026-11-25: `side_doc::script_kept_on_close`). The header's Cancel on a clean script keeps it too (the owner's later ruling the same day; `may_cancel` checks `script_kept_on_close`).

## Tried and ruled out
Treating `scripts.contains_key` as open: Replace All changed a closed script's buffer, which then showed unsaved with no tab to ask, and the module's Save wrote it; "the only one open" picked a script with no tab.
