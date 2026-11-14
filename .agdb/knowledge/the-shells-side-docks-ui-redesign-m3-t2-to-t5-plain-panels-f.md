---
type: Decision
title: "The shell's side docks (UI redesign M3, T2 to T5): plain panels, fixed panels, chosen widths"
description: "Why the Left, Right and Bottom Docks are plain egui panels (left_dock, right_dock, bottom_dock) with fixed panels and a chosen_size kept apart from the drawn width, and what tests must set."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T20:58:08Z }
touches: [crates/mg-ui/src/commands.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/palette_view.rs, crates/mg-ui/src/settings.rs, crates/mg-ui/src/shell.rs, crates/mg-ui/tests/app/area_view.rs, crates/mg-ui/tests/app/areas.rs, crates/mg-ui/tests/app/common.rs, crates/mg-ui/tests/app/icon_tips.rs, crates/mg-ui/tests/app/resources_panel.rs]
asks:
  - Why are the side docks plain egui panels and not egui_dock surfaces?
  - "Why does narrowing the window shrink a dock's width for good, and how is the chosen width kept?"
  - "Why does setting egui's panel state for a dock width in a test do nothing?"
  - "What happened to Tab::Resources and Tab::Palette and how does shell_migrate treat old open-tabs records?"
fingerprint: v2-33ac1ca10e72b59a
created: 2026-11-13
---
## Decided
- The Left, Right and Bottom Docks are plain egui panels with mg-uikit's tab bars (`crates/mg-ui/src/shell.rs`: `left_dock` id "tree", `right_dock` id "right-dock", `bottom_dock` id "log"), not egui_dock surfaces: egui_dock has one Style per DockArea, so the side docks could not get their own look and fixed widths. The editor area stays the one DockArea.
- Panels are fixed to their docks (owner's ruling R5, 2026-11-11): the registry `shell::Panel` (Module, Resources, Palette) says which dock a panel lives in. `Tab::Resources` and `Tab::Palette` are gone; `Settings::shell_migrate` (idempotent) drops their kept places, and an open-tabs record naming them shows the panel and is never written again.
- One fold button per dock, beside the dock's inner edge on the editor-area side, half way down, the same place and size folded and unfolded (issue 10's rule). It is outside the tab bar: a 190 px dock has no room for two tabs, the panel's tools and a fold button. Tabs turn to icons by the dock's width alone, never by which panel is in front.
- The width a user chose is kept apart from the width drawn (`chosen_size`, `draw_at`, `edge_dragged`; `Settings::left_dock_width`, `right_dock_width`, `bottom_dock_height`, whole points because Settings derives Eq). The chosen width changes only when the edge is dragged or on Reset Layout.
- Each list keeps a scroll id of its own (`resources-list`, `palette-list`, `palette-tiles-list`); the module tree keeps its old id because egui saves its place between runs.
- Narrow windows (under about 760 px with both docks open) leave the editor area a sliver until T14's yields (the Left Dock folding by itself); T14's plan in `docs/ui-redesign-proposal.md` lists what it inherits.

## Tried and ruled out
- Letting egui's panel state hold the dock's width with `size_range`: egui stores the clamped width, so narrowing the window shrank a dock for good, and eframe saved it.
- A fold button inside the dock's tab bar, and a chevron on the folded strip.
- A width sweep that counts widgets by their center inside the dock: a control spilling more than half past the edge is never counted, and the fold button sits on a layer of its own that `overlapping()` does not compare. The sweep takes the dock's nodes from the accessibility tree (the dock's panel has role Pane) and checks each against the dock's rect and the button's.

## Watch out
- Test helpers that set a dock's width must set the chosen width (`common.rs` `left_dock_width`, `right_dock_width`); writing egui's panel state now does nothing, silently.
- A test that cleared `palette_dock` runs without the Right Dock for good (the flag sticks); `open_palette = false` before it was undone by opening a module with game data.
- The palette's groups animate open beside the editor, so a step after opening something may need `settle` where a fixed number of frames used to do.
- `cargo test` stops at the first failing test binary: a full-suite count lower than the last one means a binary did not run. Use `--no-fail-fast` for the gate.
- After each implementation agent, an independent read-only reviewer on the commit found real faults every time (weak sweeps, vacuous checks, a deferred entry that lost an open item while the count stayed the same).
