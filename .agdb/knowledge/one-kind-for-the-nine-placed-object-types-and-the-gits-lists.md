---
type: Decision
title: "One kind for the nine placed object types, and the GIT's lists through it"
tags: []
status: stable
generated: { by: trail, at: 2026-10-21T00:00:00Z }
touches: [crates/mg-rules/src/objects.rs, crates/mg-rules/src/rows.rs, crates/mg-area/src/lib.rs, crates/mg-module/src/palette.rs, crates/mg-module/src/verify.rs, crates/mg-module/src/instances.rs, crates/mg-module/src/refs.rs]
fingerprint: v2-a2e1b9557f86d25b
created: 2026-10-21
author: August
description: "The nine placed object types are unified into one enum, mg_rules::ObjectKind, with mg_area and mg_module re-exporting it; the note covers saved-settings label pitfalls, 'Merchant' naming, refs.rs and rows.rs exceptions, and worktree Bash limits."
asks:
  - "Where is the single enum for placed object types like store and sound defined, and what does it hold?"
  - "Why did renaming the Merchants label to Stores break saved hidden_in_areas settings, and how is that handled?"
  - "Why does rows.rs have no Sound or Store kinds, and what does child_kind return for them?"
  - "Why do Bash heredocs and sed -f fail in my worktree session, and what should I use instead?"
---
## Decided
- The nine types of placed object are one enum, `mg_rules::ObjectKind` (`crates/mg-rules/src/objects.rs`): blueprint type, GIT list label and struct id (from mg-schema's generated `git::*_LIST`, so mg-rules depends on mg-schema now), the names (`name` "store", `label` "Stores"; the owner chose Stores everywhere, unlike Aurora's Merchants, so there is no `plural`), `resref_field` and `resref_field_of(ResType)`, `palette_field`, `name_field`, the palette keys and the area editor's predicates. It is in mg-rules because `rows.rs` needs it, and mg-rules already has mg-resman (`ResKey` for `custom_key` and `skeleton_key`).
- `mg_area::ObjectKind` and `mg_module::palette::BlueprintKind` are `pub use` re-exports of it, so the hundreds of call sites in mg-ui compile unchanged. Its methods can only live in mg-rules (no inherent impls on a foreign type).
- `verify::Category` keeps its variants (its `{:?}` is shown in the Build window and in `mg verify`'s JSON) and takes the nine through `From<ObjectKind>`. `instances::GIT_LISTS` is gone (`ObjectKind::ALL`); `git_list` stays, one line, for its ~20 callers.

## Watch out
- `Settings::hidden_in_areas` stores the kinds by `ObjectKind::label`: renaming a label breaks saved settings unless `area_view::shown_of` reads the old name too (it reads "Merchants" as Stores). `merchant_signs` keeps its field name for the same reason.
- "Merchant" stays where it is the game's word: the Merchant faction, and the Merchants category of the game's `storepal.itp` (the Store Wizard's test clicks it).
- `refs.rs` `list_blueprint` must not map `List` (other files use the label for other things); `classify` takes a GIT's `List` first. `ObjectKind::from_list("List")` is Item, so it is filtered out there (a GIC test holds it).
- `rows.rs` has no Sound or Store kinds on purpose: they map to its `Other`, and `child_kind` returns None for them so the search goes on outward.
- In a worktree session Bash refuses heredocs, `sed -f`, `trail note -m` with long text and `$var` arguments: use Edit/Write, or `sed -i` with literal paths.
