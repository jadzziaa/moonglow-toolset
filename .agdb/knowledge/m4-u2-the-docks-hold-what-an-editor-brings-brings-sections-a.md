---
type: Decision
title: "M4 U2: the docks hold what an editor brings (Brings, sections, arrange, two hosts, test panels)"
description: "How a panel joins the docks since U2: brings_of by kind, sections and arrange, the area's front kept for the run, orders as data, the test-only Sample panels, and the plugins' pane rule."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-15T01:16:41Z }
touches: ["crates/mg-ui/src/shell.rs:brings_of", "crates/mg-ui/src/shell.rs:arrange", "crates/mg-ui/src/shell.rs:window_panes", "crates/mg-ui/src/shell.rs:panel_body", "crates/mg-ui/src/settings.rs:DockLayout"]
asks:
  - How do I add a panel to the Right Dock or the Bottom Dock for an editor?
  - "Why is the area's Right Dock tab not kept in the settings when other kinds keep theirs?"
  - How are the dock changes tested without shipping a fake panel?
  - Why does the wide layout still show Palette and Preview in two columns?
fingerprint: v2-73dd406e967031f5
created: 2026-11-15
---
## Decided
- A new panel: a `Panel` variant with its arms (`dock`, `name`, `icon`, `tools`, `strip_tip`, `keys`, `badge`, `available_with`) and a `panel_body` arm; the editor's kind gives it in `brings_of` (static `Brings`: `right` sections, `folded` order under 1400, `bottom` own panels, `floats`). Global Bottom Dock panels go in `Panel::BOTTOM_TABS` and `SWITCHES_BEFORE`/`SWITCHES_AFTER`: the Script board's tab order (Compiler, Bookmarks, Search Results, Log) and switch order (Log, Problems, Compiler, Bookmarks, Search Results) differ, so orders are tables, not concatenation.
- `arrange` is pure: narrow → one bar in `folded` order; wide columns → first section, then the rest; a LONE section of 2+ panels stands as first panel | the rest (keeps M3's Palette | Preview columns until U6 adds Properties); else stacked.
- The single-bar path in `right_dock` keeps M3's ids, names and nesting exactly: the 44 shell/board pictures matched pixel for pixel. Keep it so when touching it.
- `Settings::dock_layouts` (kind → fronts, shares, bottom, by names, `skip_serializing_if` empty). The area and placeholder keep NO front there (`keeps_fronts`): M3's `the_palette_is_in_front_as_a_module_opens` keeps the area's tab for the run. Plan (decision 2) and test disagree; reported to the owner.
- Plugins' pane: a press in a panel an editor brings is that editor's pane (Palette/Preview → palette; area's others → area; else unchanged). In 17-plugin-api.md.
## How it is tested
- `shell::dock_tests` in the library, with `#[cfg(test)]` `Panel::Sample(n)` (n < 8 Right, ≥ 8 Bottom, 7 never available) and `Moonglow::sample_brings` (kind, Brings): compiled into no shipped binary nor tests/app. kittest works in `src` (dev-dependency).
## Watch out
- A floating editor's panels are filtered with `available_with(app, &its brings)`, not `available` (which reads the active docked editor): with `available` the window drew nothing.
- `bottom_panel` is `Option`: a kind's kept Bottom Dock front must win over a default Log.
- Fronts and shares are kept by the section's place among the kind's OWN sections (`own_index`), never for the palettes' section asked for by name beside another kind (it is inserted at 0 in `right_sections` and would shift indexes and leak "Preview" into that kind's settings).
- Seams (`section_seams`) are made after the sections' widgets, or the tab bars under them win the pointer.
- One place still names panels: `keys_leave_panels_not_drawn`'s Palette | Preview arm; a panel with its own `Keys` joins it.
