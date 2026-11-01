---
type: Research Note
title: 'The tile border: what the client draws past an area''s edge'
description: What the NWN:EE client draws past an area's edge (its tile border, the "void" that Aurora and Moonglow drew black), read from the client's pictures and its shader uniforms - which edge tile goes where, how it is turned, how deep, how it is lit - and how Moonglow's area view draws it (Tile Border, off by default).
tags: [research, tilesets, rendering, client, area-view]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-01T23:04:28Z }
verified:
  - { by: claude-code/claude-opus-5-5, at: 2026-11-01T22:54:51Z }
sources:
  - id: request
    resource: human:august (2026-11-01), passing on a builder's request on 3.1.4
    title: 'An option to simulate what the client draws for the void: faded geometry depending on tileset'
  - id: wiki-tutorial
    resource: https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial
    title: 'nwn.wiki: Tileset Construction Tutorial, Edge Tiles'
  - id: wiki-options
    resource: https://nwn.wiki/spaces/NWN1/pages/38175230
    title: 'nwn.wiki: Game Options (Enable Tile Border)'
  - id: wiki-render
    resource: https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes
    title: 'nwn.wiki: Render Distance with Fog and Skyboxes'
  - id: nwscript
    resource: the install's nwscript.nss (89.8193.37), SetAreaTileBorderDisabled and ReloadAreaBorder
    title: The game's script functions for the border
  - id: edge-2da
    resource: the install's ttr01_edge.2da, tcn01_edge.2da, tic01_edge.2da, tdc01_edge.2da, ttf02_edge.2da and the tilesets' .set files
    title: The tilesets' edge tables
  - id: readings
    resource: crates/mg-corpus-tests/tests/client_render/readings.rs (reading_tile_border), run 2026-11-01 on game 89.8193.37; pictures in target/test-output/client_reading_tile_border_* (not committed)
    title: The client's pictures and its material and light uniforms, beside Moonglow's area view
---

# The tile border: what the client draws past an area's edge

A builder asked for the area view to draw what the client draws past an
area's edge, where Aurora (and Moonglow until now) draws nothing.[^request]
The client draws the area's **tile border**: the tileset's edge tiles,
five deep, darkened. This note is what the client was seen to do, and how
Moonglow's area view draws it (`mg_area::border`, the toolbar's **Tile
Border**).

## What the game says

- Edge tiles surround the area out to five tiles beyond it; they can't be
  walked or reached, and are "grayed out"; a tileset lists them in
  `<tileset>_edge.2da` (row Z of the tile names by custom).[^wiki-tutorial]
- The client's Options › Video **Enable Tile Border** (`graphics.tile-borders.enabled`,
  on by default) draws them; off, levels "just stop in mid air".[^wiki-options]
- Tiles past the edge are drawn as far as the area's fog clip distance
  like any tile; the border is five tiles (50 m) deep, so a long clip
  distance shows its end.[^wiki-render]
- `SetAreaTileBorderDisabled(oArea, TRUE)` turns an area's off (at the
  client's next load of it); `ReloadAreaBorder` redraws it after
  `SetTile`.[^nwscript]
- Every base tileset has an edge table (`Corner1`, `Edge`, `Corner2`,
  `Height`, `Model`); interiors too: tic01's `Wall`/`Wall` row names
  `TIC01_O20_01`, a black cap.[^edge-2da]

## What the client draws

Read with `reading_tile_border` (a 4×4 area, the camera's limits and the
fog clip distance lifted, Enable Tile Border on, the area's tiles set by
hand; game 89.8193.37).[^readings]

**Which tiles.** Every edge tile of the area is continued outward by one
edge tile, the same model in each of the five cells beyond it: the row
whose `Corner1` and `Corner2` are the tile's two corners on that side
(their terrains, as the tile lies) and whose `Edge` is the crosser on
that side (a road, a stream, a wall: `****` for none). A stream leaving
the north side runs north to the end of the border as one stripe; a
grass and water side makes a channel of water out to the end. Past each
of the area's corners a square of five by five holds the row of the
corner's terrain alone (`Corner2` and `Edge` empty). A raised corner's
terrain is named with a `+` (`Grass+`); `Height` is the level the piece
stands at. A side no row fits shows nothing (an interior's walls have
their own row, the black cap).

**Order and turn.** The models are made for the **east** side: `Corner1`
is the side's north corner, `Corner2` its south one (looking out, left
and right). A side whose corners are listed the other way round (water
then grass, where the table has only grass then water) takes the row
turned half a turn: its water lands on the right side again. Seen from
straight above with an uneven side on each edge (`top2`, `top3`), the
streams, roads and channels line up with the client's to the pixel.

**Light.** The border is lit by the area's light (the same
`lightAreaAmbient` and `lightAreaDiffuse` as the area's tiles, no point
lights), but its meshes' `materialFrontAmbient` and
`materialFrontDiffuse` are both set to **half the sun's diffuse color, in
gamma space** (emissive 0): read back by painting the uniforms from a
patched `inc_standard` in the scratch override (`MG_MATERIAL=1` and `2`):

| Sun diffuse | Ambient | Border's material (of 255) | `(0.5·d)^2.2` |
|---|---|---|---|
| C0C0C0 | 606060 | 30 | 29.8 |
| FFFFFF | 000000 | 55 | 55.5 |
| 808080 | 000000 | 12 | 12.2 |
| 20 40 FF (B G R) | 000000 | 1, 3, 55 | 0.6, 2.7, 55.5 |
| 000000 | FFFFFF or 203040 | 0 | 0 |
| C0C0C0, at night with the moon's 404040 | 606060 | 3 | 2.7 |

The area's own tiles have 255 there. So a border with no sun diffuse is
black whatever the ambient, and under the usual light it shows at about
0.38 of the area's brightness: the "faded" ground. Far off, the client's
own fog (always on, from 30 m before the clip distance) blends it into
the fog's color; past the border is the fog's or sky's color. At night
the moon's diffuse takes the sun's place (`MG_MOON`: 3 of 255 for a moon
of 404040 under a sun of C0C0C0).

**Tilesets.** Rural (ttr01): grass, trees, water, roads and streams
carried on. City (tcn01): its houses, walls and water past the edge,
darkened. Dungeon (tdc01): floor and fences where the floor reaches the
edge. Interior (tic01) and a dungeon's walls: the black cap, so black
past the walls, as before.

## Moonglow's area view

`mg_area::border` lays the border out from the area's tiles, the tileset
and its edge table (`AreaModel::border`); the area scene draws it, with
**Tile Border** on, as tiles in the material `mg_area::border::material`
(`Instance::material`, which both of the renderer's paths give a mesh as
its ambient and diffuse), casting and taking no shadow. Beside the
client's pictures it measures within 0.2 of 255 over the border's grass,
water and trees and the area (`top3`, the game's shaders). At night (the
moon's light) the border measures 4.2 of 255 against the client's 4.5.
Past the border is the area's skybox, or without one its fog color: the
client clears to it whatever the fog's amount (the scratch area of the
slanted rural pictures has no skybox, a fog amount of 0 and a fog color
of 917E68, and the client's sky there is 104, 126, 145, that color).
Moonglow clears to the fog's color with the Tile Border on, as with Fog
on, and draws the skybox as it does without it. Not read in the client: a side
whose corners no row fits (Moonglow draws nothing there), a crosser with
no row of its own (Moonglow takes the row of the corners alone), and how
the corner squares are turned (their models are even all over).

**Off by default**: Aurora draws nothing past the edge, where the area
ends shows plainest so, and a builder asked for it as an option. On, an
area of 32 by 32 tiles draws 740 more tiles.

[^request]: human:august, 2026-11-01
[^wiki-tutorial]: nwn.wiki: Tileset Construction Tutorial, Edge Tiles
[^wiki-options]: nwn.wiki: Game Options
[^wiki-render]: nwn.wiki: Render Distance with Fog and Skyboxes
[^nwscript]: nwscript.nss: SetAreaTileBorderDisabled, ReloadAreaBorder
[^edge-2da]: The install's edge tables and .set files
[^readings]: `reading_tile_border`, 2026-11-01
