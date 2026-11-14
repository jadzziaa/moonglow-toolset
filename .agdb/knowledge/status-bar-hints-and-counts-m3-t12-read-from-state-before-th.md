---
type: Decision
title: "Status Bar hints and counts (M3 T12): read from state before the editor area is drawn"
description: "How the Status Bar's hints and counts are computed from the active editor's state at frame start (shell::shown, area_view::bar), why a repaint follows when the editor area changes them, how the Esc hint foresees T10's rule, and the one yield order."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T21:54:25Z }
touches: ["crates/mg-ui/src/shell.rs:shown", "crates/mg-ui/src/shell.rs:status_bar", crates/mg-ui/src/area_view/bar.rs, "crates/mg-ui/src/tabs.rs:Viewer::foresee_escape", "crates/mg-uikit/src/status.rs:Hints::show", "crates/mg-ui/tests/app/common.rs:tip_at"]
asks:
  - "Why do the Status Bar's hints lag a frame or ask for a repaint after the editor area is drawn?"
  - Why does the Esc hint not show while a text field has the keyboard?
  - "A test's tooltip read picks up Status Bar hints: why?"
  - In what order does the Status Bar give way as the window narrows?
fingerprint: v2-c12192546a581198
created: 2026-11-13
---
## Decided
- The Status Bar is drawn before the editor area, so its hints and counts are a function of state as the frame begins (`shell::shown`: the active docked editor's, or the Resources panel's while it is in use; `area_view::bar::status` for an area). After the editor area, `shell::said_next` computes them again and asks for a frame where they differ from what was shown (`Moonglow::bar_said`): no stale hint, no extra frames at rest.
- The area's former hint line had non-key text: it became notes (`Hint::note`, word-only hints at the head of the hints: what is in hand, a refusal's reason) and counts at the bar's right ("1 selected" with the object's tag, place and facing in its tip; "2 by 2 tiles selected"; "Tile (x, y) model" under a tile brush). A note that does not fit is cut with an ellipsis (`status::NOTE_LEAST`) rather than dropped; key hints drop whole.
- A tile brush's note is "In hand: <label>", not the bare label: the bare label duplicated the palette's own entry ("Eraser", a tile's name) and broke tests that find it by name.
- The Esc hint for a docked OK/Cancel tab is foreseen in `tabs::Viewer::foresee_escape` by T10's rule, only where nothing has the keyboard: where a widget has it, which tab an Escape goes to is known only at the Escape (egui drops the focus first; reading the focused response each frame broke 24 tests in T10). Omitted rather than guessed. A floating window with OK/Cancel takes Escape only where no docked tab saw it, so it needs no check of its own.
- One yield order: the folder; the hints from the end; the log's last line (from its cap, 96 beside an area, 160 otherwise, down to 48); the counts from the end (the middle is hidden once a count goes); the job's name; the module's name.

## Tried and ruled out
- Requiring the last line to be at least 48 wide: a short line ("Log") vanished. A line that fits whole always shows.

## Watch out
- `common::tip_at` finds "what appeared" after hovering; Status Bar hints change with the pointer, so it skips the Status Bar group's nodes.
- `Moonglow::status_for_tests` overrides the bar's hints and counts; T7's last-line tests use it with none so the line has the middle alone.
