---
type: Decision
title: "Builders' asks of 2026-11-16: tags in the module tree, creature Sound Set and Faction on Basic, portrait with appearance, area copy tagged as its ResRef"
description: "Why the module tree shows tags as it does, why a creature's Sound Set and Faction left Advanced, why the appearance-portrait option is off by default, and how an area copy's tag follows its ResRef."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T12:36:44Z }
touches: [crates/mg-ui/src/tree.rs, crates/mg-ui/src/copy_as.rs, crates/mg-ui/src/blueprint/creature.rs, "crates/mg-rules/src/appearance.rs:appearance_portrait"]
asks:
  - why does the module tree show tags in parentheses
  - "why is a creature's faction on the Basic page"
  - "does changing a creature's appearance change its portrait"
  - why does an area copy get its resref as its tag
fingerprint: v2-f5929e0533470be0
created: 2026-11-16
author: august
---
## Decided
- Options › General › Show tags beside names in the module tree (`Settings::tree_tags`, off): blueprints `label (tag)` after what they are listed by (`Name (resref) (tag)`, `x.utc (tag)`), baked into the groups' `names` map (so the filter finds tags; the groups' `Shape` carries the option); placed objects `Name (tag)` drawn in `contents_ui` from `ObjectRow.name/tag` (`Look::Tree { tags }`), waypoints too. The cached sort stays: waypoints by tag. Type-ahead (`row_text`) still matches the plain text.
- Creature Sound Set (after Gender) and Faction (after Category) moved to Basic: Carcerian's ask, Qlippoth's vote for after Gender. Deliberately different from Aurora in the parity checklist.
- `Settings::appearance_portrait` (off: Aurora's inventory says nothing of cbAppearanceChange setting a portrait, so the old behavior stays the default). `with_its_portrait` beside `with_a_body`, called at both appearance sites (Basic dropdown, Appearance Gallery), one command. The match: appearance.2da `PORTRAIT` minus `po_` against portraits.2da `BaseResRef`, case and trailing `_` aside, first row: all 320 PORTRAIT values of the base game resolve.
- Area Copy…: `CopyDraft::tag_follows` (areas only; blueprints keep the original's tag as Aurora's Edit Copy). The window mirrors the ResRef into the Tag until the Tag is edited, and `copy_resource` itself applies it, so a draft set by code (tests) gets it too. Tests that need the original's tag (Retag) set `tag_follows = false` (`keep_the_original_s_tag` in tests/app/areas.rs).
- With tags on, waypoints are listed and typed-ahead by `Name (tag)` (`KindContents::tagged_order`, `row_text(.., tags)`): sorted by tag while they show a name first, the list looked unsorted and a letter typed found nothing.
## Watch out
- Not captured from Aurora: whether its Create Copy of an area changes the tag, and whether its Appearance change (`cbAppearanceChange`) sets the portrait. Nothing claims either; capturing the second would settle the option's default.
