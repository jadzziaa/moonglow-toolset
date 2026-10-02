# Deferred: what was left out, for a decision later

Things each piece of the post-parity work (`PLAN.md`, "After parity")
left undone, collected in one place. Once the map's current items are
worked through, each gets a verdict: fix, add, or drop. New gaps are
added here as work finishes.

Size: S (an hour or two), M (a day or so), L (several days).

## Script intelligence (`mg lsp`, the script editor)

- **Struct members:** go to definition, references and rename don't
  follow `p.nX` to the struct's declaration. (M)
- **Incremental compiles:** Compile All recompiles everything. It could
  compile only what changed and the scripts that include it, as Arelith's
  ARE_Compile does. At 4,000 scripts a full compile is 1.4 s, so this
  matters less than expected. (M)
- **Editor setups:** the manual's Neovim and Helix snippets haven't been
  tried in those editors. On Windows, `mg lsp` is checked only by CI's
  tests. (S)
- **A VS Code extension:** today VS Code needs a generic language-server
  extension pointed at `mg lsp`. A small Moonglow extension would make
  that one click. (M)

## nasher projects

- **NWNT projects:** projects using NWNT instead of JSON for GFF files
  are refused with a message. (M)
- **Compiled scripts without source:** a module whose scripts exist
  only as `.ncs` loses them when it becomes a project, as with nasher.
  Moonglow names them. It could keep such `.ncs` files in the project
  instead. (S)

## Content doctor

- **The walkmesh crash rule:** nwn.wiki names "walkmesh outside the
  tile" as an Aurora crash cause, but the game's own tiles reach 10 m
  out and work. The real cause is unknown, so there's no check.
  Finding it would mean crashing Aurora on purpose with test tiles. (M)

## Test loop

- **Choose Character:** Test Module, Choose Character uses the game's
  `+LoadNewModule`, taken from nwn.wiki. It hasn't been run in the game
  client. (S)
- **Aurora's F9 problems:** laggy combat, AI timing errors and damaged
  modules after a crash. Moonglow launches the game the way the wiki
  recommends to avoid them, but nothing has checked that they're
  gone. (S–M)

## Persistent-world scale

- **Real CEP haks:** the test world's 150,000 hak files are tiny stand-ins.
  Loading real CEP content at scale isn't measured; the campaign budgets
  and model corpus tests cover real models. (S, needs CEP downloaded)
- **Other machines:** budgets were measured only on the development
  machine (Ryzen 7 5800X3D, Radeon RX 9070). (S, needs other hardware)
- **Inventory lists:** a store page or inventory lays out every row and
  builds every item icon the first time it's shown. 449 icons take
  0.68 s; a page of thousands would take seconds. Laying out only the
  rows on screen would fix it. (S–M)
- **Big palettes, all shown:** with 8,000–10,000 blueprints shown at once
  a palette frame takes 13–16 ms. That's within budget, but skipping
  off-screen rows would make it about 1 ms. (S)
- **Where-used at scale:** one Find References takes 0.8 s in the
  persistent world, since it scans the module each time. An index kept
  up to date would make it instant. (M)

## EE fields Aurora hides

- **Custom shader effects:** shader parameters and the extra area flags
  are saved and the game reads them. Moonglow's renderer doesn't run
  custom shaders, so they don't change what the area view shows. (L)
- **Item costs:** Additional Cost keeps Aurora's limit. The game uses the
  stored Cost, which Moonglow computes, so nothing seemed to need it. (S
  to check)
- **The Classes page in a narrow window:** the second domain picker
  sits at the window's edge. (S)

## Release and packaging (v0.1.0)

- **macOS:** the app and disk image are built by CI but have never been
  run on a Mac. (S, needs a Mac)
- **Windows signing:** the installer isn't code-signed, so SmartScreen
  warns. (S, needs a certificate)

## From the research notes

- **KTX textures:** `mg-image` doesn't read KTX yet (`PLAN.md` §4). (M)
- **Tile Properties' Defaults:** not yet compared with Aurora for a
  lighting scheme whose colors aren't black
  (`docs/research/notes_tilesets.md`). (S)
- **The manual's tables:** the built-in manual viewer can't wrap a
  table's cells, so chapters use lists where a table would read
  better. (S–M)
