---
type: Decision
title: "An encounter's own outline color (MG_OutlineColor): where it goes and where it must not"
description: "Why MG_OutlineColor stays in nasher JSON unlike MG_Locked, is stripped by Add to Palette, kept by Update Instances, sits in PLACED_ONLY, and how the selected look and the Properties row work."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T05:01:07Z }
touches: [crates/mg-rules/src/placement.rs, crates/mg-module/src/palette_add.rs, crates/mg-module/src/instances.rs, "crates/mg-ui/src/area_view.rs:overlays", "crates/mg-ui/src/blueprint/mod.rs:outline_color", crates/mg-ui/tests/app/outline_color.rs]
asks:
  - "where is an encounter's outline color stored and does the game read it"
  - why does Add to Palette strip MG_Locked and MG_OutlineColor
  - why is the outline color kept in nasher project json when locks are not
  - why do UI tests of the encounter Properties row not find the Outline color button
fingerprint: v2-68a08d17066d56b7
created: 2026-11-23
author: augustaugur
---
decided (owner, 2026-11-22: per encounter): `MG_OutlineColor`, DWORD 0x00BBGGRR (as Form::color stores colors), on the placed encounter's GIT struct; no field = the kind's color (orange, `blueprint::ENCOUNTER_OUTLINE`). Default removes the field, never writes a value. Read once in `mg_rules::placement::outline_color`, carried through `values::Object` / `AreaObject::outline_color`; the draw site (`overlays`) uses it for any outline kind, so a trigger color is only a Properties row away.
decided: unlike MG_Locked, it stays in a nasher project's JSON (a team shares colors); `locks::strip` only runs when locks exist and never touches it (test `a_project_keeps_an_outline_color_in_its_files`, with a lock beside it so the strip path runs).
decided: `palette_add::placement_fields` drops MG_Locked and MG_OutlineColor from the new blueprint (Add to Palette and Export as Blueprint both go through add_to_palette); before, a .ute made from a locked object carried MG_Locked into every instance placed from it (instances::instance keeps unknown fields). `instances::KEPT` keeps the color through Update Instances and Replace Selected (it is a mark of the placed object, like spawn points). Module export/pack and mg commands do nothing special with either field; Verify has no unknown-field check.
decided: it is in `PLACED_ONLY`: an edit of it alone re-reads placement, keeps the look (unit test in mg-area lib tests).
decided: selected, an outline with its own color keeps it (width 2.5 via the dark-edged canvas path, `OWN_SELECTED_WIDTH`); one without keeps today's selection color. The overlay does not follow the theme.
watch out: `Form::color` reads an absent field as 0 (black) and skips writing when the pick equals it: the Outline color row is its own `Form::outline_color` (default shown, commit on picker close, Default enabled when any target has it via `differs`). The color button has a role but no name: the row names it with `widget_info(labeled ColorButton "Outline color")`.
watch out: in tests the row is the Basic page's last and below the fold of the docked Properties: `scroll_to_me()` the color well before clicking, or the click does nothing. Read-only Properties are not disabled widgets (Default reports enabled); `look_only` drops the edits.
