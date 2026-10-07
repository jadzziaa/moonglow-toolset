---
type: Proposal
title: 'Lights placed on the fly: a proposal'
description: A proposal for placing a light in an area without making a custom placeable by hand first - what the game offers for lights, three ways Moonglow could do it (stock light placeables, generated content, scripted effects), what each costs, a recommended order and what must be measured first.
tags: [proposal, lights, placeables, areas, custom-content]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-07T21:52:10Z }
sources:
  - id: request
    resource: human:august, relaying the builder dafena (Discord, 2026-10-07)
    title: The request - paint a light with default values, then customize it
  - id: placeables
    resource: placeables.2da and lightcolor.2da in the installed game (read with mg cat, 2026-10-07)
    title: The game's own tables
---

# Lights placed on the fly: a proposal

Status: way A was chosen first (2026-10-07) and is built: **Add Light
Here** and **Light Color** on the area's right-click menu, and a marker
for lights that have nothing to see (see [Areas](manual/04-areas.md)).
The reach is measured and the ring drawn (see [Measured](#measured)).
The 2DA merger and way B are not started. What is still marked *to
measure* is not known yet.

## Summary

A builder asked for lights that can be painted into an area with default
values and then adjusted (color, reach, brightness), without first making
a custom placeable that carries the light.[^request]

The game has no "light" object. A light in an area is always something
else that happens to shine: a tile, a placeable, a visual effect. So a
light tool is a tool that makes or picks one of those for the builder.
Three ways are open, and they differ in what the module needs afterward:

| Way | What the builder gets | What the module needs |
| --- | --- | --- |
| A. Stock light placeables | Seven colors, fixed reach | Nothing: base game |
| B. Generated content | Any color, reach, brightness, shadows, flicker | A hak Moonglow writes (models and a placeables.2da) |
| C. Scripted effects | The game's fixed light effects, switchable in play | A script on an invisible object |

The recommendation is A first (small, no custom content, useful at
once), then B as the real feature, with C left out unless asked for.

## What the game offers

- **A placeable's row can carry a light.** placeables.2da has
  `LightColor` (a lightcolor.2da row) and `LightOffsetX/Y/Z`.[^placeables]
  The game ships seven invisible ones made for this: rows 15107 to 15113,
  "Light, Blue" to "Light, Yellow", model `dag_invisible`, each a
  different `LightColor`. The older Shaft of Light placeables (rows 166
  to 173) do the same with a visible beam. Moonglow already reads these
  columns and lights its view with them (`mg-preview`, `object.rs`).
- **A model can carry a light node**: any color, a radius, a multiplier,
  shadows, a flare, and animation of those (a flicker). This is how
  torches and braziers shine. One model is one light: a different reach
  is a different model.
- **A tile has main and source lights** (Tile Properties), colors from
  the same lightcolor.2da. Fixed places, one tile at a time.
- **A script can put a light effect on an object**
  (`EffectVisualEffect` with the `VFX_DUR_LIGHT_*` effects): fixed colors
  and sizes, and it needs something to run it.
- lightcolor.2da has 32 colors.[^placeables] A color outside it is only
  possible through a model's light node (way B) or a custom
  lightcolor.2da.

*To measure* still (in the game client, with the oracle the renderer
uses): whether a placeable's `LightColor` light and a model's light node
light creatures and placeables or only tiles; how many dynamic lights
the game draws at one place before dropping some.

### Measured

In the game client (`placeable_light_uniforms` in
`crates/mg-corpus-tests/tests/client_render.rs`, which reads the lights
the client hands its shader in a dark area), 2026-10-07:

| Placeable | lightcolor.2da | The client's light |
| --- | --- | --- |
| Light, White (15112), static, 1.5 m up | DimWhite 0.60 | one light, color 0.325, ends at 20 m |
| the same, not static | | the same |
| the same, on the ground | | the same |
| Light, Red (15111) | 1.00, 0.15, 0.10 | color 1.000, 0.015, 0.006, ends at 20 m |
| Shaft of Light, white (166) | White 1.20 | color 1.000, ends at 24 m |

So a `LightColor` light is a light of radius 10 in that color, converted
as tile lights are: the color linearised (to the power 2.2), a color
brighter than 1 scaled down to 1 and reaching that much further, and the
light ending at twice the radius. Static or not, and its height, change
nothing. This is what Moonglow already assumed for its own view, so its
view was right; the ring in the area view is drawn at where the light
ends.

## Way A: the stock light placeables

A **Lights** group in the area's tools (or on the Tools menu, as the
builder was told it could be): pick a color, click in the area, and
Moonglow places the matching stock placeable (rows 15107 to 15113) as a
plain instance: static, not useable, no blueprint needed.

- In the area view a light has no model, so it gets a marker (as a
  waypoint without a flag has) and a ring for its reach, and the view is
  lit by it as it is today by any placeable with `LightColor`.
- "Customizing" is choosing among the seven colors and moving it.
- Cost: small. A palette entry, a marker, a color chooser on the
  instance. No custom content, nothing to distribute, works on any
  server.
- Limit: seven colors, one reach. That is the whole of it.

## Way B: lights Moonglow generates

A light editor (color by picker, radius, multiplier, shadows on or off,
flicker, height above the ground) whose result Moonglow turns into
content:

1. a small model with one light node and nothing else, named from its
   settings (so two lights with the same settings share a model);
2. a placeables.2da row for it;
3. both written into a hak the module lists.

Placing, moving and editing then work as for any placeable; editing a
light's settings swaps its instance to another generated row.

What this needs, and where it is hard:

- **Writing models.** Moonglow reads models; the compiler and writers
  are in Moonglow Viewer and are due to move into the Toolset. A
  light-only model is the simplest case, and could be written as a text
  model (which the game also loads) before that move.
- **Owning a placeables.2da.** Only one placeables.2da wins in the load
  order. If the module's haks already have one (CEP, a server's own),
  Moonglow's must be a merged copy of that one plus its rows, placed
  above it, and re-merged when the other changes. Moonglow has no 2DA
  merger today (the deferred list has it open). This is the larger part
  of the work and the part most likely to cause trouble for builders.
- **Row numbers.** Generated rows need a range that will not collide with
  the game's, CEP's or the server's. A reserved range, stated in the hak
  and checked by `mg verify`, is the usual answer.
- **Distribution.** The generated hak is custom content: a persistent
  world must ship it (NWSync or by hand) like any other hak. For some
  builders that rules B out, which is one reason to do A as well.
- **Cleanup.** Models and rows no instance uses any more should go when
  the hak is rebuilt, or it grows without end.

Cost: medium to large, most of it the 2DA merge and its upkeep.

## Way C: scripted light effects

Place the invisible object (placeables.2da row 157) with a heartbeat or
spawn script that applies a `VFX_DUR_LIGHT_*` effect. It gives lights a
script can switch on and off in play, but only the game's fixed colors
and sizes, and it adds scripts to the module that the builder must keep.
Moonglow could offer it as a script-wizard recipe. Not recommended as
the answer to this request: it is no more adjustable than A and heavier.

## Could it be a plugin?

Partly. Way A could be a plugin today (place an instance of a stock
row). Way B needs writing models and merging 2DAs, which the plugin API
does not offer and which belong in Moonglow itself: the same pieces
serve other features (a 2DA merger is on the deferred list already).

## Recommended order

1. **Measure** what is marked above, in the client. A day's work with the
   existing harness; it decides the marker's reach ring in A and the
   defaults in B.
2. **Way A.** The Lights group, the marker and reach ring, the seven
   colors. Shippable on its own.
3. **The 2DA merger**, as its own feature (it is wanted anyway).
4. **Way B** on top of 3: the light editor, generated models, the hak.

## Decisions wanted

- Is A alone worth shipping first, or should lights wait for B?
- For B: is a hak that Moonglow writes and keeps acceptable, given that a
  persistent world must distribute it?
- Where the tool lives: a Lights group among the area's tools (beside
  Sounds and Waypoints), a Tools menu entry, or both.
- Whether C is wanted at all.

## Not proposed

- A new object kind in the area file. The game would not read it.
- Changing the base game's placeables.2da in place.
- Area-wide lighting (the area's sun, moon and fog colors): Area
  Properties has it.

[^request]: The request, relayed by the user.
[^placeables]: The installed game's placeables.2da and lightcolor.2da.
