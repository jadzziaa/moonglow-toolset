# Moonglow Toolset: Plan

Moonglow is a from-scratch, cross-platform reimplementation of the Neverwinter
Nights: Enhanced Edition Aurora Toolset (`nwtoolset.exe`), written in Rust with
an egui interface. The first goal is **functional parity with Aurora**: every
module Aurora can author, Moonglow can author, and every module Moonglow saves
behaves the same in the game. Byte-identical output is not a goal; functional
regressions are not allowed.

Decisions taken (2026-09-30):

| Topic | Decision |
| --- | --- |
| Language / UI | Rust (stable), egui (+ egui_dock) |
| Renderer | wgpu, with Moonglow's own shaders approximating EE's lighting closely enough to look like the game client (§5.5) |
| Platforms | Linux, Windows, macOS from the start |
| License | GPL-3.0 (§8) |
| Oracles | Files (bundled modules, neverwinter.nim, nwn.py), Aurora under Wine, the game itself (nwserver/nwmain with a separate test user directory) |

## Status

| Phase | State |
| --- | --- |
| 0 Bootstrap | Done: workspace, lint/format, CI workflow (3 OS), corpus locator, oracle tools, engine test runner, Aurora harness, parity inventory. Builds clean for Linux, Windows and macOS targets. |
| 1 Core formats | GFF, ERF (V1.0 + EE E1.0 read), KEY/BIF, 2DA, TLK, SET, SSF done; textures, TXI and MTR done in Phase 7, ITP with palettes |
| 2 Resource manager and rules | Done: load order and game data (2DA cells, talk-table strings) checked against the engine |
| 3 Schema and module workspace | Done except Aurora's blueprint defaults (Phase 8; module and area defaults done in Phase 5): typed field descriptors for all 19 authored GFF types, module workspace (archives and folders, atomic save), reference graph, verify (missing/unused), hak conflict report, ERF export/import; all checked in the engine |
| 4 Script back end | Done: compiler built in (33,196 scripts byte-identical to `nwn_script_comp`), module compile, NWScript front end (lexer, outline, nwscript.nss spec) checked against the compiler's debug output and nwn.py |
| 5 Application shell | In progress: undoable module workspace (`mg-edit`), the app (`mg-ui`, `apps/moonglow`): menus and shortcuts, module tree, docked tabs, module properties, script editor (highlighting, compile), generic GFF editor, message log, New Module and Area wizards with Aurora's defaults, unsaved-changes prompt, settings and recent modules, options, import/export, resource browser; UI flows tested with `egui_kittest`. Exit met: a module made from nothing runs in the engine |
| 6 Text-and-tree editors | Done: String Edit, Variables, resource pickers; Module Properties with Aurora's tabs; Faction, Journal and Conversation editors and the script editor's tools, each checked against Aurora's files (captured under Wine) and in the engine; parity checklists in `docs/parity/checklists.md` |
| 7 Renderer and model viewer | Done: textures (every TGA, DDS and PLT in the game decodes; a sample matches Pillow pixel for pixel), TXI and MTR; models (all 32,832 in the game read, binary and ASCII; the two readers agree through nwnmdlcomp on a sample of 176); the wgpu renderer with the game's lighting (its uniforms read back from the client and matched; reference scenes within 1/255 of the client per region), material maps, environment and cube maps, animations with supermodels, GPU skinning, animated and dangly meshes, emitters (fountain, explosion, point-to-point, lightning, chunks); blueprint previews (`mg-preview`); the model viewer (Resources: open any model or preview a blueprint) |
| 8 Blueprints and palettes | In progress: standard and custom palettes (custom ones rebuilt like Aurora's), the palette pane (edit, edit copy, delete, preview), and editors for every blueprint type (waypoint, sound, trigger, encounter, store, door, placeable, item with item properties, creature with statistics, classes, skills, feats, spells, special abilities and inventory); item costs as the engine computes them (every base-game item agrees with `nwserver`); every blueprint type, edited and written by Moonglow, spawned or placed in the engine; blueprint wizards matching Aurora's. Still to do: the creature CR calculation (needs more Aurora probes), the Creature and Levelup Wizards and creature templates, colour pickers and icons |
| 9 Area editor, objects | Exit met: every shipped area (1,462) opens and renders (tiles with lights and animation loops, objects, fog measured in the client); the area viewer with Aurora's camera and selection bindings, filters, grid, placing from the palette (doors on door hooks), moving, turning, raising, deleting, copy and paste, polygons, Adjust Location with visual transforms, Find Instance, Area Properties, Properties of placed objects (alone or several together, with their inventories), the context menu's commands (Add to Palette, Create Waypoint and Set, initial states, spawn points, …), the Preview window. What placing writes, Add to Palette, Create Waypoint and Adjust Location are identical to Aurora's (three captures); edits round-trip and load in the engine. Still to do: skybox and grass rendering, Add Popup Text, Setup Store, the Levelup Wizard |
| 10 Area editor, terrain | Exit met: painting as Aurora paints, worked out from scripted Aurora sessions (no source) and replayed: 62 recorded steps in four tilesets (terrain brushes with the primary rules on all eight neighbours in relative heights, raise and lower with neighbours settling within a step, crossers on the quarter-cells a drag passes, the eraser clearing crossers outward, refused strokes, tile groups turned and placed, the doors their hooks bring) give Aurora's corners, heights and crossers at every step; Resize Area and Rotate Area give Aurora's tiles and doors; painted areas load in the engine with the ground where the strokes put it. The area viewer's terrain mode (the tileset palette, brushes with a cursor that turns red where a stroke would be refused, groups with right-click turning), tile selection with Delete and next variant, Tile Properties (lights, animation loops), the walkmesh overlay and Area Statistics. Still to do: copying and pasting tiles, Shift + click cycling while painting |
| 11+ | Not started |

What the tests establish so far (all run in a few seconds, in release mode):

- All 57 shipped modules and haks index; all 27,179 GFFs inside them and every
  GFF among the ~114,000 base-game resources round-trip to identical trees;
  rewritten archives keep every entry byte for byte.
- 360 sampled GFFs of every type agree with neverwinter.nim's `nwn_gff` in
  both directions (its JSON of the file, and the GFF it writes from ours).
- All 822 shipped 2DAs parse and agree with `nwn_twoda`, except where the
  engine proves `nwn_twoda` wrong (below).
- All 10 installed talk tables (every language, masculine and feminine)
  round-trip.
- For ~600 resources, placed so that every precedence boundary is crossed
  (development over haks, user haks over install haks, haks over the module,
  the module over override, override over the keys), Moonglow's resman picks
  the same source as the engine's `ResManGetAliasFor`.
- Every shipped module (28), with every GFF rewritten by Moonglow and saved
  as an archive and as a folder, reopens identical; run in `nwserver`, the
  rewritten modules present exactly the same world as the originals (every
  area, and 121,287 objects with type, tag, blueprint, position and facing;
  creatures by identity).
- Verification: of 4,943 references checked across all shipped modules, the
  989 Moonglow reports missing are unknown to the engine and the sampled
  satisfied ones resolve there. An area exported from Prelude (92 resources
  with dependencies) and imported into another module presents the same 22
  objects in the engine.
- The built-in compiler (Beamdog's, vendored and built from source) compiles
  all 33,196 scripts in the base game and the shipped modules (with their
  haks) byte for byte as `nwn_script_comp` does.
- For all 33,121 compilable shipped scripts, the outline of the script and
  its includes names exactly the functions (with types), constants and
  structs in the compiler's debug output; the spec parser agrees with
  nwn.py's (which misses 78 constants with lowercase names).
- All 47 shipped tilesets parse (1,192 primary rules between them; 73 data
  warnings, such as tiles using undeclared terrains, and 5 tile models BioWare
  never shipped); all 487 soundsets round-trip byte for byte.

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
  placement and editing (creatures, doors, encounters, items, merchants,
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
   tested (§7).
7. **Interoperable.** GFF JSON compatible with neverwinter.nim and nasher, so
   Moonglow projects can live in git alongside existing tools.
8. **The engine is the reference.** Where tools disagree, an engine test
   decides (§7).
9. **Portable.** Platform differences (game/user directory detection, launching
   the game, file dialogs) live behind small interfaces; CI builds and tests all
   three platforms.

## 4. Architecture

A Cargo workspace, layered bottom-up. A crate may only depend on crates above it
in this list. ✅ = exists.

| Crate | Responsibility |
| --- | --- |
| `mg-core` ✅ | ResRef, ResType table, languages and codepages, StrRef, LocString, bounds-checked binary reader/writer |
| `mg-gff` ✅ | GFF V3.2 read/write, lossless tree, typed accessors, nwn-lib JSON, structural diff |
| `mg-erf` ✅ | ERF/MOD/HAK/NWM/SAV: read V1.0 and E1.0 (zstd), write V1.0 |
| `mg-key` ✅ | KEY/BIF V1, memory-mapped |
| `mg-2da` ✅ | 2DA V2.0, lenient like the engine, number parsing like the engine |
| `mg-tlk` ✅ | TLK V3.0, custom TLK StrRefs |
| `mg-set` ✅ | tilesets (SET/INI) with data warnings |
| `mg-ssf` ✅ | soundsets |
| `mg-resman` ✅ | layered resolution in the engine's order, DDS/TGA rule, install detection |
| `mg-image` ✅ | TGA, DDS (standard and BioWare, BC1–BC5), PLT and palettes, TXI, MTR (KTX: not yet) |
| `mg-mdl` (partial) | MDL binary and ASCII (done), WOK/PWK/DWK walkmeshes |
| `mg-schema` ✅ | typed views of IFO, ARE, GIT, GIC, UTC/UTD/UTE/UTI/UTM/UTP/UTS/UTT/UTW, DLG, JRL, FAC, ITP; Aurora defaults |
| `mg-rules` ✅ | 2DA/TLK-backed game data (appearance, baseitems, classes, feats, spells, skills, item properties, placeables, doors, portraits, sounds, ...) |
| `mg-script` | compiler integration; NWScript lexer/parser/symbol index for the editor; nwscript.nss spec |
| `mg-tiles` (partial) | tilesets, tile grid model, terrain painting engine with SET rules, walkmesh assembly; so far the corner lattice, tile fitting and new-area terrain |
| `mg-module` ✅ | module workspace: open/save, working copy, palettes, reference graph, verify, build, import/export, haks/TLK |
| `mg-edit` | editor core: documents, commands, undo/redo, selection, tools (no UI) |
| `mg-render` (partial) | renderer: scene, materials and maps, EE lighting (matched to the client), offscreen mode, animation, skinning, animated and dangly meshes, particles (done); picking, gizmos |
| `mg-preview` ✅ | blueprint previews: part-based and single-model creatures with equipment, wings and tails, PLT colours; items; placeables; doors |
| `mg-area` (partial) | areas as the area editor shows them: tiles with their lights and animation loops, placed objects with their previews and outlines, sun, moon and fog; the renderer's scene of an area |
| `mg-ui` | egui widgets and editors |
| `apps/mg` ✅ | CLI: `ls`, `pack`, `unpack`, `gff`, `which`, `cat`, `layers`, `tlk` so far; later build, verify, compile, diff, render |
| `apps/moonglow` | the GUI application |

Test support: `mg-testkit` ✅ (corpus locator, oracle tools, engine runner),
`mg-corpus-tests` ✅ (corpus, differential and engine tests). Tools:
`tools/aurora/` ✅ (Aurora under Wine, on the desktop or on an off-screen display driven by
scripts; form decoder, inventory generator, wizard capture).

## 5. Key designs

### 5.1 Lossless typed objects

`mg-gff` stores a GFF as a tree of structs (type id + ordered fields, duplicate
labels allowed). `mg-schema` defines each object type as a thin typed wrapper
generated by a derive macro: getters return the field or Aurora's default,
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
hak listed before it; the `engine_resman` test crosses every one of these
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
platform. Optimisation flags are set to 1 (dead code only), as the game and
Aurora use; it reports the first error only, as Aurora does. `nwn_script_comp`
stays the test oracle. Editor features (highlighting, outline, completion,
go-to-definition, signature help, diagnostics) come from Moonglow's own
NWScript front end in `mg-script`, fed by `nwscript.nss` from the resman.

### 5.5 Renderer

One wgpu renderer serves the area editor, blueprint previews and the model
viewer. Aurora shows none of EE's lighting, so the goal is not bit-exact parity
with the game but a close match to what the game client shows: Moonglow's own
WGSL shaders implement EE's lighting model as the research brief derives it
from the stock shaders (`docs/research/ee_tech_brief.md` §6,
`notes_shaders.md`): linear lighting with gamma correction, per-fragment
lighting from area sun/moon ambient and diffuse, tile main and source lights
(`lightcolor.2da`), placeable and model lights (top 32 per object by priority
and distance), GGX specular, MTR materials with normal, specular, roughness,
height and self-illumination maps, environment maps, fog, skyboxes; later grass,
stencil shadows, emitters and water. Editor overlays (grid, gizmos, selection,
trigger polygons, walkmesh, AABB) render in a separate pass, and toggles like
Aurora's "use area lighting" and day/night stay. The renderer also runs
offscreen for image tests (L5): reference scenes are compared with screenshots
of the same area, camera and time of day taken in the real client (`nwmain`
with a test user directory), within a tolerance.

Running the game's own shaders is kept open as a later option: a spike
(2026-09-30) showed the stock `vslit`/`fslit` pair, assembled with the engine
preamble and verbatim `#include` splicing (`tools/shaders/assemble.py`),
compiles to Vulkan SPIR-V with glslang's relaxed rules, and naga accepts it
after SPIRV-Tools splits the combined image samplers and the resource bindings
are rewritten. That would bring hak custom shaders and MTR
`customshaderVS/FS` along, at the cost of native glslang/SPIRV-Tools
dependencies.

### 5.6 Tile painting

`mg-tiles` models the area as a lattice of corner terrains and heights plus edge
crossers. Painting chooses, for each affected cell, a tile and orientation from
the SET whose corners and edges match its neighbours; where no tile provides a
transition, the SET's `[PRIMARY RULES]` (placed/adjacent → changed terrain and
height) rewrite neighbouring corners and the change propagates. Aurora's exact
choices (tie-breaking, randomness, propagation extent) are pinned down by
recording Aurora under Wine on scripted painting sequences and replaying them
against Moonglow (§7, L4), starting early so fixtures exist before Phase 10.

## 6. Phases

Each phase ends with a demonstrable result and green tests. Later phases do not
start until the earlier layer they depend on is tested against the corpus.

### Phase 0: Bootstrap ✅
Workspace, lint/format config, CI matrix, corpus locator, test tiers, project
`CLAUDE.md`, Aurora oracle harness, engine test runner, parity inventory.

### Phase 1: Core formats
✅ GFF, ERF, KEY/BIF, 2DA, TLK with corpus and differential tests; first CLI.
To do: SSF, SET, ITP, TXI, MTR (each with corpus tests).

### Phase 2: Resource manager and rules data ✅
`mg-resman` and `mg-rules` (cached 2DAs, talk tables incl. custom TLK,
dropdown choices), checked against the engine's `ResManGetAliasFor`,
`Get2DAString` and `GetStringByStrRef`. Accessors for individual tables arrive
with the editors that use them.

### Phase 3: Typed schema and module workspace (core ✅)
✅ `mg-schema`: typed field descriptors for every authored GFF type, generated
from the schema observed in the shipped data
(`docs/research/gff-observed-schema.md`); ✅ `mg-module`: archives and module
folders, atomic save with backup, module info (areas, haks, custom TLK).
✅ reference graph, verify (missing and unused resources), hak conflict
report, ERF export/import with dependencies, faction reset and area-list
bookkeeping; CLI `mg verify`, `mg haks`, `mg export`, `mg import`.
Deferred to the wizards (Phases 5 and 8): capturing Aurora's defaults for new
objects. Original scope: `mg-schema` for all authored GFF types with Aurora's defaults (captured from
Aurora's wizards), `mg-module`: open/save `.mod` and module folders, working
copy, atomic save with backup, import/export ERF, haks and custom TLK (with the
conflict check), reference graph, basic verify.
**Exit:** open → save of every bundled module is semantically identical, and the
saved modules load and run their start-up in `nwserver`.

### Phase 4: Script back end ✅
✅ The official compiler built in (`mg-script`: vendored C++ built with the
`cc` crate, C API, per-thread instances), module compile (`mg_module::build`,
`mg compile`); ✅ NWScript front end for the editor: a lexer that covers
every byte (EE literals: raw and hashed strings, binary and octal), a
tolerant declaration parser (includes, defines, structs, globals, functions
with docs) and the `nwscript.nss` spec (1,187 functions, 6,279 constants,
engine structures). Remaining: building the C++ on Windows and macOS in CI.
**Exit:** ✅ compiling every script in the base game and the shipped modules
gives the same NCS as `nwn_script_comp` (the shipped NCS match only where the
same compiler produced them: 68%).

### Phase 5: Application shell
egui app with a docked layout like Aurora's (module tree, tabbed editors,
palette, message log, status bar), menus, toolbars, shortcuts, options, game path
setup, TLK-driven labels, new-module wizard, module properties, haks/TLK,
import/export, resource browser, undo/redo wiring. Early spike: a code-editor
widget that stays fast on 10k-line scripts.
**Exit:** create a module, edit its properties, save; it loads in the engine;
`egui_kittest` UI tests cover the flows.

Done so far: `mg-edit` (a workspace of cached GFF documents over the module;
commands of field/list/resource edits, each with its inverse, so undo and redo
restore every change, including unsaved GFF edits under a whole-resource
replacement); `mg-ui` with the menu bar (File, Edit, Build), shortcuts
(Ctrl+O/S/Z/Y, F7), the module tree with filter, docked tabs (welcome, module
properties, scripts, any GFF resource), module properties (name, tag,
description with its line ends kept, starting area, XP scale, custom TLK,
time, events, haks in priority order, areas), the script editor (lexer
highlighting, save, compile one, compile all), verify to the log;
`apps/moonglow` (eframe on wgpu, native file dialogs). `egui_kittest` tests
drive open, edit, undo/redo, save and reopen through the accessibility tree.
New modules and areas (`mg_module::new`, on `mg-tiles`) are made as
Aurora's Module and Area wizards make them, captured by driving Aurora on an
off-screen display (`tools/aurora/headless.sh`, `xdrive.py`,
`capture_new_areas.py`): module info, the five default factions, custom
palettes derived from the game's palette skeletons (categories sorted with
Windows' word sort, engine-only categories dropped), and for areas the
`areag.ini` defaults, `environment.2da` lighting and weather, and tiles
fitted to `Default` terrain with a `Floor` patch at the centre (group tiles
excluded, variants, orientations and lights at random as Aurora does). All
44 captured areas (every tileset, sizes 2 to 8, rectangles) match Aurora's
field for field except the random choices, which are checked to be among
Moonglow's candidates; a module made by Moonglow with an area per tileset
runs in `nwserver`, which reports every area property and all tiles as
written. Where no tile fits (a 5×5 Lizardfolk Interior) Aurora crashes;
Moonglow reports it. File > New Module, Wizards > Area Wizard (one undoable
command) and the unsaved-changes prompt are in the UI with tests.
Also done: settings kept between runs (recent modules, game and user
folders; Tools > Options), File > Import/Export (with dependencies, overwrite
choices, one undoable import), and the resource browser (all ~114,000
resources of the load order with their layer; read-only GFF, 2DA, script and
text views; copy into the module, save to a file). The off-screen display
also runs Moonglow itself for visual checks.
The script editor spike is done (see §9: egui's editor is fast enough on
the game's largest script), and a toolbar gives the common commands. Next:
Phase 6.

### Phase 6: Text-and-tree editors
Localized string editor, variables, pickers; script editor (highlighting,
compile with error navigation, function browser, completion, search, script
wizard); conversation editor (tree, links, conditions/actions with parameters,
animations, sounds, tokens, test); journal editor; faction editor.
**Exit:** parity checklists for these editors complete; engine tests exercise the
conversations, journal and factions they produce.

Done so far: the shared editors (String Edit with StrRef and every
language, Variables, Select Resource) and Module Properties with Aurora's
tabs; the Faction Editor (`mg_module::factions`: Aurora's files exactly,
GetReputation in the engine for every pair), the Journal Editor
(`mg_module::journal`: Aurora's files exactly, XP by tag in the engine),
the Conversation Editor (`mg_module::dialog`: Aurora's conversation as an
outline; conditions with EE parameters and actions run in the engine), and
the script editor's tools (symbol lists with help, compiler messages that
go to their line, find/replace and find in files, bookmarks, completion,
templates, new script and save as, numbered bookmarks, colour options),
and the Script Wizard
(`mg_module::script_wizard`: Aurora's captured scripts byte for byte, except
where Aurora's output does not compile or work in EE). Conversation spell
check does nothing in Aurora EE and text export/import has no UI, so neither
is needed for parity.

### Phase 7: Renderer and model viewer ✅
wgpu renderer with EE-style lighting (§5.5), textures (TGA/DDS/PLT/TXI), MTR
materials, MDL (ASCII and binary, all node types), animation, skinning, dangly
meshes, emitters, offscreen snapshots; previews for part-based creatures (armour parts, PLT colours), items,
placeables, doors.
**Exit:** reference scenes look like in-game screenshots of the same scene
(within a tolerance set from the first comparisons).
**Exit met:** the reference scenes (`client_render.rs`: sun; tile lights)
match the client within 1/255 per region mean (a floor region on a dark
wall's edge within 5; tolerances 3 and 6), and the light uniforms Moonglow
uploads equal the client's (read back through a debug shader). Known
approximations: dangly-mesh dynamics and the point-to-point and lightning
emitter details follow the wiki's descriptions, not measurements.

### Phase 8: Blueprints and palettes
Standard and custom palettes, blueprint wizards and editors for items (item
properties), creatures (classes, feats, skills, spells, inventory, level-up
wizard), placeables, doors, triggers, encounters, sounds, stores, waypoints;
editing, copying, deleting custom blueprints and updating instances.
**Exit:** parity checklists complete; engine tests spawn every blueprint type.

### Phase 9: Area editor, objects
Area wizard, tileset loading, area rendering (day/night, tile lights, fog,
skybox, grass), camera, area properties, placing/selecting/moving/rotating all
object types, visual transforms, trigger and encounter polygons, filters, grid,
preview window, multi-edit, copy/paste, find instance.
**Exit:** every official area opens and renders; object edits round-trip and
load in the engine.

### Phase 10: Area editor, terrain ✅
Tile painting with SET rules (terrain, crossers, groups, heights, rotation), tile
properties (lights, animation loops), resize/rotate, door hooks, walkmesh
overlays, area statistics.
**Exit:** recorded Aurora painting scenarios produce the same tile grids.
**Exit met:** `aurora_terrain.rs` replays 62 recorded steps (54 scripted by
`tools/aurora/capture_terrain.py`, 8 group placements by hand) and four
resizes and rotations; every step gives Aurora's lattice; `engine_terrain.rs`
loads a painted area in the engine.

### Phase 11: Build, verify, test
Build module (compile, palette regeneration, bookkeeping), verify module, test
module (launch the game with the module), plot wizard, the remaining dialogs.
**Exit:** the parity checklist is complete.

### Phase 12: Hardening and release
Performance budgets on the largest campaigns, crash safety (autosave, recovery),
packaging (AppImage/Flatpak, Windows installer, macOS app bundle), user manual.

### After parity
Candidates, once the baseline is done: git-friendly project folders (nasher
layout), an NWScript language server, NWSync publishing, multi-module projects,
batch refactoring (rename a resref everywhere), live game preview.

## 7. Testing strategy

| Tier | What | Runs |
| --- | --- | --- |
| L0 unit | per-crate tests, property tests (proptest) for readers/writers, truncation tests | always |
| L1 corpus | parse and round-trip every resource in the install and bundled modules | when a game install is found |
| L2 differential | compare against neverwinter.nim (GFF JSON, 2DA; later ERF, TLK, NCS bytes) | when its tools are installed |
| L3 engine | build a module, run it in `nwserver` with a scratch user directory; a probe script logs what the engine sees (`Get2DAString`, `ResManGetAliasFor`, object state) | Linux, when the game is installed |
| L4 Aurora parity | scripted scenarios in Aurora under Wine produce reference outputs; Moonglow must match them semantically | on demand; recorded fixtures replayed always |
| L5 image | offscreen renders vs golden images (SSIM threshold); periodic comparisons with game screenshots | GPU runners / locally |
| L6 UI | `egui_kittest` flows through editors and dialogs | always |

Every feature lands with tests at the lowest tier that can catch its
regressions, plus a line ticked in the parity checklist that names the test.

When an oracle tool disagrees with Moonglow, an engine test decides and the
divergence is recorded (see `CLAUDE.md`). So far: `nwn_twoda` drops `""` 2DA
cells, but the engine reads them as empty cells in place (Beamdog's own
`id_resources.hak` depends on it); `nwn_twoda` crashes on tables without
non-empty rows; `nwn_gff` cannot read back JSON with an empty field label, which
BioWare's own palettes contain.

Performance budgets (checked in CI on Linux): index the base install, open the
largest official campaign module, open and render its largest area, save it.
Budgets are set from the first measurements and only tightened.

## 8. Licensing

Moonglow is licensed under GPL-3.0 (`LICENSE`; `GPL-3.0-only` in the crate
manifests). That allows building in the official NWScript compiler (GPL-3.0)
and depending on GPL code where it helps (e.g. nwnrs for MDL/MTR/SET
cross-checks, xoreos and Neverblender as references). Beamdog's shaders and all
game assets are loaded from the user's install and never redistributed.

## 9. Risks

| Risk | Mitigation |
| --- | --- |
| Tile painting behaviour is undocumented | SET rules plus recorded Aurora scenarios (L4) from early on |
| EE lighting looks off | Model derived from the stock shaders; compared with client screenshots (L5); stock-shader translation as a fallback (§5.5) |
| egui text editing on large scripts | Resolved (Phase 5 spike): with the laid-out text cached between frames, `nwscript.nss` (13,869 lines) costs 0.4 ms per idle frame and ~10 ms per keystroke (190 ms to open, 140 ms for the first edit); `crates/mg-ui/tests/editor_perf.rs` |
| Building the C++ compiler on all platforms | Plain C++ with no dependencies; CI builds it on all three; `nwn_script_comp` as a fallback |
| Aurora defaults we cannot see | Capture them from Aurora's wizards into fixtures |
| Scope | Parity checklist drives order; editors ship without previews until Phase 7 lands |
