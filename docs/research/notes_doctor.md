---
type: Research Note
title: 'The content doctor: what it checks, and how it was calibrated'
description: What Moonglow's content doctor checks in a module's custom content, where each check comes from, and how the checks were calibrated on shipped content.
tags: [doctor, custom-content, checks]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-02T05:53:59Z }
---

# The content doctor: what it checks, and how it was calibrated

October 2026. `crates/mg-module/src/doctor.rs` reports problems in a
module's custom content: the module, the user's haks, `override`,
`development` and NWSync content. The game's data and the haks it ships
are taken as they are.

## Where the checks come from

The checks come from the crash causes catalogued on nwn.wiki's "Common
Errors and Their Causes" (2026-03) and in Beamdog/nwn-issues, as
summarized in `community_pain_points.md`. Each check names the source
(hak or folder), the file, and the row and column, section or object.

| Check | Finding | Severity |
| --- | --- | --- |
| `set-read` | a tileset that doesn't parse (a section its counts name is missing) | error |
| `set-count` | `[TILEn]`/`[GROUPn]` sections past the `Count` (Aurora's Range Check Error) | error |
| `set-model` | a tile whose model exists nowhere | error |
| `set-group` | a group with no cells, a first tile of `-1`, a first tile shared with another group, or a tile past the list | error |
| `set-transition` | `Transition=0` with groups (raised groups won't place) | warning |
| `set-door` | a tile door type that isn't a `doortypes.2da` row with a model, one finding per type | warning |
| `set-data` | `mg-set`'s parse warnings (undeclared terrains, missing door sections) | warning |
| `tile-faces` | a custom tile model over 10,000 faces | warning |
| `mtr-name` | a material's texture name over 16 characters ("Pure virtual function called") | error |
| `2da-row` | a row with more cells than columns, in a table the game also has | error |
| `2da-limit` | `baseitems.2da` over 256 rows, `lightcolor.2da` over 32 (Aurora only) | warning |
| `2da-strref` | a StrRef past the end of `dialog.tlk` or the custom TLK, in a changed or added row of a known column | error |
| `2da-shadow` | a hak's 2DA hiding another custom layer's longer copy | warning |
| `erf-size` | resources starting 2 GiB or more into a hak, which the game can't read (`notes_scale.md`) | error |
| `object-row` | a creature (appearance, classes), placeable, door or item naming a 2DA row that doesn't exist, a reserved row (`USER`), or a row whose model doesn't exist | error placed, warning in a blueprint |

## Calibration on shipped content

`crates/mg-corpus-tests/tests/doctor.rs` runs the doctor on all 28
shipped modules and campaigns. Each module's haks are added as user haks,
so they're examined as a builder's would be. Every finding was checked by
hand. What remains are real defects in the shipped content:

- **Missing tile models:** Kingmaker's `trc01.set` (one) and Dark Dreams
  of Furiae's `tcm02.set` (two).
- **Dark Dreams of Furiae's `tcm02.set`:**
  - a group whose first tile is -1;
  - tiles declaring doors without door sections;
  - door types 5001 to 5012, past the hak's own 1,451-row `doortypes.2da`,
    which hides the game's 5,200 rows.
- **Darkness over Daggerford:** two groups named `cliff_path1` sharing a
  first tile. The hak's 512-row `appearance.2da` leaves `pm_cat.utc`'s
  appearance 750 without a row, and the world-map pin blueprints name
  models that were never shipped.
- **Doom of Icewind Dale** places an object on a reserved
  `placeables.2da` row (model `USER`).
- **Wyvern Crown of Cormyr** places trees, benches and braziers on
  `placeables.2da` rows 500–700, which its hak's copy leaves empty.

`engine_doctor.rs` checks the shadowed-row verdicts in `nwserver`. With
each campaign's haks loaded, `Get2DAString` returns empty for the cells
behind the findings and agrees with the doctor on control cells.

Calibrating removed three checks that would have been wrong:

- **Models of every 2DA row:** shipped tables are full of placeholder
  rows (`RESERVED`, `User01`, world-map pins). A missing model matters
  only for rows something uses, and the object check covers those.
- **Hiding the game's longer 2DA:** haks made before EE carry whole old
  tables (`portraits.2da` with 1,067 rows over the game's 15,427). That's
  normal, and the object check catches the rows that matter.
- **Walkmeshes reaching past the tile:** the wiki names this as a crash
  cause, but the game's own tiles have walkmesh nodes reaching up to 10 m
  from the center (`tts01_c14_01`), and they work. Whatever the crash
  really needs isn't known.

Also, over-long rows are only reported in tables the game has.
Infinite Dungeons' script-only tables carry deliberate marker lines
(`**** WARNING: EDITING ENTRIES BEFORE THIS LINE …`).

## Missing references in Verify

`verify::Missing::is_error`:
- **Errors:** references from areas, placed objects, the module,
  conversations and scripts, including scripts that exist only as source.
- **Warnings:**
  - blueprints' references, which matter once something places them;
  - placed objects' own blueprints, since the object holds all it needs;
  - sounds, portraits and movies.

With that, the eight shipped `.mod` modules have at most one error each.
The campaigns have tens to hundreds: conversations naming scripts from
other chapters, and scripts that were never compiled.
