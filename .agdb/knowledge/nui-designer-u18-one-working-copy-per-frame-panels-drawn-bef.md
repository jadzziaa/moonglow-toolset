---
type: Decision
title: "NUI designer U18: one working copy per frame, panels drawn before the editor, choices deferred to the frame's end"
description: "How the NUI designer's docked panels (Components, Properties, Bindings, Layers) edit one shared copy of the window with its editor, and why Layers' and Properties' choices wait for the frame's end."
tags: [nui]
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-17T16:35:11Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-17T19:52:46Z }
  - { by: agdb/0.2.0, at: 2026-11-17T20:45:44Z }
touches: ["crates/mg-ui/src/nui_view.rs:begin", "crates/mg-ui/src/nui_view.rs:end", crates/mg-ui/src/nui_view/panels.rs, "crates/mg-ui/src/nui_view/fields.rs:recover"]
asks:
  - why does a click in Layers not change the selection until the frame ends
  - "how do the NUI designer's dock panels and its editor share the window being edited"
  - "where is a NUI field's typed text set when its panel is folded or the page changes"
  - why does typed text vanish when a NUI window changes from elsewhere
  - do the NUI panels stay on the Bindings page and in Preview only
fingerprint: v2-2f9bcce266ac0928
created: 2026-11-17
---
M4's U18 moved the NUI designer's palette, Layers and Properties out of its tab into the docks (Right Dock Components/Properties/Bindings, Outliner Layers). The docks are drawn BEFORE the editor area (lib.rs: left_dock, right_dock, then the DockArea), and in a floating window shell::window_panes draws the right pane, then the left (Layers), then the body.

- One working copy (`nui_view::Work`) per designer, kept in `State` while the module holds the same bytes (`Held` pointers): `check()` parses only on new bytes and clones the parse into `work` once (counted by `State::copies`, `Moonglow::nui_copies`). Every part edits `work` in place: no copy in a frame that changes nothing.
- `begin()` (any part, first in the pass) empties the fields' per-designer stores, applies `select_next`, swaps a variant into `work.window["root"]` (taken out of the settings with `mem::take`, never cloned; on every page, the pages but Design putting the window's own root back while they draw: `main_root`) and prunes the selection. `end()` runs once a pass: picks, `structure::finish`, `finish_insert`, unswap, `fields::recover`, `put_back`, serialize and one `Action::Apply` at the editor's `asked` index. `ui()` calls it after `outliner::over` (the floating strip's Layers are drawn after the editor) and before `window_drawn_but`, so Cancel notes panel edits. `nui_view::follow` (after the DockArea) ends frames whose editor was not drawn.
- Layers', Properties' (swap-layout buttons) and the variants bar's selection changes are snapshotted (`Chosen`) and done at `end()` (`Picked`), so the editor's canvas and Split source show the variant swap the frame began with. Typed text does NOT need it: egui ends a field's focus on the pointer's press and rows/buttons act on the release, so a Properties field sets its text a frame before a Layers click chooses (breaks taking out the deferral, `recover` and an Undo-after-Apply reorder did not fail `typed_text_keeps_to_its_own_control`; the reorder and an `end_typing` were removed).
- Field stores (`nui-fields-unset/drawn/of`) are keyed by designer (`fields::enter`), or two designers in one frame cleared each other's unset values and typed text was written at every key.
- A field not drawn the next frame (dock folded, another tab of the one bar, page switched) is set from what `put_back` took out last frame, by JSON pointer (`fields::Reverted`, `recover`): this one a break does fail.
- The canvas's keys are the active designer's only (`table_view::active_tab`).
- Panels are brought on every page and in every Layout (the review's ruling; `panels::brings` is gone): no jump in the editor's width. On the Bindings page the Bindings panel shows a pointer line, not a second copy of the page's fields.
- Bytes changed from elsewhere under a field typed in (`State::copies` moved while `reverted` is non-empty): `begin` clears `reverted` and bumps `State::typing`, part of the fields' id salt (`design::inspector`), so egui drops the focus without setting; the typed text lands nowhere (the tip wrote it on whatever control took the path).
- A designer closed while a field is typed in: `follow` runs one more `end` for it (its tab gone, `reverted` non-empty, the module's held bytes still the ones it was typed over), so what was typed is set; header Cancel sets nothing (the press ended the field before Cancel reverted). Save calls `nui_view::store_typed` (from `Moonglow::save`): what a typed-in field shows is applied first, the field keeping the keyboard.
- The Bindings panel works on the window as kept (`main_root` around it), or a rename with a variant edited missed the main root held in `State::swapped`.
- Tests must not trust `tab_rect` for an editor's size (it is the tab's title, 102x28): measure the header record's row (`mg_uikit::header::records(..).row`).
- The kit's `segmented` named its RadioGroup on `ui.response()` (last frame's rect): in a floating window moved since, header_holds saw the group 14 points off; it now names `inner.response`.
