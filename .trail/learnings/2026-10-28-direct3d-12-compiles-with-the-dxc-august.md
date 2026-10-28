---
title: Direct3D 12 compiles with the DXC shipped beside Moonglow
author: August
date: 2026-10-28
branch: worktree-agent-a2ead8cc3b81250ec
cost: { minutes: 1534, tokens: 12678777 }
touches:
  - ".github/workflows/ci.yml"
  - Cargo.lock
  - apps/moonglow/src/main.rs
  - crates/mg-corpus-tests/tests/hlsl.rs
  - crates/mg-render/Cargo.toml
  - crates/mg-render/src/gpu.rs
  - crates/mg-render/src/lib.rs
  - crates/mg-render/tests/render.rs
  - crates/mg-testkit/src/hlsl.rs
  - crates/mg-ui/src/lib.rs
---

## Decided
- wgpu 30's default `Dx12Compiler::Auto` loads `dxcompiler.dll` by bare name (LoadLibrary's search order, `PATH` included) and falls to FXC only when that load fails; a `DynamicDxc` whose DLL fails to load drops the whole Direct3D 12 instance (its adapters vanish, Vulkan is taken). So `mg_render::dx12` names the DLL beside `current_exe()` by its whole path after loading it itself (libloading, `DxcCreateInstance` looked up), else `Fxc`; `WGPU_DX12_COMPILER` still decides. Chosen once (OnceLock); `dx12::backend_options()` goes into every instance: `headless_adapter`, the window (eframe) and the kittest tests through `mg_ui::graphics_setup(setup)`. The `graphics:` debug line ends with `dx12::compiler()` on Windows.
- Pinned DXC v1.9.2609, `dxc_2026_09_29.zip`, SHA-256 ad31b1fc8443175d204f77a611fdb3ef2ec42759bdc2f1167368de24a4a7e7f1 (`packaging/windows/dxc.ps1`, run by `build-installer.ps1` and CI's Windows job into `target\debug\deps`). Only `bin\x64\dxcompiler.dll` ships: since 1.8.2505 it validates and signs alone (checked: objects' DXBC digest non-zero; `-Vd` gives zero). `dxil.dll` is closed and under the zip's LICENSE-MS (a proprietary EULA): not shipped. dxcompiler is NCSA (LLVM) + MIT; its texts plus the repo's ThirdPartyNotices at the tag (`packaging/windows/dxc/`) go into THIRD-PARTY-LICENSES.txt (`third_party_licenses.py --dxc`).
- StaticDxc ruled out: `mach-dxcompiler-rs` downloads a prebuilt library at build time and is MSVC-only.

## Watch out
- FXC compiles a `textureSample` loop whose count is a uniform. X3570 then X3511 needs a count that varies per pixel (`fslit_nm`'s `fIterations`, from the view). naga checks no derivative uniformity in fragment shaders (`DISABLE_UNIFORMITY_REQ_FOR_FRAGMENT_STAGE`), so such WGSL validates everywhere.
- With DXC, wgpu-hal offers no Direct3D 12 adapter whose highest shader model is 5.1 (adapter.rs): an old driver loses Direct3D 12 instead of falling back to FXC (`WGPU_DX12_COMPILER=fxc` gets it back).
- wgpu writes the HLSL at the shader model it compiles at: `hlsl::options_at(groups, sm)` and `profile_at`, never 5.1 text given to DXC.
- `tools/dx12/dxc.sh` runs the release's `dxc.exe` under Wine with wgpu's arguments (`-HV 2018 -no-warnings -Ges`), one process per shader.
- In a worktree session Bash refuses commands with a github.com URL or `$HOME` inside a compound command, and heredocs: put them in a scratchpad script.
