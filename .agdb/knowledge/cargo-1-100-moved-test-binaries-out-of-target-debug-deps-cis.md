---
type: Gotcha
title: "Cargo 1.100 moved test binaries out of target/debug/deps: CI's DXC was beside none of them"
description: "Why CI's Windows render tests ran on FXC after stable became Rust 1.100 (test executables now in target/<profile>/build/<crate>/<hash>/out), and how the CI step now finds every test binary's folder."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T21:29:38Z }
touches: [.github/workflows/ci.yml, packaging/windows/dxc.ps1, crates/mg-render/src/dx12.rs]
asks:
  - Why did the Windows shadow and DXC render tests fail in CI on 2026-11-12 with no code change in mg-render?
  - "Where does cargo 1.100 put test executables, and why is dxcompiler.dll in target\\debug\\deps no longer found?"
  - "How does CI put dxcompiler.dll beside every test binary whatever Cargo's layout?"
  - Do the shadow tests pass on WARP when Direct3D 12 compiles with FXC?
fingerprint: v2-b15d067df7b5fe53
created: 2026-11-13
---
## What happened
- Run 43297784994 (develop 9e1050f7) was the first on Rust 1.100.0 (CI tracks `stable`). Cargo 1.100 builds each test executable into its own folder, `target/<profile>/build/<crate>/<hash>/out/` (the `Running` lines show it), no longer `target/<profile>/deps/`. CI's Windows job copied `dxcompiler.dll` into `target\debug\deps`, so `mg_render::dx12::choose` found none beside `render-*.exe` and Direct3D 12 compiled with FXC.
- Under FXC on WARP five shadow tests failed (Fast shadows drew no darkening at all; High's stencil volumes darkened 177 where 202 is right) and `a_texture_read_in_a_loop_of_a_varying_count_compiles_and_draws` failed on `MOONGLOW_REQUIRE_DXC`. With DXC (fa03ef5f, 1.99.1, the DLL in deps) all of render.rs passed.

## Decided
- CI builds the tests first (`cargo test ... --no-run --timings`), then on Windows lists every test executable from `cargo test ... --no-run --message-format=json` (`executable` of each artifact) and gives `dxc.ps1 -Destination` their folders, then runs the tests. Independent of Cargo's layout; `target\debug` (mg.exe, which CLI tests run) gets the DLL too.
- `dx12::choose` was not changed: "beside the program" is what ships, and the CI step is what proves it.

## Watch out
- Open: the shadow tests passed under FXC on WARP on 2026-10-28 (run 40776698995, before DXC shipped) and fail under FXC now. Real FXC (`MOONGLOW_D3DCOMPILER`, lutris's d3dcompiler_47) compiles all 43 of Moonglow's own stages, the shadow ones included, so it is how FXC's code runs on WARP, not a refused pipeline. Settle it on Windows with `WGPU_DX12_COMPILER=fxc cargo test -p mg-render --test render shadow`.
- Without `--no-fail-fast`, CI's test job stops at the first failing binary: on the 1.100 Windows run nothing after `render` ran.
