# Moonglow Toolset

A module toolset for **Neverwinter Nights: Enhanced Edition**, rebuilt from
scratch: a reimplementation of BioWare's Aurora Toolset that runs natively on
Linux, Windows and macOS. Written in Rust, with an [egui](https://github.com/emilk/egui)
interface and a [wgpu](https://wgpu.rs) renderer.

The first goal is **functional parity with Aurora**. Moonglow edits what Aurora
edits and writes what Aurora writes, and every module it saves behaves the
same in the game. Byte-identical output isn't a goal; regressions in behavior
are never allowed.

## Status

Aurora parity is done. Every row of the [parity checklist](docs/parity/checklists.md)
is done, deliberately different, or partly done with the gap named. The
project is now in its hardening and release phase: crash safety, performance
budgets, packaging and the user manual are in place, and the packages are
being tried on each system. See [the plan](docs/PLAN.md) for the phases and
what each one established.

What's there:

- **Areas**: the area viewer with Aurora's camera and selection bindings,
  drawn with the game's lighting, fog and skyboxes (checked against the game's
  own screenshots). Placing, moving, turning and raising objects, copy and
  paste, triggers and encounters drawn point by point, Find Instance, Adjust
  Location, and the context menu's commands.
- **Terrain**: painting with a tileset's terrain, crosser and group brushes,
  raise and lower, the eraser, tile properties, and resizing and rotating
  areas. Aurora's painting rules were worked out from scripted Aurora sessions;
  the same strokes give the same tiles.
- **Blueprints**: standard and custom palettes, and editors for every
  blueprint type with Aurora's pages. Item costs and creature challenge ratings
  are computed as the game and Aurora compute them. The blueprint, Creature and
  Levelup wizards are included.
- **Conversations, scripts, journal and factions**: the Conversation Editor
  with the Script Wizard, and the script editor with Beamdog's own NWScript
  compiler built in. Its output is byte-identical to `nwn_script_comp` on
  33,196 scripts.
- **Build, verify and test**: Build Module, Verify, missing and unused
  resources, and Test Module (F9) in the game.
- **Version control**: modules kept as [nasher](https://github.com/squattingmonk/nasher)
  projects (text files for git) open and save in place, writing exactly
  what nasher writes; `mg build` packs them in a build pipeline.
- **Everything else**: haks and custom talk tables, import and export, sound
  playback, unlimited undo, recovery copies of unsaved work, and a
  command-line tool (`mg`) for archives, GFF and JSON, the game's resources,
  and building modules.

Moonglow contains no game data: it reads the game's files from your
installation, so it needs a copy of Neverwinter Nights: Enhanced Edition.

## Documentation

- [User manual](docs/manual/README.md), also in the app under Help › User
  Manual (F1). If you know Aurora, start with
  [Coming from Aurora](docs/manual/14-coming-from-aurora.md).
- [The plan](docs/PLAN.md): goals, architecture, phases, testing strategy and
  licensing.
- [Packaging](packaging/README.md): building the AppImage, Flatpak, Windows
  installer and macOS app.
- [Research notes](docs/research/): EE file formats, rendering, tilesets,
  models, shaders, prior art, and what builders want changed in Aurora.

## Building from source

You need a stable Rust toolchain (1.98 or newer; `rust-toolchain.toml`
selects it with rustup) and a C++ compiler for the bundled script compiler:

- **Linux**: a C++ compiler, `pkg-config` and the ALSA headers
  (`libasound2-dev` on Debian and Ubuntu, `alsa-lib` on Arch and Fedora).
- **Windows**: Visual Studio's C++ build tools.
- **macOS**: the Xcode command-line tools.

```sh
cargo run --release -p moonglow          # the GUI
cargo run --release -p mg -- --help      # the command-line tools
```

Moonglow finds the game where Steam installs it; otherwise set its folder in
Tools › Options › Folders, or `NWN_ROOT` for the command line.

## Testing

```sh
cargo test --workspace                          # unit tests (corpus tests skip without a game install)
cargo test -p mg-corpus-tests --release         # corpus, differential and engine tests
cargo clippy --workspace --all-targets          # must be warning-free
cargo fmt                                       # rustfmt.toml: width 100
```

Features land with tests at the lowest tier that catches their regressions:

- **Unit tests.**
- **Round trips** over the installed game's files.
- **Differential tests** against [neverwinter.nim](https://github.com/niv/neverwinter.nim)'s
  tools.
- **Engine runs**: a private `nwserver` and a sandboxed game client.
- **Aurora comparisons**: Aurora runs under Wine, off-screen, and its output
  is captured for comparison.

The corpus tests read the game install (`NWN_ROOT`, or Steam's usual path).
`MOONGLOW_REQUIRE_CORPUS=1` turns their skips into failures.

## Layout

A Cargo workspace, layered bottom-up:

| Folder | What |
| --- | --- |
| `crates/mg-core` | ResRef, resource types, languages, localized strings, bounds-checked binary reading |
| `crates/mg-gff`, `mg-erf`, `mg-key`, `mg-2da`, `mg-tlk`, `mg-set`, `mg-ssf` | the game's file formats, lossless |
| `crates/mg-audio`, `mg-image`, `mg-mdl` | sounds, textures and models |
| `crates/mg-resman`, `mg-rules` | the game's load order, and its rules from its 2DA tables and talk tables |
| `crates/mg-schema`, `mg-module`, `mg-edit` | typed views of the authored files, the module workspace, undoable editing |
| `crates/mg-script` | the NWScript compiler and the editor's language support |
| `crates/mg-tiles`, `mg-area`, `mg-render`, `mg-preview` | tile painting, areas, the renderer, blueprint previews |
| `crates/mg-ui` | the egui application |
| `apps/moonglow`, `apps/mg` | the GUI and the command-line tools |
| `crates/mg-testkit`, `mg-corpus-tests` | test support, and the corpus, differential and engine tests |
| `tools/` | the Aurora and game-client harnesses, shader tools |
| `packaging/` | release packages |

## License

Moonglow is free software under the [GNU General Public License, version 3](LICENSE).
It includes Beamdog's NWScript compiler (GPL-3.0), as published in
neverwinter.nim. Game assets, including Beamdog's shaders, are only ever read
from your installation and never distributed.

Neverwinter Nights is a trademark of its owners. Moonglow is not affiliated
with Beamdog or Wizards of the Coast.
