---
type: Fact
title: "Item Wizard: Low rules and the jewelry table settled by 354 Aurora items"
description: "Aurora's Item Wizard: Low items' later tiers are one or two below, up to four properties, cap at character level 4L+1; PropColumn 16 (jewelry) uses itmwizhelmet; the captures and how they were driven."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T21:34:02Z }
touches: [crates/mg-rules/src/item_wizard.rs]
asks:
  - "Which wizard table do amulets, belts, boots, bracers, cloaks and rings use?"
  - "What is a Low magical item's most worth in the Item Wizard?"
  - "How were Aurora's wizard items captured, and what went wrong driving it?"
  - How many properties does a Low wizard item get?
fingerprint: v2-ef6154c9c1ffb9f7
created: 2026-11-26
author: augustaugur
---
Captures: ~/.local/share/moonglow-oracle/captures/item-wizard/ (127) and item-wizard-2/ (227; manifest line: file, level band, quality), both read by aurora_item_wizard.rs (asserts >= 300 checked). Under the old rules (Low later tiers two below only, at most 2, caps at levels 1/5/8/13, column 16 = armor table) 78 of them failed. This updates the 127-item fit in [[item-wizard-random-properties-and-advanced-search-actions]].

Findings: a Low item's later tiers are one or two below, as at the other qualities (ar1000, 6-10 Low, has a tier-1 second); seven 6-10 Low items have three properties; Low maxima per band 434, 4863, 12350, 32430, within itemvalue MAXSINGLEITEMVALUE at levels 1, 5, 9, 13 (4L+1). Medium to Godly items mostly have 2 properties; the singles come from an expensive first one (Regeneration), so "add while it fits" stays. Column 16: 48 jewelry items at 6-10 Low (first property tier 2) drew only the helmet table's 8 tier-2 rows, never the armor table's Weight Reduction (which itemprops 16_Misc allows): p about 0.4% under the armor table. A 16-20 Godly plan can't tell the tables apart, since tier 2 is never the first's tier there.

Driving Aurora's wizard with xdrive: click the palette tree and press Home before the right-click (the tree scrolls as items are added, and the right-click otherwise lands on an item and opens Item Properties). Level and quality radio clicks are lost now and then: click both twice with pauses, move the pointer away, click the quality again. Save (ctrl+s) every 30 items.
