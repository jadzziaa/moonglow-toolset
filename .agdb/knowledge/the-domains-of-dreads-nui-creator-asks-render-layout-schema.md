---
type: Decision
title: "The Domains of Dread's NUI Creator asks: render, layout, schema, import, AccessKit, event function"
tags: []
status: stable
generated: { by: trail, at: 2026-10-22T00:00:00Z }
touches: [docs/deferred.md, docs/log.md, docs/manual/02-main-window.md, docs/manual/11-command-line.md, docs/manual/18-nui-creator.md, docs/manual/index.md, docs/research/notes_doctor.md, Cargo.lock, Cargo.toml, crates/mg-ui/tests/frame_perf.rs]
fingerprint: v2-a9e4e7a5aa895c57
created: 2026-10-22
author: August
description: "Records design decisions for the NUI Creator's CLI render/layout (one headless frame of the editor canvas), hand-written settings schema, event-function includes, AccessKit setup, and pitfalls like TexturesDelta panics and module.ifo-less modules."
asks:
  - "How does `mg nui render` produce an image headlessly, and does it use the same drawing code as the Creator's editor canvas?"
  - "Why does my headless egui render panic on drop with TexturesDelta, and how do I avoid it?"
  - "How do I load a folder of windows or a .jui file that has no module.ifo without module_resman failing?"
  - "How do I get screen reader support working in the Flatpak build on Linux, and what does AccessKit cost per frame?"
---
## Decided
- `mg nui render` and `mg nui layout` are one frame of the Creator's canvas itself: `preview::still` (stage.rs) builds the same `Stage` the editor's `canvas` does and calls `Stage::window`, with `preview_clean` (no guides), zoom 1 and the UI scale; `nui_view/headless.rs` runs it in a bare `egui::Context` (frames until no repaint is asked: fonts added by `Assets::prepare` take effect a frame later) and draws the last frame with `egui_wgpu::Renderer` into an Rgba8Unorm texture on `mg_render::Gpu::headless()`. Exposed as `mg_ui::nui_render`; `mg` now links `mg-ui` for it.
- The layout is recorded by the canvas, not recomputed: `skin::Assets::record` (a `RefCell<Option<Record>>`, `None` in the editor) is filled in `Renderer::node` (before `draw_node`'s early returns), the window's rect in `Stage::window`, cut text in `typeset::draw_text`. While recording, a control outside the clip is still laid out (drawn clipped to nothing), so the children of an out-of-view row are reported.
- The settings' JSON Schema (`mg_nui::settings_schema`) is written by hand beside `Settings`, held to it by tests that serialize a fully set `Settings` and every `Action` and compare keys and defaults; a test also requires every key in the manual's "The settings file" section and that its example is `crates/mg-nui/tests/data/crier.txt` (the crier window, the fixture the CLI and preview tests draw).
- A window's event script can be an include for the module's own OnNuiEvent (`Settings::event_function`, skipped when false so old files write as before): `int NAME_OnEvent()`, known by `WindowId_NAME()` from the opener, opener passes no event script. `edited_outside` infers the mode from the script (`int NAME_OnEvent()\n`); `generate` compiles it through the probe caller and compiles `includers` of opener and events; rename moves its `#include` lines with `mg_module::rename::rename`.
- Nasher projects that leave windows out: `nasher::Project::uncovered(keys)` and `include_for(files)` (shared with `take_in_left_out`); `save` refreshes `left_out` when it wrote uncovered files so Build › Add Left-Out Files can take them in.
- AccessKit: only eframe's `accesskit` feature was needed (egui has the tree built in). eframe enables the tree on `InitialTreeRequested` only; on Linux accesskit_unix activates only when `org.a11y.Status.ScreenReaderEnabled` is true on the session bus, so the Flatpak needs `--talk-name=org.a11y.Bus`. Measured with `FRAME_ACCESSKIT=1` in frame_perf: +0.10 ms mean, +0.55 ms worst; not a setting.

## Watch out
- `egui::TexturesDelta` panics on drop with unapplied deltas: `clear()` it after taking what the renderer needs.
- A module with no `module.ifo` (a folder of windows, a `.jui` file) can't go through `module_resman` (`m.haks()` fails): use `ResMan::for_game`.
- Doctor findings sort by `ResKey`, whose type order puts `.txt` before `.jui`.
- In this worktree session Bash refused heredocs and most `python3 -c` edits (and anything mentioning GIT in text): Edit/Write and scripts in the scratchpad worked; GPU runs went through a scratchpad script holding the flock.
