---
type: Decision
title: "A creature's preview is drawn from DressedLook alone; the wear check is the reader's"
description: "Why mg-preview's creature drawing takes only DressedLook values (fields read in look.rs), where the wear check lives, and what that means for a client writing creatures as GIT fields."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T12:42:02Z }
touches: [crates/mg-preview/src/look.rs, "crates/mg-preview/src/creature.rs:dressed_with", "crates/mg-preview/src/item.rs:item_parts"]
asks:
  - where does the creature preview read UTC fields
  - how do I draw a creature from values without a UTC
  - "why are a client's creature's weapons or armor not drawn"
  - how is the DressedLook kept in step with the UTC path
fingerprint: v2-a8634dcff1e255ed
created: 2026-11-19
author: augustaugur
---
Since Moonglow Client's request A3 (2026-11-19) the creature preview reads no Struct: creature::dressed_with(game, &DressedLook, remembered) draws, and item_parts takes &ItemLook. Every field a preview reads of a UTC or a worn item is read in look.rs (DressedLook::read, ItemLook::from_uti), so the UTC path (creature, of_object) and values given by a client (dressed_look) cannot drift: add a new field the preview needs to DressedLook/ItemLook, its reader and to_utc/to_uti, never a Struct read in creature.rs. mg-rules calls that take a Struct (AppearanceRow::of, models::skeletons) get DressedLook::rules_fields, built from the values. The wear check (GameData::wear_problem: FeatList, ClassList, Race, alignment) is not look: from_utc_as_the_game applies it, dressed_look draws what it is given. Consequence: a creature written as GIT fields with no FeatList (Moonglow Client's mc-scene before it used dressed_look) loses every item that needs a proficiency feat (weapons, armor past cloth, shields) when drawn through AreaModel::read. Checked when it landed: the Debug dump of every installed creature's and item's Preview was byte-identical before and after (1582 UTCs, 2763 UTIs as man and woman).
