# Moonglow Toolset

A from-scratch, cross-platform (Linux, Windows, macOS) reimplementation of the
Neverwinter Nights: Enhanced Edition Aurora Toolset in Rust (egui UI, wgpu
renderer), licensed GPL-3.0. The plan, architecture and phase list are in
`docs/PLAN.md`; the Aurora parity checklist is
`docs/parity/aurora-ui-inventory.md`.

## Commands

```sh
cargo test --workspace                          # unit tests (corpus tests skip without a game install)
cargo test -p mg-corpus-tests --release         # corpus, differential and engine tests
cargo clippy --workspace --all-targets          # must be warning-free
cargo fmt                                       # rustfmt.toml: width 100, "Max" heuristics
```

Corpus tests read the installed game: `NWN_ROOT` (or the Steam default path)
and neverwinter.nim's tools as oracles (`NWN_TOOLS_BIN`, default
`~/.local/opt/neverwinter/bin`). `MOONGLOW_REQUIRE_CORPUS=1` turns skips into
failures.

## Layout

Cargo workspace, layered bottom-up (a crate depends only on crates listed
before it in `docs/PLAN.md` §4): `crates/mg-core` (ResRef, ResType, languages,
LocString, binary helpers), `mg-gff`, `mg-erf`, `mg-key`, `mg-2da`, `mg-tlk`,
... `mg-testkit` (corpus locator, oracle tools, engine runner) and
`mg-corpus-tests` (tests only). `tools/aurora/` holds the Aurora oracle
harness and the form (DFM) decoder; `docs/research/` the research briefs
(EE formats, rendering, tilesets, models, shaders, prior art).

## Rules

- **Never write to the real NWN user folder** (`~/.local/share/Neverwinter
  Nights`) or the game install. Engine tests use scratch user directories under
  `target/test-output/` (`mg_testkit::engine`); the Aurora oracle runs in its
  own Wine prefix and user directory (`tools/aurora/run-aurora.sh`, state in
  `~/.local/share/moonglow-oracle`).
- **Test servers stay private.** Run `nwserver` only through
  `mg_testkit::engine::run_server`, which passes `-publicserver 0` and random
  passwords; without them the server registers on Beamdog's public server
  list under the user's IP.
- **Game data is never committed.** Tests read the user's install; fixtures in
  the repo contain only data we author.
- **Lossless editing.** Readers keep everything (unknown GFF fields, duplicate
  labels, odd entries) and typed layers edit the raw tree in place, so an
  untouched file round-trips.
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
