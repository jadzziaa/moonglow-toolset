---
type: Proposal
title: 'Lights placed on the fly: a proposal'
description: Placing a light in an area without making a custom placeable by hand first - what the game offers for lights, what was built and measured (the stock light placeables from the area's right-click menu, their reach in the client), and the open proposal - lights of any color and reach by generated content, scripted light effects, and two questions not yet measured.
tags: [proposal, lights, placeables, areas, custom-content]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T01:34:10Z }
sources:
  - id: request
    resource: human:august, relaying the builder dafena (Discord, 2026-10-07)
    title: The request - paint a light with default values, then customize it
  - id: placeables
    resource: placeables.2da and lightcolor.2da in the installed game (read with mg cat, 2026-10-07)
    title: The game's own tables
---

# Lights placed on the fly: a proposal

A builder asked for lights that can be painted into an area with default
values and then adjusted (color, reach, brightness), without first making
a custom placeable that carries the light.[^request]

Three ways were proposed. Way A (the game's stock light placeables) was
chosen first on 2026-10-07 and is built and measured: see
[What was built](#what-was-built-way-a). Way B (lights of any color and
reach, by generated content) is built too, on 2026-10-29, behind an option
that is off by default: see [Way B](#way-b-lights-moonglow-generates). Way
C (scripted effects) is not recommended. Two questions about the game's
lights are [not measured yet](#not-measured-yet).

| Way | What the builder gets | What the module needs | State |
| --- | --- | --- | --- |
| A. Stock light placeables | Seven colors, fixed reach | Nothing: base game | built |
| B. Generated content | Any color, reach, brightness, shadows (no flicker yet) | A hak Moonglow writes (models and a placeables.2da), the builder's choice | built, behind an option off by default |
| C. Scripted effects | The game's fixed light effects, switchable in play | A script on an invisible object | not recommended |

## What the game offers

The game has no "light" object. A light in an area is always something
else that happens to shine: a tile, a placeable, a visual effect. So a
light tool is a tool that makes or picks one of those for the builder.

- **A placeable's row can carry a light.** placeables.2da has
  `LightColor` (a lightcolor.2da row) and `LightOffsetX/Y/Z`.[^placeables]
  The game ships seven invisible ones made for this: rows 15107 to 15113,
  "Light, Blue" to "Light, Yellow", model `dag_invisible`, each a
  different `LightColor`. The older Shaft of Light placeables (rows 166
  to 173) do the same with a visible beam.
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

## What was built (way A)

On an area's right-click menu: **Add Light Here** with a color puts one
of the seven stock light placeables (rows 15107 to 15113) there as a
plain instance, and **Light Color** on a light gives it another of the
colors (`crates/mg-ui/src/area_view.rs`; the manual's
[Areas](manual/04-areas.md) chapter). A light has no model, so the area
view draws a marker for it and, when it is selected, a ring where its
light ends. The view is lit by it, as by any placeable with a
`LightColor`. Nothing is added to the module: the placeables are the
base game's. The limit is the game's: seven colors, one reach.

The palette has it too (2026-10-29): **☀ Lights**, beside the blueprint
types, lists the same lights; one chosen is put where the area's view
is clicked (Shift + click places more). Way A is built whole.

### Measured

The reach was measured in the game client on 2026-10-07
(`placeable_light_uniforms` in
`crates/mg-corpus-tests/tests/client_render.rs`, which reads the lights
the client hands its shader in a dark area):

| Placeable | lightcolor.2da | The client's light |
| --- | --- | --- |
| Light, White (15112), static, 1.5 m up | DimWhite 0.60 | one light, color 0.325, ends at 20 m |
| the same, not static | | the same |
| the same, on the ground | | the same |
| Light, Red (15111) | 1.00, 0.15, 0.10 | color 1.000, 0.015, 0.006, ends at 20 m |
| Shaft of Light, white (166) | White 1.20 | color 1.000, ends at 24 m |

So a `LightColor` light is a light of radius 10 in that color, converted
as tile lights are: the color linearized (to the power 2.2), a color
brighter than 1 scaled down to 1 and reaching that much further, and the
light ending at twice the radius. Static or not, and its height, change
nothing. This is what Moonglow already assumed for its own view, so its
view was right; the ring in the area view is drawn at where the light
ends.

### Not measured yet

In the game client, with the oracle the renderer uses:

- whether a placeable's `LightColor` light and a model's light node light
  creatures and placeables or only tiles;
- how many dynamic lights the game draws at one place before dropping
  some.

Both bear on the defaults of way B.

## Way B: lights Moonglow generates

Built on 2026-10-29 as the owner decided it ("As an option, default off
to prevent unwittingly touching PW haks from maintainers"): Options ›
Areas, and the project file's `[lights]` (`generated`, `hak`,
`first_row`), with Custom Light… on the area's menu and the palette
(`mg_module::lights`, `crates/mg-ui/src/light_maker.rs`; the manual's
[Areas](manual/04-areas.md#lights-of-any-color-and-reach)). The hak is
the builder's choice in the window, which says what it writes and that
the players must have the hak before it writes. The placeables.2da is
the copy the game reads with the row added, so no merge is needed; a
hak below another's copy is refused. Not built: the flicker, and taking
rows and models out that no light uses (the [deferred
list](deferred.md) has both, with the client's two questions). What
follows is the proposal as it was.

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

- **Writing models.** The model compiler and the binary and ASCII
  writers moved into the Toolset from Moonglow Viewer on 2026-10-09
  (`crates/mg-mdl`: `compile.rs`, `binary_write.rs`, `ascii_write.rs`),
  so this part is at hand. A light-only model is the simplest case.
- **Owning a placeables.2da.** Since 2026-10-29 there is a way that
  needs no merge, made for lighting schemes of one's own (Area
  Properties › Saved Lighting › Make a Lighting Scheme): the table is
  opened in the 2DA editor where the builder chooses (the module's, a
  copy in it, the table of a hak of the module, a hak open in the hak
  editor) and the row added there (`Editing::add_row_of`), to be saved
  with the hak. A generated light's row could be added so, to the copy
  of placeables.2da the game reads. What follows is the case of a
  generated hak of Moonglow's own, beside the others. Only one
  placeables.2da wins in the load order. If the module's haks already have one (CEP, a server's own),
  Moonglow's must be a merged copy of that one plus its rows, placed
  above it, and re-merged when the other changes. `mg 2da-diff --base`
  merges two tables against the one both were made from, but nothing in
  Moonglow merges several haks' copies of a table or keeps such a merge
  up to date (the [deferred list](deferred.md) has it open). This is the
  larger part of the work and the part most likely to cause trouble for
  builders.
- **Row numbers.** Generated rows need a range that will not collide with
  the game's, CEP's or the server's. A reserved range, stated in the hak
  and checked by `mg verify`, is the usual answer.
- **Distribution.** The generated hak is custom content: a persistent
  world must ship it (NWSync or by hand) like any other hak. For some
  builders that rules B out, which is one reason A was done as well.
- **Cleanup.** Models and rows no instance uses any more should go when
  the hak is rebuilt, or it grows without end.

Cost: medium to large, most of it the 2DA merge and its upkeep.

## Way C: scripted light effects (not recommended)

Place the invisible object (placeables.2da row 157) with a heartbeat or
spawn script that applies a `VFX_DUR_LIGHT_*` effect. It gives lights a
script can switch on and off in play, but only the game's fixed colors
and sizes, and it adds scripts to the module that the builder must keep.
Moonglow could offer it as a script-wizard recipe. Not recommended as
the answer to this request: it is no more adjustable than A and heavier.

## Could way B be a plugin?

Not yet. Plugin API 0.3 writes 2DAs (`mg.twoda`) and haks (`ctx.hak`),
so a plugin could add a placeables.2da row and put it into a hak. It
cannot write a model, and it has no merge of several haks' tables
either. Both belong in Moonglow itself: the same pieces serve other
features.

## Order of what is left

1. Measure the two questions above (the next client batch).

## Decisions wanted

- Whether C is wanted at all.

Decided already: A was worth shipping first (2026-10-07), and the tool
lives on the area's right-click menu; B as an option, off by default
(2026-10-29).

## Not proposed

- A new object kind in the area file. The game would not read it.
- Changing the base game's placeables.2da in place.
- Area-wide lighting (the area's sun, moon and fog colors): Area
  Properties has it.

[^request]: The request, relayed by the user.
[^placeables]: The installed game's placeables.2da and lightcolor.2da.
