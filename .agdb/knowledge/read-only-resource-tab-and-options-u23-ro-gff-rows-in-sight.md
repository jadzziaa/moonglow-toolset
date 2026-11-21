---
type: Gotcha
title: "Read-only resource tab and Options (U23): ro_gff rows in sight, names via mg_rules::rows, the row menu by keyboard, toolbar_icons_only retired"
description: "How U23 built the read-only GFF view and Options' General page, and the traps found in it and its review round: kittest steps each event alone, Shift+F10 eaten by the menu bar, the harness's 8-point margin, folder modules losing extensionless kinds, header group nodes without rects."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T12:00:00Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-19T07:32:53Z }
touches: [crates/mg-ui/src/ro_gff.rs, "crates/mg-ui/src/browser.rs:resource_header", "crates/mg-ui/src/options.rs:general", crates/mg-ui/src/commands.rs, "crates/mg-ui/src/shell.rs:enter_is_options_ok"]
asks:
  - how does the read-only resource tab show a GFF with names beside numbers
  - why does a spin field have no accessible name in a sweep test
  - how is an old settings file with toolbar_icons_only tested
  - "why did a resource test find two nodes with the panel's count"
  - why does a kittest test see nothing copied after Enter on a button
  - why does Shift+F10 not open a context menu
  - "why is a UI test's window 16 points narrower than its harness"
fingerprint: v2-0e489af7b1e80599
created: 2026-11-18
author: augustaugur
---
## What was built
- `ro_gff` is the one GFF view for the read-only tab and the hak panels' preview (`browser::plain_ui`). Rows are listed once per fold (`Tree::changes`) and laid out only in sight (`ScrollArea::show_rows`); one thread-local count of every listing wherever a tree is drawn (`ro_gff::counts`, `Moonglow::read_only_counts`). Names beside numbers are `mg gff --names`' own code: `mg_rules::rows::{meaning, list_item_name}` and `GameData::meaning_name`; where that gives nothing, `ro_gff::missing` says nothing for a value meaning none (StrRef none, below 0, the field type's largest, a row present with a blank or `****` label) and "(no row N in T.2da)" only for a row the table lacks. Checked against `mg gff --names` over 21 game files: no mismatch.
- The row menu: Copy Value is `value_copy` (the raw value), a locstring's item names what it copies, Copy All Languages, Copy Meaning; Shift+F10 opens it with the first item focused, Escape gives the keyboard back to the row, Ctrl+C (`Event::Copy`) on a focused row copies its value.
- The Resources panel's count (`Browser::count_text`) is inserted first in `shell::shown`'s counts while the panel is in use. A test that `get_by_label_contains("1 of ")` now finds two nodes: the panel and the Status Bar.
- Options is flush in its pane (`tabs::form_window`); Questions' label column is the board's 186, shrinking to the longest label (177) before a label goes over its dropdown (`question_least`, measured from the choices).

## Watch out
- kittest's `step` runs one pass per queued event and keeps only the last pass's output: a copy made on Enter's press is gone after its release. Queue the press alone (`h.event(Key{pressed:true})`, `step`, read `output()`), then the release.
- egui's `consume_key(Modifiers::NONE, F10)` matches Shift+F10 too (Shift is matched loosely): the menu bar's F10 ate Shift+F10 until it checked `!modifiers.shift`. egui and egui-winit have no menu (ContextMenu) key.
- `Harness::builder().build_ui_state` puts the app in a CentralPanel with egui's 8-point margin; the real window is flush. A 1600 harness gives the app 1584: test the board's 1600 at 1616.
- A module kept as files (folder, nasher project) names a resource by its type's extension; a type with none is written as `name.#id` and never read back. Copy to Module is off there (`browser::resource_header`).
- The kit's `Item::custom` ignores `.tip()`: put the tip on with `ui.interact(..).on_hover_text`.
- `widgets::dropdown(..).show` gives an unnamed ComboBox. Name it with `ctx.accesskit_node_builder(r.id, |n| n.set_label(..))` (`options::say_name`).
- A group named with `name_group` (a kit header, the page list) has no rect: `Node::rect()` panics. Use `mg_uikit::header::records(&ctx, name)[0].row`, or a row inside it.
- Replacing `app.settings` with a whole fixture file in a UI test changes everything else too (open_editors Floating, ui_scale 125). Add the old field to the window's own `saved_text()` instead (`with_old_toolbar_field`).
- Enter OK for a floating Options adds one more Status Bar change after opening: a test that `h.run()`s right after `Action::OptionsDialog` must `settle`.
- The header's compact form (the lock alone) is reached docked at 1600 with both docks at x1.25 and x1.5, and in a 760-wide window; a test looking for its tip must look for the tip's own words (the Status Bar has the count too).
