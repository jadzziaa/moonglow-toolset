---
type: Decision
title: "A tileset kept in the module: what reloads when its .set changes"
description: "Item 4 (2026-11-25): module_tilesets_changed reloads only open areas drawn from a changed module .set, plus the tiles palette's entries, a brush on it and its group pictures; the routes that reach it and the one that waits."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T17:49:44Z }
asks:
  - Why did an open area keep drawing the old tileset after a .set was imported into the module?
  - "Which paths into the game data's module layer redraw a changed tileset?"
  - "Why does Moonglow::tileset return a stale tileset while an area is open?"
created: 2026-11-25
author: augustaugur
---
Cause: there is no tileset cache in GameData; an open area view keeps the Tileset it read (and its terrain tools), and Moonglow::tileset returns the open view's copy first, so a .set changed in the module layer was never read again until the area was reopened. Fix: update_module_layer (files changed on disk) passes its SET keys, refresh_module_layer (import, Save, builds, before jobs) compares the old layer's SET Arcs with the module's (sets_changed, as drawn_with_changed does for textures), and module_tilesets_changed reloads only views whose tileset_read() is one of them, forgets palette_view's tile_palettes/tileset_names/tileset_grass for them (and the tile brush/choice if on them) and Thumbnails' Group pictures. Not reloading every view on any module change was the point (the deferred entry feared that).
Watch out: a plugin's write reaches the layer only at the next refresh (Save, build, job): no refresh after a job's edits. Test: tileset_panels::a_tileset_kept_in_the_module_changed_is_redrawn drives Import Files with NoDialogs (area harness modules are .mod archives, so no outside watch).
