---
type: Gotcha
title: mg-ui test binaries set MOONGLOW_CACHE_DIR before main (tests/support/own_cache.rs)
description: "How mg-ui's UI tests keep Moonglow's cache (NWSync cache, test-user, build scratch) out of ~/.cache/moonglow, and why it is a pre-main link_section constructor."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T07:29:58Z }
touches: [crates/mg-ui/tests/support/own_cache.rs, "crates/mg-module/src/build.rs:cache_dir"]
asks:
  - why do mg-ui tests not write ~/.cache/moonglow
  - where is MOONGLOW_CACHE_DIR set for the UI tests
  - how do I add the test cache folder to a new mg-ui test binary
fingerprint: v2-fe1c2c4c0d1d5fbf
created: 2026-11-27
author: augustaugur
---
mg_module::build::cache_dir reads MOONGLOW_CACHE_DIR at every call; the NWSync cache (nwsync_content::cache_of without [nwsync] cache), Test Module's test-user and build scratch all go through it. mg-ui's tests make Moonglow (about 300 Moonglow::new) and harnesses (about 220 builders) on parallel threads, and set_var is unsafe in edition 2024 while other threads read the environment, so there is no safe shared init: tests/support/own_cache.rs puts set() in .init_array / __DATA,__mod_init_func / .CRT$XCU (#[used] static, #[allow(unsafe_code)] against the workspace's deny) and sets test_output()/moonglow-cache (not scratch_dir: not emptied; per MOONGLOW_TEST_OUTPUT). Included with #[path] by tests/app/main.rs, screens.rs and nui.rs; a new binary that makes Moonglow should include it. Checked by the_tests_keep_moonglow_s_cache_in_their_own_folder (app) and the assert in screens' nwsync_content_window; the macOS section is not built on the Linux machine (no Apple C toolchain), CI checks it.
