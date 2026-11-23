---
type: Gotcha
title: Headless UI tests run with background_jobs false and prefer CPU-only area_ui_harness
description: "Headless editors must run work inline when background_jobs is false, and non-pixel area tests should use CPU-only area_ui_harness."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T23:34:33Z }
touches: ["crates/mg-ui/src/script_nav.rs:live_check", "crates/mg-ui/src/script_tools.rs:symbols_for_ui", crates/mg-ui/src/script_view.rs, "crates/mg-ui/tests/app/common.rs:area_ui_harness"]
asks:
  - Why do background threads in editors flake or hang headless mg-ui tests?
  - How should editor background compilation or symbol extraction handle background_jobs == false?
  - Which test harness should non-pixel area UI tests use to avoid wgpu device startup?
fingerprint: v2-4b93a74b8d1bdbc2
created: 2026-11-23
author: augustaugur
---
Headless UI tests construct `Moonglow` with `background_jobs = false`. Any background worker thread added to editors (`script_nav::live_check`, `script_tools::symbols_for_ui`, and similar helpers) must check `background_jobs` and execute inline when false; otherwise async completions schedule spurious repaints or race headless assertions. For area tests that assert UI or module state without inspecting 3D pixels or GPU resources, use `common::area_ui_harness` or `common::read_only_area_ui_harness` (CPU-only, no wgpu initialization) instead of the GPU area harness.
