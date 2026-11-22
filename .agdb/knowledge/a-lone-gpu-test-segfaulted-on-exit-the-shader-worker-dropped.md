---
type: Gotcha
title: "A lone GPU test segfaulted on exit: GPU threads dropped their device as the process ended (debug builds, validation layer)"
description: "Why lone GPU tests crashed after passing in debug builds, why the shader worker and the lit check now join their threads on drop, and why the texture and picture readers do not"
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T10:55:45Z }
touches: ["crates/mg-render/src/game.rs:WorkerLink", "crates/mg-render/src/game.rs:Worker::run", "crates/mg-ui/src/renderer_check.rs:LitCheck"]
asks:
  - "Why does a test binary segfault after the test passes, at exit?"
  - Can Moonglow crash on quit because of a GPU thread?
  - Why do WorkerLink and LitCheck join their threads when dropped?
  - Why did the crash show only with the Vulkan validation layer?
fingerprint: v2-3ce9134a5d0e5f3b
created: 2026-11-22
author: augustaugur
---
What is shown (M5 S1 and its review, 2026-11-22): a crash at process exit in debug builds with Vulkan's validation layer, which wgpu turns on wherever debug assertions are (release builds do not load it). No crash was seen with the layers off (`VK_LOADER_LAYERS_DISABLE='~all~'`). Two cases, each a detached thread holding a `Gpu` clone and still working inside libVkLayer_khronos_validation while main ran libc `exit()` and the layer's destructors:
- `model_view::tests::a_picture_s_water_is_drawn_as_the_area_view_draws_it`, run alone: thread "moonglow shaders" (`mg_render::game::Worker::run`) dropping its device; SIGSEGV 18 runs in 20, 0 in 12 with layers off.
- `drawing::reload_resources_reads_an_area_again_on_a_thread`, run alone: thread "moonglow lit check" (`renderer_check::lit_check`, `probe_lit`) inside `create_render_pipeline`; 9 in 10, 0 in 10 with layers off.

Can Moonglow crash on quit because of a GPU thread? In a debug build with the layer, it could; both of those threads are now joined at drop. `WorkerLink`'s Drop sets a stop flag, drops the job sender and done receiver, and joins (the worker finishes the program it is making, takes no queued job, and leaves the `guard::hold_programs` test loop when told). `LitCheck` keeps its JoinHandle and joins at drop (one `probe_lit` at most, at quit). After: 0 in 20 for each lone test.

Not joined, by the owner's ruling (filed in docs/deferred.md): the texture readers ("moonglow textures", `mg_render::renderer`) and the picture readers ("moonglow pictures", `model_view`) hold `Gpu` clones and end detached the same way, but are replaced mid-session (the texture reader whenever the game data changes), where a join would hold the window. Picture readers: 0 crashes in 10.

Test: `game::tests::the_shader_worker_has_let_the_gpu_go_when_its_renderer_goes` drops the renderer's shaders on a helper thread and waits 5 s with `recv_timeout`, then checks the waker Arc's count is 1. It needs the 100 ms pause before the drop, or a just-started worker ends within the drop by itself and the test passes with the join removed; with the stop check removed from the hold loop it fails in about 5 s.
