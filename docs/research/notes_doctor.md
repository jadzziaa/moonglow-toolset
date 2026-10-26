---
type: Research Note
title: 'The content doctor: what it checks, and how it was calibrated'
description: What Moonglow's content doctor checks in a module's custom content, where each check comes from, and how the checks were calibrated on shipped content.
tags: [doctor, custom-content, checks]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-26T23:58:14Z }
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
| `custom-tlk` | the custom talk table the module names is not in its haks, the module or the `tlk` folder (or is named with `.tlk`, or in another case on Linux) | error |
| `set-read` | a file that is no tileset (no `[GENERAL]`) | error |
| `set-line-ends` | a tileset whose lines end in a bare LF, not CRLF: the game's server never finishes loading a module that uses it | error |
| `set-count` | `[TILEn]`/`[GROUPn]` sections past the `Count` (Aurora's Range Check Error), and a `Count` past a list's last section (warning) | error |
| `set-hole` | a section a count calls for that is missing, or a tile without `Model=`: the game reads the rest, draws such a tile as nothing and gives it no walkmesh, and leaves such a terrain, crosser or rule out (the first 50, then a count) | warning |
| `set-model` | a tile whose model exists nowhere | error |
| `set-group` | a group with no cells, a first tile of `-1`, a first tile shared with another group, or a tile past the list | error |
| `set-transition` | `Transition=0` with groups (raised groups won't place) | warning |
| `set-door` | a tile door type that isn't a `doortypes.2da` row with a model, one finding per type | warning |
| `set-data` | `mg-set`'s parse warnings (undeclared terrains, missing door sections) | warning |
| `tile-faces` | a custom tile model over 10,000 faces | warning |
| `mdl-read` | a tile's custom model that can't be read | error |
| `mdl-text` | a custom model kept as text with something the game refuses, misreads or crashes on (the linter's errors, and its first five warnings a model) | as the linter says |
| `mtr-name` | a material's texture name over 16 characters ("Pure virtual function called") | error |
| `2da-row` | a row with more cells than columns, in a table the game also has | error |
| `2da-read` | a custom 2DA that can't be read | error |
| `2da-limit` | `baseitems.2da` over 256 rows, `lightcolor.2da` over 32 (Aurora only) | warning |
| `2da-strref` | a StrRef past the end of `dialog.tlk` or the custom TLK, in a changed or added row of a known column | error |
| `2da-ref` | a row number past the end of the table it names (`feat.PREREQFEAT1`, `spells.FeatID`, `racialtypes.Appearance`…: `ROW_COLUMNS`; the lists a class, a race or a package names: `NAMED_TABLES`), in a changed or added row | warning |
| `2da-described` | a cell of one of the module's own tables that is not what the table's description (`name.2da.toml`, `mg_module::table_desc`) says its column holds, or a description that can't be read | warning |
| `2da-shadow` | a hak's 2DA hiding another custom layer's longer copy | warning |
| `erf-size` | resources starting 2 GiB or more into a hak, which the game can't read (`notes_scale.md`) | error |
| `object-row` | a creature (appearance, classes), placeable, door or item naming a 2DA row that doesn't exist, a reserved row (`USER`), or a row whose model doesn't exist | error placed, warning in a blueprint |
| `pc-faction` | a creature, placeable, door, trigger or encounter (blueprint or placed) of faction 0, the PC faction, the players' alone (Jasperre's report: the game puts such a creature in Hostile, `engine_factions.rs`); a missing faction field is not reported. None in the shipped modules | warning |
| `area-variables` | an area with variables in its ARE, which the game does not read (it reads an area's from its GIT: `engine_area_variables`); Moonglow wrote them there before 1.34.3 | error |
| `nui-sources` | a NUI window's file (`.jui`, or the `.txt` beside one) that a nasher project's include leaves out: in the sources where nasher doesn't pack it and Moonglow doesn't read it, or the module's, to be written there; said with the `include` line that takes it in (The Domains of Dread's project, made with `src/**/*.{nss,json}`, lost a window this way) | warning |

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
  `placeables.2da` rows 500–700, which its hak's copy leaves empty; two
  spells of its `spells.2da` (rows 807 and 809) name feats 3000 and
  2589 of a `feat.2da` of 1,607 rows (`2da-ref`).
- **Tyrants of the Moonsea:** a blueprint for appearance 15028, past
  its hak's `appearance.2da`, and two of `mg-set`'s warnings on a
  tileset of the hak (`set-data`).
- **Models kept as text** (`mdl-text`, all warnings: a second node of
  a name, a robe with more than four bones on a vertex): 13 and 8 in
  Darkness over Daggerford's two haks, 5 in Pirates of the Sword
  Coast's, one each in Doom of Icewind Dale's and Tyrants of the
  Moonsea's.

The test's `KNOWN` list is the record: this is that list in words.

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
