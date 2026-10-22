---
title: "The shader compiler (M0): glslang-rs, the sampler split in Rust, Verify's shader-compile"
author: August
date: 2026-10-21
branch: claude/optimistic-booth-2195c7
cost: { minutes: 150, tokens: 400000 }
touches:
  - crates/mg-shader/src/lib.rs
  - crates/mg-shader/src/assemble.rs
  - crates/mg-shader/src/glsl.rs
  - crates/mg-shader/src/spirv.rs
  - crates/mg-shader/src/reflect.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-corpus-tests/tests/shaders.rs
  - crates/mg-testkit/src/lib.rs
  - packaging/third_party_licenses.py
  - docs/shaders-proposal.md
---

## Decided
- Crate: `glslang` 0.9 (SnowflakePowered/glslang-rs). glslang-sys vendors glslang's C++ and builds it with `cc` alone (no CMake/Python/network: the Flatpak's offline build takes it). Options VULKAN_RULES_RELAXED (= glslangValidator -R) | AUTO_MAP_BINDINGS | AUTO_MAP_LOCATIONS.
- No SPIRV-Tools: the combined-sampler split is a Rust pass on the SPIR-V words (mg-shader `spirv.rs`), naming halves `NAME_image`/`NAME_sampler` as spirv-opt does; the same pass gives inputs/outputs locations. Bindings renumbered in naga IR (group 0: blocks, then texture+sampler pairs, then the rest).
- `mg-shader` sits right after `mg-image` (PLAN §4) because Verify lives in `mg-module`, which comes before `mg-render`.
- Verify `shader-compile` (Models group): MTR-named programs, custom .shd programs, and stock programs when a custom file replaces a stock one (only if their assembly used a custom layer). First error per program; one finding per (file, line, message) listing programs.
- Corpus test `shaders.rs` (formats group); `MOONGLOW_HAKS` (mg_testkit::builder_hak_dirs) adds a builder's hak folders. Baseline 65/65 stock, 102/102 in ~/Projects/tdod-module/build/hak.
## Tried and ruled out
- shaderc-sys (build-from-source): CMake, and its API has neither -R nor SPIRV-Tools passes by name.
- spirv-tools-sys 0.13: compiles every opt pass but its C shim's `Passes` enum omits SplitCombinedImageSampler; a patched git dep is refused by packaging/flatpak/cargo_sources.py.
- glslang-rs `Program::map_io`: needs a linked program, but `link`/`compile` consume it, so glslang's I/O mapping never runs: no Location on inputs/outputs (naga spv-in fails "missing decoration Location"), every Binding 0.
## Watch out
- glslang reports `'' : compilation terminated` and a count after the first error; glsl.rs drops them. It also cascades (a second message from the first): Verify keeps the first.
- naga spv-in puts outputs in an unnamed result struct: output names come from the SPIR-V pass (`Io`), matched by location.
- glslang declares inputs in its own order, not the source's: tests must not assume locations by order. M1 sets Moonglow's vertex layout.
- `tdod.mod` as built lists no shader haks: to see the check on TDoD, attach them to a scratch copy (`mg attach --user-dir SCRATCH`).
- glslang-sys ships its license in native/glslang/LICENSE.txt: third_party_licenses.py now reads a crate's `native/*/` license files.
