---
type: Gotcha
title: "app.scripts is not 'open scripts': a clean buffer stays after its tab closes"
description: "Moonglow::scripts keeps a clean script's editor state for the session after its tab closes; use script_open for 'open', and know when kept buffers are dropped."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T09:38:44Z }
touches: [crates/mg-ui/src/script_view.rs, crates/mg-ui/src/commands.rs]
asks:
  - Why did Replace All change a script whose tab was closed?
  - Does app.scripts.contains_key mean the script is open?
  - "When is a closed script's editor state dropped?"
fingerprint: v2-a2f50a9c5ec8a213
created: 2026-11-25
author: augustaugur
---
## Decided
`app.scripts` holds editor state, not open scripts: a clean script closed with Close keeps its buffer for the session (bookmarks, Undo, codepage, External Editor link). "Open" is `Moonglow::script_open(key)` (`dock.find_tab(&Tab::Script(key))`). Use it in Find/Replace in open scripts, Edit > Undo, `script_in_front`. Not usable inside the DockArea pass, where `app.dock` is empty.

## Dropped when
- Rename moves the buffer with the script.
- Module Undo drops every clean buffer.
- `forget_closed` drops clean buffers of scripts the module no longer has.
- Don't Save / the question's Save drop it at close.

## Tried and ruled out
Treating `scripts.contains_key` as open: Replace All changed a closed script's buffer, which then showed unsaved with no tab to ask, and the module's Save wrote it; "the only one open" picked a script with no tab.
