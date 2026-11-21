---
type: Decision
title: "Merging develop into the UI redesign (M4): where develop's features went and what the conflicts hid"
description: "How develop (cf73a288: Builders' asks, Moonglow Client's asks, OpenGL fixes) was merged into ui-redesign's M4: develop's creature fields drawn by the redesigned Basic page, its Options in U23's section, the log merged by date, and the deferred count's method."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T11:59:22Z }
touches: ["crates/mg-ui/src/blueprint/creature.rs:sound_set", "crates/mg-ui/src/options.rs:general_section", docs/log.md, docs/deferred.md, docs/ui-redesign-moves.md]
asks:
  - "Where did develop's creature Sound Set and Faction go in the redesigned creature form?"
  - "A merge of develop into ui-redesign conflicts in options.rs or creature.rs: which side's layout wins?"
  - "The deferred count did not change after develop rewrote the OpenGL entry: why?"
  - How were docs/log.md conflicts between develop and ui-redesign resolved?
fingerprint: v2-669245414886d8e6
created: 2026-11-21
author: augustaugur
---
## Decided (the owner's rule for the merge)
Both sides survive: the redesign's layout, names and tests stand; develop's features are carried into it, never put back in the old layout.

- **Creature Basic page** (`blueprint/creature.rs`): develop moved Sound Set and Faction from Advanced to Basic in the old flat grid. In the redesign the Basic page is `FormColumns` of sections; Sound set goes after Gender in "Race and Appearance" (before the Preview row), Faction with Edit Factions after Category in "Palette", as develop's own moves rows already said. The Sound set row is the Advanced page's row as U10 drew it (filters on a line of their own where `room_right < SOUND_SET_ROW`), moved into `sound_set(f, ui)`, which reads the soundset choices itself (develop had dropped `sounds` from `advanced`, and git auto-merged that drop, so HEAD's Advanced row would not have compiled anyway).
- Labels keep the redesign's sentence case ("Sound set", "Treasure model"), so develop's test `a_creature_s_sound_set_and_faction_are_on_its_basic_page` was adapted to those labels and to `page(&mut h, "Advanced")` (pages follow one another on one scroll; a page out of sight draws nothing, so the query for "Sound set" on Advanced is not vacuous: seen to fail with the Faction label broken).
- `with_its_portrait` (Set a creature's portrait with its appearance) is called beside `with_a_body` in the redesigned Appearance row and in the Appearance Gallery.
- **Options**: develop's two checkboxes go into `general_section`'s "Module Tree and Palettes" arm (U23's sections), after Show ResRefs beside names and after Show armor and cloaks on a woman.
- **docs/log.md**: one conflict block holding all of both sides' recent days; merged by splitting each side by `## date` and appending develop's entries at the end of the same day's section (entries counted: base + HEAD's + develop's = merged).
- **docs/deferred.md**: count `^- \*\*` only counts top-level entries; develop's OpenGL change is a nested `  - **` item, so develop's count equals the base's. The account is done by diffing the sorted bold titles of base, HEAD, develop and the merge.
- Frontmatter: the later `generated.at` kept on each conflicted stamp (the brief's rule), not restamped.

## Watch out
- The module tree's tags are develop's text (`Name (tag)`) in the kit's `TreeRow`, which is not `fit`: a long row extends and the tree scrolls sideways (no ellipsis), while the Outliner draws the tag weak after the name with `.second(tag)`. Whether the tree should take the Outliner's weak tag is a design question left to the owner.
- Picturing the fog nearer than 20 m on Medieval City 2: the city's ground is raised above its tiles, so an orbit with its target at z 0 and a low pitch puts the eye under the ground (all fog color, the grid at the area's foot seen from below). Raise `orbit.target.z` (8 m worked) before judging the fog.
