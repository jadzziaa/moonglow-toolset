---
type: Decision
title: "Engine readings: placeable walkmesh node position, tileset line rules"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/engine_placeable_walkmesh.rs, crates/mg-corpus-tests/tests/engine_tileset_holes.rs, crates/mg-module/src/doctor.rs]
fingerprint: v2-6ba635d5258c5ff6
created: 2026-10-30
author: August
description: "The server honors a placeable walkmesh's node position in pathing, and any tileset line over 255 characters (or over 256 bytes after the last CRLF) stops a module loading; also covers test setup tips."
asks:
  - "Why won't my module load when a tileset file has a very long line, and what is the exact length limit?"
  - "Does the server use the node position in a placeable's .pwk walkmesh when a creature paths around it?"
  - "How do I set up a creature to test pathing on a walkmesh in a server with no players?"
  - "Why do quads in my ASCII test model look invisible, and which winding order faces the viewer?"
---
Decided: the server honors a placeable walkmesh's node position (a creature's ActionMoveToLocation path goes round a square whose node sits 15 m from its placeable, and straight over the placeable's own spot); module and override copies of a .pwk are both read. Measure pathing with paths, not CreateObject: a creature made on a mesh is not moved off it reliably. Walkers need a UTC with its Script* fields emptied and SetAILevel(AI_LEVEL_NORMAL) in a server with no players. Tileset rule: a line of more than 255 characters (CR/LF not counted) stops a module loading anywhere, however it ends; more than 256 bytes after the last CRLF do too; other LF lines read as CRLF. Ruled out: the earlier '389-byte LF line at the end' reading was the line's length, not its LF. Run failing-to-load cases side by side in threads (each waits out its no_load timeout). Watch out: quads in ASCII test models face the viewer only with the winding 0 2 1 (models face +Y); the hak editor's model view must not keep an Arc<GameData> past Moonglow::release_game_data.
