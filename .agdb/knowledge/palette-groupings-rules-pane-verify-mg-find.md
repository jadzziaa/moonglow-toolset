---
type: Decision
title: "Palette groupings: rules, pane, Verify, mg find"
tags: []
status: stable
generated: { by: trail, at: 2026-11-03T00:00:00Z }
touches: [docs/log.md, docs/manual/03-modules.md, docs/manual/05-blueprints.md, docs/manual/09-build-and-test.md, docs/manual/11-command-line.md, docs/palette-groupings-proposal.md, crates/mg-corpus-tests/tests/palettes.rs, crates/mg-module/src/doctor.rs, crates/mg-module/src/palette_groups/tests.rs, crates/mg-ui/src/lib.rs]
fingerprint: v2-86e6f5fffaf6c248
created: 2026-11-03
author: August
description: "Records how palette groupings (`[palette.<id>]`) are implemented in `mg_module::palette_groups`: Arc-held Groupings with a fingerprint, derived Palette for the pane, Verify and `mg find --palette-group` handling, plus TDoD naming and test pitfalls."
asks:
  - "Where do palette groupings live in the code and how does the pane avoid recomputing them every frame?"
  - "Why does my 'ham' palette rule match ferns like 'Autumn: Fern 01, after HAM (AUT)'?"
  - "Why does `mg find --palette-group` find TDoD placeables that normal module resource search never returns?"
  - "Why does `mg_testkit::corpus!()` fail in my helper, and why is `get_by_label(\"Edit\")` ambiguous in kittest?"
---
## Decided
- Palette groupings (`[palette.<id>]`, design A of docs/palette-groupings-proposal.md) live in `mg_module::palette_groups`: `Groupings` hangs on `ProjectFile.palette` as an `Arc` (the UI clones it every frame; `Arc::make_mut` where it is read), with a `fingerprint` (hash of the parsed groupings and lists) the pane's cache compares instead of the rules. The `from` files' stamps follow the four fixed ones in `ProjectFile.stamps`, in groupings' order, in `read_at` and `changed()` alike: a list rewritten is a project-file change.
- The pane draws a grouping as a derived `Palette` (`group_palette`: a node per group with blueprints, Other last, categories pruned under each when nested), so `Tree::node`, search, Gallery, menus and keys are unchanged. Rows are keyed by their path in the tree shown; `At` carries the grouping (id, nested) so a cursor isn't taken across groupings; the egui id salt has it too. Open groups are remembered as `utp/standard/@kind/<group>[/<original category path>]` via `Grouped.origin`.
- Verify's notes need palette names (talk-table strings): `doctor::palette_groups(module, resman, name, skip)` is called beside `talk_range`, in the window (GameData) and `mg verify` (base and custom Tlk kept, `text_in` with the game's codepage).
- `mg find --palette-group` lists palette entries (standard: the game's and haks'; custom: the module's), not module resources: TDoD's placeables are hak blueprints that `find_named` never sees.

## Watch out
- TDoD's names are not all `Theme: Name (source)`: `Autumn: Fern 01, after HAM (AUT)` puts "after HAM" before the bracket; a words_of must skip `, after …` or a "ham" rule takes ferns.
- `mg_testkit::corpus!()` returns `()` when there is no game: call it in the test body, not in a helper returning a value.
- Kittest: the menu bar's Edit makes `get_by_label("Edit")` ambiguous; count before and after a right-click.
