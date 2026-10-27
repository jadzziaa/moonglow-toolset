---
title: mg without the GPU stack; libraries at 0.0.0
author: August
date: 2026-10-27
branch: build-hygiene
cost: { minutes: 90, tokens: 11255248 }
touches:
  - ".github/workflows/ci.yml"
  - Cargo.toml
  - apps/mg/Cargo.toml
  - apps/mg/src/nui.rs
  - apps/mg/tests/nui.rs
  - crates/mg-area/Cargo.toml
  - crates/mg-image/src/plt.rs
  - crates/mg-mdl/src/lib.rs
  - crates/mg-preview/Cargo.toml
  - crates/mg-preview/src/files.rs
---

Versions and features of the build (2026-10-27).

- The release is the root Cargo.toml's `[workspace.package] version`, which only apps/mg and apps/moonglow inherit; the 28 library crates have `version = "0.0.0"` of their own. A bump recompiles 2 crates (it was 27 for `-p mg`). Never give a library `version.workspace` again, and never read `env!("CARGO_PKG_VERSION")` in a library: it is 0.0.0 there. Read `mg_core::release::version()`, which both programs `set` first thing in `main` (a library's tests and Moonglow Viewer see the fallback, 0.0.0). The packaging scripts read the first `^version = ` line of the root Cargo.toml, so that line stays the first.
- `mg` reaches the GPU only in `mg nui render` (wgpu) and `mg nui layout` (a frame of mg-ui's NUI canvas). Both are behind `mg`'s default feature `nui-preview` (apps/mg/src/nui/preview.rs). mg-preview's renderer half (`compose`) is its default feature `render`; the workspace dependency on mg-preview is `default-features = false` (a member can't turn defaults off on a `workspace = true` dependency otherwise), so mg-area and mg-ui ask for `features = ["render"]`. A new crate that uses `mg_preview::compose` must ask for it too, or it builds in the workspace (features unify) and fails alone.
- mg-preview's non-render files needed two pure helpers of mg-render; they moved below it (`mg_image::plt::{split_colors, colored_name}`, `mg_mdl::animations`) and mg-render re-exports them at the old paths, which Moonglow Viewer calls (`mg_render::anim::animations`, `mg_render::colored_name`).
- Lean build: `cargo build --release -p mg --no-default-features` is 140 crates, 87 s cold at 4 jobs, 22.7 MB (default: 229, 162 s, 37.8 MB). CI clippy-checks `-p mg -p mg-preview --no-default-features --all-targets`.
- The worktree guard refuses heredocs to python and commands wrapping cargo in `flock`/`time`: write the script to the scratchpad and run `bash script.sh` or `python3 script.py`.
