---
type: Reference
title: 'Moonglow Toolset: Architecture and Record of the Phases'
description: What Moonglow Toolset reproduces of Aurora, the landscape, its principles, crate architecture and key designs, a record of the phases it was built in with the tests that hold each, the testing strategy and licensing. The plan it began as, kept as the description of what was built.
tags: [architecture, testing, parity, phases, plan]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-27T08:33:19Z }
---

# Moonglow Toolset: Architecture and Record of the Phases

Moonglow is a from-scratch, cross-platform reimplementation of the Neverwinter
Nights: Enhanced Edition Aurora Toolset (`nwtoolset.exe`), written in Rust with
an egui interface. The first goal was **functional parity with Aurora**: every
module Aurora can author, Moonglow can author, and every module Moonglow saves
behaves the same in the game. Byte-identical output is not a goal; functional
regressions are not allowed.

This document began as the plan (2026-09-30). The phases it planned are
built, so it is now the description of the architecture (§1 to §5), a record
of the phases (§6), the testing strategy (§7) and licensing (§8). What is
still open is not here: it is in the [deferred list](deferred.md).

Decisions taken (2026-09-30):

| Topic | Decision |
| --- | --- |
| Language / UI | Rust (stable), egui (+ egui_dock) |
| Renderer | wgpu, with Moonglow's own shaders approximating EE's lighting closely enough to look like the game client (§5.5) |
| Platforms | Linux, Windows, macOS from the start |
| License | GPL-3.0 (§8) |
| Oracles | Files (bundled modules, neverwinter.nim, nwn.py), Aurora under Wine, the game itself (nwserver/nwmain with a separate test user directory) |

## 1. What we are reproducing

Aurora is a 32-bit C++Builder (VCL) program, version 1.5.0.5 in game build
89.8193.37-17. Its 105 forms (dialogs and frames) are embedded as DFM resources;
`tools/aurora/run-aurora.sh extract-forms DIR` decodes them to text.
`docs/parity/aurora-ui-inventory.md` turns them into the parity checklist: every
form, tab and control, with the GFF field or 2DA it edits, event handlers,
shortcuts, dead (hidden) controls, and a complexity estimate per subsystem.

Aurora's labels, tooltips and status-bar help are not in the forms but in
`dialog.tlk`, looked up through a per-form table of StrRefs in the executable
(recovered for 83 of 105 forms: `docs/parity/aurora-form-strref*.json`).
Moonglow can use the same StrRefs, so its labels match Aurora's wording and are
localized in every game language for free.

Subsystems, roughly in the order a user meets them:

- **Module**: new-module wizard, open/save/close (.mod and module folders),
  module properties (IFO: events incl. the EE ones, haks with priority and
  conflict check, custom TLK, areas, start location, variables, load screen, XP
  scale, time, tag), import/export ERF.
- **Area editor**: area wizard, tile painting (terrain, crossers, groups,
  heights), area properties (lighting, fog and fog clip, shadows, sound, music,
  weather, skybox, scripts), resize/rotate, 3D view with EE lighting, object
  placement and editing (creatures, doors, encounters, items, stores,
  placeables, sounds, triggers, waypoints, start location, visual transforms),
  filters, grid, fade geometry, AABB view, find instance, multi-edit,
  copy/paste, undo/redo, preview window, area statistics.
- **Blueprints and palettes**: standard and custom palettes (ITP), wizards and
  property editors for the nine blueprint types, including creature classes
  (8), feats, spells, skills, inventory, level-up wizard, wings/tail, and the
  item property system.
- **Conversation editor** (with script parameters), **script editor** (with the
  official compiler, optional external editor), **journal editor**, **faction
  editor**, **plot wizard**.
- **Build**: build (compile) module, verify module, test module (F9: saves, then
  runs `nwmain -userdirectory … +TestNewModule …`).
- **Options**, pickers (portrait, sound, soundset, load screen, resource),
  localized-string editor, variable tables, comments.

Controls Aurora still carries but hides (world map, secret door DC, item
generators, save to savegame, static lighting) are listed in the inventory and
skipped deliberately.

## 2. Landscape

Surveyed 2026-09-30. Nothing makes this project redundant: Beamdog has said a
toolset rewrite will not happen, and the four 2026 replacement attempts are
partial (rollnw's toolset is viewer-first; SWLOR.Toolset is Windows-only and
server-specific; AuroraBorealius, also Rust/egui, was abandoned after a few
days; Radoub has no area editor). Gaps nobody has filled, which Moonglow must:

- **Faithful terrain painting**: two painters exist (rollnw, SWLOR) but neither
  applies the tileset's `[PRIMARY RULES]`, which BioWare tilesets rely on
  (all 47 shipped tilesets together have 1,192).
- **An EE-parity renderer**: no open renderer reproduces EE's lighting and
  materials; the game's own GLSL shaders ship as plain text in the install.
- **A complete, cross-platform, stably licensed module toolset.**

What we use: neverwinter.nim's tools as oracles and its GFF-JSON as the
interchange format (nasher-compatible); the official NWScript compiler
(`nwn_script_comp`) for NCS; rollnw's and SWLOR's painters and PyKotor/Holocron's
architecture as references. The format crates are our own (the Rust
alternatives are GPL, LGPL or incomplete; §8).

## 3. Principles

1. **Game data is never shipped.** Moonglow reads the user's installed game at
   run time. The test corpus is the user's install, found through `NWN_ROOT`
   (or auto-detection); corpus tests skip, loudly, when it is absent. Fixtures we
   commit contain only data we author.
2. **Lossless editing.** Typed editors are views over the raw GFF tree: unknown
   fields, structs and list entries survive every edit. Saving an untouched
   module yields a semantically identical module.
3. **UI-agnostic core.** Every operation (painting a tile, placing an object,
   editing a field, compiling, building) is a command in a non-UI crate, with
   undo, testable without a window and scriptable through the CLI. The egui layer
   only renders state and issues commands.
4. **Everything headless.** The renderer renders offscreen, the CLI covers
   build/verify/pack/compile/diff, and the UI runs under `egui_kittest` in tests.
5. **Deterministic output.** Same input, same bytes: stable ordering, no
   timestamps unless the format needs them.
6. **Fast by construction.** Memory-mapped BIF/ERF, lazy parsing, parallel
   indexing, asynchronous asset loading, GPU instancing. Performance budgets are
   measured (§7).
7. **Interoperable.** GFF JSON compatible with neverwinter.nim and nasher, so
   Moonglow projects can live in git alongside existing tools.
8. **The engine is the reference.** Where tools disagree, an engine test
   decides (§7).
9. **Portable.** Platform differences (game/user directory detection, launching
   the game, file dialogs) live behind small interfaces; CI builds and tests all
   three platforms.

## 4. Architecture

A Cargo workspace, layered bottom-up. A crate may only depend on crates above it
in this list (test code may also use `mg-testkit`). All of these exist.

| Crate | Responsibility |
| --- | --- |
| `mg-core` | ResRef, ResType table, languages and codepages, StrRef, LocString, bounds-checked binary reader/writer, SHA-1 |
| `mg-gff` | GFF V3.2 read/write, lossless tree, typed accessors, nwn-lib JSON, structural diff |
| `mg-erf` | ERF/MOD/HAK/NWM/SAV: read V1.0 and E1.0 (zstd), write V1.0 |
| `mg-key` | KEY/BIF V1, memory-mapped |
| `mg-2da` | 2DA V2.0, lenient like the engine, number parsing like the engine |
| `mg-tlk` | TLK V3.0, custom TLK StrRefs |
| `mg-set` | tilesets (SET/INI) with data warnings |
| `mg-ssf` | soundsets |
| `mg-audio` | sounds and music decoded (MP3 behind `BMU V1.0`, IMA ADPCM and PCM WAVE) for playback |
| `mg-resman` | layered resolution in the engine's order, DDS/TGA rule, install detection |
| `mg-image` | TGA, DDS (standard and BioWare, BC1–BC5), PLT and palettes, TXI, MTR (KTX: not yet) |
| `mg-shader` | the game's own shaders (`.shd`): a program assembled as the engine does it (preamble, `#include` splicing through the load order), compiled by glslang (built in), its combined samplers split and locations given on the SPIR-V, read into a validated naga module with its uniforms, textures and varyings by name; errors at their file and line ([the shaders proposal](shaders-proposal.md), M0) |
| `mg-mdl` | MDL binary and ASCII, read and written; the model compiler, the linter and the outline; WOK/PWK/DWK walkmeshes |
| `mg-schema` | typed views of IFO, ARE, GIT, GIC, UTC/UTD/UTE/UTI/UTM/UTP/UTS/UTT/UTW, DLG, JRL, FAC, ITP, PTM and PTT, generated from the observed schema |
| `mg-rules` | 2DA/TLK-backed game data (appearance, baseitems, classes, feats, spells, skills, item properties, placeables, doors, portraits, sounds, ...); the nine types of placed object (`ObjectKind`: blueprint type, GIT list and struct id, names) |
| `mg-script` | the compiler built in; NWScript lexer, outline and analysis for the editor and `mg lsp`; the `nwscript.nss` spec |
| `mg-tiles` | the tile grid over a lattice of corners, tile fitting, and terrain painting with the SET's rules |
| `mg-module` | module workspace: open/save, working copy, palettes, reference graph, verify and the content doctor, build, import/export, haks/TLK, nasher projects |
| `mg-edit` | editor core: the undoable workspace, commands with their inverses, edits as JSON (no UI) |
| `mg-nui` | NUI windows: the JUI document and its settings, the checks against the stock `nw_inc_nui` API and what the client does, and the generated opener and event scripts (`mg nui`; the manual's `18-nui-creator.md`) |
| `mg-plugin` | plugins: manifests, and a sandboxed Luau runtime in which a plugin's commands and checks read a module and hand back edits and findings (the API in `docs/manual/17-plugin-api.md`; the decisions in `docs/plugin-proposal.md`) |
| `mg-serve` | the game's headless server run on a module: private unless asked, a user directory of the caller's, its log followed, its console on a pipe (`mg serve`, `mg test`, the engine tests) |
| `mg-render` | renderer: scene, materials and maps, EE lighting (matched to the client), offscreen mode, animation, skinning, animated and dangly meshes, particles |
| `mg-preview` | blueprint previews: part-based and single-model creatures with equipment, wings and tails, PLT colors; items; placeables; doors (the renderer's half, `compose`, is the default feature `render`: without it, no mg-render) |
| `mg-area` | areas as the area editor shows them: tiles with their lights and animation loops, placed objects with their previews and outlines, sun, moon and fog; the renderer's scene of an area; picking, arranging, reshaping and the walkmesh |
| `mg-ui` | egui widgets and editors |
| `apps/mg` | the command line: 44 commands (`mg --help`; the manual's `11-command-line.md`), each able to answer in JSON; `mg nui render` and `mg nui layout` (mg-ui, wgpu) are the default feature `nui-preview`, and the rest builds without the GPU stack |
| `apps/moonglow` | the GUI application |

Test support: `mg-testkit` (corpus locator, oracle tools, engine runner),
`mg-corpus-tests` (corpus, differential and engine tests). Tools:
`tools/aurora/` (Aurora under Wine, on the desktop or on an off-screen display driven by
scripts; form decoder, inventory generator, wizard capture), `tools/nwclient/`
(the sandboxed game client) and `tools/shaders/`.

## 5. Key designs

### 5.1 Lossless typed objects

`mg-gff` stores a GFF as a tree of structs (type id + ordered fields, duplicate
labels allowed). `mg-schema` defines each object type as a thin typed wrapper
generated from the observed schema (`schema_codegen.rs`): getters return the field or Aurora's default,
setters write only that field. Nothing is discarded, so unknown EE or NWNX fields
survive. Round-trip tests compare trees exactly; oracle comparisons normalize
field order (which the game ignores).

### 5.2 Resource manager

Layers carry the engine's numeric priorities (`mg_resman::priority`, from the
engine's own table): portraits 91/90, `development/` 71, NWSync 40, haks 31
(user `hak/`) and 30 (`data/hk`) in `Mod_HakList` order, the module 20,
`override/` 12, ambient/music 9–6, keys 1 (`nwn_retail*` over `nwn_base*`,
`_loc` keys from `lang/<xx>/data` over their base). So haks beat the module,
the module beats `override/`, and a hak in the user's folder beats an install
hak listed before it; the `engine_resman` test crosses each of these
boundaries and matches `ResManGetAliasFor`. Textures follow the engine's
exception: DDS beats TGA within a class (directories, NWSync, ERFs, keys)
before a lower class counts. The install's `ovr/` is not searched (EE keeps
`nwscript.nss` there for external tools only), and `data/txpk/*.erf` are
empty stubs in EE (textures ship in the keys). Containers are memory-mapped
and indexed once; lookups are hash-map hits.

### 5.3 Editing model

A module is a set of documents (module info, areas, blueprints, dialogs, scripts,
journal, factions). Each edit is a `Command` with `apply`/`revert` that the
`mg-edit` undo stack records; multi-object edits are composite commands. The UI
never mutates documents directly. Undo/redo, the multi-editor, scripted tests
and the CLI share one code path.

### 5.4 Scripts

The official NWScript compiler (Beamdog's, open-sourced in neverwinter.nim,
GPL-3.0) produces the NCS; Moonglow does not write its own code generator.
`mg-script` builds the compiler's C++ sources with the `cc` crate and calls its
small C API (`compilerapi.h`: new compiler, deliver source through a callback,
compile, receive NCS/NDB) in process, so no external binary is needed on any
platform. Optimization flags are 1 (dead code only), as in the game and
Aurora; only the first error is reported, as in Aurora. `nwn_script_comp`
stays the test oracle. Editor features (highlighting, outline, completion,
go-to-definition, signature help, diagnostics) come from Moonglow's own
NWScript front end in `mg-script`, fed by `nwscript.nss` from the resman.

### 5.5 Renderer

One wgpu renderer serves the area editor, blueprint previews and the model
viewer. Aurora shows none of EE's lighting, so the goal is a close match to the
game client, not bit-exact parity. Moonglow's own WGSL shaders implement EE's
lighting model as the research brief derives it from the stock shaders
(`docs/research/ee_tech_brief.md` §6, `notes_shaders.md`): linear lighting with
gamma correction; per-fragment lighting from area sun/moon ambient and diffuse,
tile main and source lights (`lightcolor.2da`), and placeable and model lights
(top 32 per object by priority and distance); GGX specular; MTR materials with
normal, specular, roughness, height and self-illumination maps; environment
maps, fog, skyboxes, emitters and water; and an area's grass, which Aurora
draws none of, grown on the tiles' walkmeshes as the game grows it and
drawn wherever the view shows grass ground (the client draws it within 30 m
of its camera by default), thinned far off (`mg_area::grass_view`). Not
drawn: shadows (see the [deferred list](deferred.md)). Editor overlays (grid, selection, trigger
polygons, walkmeshes) are drawn over the scene. Toggles like Aurora's "use area lighting" and day/night stay.
The renderer also runs offscreen for image tests (§7, L5): reference scenes are
compared, within a tolerance, with screenshots of the same area, camera and
time of day taken in the real client (`nwmain` with a test user directory).

Running the game's own shaders was not chosen for 1.x, and exact
stock-shader parity is not a goal of the WGSL shader. For 2.0 it is
chosen ([the shaders proposal](shaders-proposal.md), accepted
2026-10-21): every mesh draws with the program the engine would choose,
compiled on the user's machine from the install's and the haks' `.shd`
files (a hak's override of a stock include, and the custom shaders MTRs
name, included), by native glslang (relaxed Vulkan rules) into naga. The
WGSL shader stays as the fallback: for a program still compiling, for one
that doesn't compile (reported by Verify, not drawn magenta) and with the
view's Shaders switch off. The switch is on by default once the frame
budget is met. The compiler is built (`mg-shader`, M0, 2026-10-21:
glslang-rs, with the combined-sampler split SPIRV-Tools did in the spike
done in Rust on the SPIR-V words): it takes all 102 programs on the
development machine to validated naga modules, and Verify's
`shader-compile` check reports a module's programs that don't compile.
Nothing is drawn with them yet.

### 5.6 Tile painting

`mg-tiles` models the area as a lattice of corner terrains and heights plus edge
crossers. Painting chooses, for each affected cell, a tile and orientation from
the SET whose corners and edges match its neighbors; where no tile provides a
transition, the SET's `[PRIMARY RULES]` (placed/adjacent → changed terrain and
height) rewrite neighboring corners and the change propagates. Aurora's exact
choices (tie-breaking, randomness, propagation extent) were pinned down by
recording Aurora under Wine on scripted painting sequences and replaying them
against Moonglow (§7, L4; `aurora_terrain.rs`).

## 6. Record of the phases

The work was planned as thirteen phases, each ending with a demonstrable
result and green tests, and a list of improvements after parity. All of it
is built; the releases are the tags `v0.1.0` (2026-10-01) to `v1.21.3`
(2026-10-11). Each row says what the phase was and what holds it. Test
files are in `crates/mg-corpus-tests/tests/` unless a path says otherwise.
What a phase left undone is in the [deferred list](deferred.md).

| Phase | What it was | Held by |
| --- | --- | --- |
| 0 Bootstrap | Workspace, lint and format settings, CI on three systems, corpus locator, oracle tools, engine test runner, Aurora harness, parity inventory | `.github/workflows/ci.yml`, `mg-testkit`, `tools/aurora/`, `docs/parity/aurora-ui-inventory.md` |
| 1 Core formats | GFF, ERF (V1.0, and EE's E1.0 read), KEY/BIF, 2DA, TLK, SET, SSF, ITP; textures, TXI and MTR came with Phase 7 | `archives.rs`, `base_game.rs`, `nwn_gff_diff.rs`, `twoda.rs`, `tlk.rs`, `small_formats.rs`, `palettes.rs` |
| 2 Resource manager and rules | The engine's load order and the game's data (2DA cells, talk-table strings) | `engine_resman.rs`, `engine_2da.rs`, `engine_rules.rs` |
| 3 Schema and module workspace | Typed field descriptors for all 19 authored GFF types, generated from the schema observed in the shipped data; the module workspace (archives and folders, atomic save with backup), reference graph, verify, hak conflict report, ERF export and import | `modules.rs`, `engine_modules.rs`, `engine_verify.rs`, `engine_transfer.rs`; `mg verify`, `mg haks`, `mg export`, `mg import` |
| 4 Script back end | The official compiler built in (vendored C++ built with the `cc` crate, on the three systems in CI), module compile, and the NWScript front end (lexer, declaration parser, the `nwscript.nss` spec) | `scripts.rs`, `script_frontend.rs`, `script_analysis.rs`, `nwscript_spec.rs` |
| 5 Application shell | The undoable workspace (`mg-edit`) and the application (`mg-ui`, `apps/moonglow`): menus, toolbar and keys, module tree, docked tabs, Module Properties, the script editor, a generic GFF editor, the log, the New Module and Area wizards with Aurora's defaults, settings, options, import and export, the resource browser | `crates/mg-ui/tests/app/` (`egui_kittest`), `aurora_new.rs`, `engine_new_module.rs` |
| 6 Text-and-tree editors | String Edit, Variables, resource pickers; the Faction, Journal and Conversation editors; the script editor's tools and the Script Wizard | `aurora_factions.rs`, `aurora_journal.rs`, `aurora_dialog.rs`, `aurora_script_wizard.rs`, `engine_factions.rs`, `engine_journal.rs`, `engine_dialog.rs`; `docs/parity/checklists.md` |
| 7 Renderer and model viewer | Textures (TGA, DDS, PLT), TXI and MTR; models, binary and ASCII; the wgpu renderer with the game's lighting, material maps, animations with supermodels, skinning, animated and dangly meshes, emitters; blueprint previews (`mg-preview`); the model viewer | `textures.rs`, `models.rs`, `models_nwnmdlcomp.rs`, `crates/mg-render/tests/render.rs`, `crates/mg-preview/tests/previews.rs`, `client_render.rs` (by hand) |
| 8 Blueprints and palettes | Standard and custom palettes, the palette pane, editors and wizards for every blueprint type, item costs as the engine computes them | `aurora_blueprints.rs`, `aurora_palette_add.rs`, `engine_blueprints.rs`, `engine_item_cost.rs`, `item_properties.rs` |
| 9 Area editor, objects | Every shipped area opens and draws; the area viewer with Aurora's camera and selection bindings, filters, grid, placing, moving, turning, raising, copy and paste, polygons, Adjust Location, Find Instance, Area Properties, the context menu's commands, the Preview window | `areas.rs`, `aurora_instances.rs`, `engine_area_edits.rs`, `engine_facings.rs` |
| 10 Area editor, terrain | Painting as Aurora paints (terrain, crossers, groups, heights), Resize Area and Rotate Area, tile properties, the walkmesh overlay, Area Statistics | `aurora_terrain.rs`, `engine_terrain.rs`, `tiles.rs` |
| 11 Build, verify, test | Build Module with Aurora's options, the challenge rating calculation, Test Module (F9), Verify, Update Instances, the Levelup and Creature Wizards, creature spell and inventory warnings, sound playback, the remaining dialogs | `aurora_creatures.rs`, `aurora_levelup.rs`, `aurora_creature_wizard.rs`, `aurora_spell_warnings.rs`, `aurora_inventory.rs`, `aurora_update_instances.rs`, `audio.rs` |
| 12 Hardening and release | Recovery copies of unsaved work and crash reports; performance budgets on the largest campaigns; packages (AppImage, Flatpak, Windows installer, macOS disk image, built by `.github/workflows/release.yml` on every `v*` tag); the user manual (17 chapters, also in the application) | `crates/mg-ui/src/recovery.rs`, `crates/mg-ui/tests/campaign_perf.rs` (by hand), `packaging/`, `docs/manual/` |

Parity itself is tracked row by row in `docs/parity/checklists.md`: each row
is done, deliberately different, or partly done with its gap named. Known
gaps against the original scope: the Plot Wizard is not built; spell
checking is left out (EE ships no lexicon, and Aurora's finds nothing);
the Conversation Editor's Scrap tab is partly done. The renderer's approximations (dangly-mesh dynamics and the
point-to-point and lightning emitters follow the wiki's descriptions, not
measurements) and what it does not draw (shadows) are in §5.5.

### After parity

Ordered by what builders report about Aurora
(`docs/research/community_pain_points.md`, surveyed after 0.2.0). All
three tiers are built; the manual describes each feature, and the deferred
list holds what each left undone.

| Item | Where it is | Held by |
| --- | --- | --- |
| git-native modules (nasher trees opened and saved in place) | File › Open Folder…, Save As nasher Project…, `mg init`, `mg build` | `nasher.rs`; `docs/research/notes_nasher.md` |
| Where-used and rename everywhere | Find References, Rename, `mg refs`, `mg rename` | `engine_rename.rs` |
| A content doctor | Verify Module, `mg verify --json`, `mg checks` | `doctor.rs`, `engine_doctor.rs`; `docs/research/notes_doctor.md` |
| A test loop (choose the character, test from here, reload haks and 2DAs) | Test Module, Test From Here, Reload Resources | `engine_test_start.rs` |
| Placement tools (drop to ground, turning keys, snapping, locking, prefabs) | the area view | `engine_locked.rs` |
| A "Coming from Aurora" chapter | `docs/manual/14-coming-from-aurora.md` | |
| Script intelligence | the script editor, `mg lsp` | `script_analysis.rs`, `apps/mg/tests/lsp.rs` |
| Budgets at persistent-world scale; haks over 2 GiB | | `crates/mg-ui/tests/world_perf.rs` (by hand), `engine_big_hak.rs`; `docs/research/notes_scale.md` |
| The EE fields Aurora hides | the editors' Visuals pages and others | `engine_ee_fields.rs`; `docs/research/notes_ee_fields.md` |
| Bulk editing | Update Instances for a selection, several blueprints or areas together, Find and Replace Text, `mg update-instances`, `mg replace`, `mg areas` | `aurora_update_instances.rs`, `text_replace.rs` |
| Conversation authoring (markers, play-through, export and import) | the Conversation Editor, `mg dialog-export`, `mg dialog-import` | `dialog_formats.rs` |
| Palette search, favorites and recent | the palette pane | `crates/mg-ui/tests/app/palettes.rs` |
| 2DA view and talk-table editor | the resource browser, Tools › Talk Table | `engine_tlk.rs` |
| Haks in the GUI | Tools › Haks, `mg attach` | `hak_edit.rs` |
| Object walkmeshes and minimap export | the area view, `mg minimap` | `object_walkmeshes.rs`, `minimaps.rs`, `client_minimap.rs` (by hand) |
| Remappable keys | Tools › Options › Keyboard: every command of the menus, kept by command id | `crates/mg-ui/src/keys.rs`, `crates/mg-ui/src/commands.rs` |
| Automation | `--json` on every `mg` command, `mg find`, `mg info`, `mg set`, `mg apply`; plugins (`mg-plugin`, experimental) | `apps/mg/tests/json.rs`, `crates/mg-plugin/tests/` |
| NWSync publishing | Build › Publish to NWSync…, `mg nwsync` | `nwsync.rs` |
| Tileset authoring | Tools › Tilesets, `mg tileset-palette` | `tileset_authoring.rs` |

Built since, outside this list (see the manual's
[command-line chapter](manual/11-command-line.md) and the
[log](log.md)): `mg serve`, `mg test` and `mg watch` (`mg-serve`), `mg
2da-diff` and `mg tlk-diff`, `mg tlk-alloc`, lights from the area's menu
([the proposal](lights-proposal.md)), and the model compiler and writers
in `mg-mdl`.

### What the test runs recorded

These counts are what the test runs printed when the work landed
(October 2026, game build 89.8193.37-17). They were not measured again
for this revision, and the tests assert lower bounds
(`mg_testkit::at_least`), not these figures.

- All 57 shipped modules and haks index; all 27,179 GFFs inside them and every
  GFF among the ~114,000 base-game resources round-trip to identical trees;
  rewritten archives keep every entry byte for byte.
- 360 sampled GFFs of every type agree with neverwinter.nim's `nwn_gff` in
  both directions (its JSON of the file, and the GFF it writes from ours).
- All 822 shipped 2DAs parse and agree with `nwn_twoda`, except where the
  engine proves `nwn_twoda` wrong (§7).
- All 10 installed talk tables (every language, masculine and feminine)
  round-trip.
- For ~600 resources placed to cross every precedence boundary, Moonglow's
  resman picks the same source as the engine's `ResManGetAliasFor`.
- Every shipped module (28), with every GFF rewritten by Moonglow and saved
  as an archive and as a folder, reopens identical; run in `nwserver`, the
  rewritten modules present exactly the same world as the originals (every
  area, and 121,287 objects with type, tag, blueprint, position and facing).
- Of 4,943 references checked across all shipped modules, the 989 Moonglow
  reports missing are unknown to the engine.
- The built-in compiler compiles all 33,196 scripts in the base game and
  the shipped modules byte for byte as `nwn_script_comp` does; the shipped
  NCS match only where the same compiler produced them (68%). For the
  33,121 compilable ones, the outline names exactly the functions,
  constants and structs in the compiler's debug output.
- All 47 shipped tilesets parse (1,192 primary rules between them); all 487
  soundsets round-trip byte for byte.
- All 32,832 models in the game read, binary and ASCII; the two readers
  agree through nwnmdlcomp on a sample of 176.
- The reference scenes (`client_render.rs`: sun; tile lights) match the
  client within 1/255 per region mean (a floor region on a dark wall's
  edge within 5; tolerances 3 and 6), and the light uniforms Moonglow
  uploads equal the client's.
- Every shipped area (1,462) opens and renders.
- `aurora_terrain.rs` replays 62 recorded painting steps in four tilesets
  (54 scripted by `tools/aurora/capture_terrain.py`, 8 group placements by
  hand) and four resizes and rotations; every step gives Aurora's lattice.
- All 44 captured new areas (every tileset, sizes 2 to 8) match Aurora's
  field for field except the random choices.
- Every base-game item's cost agrees with `nwserver`; Aurora's challenge
  ratings match for 4,204 probe creatures.
- All 2,854 shipped conversations survive a round trip through the export
  formats (`dialog_formats.rs`).
- Tyrants of the Moonsea (5,295 resources) opens in 0.05 s, its largest
  area (784 tiles) in 0.3 s and draws in 13 ms, verify takes 0.7 s, all
  1,433 scripts compile in 0.7 s and the module writes in 0.02 s, against
  budgets of 3 s, 5 s, 50 ms, 10 s, 120 s and 5 s (`campaign_perf.rs`, on
  the development machine).

## 7. Testing strategy

| Tier | What | Runs |
| --- | --- | --- |
| L0 unit | per-crate tests, property tests (proptest, in `mg-core`, `mg-gff` and `mg-edit`), truncation tests | always |
| L1 corpus | parse and round-trip every resource in the install and bundled modules | when a game install is found: a sample of the long sweeps on every run, all of them before a release |
| L2 differential | compare against neverwinter.nim and nasher (GFF JSON, 2DA, NCS bytes, ERF, nasher source trees, NWSync manifests) | when the tools are installed; sampled as L1 |
| L3 engine | build a module, run it in `nwserver` with a scratch user directory; a probe script logs what the engine sees (`Get2DAString`, `ResManGetAliasFor`, object state) | Linux, when the game is installed; the tests over every shipped module sampled as L1 |
| L4 Aurora parity | scripted scenarios in Aurora under Wine produce reference outputs; Moonglow must match them semantically | captures are made on demand and kept outside the repository (`~/.local/share/moonglow-oracle/captures/`); the tests replay them when they are there |
| L5 image | offscreen renders (`mg-render`, `mg-preview`) on every run with a GPU; comparisons with the game client's screenshots and shader uniforms (`client_render.rs`, region means within a tolerance) | with a GPU; the client comparisons by hand |
| L6 UI | `egui_kittest` flows through editors and dialogs | always |

Every feature lands with tests at the lowest tier that can catch its
regressions, plus a line ticked in the parity checklist that names the test.

**Sweeps and samples.** A test that goes over the whole install (every
area rendered, every model decompiled and compiled back, every script
compiled, every sound decoded, every shipped module run in the server)
takes a fixed sample on an ordinary run, every fifth or sixth module and
every eighth or tenth resource (`mg_testkit::sweep`), and all of it when
`MOONGLOW_FULL_CORPUS=1` is set, as it is before a release. The sample is
the same each run, so a failure stays where it was; counts a sweep must
reach are scaled (`mg_testkit::at_least`), and shares that describe the
whole game get some slack on a sample. Sweeps that take a second or two
are not sampled. Where several tests need the same costly thing they share
it: one decompile of each model feeds three tests, one compile of each
script two, and the engine tests that compare a changed module with the
original run the original once (`common::original_world`). The corpus
tests are six binaries (`tests/groups/`), so that tests of different
files run side by side; at most eight servers run at once
(`mg_testkit::engine`), and GPU tests four at a time (`mg_testkit::gpu`;
`MG_TEST_GPUS`).
How this came about, with the timings before and after, is in the
[test review](test-review.md).

When an oracle tool disagrees with Moonglow, an engine test decides and the
divergence is recorded (see `CLAUDE.md`). So far: `nwn_twoda` drops `""` 2DA
cells, but the engine reads them as empty cells in place (Beamdog's own
`id_resources.hak` depends on it); `nwn_twoda` crashes on tables without
non-empty rows; `nwn_gff` cannot read back JSON with an empty field label, which
BioWare's own palettes contain.

**Performance.** Performance is measured by four test targets in
`crates/mg-ui/tests/`, run by hand in release (their tests are ignored,
and the targets are `test = false`, so neither `cargo test` nor CI runs
them): `campaign_perf.rs` (the three largest campaigns) and
`world_perf.rs` (a synthetic persistent world) hold each step to a budget;
`frame_perf.rs` (frames against 144 a second) and `editor_perf.rs` (the
script editor on `nwscript.nss`) print timings. They need the game, and
all but `editor_perf.rs` a GPU. Budgets are set from the first
measurements (three to five times them in `world_perf.rs`) and only
tightened.

## 8. Licensing

Moonglow is licensed under GPL-3.0 (`LICENSE`; `GPL-3.0-only` in the crate
manifests). That allows building in the official NWScript compiler (GPL-3.0)
and depending on GPL code where it helps (nwnrs for MDL/MTR/SET cross-checks,
xoreos and Neverblender as references; nwnrs is in no manifest today).
Beamdog's shaders and all game assets are loaded from the user's install
and never redistributed.

## 9. Risks

The risks named at the outset are settled, so the table is dropped: tile
painting was pinned down by recorded Aurora sessions (`aurora_terrain.rs`);
the lighting was matched to the client (`client_render.rs`); egui's text
editor proved fast enough on the game's largest script
(`crates/mg-ui/tests/editor_perf.rs`); the C++ compiler builds on the
three systems in CI; Aurora's defaults were captured from its wizards
(`aurora_new.rs`, `aurora_blueprints.rs`). What is open today is in the
[deferred list](deferred.md).
