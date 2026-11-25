---
type: Decision
title: "Owner's rulings of 2026-11-25 on the UI/UX review: script close asks, pane sizes for every kind, gallery rim outside"
description: "The UI/UX review rulings that reverse written designs (script tab close, pane sizes scripts-only, gallery rim inside) and the two that keep or narrow behavior, with where each is recorded."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T08:44:34Z }
touches: ["crates/mg-ui/src/side_doc.rs:may_close_side", "crates/mg-ui/src/shell.rs:window_kept", crates/mg-uikit/src/gallery.rs, crates/mg-uikit/src/controls.rs, crates/mg-ui/src/area_view.rs]
asks:
  - "Should closing a script tab with unsaved text keep it for the module's Save or ask?"
  - Are floating window pane sizes kept for scripts only?
  - "Is the gallery tile's rim drawn inside or outside the tile?"
  - "Why are the light switch-off and slider-ring colors under 3:1 left as they are?"
fingerprint: v2-ebd5359480e148ef
created: 2026-11-25
author: augustaugur
---
## Decided (the owner, 2026-11-25)
1. **Closing a script tab with unsaved text asks Save, Don't Save or Cancel** (its ×, Ctrl+W, the tab menu, its window's close, its Cancel). Reverses the design of 2026-10-29, where a script's × kept the text for the module's Save and recovery. A clean close keeps the editor's state (bookmarks, history, codepage, external editor link) for the session; Don't Save and the question's Save drop it. The talk table is unchanged: it asks only for lines changed in its own editor.
2. **Pane sizes are kept for every window kind**, not scripts only. Reverses 5.2.0 W5 (`KEPT_WINDOW_KINDS`, removed).
3. **The gallery tile's rim is drawn outside the tile.** Reverses docs/ui-redesign-proposal.md §8.4 (inside), now amended.
4. **The light theme's switch-off track and slider-thumb ring stay as the proposal's §4.2 has them**, though both miss 3:1.
5. **Paint with nothing paintable chosen swallows only the placing click**, not all input.

## Where recorded
docs/ui-ux-review-2026-11-25.md (findings, rulings, fixes); dated notes in docs/log.md's 2026-10-29 and 2026-11-23 entries; the manual's Scripts and Main window chapters.

## Watch out
Don't restore the older designs from the log or the old notes: these supersede them.
