---
type: Research Note
title: 'Frames at 144 a second: where a frame''s time went'
description: Frame times against a 144 fps budget (6.94 ms) on the development machine - how each view is timed, what an area's picture, the galleries and the lists cost before and after October 2026's work, what the profile found, and what is left.
tags: [performance, frames, renderer, area-view, galleries]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-08T01:02:33Z }
sources:
  - id: frame-perf
    resource: crates/mg-ui/tests/frame_perf.rs
    title: The frame-time test (release build, Tyrants of the Moonsea and the persistent world)
  - id: budget
    resource: human:august
    title: 'The budget: a steady 144 frames a second on the development machine (7 October 2026)'
verified:
  - { by: claude-code/claude-opus-5-5, at: 2026-10-08T01:02:33Z }
---

# Frames at 144 a second: where a frame's time went

October 2026. The budget is a steady 144 frames a second on the
development machine (Ryzen 7 5800X3D, Radeon RX 9070, a 3440×1440 screen at
144 Hz)[^budget]: 6.94 ms for all a frame does. The budgets of
[the scale note](notes_scale.md) are of another kind (33 to 50 ms a frame,
with the accessibility tree built): they say that nothing is slow, not that
a frame is in time.

## How a frame is timed

`crates/mg-ui/tests/frame_perf.rs`[^frame-perf] runs each view as the
desktop app runs it: no accessibility tree, the clock a 144th of a second
on each frame, a 3440×1400 window. It times the interface's pass (which
draws a 3D view's picture too), the tessellation, and how long the GPU
then takes; with the pointer moving over the view, with nothing happening,
and with the wheel turning. It says how soon the view asks to be drawn
again, and with `FRAME_DETAIL=1` who asked.

```sh
cargo test --release -p mg-ui --test frame_perf -- --ignored --nocapture --test-threads=1
```

`FRAME_VIEW=<part of a name>` times only those views, `FRAME_COUNT=<n>`
sets the frames timed (for a profiler), `FRAME_SHOTS=1` saves each view as
a picture instead. Not timed: egui's own painting of the frame and its
presentation.

The machine has no `perf`. What a frame's time went on was found by
sampling the test's stacks with `eu-stack` (from a script that starts the
test with `prctl(PR_SET_PTRACER)` set, as Yama's `ptrace_scope` of 1 asks)
and counting the functions.

## Measurements

Milliseconds of the processor a frame (the median; "redrawn" is the frame
that draws the area's picture anew). The GPU took 2.6 ms at most
throughout, with 4x multisampling.

| View | Before | After |
| --- | --- | --- |
| A campaign as it opens (a 30-tile area, the palettes) | 6.0 | 0.65; 3.4 redrawn |
| A 49-tile area (1,191 meshes), all of it in sight | 4.4 | 0.6; 2.7 redrawn |
| A 784-tile area (3,671 meshes, 749 lights), all in sight | 15.5 | 0.65; 4.1 redrawn |
| …close up (the camera 25 m from the ground) | 15.7 | 1.9 redrawn |
| The palette's Gallery, scrolled (pictures being made) | 25 | 3.7; a slow picture up to 9 |
| The Gallery over the whole window, smallest pictures, left alone | 26, without end | 1.7 |
| Editors, palettes, the resource browser, menus (a campaign) | 0.3 to 2.0 | the same |
| A store's inventory of 1,000 items (persistent world) | 6.1 | 5.9 |
| The module tree, 25,000 resources listed | 3.4 | 3.4 |

Of the 784-tile area's picture: its draws took 13.4 ms to encode and take
2.7 (0.5 close up; 3.9 before meshes of one model were drawn as
instances); its scene (tiles and objects posed) took 1.2 ms and takes 0.5.

## What the measurements found

### The interface is cheap; an area's picture was most of every frame

With no area open a frame is under a millisecond. But an area is open
behind nearly everything, and its view drew its scene on every frame of
the window: a move of the pointer over a menu, a key typed in a window in
front of it. The GPU had little to do; the processor's time went before
the GPU was given anything.

- **The picture is kept** (`area_view.rs`, `Drawn`): drawn again when the
  camera, the view's settings or the area change, every frame while
  something follows the pointer (a blueprint to place, a drag), and every
  40 ms (`ANIMATION_STEP`) for the animations. A view's usual frame is now
  the interface's alone.

### Drawing a large area: 13.4 ms, none of it the GPU's

Sampled on the 784-tile area:

| Where the time went | Share | What was done |
| --- | --- | --- |
| Each mesh tried all 749 lights for the 32 it takes | 36% | The lights are sorted into a grid's squares on the ground they reach; a mesh tries those of its squares (`reach.rs`) |
| wgpu binding each draw's ten-texture material and its uniforms anew | 26% | The draws' values lie in one storage buffer, a draw's found by its instance number; a material is bound when it is another than the last draw's |
| Textures looked up by name for every mesh (renames, PLT colors, environment map, MTR) | 16% | What they come to is kept by model and by how the instance dresses it (`Surface`) |
| Draws copied and sorted by value, 700 bytes each | 8% | A draw is a few words; its uniforms lie apart |

More:

- **A bind group bound anew has those after it bound anew.** wgpu
  processes a group again whenever a group before it in the layout
  changes. With the draw's own group before the material's, every draw
  cost the material's ten textures again, whether or not the material was
  set. What changes most often goes last, or (as now) is no group at all.
- **The draws' order is as it was, where it matters.** Sorting the opaque
  draws by material would bind fewer materials (805 of 4,807 draws change
  it as it is), but coplanar meshes (tile floors) have equal depths, and
  which shows depends on the order. See the instances, below.
- **What the camera can't see isn't drawn** (a mesh's box against the
  camera's six planes; not skinned or animated meshes, which leave their
  box). Close up, 3,461 of the area's 3,700 meshes are left out.
- **Tiles of one model are posed once.** A scene posed each tile for
  itself, three lookups of its animations' nodes by lower-cased name
  each; an area's tiles are of few models.

Checked on 192 pictures of the campaigns' areas, before and after: 177
the same to the bit, 5 with one or two pixels different by 1 of 255, and
10 that differ from run to run of the same program (before as after: see
Not covered).

### Meshes of one model as instances of one draw

What was left of a large area's picture was mostly wgpu's cost of each
draw, the same however little it draws: 4,807 draws of some 1,000
different meshes, an area's tiles being of few models. Draws of one mesh with one
material are now one draw of as many instances (`batch.rs`); the draws'
values were by instance number already.

The order of draws matters where two meet (of coplanar meshes the later
shows; see-through ones blend in order), so a draw joins those of its
mesh only if that takes it past none it meets:

- **Solid meshes meet where their boxes do in the scene:** overlapping,
  or a flat one on a face of the other (a decal on a floor or a wall).
  Boxes that only touch do not meet (tiles lie edge to edge), and a
  model's own meshes keep their order whatever their boxes.
- **See-through meshes meet where their boxes do on the screen.**
- Skinned and animated meshes, which leave their boxes, are taken past
  nothing.

| | Meshes | Draws | Encoded | The GPU |
| --- | --- | --- | --- | --- |
| The 784-tile area, all in sight | 4,807 | 1,028 | 3.9 ms, now 2.7 | 1.08 ms, now 0.94 |
| …close up | 280 | 187 | 0.5 ms, as before | as before |
| A 49-tile area, all in sight | 1,184 | 638 | 1.7 ms, now 1.5 | 1.0 ms, as before |
| The 192 pictures of the campaigns' areas | 255,452 | 170,858 | | |

An area of one tileset's few tiles gains most; interiors, each tile
another model, little.

**Not quite the same picture.** Of the 192 pictures, 12 differ from those
drawn a mesh at a time, by one to three pixels each of 480,000: single
samples on the seam between two tiles, where the floors of both cover
the sample and now the other is drawn last. (Only the solid pass: with
the see-through passes alone drawn as instances, every picture is the
same.) Keeping the order of floors that share an edge was tried: it gave
up a fifth of the draws saved and mended 4 of the 12, the rest being
seams of meshes that are not flat. `Renderer::instancing` switches it
off, to compare.

(Timed first with the game running beside the test, the GPU's times
went from half to twice a draw for each mesh's, run by run: the game's
own frames. Its times are to be taken with nothing else drawing.)

### The galleries: 25 ms a frame, and a wide one for ever

- **Three pictures were made a frame, 8 ms each** in the test; a picture
  now takes 2 to 3. With each one, every document the workspace had read
  was written back into the module (`Workspace::flush`, for the editors'
  working copies: the more a session has opened, the longer; the test
  had read every area's ARE) and three render targets were made. The
  flush is now once a revision, the targets shared, and pictures are made
  for 2 ms a frame (`THUMBNAIL_TIME`; one at least). How the 8 ms
  divided was not measured.
- **All pictures were let go at once** when there were more than 192, or
  when the module changed at all. A gallery over a wide window shows more
  than 192: it made them again three a frame without end. Now those in
  sight are kept however many, the ones longest out of sight go past 512,
  and after an edit a blueprint's picture is kept if the blueprint still
  looks as it did.
- **Each picture kept its multisampled and depth targets**, nine times
  the picture's memory (1.8 MB each). A picture is now its own texture
  alone (0.2 MB).

### Smaller things

- The area's sounds copied the area's whole GIT every frame
  (`area_audio.rs`): now only its sound properties and placed sounds.
- The debug log's per-frame lines cost nothing measurable.

## Not covered

What is left is in [the deferred list](../deferred.md), under "Frames".
In short:

- **The largest areas in full view.** 4.1 ms for 784 tiles: a 32 by 32
  area of a tileset with more meshes a tile is near the budget when all of
  it is in sight. Close up it is not.
- **Opening an area** holds the window for 0.2 s (its models and
  textures are read in the frame).
- **A slow picture** (a model with large textures) still takes a frame
  of 7 to 9 ms in a gallery.
- **The store's inventory and the module tree** at persistent-world
  sizes are as they were.
- **Pictures that differ from run to run:** 10 of 192 area pictures,
  by a few pixels to a few hundred; the cause wasn't looked for.
- **Other machines, and the time egui takes to paint and present.**

[^frame-perf]: The frame-time test (release build, Tyrants of the Moonsea and the persistent world)
[^budget]: The budget: a steady 144 frames a second on the development machine (7 October 2026)
