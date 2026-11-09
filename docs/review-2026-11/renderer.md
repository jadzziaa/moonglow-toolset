---
type: Report
title: 'Code review of November 2026: the renderer'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the renderer (mg-render): 30 in all (2 high, 7 medium, 21 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, renderer, mg-render, gpu]
status: draft
generated: { by: claude-code/claude-fable-5-2, at: 2026-11-09T22:00:43Z }
stale_after: 2027-01-01T00:00:00Z
sources:
  - id: code
    resource: 'The source named in the title, at commit a46ba28a (develop, 2026-11-09), read by a reviewing agent under the brief the overview gives'
    title: The code as read
  - id: last-review
    resource: commit 2253fa78 (2026-10-27)
    title: The last whole-workspace review, which an entry's Since is told against
---

# Code review of November 2026: the renderer

One part of [the review of 9 November 2026](overview.md): the renderer (mg-render). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## High

### R1. The batcher's count of a box's squares overflows, and a huge solid mesh then loops over 2^64 squares, allocating each

- **Kind:** bug
- **Severity:** high (the window hangs and memory grows without bound; nothing panics in a release build, so no recovery copy)
- **Where:** crates/mg-render/src/batch.rs:243 (the product), :350-368 (the loop in `place`), :267-268 (the same loop in `meets_after`); reached from crates/mg-render/src/renderer.rs:2918
- **Since:** before 2253fa78 (f2eda00e)
- **Scenario:** a drawn mesh has a vertex at (-3e10, -3e10, 0) and another at (3e10, 3e10, 0) (two stray `verts` rows are enough: every vertex is boxed; or garbage floats in a compiled model, or an instance transform with a huge scale). In the solid pass (squares of 10 m) both spans saturate to `i32::MIN..=i32::MAX`; `count` is 2^32 × 2^32, which wraps to 0 as `i64` (2^32 × 2^31 wraps to `i64::MIN`), so `count <= WIDE` holds and `place` walks every square with a hash-map entry each. Debug and test builds panic instead ("attempt to multiply with overflow"). Taken whenever Moonglow's own shader draws the mesh solid: Shaders off, a pipeline still pending, a program that failed. See-through draws are safe (`screen_box` clamps to ±1).
- **Fix:** compare each span with `WIDE` before multiplying (or `checked_mul`), so the box goes to the `wide` list.
- **Test:** unit test in `batch.rs`: `pass(10.0, 1e-3)`, then `place` with `Bounds { min: (-1e30, -1e30, 0), max: (1e30, 1e30, 1) }` returns, and a second draw that meets it keeps its order.
- **Confidence:** confirmed by reading, re-read (`batch.rs:225-245` and `:350-368`; the workspace's profiles set no `overflow-checks`; the box is made at `renderer.rs:2382-2401` after a frustum test a huge ball passes)
- **Read again by the reviewer:** holds: the product of the two spans is taken in `i64` before it is compared with `WIDE` (batch.rs:243), and `place` walks every square (:352-367).

### R2. Vertex animation on a mesh with a mirrored texture seam indexes past the mesh's vertices and panics

- **Kind:** bug
- **Severity:** high (a crash in the model viewer, whose call is not guarded; in the area view and previews the guard leaves the object out)
- **Where:** crates/mg-render/src/anim.rs:557-567, :588-590; crates/mg-render/src/model.rs:405-410, :489-490
- **Since:** after (ee50505f made `mesh_vertices` return the copies)
- **Scenario:** a text model has a `trimesh` with a mirrored UV seam and no tangents, and an animation whose node of that name lists `animverts`. Playing the animation in the model viewer ends the program (index out of bounds). Without the panic the copies would stay at rest while their originals move: line 561 zips with `m.source`, which has no entry for a copy.
- **Fix:** carry `Uploaded::originals` into `animated_vertices` and `smooth_normals` (position, normal and source by the original, as `Dangly::new` does), or return `None` unless the mesh is `MeshExtra::Anim`; guard the call in `model_view.rs` either way.
- **Test:** unit test in `anim.rs`: `animated_vertices` of the mirrored card of `made_tangents_split_a_mirrored_seam_as_the_client_does` with `extra: MeshExtra::None` and two vertex sets returns 8 vertices, the copies moved with their originals.
- **Confidence:** confirmed by reading, re-read for the index (`anim.rs:545-590`: `mesh_vertices(m)` returns the seam's copies after the mesh's own vertices, and `smooth_normals` reads `m.vertices[i]` for every `i` below that length; `model.rs:405-410`, `:480-490`: only `MeshExtra::Anim` is spared the split). Sub-reader for the rest: mg-mdl's text reader gives an animation node `anim_mesh` whatever the geometry node is (`ascii.rs:895-914`), and `mg-ui/src/model_view.rs:1192` is outside `guard::guarded`.
- **Read again by the reviewer:** holds for the index: `smooth_normals` reads `m.vertices[i]` for every vertex `mesh_vertices` returned (anim.rs:557, :588-590).

## Medium

### R3. A light's shadow is planned per instance, so a creature made of parts takes one of the 16 maps for each body part

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-render/src/shadow.rs:482 (the filter), :501 (the cap), :524-526 (ball, feet and light per instance); crates/mg-area/src/scene.rs:1165
- **Since:** after (9a555692)
- **Scenario:** an interior with a lamp and two human NPCs. The skeleton instance has no meshes; each body part, head and held item is a "creature" to `light_plan`. The 16 part instances nearest `camera.target` get a 512² map each; the rest of that NPC and the other NPC cast nothing. Each part picks its own nearest light from its own attach point and tests "the light is inside me" against its own small ball. The readings and tests used an ogre: one model, one instance.
- **Fix:** give `Instance` (or `Scene`) the object its instances belong to; in `light_plan` box a creature's instances together, pick one light from the base's place and draw them into one map with one range.
- **Test:** `tests/render.rs` beside `a_creature_s_shadow_falls_away_from_its_light`: two creatures of 20 flagged instances each under one lamp, every part of both darkens the floor.
- **Confidence:** confirmed by reading, re-read (`shadow.rs:478-541`: one map per flagged instance, stopped at `MAX_LIGHT_SHADOWS`; `mg-area/src/scene.rs:1161-1167` flags every instance of a creature; `mg-preview/src/compose.rs:465-499` makes one `Instance` per part). No picture taken. `docs/deferred.md:542-545` says "Moonglow casts up to 16" creatures: the code casts up to 16 instances.

### R4. Light Shadows ticked with Creature Shadows unticked draws no light shadows

- **Kind:** bug
- **Severity:** medium (if the two are meant to be independent, as the menu and `docs/manual/04-areas.md:191-200` present them; the owner's call)
- **Where:** crates/mg-render/src/shadow.rs:511; crates/mg-area/src/scene.rs:1164-1165
- **Since:** after (9a555692, fcd413d3)
- **Scenario:** Shadows menu, untick Creature Shadows, leave Light Shadows on, in an interior with a lamp. Every creature's instances have `casts_shadows == false` and `casts_light_shadows == true`; `light_plan` keeps them, every mesh fails `casts()`, the plan is `None`. The button still shows the kind as on.
- **Fix:** in `light_plan` test the mesh conditions without `inst.casts_shadows` (`casts_light_shadows` carries the instance's own flag); or, if the coupling is meant, gray Light Shadows out and say so.
- **Test:** the `render.rs` light-shadow test with `casts_shadows: false, casts_light_shadows: true`; an mg-area unit test of the flags.
- **Confidence:** confirmed by reading, re-read (`shadow.rs:103-111`: `casts()` requires `inst.casts_shadows`; `:511` calls it in `light_plan`; `mg-area/src/scene.rs:1154-1165` sets `casts_shadows = view.creature_shadows && …` for a creature)

### R5. The stencil volumes' target is never let go of, is one size for the renderer every view shares, and the model viewer draws with whichever kind the last area view set

- **Kind:** performance (the last part a bug)
- **Severity:** medium
- **Where:** crates/mg-render/src/volume.rs:526-538; crates/mg-render/src/renderer.rs:3292-3303, :472-475; crates/mg-ui/src/model_view.rs:1282
- **Since:** after (078e163f, 2a0125c0)
- **Scenario:** (a) a High-shadows area view at 3440×1400 with 4 samples makes a depth-and-stencil target of about 100 to 150 MB; closing the tab, switching to Fast, or the view letting go of its targets after two seconds frees none of it, so most of what the 2026-11-01 change freed stays held. (b) Two views of different sizes drawn in one frame with High on (an area tab beside the model viewer, which inherits `shadow_volumes`) remake that target twice a frame. (c) The model viewer's shadow is the map's before any area was drawn and the stencil volumes' after (skins then cast nothing, `volume.rs:482`). (d) Sub-reader, by arithmetic: each casting mesh's volume buffers (444 to 672 B a face against the mesh's own 90 to 130) live in a `OnceLock` in `GpuMesh` for the model's life, High on or off.
- **Fix:** keep the stencil target with the view's `Targets` (made with them when High is the setting), or per (size, samples) and dropped when not laid for a few frames; drop it when `shadow_volumes` goes off; set `shadow_volumes` in the model view before it draws.
- **Test:** `frame_perf` with `FRAME_SHADOW_QUALITY=high FRAME_MEMORY=labels` (label "shadow volumes") after the view lets go; an app test that `holds_targets() == false` leaves no target of that size in the renderer.
- **Confidence:** confirmed by reading, re-read (`volume.rs:436-446`, `:526-538`: the target is remade when the size or samples differ and never dropped; `renderer.rs:3290-3303` is the only use of `self.volumes`; `mg-ui/src/model_view.rs:91` makes the one `Renderer`; `mg-ui/src/area_view.rs:3558` is the only write of `shadow_volumes`). Sizes by arithmetic, not measured.

### R6. A backend is recorded as failing "until the driver changes" on evidence that doesn't implicate it

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-render/src/backend.rs:136-167, :230-244; apps/moonglow/src/main.rs:369-374; crates/mg-ui/src/renderer_check.rs:281-305, :373-382
- **Since:** after (16127cf0, dcb436f5)
- **Scenario:** (a) `startup_check` records every failed check alike: "no device: …", "opening the device panicked" and "the GPU did not finish" (a 10 s read-back) are not pictures compared, yet one failed `request_device` (GPU memory taken by another program) makes Automatic pass that backend over at every later start. (b) The watch's notice offers Restart with … also when every tile's model failed to build or textures never came; the click records the backend though the cause is the content. (c) The key is GPU, driver and backend: a Moonglow release that fixes the cause never tries the backend again on that driver.
- **Fix:** keep Moonglow's version in the record and drop another version's at start; record only a compared picture or a GPU error; for a watch failure with failed tiles or a wait, restart without recording.
- **Test:** `settings.rs` unit: `failed_backends()` leaves out another version's record; `main.rs` unit: `startup_check` with `failed = [(info, "no device: …")]` records nothing.
- **Confidence:** confirmed by reading, sub-reader (`Failed`, `driver_of`, `pick_adapter` in `backend.rs`; `startup_check` in `apps/moonglow/src/main.rs`; `watch_failed` and `restart` in `mg-ui/src/renderer_check.rs`; `GraphicsFailure` in `settings.rs`)

### R7. Automatic takes OpenGL or a software adapter without a word, and can take OpenGL where a GPU adapter is left

- **Kind:** bug (an error swallowed; a wrong pick)
- **Severity:** medium
- **Where:** crates/mg-render/src/backend.rs:233-257; apps/moonglow/src/main.rs:363-404
- **Since:** the silence before 2253fa78 (2f5fba52); the pick after (0823eb02)
- **Scenario:** (a) Linux without the Vulkan driver package, or Windows on the basic display driver, RDP or a VM: the only adapters are OpenGL or llvmpipe; Automatic takes one, the startup check may pass (unlit quads), and the views are then drawn wrong (the deferred list: 20 of 44 render tests fail on GL) or at a few frames a second, with no log line or notice. (b) Linux laptop with `WGPU_POWER_PREF=low`, adapters [Vulkan Intel, Vulkan NVIDIA, llvmpipe, OpenGL Intel], Vulkan Intel recorded: the kept order begins with OpenGL Intel, which is taken, against the commit's rule.
- **Fix:** when `skip`, take the first of `wgpu_order(adapters, power, |a| worth(a) && !recorded(a))`; let `Pick` say the adapter isn't `worth` and warn in `startup_check` (log and notice bar) with what to install.
- **Test:** `backend.rs` units: `pick_adapter` over those four adapters gives index 1; over `[GL]` and over `[Dx12 Cpu]` gives a pick marked as a fallback.
- **Confidence:** confirmed by reading; re-read for the pick (`backend.rs:230-245`: `skip` is decided among `worth` adapters, but `first` is the first of all kept ones in kind order); sub-reader for the silence (`main.rs:363-404` warns only for `missing`, `switched`, `failing`) and for wgpu-hal reporting an Intel GL adapter as `IntegratedGpu`

### R8. A lost device (driver reset or update, GPU removed) is told to no one

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-render/src/guard.rs:101-114; crates/mg-render/src/probe.rs:118-124
- **Since:** before 2253fa78 (never handled)
- **Scenario:** the device is lost with a module open. Every wgpu call is a silent no-op from then on: the window shows stale frames, with no log line and no notice. The builder kills Moonglow; unsaved work is in the last recovery copy only. In the startup check a loss shows as "the GPU did not finish" and is recorded as the backend's failure (finding 6).
- **Fix:** `device.set_device_lost_callback` in `note_gpu_errors` and in `probe`: note it, write the recovery copy at once, tell the builder with a native dialog; in the probe say "the device was lost" and don't record.
- **Test:** `tests/render.rs`: a headless device with the handlers set, `device.destroy()`, poll; the loss is noted once.
- **Confidence:** suspected, sub-reader. Read: wgpu 30.0.1 drops `DeviceLost` errors from the uncaptured-error handler ("surfaced via callback"), and `rg -i 'device_lost|DeviceLost'` over `crates/` and `apps/` finds nothing. Settled by restarting the graphics driver on Windows (Win+Ctrl+Shift+B) with an area open.

### R9. The renderer every 3D view shares keeps one view's frame-to-frame state, so a second view drawn between an area's frames undoes it

- **Kind:** performance
- **Severity:** medium (low if two 3D views in one frame turn out rare)
- **Where:** crates/mg-render/src/shadow.rs:889-901; crates/mg-render/src/reach.rs:281-303; crates/mg-render/src/renderer.rs:3514, :513-515
- **Since:** after (fcd413d3 the map's `holds`, 71be7af6 the light choices, 2e5fc3c4 the shapes)
- **Scenario:** a large area with shadows on, animating (torches), and the model viewer in a split beside it playing, a blueprint's preview open over it, or the palette's thumbnails being made (a few a frame). Each draw of the other scene replaces `holds`, so the area's next frame draws its whole shadow region into the 2048² map again; clears the reach cache, so every mesh chooses its lights again (the cache's own commit measured 1.2 ms a frame on area084); and drops the area's line shapes, sent again. Finding 5's stencil target and finding 10's copies are the same pattern.
- **Fix:** a per-view state the caller owns and hands to `render_targets` (the reach cache, the shape buffers, the sun map with its `holds`), or two-slot caches keyed by the scene's lights and the plan.
- **Test:** `tests/render.rs`: draw A, B, A with a counter of shadow-map casts: one for A in all; `frame_perf` with `FRAME_DETAIL=1` and a model view drawn each frame.
- **Confidence:** the mechanism confirmed by reading, mine (`shadow.rs:883-901`: the sun's map is redrawn unless `holds` equals this plan's light, casters and bones; `reach.rs:281-303`: the kept light choices are dropped when the lights differ; `renderer.rs:3514`: every shape buffer the current scene doesn't draw is dropped; `mg-ui/src/model_view.rs:91`, `:1282`, `:2335`: the model viewer, the previews and the thumbnails draw through the same `Renderer`, the model viewer with a shadow of its own). The cost is suspected: `frame_perf` with a second view drawn between the area's frames.

## Low

### R10. A target's framebuffer copies stay alive in the renderer after the view lets go of the target

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-render/src/renderer.rs:3189-3195; crates/mg-render/src/game.rs:2761, :2685
- **Since:** after for the particle depth group (222f7c0d); shows since views let go of targets (2026-11-01)
- **Scenario:** an area with water or soft particles is drawn, its tab closed, and the builder works in the script editor. The color and depth copies (8 B a pixel, about 38 MB at 3440×1400) stay until another view draws the same pipelines with other copies.
- **Fix:** `Renderer::forget_copies(id)` called from `Viewport3d::let_go_of_targets`, clearing `particle_depth`, `game.fb` and that id's `fb_groups`; or clear them at the end of `draw_scene`.
- **Test:** `frame_perf` `FRAME_MEMORY=labels` ("framebuffer colour", "framebuffer depth") after a view is let go.
- **Confidence:** confirmed by reading, re-read (`renderer.rs:3185-3195`: `particle_depth` holds a bind group on the depth copy until another id replaces it, and `game.fb` its views until the next frame; `game.rs:2750-2762`: `fb_groups` holds one group per pipeline on both copies until redrawn with another id or the buffers' generation changes; `Copies::id` is unique, so nothing is confused, only kept)

### R11. The surfaces kept for a model leave out `Instance::env_map_default`, so two placeables of one model share the first one's reflection

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/renderer.rs:1810, :1912
- **Since:** after (86a2a56b)
- **Scenario:** two placeables.2da rows use one model, one with `Reflection` `default` (map `chrome1`, flag set) and one naming `chrome1` (flag clear), in an area whose map isn't `chrome1`. Same model id, same map number, same key: a mesh whose TXI says `envmaptexture default` gets whichever was drawn first.
- **Fix:** add `env_map_default` to `SurfaceKey`.
- **Test:** `tests/render.rs`: two instances of one model with the same `env_map`, differing in the flag, in a scene with another `env_map`; the two pictures differ.
- **Confidence:** confirmed by reading, re-read (`renderer.rs:361-369` and `:1810-1817`: the key; `:1912`: `surface` reads the flag)

### R12. A texture that is no PLT is read and uploaded once for every set of colors that asks for it

- **Kind:** performance (GPU memory, reading time)
- **Severity:** low
- **Where:** crates/mg-render/src/assets.rs:89-93; crates/mg-render/src/renderer.rs:1864
- **Since:** before 2253fa78
- **Scenario:** body parts, cloaks, helmets and items all carry colors. A part whose texture is a plain DDS (custom heads commonly are) is kept as `head#1,2,…` and again as `head#3,4,…`: the same pixels once per differently colored NPC.
- **Fix:** `LoadedTexture` says whether the colors were used; where not, keep the entry under the plain name and point the colored name at it.
- **Test:** `tests/render.rs`: two instances of a TGA-textured model with different `plt_colors`; `texture_memory().textures` counts one.
- **Confidence:** the path confirmed by reading, re-read (`renderer.rs:1863-1866`: slot 0 of any instance with `plt_colors` gets `colored_name`; `assets.rs:68-93`: a plain DDS or TGA is returned for a colored name; `Renderer::texture` keeps it under that name). The cost in a real module is suspected: `Renderer::texture_memory()` in a module with DDS heads.

### R13. Every PLT texture reads and decodes the palettes ten times over

- **Kind:** performance (and duplication: mg-ui's loader keeps them)
- **Severity:** low
- **Where:** crates/mg-render/src/assets.rs:131-137; the kept copy is crates/mg-ui/src/images.rs:290-303
- **Since:** before 2253fa78
- **Scenario:** a creature of parts has about twenty PLT textures; fifty differently colored NPCs come to 10,000 palette reads (a resource lookup, a TGA decode and an RGBA copy each) where six would do.
- **Fix:** keep the six palettes per load order (dropped when the resources change) and ask only for the layers the PLT uses; one place both loaders use.
- **Test:** unit test with a counting loader: coloring a second PLT asks for no palette.
- **Confidence:** the path confirmed by reading (re-read `assets.rs:131-137`: the palette closure reads and decodes on each call; sub-reader for `Plt::colorize_by` calling it per layer, `mg-image/src/plt.rs:119-128`). The cost is suspected: time an area of part-based creatures with a kept palette.

### R14. Cube maps ignore Options › Largest textures, and are always uploaded uncompressed

- **Kind:** bug
- **Severity:** low (memory, on the GPUs the setting is for)
- **Where:** crates/mg-render/src/texture.rs:219-232
- **Since:** after (94e4a46e gave `upload` the limit alone)
- **Scenario:** with the limit at 256, a hak's 1024-pixel cube map takes 6 × 4 MB × 4/3 = 32 MB where 2 MB was asked for; a DXT cube's faces are decoded to four to eight times their stored size.
- **Fix:** start the cube at the first level within the limit; upload BC faces as they are where all six agree and the device takes them.
- **Test:** the cube test in `texture.rs` with `set_texture_limit`: the texture's width is the limit.
- **Confidence:** confirmed by reading, re-read (`texture.rs:123-135`: `upload` applies `gpu.texture_limit()`; `:219-232`: `upload_cube` has no limit and makes `Rgba8Unorm` whatever the faces are)

### R15. The shadow plans carry bones nothing draws: the sun's map is redrawn for creatures out of its region

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-render/src/shadow.rs:283-293, :889-901, :515-529
- **Since:** after (fcd413d3)
- **Scenario:** (a) `plan` pushes bones for every casting skin of the scene before the cull to the map's region, and `cast` compares all of them: one animated cloaked NPC anywhere in a 32×32 area changes them every frame, so the map is redrawn each frame with a still camera. (b) `light_plan` truncates a creature's casters when no light casts for it but leaves its bones: with one lamp and 200 NPCs all their skins' bones are computed and written. (c) With High, skins cast nothing, yet both plans compute them.
- **Fix:** decide what is kept from the boxes first, then push bones for the kept casters; truncate `plan.bones` with the casters; skip skins by the quality.
- **Test:** unit tests in `shadow.rs`: a posed skinned caster outside the region leaves `plan.bones` empty; a creature out of every light's reach leaves none.
- **Confidence:** confirmed by reading, sub-reader (`shadow.rs:283-293`, `:314-322`, `:515-529`, `:889-901`; `volume.rs:482`, `:498`); I re-read `cast`'s comparison of `plan.bones` (`:889-893`)

### R16. Particles: about thirty lookups by name an emitter a frame, each emitter's quads built twice, and a material key of strings made for every batch

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-render/src/particles.rs:195-218, :298-370, :500-546 (`param` by name, `to_ascii_lowercase` per call), :581-679 (a `Vec<(f32, [ParticleVertex; 6])>` with a distance nothing reads, flattened into a second `Vec`); crates/mg-render/src/renderer.rs:3029 (a fresh `Vec` each frame), :3046-3049 (eleven `Option<String>`s, a `String` and a hash per batch for `material_group`)
- **Since:** before 2253fa78 (the softness lookups and the unused sort key after: 03b0023a)
- **Scenario:** mg-area steps up to 256 objects, 128 tiles and 256 tile flames. Each emitter does about 15 linear searches by string in `update` and 15 in `batches`, most for values it doesn't use that frame (the point-to-point and spawn groups), plus five to seven `String`s; a new simulation is stepped 80 times in the frame it appears. At 12,000 particles the vertices (2.9 MB) are written three times a frame before the upload.
- **Fix:** resolve each parameter's controller once in `Particles::new` and parse `update`/`render`/`blend` into enums; push the six vertices straight into one `Vec` kept from frame to frame; keep each batch's material by its texture's name alone.
- **Test:** a `frame_perf` view of an interior with a flame on every tile.
- **Confidence:** suspected for the time (counted by reading, not measured: `frame_perf` on an interior with a flame on every tile, `FRAME_DETAIL=1`). Sub-reader for `particles.rs`; mine, re-read, for `renderer.rs:3029-3074`.

### R17. A looping Single emitter's particle stays where it was born when its object is moved

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/particles.rs:452-466
- **Since:** before 2253fa78
- **Scenario:** drag or rotate a placeable whose model has `update Single`, looping, without `inherit`: its one particle stays at the old place until the area is read again.
- **Fix:** keep a Single emitter's particle in the emitter's space, or place it again when the emitter moved.
- **Test:** unit test: a Single looping emitter, two updates with different transforms, `batches` centered on the new place.
- **Confidence:** confirmed by reading for the mechanism, sub-reader (`particles.rs:452-466`: born in world space unless `inherit`, life -1; `:390-402`: moved only by its velocity, never dies; `mg-area/src/scene.rs:805`, `:1395`: the simulation's key doesn't change on a move). Which stock models have such emitters was not checked.

### R18. Chunk particles still tumble at `particlerot` turns a second; quads were corrected to radians

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/particles.rs:707; the module doc, lines 21-25, still says turns
- **Since:** after (03b0023a changed the quads alone)
- **Scenario:** a chunk emitter with `particlerot 1` tumbles 6.28 times faster than a quad of the same emitter turns.
- **Fix:** one reading of `particlerot` for both, and the doc brought up to date.
- **Test:** `chunks_and_trails` with `particlerot` set: the chunk's angle after a second.
- **Confidence:** suspected, sub-reader (the drift confirmed by reading `particles.rs:707` against `:538`; what the client does with chunks was not read: a chunk emitter beside `reading_particle_spin_and_order` in `client_render.rs`)

### R19. Every `Particles` starts from the same seed: identical emitters made together move in lockstep

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/particles.rs:266; made at crates/mg-area/src/scene.rs:817, :886, :955
- **Since:** before 2253fa78
- **Scenario:** an area opens with a row of tile torches: each flame draws the same random numbers in the same order and is run ahead the same 20 s, so every flame is a translated copy of the others.
- **Fix:** `Particles::new_seeded(model, seed)`, the seed mixed from the caller's key (object or tile index); the picture tests stay deterministic.
- **Test:** unit test: two simulations with different seeds differ after ten updates; the same seed repeats.
- **Confidence:** suspected, sub-reader (`particles.rs:266`; all simulations of a frame get the same `dt`; how visible it is needs a picture: two `fx_flame01` side by side in the area harness)
- **Read again by the reviewer:** holds (drawn in the sample): `rng: Rng(0x9E37_79B9_7F4A_7C15)` for every `Particles` (particles.rs:266).

### R20. "The box around these points" is written sixteen times; each copy forgets what came before a NaN, and one such box removes the sun's shadows

- **Kind:** duplication (with a bug)
- **Severity:** low
- **Where:** crates/mg-render/src/model.rs:244-248; crates/mg-render/src/shadow.rs:172-178, :279-281; crates/mg-render/src/renderer.rs:3671-3676; and `rg 'splat\(f32::MAX\)'`: shadow.rs 4, renderer.rs 3, mg-ui `model_view.rs` 3, mg-area `grass_view.rs` 2, one each in mg-area `walk.rs` and `scene.rs`, mg-preview `compose.rs`
- **Since:** before 2253fa78; the shadows' use after (01bc8272)
- **Scenario:** a mesh with one NaN coordinate mid-list gets a box of only the vertices after it, and is culled while in view. A tile or static placeable with a NaN position (a corrupt GIT float): the receivers' box loses every instance before it; if it is the last, `seen` returns `None` and the area has no sun shadows.
- **Fix:** one `box_of(points) -> Option<(Vec3, Vec3)>` beside `box_corners` in `reach.rs` that skips points that aren't finite; receivers and casters with no finite box are skipped.
- **Test:** unit tests: a NaN in the middle changes nothing; `shadow::plan` with a NaN-transform receiver between two good ones gives the same light matrix as without it.
- **Confidence:** confirmed by reading, sub-readers (glam 0.30's `Vec3::min` is `if a < b { a } else { b }`: a NaN replaces the running value and the next point replaces the NaN); I re-read `renderer.rs:3671-3676`. Not checked: whether mg-mdl or mg-area lets a NaN through.

### R21. A dangly mesh's simulation takes NaN parameters and never recovers once a vertex is NaN

- **Kind:** bug
- **Severity:** low (the mesh vanishes in the model viewer)
- **Where:** crates/mg-render/src/dangly.rs:53, :99-100, :115-128
- **Since:** before 2253fa78
- **Scenario:** `period nan` passes `>= 60.0 || <= 0.0`; positions go NaN on the second update, `offset.length() > limit` is false for NaN, and the positions are reset only when the vertex count changes. Separately `steps` is capped at 240 but `dt` is not: past about 8 s at a period near 60 the step is unstable.
- **Fix:** require finite, positive parameters in `new`; cap `dt` in `update`; set a vertex that isn't finite back to its anchor.
- **Test:** extend `vertices_lag_stay_within_limits_and_settle`: 60 s at omega 59 leaves every position finite; a NaN period makes no dangly mesh.
- **Confidence:** confirmed by reading, sub-reader (`dangly.rs:53`, `:99-128`). The toolset's one caller caps `dt` at 0.25 s, so the long-step case needs another caller (Moonglow Viewer's was not traced).

### R22. Material value sets, renames and map names are numbered for good, and a value set is found by a linear search for every instance of every frame

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-render/src/renderer.rs:1748-1760, :1725-1744, :548, :1406-1420
- **Since:** after (84d05106)
- **Scenario:** The Domains of Dread tints placeables by their locals (`mg_preview::tinted`): 2,000 tinted placeables in 50 colorways come to about 50,000 slice comparisons a frame (in the order of a millisecond). Dragging a tint or a material value in an editor adds a set, a cloned `Slots`, surfaces and the game's templates per step, kept until the resources change; the sets themselves and the renames are kept across modules, and the search grows with them.
- **Fix:** number a set by the `Arc`'s address first (`Arc::ptr_eq`), then by a hash of its contents in a map; clear `value_sets`, `valued`, `renames` and `names` with the surfaces in `clear_textures`.
- **Test:** unit test: `values_id` of 1,000 distinct sets, then of one of them, compares no more than a few; `clear_textures` leaves `value_sets` empty.
- **Confidence:** confirmed by reading for the mechanism, mine (`renderer.rs:1748-1760`: `position(|v| v == values)` compares the slices' contents; called from `surfaces` at `:1816` for every instance with values, before culling; `:1406-1420`: `clear_textures` clears neither `value_sets`, `renames` nor `names`; `valued` holds a cloned `Slots` per slots and set). The time is suspected: `frame_perf` on a TDoD area with tinted placeables.

### R23. The light grid is built every frame, though with unchanged lights no mesh asks it

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-render/src/renderer.rs:2279; crates/mg-render/src/reach.rs:53-139
- **Since:** after (71be7af6)
- **Scenario:** an area of several hundred to a few thousand lights with the camera still or turning: every mesh finds its choice by place, and the grid (two passes over each light's squares, four allocations) is filled and dropped.
- **Fix:** build the grid on the first miss, or keep it in `ReachCache` while the lights stay.
- **Test:** `frame_perf`'s area084 view before and after.
- **Confidence:** confirmed by reading, re-read (`renderer.rs:2279` builds `LightGrid::new` before `reach_cache.begin`; `reach.rs:338-360` uses it only on a miss). The cost is suspected (0.05 to 0.1 ms for 750 lights): `frame_perf`'s area084.

### R24. A look the GPU has done but nobody has read is given up as "lost" when the view wasn't drawn for 5 s

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/watch.rs:595-601; crates/mg-ui/src/area_view.rs:726-737
- **Since:** after (7b6f644d)
- **Scenario:** take a look, then let the view's next drawn frame come more than 5 s later (its tab behind another, or an idle window and then a camera key). The look, long done, is dropped and the debug log says "a look at the area view was lost (the GPU didn't give it back in 5 s)": the log the deferred list asks builders to send for the AMD cases. A view drawn more than 5 s apart never has a look judged.
- **Fix:** read before giving up: call `take` first, or make `give_up(&gpu, now)` poll and drop only what is still unmapped.
- **Test:** `tests/render.rs`: `look(…, 100.0)`, wait on the device, then `give_up(&gpu, 106.0)` is false and `take` gives the look.
- **Confidence:** confirmed by reading, re-read (`watch.rs:595-601`: `give_up` judges by the clock alone; `mg-ui/src/area_view.rs:726-737` calls it before `take`, the only poll and read, and only on this view's frames)

### R25. On Metal a record names every GPU and never ends; with one backend, Options says Automatic passes over what it takes

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/backend.rs:147-167; crates/mg-ui/src/options.rs:1264-1274
- **Since:** after (16127cf0)
- **Scenario:** on a Mac `Failed::of` is (0, 0, "", Metal) for any GPU and any macOS: Don't Show Again on the lit check's notice hides it for good, though the manual says a driver update shows it again. Wherever one `worth` adapter exists, a record is never passed over, yet Options says "Automatic passes over Vulkan on …" and offers Try Again.
- **Fix:** `driver_of` falls back to the adapter's name and the OS version where wgpu gives no driver; Options says "failed here" where `pick_adapter` would not pass the record over.
- **Test:** `backend.rs` unit: two Metal infos of different names give different `Failed`.
- **Confidence:** confirmed by reading, sub-reader (wgpu-hal 30.0.1's Metal adapter fills the name only: vendor 0, device 0, driver ""; `backend.rs:145-167`, `:236`; `mg-ui/src/options.rs:1260-1283`)

### R26. `WGPU_BACKEND` set but empty or misspelled leaves no backend at all

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/gpu.rs:137-138; apps/moonglow/src/main.rs:313
- **Since:** before 2253fa78
- **Scenario:** `WGPU_BACKEND= moonglow` or `WGPU_BACKEND=d3d`: no adapters, the window doesn't open ("no graphics adapter can draw to the window", on stderr only), and `Gpu::headless()` is `None`, so `mg`'s pictures say there is no GPU.
- **Fix:** one helper in `backend.rs` (`None` when unset or naming no backend, and it says an unknown value), used by `gpu.rs`, `main.rs` and `graphics_setup`.
- **Test:** unit test of the helper over "", "d3d", "vulkan,nope".
- **Confidence:** confirmed by reading, sub-reader (wgpu-types 30.0.1 `Backends::from_env` gives `Some(empty)`; `gpu.rs:137-138`; `main.rs:313`; `options.rs:1232` treats empty as unset)

### R27. A panic in the lit check's second draw is put on Moonglow's own shader, and a panic the checks catch is written up as a crash

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-render/src/probe.rs:500-507, :98, :126, :454
- **Since:** after (3680a48f, 16127cf0)
- **Scenario:** the own draw passes, then the game's draw panics: `catch_unwind` sets `lit.own = Err("it panicked: …")` and leaves `lit.game` `None`, so the notice says the backend draws "the lit, textured test mesh in the wrong colors" though the own color is right, and the game's shaders stay on for the area view, where the same panic lands on the window's thread. And any panic the probe catches (a bad backend opening its device) has already made the hook write a crash report and "crashed: …" in the debug log builders are asked to send.
- **Fix:** keep which draw was under way and set `lit.game = Some(Err(…))` for the second; a scope in `guard` for the checks that the hook reads to name the report apart and skip `note_panic`.
- **Test:** `tests/render.rs` beside the lit check's tests: an `Assets` whose `game_shader` panics gives `lit.own.is_ok() && lit.game_fails()`.
- **Confidence:** confirmed by reading, sub-reader (`probe.rs:454-507`; `mg-ui/src/renderer_check.rs:172-209`; `guard.rs:54-63`; `main.rs:194-229`: with `guarding()` false the hook writes `crash-<time>.txt` and calls `recovery::note_panic()`)

### R28. Moonglow's own solid and sky pipelines write the picture's alpha, where the game's leave it

- **Kind:** bug (the two paths drifted)
- **Severity:** low
- **Where:** crates/mg-render/src/renderer.rs:1113-1117; crates/mg-render/src/shader.wgsl:493-499, :500, :543
- **Since:** before 2253fa78 for the pipelines; the game's mask is c22accb8 (after), which left the own shader's as it was
- **Scenario:** Shaders off (or a program pending or failed): a tileset layer drawn by the `vertexalpha` effect without `transparency`, its vertex alpha below 1, or a skybox whose texture has alpha: the target's alpha falls below 1 there and the panel shows through the picture (and `mgv render`'s PNG is see-through).
- **Fix:** `ColorWrites::COLOR` for the own `Sky` and `Opaque` pipelines, as `GamePass::write_mask`; no cut for the sky.
- **Test:** `tests/render.rs`: a `vertexalpha` card with vertex alpha 0.5 and no `transparency`, Shaders off: every pixel's alpha is 255.
- **Confidence:** suspected, mine. Read: `renderer.rs:1113-1117` gives every own pipeline `ColorWrites::ALL`; `game.rs:1092-1102` gives a pass that doesn't blend `COLOR`, its comment saying why (alpha below 1 written by a solid draw made the picture see-through: white lines in the views); `shader.wgsl:493-499`, `:543`, `:753` return `color.a` unchanged for a `vertexalpha` layer drawn solid (mask or vertex alpha, its MTR without `transparency`) and for the sky (a sky texture with alpha, also cut at 0.2 at `:500`, where the game's sky has `NO_DISCARD` 1, `renderer.rs:2513`). Every other solid draw of the own shader has alpha 1 (checked case by case: no alpha, punch-through set to 1 at `:524`, environment-mapped). Settled by a picture read back with its alpha.

### R29. The same pieces written two or three times, three of them drifted

- **Kind:** duplication
- **Severity:** low
- **Where:**
  - `Frame`: crates/mg-render/src/particle.wgsl:6-16 and line.wgsl:4-14 against shader.wgsl:15-33 and `FrameUniform` (renderer.rs:62-73). Drifted: nine fields where the others have ten (`time`); it works as a prefix, and a field added before `scene_color` would be misread silently.
  - The fast hasher: crates/mg-render/src/batch.rs:78-115 (`Mix`, whose `finish` folds the high bits down, with a comment saying why) and game.rs:1264-1299 (`Quick`, which doesn't). Drifted: `ReachCache::by_ball` (reach.rs:244) keys float bits with `Quick`, and tile-aligned balls land in a few buckets.
  - The device's descriptor: apps/moonglow/src/main.rs:672-683 (`MemoryHints::MemoryUsage`), crates/mg-render/src/probe.rs:99-104 (none, though its doc says "as the window opens one"), gpu.rs:61-66. Drifted: the startup check takes memory in other blocks than the window.
  - The game's tangents: crates/mg-render/src/model.rs:427-473 against crates/mg-mdl/src/compile.rs:104-120, :175-205; they disagree on a face flat in UV (suspected: `reading_tangents` with a card whose UVs are one point would say which the client matches).
  - The full-screen triangle's vertex program in shadow.rs:746-749, volume.rs:365-368, framebuffer.rs:86-89; the darkening blend in shadow.rs:1322-1333 and volume.rs:763-774; grow-then-write a buffer in shadow.rs:1165-1180, volume.rs:613-622 and renderer.rs:909-929; a panic's message taken out four ways (probe.rs:129-136, :501-505, guard.rs:67-71, mg-ui `jobs.rs:121`).
- **Since:** the `Frame` copies and `Mix` before 2253fa78; the rest after
- **Scenario:** none at run time today but the hasher's buckets and the check's memory blocks; each is a place where the next change is made in one copy.
- **Fix:** one `Frame` text prepended to the three shaders (source of truth: `FrameUniform`); one hasher (`Mix`'s `finish`); one function for the window's device descriptor; mg-mdl's tangents returned with their split and uploaded as they are; one `FULLSCREEN_VS`, one `DARKEN_BLEND`, the renderer's `upload` for the three buffers.
- **Test:** unit test that each shader's `Frame` (through naga) is as large as `size_of::<FrameUniform>()`; `made_tangents(m)` equals what `mg_mdl::compile` gives the same mesh.
- **Confidence:** confirmed by reading (sub-readers; I re-read the `Frame` copies in `particle.wgsl` and `shader.wgsl`, and `Quick` in `game.rs:1257-1299`)

### R30. Dead code, values always the same, and comments that say what the code no longer does

- **Kind:** hygiene
- **Severity:** low
- **Where:**
  - Never called: crates/mg-render/src/model.rs:526-528 (`local`); anim.rs:125-127 (`Binding::new`), :130-133 (`Binding::all`, a unit test only; real chains must use `all_scaled`); watch.rs:298-304 (`Watch::restart`, its own test only; its doc says the area view calls it), :476-479 (`Sampler::samples`); dx12.rs:50 (`is_dxc`, one test); crates/mg-ui/src/settings.rs:240 (`GraphicsFailure::when`, written and never read).
  - Always the same or never read: `FrameUniform::scene_color` is always zero (renderer.rs:2195), so shader.wgsl:729's `max(total, min(frame.scene_color.rgb * …))` does nothing; `light_params.z` (the light count, renderer.rs:2193) and `Draw.light_count.y` ("1 if skinned", renderer.rs:2748) are written and read by no shader; shadow.rs:263-267 computes each tile's `bottom` with Fast, which reads none.
  - Stale comments: renderer.rs:447-450 says `game_water` is "by default: Moonglow's own water", and `GAME_WATER` (:162) is `true`; renderer.rs:1539-1552: the doc of `capture_game_modules` sits on `wait_for_game_programs`, with a second doc under `#[doc(hidden)]`; particles.rs:54-55 says alpha-blended particles are "drawn back to front" (birth order since 03b0023a), and the module doc (:21-25, :43-44) still says `particlerot` turns and leaves `inheritvel` and `inherit_part` out of what is not simulated.
- **Since:** mostly after
- **Scenario:** none at run time; `Binding::all` invites the wrong scale, and the `scene_color` line reads as a feature that is not there.
- **Fix:** remove them, or use `when` for finding 6; correct the comments.
- **Test:** none (the compiler, once they are not `pub`).
- **Confidence:** confirmed by reading (mine for `renderer.rs` and the shader; sub-readers, by `rg` over the workspace and `~/Projects/moonglow-viewer`, for the rest)

## Left out of the thirty

(sub-readers', low): a backend that ends the process natively in the startup check is taken again at every start, nothing being written before the check (`apps/moonglow/src/main.rs:316-321`; suspected); in `renderer.rs`, bones of skinned meshes the game's programs draw are still uploaded to the own shader's bone buffer (`:2358-2365`, `:2966`), and a mesh whose pipeline is pending builds its program pair's key again each frame (`:1460-1477`).

## Not read

- By me: `shadow.rs`, `volume.rs`, `particles.rs`, `line.wgsl`, `pool.rs`, `model.rs`, `anim.rs`, `dangly.rs`, `batch.rs`, `reach.rs`, `scene.rs`, `assets.rs`, `texture.rs`, `watch.rs`, `probe.rs`, `backend.rs`, `dx12.rs`, `guard.rs`, `gpu.rs`, `lib.rs` beyond the ranges the entries above mark "re-read". Each was read whole by a sub-reader, who reported nothing left out.
- By anyone: the tests of `game.rs` (lines 3235-3719) and `crates/mg-render/tests/render.rs`, but for what the findings cite; `crates/mg-render/examples/snapshot.rs`.

## Looked at and found sound

- `renderer.rs`'s frame: the pass cuts (shadows laid, the framebuffer copied, the resolve kept for the last pass), the three ranges of the game's solid draws, the see-through draws' sort key (NaN and infinite depths, the hint's 29 bits) and the game's see-through draws put in drawing order before `upload` gives them their instances; a draw's uniform index in every branch (ghost, ordered, two parts).
- Stale indices and addresses used as keys: material places die with the surfaces that hold them (`let_go_of_stale_textures`, `clear_textures`, `SURFACES_KEPT`); `Arc<Slots>` addresses (`game_pipelines`, `valued`, the game's `templates` by `params.as_ptr()`) stay allocated until `clear_textures` clears every map keyed by them; `Copies::id` is unique, so a target let go is never mistaken for another.
- The texture readers and the shader worker: eras drop what was read or made before a clear, a dropped reader ends its threads, a dead worker is told and not waited on; the window waits for the readers to let go of the game data before it changes it (`release_game_data`).
- `game.rs`'s values: every write into a block is bounds-checked (`put`, `put_floats`, `put_array`), the lights are cut to 32 before `light_index` is filled, regions and bind groups are remade together when a buffer grows (`generation`), and a view shown in the frame is never let go of in it (`last_frame` is stamped before `let_go_of_unseen` runs). Layouts counted byte for byte by a sub-reader: `DarkenUniform`, `CasterData`, `VolumeVertex`, `VolumeCaster`, `VolumeFrame`, `ParticleVertex`, the line copies.
