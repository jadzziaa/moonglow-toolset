---
type: Decision
title: "mg merge-gff: conversation lines renumbered, and the conflict file"
description: "How mg merge-gff keeps lines both sides added to a conversation (theirs renumbered after ours), the guard that turns deletions and reorders into one /(lines) conflict, and the NAME.conflicts edit file for mg apply written on a conflict."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T22:45:04Z }
touches: [crates/mg-module/src/merge.rs, crates/mg-edit/src/diff.rs, apps/mg/src/merge.rs, crates/mg-gff/src/merge.rs, crates/mg-schema/src/merge.rs, crates/mg-edit/src/wire.rs]
asks:
  - How does mg merge-gff merge two builders adding nodes to one dlg conversation?
  - Why does a conversation merge give a /(lines) conflict?
  - "What is the .conflicts file mg merge-gff writes beside a file, and how are its edits ordered?"
  - How do I take one side per conflict after a failed GFF merge?
fingerprint: v2-28524ccd9dfa7011
created: 2026-11-26
author: augustaugur
---
## Decided
- **Conversations** (`mg_module::merge`, not mg-schema: it needs `dialog::Renumbering::by_lines` and `dialog::renumber`, and mg-schema depends only on mg-core/mg-gff). Lines (`EntryList`, `ReplyList`) merge by place, so theirs is rebased first: where both sides added lines of a kind (and not the same ones), theirs' new lines go after ours' (ours' new ones copied into theirs' list at their numbers, so the by-place merge sees them on both sides), every link of theirs with `Index >= base count` shifted by ours' added count (`StartingList` and `EntriesList` point at entries, `RepliesList` at replies; `IsChild` links too), and a struct id that was its place moves with it. New lines both sides begin with the same (a cherry-pick) are taken as one: the shift starts after the leading equal new lines. mg-schema's rules key link lists by target: `StartingList` by `Index` (its links have no `IsChild`), `RepliesList`/`EntriesList` by `Index`+`IsChild`, so both sides' links appended to one line both stay.
- **Guard** (`in_place`): a side keeps the base's numbering only if it has at least as many lines of each kind, every owning link of the base (`IsChild` 0, and starting links) is still there at each base line (multiset), and `Renumbering::by_lines(base, side)` finds no base line at another number (`Some(j)`, j != i; `None` is fine: a text edit). Failing that on either side while both changed the lines differently: ONE conflict `/(lines)` (`mg_module::merge::LINES_CONFLICT`), number 0, taking one side's three lists whole (per-list conflicts let `--prefer theirs` pair one side's lines with the other's links). Moonglow's `move_link` trips it too (safe direction).
- `NumWords` both changed: O + T - B. When both sides are as Aurora writes them (`dialog::renumber` leaves them unchanged), the merge gets `dialog::renumber` (ids by place, word count recounted).
- **Conflict file**: on an unsettled conflict, `%A` stays raw ours and exit 1, and `NAME.conflicts` is written beside `--path` (else OURS; resource name from the file name minus `.json`/`.nwnt`, else stem + GFF type). Not `.json`: nasher's `src/**/*.{nss,json}` would take it. Contents: an `mg apply` edit file, first `gff_edits(ours -> merge with ours' side of all)` (theirs' clean changes, must be kept), then per conflict k `gff_edits(M_ours -> merge_choosing(k theirs, rest ours))`, groups sorted by first edit's place descending (ties: later conflict first) so any subset applies. Groups whose edits overlap (one inserts/removes items of a list over a stretch the other edits within, by doubled places: item i = 2i, insert before i = 2i-1; e.g. an `(order)` conflict and a value in that list) are joined and their edits recomputed for both (`overlap` in apps/mg/src/merge.rs); field edits on one item, or inserts at one place, don't join. 286 conflicts in a 2,000-object area: 0.8 s. Each edit has a `note` (wire.rs now reads past `note`, `mg_edit::wire::NOTE`); the JSON maps each conflict to its edit indices (`conflicts[i].edits`).
- `mg_gff::merge::merge_choosing(.., choose: &dyn Fn(usize) -> Prefer, ..)`: conflicts are numbered in discovery order, which doesn't depend on the choices.
- `mg_edit::diff::gff_edits`: struct fields set/removed/appended (field order not kept); lists: common prefix/suffix, LCS of the middle (capped at 1M pairs), gaps of equal length edited in place, others removed and inserted; edits go last place first. A struct id change or duplicate labels: the item replaced (remove + insert), a struct field set whole, the root: `SetResource`.

## Watch out
- Aurora numbers a conversation's lines anew on every save (`aurora_dialog.rs`; capdlg shows a stack-order walk of the tree), so lines added in Aurora shift others: such merges are the `/(lines)` conflict, not renumbered.
- The second merge: after a branch merged the other's lines (numbered after its own), the next merge's base has the other branch's numbers, so lines both add again are the `/(lines)` conflict (a rebase of two line-adding commits hits it at the second). Filed in docs/deferred.md with deletion: both need lines matched by what they are.
- What the guard can't see: the last line deleted and another added under the same parent at the same place (indistinguishable from editing it).
- Tests: `crates/mg-module/src/merge.rs` (oracle: ours with theirs' additions replayed by `add_node`), `apps/mg/tests/merge_gff.rs` (every subset of three conflicts through `mg apply`; the git driver on a nasher project needs `[target.sources] include` in nasher.cfg or `mg apply` finds no resource).
