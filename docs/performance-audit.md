---
type: Report
title: 'Audit: frames and GPU memory after 3.0.0, and on weaker hardware'
description: A dated record (2026-11-01, develop at 3.1.4) of an audit prompted by reports that performance fell and video memory rose since 3.0.0 - how frames and the GPU's memory were measured (v2.13.3, v3.0.0 and develop side by side; a discrete and an integrated GPU), what was found (no frame or memory regression since 2.13.3; views, textures and memory blocks held for good, an idle window drawing 25 pictures a second, a fill-bound picture on an integrated GPU), what was fixed with numbers before and after, and what is left with its cost.
tags: [performance, gpu, memory, renderer, area-view, audit]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-01T23:40:41Z }
sources:
  - id: owner
    resource: human:august
    title: 'The ask: reports that performance is down since 3.0.0 and VRAM usage high; a general audit for weaker hardware (2026-11-01)'
  - id: frame-perf
    resource: crates/mg-ui/tests/frame_perf.rs
    title: 'The frame test: frames_at_144 (Tyrants of the Moonsea), grass_frames_at_144, game_shaders_frames_at_144 (FRAME_AREA=tdod with the builder''s haks), with FRAME_MEMORY'
  - id: tdod-haks
    resource: ~/Projects/tdod-module/build/hak
    title: The Domains of Dread's haks, read-only (MOONGLOW_HAKS)
  - id: old-tags
    resource: target/old/
    title: v2.13.3 and v3.0.0 built from git archive, their frame test given the same memory report and device
  - id: wgpu-report
    resource: https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html#method.generate_allocator_report
    title: wgpu 30's allocator report (Vulkan, Direct3D 12)
---

# Audit: frames and GPU memory after 3.0.0, and on weaker hardware

The owner had reports, without much detail, that performance fell and the
GPU's memory rose since 3.0.0, and asked for an audit with weaker hardware
in mind.[^owner] In short: **nothing measurable got slower or larger
between 2.13.3 and 3.1.4**; what weighs is older and structural - every
area, model and picture of a session kept its GPU memory until the module
closed, wgpu held memory in 256 MB blocks, a visible area drew 25 pictures a
second even with the window in the background, and on an integrated GPU an
area's picture is bound by its pixels. Seven commits fix or relieve those;
what is left is at the end.

## How it was measured

- **Machine**: Ryzen 9 9950X, 60 GB, CachyOS (Linux 7.3), Vulkan through
  RADV. Two GPUs: the **Radeon RX 9070 XT** (16 GB, the owner's) and the
  CPU's own **integrated Radeon** (RAPHAEL, 2 compute units, 2 GB set
  aside), chosen with `MESA_VK_DEVICE_SELECT=1002:13c0
  MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE=1`. The integrated GPU stands
  in for weaker hardware; it is weaker than most laptops' (a Radeon 680M
  has 12 compute units).
- **Frames**: `frame_perf.rs`[^frame-perf] at 3440×1400, each view timed
  with the pointer moving, left alone and with the wheel: the CPU's time
  for a frame (interface and draws encoded) and the GPU's ("g"). The
  areas taken apart (`area_parts`) give a picture's GPU time alone, at
  3000×1250. Other agents were building on the machine (load 9 to 55):
  every comparison below was run back to back, most twice; a view whose
  times differ by less than about 15% between runs is "the same".
- **Memory**: wgpu's allocator report[^wgpu-report] after each view
  (`FRAME_MEMORY=1`, new): **allocated** (what lives) and **reserved** (the
  memory blocks held, what the card gives up), and by label
  (`FRAME_MEMORY=labels`): render targets, the meshes' chunks, the
  textures. `Renderer::texture_memory` (new) says what the textures take,
  uncompressed and stale.
- **Versions**: v2.13.3 (the last before 3.0.0), v3.0.0 and develop
  (3.1.4), the older two built from `git archive` with their own target
  folders,[^old-tags] their frame test given the same memory report and
  the same device (below).
- **Content**: Tyrants of the Moonsea (the campaign's areas 026, 49 tiles,
  and 092, 784 tiles; its galleries, editors and model viewer), the frame
  test's grass field, and The Domains of Dread's woods (576 tiles, 400
  objects) with the builder's haks.[^tdod-haks]

**The frame test's device had no BC textures.** It asked for no features,
so every BC texture was uploaded as RGBA8, four to eight times its size:
TDoD's woods took **4.4 GB** of textures in the test where the app, which
asks for BC, takes **870 MB**. Large areas were timed in a card that was
full (the framebuffer learning's "16.8 of 17.1 GB" was this). The test's
device now asks for what the app's does, and the app's descriptor is a
function with a test (`the_device_asks_for_bc_textures_and_small_blocks`).
Every number below is with BC textures, but the row that says otherwise.

## Versions side by side

| | v2.13.3 | v3.0.0 | develop |
|---|---:|---:|---:|
| TotM memory after area092 opens (allocated / reserved, MiB) | 607 / 1024 | 602 / 1024 | 603 / 1024 |
| TotM, area026 close, left alone (CPU ms, GPU ms; two runs) | 1.9–4.2, 1.8–1.9 | 1.5, 1.9 | 1.6–2.2, 1.8 |
| TotM, area092 turning, left alone | 1.9–2.0, 2.5–2.6 | 1.8–2.3, 2.5–2.6 | 1.7–1.9, 2.6 |
| TotM, area092's opening, until drawn whole (ms) | 112–119 | 164–168 | 165–221 |
| Integrated GPU, area026 picture: overview / close (GPU ms) | 25.9 / 78.2 | 25.4 / 78.0 | 30.3 / 79.5 |
| Integrated GPU, area092 picture: overview / close | 28.0 / 52.8 | 28.8 / 55.3 | 28.0 / 57.7 |
| Grass field across / orbiting, grass on (GPU ms)* | 7.9 / 6.0 | 8.7 / 6.7 | 8.7 / 6.8 |
| TDoD woods (uncompressed, before the device fix): textures | 4438 MiB | – | 4438 MiB |

(*The grass rows were run while something else used the RX 9070 XT:
the same field without grass took 2.0 ms where it took 0.5 an hour
before, alike for the three versions. Their ratios hold; their times
don't.) The integrated GPU's rows were run again with BC textures, and
before the device fix without them: the three versions alike either
way.

The views of editors, palettes, galleries and menus took the same within
0.05 ms (0.15 to 0.30 ms of CPU a frame), the stock and TDoD areas with the
Shaders switch off and on the same within run-to-run spread. What did grow
since 2.13.3: **an area's opening** to its last texture (area092 about 50
ms longer: not taken apart) and **grass** (10 to 20% more GPU where it
fills the view: it sways since 3.0.0). Neither is what a builder would call
"performance down". If the reports are of a session that slows and fills
the card, the causes below are the likelier ones; they were there in 2.13.3
too.

## What was found

1. **Views kept their GPU memory for good.** Closing an area's tab kept
   its view (`Moonglow::area_views` is emptied only when the module
   closes): its targets (at 3440×1400, 4× MSAA colour and depth, the
   picture and the framebuffer copies: about 170 MB) and its scene (every
   tile's and object's meshes). A tab behind another kept its targets too,
   and so did each model viewer and blueprint preview. egui's binding of a
   target (`register_native_texture`) held it on the GPU even after a view
   let go of it: the area views never freed theirs. Tyrants of the
   Moonsea's frame test ended with **six** sets of targets alive (about
   500 MB).
2. **Textures were never let go of.** `Renderer::textures` keeps every
   texture met until the resources change: every area opened, every
   Gallery picture's and every model viewed. In the test, the palettes'
   searches and the Galleries alone left 444 textures behind.
3. **wgpu held memory in blocks of 128 to 256 MB** (its default memory
   hints): a block stays while anything is in it. After the frame test
   1024 MB were held for 611 used; after the first fixes, 768 for 267.
4. **An area's view drew 25 pictures a second as long as it was on
   screen**, the window behind another program's or not (its animations
   step every 40 ms, and tiles always animate). On the RX 9070 XT that is
   2 to 3 ms of GPU each, 6% of it; on the integrated GPU 25 to 80 ms each:
   **the GPU never idle**, and every other frame of the window waiting
   behind them.
5. **On an integrated GPU the picture is bound by its pixels.** A quarter
   of the pixels took a third to two fifths of the time (area026 close up
   79.5 ms at 3000×1250, 27.4 at 1500×625; seen whole 25.9 and 10.2).
   Shadows off changed nothing; MSAA off saved about 10% (25.9 to 23.6
   ms).
6. **A builder's textures are large.** TDoD's woods use 965 textures, 791
   of them 1024 pixels (BC3, BC4, BC5): **868 MiB**, and the woods' view
   1456 MiB in all (meshes about 370 MB of chunks). On a card of 2 to 4 GB
   that is most of it before anything else.

## What was fixed

| Commit | What | Before | After |
|---|---|---|---|
| `mg-render: textures not drawn lately let go of…` | A texture not drawn for 30 s of frames (a frame counts a second at most: a window left alone makes nothing stale) is stale; past 1 GiB of stale ones (256 MiB on an integrated GPU) the longest unused go, with the bind groups and surfaces made of them. What a frame draws is never let go of. | every texture kept | bounded; `textures_not_drawn_lately_are_let_go_of_past_what_is_kept`, `the_clock_is_the_frames_time` |
| `3D views not drawn for two seconds let go of their targets, a closed area of its scene` | Area and model views not drawn for 2 s let go of their targets and egui's binding; an area whose tab is closed lets go of its scene (read again, where its camera was, when reopened), as Reload Resources does. | TotM test ends at 611 MiB allocated, 1024 reserved | **267** allocated, 768 reserved; `a_closed_area_lets_go_of_its_targets_and_scene`, `an_area_behind_another_tab_lets_go_of_its_targets_only` |
| `frame_perf: the GPU's memory after each view…` | The memory report, and the test's device with BC textures as the app's. | TDoD woods 4.4 GB of textures in the test | 868 MiB, as the app |
| `The 3D views hold still behind another program's window…` | Without the window's focus, an area view whose picture is whole asks for no animation step (the model viewer likewise); it goes on as the focus comes back. **Changes what is shown**: animations pause in the background. | 25 pictures a second in the background (integrated GPU: busy all the time) | none; `an_area_s_animations_wait_while_the_window_has_not_the_focus` |
| `The window's device takes the GPU's memory in blocks of 8 to 64 MB…` | `MemoryHints::MemoryUsage` in the app's device. Frames and openings as long. | TotM 768 MiB held; TDoD woods 1856 | **524**; **1552** |
| `Options › Area › Largest textures…` | 1024, 512 or 256 pixels: larger textures uploaded without their larger mip levels (a BC one from its own smaller levels; one without them as it is). Default: as they are. **Changes what is shown** when chosen. | TDoD woods textures 868 MiB, the view 1456 allocated / 1552 reserved | at 512: **244** MiB, **832** / 912; `a_texture_limit_leaves_out_the_levels_over_it`, `the_largest_textures_option_reads_the_textures_again_smaller` |
| `Options › Area › Area view resolution…` | 75% or 50% of the pixels each way, the picture stretched to the view, overlays sharp (seen in a picture of area092 at 50%: the selection and Sound Ranges' rings on their boats). Default: Full. **Changes what is shown** when chosen. | integrated GPU, area092 turning 80 ms a picture, close 73; its memory 460 MiB held | 75%: 45, 41, 284; **50%: 20, 19, 188**; `the_area_view_resolution_draws_the_picture_at_a_share_of_its_pixels` |

On a strong machine nothing looks or runs otherwise by default, but the
pause in the background. On an integrated GPU, Largest textures at 512 and
Area view resolution at 50% together take TDoD's woods from 1.5 GB and 75
ms a picture turning to about 0.8 GB and 20 ms; the manual's
[Options](manual/10-options.md) and [Areas](manual/04-areas.md) chapters
say what each costs.

## What is left, with its cost

- **The renderer changes need the owner's CI on a branch** (ubuntu,
  windows, macos) before develop: `mg-render`'s `texture.rs`, `gpu.rs`,
  `renderer.rs`, `renderer/kept.rs` and `game.rs` changed. Nothing was
  pushed.
- **Not tried on real weaker hardware**: Windows (Direct3D 12's heaps
  under `MemoryUsage`), an Intel integrated GPU, macOS. The integrated
  Radeon here is the only stand-in; it waits on a builder's machine.
- **The meshes' chunks** (`pool.rs`) grow to the size of all before: TDoD's
  woods hold 184 + 92 + 46 + … MB of chunks, about 370, for meshes not
  measured; up to half of it may be room. A cap on a chunk's growth, or
  chunks sized to what is read, would give some back (S, renderer, CI).
- **A setting for MSAA**: 4× is fixed (`model_view::SAMPLES`). Off saves
  about 10% of an integrated GPU's picture and 107 MB of targets a view at
  3440×1400 (27 MB at 50% resolution), and needs the renderer made anew on
  a change (M). Area view resolution does more for less; left.
- **Uncompressed textures**: TGA and PLT are RGBA8 (Tyrants of the
  Moonsea's 70 of 96 MB). Compressing them on the CPU as they are read
  would take a quarter to an eighth of that, at a cost in reading time
  and a new encoder (M).
- **The animation step could follow what a picture costs** (a slow GPU
  stepping less often while the window is in front), measured with the
  GPU's timestamps (M; not on every backend).
- **An area's opening** to its last texture is about 50 ms longer since
  3.0.0 (area092: 112 to 165 ms); not taken apart.
- **The model viewer playing an animation** asks for a frame at once,
  every frame, while the window has the focus (as the display refreshes:
  144 a second here); a step as the area view's would do (S).
- **Thumbnails** keep up to 512 out of sight, 196 KB each (about 100 MB at
  most); bounded already.

[^owner]: The owner, 2026-11-01.
[^frame-perf]: `crates/mg-ui/tests/frame_perf.rs`, run with `--ignored --nocapture --test-threads=1`, under a 12 GB memory cap with `MG_TEST_GPUS=1`.
[^wgpu-report]: `wgpu::Device::generate_allocator_report`, wgpu 30.0.1.
[^old-tags]: `target/old/v2.13.3`, `target/old/v3.0.0`.
[^tdod-haks]: `MOONGLOW_HAKS=~/Projects/tdod-module/build/hak`, `FRAME_AREA=tdod`.
