---
type: Decision
title: "NUI designer U19: the source group is a tab of its own (Tab::NuiSource), Checks a Bottom Dock panel, and app.dock is empty while tabs draw"
description: "How M4 U19 built the NUI designer's source group, Checks, Status Bar state and canvas corners, and the traps met doing it."
tags: [nui]
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-18T00:33:14Z }
touches: [crates/mg-ui/src/nui_view/source.rs, crates/mg-ui/src/nui_view/checks.rs, "crates/mg-ui/src/nui_view.rs:editor", "crates/mg-ui/src/lib.rs:open_nui_source", "crates/mg-ui/src/shell.rs:owner_of"]
asks:
  - "How does the NUI designer's Split source work now?"
  - Why does a NUI tab body see an empty app.dock?
  - "Where does the NUI designer's Not built or Build failed show when it floats?"
  - How does the NUI source text follow the control chosen?
fingerprint: v2-6c0f9f1375783c2d
created: 2026-11-18
---
- Split source is `Tab::NuiSource(key)`, a real editor group at the design's right (the board's `crier.jui` beside `crier_e.nss`; U2 had reserved `shell::owner_of` for it). `owner_of` maps it to `Tab::Nui(key)` (returns a `Cow` now), so its docks, Outliner (Layers), Checks and Status Bar counts are the designer's; `window_panes` skips it (the designer's leaf draws the panes). `panels::nui_of` accepts both.
- Opened/closed by `Action::NuiSource(key, bool)` (`Moonglow::open_nui_source`): split right of the designer's leaf with `nui_view::SOURCE_SHARE` (640/1018); in a floating window the window is widened by `SOURCE_WIDTH` (moved left at the screen's edge) and `nui_widened` gives the width back on close (also when closed by its ×, via `nui_view::follow`). `follow` closes a source whose designer is gone.
- TRAP: while the editor area draws, `app.dock` is a placeholder (lib.rs swaps the DockState out). Inside a tab body use `app.panes_tabs` (each pane's tabs as the frame began), never `app.dock.find_tab`. The first version read `app.dock` in `nui_view::editor` and Split source never showed as open.
- The source reads the module's bytes (the old JUI Source page's raw text, not Split source's re-serialized copy), laid out by `script_layout::LaidOut` (numbers + line galleys, kept per text). Typed text is a `SourceDraft`; set on leave by `source::set`: through the designer's `end` when it has yet to run this pass, else its own `Action::Apply` noted for Cancel (`blueprint::note_command`). `follow` sets a draft whose group was not drawn.
- Following: `source::span`/`control_at` walk the JSON text (pointer -> byte span, byte -> deepest control); design -> source sets the TextEditState range and marks the lines (`source_lines`, painted under the text); source -> design goes through `picked.chosen` (done at `end`). A variant edited maps `/root/...` to `/views/<name>/...` in Binding JSON.
- Checks: `Panel::Checks` (Bottom Dock, `NUI_BOTTOM`, `Keys::NuiChecks`), body `nui_view::checks::panel`; badge = errors+warnings (Error/Warn kind). New errors push `app.nui_checks_forward`, handled in `shell::bottom_forward` at frame start (docked: focus_bottom + hide_log=false if the designer is the active editor; floating: `window_bottoms`). Layers' warning mark click pushes the same.
- Status Bar: `nui_view::shown` (hints only for the Design page, nothing typed, no popover, the design the active editor); `Count::warn` + kit `StatusItem::warn` for Not built / Build failed. Floating, the header's `BUILD_STATE` custom item says it (the Status Bar is the main window's).

- REVIEW ROUND (the coordinator's rulings): the source is bound to its design. EVERY opening goes through `open_nui_source` (`Action::OpenTab(Tab::NuiSource)` is redirected there; it opens the design first with `run_now(OpenTab(Nui))`); `restore_tabs` reopens a recorded source without its design the same way; `follow` closes a source whose design is gone wherever it was dragged. A designer opens among the designs (`Moonglow::design_group`), never into a source group.
- Floating place: `nui_widened` keeps (place before, place given, panes' corner); closing puts the place back and sets `WindowTrack::aim` (as a maximized window's way back), because egui holds a still-wide window inside the screen. `remember_window_sizes` now returns whether it put a window again, and the frame is asked for: without it a test's next click hit the window mid-move.
- The Bindings table walks the window (`bind_names`, `binds_uses`) once per bytes (`design::BindsSeen`); the tip walked twice a frame, which is what the chart's 30 ms was.
- Following: design follows the text only on a click, or a caret move while the text chose (`State::source_chose`), never clearing several chosen. Only the cursor's line is marked.
- The source's draft keeps `from` (the module's Arc it was read from): read again by pointer comparison, set on Save (`source::store` from `store_typed`) or on close only onto those bytes (`typed_over`).
- `chip_asked(ctx)` reads ctx data: compute the id BEFORE `ctx.data_mut(...)` or egui deadlocks (RwLock, "Deadlock?" after 10 s).
