---
type: Research Note
title: 'Moonglow Toolset: prior-art brief'
description: Survey of 2026-09-30 of existing NWN tools and libraries (rollnw, neverwinter.nim, nwn-lib-rs, NWNExplorer and others), focused on terrain painting, EE lighting and libraries Moonglow could depend on, with recommendations.
tags: [prior-art, survey, libraries]
generated: { by: claude-code/claude-opus-5-5, at: 2026-09-30T13:38:36Z }
---

# Moonglow Toolset: prior-art brief

Surveyed 2026-09-30. Repository metadata (license, last commit, releases) comes from the GitHub, GitLab
and crates.io APIs on that date. I shallow-cloned and read the code of rollnw, neverwinter.nim,
nwn-lib-rs and NWNExplorer, and read the SWLOR.Toolset, AuroraBorealius, Radoub, nwnrs and PyKotor
code through the API. For NWN facts I used the local nwn.wiki mirror, and I inspected the game install
(`nwtoolset.exe`, `ttr01.set`) directly.

---

## 1. Summary of findings

1. **Beamdog will not rewrite or port the toolset.** Beamdog staff said in 2021: *"It's not in the
   works"* (JuliusBorisov) and *"Rewriting the Toolset from scratch would be an enormous amount of
   work … it's not going to happen"* (virusman). The shipped `nwtoolset.exe` (Oct 2025 build) is still
   a 32-bit PE built with Embarcadero RAD Studio / C++Builder (VCL). Since 8193.36, EE has been
   maintained by unpaid community engineers (niv, virusman, Soren, Daz, clippy, and others). They keep
   making small quality-of-life changes to the toolset, for example 8-class creature editing and
   script sets in 8193.37. Nobody is going to remove our reason to exist.
2. **The field changed a lot in 2026.** Four new projects try to replace some or all of the toolset:
   - **rollnw | toolset** (C++/Vulkan, MIT). The most serious one: it has area editing, tile painting,
     blueprint editors and a play preview.
   - **SWLOR.Toolset** (C#/Avalonia/OpenGL, MIT). Windows-only and specific to the SWLOR server, but
     it has a painter validated against Aurora.
   - **AuroraBorealius** (Rust/egui/wgpu, LGPL). A prototype that was dumped once and then abandoned.
   - **Radoub** (C#/Avalonia, GPL-3). Standalone editors for individual file types, with no area
     editor.

   Several are visibly AI-assisted (AGENTS.md or CLAUDE.md files, "Codex" provenance notes).
3. **(a) Tile painting:** two real implementations exist, in rollnw and SWLOR.Toolset. Both solve
   corner terrain, heights and edge crossers by exhaustive search over tile × orientation. **Neither
   implements the SET `[PRIMARY RULES]` propagation (Placed/Adjacent → Changed)**, which base-game
   tilesets rely on (`ttr01.set` has 28 rules). Faithful Aurora terrain matching remains an open
   problem.
4. **(b) EE lighting:** **no open renderer reproduces EE lighting.** rollnw's Forward+ PBR renderer
   deliberately *"does not reproduce NWN fixed-function lighting."* The others use basic Phong or
   three.js approximations. Even Beamdog's own toolset preview differs from in-game lighting (the wiki
   has side-by-side screenshots). The shortest route to parity is unused: EE's stock GLSL `.shd`
   shaders ship as plain text in the game data, and the wiki documents their uniforms and defines.
5. **(c) Libraries we could depend on:**
   - C++: **rollnw** (MIT, broad and well tested, but "lives at HEAD" with no API stability).
   - Rust: **nwnrs** (the broadest, including MDL/MTR/SET and a pure-Rust NWScript compiler, but
     **GPL-3.0-only** and pre-release on nightly Rust) and **nwn-lib-rs** (LGPL-3, GFF/ERF/TLK/2DA
     only, with no KEY/BIF).
   - **The official script compiler is GPL-3.0.** It is exposed through a C ABI
     (`libnwnscriptcomp`) by neverwinter.nim.

---

## 2. Landscape at a glance

Verdicts:
- **Dep**: usable as a library dependency.
- **Ref**: read it and learn from it, don't link it.
- **Oracle**: use it as a test oracle.
- **GPL**: linking would force Moonglow to be GPL-3.

| Project | Stack | License | Activity (last commit / release) | Scope relevant to us | Verdict |
|---|---|---|---|---|---|
| [rollnw](https://github.com/jd28/rollnw) + `rollnw-client` | C++20, Vulkan (`nw::gfx`), SDL + RmlUi, Smalls scripting | MIT | 2026-09-28, snapshot `2026.09.28` | Nearly all NWN formats, resman, objects, NWScript parser, MDL/particles/Forward+ renderer; toolset with area/tile editing, blueprints, dialog view, F9 walk preview | **Dep (C++)** / Ref / Oracle |
| [arclight](https://github.com/jd28-archive/arclight) | C++, Qt, DiligentEngine | MIT | Archived ("moved to rollnw client") | erfherder, texview, alpha DLG editor | Ref |
| [SWLOR.Toolset](https://github.com/zunath/SWLOR_NWN/tree/master/SWLOR.Toolset) | C#/.NET 10, Avalonia, Dock, AvaloniaEdit, Silk.NET OpenGL | MIT | Started 2026-07-19; last 2026-09-26 | Area, instance, blueprint, **tile painting**, script editor; 3,400+ tests incl. corpus gates; Windows-only (WinExe), SWLOR-scoped | **Ref** (painter, tests) |
| [AuroraBorealius](https://github.com/Evangelion1337/AuroraBorealius-Toolset-and-Shargast-by-Jaysn) ([Vault](https://neverwintervault.org/project/nwn1/other/tool/auroraborealius-toolset)) | Rust, eframe/egui, wgpu 24 | LGPL-3.0+ (Cargo.toml only, no LICENSE file) | Vault v0.42 2026-05-28; commits 05-31…06-02, then nothing | Viewport, DLG/JRL/FAC/2DA editors, script editor (bundles nwnsc), NWSync; ~35 GB RAM reported; 8k-line `lib.rs`; no SET-rule painting | Ref (cautionary) |
| [Radoub](https://github.com/LordOfMyatar/Radoub) | C#/.NET 9, Avalonia, Silk.NET OpenGL | GPL-3.0 | v0.11.0 2026-06-08; last 2026-07-21 | Parley (DLG, with flowchart and simulator), Manifest (JRL), Quartermaster (UTC/BIC), Fence (UTM), Relique (UTI), Reliquary (UTP), Trebuchet hub; no areas | GPL / Ref (DLG UX) |
| [Eos Toolset](https://github.com/Cjreek/Eos-Toolset) | C#, Avalonia 11 | MIT | 0.79c 2026-09-04 | 2DA/TLK/SSF custom-content editor (classes, feats, spells…), JSON projects | Ref (2DA schemas) |
| [neverwinter.nim](https://github.com/niv/neverwinter.nim) | Nim + C++ (compiler) | MIT, **compiler GPL-3.0** | 2.3.1 2026-08-25 | Resman/KEY/BIF/ERF/GFF(+JSON)/2DA/TLK/SSF/NWSync CLI; official compiler as CLI and `libnwnscriptcomp` | **Dep (compiler)** / **Oracle** |
| [nwn.py](https://github.com/niv/nwn.py) | Python | MIT (bundles GPL compiler .so) | 2026-09-10, alpha | GFF/ERF/KEY/2DA/TLK/SSF, SET parser (incl. rules), compiler via ctypes, NCS VM | Ref / Oracle scripts |
| [nwnsc](https://github.com/nwneetools/nwnsc) | C++ | MIT (Skywing) | v1.1.5 2023-03 | Legacy community compiler, superseded by the official one | Ref only |
| [xoreos](https://github.com/xoreos/xoreos) / [xoreos-tools](https://github.com/xoreos/xoreos-tools) | C++, legacy OpenGL | GPL-3.0+ | 2026-09-02 / 2026-08-28; last release v0.0.6 (2020) | Aurora-family engine: NWN areas, tilesets, walkmesh, MDL; tools gff2xml, ncsdis/ncsdecomp, erf, keybif; no EE MTR/PBR | GPL / Ref |
| [NWNExplorer](https://github.com/virusman/nwnexplorer) | C++ WTL (Windows), OpenGL | BSD-style (Edward T. Smith / OpenKnights) | 1.8.5 2026-04-07 | Resource browser and model viewer; `_NwnLib` binary MDL/PLT/texture code; nwnmdlcomp | **Ref** (binary MDL) |
| [Neverblender](https://github.com/gyoerkaa/mdltools) (local port in `~/Projects/neverblender`) | Python (Blender add-on) | GPL-3.0+ | Upstream 2021–23; Vault 4.0; local 4.2 for Blender 4.4–5.2 | ASCII MDL import/export, MTR, SET import helpers | GPL / Ref / pipeline |
| [Borealis MDL / viewer](https://github.com/varenx/borealis_nwn_mdl) | C++23, Qt6/OpenGL | GPL-3.0 | 2026-04 | Binary+ASCII MDL, decompile, animation, particles | GPL / Ref |
| [nwn_mdl_webviewer](https://github.com/dunahan/nwn_mdl_webviewer) | JS, three.js (+ CleanModels WASM) | MIT | 2026-09-25 | MDL viewer, MTR status, SET browser; PBR via MeshStandardMaterial (not EE parity) | Ref |
| [nasher](https://github.com/squattingmonk/nasher) | Nim | MIT | 2026-08 | Module-as-source-tree build tool (JSON/NWNT) | **Interop target** |
| [nwnrs](https://github.com/urothis/nwnrs) | Rust (nightly) | **GPL-3.0-only** | Created 2026-04; pushed 2026-09-16; crates 0.0.1 | Resman/install discovery, GFF, ERF, KEY, 2DA, TLK, SSF, DDS/TGA/PLT/TXI, **MDL (bin↔ASCII), MTR, SET**, NWSync; pure-Rust NWScript compiler (NCS/NDB/asm/VM) | **Dep if GPL**, else Ref |
| [nwn-lib-rs](https://gitlab.com/CromFr/nwn-lib-rs) | Rust (nom, serde) | LGPL-3.0+ | 2026-07-22; crates 0.4.0 (2025-02) | GFF (JSON/YAML), ERF, TLK, 2DA (+NWN2 TRN/MDB); **no KEY/BIF** | Dep (LGPL) / Ref |
| [serde-gff](https://github.com/Mingun/serde-gff), [gff-rs](https://github.com/Youx/gff-rs) | Rust | MIT / LGPL-2.1 | 2020 / 2022 (stale) | GFF only | Ref |
| [Aurora-Hak/TLK-Explorer](https://github.com/winternite/Aurora-Hak-Explorer) | Rust, egui | GPL-3.0+ | 2026-09 | HAK/ERF/MOD/SAV; TLK/2DA/ITP editors | GPL / Ref |
| [nwscript-lsp](https://github.com/cgtudor/nwscript-lsp) | Rust, tower-lsp | none declared | 2026-06 | NWScript LSP (uses nwn_script_comp for diagnostics) | Ref |
| [nwscript-ee-language-server](https://github.com/PhilippeChab/nwscript-ee-language-server) | TypeScript | GPL-3.0 | 2026-09-22 | NWScript LSP (VS Code) | Out-of-process LSP (GPL is fine across a process boundary) |
| [neveredit](https://github.com/sumpfork/neveredit) | Python/wxPython | BSD (OpenKnights) | 2003–06 (imports 2020/22) | The original open NWN editor; tile adjacency helpers | Historical |
| [PyKotor / Holocron Toolset](https://github.com/OpenKotOR/PyKotor) | Python, qtpy (PyQt5/PySide6), PyOpenGL | LGPL-3.0+ | Fork active 2026-08; toolset v3.1.3 2026-04 | KotOR: every GFF editor, module designer, walkmesh editor, kit-based indoor map builder | **Ref (architecture)** |
| [KotOR.js](https://github.com/KobaltBlu/KotOR.js) | TypeScript, three.js, Electron, React | GPL-3.0 | 2026-09-14 | Odyssey engine remake + "KotOR Forge" editor | Ref |
| [reone](https://github.com/OpenKotOR/reone) | C++ | GPL-3.0 | seedhartha 2025-04; OpenKotOR fork 2026-06 | KotOR engine reimplementation | Ref |
| [bioware-kaitai-formats](https://github.com/OpenKotOR/bioware-kaitai-formats) | Kaitai KSY (+Rust) | MIT | 2026-05 | Machine-readable GFF/ERF/KEY/BIF/2DA/TLK/MDL specs (KotOR-leaning) | Ref (spec cross-check) |

---

## 3. Project notes (only what matters for Moonglow)

### rollnw / rollnw | toolset (jd28)
- **What it is:** a large (about 350k LOC), well-engineered C++20 codebase with CI, codecov,
  fuzzing and 147 test files. jd28 describes it as the foundation for a modern NWN-inspired
  toolset and game, and it now reaches well beyond format parsing (Smalls scripting language, rules
  engine, networking).
- **Formats:** GFF, ERF, KEY/BIF, ZIP, 2DA, TLK, BioWare DDS, PLT, TXI, MDL (binary and ASCII), SET,
  ITP, DLG, JRL, FAC. There is a recursive-descent NWScript parser (a parser, not a compiler).
- **`rollnw-client`** (SDL + RmlUi + Vulkan, Linux and Windows):
  - imports a `.mod` into a JSON/CAF project;
  - area editor: place, move and delete every object type, doors snap to tile hooks;
  - **Tiles tab** for terrain, features, groups, raise/lower and crossers;
  - blueprint workbenches (creature classes, feats, spells, inventory; item properties);
  - "Update Blueprint References"; a dialog *view*;
  - F9 movement preview.

  jd28 calls it *"viewer-first rather than a claim of complete NWToolset parity."*
- **Renderer:** Forward+, shadows, ozz skinning, particles and glTF, but it deliberately uses a
  modern PBR path rather than EE's look. Its open issue "NWN Modern PBR Material Calibration" admits
  that legacy assets look off under it.
- **Caveats for depending on it:**
  - "Lives at HEAD": no API or ABI stability, so you must pin a revision.
  - Heavy vendored dependency set (abseil, RmlUi, SDL, glslang, recast, …).
  - Single maintainer.
  - It explicitly declares NWN:EE-specific infrastructure (NWSync, `ruleset.2da`) out of scope.

### SWLOR.Toolset (zunath, inside SWLOR_NWN)
- **What it is:** a headless `Domain` library with an Avalonia shell and an OpenGL viewport. It reads
  and writes per-resource neverwinter.nim GFF-JSON, compiles with the vendored `nwn_script_comp`, and
  has around 3,400 tests, including full-corpus gates.
- **Tile painter:** `SetRuleMatcher` plus `TilePainter` (vertex paint and whole-cell paint). The
  authors say they verified it against the live Aurora toolset: painting one vertex rewrote exactly
  the four surrounding cells.
- **Limits:** Windows-only today and scoped to SWLOR's needs. Its early revisions linked Radoub (GPL),
  and the authors later swapped that code out with a documented clean-room process
  (`REPLACEMENT-PROVENANCE.md`).
- **For us:** a good design reference (the transaction and undo model, corpus tests, the painter
  semantics) under MIT.

### AuroraBorealius (Grimshackle)
- A Rust/egui/wgpu "complete toolset" posted to the Vault in May 2026, abandoned after about three
  days of public commits.
- It bundles `nwnsc` as a binary and renders with a custom WGSL shader (32 point lights plus
  ambient). Users reported extreme memory use.
- It is useful mainly as proof that egui + wgpu can host this UI, and as a warning about
  breadth-first scope.

### Radoub (LordOfMyatar)
- A polished GPL-3 Avalonia suite of per-file editors. Parley (the DLG editor) is the most mature:
  undo, a flowchart view, a conversation simulator, spell-check, script-parameter preview.
- There is no area or tile editing.
- It is a good UX reference for the conversation editor. Moonglow cannot use its code unless
  Moonglow is GPL.

### neverwinter.nim / nwn.py / the official compiler (niv)
- neverwinter.nim is the de-facto reference tooling. Its GFF-JSON format is what nasher,
  SWLOR.Toolset and many server repositories keep under version control, so it is Moonglow's natural
  on-disk interchange.
- **The official compiler:** Beamdog's NWScript compiler was open-sourced in neverwinter.nim 1.6.0
  (2023-07). Since 2.1.0 (2025-05) it also ships as a dynamic library on all platforms, with a C ABI
  (`compilerapi.h`: `scriptCompApiNewCompiler`, `…CompileFile`).
- **Compiler license:** the header says *"initial source release is licensed under GPL-3.0 … all
  subsequent changes … MIT … the project overall will still be GPL-3.0."*
  - Invoking `nwn_script_comp` as a subprocess keeps Moonglow's own license free.
  - Linking or dlopen'ing `libnwnscriptcomp` is the GPL-risky option. nwn.py does it anyway via
    ctypes.
- nwn.py has a Python SET parser, including `Rule` (placed/adjacent/changed). It is handy for writing
  oracle scripts.

### xoreos / xoreos-tools
- A clean, GPL-3+ C++ reimplementation of the Aurora engine. It loads NWN areas, tilesets, walkmesh
  and MDL and lets you fly around them.
- There is no EE PBR/MTR support, no editing, and no release since 2020. Recent commits are
  modernisation refactors.
- Good for reading about format edge cases. `ncsdis`/`ncsdecomp` are useful for inspecting bytecode.

### NWNExplorer / nwntools (virusman)
- Windows WTL, BSD-style license, and updated for EE (1.8.5, April 2026).
- `_NwnLib` is the classic, liberally licensed reference for the **binary MDL layout**, PLT and
  textures. Its companion `nwnmdlcomp` is the model compiler the local Neverblender pipeline already
  builds.

### Neverblender and the model tooling
- Neverblender (GPL-3+) is the ASCII-MDL semantics reference, including MTR and SET import helpers.
  The user's local port and pipeline already compile models and test them in-game under gamescope,
  and that pipeline can be reused as a parity rig.
- Also relevant:
  - Borealis (GPL-3, C++23): a binary and ASCII MDL library with a decompiler.
  - CleanModels (the Go/WASM rewrite).
  - dunahan's MIT three.js web viewer.

### KotOR precedents
- **Holocron Toolset (PyKotor):** the closest architectural analogue: a Qt desktop app over a pure
  format library. It provides:
  - one editor per GFF type (ARE/GIT/UTC/UTD/UTP/DLG…);
  - a 3D module designer (PyOpenGL scene and camera, walkmesh editor);
  - a kit-based **indoor map builder**, which is KotOR's equivalent of tile painting.

  Its lessons: keep the library GUI-free (`pykotor` vs. `toolset`), use qtpy to stay binding-agnostic,
  and note that its own plan docs show performance problems in a Python 3D module designer.
- **KotOR.js "Forge"** shows a web-tech editor on three.js.
- **reone** shows a C++ engine-grade renderer for the same family of MDL formats.

---

## 4. Focus (a): tile painting / terrain matching

**Implementations found:**

| Where | Model | Heights | Crossers | Groups | SET `[PRIMARY RULES]` | Validation |
|---|---|---|---|---|---|---|
| rollnw `tools/client/area_tile_brush.cpp` (about 1.9k LOC, plus a 2.4k-LOC test) | Corner lattice (w+1)×(h+1) of terrain and height, plus edge crossers; `fit_cell` enumerates tile × 4 orientations; random choice among matches (seeded); preserve/random/canonical fit modes | yes | yes | yes (whole footprints, doors follow) | **not parsed** | Unit tests; fails atomically with "No SET tile fits…" |
| SWLOR `GameData/Tilesets/TilePainter.cs` + `SetRuleMatcher.cs` | The same constraint model; greedy centre-then-ring solve; vertex paint that touches only the 4 incident cells, "verified against Aurora live" | yes | yes (blank-tolerant) | yes | parsed, **not used** for propagation | Corpus tests over the SWLOR haks and base game |
| AuroraBorealius, xoreos, neveredit | none (xoreos loads and renders only; neveredit has adjacency helpers) | – | – | – | – | – |

**What is still missing:**
- Aurora's rule-driven behaviour. When you paint terrain X next to Y where no tile provides that
  transition, Aurora uses the `Placed/PlacedHeight/Adjacent/AdjacentHeight → Changed/ChangedHeight`
  rules to change neighbouring corners, which propagates outward. Neither implementation does this;
  both just refuse.
- Aurora's exact variant-selection and randomisation policy.
- How features and groups interact with raise/lower.

**Recommended approach:** build the constraint solver as rollnw and SWLOR did, then add rule
propagation as a worklist over corners, and pin the behaviour with oracle tests captured from the
real toolset running under Wine.

The community wiki claims the rules are "not used in practice" and says to set `Count=0`. That is true
for custom tilesets, but **false for BioWare tilesets**: `ttr01.set` has 28 rules.

---

## 5. Focus (b): EE lighting and material rendering

**What EE does** (per the local wiki pages "Shaders", "Shader Engine Support", "MTR", "Area Lighting"
and "Enhanced Lighting Engine and PBR"):
- Rendering is driven by **text GLSL `.shd` shaders loaded through resman** (`vs*`, `fs*`, `inc_*`,
  water, post-processing, HDR bloom since 8193.36).
- MTR files bind textures and parameters to those shaders.
- The engine pushes a documented set of uniforms: matrices, skin, lights, fog, time, wind, area,
  camera, keyhole and scriptable uniforms, plus defines.
- Tile main and source lights use `tilecolor` indices; area sun and moon ambient and diffuse colours
  come from the ARE.
- The Beamdog toolset compiles the same shaders (wiki: shader "redefinition" errors appear *in the
  toolset*), but its lighting still **differs visibly from in-game**.

**What prior art does:**

| Project | Approach |
|---|---|
| rollnw | Modern Forward+ PBR with explicit non-parity; has tile-light slot and `tilecolor` resolution worth reading |
| AuroraBorealius, SWLOR, Radoub | Simple forward Phong with ambient and diffuse from the ARE (SWLOR adds brightness floors) |
| dunahan viewer | three.js `MeshStandardMaterial` |
| xoreos, NWNExplorer | Legacy fixed-function style |

GitHub code search found **no project that runs EE's stock shaders outside the game**. The only hits
are content repositories that override them.

**Recommendation:** implement the *engine side* of EE's shader contract, meaning resman-loaded `.shd`
files, `#include` expansion, the standard defines, the uniform blocks, the light lists and fog, and
then run **Beamdog's own shaders from the user's install** at runtime. We must not redistribute them,
since they are Beamdog's copyrighted content.

- An OpenGL 4.x viewport can consume them almost verbatim.
- Vulkan or wgpu needs GLSL → SPIR-V through glslang or naga, plus a compatibility shim.
- Validate against in-game screenshots captured with `nwmain` under gamescope, from identical camera
  and ARE settings.

This is the only credible route to "matches EE", and it keeps working when Beamdog updates the
shaders.

---

## 6. Focus (c): libraries we could depend on

**C++:**
- **rollnw** (MIT) is the only broad, permissive, actively tested NWN library. Vendor or pin its
  `lib/nw/{formats,resources,model,objects,i18n}` at a dated snapshot, and treat the renderer and
  Smalls as optional.
- The official compiler's C ABI (GPL, so it belongs behind a process boundary unless we are GPL).
- NWNExplorer `_NwnLib` (BSD) as a binary-MDL reference.
- Borealis MDL (GPL).

**Rust:**
- **nwnrs** has the best coverage by far: MDL, MTR, SET, KEY and a native compiler. But it is
  GPL-3.0-only, 0.0.x, on a nightly toolchain, about 6 months old, and effectively single-maintainer.
- **nwn-lib-rs** is LGPL-3+ and mature for GFF/ERF/TLK/2DA with serde support, but it has no
  KEY/BIF/MDL/SET and leans toward NWN2.
- Nothing permissive covers MDL, MTR, SET or KEY/BIF in Rust.
- The small core formats (GFF, ERF, KEY/BIF, 2DA, TLK, SSF) are cheap to write ourselves and to
  differential-test against neverwinter.nim.

**Decide the license first. It decides everything here:**
- **GPL-3.0+** unlocks nwnrs, xoreos, the official compiler in-process, Borealis and the Radoub code.
- **Permissive** limits us to rollnw (C++), our own Rust crates, and subprocess use of the GPL tools.

---

## 7. Recommendations

**Depend on:**
- **The official NWScript compiler** for byte-exact NCS. Call it as `nwn_script_comp` (subprocess, so
  Moonglow keeps any license) or through `libnwnscriptcomp`'s C ABI (only if Moonglow is GPL-3).
  Do not reimplement a compiler for v1.
- **neverwinter.nim's GFF-JSON format** as the source-tree and version-control format, compatible
  with nasher. Read and write it natively.
- **Format library:**
  - C++ path: rollnw, pinned to a dated snapshot.
  - Rust path: our own permissive crate for GFF/ERF/KEY/BIF/2DA/TLK/SSF/SET/MTR/MDL, borrowing
    structure from nwn-lib-rs and nwnrs (read, not copied, unless we go GPL). If Moonglow is GPL,
    depending on nwnrs is the fastest route.
- **EE's own `.shd` shaders**, loaded from the install at runtime, for the viewport.
- **An out-of-process NWScript LSP** (nwscript-ee-language-server or cgtudor/nwscript-lsp) for the
  script editor, until we have our own.

**Use as reference or test oracle:**
- **Formats:** round-trip every resource through `nwn_gff`, `nwn_erf`, `nwn_twoda` and `nwn_tlk`
  (neverwinter.nim, installed at `~/.local/opt/neverwinter/bin`). Run corpus tests over the base game
  and big community haks and modules, the way SWLOR does.
- **Scripts:** require byte-identical NCS output against `nwn_script_comp`.
- **Tile painting:** the **Aurora toolset under Wine/Proton** as ground truth, recording paint
  scenarios and their results, as SWLOR did. Use the rollnw and SWLOR painter tests as secondary
  cross-checks. The base-game and OC module corpus provides valid-adjacency invariants.
- **Rendering:** `nwmain` screenshots (gamescope, fixed camera, controlled ARE/time of day) compared
  against Moonglow renders. rollnw's tile-light and `tilecolor` code and the wiki's uniform tables
  are the spec.
- **Models:** NWNExplorer `_NwnLib` and Neverblender for MDL semantics; the engine's own model
  compile (the existing local pipeline) as the oracle.
- **UX and architecture:**
  - Holocron (library/GUI split, per-type editors, module designer);
  - Radoub Parley (DLG UX);
  - rollnw client (project import, command palette, transaction/undo docs);
  - SWLOR (headless Domain library, corpus gates).

**Gaps nobody has filled:**
1. **Faithful Aurora terrain matching**: SET primary-rule propagation, height interplay and variant
   selection. Both existing painters stop short of this.
2. **An EE-parity renderer**: MTR/PBR, tile and area lights, fog and post-processing driven by the
   stock shaders. Nobody has attempted it.
3. **A complete, cross-platform (Linux-first), stable-licensed module toolset.** No project covers all
   of these together: areas and tiles, a full conversation editor with journal and script-parameter
   integration, every blueprint type, a script editor with the official compiler, and Build/Test
   (F9). rollnw is closest but partial and churning; SWLOR is Windows-only and SWLOR-only;
   AuroraBorealius is abandoned; Radoub has no areas.
4. **A permissive Rust crate** for NWN1 MDL/MTR/SET/KEY-BIF.
5. **Toolset build semantics.** Nobody reimplements the toolset's module "Build" side effects
   (palette ITP regeneration, compile-all, IFO area and HAK bookkeeping). I did not verify this
   exhaustively; I saw no implementation in the projects above apart from rollnw's
   blueprint-reference updates.

---

## Sources

**Repositories:**
- https://github.com/jd28/rollnw
- https://github.com/jd28-archive/arclight
- https://github.com/zunath/SWLOR_NWN
- https://github.com/Evangelion1337/AuroraBorealius-Toolset-and-Shargast-by-Jaysn
- https://github.com/LordOfMyatar/Radoub
- https://github.com/Cjreek/Eos-Toolset
- https://github.com/niv/neverwinter.nim
- https://github.com/niv/nwn.py
- https://github.com/nwneetools/nwnsc
- https://github.com/xoreos/xoreos
- https://github.com/xoreos/xoreos-tools
- https://github.com/virusman/nwnexplorer
- https://github.com/gyoerkaa/mdltools
- https://github.com/varenx/borealis_nwn_mdl
- https://github.com/dunahan/nwn_mdl_webviewer
- https://github.com/squattingmonk/nasher
- https://github.com/urothis/nwnrs
- https://gitlab.com/CromFr/nwn-lib-rs
- https://github.com/Mingun/serde-gff
- https://github.com/Youx/gff-rs
- https://github.com/winternite/Aurora-Hak-Explorer
- https://github.com/cgtudor/nwscript-lsp
- https://github.com/PhilippeChab/nwscript-ee-language-server
- https://github.com/sumpfork/neveredit
- https://github.com/OpenKotOR/PyKotor
- https://github.com/OpenKotOR/HolocronToolset
- https://github.com/KobaltBlu/KotOR.js
- https://github.com/OpenKotOR/reone
- https://github.com/OpenKotOR/bioware-kaitai-formats

**Beamdog statements:**
- https://forums.beamdog.com/discussion/82450/ (JuliusBorisov and virusman, July/August 2021)
- https://forums.beamdog.com/discussion/66930/linux-version-of-the-toolset (2017)

**Other:**
- https://neverwintervault.org/project/nwn1/other/tool/auroraborealius-toolset
- Local nwn.wiki pages: Aurora Toolset, SET, Shaders, Shader Engine Support, MTR, Area Lighting,
  patch notes 8193.36 and 8193.37.
- Local game install: `bin/win32/nwtoolset.exe` (PE32, Embarcadero RAD Studio strings; build
  2025-10-06) and `ttr01.set`.
