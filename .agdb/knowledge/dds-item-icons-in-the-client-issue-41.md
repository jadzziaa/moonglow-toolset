---
type: Decision
title: DDS item icons in the client (issue 41)
tags: []
status: stable
generated: { by: trail, at: 2026-11-03T00:00:00Z }
touches: [crates/mg-ui/src/images.rs, crates/mg-corpus-tests/tests/client_render/icons.rs, docs/research/notes_models.md]
fingerprint: v2-72b4fe231ac693b6
created: 2026-11-03
author: August
description: "Issue 41 finding: the game draws DDS icons with the first stored row at the bottom, so Moonglow's rows were kept; only DDS-only item icons differ (game shows red-cross DefaultIcon), so non-PLT icon layers now require a TGA."
asks:
  - "Why do DDS item icons look upside down in Moonglow, and does the game flip them too?"
  - "What does the game show for an item icon that has a DDS but no TGA?"
  - "How do I reliably screenshot many item icons from the client without the examine window lagging behind?"
  - "My test module loads then the client drops to the main menu after 'Loading Module' with 'Server shutting down'; what causes that?"
---
## Decided
- GitHub issue 41 (DDS icons upside down): read in the client, `client_render`'s `reading_icon_rows` (icons.rs), one client run per case. The game draws DDS icons (both kinds) with the first stored row at the bottom, as Moonglow; it picks DDS vs TGA exactly as `ResMan::texture_over` in ten layer arrangements. Rows and file choice were not changed.
- What differed: an item icon that is only a DDS (no TGA of its name in any layer) is drawn by the game as the base item's `DefaultIcon` (`iinvalid_1x1`, red cross). Moonglow now gates non-PLT icon layers on a TGA existing (`images::Loader::icon_picture`); every base-game item-icon DDS has its TGA, so no game icon is lost.
## Tried and ruled out
- One client run with every case in the bag, examined in turn (`ActionExamine` every few seconds, screenshots on a timer or on the log line): the examine window's icon and title lag a case behind, unpredictably. One run per case (item made, inventory opened, examined, one picture) is clean and takes ~10 s a case.
## Watch out
- A module whose hak list names an empty hak (an ERF with no resources) does not load: the client logs "Server shutting down" right after "Loading Module" and sits in the main menu until the 180 s timeout. List only haks with files.
- `GUI_PANEL_EXAMINE_*` cannot be opened by `PopUpGUIPanel`; `AssignCommand(pc, ActionExamine(item))` opens the examine window.
- Not measured: PLT icons, composite and armor icons, portraits, whether a standard DXT1 or uncompressed DDS is drawn with a TGA beside it.
