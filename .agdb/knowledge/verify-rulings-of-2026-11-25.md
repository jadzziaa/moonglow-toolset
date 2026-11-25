---
type: Decision
title: "Verify rulings of 2026-11-25: where each check lives and the pitfalls"
description: "How the four Verify and tileset rulings of 2026-11-25 were built (description mistakes, dds-luminance, item-model, tileset counts) and what is not obvious about them."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T23:07:09Z }
touches: [crates/mg-module/src/doctor.rs, crates/mg-rules/src/appearance.rs, crates/mg-set/src/edit.rs, crates/mg-module/src/table_desc.rs]
asks:
  - Why does Verify say every mistake of a table description and where does the list of them come from?
  - "Why is the item-model rule in mg-rules and not mg-preview, and what does it share with the preview?"
  - When does Verify not warn of a luminance DDS?
  - "How does add_tile number a tile when the tileset's Count is junk?"
fingerprint: v2-fe80aabe2847c0f7
created: 2026-11-25
author: augustaugur
---
Table descriptions: Description::read keeps each unreadable column or rule as a note AND in the new mistakes list (parse stops at the first). doctor's described_tables and apps/mg diff.rs (2da-check) use read; 2da-check counts each mistake as unread. Plugins' mg.twoda.describe still uses parse.

dds-luminance: DDS joins the custom keys of examine_groups; the check reads 128 bytes (read_start), and warns only where ResMan::texture_layer says this copy is the drawn one and no PLT of the name exists in the load order (the renderer lets a PLT win over a gray texture on colored parts). mg_image::dds::is_luminance_alone is luminance without alpha or FourCC.

item-model: mg-module cannot depend on mg-preview (mg-preview optionally depends on mg-module), so the rule is mg_rules::appearance::item_without_model, built on models::item_models and models::item_default_model (the preview's item_parts fallback uses the latter). A UTC blueprint's ItemList/Equip_ItemList entries name the item (InventoryRes/EquippedRes), resolved through the ResMan (the module must be layered in: tests add a module layer); a GIT creature's entries are the items themselves. Armor (ModelType 3) is skipped. Ground items in a GIT's List are not checked.

mg-set: the reader ignores sections past Count and reads up to the last section below it, so SetFile::listed(list, item) = last section below Count + 1 (below the reader's bound), holes counted. A section past a short Count is reused, as the rules test already required; numbering by the highest section instead broke rules_keep_their_counts_and_places.
