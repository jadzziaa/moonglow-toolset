---
type: Decision
title: "Map notes on the minimap: what the client's map draws"
tags: []
status: stable
generated: { by: trail, at: 2026-11-05T00:00:00Z }
touches: [crates/mg-module/src/minimap.rs, crates/mg-corpus-tests/tests/client_minimap.rs, crates/mg-ui/src/minimap_export.rs, apps/mg/src/main.rs]
fingerprint: v2-22a67522fd54798c
created: 2026-11-05
author: August
description: "The client map draws a waypoint's note only as a 12 px `ctl_map_pin` GUI model when HasMapNote and MapNoteEnabled are both set, with no text; Moonglow reproduces it within 2 px."
asks:
  - "When does the NWN client show a map note pin on the minimap for a waypoint?"
  - "Why can't I find ctl_map_pin.tga or .dds with mg cat, and how do I extract and read it?"
  - "My waypoint's map note text doesn't appear on the map; is the client supposed to draw it?"
  - "How accurately does Moonglow place map note pins compared to the client, and where does it read the pin from?"
---
## Decided
- The client's map marks a waypoint's note only with HasMapNote and MapNoteEnabled both set; the mark is the GUI model `ctl_map_pin` (a 0.1205-unit quad = 12 px, its UVs a 12x12 texel square of `gui_map_pieces`, mesh offset 0.01 right / 0.01 down), drawn unscaled over a map of 16 px tiles (the game's mi_ pictures are 16x16). No note text is drawn on the map.
- Moonglow reads the pin from the load order (`mg_module::minimap::pin`) and marks the waypoint's true place; the client's pins are within 2 px (`map_notes_are_marked_as_the_client_marks_them`, ignored, off-screen display).
- Two menu items ("... with Map Notes...") instead of a checkbox: egui 0.36's context menu closes on any click inside it (MenuConfig close_behavior comes from the menu root, not per submenu).
## Tried and ruled out
- `mg cat ctl_map_pin.tga/.dds`: not found. The ctl_map_* names in the keys are GUI models; `nwn_resman_extract -p ctl_map_ -d DIR` finds them, `mg cat --text NAME.mdl` decompiles.
- Reproducing the client's pixel rule exactly: on 4x3 and 8x6 areas every pin fits left = floor(s*x) - 3, top = floor(s*(height_m - y)) - 4 with s = (16*W - 2)/(10*W) px/m; a 3x5 (tall) area's map is drawn in two grays and the color masks didn't settle whether width or height sets s. Left as the true position.
## Watch out
- `client_screenshot`'s winshot shows GUI hover state poorly: a hover over a pin showed no tooltip; whether one appears wasn't settled.
- A client started with `setsid ... &` from a Bash call dies when the call returns; run run-client.sh as a background task and stop it with TaskStop.
- Run the suite with an absolute TMPDIR: a relative one fails sign_release (mktemp from another cwd) and a scripts compile test.
