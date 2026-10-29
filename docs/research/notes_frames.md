---
type: Research Note
title: 'Frames at 144 a second: where a frame''s time went'
description: Frame times against a 144 fps budget (6.94 ms) on the development machine - how each view is timed, what an area's picture, the galleries and the lists cost before and after October 2026's work, a dense area seen whole and an area's opening (27 October), what the profile found, and what is left.
tags: [performance, frames, renderer, area-view, galleries]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-29T19:52:35Z }
sources:
  - id: frame-perf
    resource: crates/mg-ui/tests/frame_perf.rs
    title: The frame-time test (release build, Tyrants of the Moonsea and the persistent world)
  - id: budget
    resource: human:august
    title: 'The budget: a steady 144 frames a second on the development machine (7 October 2026)'
  - id: metal-ci
    resource: https://github.com/jadzziaa/moonglow-toolset/actions/runs/37715321115
    title: The macOS job of a CI run that drew one scene with six variants of the lit shader (8 October 2026)
  - id: woods
    resource: crates/mg-ui/tests/frame_perf.rs
    title: 'game_shaders_frames_at_144 with the builder''s haks: TDoD''s woods, area092 and the water area, before and after (27 October 2026)'
---

# Frames at 144 a second: where a frame's time went

October 2026. The budget is a steady 144 frames a second on the
development machine (Ryzen 7 5800X3D, Radeon RX 9070, a 3440×1440 screen at
144 Hz)[^budget]: 6.94 ms for all a frame does. The budgets of
[the scale note](notes_scale.md) are of another kind (33 to 50 ms a frame,
with the accessibility tree built): they say that nothing is slow, not that
a frame is in time.

The measurements below are of 7 and 8 October 2026 and were not taken
again. The renderer has changed since: an MTR's `twosided` faces are drawn
from behind too (d516013), cut-out textures and punch-through particles are
cut at alpha 0.2 (327ef58, eb6eab3), and the editors copy and gather less
every frame (b394486).

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
| A store's inventory of 1,000 items (persistent world) | 6.1 | 0.5 (6.3 until its list laid out only the rows in sight, 27 October) |
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
- **On Metal, a fragment that is discarded has to stop there.** With the
  draws' values read from the storage buffer, macOS (the CI's machine;
  Linux and Windows were right) drew a see-through mesh's solid part with
  the depth of every fragment of it, the discarded ones too: a plant's
  soft edge hid the ground behind it, and a quad discarded altogether hid
  it as well. WGSL's `discard` lets the fragment run on (as Metal's
  `discard_fragment` does), and what ran after it there was the reading
  of the draw's values. Drawn right in variants of the shader tried on
  macOS: a `return` after each `discard`, or the draw's values copied
  before any; not with the solid part alone told by the vertex stage, and
  no differently a draw for each mesh. Every `discard` of the shaders is
  now followed by a `return` (a test holds them to it); nothing after
  takes a derivative. The same test had failed on macOS alone once
  before, for a texture read in a branch after the discards.
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
  divided was not measured. Since 2026-10-29 a flush writes only the
  documents edited since the last one: with 300 GITs of 200 creatures
  read (5 MB), it was 5.0 ms of writing them out and is now nothing
  (0.5 µs; 19 µs with one of them edited; release, the machine at a load
  of 11).
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
- **A tile's animations were bound to its model by name each time it
  was posed**, three times a frame (its pose, its lights, its meshes),
  every node's name lower-cased. They are bound once, as the model is
  loaded (`anim::Binding`, kept in the area's `Loaded`), and their keys
  gathered once for the three (`anim::Playing`). The scene's making
  (`render_area`'s `MG_TIME=400`, the median; each binary run twice in
  turn, the machine at a load of 26 to 31 from other builds): area092
  0.30 and 0.43 ms, now 0.13 and 0.12; area075 1.65 and 1.72, now 0.60
  and 0.41; area074 1.45 and 1.39, now 0.59; Elmwood (its objects
  posed too) 5.4 and 2.8, now 1.9 and 1.4. The pictures are the same,
  every pixel.

## A dense area seen whole, and opening an area (27 October 2026)

M3 of [the shaders proposal](../shaders-proposal.md) left two views over
the budget with either shader: The Domains of Dread's synthetic woods
(24 by 24 `mst04` tiles and 400 of its placeables, `frame_perf`'s
`tdod_area`) seen whole, 33,000 meshes in sight, and an area's opening,
which held the window while its models and textures were read.[^woods]

**Measured** back to back in one session (release build, the builder's
haks, other sessions building beside it; `game_shaders_frames_at_144`,
the median of the processor's time a frame with the wheel turning, the
GPU's in brackets). The harness now opens each area afresh from its
overview, its textures read anew: before, the switch-on pass began where
the switch-off pass had left the camera, close up.

| View (TDoD's woods) | Off, before | Off, after | On, before | On, after |
| --- | --- | --- | --- | --- |
| overview, wheel | 25.6 (7.3) | **13.1** (3.5) | 34.3 (6.1) | **15.8** (3.7) |
| turning, wheel | 36.0 (23.9) | **21.4** (7.7) | 50.6 (21.8) | **28.1** (9.5) |
| turning, pointer | 4.3 | 2.7 | 5.5 | 3.5 |
| close, pointer | 2.5 | 1.8 | 3.1 | 2.3 |

| Taken apart (`area_parts`, 3000×1250, 4× MSAA): encoded / GPU, ms | Off, before | Off, after | On, before | On, after |
| --- | --- | --- | --- | --- |
| woods, overview | 24.1 / 11.8 | **10.9 / 7.8** | 30.0 / 10.8 | **14.3 / 7.2** |
| woods, turned (pitch 0.9, a new view) | 34.3 / 28.4 | **17.0 / 9.6** | 47.0 / 24.7 | **23.1 / 15.3** |
| woods, close | 1.9 / 3.6 | 1.1 / 2.9 | 2.6 / 3.6 | 1.7 / 3.2 |
| area092, overview | 1.9 / 1.1 | 1.5 / 1.1 | 2.3 / 0.9 | 2.1 / 0.9 |
| water area, overview | 2.5 / 1.7 | 2.2 / 1.7 | 2.8 / 1.7 | 2.6 / 1.6 |

The woods' overview: 26,965 draws of 6,616 materials in turn before,
25,023 of 1,778 (and 349 pipelines) after; with the switch on, 26,263
draws of 1,898 materials and 796 pipelines. The stock and water views
were within the budget and stay so, a little quicker.

| Opening (cold: textures read anew) | Before | After |
| --- | --- | --- |
| TDoD's woods | one frame of 3.8 s (3.9 s on) | shown after 0.33 s, whole after 1.4 s; no frame over 22 ms (30 on) |
| area092 (784 tiles) | one frame of 189 ms (157 on) | shown after 95 ms, whole after 178 ms; frames of 4.5 ms at most (8.9 on) |
| the water area | one frame of 49 ms | shown after 37 ms, whole after 63 ms; one frame of 23 ms |

**What was done,** for both shaders where not said:

- **Meshes in shared buffers** (`mg-render/src/pool.rs`). Each mesh had
  a vertex and an index buffer of its own, which every draw bound: wgpu's
  work for binding them (the buffers looked up, their uses tracked and
  merged, their initialized ranges checked, the driver's calls) was a
  third of a frame of 30,000 draws. Meshes now lie in chunks shared by
  many (`GpuMesh::first_index`, `base_vertex`); a draw binds its chunk's
  buffers where the draw before was of another. Chunks grow to the size
  of those before them, within the device's largest buffer: the woods
  take ten. Encoding 25.5 to 17 ms off, 31.6 to 24.2 on.
- **See-through draws by state.** In an area seen whole most draws are
  see-through, in the order of their depths, and their materials and
  pipelines changed every few draws. The batcher (`batch.rs`) now puts a
  draw that can't join its mesh's batch at the end of the latest run of
  draws of its pipeline and material, where that takes it past no draw
  it meets on the screen, as it already did for a mesh's batch. wgpu's
  `finish` went from 7 to 4 ms. A program of the game's that moves its
  vertices (it reads the clock or the wind, or samples a texture: a
  sway) takes them out of its box, so its box is grown by a meter
  (`MOVED`) first: with the box as it was, a swaying leaf's soft edge was
  blended in another order in 226 pixels of the woods close up.
- **The game's draws** bind their material groups last (groups 2 and 3;
  a group bound anew has wgpu bind those after it anew), share one group
  of the draws' values across pipelines (it stays bound as they change),
  write a draw's transforms and colors a matrix or a vector at a time
  (`Own`), and look a pipeline's groups up once for draws of it in a row.
- **Less of the renderer's own work a draw**: draws sorted by a number
  (`(u64, u32)`) rather than by comparing them, a material's linear
  colors worked out once (`Surface::linear`), a see-through mesh's screen
  box once for its two parts, a vertex's buffers kept bound across
  pipelines and between Moonglow's draws and the game's.
- **An area's first reading on a thread of its own** (`area_view.rs`,
  `Reading`: its tileset and tile tools, its model, the walkmeshes and the
  scene's models, read and uploaded; a model that fails is left out and
  told, as on the window's thread), the view saying it is reading the
  area meanwhile; and **textures read apart** on up to four threads
  (`Renderer::read_textures_apart`, `textures_apart`): a model is drawn
  once its textures are read (`Drawn::textures_pending`), the view drawn
  again at the next animation step after textures arrive. Only where the
  window is kept going (`Moonglow::background_jobs`: the desktop app);
  tests and pictures drawn once read as before.

**Pictures compared.** `area_parts`' last frames (`FRAME_PARTS_SHOTS`, a
fixed clock): 27 pictures of the three areas, each config with either
shader, before and after. All the same to the bit but area092's with the
switch on, by 1 to 5 pixels of 3.75 million at most 4 of 255, as two
runs of the same build before differ (base against base: 5 pixels).
`an_area_read_apart_looks_as_one_read_in_the_frame` (mg-ui) holds an
area read apart to the picture of one read in the frame, and
`a_model_is_drawn_once_its_textures_are_read_apart` (mg-render) the
renderer's part.

**What is left** (in [the deferred list](../deferred.md)):

- **Over the budget still**, seen whole: the woods' overview 13 ms off and
  16 on while zooming, 21 and 28 turning. The switch on is still slower
  than off there: its draws bind two groups a material and more
  pipelines, and write their values through the programs' plans. What is
  left is per draw: some 25,000 draws whose meshes are mostly of one
  instance each (the area has each tile model four times), each a draw
  call; and the chunks the draws switch between.
- ~~**While an area fills in,** its picture is drawn again as textures
  arrive: those frames cost what the view costs drawn whole (13 to 22 ms
  for the woods). The frame an area is first shown in looks up its
  materials (MTRs) on the window's thread: 19 to 30 ms for the woods,
  23 ms for the water area, not taken apart.~~ Taken apart on 29
  October: the harness stepped its clock a quarter of a second a frame,
  so every frame of the opening was an animation step's, its picture
  drawn whole. At a 144th of a second a step, as the window's frames go
  (`frame_perf`'s harness since), the woods open with one frame in six
  drawn (the animations' steps, 25 a second, as an open view's are),
  each costing what the view costs drawn whole (15 to 21 ms off, 17 to
  26 on, more as more textures are in), the others under a millisecond
  and a half: 22 frames over the budget in 1.45 s. The frame it is first
  shown in is 11 ms, and its materials are not what it costs: the 298
  meshes' slots (their MTRs looked up and read) took 0.6 to 1.1 ms of it.
  An edit, read in the frame, costs a millisecond besides its picture
  (14.2 ms off, 17.8 on, against 13 and 18 drawn alone); Reload Resources
  reads the area again on a thread. What is left is the picture's cost
  seen whole, in "The largest areas in full view".
- A program that moves its vertices farther than a meter (TDoD's grass
  lifted by `fTilesetMeshOffsetZ`, 2 m) may be blended in another order
  where it reaches past its grown box.

## Not covered

What is left is in [the deferred list](../deferred.md), under "Frames".
In short:

- **The largest areas in full view.** 784 stock tiles take 2.5 ms
  zooming out; TDoD's woods, 33,000 meshes in sight, 13 ms (16 with the
  game's shaders) and 21 (28) turned to the horizon. Close up they are
  within it.
- **Opening an area** no longer holds the window, nor does Reload
  Resources; the frames that draw it as it fills in are the animations'
  steps and cost what the view costs drawn whole.
- **A slow picture** (a model with large textures) still takes a frame
  of 7 to 9 ms in a gallery.
- **The module tree** at persistent-world sizes is as it was. (The
  store's inventory lays out the rows in sight only since 27 October.)
- **Pictures that differ from run to run:** 10 of 192 area pictures,
  by a few pixels to a few hundred; the cause wasn't looked for.
- **Other machines, and the time egui takes to paint and present.**

[^frame-perf]: The frame-time test (release build, Tyrants of the Moonsea and the persistent world)
[^budget]: The budget: a steady 144 frames a second on the development machine (7 October 2026)
[^woods]: game_shaders_frames_at_144 with the builder's haks: TDoD's woods, area092 and the water area, before and after (27 October 2026)
