---
type: Proposal
title: 'A placed object that blocks as it is drawn: a design'
description: How Moonglow could give one placed placeable a walkmesh that follows its visual transform (a large stone scaled down to sit on a desk blocks as a small one), which the game never does by itself - the engine's rule (settled), what the bake makes (a copy of the model under a new name, its walkmesh transformed as the game draws the model, a placeables.2da row), why it must go in a hak, how its rows meet a project's own table (TDoD's library build writes placeables.2da), reuse, undoing it, readings to make in the engine, milestones and the owner's questions.
tags: [proposal, walkmesh, visual-transform, placeables, custom-content, haks]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-03T04:50:42Z }
sources:
  - id: owner
    resource: human:august
    title: 'The owner, 2026-11-03: rebaking walkmeshes to the transform would be niche but useful (a large stone downsized as decoration on a desk); not the default, since builders may expect the walkmeshes unchanged; wanted as an option if it can be done cheaply'
  - id: answers
    resource: human:august
    title: 'The owner''s answers, 2026-11-03: the walkmesh only; the rows by a rows file (a); TDoD''s range 40000-40999 and hak tdod_baked, told to the TDoD Module session; placeables first, doors later; Tidy Baked Models offered on save'
  - id: finding
    resource: findings.md
    title: '"A visual transform doesn''t move a walkmesh" (engine_placeable_walkmesh.rs, walkmeshes_by_visual_transforms, 2026-11-03)'
  - id: models
    resource: research/notes_models.md
    title: 'Placeables: the model named by placeables.2da ModelName; .pwk and .dwk walkmeshes'
  - id: code
    resource: crates/mg-area/src/walk.rs (ObjectWalkmeshes), crates/mg-area/src/lib.rs (VisualTransform::matrix), crates/mg-mdl (read, compile), crates/mg-plugin and crates/mg-ui (haks written with Undo) (read 2026-11-03, at 3.5.0 + develop)
  - id: tdod-build
    resource: ~/Projects/tdod-modulework/docs/assets/library-build.md
    title: 'TDoD''s library build: tdod_top.hak''s placeables.2da (the game''s table plus its rows), row blocks of 500 from 20000'
---

# A placed object that blocks as it is drawn: a design

A builder scales a large stone down to sit on a desk. The model shrinks. Its walkmesh doesn't,
so the stone still blocks a doorway's width of floor around the desk. The owner would like an
option, never the default, that makes such an object block as it is drawn.[^owner] This
proposal sets out what the game allows, what the option would make, and where that has to go.
The owner chose (below), and it is built: see "As built".

## What the game does (settled)

The engine never applies a placeable's or a door's visual transform to its walkmesh: not the
scale, the translation, the turn or the tilts, static or dynamic. That was measured with
creatures walking lanes in the headless server.[^finding] A placeable's walkmesh is the `.pwk`
named after its model, and the model is the `ModelName` of its `placeables.2da` row.[^models]
So no field of the object can change what it blocks. Only another model, with another walkmesh,
can.

## What the bake makes

**Bake the walkmesh only** (recommended). For one placed placeable, Moonglow:

1. **Copies its model unchanged under a new name** (`mgbk_0001`, from the project's range,
   below): the same geometry, animations, emitters and textures. Only the model's own name
   changes, and the copy is compiled again.
2. **Writes the copy's walkmesh** (`mgbk_0001.pwk`): the original's faces, moved by the object's
   visual transform exactly as the game draws the model (`VisualTransform::matrix`: scale, the
   tilts, the turn, the translation).
3. **Adds a `placeables.2da` row** that is the original's row with that `ModelName`, and points
   the object's `Appearance` at it.
4. **Keeps the object's visual transform as it is.** The game still draws the model scaled as
   before, so it looks the same, and now its walkmesh matches.

The alternative is to **bake everything**: scale the model's own geometry, its animations, its
emitters and hooks, and clear the visual transform. That's much more to get right (an animation's
positions, an emitter's sizes, a supermodel's animations at the old scale), and the result can't
be scaled again without baking again. Its only gain is a model drawn without a visual transform,
which the game handles well already.

**One bake per model and transform.** Two objects with the same model and the same transform
(within a millimeter and a tenth of a degree) share one baked model and row. Baking five stones
the same way makes one model; five different sizes make five.

**Undoing it:** **Unbake** sets the object's `Appearance` back to the original row, and its
visual transform stays. A baked model no object uses any longer is listed by Verify
(`bake-unused`) and removed by **Tidy Baked Models**, never on its own: another module or a hak's
blueprint may use it.

## Where it goes

Players' games read new models and `placeables.2da` rows only from haks (or NWSync, which serves
haks). The module itself is sent to no client, so whatever the bake makes goes in a hak. Two
parts:

- **The models** (`.mdl` and `.pwk`, no textures: they keep the original's) go in a hak of the
  project's choosing, written by Moonglow as plugins' haks are written, with Undo.
- **The rows** are the hard part. `placeables.2da` is one table: the highest hak that has one
  wins. TDoD's is written by its library build into `tdod_top.hak` (the game's table, padded,
  plus its rows in blocks of 500 from row 20000).[^tdod-build] Three ways for the baked rows to
  reach it:

| | How | For | Against |
|---|---|---|---|
| (a) | Moonglow writes a **rows file** (`baked_rows.toml`: row number, the row it copies, the model) beside `moonglow.toml`, and the project's own build adds them to the table it writes | TDoD: its build already owns the table, as it owns the palette lists | A project needs a build that reads the file; until it runs, the game draws the original walkmesh |
| (b) | Moonglow writes the **whole table**, the one under it plus the baked rows, into its hak, which goes above the others | Projects with no build of their own | When the table under it changes (a library build), Moonglow's copy is stale until written again; Verify says so (`bake-table-stale`) and offers to write it again |
| (c) | Moonglow writes the rows into the project's **own 2DA file** in its sources, where `moonglow.toml` names it (`[bake] table = "src/2da/placeables.2da"`) | Projects that keep the table as a file they build into a hak | A file Moonglow didn't write, changed in place (Moonglow keeps a 2DA's layout, as the 2DA editor does) |

Recommended: **(a)** for TDoD, **(b)** where a project has no build of its own, and (c) only
if asked. All three take rows from a range the project sets aside, as typed text takes talk-table
lines:

```toml
[bake]
rows = "40000-40999"          # placeables.2da rows baked models take (outside the library's blocks)
hak = "tdod_baked"            # the hak the models go in (and, with (b), the table)
names = "mgbk_"               # baked models' names begin so: mgbk_0001
rows_file = "baked_rows.toml" # (a): the rows, for the project's build
```

Without `[bake]` in the project file, the command isn't offered, with a tip saying why.

## In the window

- On a placed placeable's right-click menu, under **Position ›**: **Bake Walkmesh to Transform…**
  (and **Unbake Walkmesh** on a baked one). It's offered only where the object has a visual
  transform and the project has `[bake]`.
- The window says what it will make before anything is written: the model's name, the row, the
  hak, the rows file or table, and whether an earlier bake is reused. It shows the walkmesh
  before and after, in the area view's Object Walkmeshes.
- Each bake is one step of Undo: the object's change, the hak's files, the row.
- Several objects selected: each one, the same transforms sharing one bake.
- `mg bake-walkmesh MODULE --area A --object N` does the same from the command line.

## Readings to make first

1. **The game draws the copy as the original**, with its model renamed and compiled again (the
   client, side by side). A supermodel's animations still play.
2. **The baked walkmesh blocks as it is drawn**, scaled down and tilted (creatures' lanes in the
   engine, as `walkmeshes_by_visual_transforms` does).
3. **Whether a module can carry its own `placeables.2da`** for single-player modules with no hak.
   If it can, a hak could be optional there. Players of a server need a hak whatever this finds.
4. **Doors** (`.dwk`, from `genericdoors.2da` or `doortypes.2da`): the same, later, if the owner
   wants them.

## Cost

Medium. Moonglow already reads, transforms and compiles models, writes haks with Undo, and keeps
a project's ranges, so most of the pieces exist. The new parts are the bake itself, the window,
the rows' three destinations, reuse, Unbake and Tidy, and Verify's two checks. The readings take
the client once and the engine once.

## Milestones

1. **M1, the readings** (done, 2026-11-03; below): 1, 2 and 3. If the client draws the copy
   differently, stop and report.
2. **M2, the bake** (`mg-module`; done): the copy, its walkmesh, the row, reuse, `[bake]`, the
   rows file (a) (the whole table (b) not wanted), Unbake. Unit tests, plus an engine test of a
   baked object's lanes.
3. **M3, the window and `mg`** (done): the command and its window, the walkmesh shown before and
   after, Verify's `bake-unused` and `bake-row-pending` (`bake-table-stale` was (b)'s), `mg
   bake-walkmesh`, the manual.
4. **M4, TDoD's side** (its session): its library build reads `baked_rows.toml`, and a row range
   outside its blocks.

## As built (2026-11-03)

**The readings.**[^built]

1. The client draws a baked copy as the original (`reading_baked_copy_drawn_as_the_original` in
   `client_render/readings.rs`): the armoire and its copy opened alike by its own `open`; a model
   whose animations are all its supermodel's (the armoire without its own, the armoire its
   supermodel) and its copy opened alike, the copy finding the supermodel's nodes by the part
   numbers it keeps (asserted); the game's one placeable with a supermodel, kelp, drawn alike, and
   still in the original as in the copy (the client doesn't sway it).
2. A baked walkmesh blocks as the model is drawn (`baked_walkmeshes_block_as_drawn` in
   `engine_placeable_walkmesh.rs`, kept as the engine test): a 12 m square at Scale 0.25 is walked
   beside where the unbaked one would turn walkers, and gone round where the baked 3 m one is; a
   square at Scale 0.5 turned 90 is gone round where the bake put it, 3 m west, and its old place
   walked over.
3. **A module can't carry its own `placeables.2da`:** a row only the module's copy has gives the
   placeable no walkmesh in the engine (the placeable is there). The rows reach the game through a
   hak or the override alone.

**What differs from the design, and why:**

- **Rows end at 65535,** the last row the game carries (the engine and the network carry an
  object's row as a 16-bit number: TDoD measured 65536 as row 0). A `[bake] rows` reaching past
  it is a problem of the project file, said in the log and by Verify, and the bake isn't offered;
  and a placeable whose `Appearance` is past it, placed or a blueprint, is Verify's
  `appearance-range`, with or without a bake.
- **A bake never takes a row the game's table holds** (the haks' `placeables.2da`, or the game's;
  a row of `****` alone, a padded table's, is free), not only the rows file's; Verify's `bake-rules`
  says how many rows of the range the table holds.
- **The rows file is the settled four keys,** so a bake is found again by its walkmesh: the same
  original row, and every corner of the walkmesh within a millimeter, and a tenth of a degree at
  its reach, of the one in the hak.
- **The rows the table lacks are drawn as the build will add them**: a layer over the haks
  (`bake:rows`) holds the table with them, so that a baked object shows its copy, and its walkmesh,
  in the area view before the build has run. Verify reads the table under that layer.
- **The keys and names in the examples** are only examples; TDoD's range is the owner's to choose.
- **Tidy Up** keeps the hak beside itself as `.bak`; it is not a step of Undo (it runs as a save
  begins). `[bake]` is read beside a `.mod` too, its rows file beside `NAME.moonglow.toml`.

**For TDoD's build (M4):** read `[[row]]` entries of `baked_rows.toml` (beside `moonglow.toml`):
`row`, `copy_of`, `model`, `label`; for each, the row `row` of the `placeables.2da` it writes is
row `copy_of`'s, its `ModelName` `model` and its `Label` `label` (quoted: it has spaces). Rows are
at most 65535. The baked models are in `[bake] hak`, which the module lists. Verify's
`bake-row-pending` names each row the built table lacks.

[^built]: Built on branch `worktree-agent-aecc87e823c69c32d`, 2026-11-03: `mg_module::bake`, `mg-ui`'s
`bake_view`, `mg bake-walkmesh`, Verify's four checks; the manual's Areas, Modules, Build and Test
and Command Line chapters.

## The owner's answers (2026-11-03)

1. **The walkmesh only** is baked: the model copied unchanged under a new name, its visual
   transform kept.[^answers]
2. **The rows go by a rows file (a)**, for the project's build to add. (b) and (c) aren't built.
   A project whose build doesn't read the file yet sees Verify's `bake-row-pending` for each row
   the table in its haks lacks, and the game draws the original walkmesh until then.
3. **TDoD: rows 40000-40999, the hak `tdod_baked`.** The TDoD Module session is told what its
   build has to do (below). (Since corrected, by TDoD's own measurements: its library uses rows
   40000-40999, and the owner will choose another range; rows past 65535 the game can't carry.
   These numbers are only an example now.)
4. **Placeables first,** doors later.
5. **Tidy Baked Models is offered on save:** where a save finds baked models no object of the
   module uses any longer, it asks (Tidy Up, Keep Them) and takes nothing out unasked. Another
   module, or a hak's blueprint, may still use one, so the question says it checked this module
   alone.

### The keys and the rows file, as settled

```toml
[bake]
rows = "40000-40999"          # the placeables.2da rows baked models take
hak = "tdod_baked"            # the hak the baked models (.mdl, .pwk) go in, in the module's hak list
names = "mgbk_"               # baked models' names begin so: mgbk_0001 (at most 16 characters)
rows_file = "baked_rows.toml" # the rows, beside moonglow.toml, for the project's build
```

```toml
# baked_rows.toml: written by Moonglow Toolset (Bake Walkmesh to Transform); the project's
# build adds each row to the placeables.2da it writes. Edit it in Moonglow, not by hand.
[[row]]
row = 40000                   # the row it is, in the range
copy_of = 1234                # the placeables.2da row it copies: every column of it, but
model = "mgbk_0001"           # ModelName, which is this
label = "Baked: Boulder 03 at 0.25"   # its Label (spaces kept quoted, as 2DAs quote them)
```

The first bake adds the hak to the module's hak list where it isn't, one step of Undo with the
rest. A row no object uses any longer stays in the file until Tidy Baked Models takes it, with
its model.

## Questions for the owner (answered above)

1. **Walkmesh only, or everything baked?** Recommended: walkmesh only (the model copied
   unchanged, the transform kept).
2. **Where the rows go:** (a) a rows file for the project's build, (b) the whole table in
   Moonglow's hak, or (c) the project's own 2DA file? Recommended: (a) for TDoD, (b) otherwise.
3. **The range and the hak** for TDoD: rows 40000-40999 and a hak `tdod_baked`, or others?
4. **Placeables only at first,** doors later?
5. **Tidy Baked Models:** by hand only (recommended), or offered on save?

[^owner]: The owner, 2026-11-03, in the session that wrote this proposal.
[^answers]: The owner, 2026-11-03, in the same session.
[^finding]: [A visual transform doesn't move a walkmesh](findings.md), measured in the headless server on 2026-11-03.
[^models]: [Model notes](research/notes_models.md): placeables named by `ModelName`; the `.pwk`.
[^tdod-build]: TDoD's `docs/assets/library-build.md`: `tdod_top.hak`, row blocks.
