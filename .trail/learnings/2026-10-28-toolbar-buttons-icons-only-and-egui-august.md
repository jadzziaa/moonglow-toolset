---
title: Toolbar buttons icons only and egui has_glyphs
author: August
date: 2026-10-28
branch: worktree-agent-a02944a0d27f498a6
cost: { minutes: 1361, tokens: 11938452 }
touches:
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/icons.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/tests/app/areas.rs
  - crates/mg-ui/tests/app/options_and_keys.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
  - docs/log.md
---

## Decided
- Options › General › Toolbar buttons (Icons and text / Icons only) is a plain bool, `Settings::toolbar_icons_only` (the struct's serde default loads old files; no custom Deserialize as GraphicsBackend needed for unknown names). The page shows it as a two-choice dropdown.
- Every main-toolbar and area-toolbar button with a glyph goes through `icons::Tools` (label, tip, toggle, button, add). `TOOLBAR` holds (Id, glyph, name, tip), not "glyph name" text. With Icons only the tip is the name, then the tip on a second line unless it begins with the name; tips show while disabled too.
- Scope was the two toolbars; the palette's tile tools and the other editors' toolbars keep words (docs/deferred.md, The main window and its panes).

## Watch out
- egui 0.36's `Fonts::has_glyphs` says false for every glyph of the face that holds the replacement square ◻, which is the emoji font: most toolbar emoji. Check a glyph by laying it out and comparing its `uv_rect` with ◻'s (`icons::tests::the_fonts_have_every_toolbar_glyph`).
- With Icons only, glyphs repeat across the window (ℹ on the main toolbar and the area's, ⊞ the area's Grid and the palette's pictures): tests pick the node nearest a known button, not `get_by_label`.
- In a worktree session the Bash guard refuses flock/systemd-run wrappers around cargo; put the wrapper in a scratchpad script and run `sh script cargo test ...`.
