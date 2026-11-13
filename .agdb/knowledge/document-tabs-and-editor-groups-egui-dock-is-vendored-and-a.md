---
type: Decision
title: "Document tabs and editor groups: egui_dock is vendored, and a tab's unsaved dot is compared only once the document rests"
description: "Why egui_dock 0.21.1 is vendored with additions (fold arrow, close-all, Split Editor end room) and how a tab's unsaved dot is compared with the saved copy (differs_from_saved, 200 ms rest) instead of on every change."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T20:58:08Z }
touches: [vendor/egui_dock/MOONGLOW.md, crates/mg-ui/tests/app/doc_tabs.rs]
asks:
  - Why is egui_dock vendored instead of using the published crate?
  - "When does a tab's unsaved-changes dot compare the document with its saved copy, and why not on every change?"
  - What does Split Editor do with one editor group versus two or more?
  - "Why did a 2DA window stay 'under way' and a settle test fail now and then?"
fingerprint: v2-70b9a6f31c41c5df
created: 2026-11-13
---
## Decided
- egui_dock 0.21.1 is vendored with two small additions instead of worked around. The published crate draws the fold arrow and the close-all button on every tab bar or on none, and the approved boards draw them on floating windows only, with Split Editor at the end of the editor groups' bars. A window's fold goes through crate-private state, so the application cannot draw that arrow itself. The kit (which Moonglow Viewer builds by git tag against the published crate) must never call the additions.
- A tab's dot for unsaved changes is true to the saved module (an edit undone loses it), for areas and blueprints too. It shows at once on a change and the exact comparison with the saved copy runs only after the document has rested (200 ms without a change, no button held): a drag changes an area every frame and the comparison costs about 1 ms on the largest GIT.
- A tool's editor group left alone takes the whole editor area, as any group does; the placeholder shows only once no tab is left. This settled the open item of the GitHub issues 11 and 20 entry by decision.
- Split Editor with one group makes a second; with two or more it moves the tab into the neighboring group. Its tip says "another editor group" so it is true in both cases.

## Tried and ruled out
- Comparing a document with its saved copy by writing it out and reading the saved bytes on every change: 3.2 ms on a 1.3 MB GIT. The saved copy is kept parsed instead, and let go of on save, on its resource's removal and when no tab shows it.
- Keying that kept copy by the address of the saved bytes: a freed address can be reused and match a stale copy. The copy holds the bytes' Arc and is matched with Arc::ptr_eq.
- Blaming the dot for area092's edit frame over the 6.94 ms budget: the parent commit measured 7.2 to 7.9 ms on the same loaded machine, so it was over before.

## Watch out
- A background look a view starts must be collected whether or not its tab is drawn. A 2DA's look for its description was collected only while its tab was in front, so a tab sent behind another while the folder walk ran left the window "under way" for good, and a UI test waiting with `settle` failed now and then on a slow first run.
- `settle` never ends beside an animating area view in the screens cases: wait for what the picture needs with a frame-bounded helper there.
- A test cannot run an older release's reader: where a record must stay readable by one, name the test for what it runs and write in its comment what was only read in the old code.
- An implementer's own report misses faults of this kind; an independent review of the commit finds them.

In the code: `vendor/egui_dock/MOONGLOW.md`, `DockArea::main_surface_end_room`, `TabViewer::on_tab_dragged`; `mg_edit::Workspace::differs_from_saved`, `differs_from_saved_at_once`, `forget_saved_copies`; `tabs::prepare`, `ResourceStamp`; `Moonglow::dot_comparisons` (test counter), `split_editor`, `editor_groups`, `focused_group`; `table_view::follow_looks`; tests in `crates/mg-ui/tests/app/doc_tabs.rs`.
