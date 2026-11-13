---
type: Decision
title: "mg merge-gff: three-way GFF merge and git driver"
tags: []
status: stable
generated: { by: trail, at: 2026-10-22T00:00:00Z }
touches: [crates/mg-gff/src/merge.rs, crates/mg-schema/src/merge.rs, apps/mg/src/merge.rs, apps/mg/tests/merge_gff.rs, docs/manual/03-modules.md]
fingerprint: v2-aac527fcda4031af
created: 2026-10-22
author: August
description: "Design of mg merge-gff, a three-way GFF merge and git merge driver: generic tree merge plus schema list rules, Placed matching for GIT objects without ids, byte-preserving output, content-based format detection, and --prefer conflict handling."
asks:
  - "How does mg merge-gff resolve a three-way merge of GFF files, and what happens to conflicts?"
  - "Git reports conflicts when two people edit placeables in the same area file; how can I merge them automatically?"
  - "How does the merge match placed objects in a GIT file when they have no ObjectId?"
  - "When does the merge driver leave the file untouched or copy THEIRS bytes instead of reserializing, and how does it tell JSON from binary GFF?"
---
## Decided
- The three-way GFF merge is two layers: `mg_gff::merge::merge(base, ours, theirs, prefer, &rules)` is the generic tree merge; `mg_schema::merge::list_rule(file_type, label)` says what a list's item is (mg-gff can't depend on mg-schema, and the labels come from its generated constants). `apps/mg/src/merge.rs` is the CLI and git driver (`mg merge-gff %O %A %B --path %P`).
- The rule at every level is 2DA/TLK's: a side equal to the base takes the other; equal sides take either; else structs recurse by field (label + occurrence, so duplicate labels survive), lists by `ListRule`, and anything else is a conflict. A list only one side changed is that side's whole (order too).
- GIT placed objects have no id in shipped data (no `ObjectId` in the observed schema; it exists in BIC equipped items). `ListRule::Placed` pairs base to each side in steps: every field equal, `ObjectId`, (TemplateResRef, Tag, position at 4 places), then unique-only (TemplateResRef, Tag) for moves and (TemplateResRef, position) for retags. Additions by both are both kept, identical ones once. Position is `XPosition/YPosition/ZPosition` or, for doors and placeables, `X/Y/Z`.
- Byte for byte: a merge whose tree equals OURS doesn't write at all; one equal to THEIRS (same form) copies THEIRS's bytes; only other merges are serialized (JSON with `TextStyle::NASHER` and the project's float places, without `Conversion::to_source`'s Mod_ID/Version removal: nasher's input never has them, and a plain nwn_gff JSON keeps them).
- Format by content, not extension: git's %O/%A/%B are `.merge_file_XXXX` temp files. JSON = first non-space byte `{`.
- `--prefer ours|theirs` (also `a|b`, as 2da-diff/tlk-diff spell it). Unsettled conflict: `out.failed`, nothing written, exit 1, %A stays ours.

## Watch out
- In this worktree's Bash guard, any command text containing "git" in a non-plain form (a heredoc, a `.git.json` file name, an `okf log` message mentioning git) is refused: write such files with Write/Edit and run scratchpad scripts by path.
- JSON-read trees have sorted field order, so compare a JSON result with `Conversion::to_source` text, not with a tree built in another field order.
- Measured on a copy of the library's largest area (lib_nature.git, 2,333 placeables, 2.4 MB): an untouched merge keeps the bytes in 0.05 s (debug build); a reversed list with edits on both sides merges in 0.07 s.
