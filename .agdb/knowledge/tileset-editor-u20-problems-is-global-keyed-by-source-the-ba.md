---
type: Decision
title: "Tileset editor U20: Problems is global, keyed by source; the bar's buttons act after the form; focus a field the frame after a click"
description: "How M4 U20 built the tileset editor's header, table, form, Preview, Textures and Outliner, and Problems as the Bottom Dock's global panel U28 extends, with the traps met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-18T05:08:20Z }
touches: [crates/mg-ui/src/problems.rs, crates/mg-ui/src/tileset_view.rs, crates/mg-ui/src/tileset_view/panels.rs, "crates/mg-ui/src/outliner.rs:tileset"]
asks:
  - How do I feed Problems from Build Module or Verify?
  - "Why does a tileset's finding say changed since the check?"
  - Why is a field asked to take the keyboard a frame late?
  - "Why do the tileset bar's buttons act after the form is drawn?"
fingerprint: v2-7430720cca0747ca
created: 2026-11-18
---
- Problems (`problems.rs`): `app.problems: Problems { sets: Vec<Set>, revision, forward, drawn }`; `Set { source: Source, problems: Vec<Problem>, stale }`; `Problem { severity, at (words), message, go: Option<Go> }`. `replace(source, problems)` replaces a source's findings (errors first); never append. U28: add `Source::Build`/`Source::Verify` and `Go` arms (and a fix on Problem for Verify, Q5). `ask_forward(open)` -> `shell::problems_forward` at frame start (U13's search rule: anything found opens the main window's dock; nothing found opens nothing). Global panel: in `BOTTOM_TABS` (Problems, Search Results, Log) and `SWITCHES_BEFORE` (Log, Problems); never in a floating window's bottom pane.
- Redraw: the panel is drawn before the editor; a tileset marking its findings stale later in the pass would show next event only. `Problems::drawn = (pass, revision)` set by the body; `follow_bottom` asks one more frame when they differ in the same pass. Don't compare without the pass (a closed dock would repaint forever).
- Staleness (review fix): by FINGERPRINT, not revision. `Set::checked` = `problems::fingerprint` of the text Check read; `tileset_view::follow_all` (every open tileset at each frame's start, drawn or not; a tileset changed behind another tab stayed counted when this ran only in its `ui`) and `follow_problems` (the doc's hash cached per revision) sets `Stale::Changed` or clears it, so Undo back to the checked text counts again; `tileset_view::closed` (from `forget_side`) compares the file on disk; `open_path` compares on reopen and marks `Stale::Missing` when the file is gone. Stale sets are listed, NOT counted in the badge (coordinator's ruling). `Problems` fields are private: change only through `replace`, `set_stale`, `remove`.
- Preview's 3D view of a tile starts paused (`model_view::Shown::paused`): the game's programs that read the clock (`worldtimerTimeOfDay`) asked a frame every step even with nothing moving.
- Ctrl+S stays the module's Save (owner's ruling 2026-11-18): no Save hint beside a tileset.
- The bar's actions run after the list and form (`BarAsked`) so both read one revision (the list's rows built from a stale Arc under a new revision indexed past the end). NOT for typed text: the review's G4 probe and `typed_text_is_set_before_a_one_frame_click_removes_its_tile` (raw press+release in one frame, the field still focused) pass with the action moved first; the text is set before the press acts whatever the order.
- Trap: egui drops the focus at a click elsewhere when the pass ends, so `focus_field` (a finding's field) waits for a frame with no press/release (`focus_asked`).
- Outliner: `Owner::Tileset(id)`, `Contents::Tileset` (no module needed); rows flattened (`outline_rows`, cached by revision and the open groups), drawn in sight via `show_viewport`; part rows labeled "Terrain 0: Grass" (a bare "Water" collided with combo options); the group arrow is its own interact named "Open Tiles" (TreeRow's arrow_apart leaves the column out of the row's response).
- Preview: `Brings::diagram` makes a tileset's Preview available whatever `palette_dock`; the open-in-tab icon reuses `SectionBar::opened` (`shell::opened_in_tab` dispatches).
