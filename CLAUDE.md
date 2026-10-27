# Moonglow Toolset

A from-scratch, cross-platform (Linux, Windows, macOS) reimplementation of the
Neverwinter Nights: Enhanced Edition Aurora Toolset in Rust (egui UI, wgpu
renderer), licensed GPL-3.0. The architecture and the record of the phases are in
`docs/PLAN.md` (what is open: `docs/deferred.md`); the Aurora parity checklist is
`docs/parity/aurora-ui-inventory.md`.

## Commands

```sh
cargo test --workspace --exclude mg-corpus-tests --lib --bins --tests   # unit, UI and command-line tests
cargo test -p mg-corpus-tests --release --tests                         # corpus, differential and engine tests (skip without a game install)
MOONGLOW_FULL_CORPUS=1 cargo test -p mg-corpus-tests --release --tests    # the same over the whole install: before a release
cargo clippy --workspace --all-targets          # must be warning-free
cargo clippy -p mg-ui --test screens --test frame_perf --test world_perf --test campaign_perf --test editor_perf --test nui_perf   # the tests run by hand, which --all-targets leaves out
cargo clippy -p mg-corpus-tests --test client_render --test client_minimap --test client_nui
cargo fmt                                       # rustfmt.toml: width 100, "Max" heuristics
```

The first two test commands are the whole suite, each test once: a plain
`cargo test --workspace` also runs the corpus tests a second time,
unoptimized, and links the 22 examples (documentation tests, as CI runs
them: `cargo test --workspace --doc`).
The corpus tests' sweeps (every area, model, script, sound, texture and
module of the install) take a sample on an everyday run (under a minute)
and everything with `MOONGLOW_FULL_CORPUS=1` (four minutes), which a
release is tested with: `mg_testkit::sweep` picks, `at_least` scales the
counts a sweep must reach. A new sweep over the install goes through
them. Test files are grouped into a few binaries (`tests/groups/` in
`mg-corpus-tests` and `apps/mg`): a new test file is named in its group,
and one group runs with `--test engine` (a file's tests: `--test engine
engine_tlk`). mg-ui's UI tests are one binary, `--test app`, a file a
subject in `crates/mg-ui/tests/app/` (a new file is named in its
`main.rs`), with the shared set-ups in `app/common.rs`.

Corpus tests read the installed game: `NWN_ROOT` (or the Steam default path)
and neverwinter.nim's tools and nasher as oracles (`NWN_TOOLS_BIN`, default
`~/.local/opt/neverwinter/bin`), plus nwnmdlcomp for models (also found in
`~/Projects/neverblender/tools/bin`) and Pillow (`python3`) for textures.
`MOONGLOW_REQUIRE_CORPUS=1` turns skips into failures. `MOONGLOW_HAKS`
(folders, separated as `PATH` is) adds a builder's haks to those the
tests take as installed: the shader test's baseline is 102 of 102 with
`~/Projects/tdod-module/build/hak`.

## Layout

Cargo workspace, layered bottom-up (a crate depends only on crates listed
before it in `docs/PLAN.md` §4): `crates/mg-core` (ResRef, ResType, languages,
LocString, binary helpers), `mg-gff`, `mg-erf`, `mg-key`, `mg-2da`, `mg-tlk`,
... `mg-shader` (the game's `.shd` programs assembled, compiled by glslang
and read into naga modules; Verify's `shader-compile`), ... `mg-edit`
(undoable workspace), `mg-nui` (NUI windows: the JUI document,
its checks and the generated scripts), `mg-plugin` (the plugin host: manifests
and the sandboxed Luau runtime; the API is documented in
`docs/manual/16-writing-plugins.md` and `17-plugin-api.md`, and
`crates/mg-plugin/tests/examples.rs` holds those and the examples in
`docs/plugins/` to the code), `mg-render` (wgpu renderer), `mg-preview`
(blueprint previews: what a creature, item, placeable or door looks like),
`mg-serve` (runs the game's headless server on a module: `mg serve`,
`mg test`, the engine tests), `mg-ui` (egui app; UI tests with
`egui_kittest` in `crates/mg-ui/tests`; to look at a window's layout,
render it with `tests/screens.rs`: `cargo test -p mg-ui --test screens --
--ignored` writes PNGs to `target/test-output/screens/`), `mg-testkit` (corpus locator, oracle
tools, engine runner) and `mg-corpus-tests` (tests only). Binaries:
`apps/mg` (CLI) and `apps/moonglow` (the GUI). `packaging/` builds the release packages
(AppImage, Flatpak, Windows installer, macOS app; `packaging/README.md`) and
`docs/manual/` is the user manual, also built into the app (Help › User
Manual). `tools/aurora/` holds the Aurora oracle
harness and the form (DFM) decoder; `docs/research/` the research briefs
(EE formats, rendering, tilesets, models, shaders, prior art, builders' pain points).

To drive Aurora (capture what it writes), run it on the off-screen display,
never on the user's desktop: `D=$(tools/aurora/headless.sh start)`, then
`DISPLAY=$D tools/aurora/run-aurora.sh` and `DISPLAY=$D tools/aurora/xdrive.py
shot|click|key|type|windows`; `tools/aurora/headless.sh stop` when done.
Captures that tests compare against go to `~/.local/share/moonglow-oracle/captures/`
(`mg_testkit::aurora_capture!`); like game data, they are not committed.

The game client is the renderer's oracle (`crates/mg-corpus-tests/tests/client_render.rs`,
ignored: `DISPLAY=:1 cargo test -p mg-corpus-tests --test client_render -- --ignored`).
It starts a scratch module with `+TestNewModule` and screenshots the client's
window. To read what the client actually computes, put a patched copy of the
game's own shader include in the scratch user directory's `override` (as
`light_uniforms_match_the_client` does): no tracing tools needed.

## Documents

`docs/` is an Open Knowledge Format (OKF v0.2) bundle (the user-level `okf`
skill): start at `docs/index.md`. Every document opens with frontmatter
(`type`, `title`, `description`, `generated`, `sources`...), and `okf lint
docs --links` stays clean. After adding or changing one: `okf index docs`
and an entry in `docs/log.md`. Manual chapters carry frontmatter too; the
app shows them without it (`mg-ui/src/manual.rs`, whose tests require it).
The two generated documents get theirs from their generators
(`tools/aurora/uiinv/gen.py`, after `inputs.py` has made its inputs from the
installed game; `crates/mg-corpus-tests/examples/observed_schema.rs`).

## Rules

- **Never write to the real NWN user folder** (`~/.local/share/Neverwinter
  Nights`) or the game install. Engine tests use scratch user directories under
  `target/test-output/` (`mg_testkit::engine`); the Aurora oracle runs in its
  own Wine prefix and user directory (`tools/aurora/run-aurora.sh`, state in
  `~/.local/share/moonglow-oracle`).
- **Run the game client only through `tools/nwclient/run-client.sh
  SCRATCH`**, on the off-screen display: a bubblewrap sandbox with a
  read-only system, Steam hidden (no Steam API, presence or cloud sync), no
  network, D-Bus or audio, and only SCRATCH writable (its user directory is
  SCRATCH/user). It refuses the desktop display :0.
- **Test servers stay private.** Run `nwserver` only through
  `mg_serve::Server::start` with its defaults (which
  `mg_testkit::engine::run_server`, `mg serve` and `mg test` do): it passes
  `-publicserver 0` and random passwords; without them the server registers
  on Beamdog's public server list under the user's IP. Only a user's own
  `mg serve --public` lists one.
- **Game data is never committed.** Tests read the user's install; fixtures in
  the repo contain only data we author.
- **Lossless editing.** Readers keep everything (unknown GFF fields, duplicate
  labels, odd entries) and typed layers edit the raw tree in place, so an
  untouched file round-trips.
- **Tests that open a GPU device call `mg_testkit::gpu::hold()` first**, so a
  test binary has a few devices open at a time (4; `MG_TEST_GPUS` sets how
  many, 1 for one at a time): a test suite with dozens at once coincided
  with a kernel panic on the development machine. A test that changes what
  every render does (`mg_render::guard::fail_on`) takes the GPU alone
  (`gpu::alone()`), and process-wide state a window reads is the window's
  own, or other tests' windows take it.
- **Something that looks wrong in a view is reproduced as a picture
  before it is fixed.** Render the view itself and look at it, before
  and after the change: `crates/mg-ui/tests/screens.rs` for the window;
  for an area's view, a harness with a GPU render state, as
  `area_harness` in `crates/mg-ui/tests/app/common.rs` sets one up; with the
  builder's module and user directory where they are on this machine. A
  simulation of the arithmetic is not the picture: spikes across an
  area's view took three releases because the projection was simulated,
  and the cause was in how the outline was drawn.
- **Parsers never panic on bad input**: bounds-checked reads
  (`mg_core::bin::Reader`), errors with context, and a truncation test per
  format.
- Every format and feature lands with tests at the lowest tier that catches its
  regressions (unit, corpus round-trip, differential against neverwinter.nim,
  engine run, Aurora comparison); see `docs/PLAN.md` §7.
- Match the behaviour of the game engine and Aurora, not of other tools. When
  an oracle tool disagrees, settle it in the engine (`tests/engine_*.rs`) and
  record the divergence where the comparison skips it. Known ones:
  `nwn_gff` keeps fields in hash order and cannot read back JSON with an empty
  label (BioWare palettes have one); `nwn_twoda` drops `""` cells (the engine
  reads them as empty cells in place) and crashes on tables with no non-empty
  rows.
- Moonglow is GPL-3.0: GPL code (the official NWScript compiler, GPL crates)
  may be built in. Game assets, including Beamdog's shaders, are only ever
  loaded from the user's install.
- The renderer is wgpu with Moonglow's own shaders approximating EE lighting
  (validated against client screenshots); exact stock-shader parity is not a
  goal (`docs/PLAN.md` §5.5).
