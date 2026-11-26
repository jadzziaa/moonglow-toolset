---
type: Research Note
title: 'Frames at 144 a second: where a frame''s time went'
description: Frame times against a 144 fps budget (6.94 ms) on the development machine - how each view is timed, what an area's picture, the galleries and the lists cost before and after October 2026's work, a dense area seen whole and an area's opening (27 October), the largest stock area seen whole taken apart (30 October) and again (2 November), the script editor on nwscript.nss (2 November), High shadows' first frame with the stencil volumes made apart, textures compressed as read, the opening's wait and anti-aliasing, the Shaders switch's cost in the woods and the water and the review's small slowdowns (25 November), the woods test's two GPU levels (26 November), what the profile found, and what is left.
tags: [performance, frames, renderer, area-view, galleries]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-26T11:40:00Z }
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
  - id: area084
    resource: crates/mg-corpus-tests/examples/render_area.rs
    title: render_area MG_TIME_RENDER on Tyrants of the Moonsea's area084, its stacks sampled with eu-stack (30 October 2026)
  - id: texmem
    resource: crates/mg-ui/tests/frame_perf.rs
    title: 'frames_at_144 (FRAME_VIEW=area, FRAME_DETAIL, FRAME_MEMORY, FRAME_UNCOMPRESSED, FRAME_SAMPLES) on area092 and TDoD''s woods, v2.13.3 and v3.0.0 built beside develop, with and without compression and the fix (25 November 2026)'
  - id: volumes-apart
    resource: crates/mg-ui/tests/frame_perf.rs
    title: sound_ranges_frames_at_144 with FRAME_SHADOW_QUALITY=high, Fast and High, before and after the stencil volumes were made apart, and FRAME_MEMORY=labels after the area closed (25 November 2026)
  - id: area084-again
    resource: crates/mg-ui/tests/frame_perf.rs
    title: sound_ranges_frames_at_144 and game_shaders_frames_at_144 (with the builder's haks), and render_area MG_TIME_RENDER on area084, on the RX 9070 XT and the Radeon 610M, before and after, its stacks sampled with eu-stack (2 November 2026)
  - id: framecost
    resource: crates/mg-ui/tests/frame_perf.rs
    title: 'game_shaders_frames_at_144 (FRAME_AREA=tdod and water), frames_at_144 and grass_frames_at_144, three runs alternated with 27b2e44b, v3.18.1 and each side of 6b26e702 built beside develop; render_area MG_TIME_RENDER with both paths on alternate frames; eu-stack samples (25 November 2026)'
  - id: levels
    resource: crates/mg-ui/tests/frame_perf.rs
    title: 'game_shaders_frames_at_144 (FRAME_AREA=tdod, "overview, 4x MSAA, Shaders on") built at 42027e27 and with its order''s variants on alternate frames of one run; six runs with FRAME_MEMORY=1 beside a read-only sampler of the RX 9070 XT''s pp_dpm_sclk, gpu_busy_percent, mem_info_vram_used and mem_info_gtt_used (26 November 2026)'
  - id: editor-perf
    resource: crates/mg-ui/tests/editor_perf.rs
    title: editing_a_huge_script on the game's nwscript.nss, before and after the lines laid out one at a time, its stacks sampled with eu-stack (2 November 2026)
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

### A slow picture, read apart (29 October 2026)

What was left of a gallery's frames was the picture that took one: a
model with large textures, read, uploaded and drawn on the window's
thread in one frame. Timed in `frames_at_144` (`FRAME_VIEW=Gallery
FRAME_DETAIL=1`, release, the machine at a load of 6 to 16 from other
builds), the frames the wheel turns the palette's Gallery in, three
runs each way in turn: made in the frame, the slowest 16 to 20 ms (the
smallest pictures', 11 to 13) and one to three frames over the budget a
run; read apart, the slowest 1.8 to 3.2 ms and none over it.

Where the frame went, taken apart: the models' reading (`Model::read`
of the model and its supermodels) and their upload; the textures' reading,
decoding and mipmaps; and, the most of the slowest pictures (13 to 27 ms
of `tnp_balc_a02`'s), the game's programs a picture was the first to
draw with, made while the frame waited. Now, where the window keeps going
(`background_jobs`, the desktop app), a picture's models are read and
uploaded by two threads of their own (`PictureReader` in `model_view.rs`,
from the module's models as they were then and the game's), its textures
are read apart as the area view's are, and the game's programs made apart
as the area view makes them (`game_background`); the picture is drawn in
the frame after they are, and kept only once nothing it draws waited. The
jobs waiting are let go of with the game data
(`Moonglow::release_game_data`). Tests that make pictures with it off make
them in the frame, as before.

The galleries' pictures are the same, pixel for pixel, read either way
(`frames_at_144` with `FRAME_SHOTS=1`, the palette's Gallery at both
sizes; and `a_gallery_s_pictures_are_read_apart_and_look_the_same`).

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
  where it reaches past its grown box. (Since 2026-11-02 its box is grown
  by its MTR's offset besides: `GamePipeline::moved_by`.)

## The largest stock area seen whole, taken apart (30 October 2026)

Tyrants of the Moonsea's area084 (676 tiles, 549 objects) with the
wheel zooming over all of it is over the budget: 9.8 ms a frame, 10.8
turning (`sound_ranges_frames_at_144`, the machine at a load of 1 to
2), all of it the window's thread, the GPU's 1.2 to 1.7 ms. Drawn alone
at the frame test's size with the game's shaders (`render_area`'s new
`MG_TIME_RENDER`, the median of 500 frames, the camera turning a little
each): the scene made in 0.5 ms, its draws encoded in 8.0 ms, the GPU's
2.6 ms; 16,651 meshes in 9,680 draws of 1,606 materials and 278
pipelines in turn, 942 out of sight, 4,779 casting the sun's
shadows.[^area084]

| Where the encoding went (221 stacks) | Share |
| --- | --- |
| wgpu's `finish`: each draw's groups and buffers bound, their uses and initialized ranges tracked, the driver's calls | 33% |
| The game's draws' values worked out and written (`GameShaders::push`, `upload`, `fill_member`, `write_buffer`) | 20% |
| The lights each mesh takes (`ReachCache::reaching`), most of it moving kept choices between two maps | 9% |
| The batcher (`Batcher::place`) | 9% |
| The sun's shadows: their plan and map (1.3 ms with them off) | 6% |
| Sorting, the light sets' map growing, the screen boxes | 8% |

The lights a mesh took are now kept in one map with the frame that last
took them, and the light sets' map keeps its room from frame to frame:
encoded in 7.5 ms (four runs in turn with the build before, 8.0), the
picture the same to the bit.

What would not do it:

- **Leaving out what is too small to see.** Of the 12,149 meshes in
  sight, 47 are under a pixel across and 632 under two: the area's
  meshes are tile-sized.
- **Merging far tiles' meshes.** Each draw lights its mesh with the 32
  lights that matter most to it; a merged mesh would take 32 for the
  whole, and the picture would change where more reach it.
- **Keeping the draws' values while only the camera moves.** The game's
  programs take `m_mv` and `m_mvp` a draw and the lights in view space:
  every value block is written anew as the camera moves, and a wheel's
  zoom moves it every frame. The sun's map is fitted to the view, so it
  is drawn anew as well.

What is left is wgpu's work for each of some 10,000 draws: fewer draws
(one material's meshes of many tiles drawn as one, each mesh's lights
found by its vertices), or render bundles recorded once for what doesn't
move, which wgpu replays without tracking each draw again.

## The largest stock area seen whole, again (2 November 2026)

The same view's frames, its draws taken apart again with
`render_area`'s `MG_TIME_RENDER` (the medians of 300 frames at the frame
test's size, the camera turning) and its stacks sampled with `eu-stack`,
its source lines read from a build with line tables
(`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, `eu-stack -i
-s`).[^area084-again] The machine at a load of 2 (other sessions build
here; under load the times were a tenth to a fifth longer).

| area084 | Before | After |
| --- | --- | --- |
| Draws encoded, the game's shaders (RX 9070 XT / Radeon 610M) | 7.43 / 8.22 ms | **6.06 / 6.34** |
| Draws encoded, Moonglow's own | 5.36 / 5.47 | **4.95 / 5.01** |
| The GPU's, the game's shaders (9070 XT / 610M) | 2.99 / 52.4 | 2.98 / 52.5 |
| Frames, Sound Ranges off, overview / turning (9070 XT) | 9.11 / 10.31 | **7.52 / 8.89** |
| Frames, Sound Ranges on, overview / turning (9070 XT) | 9.61 / 10.81 | **7.86 / 9.56** |
| Frames, off, overview / turning (610M; its GPU 25 / 34 ms, as before) | 8.98 / 11.59 | **7.48 / 8.37** |
| Frames, on, overview / turning (610M; its GPU 27 / 38 ms, as before) | 10.00 / 11.84 | **7.61 / 9.65** |

The stock area092 and the water area are a little quicker with the
switch on (2.39 to 2.19 ms zooming, 3.19 to 2.80); TDoD's woods are as
they were (14 to 31 ms: they have two lights and no sun's shadows, which
the work below was on). The 27 pictures of `area_parts`
(`FRAME_PARTS_SHOTS`: the three areas, either shader) and area084's
`render_area` pictures with either shader are the same to the bit.

What was done (`renderer.rs`, `reach.rs`):

- **A light set numbered as it is chosen** (0.35 ms). The game's draws
  share the values of the lights a set comes to; each draw's set was
  found in a map made anew every frame, its lights copied into a key
  and let go of after (an eighth of the frame: the map emptied, the
  keys made and hashed). `ReachCache` now numbers a set once, as a
  mesh's lights are chosen, and a frame numbers the sets it asks for
  from 0 (`in_frame`), the order the game's values take them in.
- **A mesh's lights by its place** (0.3 ms). A kept choice was looked up
  by the mesh's ball in a map of 16,000, its lights copied from a box of
  their own: a lookup that missed the processor's caches each time. A
  mesh now asks first where the mesh in its place among the scene's
  took one last frame, its lights one after another in a list of all
  the choices'; then by its ball. The choices no mesh took lately are
  let go once they are more than twice a frame's and 4,096.
- **The sun's shadows planned beside the draws** (0.25 ms): every mesh of
  the scene tried as a caster and the casters sorted, on a thread of its
  own while the meshes' draws are made, in scenes of 64 instances or
  more. What the shadows may fall on is drawn first in the meantime and
  put back among the rest where there are none after all.
- **The game's solid draws sorted beside the batcher** (0.25 ms), on a
  thread of their own where there are 2,048 or more.
- **A see-through draw's batch key kept with it**, not looked up in the
  game's draws for each (an address followed to its pipeline's number).

Tried and given up, the pictures the same each time:

- **One indirect draw for draws in a row of one state**
  (`multi_draw_indexed_indirect`, each draw its mesh's indices and its
  own instances; `INDIRECT_FIRST_INSTANCE`): area084's 10,700 calls
  became 3,200, its encoding 0.1 to 0.3 ms quicker and the 610M's GPU no
  slower, but the woods a millisecond slower turned with the switch on.
  wgpu checks an indirect draw's arguments on the GPU (its
  `VALIDATION_INDIRECT_CALL`, on in release; Direct3D 12 needs it for an
  instance's number), and the bookkeeping of each sub-draw costs what a
  draw by itself saved: most of wgpu's work a draw is binding, not the
  draw.
- **Each pass's batcher on a thread** (the see-through part and its
  fringe apart): one pass is most of the draws, and waking a thread on
  an idle core took what the others saved (0.1 ms on area084, none on
  the woods).
- **The meshes' chunk in the batcher's state** (see-through draws of a
  chunk kept together): 3,400 calls became 2,300 and nothing quicker,
  the GPU's time 2.9 to 3.3 ms.

What is left, in [the deferred list](../deferred.md): of area084's
encoding, wgpu's `finish` is a quarter, nine tenths of it the
see-through draws, where binding is most of it (each group's textures
tracked and their initialized state checked: 1,270 changes of material
and 2,600 of the meshes' chunk among 9,500 draws, 11 chunks since a
chunk holds half a million vertices at most); the game's draws' values
a fifth (23,000 blocks of 120 bytes and 8,700 of their lights' 144, each
written, then copied for the GPU); the batcher a tenth. Render bundles
recorded on threads would take at most half of `finish` off the
window's thread (a bundle's initialized-state checks and the driver's
calls stay on it); the values could be written on threads straight into
the GPU's staging memory.

## The script editor on nwscript.nss (2 November 2026)

`editor_perf`'s `editing_a_huge_script`[^editor-perf] opens the game's
`nwscript.nss` (13,869 lines, 650 KB) as a module's script at 3440×1400,
the game loaded, and times plain egui frames as the desktop app runs them
(no accessibility tree, the clock a 144th of a second a frame), each held
to the budget as `frame_perf`'s are. Medians, ms, before and after the
lines were laid out one at a time (`script_layout`), on the development
machine with another session's builds running beside it:

| Frame | Before | After |
|---|---|---|
| Idle | 1.5 | 0.9 |
| A key, every frame | 11.5–12.1 | 1.8 |
| A key at a typist's pace (every eighth frame) | 98 | 2.3 |
| The frame after it (the side lists' symbols read again) | 21 | 6.3 |
| Backspace | 11.4 | 1.6 |
| A new line | 15 | 2.6 |
| Text composed (IME) | 31 | 2.0 |
| A bookmark set | 3.6 | 1.6 |
| The cursor down a line, a page | 1.5, 1.7–2.0 | 0.9, 1.0 |
| The selection a line, a page longer | 1.8–2.0, 2.0–2.2 | 1.1, 1.3 |
| Scrolling | 1.9 | 1.1 |
| 500 lines pasted (one frame) | 100 | 6.2 |
| Select All typed over, taken back (one frame) | 124 | 110 |

Where the time went, its stacks sampled: a key laid the whole text out
again (highlighted whole, the job hashed, split at its lines and put
together again by egui, the error's underline cut through every section);
at a typist's pace every key laid out all 14,000 lines anew, egui keeping
a text's laid-out lines only while they are asked for at every frame,
which the frames between keys did not ask for. After: a key lexes and lays
out the line it changed and puts the lines' galleys together (half a
millisecond for 14,000); egui's own text edit (its character offsets) and
the rest of the window are most of what is left. The frame after a key is
the symbols of a script of 7,466 declarations read again: its outline
(2.2 ms), its symbols (1.3) and the game's put in among them (1.8).
Taking back a change of the whole text lays it out again whole, as
opening it does.

## High's first frame in an area: the volumes made apart (25 November 2026)

Shadows quality High drew an area's first frame with every casting
mesh's stencil volume made in it ([the deferred list](../deferred.md)
had area084's longest opening frame at 48 to 51 ms with High on 30
October[^volumes-apart]). Read again on 25 November with
`sound_ranges_frames_at_144` (area084, Tyrants of the Moonsea: 4,764
casters of 846 meshes; `FRAME_VIEW=none-at-all FRAME_DETAIL=1`, release,
`--test-threads=1`, the first opening of each run, two runs each):

| Longest frame of area084's opening | Fast | High |
|---|---|---|
| Before (`e0cd765d`) | 22.1, 19.2 ms | 34.6, 34.9 ms |
| After | 19.0, 21.2 ms | 21.9, 21.2 ms |

A timer around the volumes' part of the frame, put in and taken out:
before, the first High frame spent 19.7 ms making the volumes and 1.6
ms on the stencil target and the volumes' pipelines (made once a
session, left in the frame). The volumes are now made on two threads of
their own (`mg_render::volume::VolumeMeshes`, the area view's
`Renderer::volumes_apart`); a caster casts nothing until its volume is
made, and the frame never waits. In both runs the casters waited for
6 of the window's frames, 29 ms from the first to the last, before
every shadow showed. What is left of the opening's longest frame (13
to 15 ms where the area is first drawn, 19 to 22 ms twice after) is
there with Fast too: it is not the shadows'.

The frames after the opening: the volumes are the renderer's now, not
each mesh's, and are looked up for each caster of each frame. Looked up
in a map by their key, the volumes' part of a steady High frame took
0.14 to 0.22 ms against 0.08 to 0.09 before. The map is now a slab whose
place each mesh remembers (checked against the key), and the casters
point at their places, with no reference count for each. That made it
0.08 to 0.09 ms again (medians of about 75 frames, timed in turn with
the build before).

The pictures are unchanged. `FRAME_SHOTS=1 FRAME_SHADOW_QUALITY=high`
shots of area084's 14 views (Sound Ranges off) from the build with the
volumes made in the frame (and the opening timed as before) matched
those from the build before, bit for bit, all but the Log's clock. With
the volumes made apart, where the opening goes on until no caster waits
(6 frames longer), 13 views match and the close view's grass
tufts move by up to 16 of 255 (172 pixels). Fast's close view is the
same with either build. The cause of the grass's change was not taken
further than that: it is shaped as the tufts, not as a shadow.

The renderer lets go of a model's volumes with the model, or when none
of them has been cast for 30 s of its frames (High turned to Fast). With
`FRAME_MEMORY=labels FRAME_SHADOW_QUALITY=high`, area084 open held 11.4
MiB of volumes (1,692 buffers) and 69.8 MiB of stencil target. Three
seconds after its tab closed (the view lets go of its scene) both were
gone, with no volume kept (`shadow volumes kept: 0`), and no other view
drew. So were area084's targets and framebuffer copies (the review's
R10): the 238.5 MiB left are the module's own first area's view
(aquarium001, its tab still open), the one view the renderer still
keeps something for.

## Textures compressed as read, the opening's wait and anti-aliasing (25 November 2026)

Three of the performance audit's leftovers, read with `frame_perf`
(release, the RX 9070 XT unless named; another session's UI tests
loaded the machine to 9 to 18 throughout, so single runs scatter and the
lowest of each set is the one to compare)[^texmem].

**TGA textures compressed as they are read; PLTs kept.** The encoder
(`mg_image::encode`) took 72 ms a megapixel for BC1 and 80 to 192 for
BC3 on one thread. Its quick effort (one refit, no raw-ends candidate;
the alpha's six-step mode only for blocks that hold 0 or 255) halves
that, and block rows on rayon's threads bring it to 4.0 to 4.7 ms a
megapixel for BC1 and 5.6 to 5.8 for BC3. The quick effort's error is
within 1.5 times the best's (`the_quick_effort_stays_near_the_best`).
Area092 (Tyrants of the Moonsea, `FRAME_MEMORY=1`) reads about 142
textures, of which 9 TGAs become BC1 and 4 BC3 (a 900² sky took 19 ms
on one thread); its 24 colored PLT copies stay RGBA8 (below): its
textures go from 31.2 to 21.2 MiB (17.9 with the PLTs compressed too).
TDoD's woods are unchanged: of their 968.9 MiB of textures all but 16
small RGBA8 ones are DDS, and none of those 16 is large enough. What is compressed: a texture read
as RGBA8 into a material's first slot (a TGA picture, a cutout or
see-through texture by its alpha: BC1 when every texel is opaque, else
BC3), with both sides multiples of 4 and the smaller at
least 64 (`texture::COMPRESSED_FROM`). Not: the normal, specular and
roughness maps of slots 1 and up (both areas read none as RGBA8, and the
encoder makes neither BC4 nor BC5), PLTs' colored copies, particles'
pictures and those a TXI adds (`blending additive`; below for each),
procedural textures (TXI
`proceduretype`), and the UI's pictures (egui's, not the renderer's).
A device without `TEXTURE_COMPRESSION_BC` keeps RGBA8. The reading is
on the renderer's reader threads already, so nothing new went
off-thread; what it adds to the opening is below.

Shots of area092 and area026 (`FRAME_SHOTS=1`) with and without
differed less than two runs of one build do, but for area026's floor
close up: green and magenta blotches on the floor around a placeable's
glow, the same in two runs of each build (the runs of one build match
bit for bit there; the compressed build differs from the uncompressed
by up to 38 of 255, 3,488 pixels of a 300×200 crop greener against red
by more than 8). The glow is a particle emitter's, Lighten (added to
what is behind), its picture `fxpa_lensflare`, a white flare in pure
grays on black (64², compressed to BC1). Left uncompressed alone, the
blotches go; the best effort (3,575 pixels) and BC3 (3,488: its color
block is BC1's) keep them. The cause: BC1's 565 colors cannot hold a
dark gray (5 bits of red and blue, 6 of green), so each texel of the
flare's dark rim takes a tint of a few levels, and where many
particles are added, layer on layer and scaled by their colors, the
tints add up to blotches. So a particle's picture, and one a TXI adds,
are kept as stored (`Role::compressed`; tests
`only_pictures_drawn_over_what_is_behind_are_compressed` and the kinds
of `a_texture_compressed_as_it_is_read_keeps_what_is_known_of_it`).
After it, area026's crop matches the uncompressed build's in two runs,
and the whole close view differs from it by a mean of 0.007 to 0.009
of 255, as two uncompressed runs differ by 0.006. What it costs:
area026's textures 4.6 to 4.9 MiB, area092's 17.7 to 17.9 (of 31.2
uncompressed, on develop at 03340a65); area092's opening was whole
after 111 to 129 ms in five runs (median 121) against 122 before it.

PLTs' colored copies were compressed at first (BC3 or BC1), with the
largest errors of all: `pmh0_chest*` copies up to 82 to 167 of 255 at a
texel, a mean of up to 7.5. A PLT's layers (skin, hair, cloth, leather,
metal, tattoos) are colored apart, so unrelated colors meet in sharp
edges inside a 4×4 block, which BC's two colors a block can't hold. To
see it, `look_plt_creatures` (mg-ui's `looks.rs`, ignored) stands eight
humans of parts in a row, their armor and clothes dyed with colors far
apart on each layer, and draws them close up on a device with BC (the
UI tests' own device has none, so nothing is compressed in them). Two
runs of each build match bit for bit; compressed against uncompressed,
the crop differed by up to 72 of 255 (mean 0.33, 590 pixels greener),
and at 4× the plates' edges were softened, the iridescent metal trim
flattened and the rivets dulled; at 1× it is hard to see. The copies
are now kept as stored (`Role::compressed`'s `plt`, a name in colors;
the unit test's PLT row and the load test's colored copy), and the crop
matches the uncompressed build's (one view; the other's 0.004 mean is a
weapon's TGA). What it costs, `FRAME_AREAS` on the campaign's areas
with the most creatures (one run each, the machine at a load of 13 to
15; textures allocated after the area opened, MiB):

| Area | Creatures | RGBA8 | PLTs kept | PLTs compressed |
|---|---|---|---|---|
| moonseabattle | 65 | 160.1 | 156.7 | 45.2 |
| zhentkeepwar | 67 | 36.5 | 35.2 | 29.0 |
| area089 | 55 | 28.1 | 22.1 | 19.4 |
| area092 | | 31.2 | 21.2 | 17.9 |

Moonseabattle's 933 textures are mostly PLT copies (its creatures in
their own colors), so keeping them gives back nearly all of what
compression saved there: 111.5 MiB. That is the price of the picture;
the copies are as the game composes them, and a PLT compressed without
the loss would need an encoder that keeps more colors a block (BC7),
which Moonglow doesn't have. Area092's opening with them kept: whole
after 113 to 126 ms in three runs.

**The opening's extra 50 ms.** `FRAME_VIEW=area FRAME_DETAIL=1`,
area092 with the Shaders off, the old tags built from `git archive`
with their own target folders and the same timing patched in (three
runs each; shown after, then whole after):

| Build | Shown | Whole |
|---|---|---|
| v2.13.3 | 75 to 82 ms | 102 to 110 ms |
| v3.0.0 | 83 ms | 174 to 175 ms |
| develop, RGBA8 | | 175 to 177 ms |
| develop, compressed | | 174 to 183 ms |
| fixed, RGBA8 | 84 to 91 ms | 105 to 119 ms (median 111) |
| fixed, compressed | 79 to 111 ms | 112 to 160 ms (median 122) |

(The fixed build's rows: five runs of each, taken in turn, the machine
at a load of 6 to 10.)

Every texture was read long before the last one showed: after the
area was first shown, the picture was drawn again only at the
animation's 40 ms steps, which two waits of about 45 ms each took.
v2.13.3 drew it within a frame. Ruled out: the harness's frame time
(v2.13.3 with 1/144 still 103 to 113 ms) and the test's drag (175
without it). A bisection over develop's merges named "wave 5d: editors
and bulk editing", which touched neither the renderer nor the area
view; one merge before it gave 110 and 156 ms in its two runs, so the
step was there before, by timing. The fix: the view draws again as soon
as every texture asked for is read and some were missing from the last
picture (`Renderer::textures_still_read`). The model viewer reads its
textures in the frame (only the area view reads them apart), so it never
waited for them.
Test `an_area_is_drawn_again_as_soon_as_its_textures_are_read` fails
without it. Compression costs the opening about 10 ms (the medians),
the last textures' encoding on the reader threads; the frames are not
longer for it (the longest 10.2 to 11.0 ms either way). It stays on: it
is paid once an area, off the window's thread, for 13.3 MiB of
area092's textures kept. Area026 is whole after 33 to 45 ms either way.
TDoD's woods (`game_shaders_frames_at_144`, two runs each, after the
fix) were whole after 549 to 555 ms with the Shaders on and 569 to 703
with them off, against 545 to 560 before: their opening is the DDS
textures' reading, which the step never held back.

**Anti-aliasing** (Options' Anti-aliasing, `FRAME_SAMPLES`), area092
at 3440×1400 (the GPU time of `frames_at_144`'s "turning idle" row,
the camera turning with the pointer still; the memory allocated at its
"overview" row; three runs on the RX 9070 XT, two on the Radeon 610M,
the integrated GPU):

| Samples | RX 9070 XT | allocated | 610M | allocated |
|---|---|---|---|---|
| 1 (Off) | 2.44 ms | 210 MiB | 27.0 ms | 274 MiB |
| 2 | 2.89 ms | 292 MiB | 29.2 ms | 380 MiB |
| 4 | 2.88 ms | 403 MiB | 30.4 ms | 494 MiB |

At 4× the color target holds 108.8 MiB and the depth and stencil 110.7;
at 2× 54.5 and 55.4; at 1× the depth 27.8 and no color target or
resolve. Off saves about 192 MiB a view and a tenth of the integrated
GPU's frame (15% of the discrete one's). 2× is offered only where every
target's format takes it (`TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`;
without it wgpu allows 1 and 4).

## The Shaders switch's cost, and the review's small slowdowns (25 November 2026)

Three deferred entries on what a frame costs, read on the RX 9070 XT
with `frame_perf` (release) and `render_area` (`MG_TIME_RENDER`),
TDoD's woods with the builder's haks (`MOONGLOW_HAKS`). Other sessions
loaded the machine throughout: 4 to 18 for the first sets, 26 to 61 for
the last, where every encoding time doubled. Each figure is
lowest / median / spread of three runs, builds alternated[^framecost].

**How a change under a millisecond was read.** At load 4 to 12,
`frame_perf`'s views of 20 to 30 ms move about 1 ms from run to run,
the Shaders-off path too, which none of the changes touched. Its "on
the processor" counts in the clock's ticks (about 1 ms). What resolved
the changes was both code paths built into one `render_area`, taken
in turn on alternate frames (a flag read once), with the medians of the
draws' encoding compared within one run. The camera turns as the frames
go, so odd and even frames are not quite the same work: a difference
the size of the swing between runs' medians is not taken as real.

**The woods seen whole, the switch on (the Shaders switch's dense-area
entry).** Every draw of the game's set group 1 (bones, the framebuffer
copies, the mask) and the zero vertex buffer again: 5,514 binds a frame
each. Group 1 is now one bind group per kind (each stage's bones
binding and size, whether it reads the framebuffer, whether it is
shaded, the frame) and is set only where another is bound. The zeros
are bound once per slot until a skin takes the slot
(`the_zeros_are_bound_again_after_a_skin_took_their_slot`). That cut
5,512 group 1 binds and 5,512 zero binds a frame, about 11,000 commands,
and 0.9 to 1.0 ms of encoding in the alternate-frame comparison (medians
26.77 to 25.75, 27.63 to 26.59, 29.44 to 28.55 ms in three runs).
`m_texture` and `skinmesh` are now written as a draw's other values
are, a small step. The pictures are the same: `render_area`'s woods,
water and area092, seen whole, close and turned, the switch on and off
(18 pictures), identical pixel for pixel with 27b2e44b's.

What is left with the switch on is the game's programs themselves:
5,500 to 7,500 pipeline changes a frame against about 3,200 off (a
program per material kind), two groups set at each (the vertex program
reads `texUnit0`, so group 2 is the material's), and the game's
see-through draws, kept in runs by material, change chunk 21,800 times
against 5,900 off. `frame_perf`'s parts at load 2 to 9 (27b2e44b, then
this work):

| Woods, 4× MSAA | Before | After |
|---|---|---|
| Seen whole, off, encoded | 12.28 / 13.69 / 1.47 | 12.80 / 13.61 / 1.23 |
| Seen whole, on, encoded | 16.95 / 18.06 / 1.61 | 16.10 / 17.78 / 1.75 |
| Turned, off, encoded | 18.08 / 18.26 / 0.92 | 19.17 / 19.58 / 2.67 |
| Turned, on, encoded | 28.14 / 28.44 / 1.01 | 29.14 / 30.68 / 3.09 |

Inside the spread either way, as the off rows show. The window's frame
with the wheel turning, seen whole, the switch on, pointed the other
way: 17.49 / 17.58 / 1.97 before and 19.54 / 20.77 / 19.62 after (two of
the three runs after above all three before; off level, 14.46 / 17.58
against 14.71 / 17.94). Timed again at load 32 to 61, every view took
twice as long (off 34 to 41 ms before, 41 to 49 after, the switch on 50
to 58 either way). So the 1.0 ms is `render_area`'s encoding, in the
alternate-frame comparison; the window's frame did not confirm it, and
in one set pointed the other way. The bar (on no
slower than off) is not cleared: on 16.1 against 12.8 seen whole, 29.1
against 19.2 turned.

Tried and dropped: the chunk as part of the game's see-through state
(slower: more pipeline changes than the chunk changes it saved); a run
split into its chunks' parts where nothing met lies between (chunk
changes 21,800 to 16,500, the encoding's medians 0.4 to 0.5 ms less in
three runs, the lowest figures one way and the other: inside the
spread). Not tried: the meshes' indices in one buffer for all chunks,
which would take a chunk change down to a vertex buffer alone. By the
trial before it, 5,300 fewer chunk changes were worth 0.4 to 0.5 ms,
so halving what 21,800 cost would be about 1 ms at most, against a gap
of 3.3 ms seen whole: a change to how every mesh is stored, for less
than the bar.

**The water view, the switch on (the 0.9 ms entry).** The gap is now
about 0.3 ms of the draws' encoding, not 0.9: lake seen whole, 4× MSAA,
0.85 / 0.90 off against 1.19 / 1.26 on (`render_area`: 0.66 against
0.92 to 0.97); the window's frame with the wheel turning 1.81 / 2.17 off
against 2.19 / 2.43 on, 2.06 against 2.44 at load 61, inside the budget.
The entry's ideas do not apply: these programs read neither `m_normal`
nor `m_mvp`, and Moonglow's own path works out the same inverse a
draw. What the 0.3 ms is: the game's programs' own pipelines and
material groups, as in the woods (a pipeline and two groups per
material kind, the pipelines looked up a draw); the profile shows no
single part apart from the same functions as with the switch off. The
work above changed the water by nothing measurable (medians 0.99 to
0.98, 1.17 to 1.14).

**The review's small slowdowns, against the build it timed** (develop
at a46ba28a, version 3.18.1, built beside this one; alternated, load
10 to 39):

| View | 3.18.1 | Now |
|---|---|---|
| Area092 turned, encoded | 2.28 / 2.39 / 2.02 | 1.86 / 1.89 / 3.28 |
| Area092 turned, on the processor | 1.99 / 1.99 / 1.99 | 1.37 / 1.63 / 3.60 |
| Grass field close, no grass, frame | 0.98 (load 10.5) | 1.17 (load 9.8) |
| The same, wheel, at load 16 to 21 | 2.21 / 2.25 / 0.05 | 2.94 / 3.25 / 1.20 |
| Grass field close, off, GPU (load 16 to 21) | 3.52 / 3.53 / 0.09 | 3.74 / 3.76 / 0.05 |
| Grass field close, on, encoded | 0.44 / 0.45 / 0.90 | 0.62 / 0.68 / 0.47 |
| Woods turned, off, GPU | 6.62 / 6.73 / 1.69 | 7.70 / 9.40 / 2.01 |
| Woods turned, off, GPU's quickest tenth | 6.21 to 7.08 | 7.30 to 8.87 |

Area092 is quicker than in 3.18.1 now.

The grass field close up is slower in every pair, not only the calmest.
Timed again at load 16 to 21 (three runs, alternated), the window's
frames without grass (pointer, idle and wheel) took 1.62 to 2.50 ms in
3.18.1 against 2.63 to 4.14 now, and the close view's GPU 3.52 to 3.61 against 3.74 to 3.79. The
two have different causes. The GPU's step was bisected over develop's
first-parent commits (old, middle and new alternated, three rounds a
step, load 11 to 30) to dd8d0bc1 and 03340a65, the next merge. Timed
again four rounds each, alternated: 3.47 to 3.63 ms before dd8d0bc1,
3.60 to 3.70 at it, 3.68 to 3.75 at 03340a65, which changes nothing
that draws (its renderer-side change is a test of a DDS header). The
step is dd8d0bc1, grass and see-through meshes taking the shadows
through a mask of the shadows laid and read after them, as in the
client. Its own measurement was 0.3 to 0.6 ms of GPU in area084 seen
close. It is kept: it is the client's look.

The window's frame did not bisect. Its old, middle and new figures
crossed at every step, at load 11 to 30: 3.18.1's ranged from 0.83 to
3.22 ms within one step. The review saw it as waiting rather than work
(less on the processor than before), and a frame that waits grows with
the machine's load. It is left in the deferred list for a calm machine.

The woods turned with Moonglow's shader take about
1 ms more of the GPU, beyond the spread in the quickest tenth. Built on
each side of 6b26e702 ("See-through draws of a material are drawn chunk
by chunk", 10 November) and alternated (load 27 to 42), the quickest
tenth was 7.27 to 7.90 before it and 8.69 to 9.64 after: that commit is
the step, as its own message measured (8.3 where 7.3, more material
changes). It is kept: it took 1.5 to 2.4 ms off the processor's
encoding of the same views, and that frame waits on the processor (19
ms encoded against 8 of GPU).

## The woods test's two GPU levels (26 November 2026)

`game_shaders_frames_at_144`'s woods overview with 4x MSAA and the
Shaders on (TDoD's haks) lands in one of two levels from run to run,
the code unchanged[^levels]. In the slow one the GPU's median is pinned
at 11.1 to 11.4 ms and its quickest tenth is 6.8 to 8.0; in the quick
one the median is 6 to 8.5 and the tenth 5.3 to 6.9. The same binary
falls in either, at load 2 as at load 50. Two builds timed in separate
runs can differ by 1.8 ms in the tenth (7.99 against 6.18) with
nothing between them but the level: that is how 94db6613 (the late
see-through draws after the others) was first read as costing 1.8 ms.

**The order costs nothing.** Built into one binary, 42027e27's order on
even frames and a variant on odd ones, the quickest tenths agreed within
0.3 ms in every run (base / variant): the same order both times 6.83 /
6.68, 7.97 / 7.82, 5.79 / 5.85; the late draws back among the others
6.47 / 6.67, 6.23 / 6.18, 6.79 / 6.86; the copy at the first see-through
draw 7.93 / 7.78, 5.98 / 6.00, 7.27 / 7.56; both, the order before
94db6613, 7.90 / 7.43, 5.86 / 5.82; the four late meshes (TDoD's
waterfall, `mtr_fm01_0475`, a flow map with `sample_framebuffer 2`) not
drawn 6.17 / 6.18, 6.07 / 6.29, 6.41 / 6.36; the late water drawn right
after the solid draws 6.88 / 6.87, 5.49 / 5.48. Two checks show the
method works. A debug count showed the late draws' place alternating
frame by frame (16,758, then 15,531). With every soft part left out the
odd frames took 4.78 against 7.93, at load 52. In whole runs of one
order each, 42027e27's order got 7.05, 5.37 and 5.31, and the order
before 94db6613 got 5.88, 6.87 and 5.66.

**The slow level goes with low clocks, not with memory.** Six runs of
42027e27, with the card's clock, its busy share, its VRAM and its GTT
read every 0.2 s beside each run (read only; nothing set):

| Run | Load | GPU median / tenth (ms) | Encoded (ms) | Clock, median (max) MHz | Busy | VRAM during (GiB) | GTT during (GiB) |
|---|---|---|---|---|---|---|---|
| 1 | 44 | 10.66 / 6.74 | 83.6 | 288 (814) | 4% | 6.90 | 0.32 |
| 2 | 10 | 7.06 / 6.14 | 21.6 | 963 (1,183) | 29% | 7.01 | 0.34 |
| 3 | 35 | 6.33 / 5.81 | 20.4 | 1,321 (1,549) | 37% | 7.39 | 0.39 |
| 4 | 29 | 5.96 / 5.79 | 24.5 | 1,514 (1,607) | 24% | 7.73 | 0.40 |
| 5 | 31 | 11.17 / 7.44 | 18.8 | 815 (1,046) | 38% | 6.91 | 0.32 |
| 6 | 34 | 7.14 / 6.27 | 17.9 | 948 (1,325) | 35% | 6.91 | 0.33 |

The two slow runs (1 and 5) had the two lowest clocks: 288 and 815 MHz,
against 948 to 1,514 MHz in the quick ones. The GTT stayed at 0.32 to 0.40 GiB in
every run and the VRAM at 6.9 to 7.7 of 16 GiB. `FRAME_MEMORY` said
1,580 MiB allocated each time. Nothing was pushed into system memory.
The frame keeps the GPU busy for a fraction of the time (a frame is
encoded in 18 to 84 ms and drawn in 6 to 11), so the driver's automatic
clocks settle low or high for the run. A sample every 0.2 s against
frames of 20 to 80 ms reads the clock only roughly, and run 5 was slow
at a clock close to run 6's: the link is the clock, not one figure.

**How to compare under it.** Don't compare one run with another. Either
build both paths into one binary and switch on alternate frames
(`renderer`'s variant flag toggled from `frame_perf`'s loop, the
quickest tenths of even and odd frames printed apart), or take many
runs of each, sort them by level (the median pinned near 11.2 ms or
not), and compare within a level.

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
[^area084]: render_area MG_TIME_RENDER on Tyrants of the Moonsea's area084, its stacks sampled with eu-stack (30 October 2026)
[^area084-again]: sound_ranges_frames_at_144 and game_shaders_frames_at_144 (with the builder's haks), and render_area MG_TIME_RENDER on area084, on the RX 9070 XT and the Radeon 610M, before and after, its stacks sampled with eu-stack (2 November 2026)
[^editor-perf]: editing_a_huge_script on the game's nwscript.nss, before and after the lines laid out one at a time, its stacks sampled with eu-stack (2 November 2026)
[^texmem]: frames_at_144 (FRAME_VIEW=area, FRAME_DETAIL, FRAME_MEMORY, FRAME_UNCOMPRESSED, FRAME_SAMPLES) on area092 and TDoD's woods, v2.13.3 and v3.0.0 built beside develop, with and without compression and the fix (25 November 2026)
[^volumes-apart]: sound_ranges_frames_at_144 with FRAME_SHADOW_QUALITY=high, Fast and High, before and after the stencil volumes were made apart, and FRAME_MEMORY=labels after the area closed (25 November 2026)
[^framecost]: game_shaders_frames_at_144 (FRAME_AREA=tdod and water), frames_at_144 and grass_frames_at_144, three runs alternated with 27b2e44b, v3.18.1 and each side of 6b26e702 built beside develop; render_area MG_TIME_RENDER with both paths on alternate frames; eu-stack samples (25 November 2026)
[^woods]: game_shaders_frames_at_144 with the builder's haks: TDoD's woods, area092 and the water area, before and after (27 October 2026)
[^levels]: game_shaders_frames_at_144 (FRAME_AREA=tdod, "overview, 4x MSAA, Shaders on") built at 42027e27 and with its order's variants on alternate frames of one run; six runs with FRAME_MEMORY=1 beside a read-only sampler of the RX 9070 XT's pp_dpm_sclk, gpu_busy_percent, mem_info_vram_used and mem_info_gtt_used (26 November 2026)
