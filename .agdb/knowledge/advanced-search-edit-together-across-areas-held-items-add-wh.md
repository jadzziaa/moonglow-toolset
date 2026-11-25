---
type: Decision
title: "Advanced Search: Edit Together across areas, held items, Add where missing"
description: "Why Tab::Instances holds each object's area, how held items are grouped for Edit Together, why Add where missing is off by default and typed by mg_schema, and why conversations are not edited together."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T21:18:34Z }
touches: ["crates/mg-ui/src/tabs.rs:Tab", "crates/mg-ui/src/search.rs:together", "crates/mg-edit/src/found.rs:fields_of", crates/mg-ui/src/open_tabs.rs]
asks:
  - How do I edit placed objects from several areas together in one Properties window?
  - "Why does Advanced Search's Set a field not add a field an object lacks unless asked?"
  - Where does the type of a GFF field added to a placed object or held item come from?
  - Can conversations or items in a bag be opened with Edit Together from Advanced Search?
fingerprint: v2-00e1ba3dc617d787
created: 2026-11-25
author: augustaugur
---
## Decided (2026-11-25, closing the deferred entry "Advanced Search, what it doesn't do")
- `Tab::Instances` is `{ objects: Vec<(ResRef, GffPath)> }`: each object its own area. `edit_many`, `Also`, `partial` and `FieldTarget::command` already took a document per object, so a field set in the window lands in each area's GIT, one Command, one Undo; Cancel too. Changing the shape (not adding a variant) made the compiler stop at every "tabs of this area" check (`Tab::edits_in`, close on delete, popped-out panel); a new variant would have slipped past `_ =>` arms. Helpers: `Tab::instances(area, paths)`, `Tab::one_area`, `Tab::areas_of`. Kept tabs: one area still `instances:area:paths`; several `instances-of-areas:area:path,...` (`open_tabs.rs`, `object_text`).
- `Tab::tree_keys` names every area of the tab, so `shows_any` closes it when any of its areas is deleted and `retarget` renames each; `tree_key` (singular) is the first's.
- `tab_of` used to drop `also`'s keys (`(_, p)`): it now keeps each area. Grep for that habit before trusting a cross-document window.
- Held items: `search::together` groups `o.placed() && blueprint::held_item(path)` (the path ends exactly at an ItemList/Equip_ItemList entry, store pages too) as `Placed { kind: None }`, apart from item blueprints (other pages: Comments) and items lying in an area (`List`). An item inside a bag is not held_item (the path goes on) and is left out.
- Add where missing (`found::Change::SetField { add }`): off by default because the engine reads a missing field as its default, and adding one changes what it reads. Only where the object's kind has the label in the observed schema (`mg_schema`; `found::fields_of`: root fields for blueprints/areas, the GIT list module's FIELDS for placed objects, `item_list`/`equip_item_list`/store `item_list` for held items), of that type, spelled as the schema spells it; others are skipped "no X". `mg areas` already did the same for ARE fields (`root_field_type`). mg-edit now depends on mg-schema.
- Conversations: settled by design (deferred.md, Settled): a conversation is one tree in its own editor with no shared form; Change the N checked already sets a script or field on each line found.

## Watch out
- Advanced Search's hits come sorted by resource: objects of "field" come before "start", so a test comparing the tab's objects sorts them.
- The item form's plot check is labeled "Plot item", not "Plot".
- codemode's `edit()` is async: an unawaited call silently does nothing (or races).
