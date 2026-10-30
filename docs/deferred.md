---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What finished work left undone or unchecked, filed by area of the program (an entry's last words say which report, issue or review it came from), each awaiting a verdict - fix, add or drop; what is settled by decision or design is gathered at the end.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-29T20:48:14Z }
---

# Deferred: what was left out, for a decision later

What finished work left undone or unchecked, in one place: each entry
awaits a verdict (fix, add, or drop). Work is deferred only when doing
it would have real unintended consequences or would go beyond what was
asked, and the entry says which; what can't be done or checked on the
development machine is filed as waiting on it (`CLAUDE.md`, Rules). The list is filed by area of the program, one entry a gap, in
the section of its area; an entry that came from a builder's report, a
GitHub issue or a review says so in its last words. What was fixed is in
the [log](log.md) and the release notes, what was measured in the game
or Aurora is in the [findings](findings.md), a question asked and not
answered is in the section before the last, and what stays as it is by
decision or by design is in the last section.

Size: S (an hour or two), M (a day or so), L (several days).

## Tests and the build (the [review](test-review.md) of October 2026)

- **The `app` tests' set-ups left by hand** (`crates/mg-ui/tests/app/`,
  split by subject, with `harness`, `game_app` and `gpu` in `common.rs`):
  86 harnesses at kittest's default size (`Harness::new_ui_state`), 27
  installs made apart from the application (read before it, in another
  language, or by a test's own helper), 14 scratch user folders, each
  with its own subfolders, and 6 harnesses with a step of their own.
  They are in 17 of the folder's 21 files, the scripts', the area
  view's and the lists' among them, which the deferred sprint's other
  packages of 2026-10-29 change: folded after those merge, not beside
  them. (A file not named in `main.rs` fails
  `every_test_file_is_a_module` since 2026-10-29.) (M)
- **`mg` builds the GPU stack by default** for `mg nui render` (drawn
  with wgpu) and `mg nui layout` (a frame of the NUI Creator's canvas,
  egui, no GPU), both of mg-ui:
  229 crates, 2 min 42 s cold (`cargo build --release -p mg`, 4 jobs),
  37.8 MB. Without the feature `nui-preview` (`--no-default-features`)
  `mg` is 140 crates, 87 s and 22.7 MB, and has neither command (clap
  says "unrecognized subcommand"). The canvas in a crate of its own,
  with egui and no wgpu, would leave `render` alone behind the feature;
  the packages keep the default, so only builds of `mg` from source
  gain. Looked into on 2026-10-29 (the deferred sprint's package m11), it
  is several days, not one: the crate would take `nui_view/preview/`
  (the canvas, about 3,400 lines), `skin.rs` (1,400), the painting half
  of `draw.rs`, `layouts.rs`'s variant in its window and `interaction.rs`'s
  session, and the 25 or so fields of `nui_view::State` the canvas reads
  (a state of the canvas's own, carved out of the editor's); and what the
  canvas reaches into the editor for would become hooks: `design::node_name`
  and `Insert`, `structure::select`, `drag_target` and `Drag` (selecting,
  dragging and dropping on the canvas), and `widgets::field_label`,
  `widgets::dropdown` and `fields::hinted` (the draw layers' fields in
  `draw.rs`). (settle?: proposed by the sprint on 2026-10-29, the owner's
  to say: kept as it is while no one builds `mg` alone.) (L)
- **Moonglow Viewer's shader cache across toolset tags:** the libraries
  are 0.0.0 and read the release from the program (`mg_core::release`),
  which the viewer doesn't set, so its compiled-shader cache is named by
  the passes' `FORMAT` alone and no longer starts anew when it moves to a
  newer toolset tag. One line at the viewer's start
  (`mg_core::release::set`, its own version) names it as before. Not
  yet a fault: the viewer is on v2.8.0, whose `mg-shader` names the
  cache by its own version (`CARGO_PKG_VERSION`, 2.8.0), and
  `mg_core::release` is in v2.9.0 and after, so the line can only go in
  with the viewer's move to a newer tag (in its repository, not this
  one). (S, waits on the viewer's next toolset tag)

## Script intelligence (`mg lsp`, the script editor)

- **Editor setups:** the manual's Neovim and Helix snippets are untried
  in those editors. On Windows, `mg lsp` is checked only by CI's
  tests. (S) (waits on Neovim and Helix: neither is on the
  development machine, and putting them there is a download)
- **The external compiler with `nwnsc`** (Options › Script Editor):
  its line of arguments (`-q -y -o -n {game} -i {src} -b {out}
  {files}`) is from its documentation, not run, and its messages are
  matched to scripts by their file names and the word "error":
  nwnsc's own wording, not seen, may be missed (the script is then
  said to have made no compiled script). `nwn_script_comp` is tried
  (its bytecode is the built-in compiler's, byte for byte). (S) (waits
  on a copy of nwnsc: none is on the development machine) (from
  builders' reports after 1.14.1)

## nasher projects

- **NWNT projects:** projects using NWNT instead of JSON for GFF files
  are refused with a message. (M)
- **`mg merge-gff`, what it leaves out** (the [manual](manual/03-modules.md#merging-in-git)
  has the rules):
  - **Lists merged by place** that have an identity of their own: an
    inventory (`ItemList`, which could pair by `InventoryRes` and its
    `Repos_PosX`/`Repos_Posy`), a store's panes, an encounter's
    creatures, item properties, a creature's special abilities. Two
    sides adding one each is a conflict today, not both kept. (S)
  - **Conversations** merge their entry and reply lists by place, and the
    links name nodes by number: two builders adding nodes to one
    conversation conflict, where renumbering one side's new nodes (and
    the links to them) would keep both. (M)
  - **An object both moved and retagged on one side** is no longer
    recognized: a removed object and an added one (a conflict if the
    other side changed it). A nearest-position match could pair it. (S)
  - **Order:** where both sides changed a list, one side's reordering
    (a list sorted, an area moved up the area list) is lost; additions
    go at the end. A sequence merge would keep both. (S)
  - **Localized strings** merge whole: one side changing the English
    name and the other the French one is a conflict. (S)
  - **On a conflict nothing is written** and git leaves ours in the file;
    there is no file of the conflicts to edit, and no way to take one
    side for one conflict and the other for the next but `-o` from
    git's stages. A conflict file (the edits each side wants, for `mg
    apply`) or a Moonglow window would. (M)
  - **NWNT files** aren't merged (the projects aren't read either). (S)
  - **The settings:** a JSON file's float places and codepage come from
    the nasher project above `--path` (or OURS's folder); a repository
    whose project sits elsewhere gets nasher's defaults. (S)
  (The Domains of Dread's builder proposal, C9)

## Modules kept as folders

- **A module's files changed outside, read on a thread of their own**
  (`mg_module::outside`; their reads' allocator arena is Settled): what
  the entry said besides, as it is and not among the triage's items: a
  reload that touches resources the undo history has
  changes to clears the whole history; a script open in the editor with
  typing not yet applied counts as changed here; the area list isn't
  settled again for areas added outside; the haks and the `override`,
  `development` and portrait folders (and the custom talk table) are
  read again on the window's thread after a change (not measured on a
  large `override`). (S each)

## Test loop

- **`mg serve` on Windows:** not tried there: the server's console on
  pipes, the links for haks, and Ctrl+C (the server in a console
  process group of its own, `CREATE_NEW_PROCESS_GROUP`, told to `exit`
  by `mg`). (S) (waits on a Windows machine)
- **`mg watch`** (a project followed into a development folder):
  nothing is pushed through NWNX's console (what changed goes to the
  development folder, which a server with NWNX reads as any other).
  Dropping it is proposed: an NWNX integration no one asked for.
  (settle?: proposed by the sprint on 2026-10-29, the owner's to say)
  (M)
- **Test Module and Steam:** the game started for a test is told which
  Steam game it is (`SteamAppId`), so that a Steam copy reaches a running
  Steam and loads the Workshop's content, as when started from Steam;
  started bare it had none of it (a builder's "F9 won't load the stuff
  correctly either"). Not tried: the client is only ever run here with
  Steam hidden. With Steam running, the test then shows in Steam as the
  game being played. (needs a builder's word)
  (from builders' reports after 1.14.1)

## Persistent-world scale

- **Other machines:** budgets were measured only on the development
  machine (Ryzen 7 5800X3D, Radeon RX 9070). (S, needs other hardware)

## Frames (a steady 144 a second)

The budget and the measurements are in
[the frames note](research/notes_frames.md). Done: an area's picture kept
between its animations' steps, its drawing at a fifth of the cost (lights
by the ground they reach, materials worked out once, a draw's values by
its number, what is out of sight left out, tiles of one model posed
once, meshes of one model drawn as instances of one draw), the
galleries' pictures made for 2 ms a frame and kept while in sight.

- **The largest areas in full view:** a 784-tile stock area takes 2.5 ms
  a frame zooming out; TDoD's synthetic woods (33,000 meshes in sight)
  13 ms (21 turned to the horizon), 25,000 draws of meshes mostly drawn
  once each. What is left is each draw's own work: wgpu's call and the
  chunk of shared buffers it switches to (a large area spans ten), each
  mesh's lights chosen and its values written. Fewer, larger chunks, and
  values kept from frame to frame for what doesn't move, would cut it.
  ([the frames note](research/notes_frames.md), 27 October) (M–L)

## EE fields Aurora hides

- **The PC faction, what's left** (waits on an Aurora capture and the
  client): Moonglow keeps objects out of faction 0 and says so where it
  is written (the faction lists, the Store Wizard, `mg set`, the raw
  fields view, Verify's `pc-faction`, an object with no faction too,
  which the server reads as 0: `engine_factions.rs`). Not known:
  whether Aurora's own faction lists offer PC (not captured; Aurora
  writes faction 0 for a trigger whose blueprint has none, which is why
  triggers are left out, the owner's decision); how an object of
  faction 0 regards a player, or a player it (no player joins a
  headless server); and what an encounter of faction 0, or of none,
  gives the creatures it spawns (a player's entering spawns them). An
  Aurora capture of a faction list and a client run with a player
  entering an encounter settle it. (S) (Jasperre's report, October
  2026)
- **The Visuals page's texture and animation pickers, what's left:**
  waiting on the client: whether the game replaces a PLT texture by name (the page and the
  preview take it that it doesn't, from `ReplaceObjectTexture`'s comment
  that the new one must be simple), and whether a creature's replacement
  reaches what it holds, its helmet or cloak (left out of its list). A
  client screenshot of `ReplaceObjectTexture` on each settles it. (S)
  (GitHub issue 32)

- **A tile's replacement texture, out of Tile Properties:** the form
  had a Replacement Texture (`Tile_ReplaceTex`, a `replacetexture.2da`
  row for a texture named `replace_tex` in the tile's model); a builder
  called it untested and likely not working, and it was taken out
  (2026-10-22). What is known: the server keeps the byte
  (`engine_ee_fields.rs`, `ObjectToJson`); nwn.wiki calls it BioWare's
  unused functionality ("Replacing Simple Textures", "2da Files"); the
  game's `replacetexture.2da` has two rows, both `T_Flag02`; the one
  model with a `replace_tex` texture, `tms01_c08_01`, is in no tileset
  (not in `tms01.set`); none of BioWare's 20 campaign modules
  (`data/nwm`) writes the field. The client draws it (2026-10-29,
  `reading_tile_replacement_texture`, an area's tiles made
  `tms01_c08_01`): a value is the row after the one drawn (1 draws row
  0, 2 row 1), a tile without the field draws row 0, and 0 or a value
  past the table crashes the client as it loads the area
  (`CNWCTile::SetReplaceTexture`); the area view now draws as the
  client does (`mg_area::replacement_row`). Left: the field back in
  Tile Properties with that rule (a row chosen written as its number
  plus one, 0 never written), and Verify warning of a tile whose model
  has `replace_tex` and whose value the client crashes on. (S) (from a
  builder's tile feedback, October 2026)

## Blueprint editors and inventories

- **An appearance with no model, what the warning leaves out:** the rule
  (`mg_rules::appearance`) checks a placeable's, a door's, a
  single-model creature's and a waypoint's row and model by name (the
  waypoint's flag since 2026-10-29: Verify warns of it, the preview and
  the Properties go by it; a waypoint without the field, as 190 of
  BioWare's blueprints are, is not judged, and what Aurora draws for
  one was not captured), and an item's base item row.
  Not checked by it, each S, about a day together: an item's own models
  (its parts, `ModelPart1`… and an armor's), whose names mg-preview
  alone makes (the naming would move down into mg-rules); a body of
  parts' skeleton and parts, likewise; a model that is there but cannot
  be read (a broken file is found when it is drawn; reading each
  appearance's model in Verify was not timed on a large module); the
  Appearance lists in the Properties don't mark the rows with no model
  (the Appearance Gallery does, for those in sight: a mark in
  `widgets::row_choice`, the rows checked once a revision, not each
  frame); and the palette's own Gallery doesn't mark blueprints whose
  appearance has none. (dansen9191's report, October 2026)
- **Number fields' limits against Aurora, what is left:** the rest were
  measured in Aurora (2026-10-29, typed past their ends: an encounter's
  creatures 1 to 8, its respawn time 1 to 32000 s and count 0 to 32000,
  a sound's interval and variation to 60 s, a trigger's highlight height
  0 to 10, a creature's CR adjustment -125 to 125, an area's fog clip
  distance 30 to 9999 m; weather, shadow opacity, fog amounts, a store's
  mark up, mark down and stolen goods' mark down as Moonglow had them)
  and are Moonglow's now, as is the encounter's maximum raised to a
  minimum typed above it (since 2026-10-30). Left, waiting on Aurora: an
  item property's Chance of Appearing (Moonglow 0 to 100) typed past
  both ends, -5 and 150, in the property's window (`TDLGPROPEDIT`'s
  `eChance`), and what the saved `.uti`'s `ChanceAppear` holds; its
  up-down's form says only Min 0, and the form's limits are not the
  code's (the encounter's say 0, its code 1 to 8). (S) (a second
  builder's review, October 2026)
- **Random item properties, what is left** (done: the Item Wizard's
  Magical, Item Level and Item Quality, with rules fitted to 127 items
  Aurora's wizard made, `docs/findings.md`): the rules are fitted, not
  Aurora's code: whether a later property's tier is drawn (one or two
  below the first) or tried in order, and Low items' most worth (levels
  1, 5, 8 and 13 fit what was seen; nothing in itemvalue.2da says so)
  would need more of Aurora's items to settle; amulets, belts, boots,
  bracers, cloaks and rings get the armor table, where Aurora may use
  the helmet's (they differ in Weight Reduction only). All of it waits
  on Aurora: a few hundred of its wizard items, made by driving its
  wizard on the oracle (Low items at each level band; later properties'
  tiers; an amulet, belt, boots, bracers, cloak and ring each at 16-20
  Godly, whose Weight Reduction would tell the table), read against
  `aurora_item_wizard.rs` (M). (Gems and gauntlets are offered Magical
  since 2026-10-30.) (a second builder's review, October 2026)
- **Inventories, what is left** (done: the Creature Wizard's ResRef and Tag, its appearances
  as pictures with Find, and every portrait; the inventory's Equip to a
  free slot, To Backpack and Open Blueprint; talk-table text in its own
  color; a Variables window that is sized; drag and drop and right-click
  menus in the inventory, items dragged between inventories and within
  a list, Copy and Paste; an item moved as it is, in one step; what a
  taken slot held sent to the backpack, as Aurora does; Copy and Paste
  carrying the item as it is, as a drag does; drags between two
  objects' windows under test both ways, a slot's item onto a chest and
  a chest's onto a taken slot; a weapon dropped on the secondary weapon
  slot while the primary is free going in the primary, as Aurora puts a
  longsword). Left:
  - **Several items at once:** the inventory selects several (Ctrl,
    Shift), but a drag, Copy and Paste carry one: `Dragged` and
    `item_clip` hold one item each place; carrying the selection is a
    drag of many rows, and where each lands (slots, grid places). (M)
  - **The secondary slot, for other weapons:** Aurora was seen with a
    longsword only; the Aurora batch should drop a dagger, a one-handed
    axe and a torch (which fits the secondary alone) on the secondary
    slot of a creature whose primary is empty, and say where each goes.
    (waits on Aurora)
  - **A move between two slots that needs a feat:** Moonglow asks the
    question about a missing feat as any Equip does; Aurora asks when a
    weapon is dropped on another weapon slot, but a true move of an item
    that needs a feat between two slots it takes was not seen (the
    Aurora batch: a creature without Martial Weapon Proficiency, a
    longsword dragged from its primary slot to its secondary). (waits on
    Aurora)
  - **A blueprint made in Familiars:** the Creature Wizard offers every
    category of `creaturepal.itp`, Familiars (ID 254, `TYPE` 1) too, and
    the custom palette is made without `TYPE` 0 and 1 nodes, as Aurora
    makes it (`docs/findings.md`, "Custom palettes"): a blueprint made
    there is in no category the palette shows. Whether Aurora's wizard
    offers Familiars, and where Aurora shows such a blueprint, the
    Aurora batch should read. (S once read; waits on Aurora)
  - **New strings written to a talk-table range:** a name or
    description typed in a blueprint goes into the file, Aurora's way;
    writing it as a line of the module's custom talk table instead, at
    the next free line of a range the project keeps (`mg tlk-alloc`'s
    map is one), touches every localized-string field and the talk
    table's saving. (L)
  - **Made from the blueprint, between a placed object and a
    blueprint:** an item carried (dragged, pasted) between them is made
    from its blueprint where it lands, keeping its flags, as a
    blueprint's inventory holds only a blueprint's name. (settle?:
    proposed by the sprint on 2026-10-29, the owner's to say)
  - **A moved row keeps its grid place:** a row dragged within a list
    takes its place in the list, and keeps its `Repos_PosX` and
    `Repos_Posy` (where the game's inventory grid shows it), as
    Moonglow shows a list, not Aurora's grid. (settle?: proposed by the
    sprint on 2026-10-29, the owner's to say)
  (a second builder's review, October 2026)

## Bulk editing

- **Lists across several blueprints:** Edit Together leaves out the
  pages that edit lists (`blueprint::LIST_PAGES`: inventories, classes,
  skills, feats, spells, special abilities, item properties, an
  encounter's creatures, a store's restrictions); those are edited one
  blueprint at a time. Each page builds its commands for the first
  object alone, and `blueprint::partial` drops a command that reaches
  some of the objects but not all, as "add this feat to each" would
  where some have it already. What each page needs, looked at in the
  deferred sprint (2026-10-29): skills, a rank set at the skill's index
  on each (`creature.rs` `skills`, the simplest); feats, special
  abilities, an encounter's creatures and a store's restrictions, an
  entry added to those that lack it and taken from those that have it,
  marked (≠) where only some have it; classes, a level per class on
  each and a class added where there is room; spells, per class; item
  properties, matched by type, subtype and value; inventories
  (`inventory.rs`), items added to each. Several days' work with a
  design of its own for the marks and the guard. (L)
- **Advanced Search, what it doesn't do** (from builders' asks on
  Discord, October 2026; done: changing what it found, ticked or all, in
  one step: a variable set or removed, an event's script, a field;
  Edit Together and Edit Areas Together from its results; conditions
  together, a class by its levels, number ranges and floats; an item
  blueprint followed through bags in bags, and a creature blueprint
  into the encounters that spawn it, since 2026-10-29):
  - Changes are a variable, a script or a field: item properties aren't
    added or removed on the items found, nor list entries (feats,
    classes, inventory) on the creatures; a field is set only where the
    object has it (none is added). Edit Together takes blueprints of one
    type and objects of one kind placed in one area; items held by
    placed objects, and conversations, open one by one. (S–M; shares
    its design with editing lists across several blueprints)
  - No spell by the class that knows it (a spell found is any class's);
    conditions are all ANDed (no OR, no NOT: the window's conditions
    and `Query` both, each half a day); a conversation's lines and other
    things on no object drop out of a search with more than one
    condition (there is no object for the others to be found on);
    `mg search` takes one condition at a time, without class levels or
    ranges. (S each)
  - The module only: blueprints in haks and the game's own palettes
    aren't searched (`Kept::search` walks `Module::keys`; the haks' and
    palettes' blueprints read as the module's are would be a layer of
    hits of their own, read-only, and a change offered on none of
    them). (S–M)

## Palettes

- **A custom palette's names in the game, issue 13's other half** (a
  builder's "HipÃ³lito" in the DM's Creator, after a save in Moonglow, is
  fixed and seen right in the DM client: the deferred sprint's log,
  2026-10-29). With Polish the editing language, String Edit read and wrote a string's
  English text as Windows-1252 (mojibake, and Polish letters typed
  could not be saved, under an error that named the String Ref). It
  reads and writes the Windows-1252 languages' text as Windows-1250
  then, as the lists did already and as a Polish game shows it, and
  says which letters a text cannot hold (`text.rs`'s `entry_codepage`).
  Left, waiting on the reporter: String Edit on their module with Polish
  the editing language, its names seen right and one saved. With another
  editing language a Polish name kept in the English text is read as
  Windows-1252, as an English game and Aurora (the system's codepage)
  show it: Moonglow has nothing else to know the text's codepage by (settle?:
  proposed by the sprint on 2026-10-30, the owner's to say). (S)
  (from builders' reports after 1.19.1)

- **The custom palettes kept, what is left** (since a save changes only
  the entries of the blueprints changed, as Aurora does:
  `update_custom_palette`, the captures in `palettes/`):
  - *Where Moonglow differs from Aurora, by choice:* a save doesn't put
    each category's entries in order by name or drop fields it doesn't
    know, as Aurora's does, so a palette Moonglow didn't change is the
    same bytes; a new entry for a talk-table name is its `STRREF` (as
    Aurora's Build writes it) where Aurora's Edit Copy and Add to
    Palette write the text; a module with no `palcus.itp` gets one made
    in full at a save, where Aurora writes the categories alone, empty.
    (S, if any should follow Aurora)
  - *An entry made stale outside stays so, where Moonglow didn't see the
    blueprint change:* Aurora rewrites an entry at Edit and OK even when
    nothing changed (it writes the blueprint again); Moonglow when the
    blueprint's bytes changed since the module was opened or saved. A
    blueprint added, changed or deleted in an open module folder or
    nasher project is such a change (read again, it is not taken into
    what the save compares with: Troubletcat's report, October 2026),
    and the palette changed so is unsaved work. An entry left stale by
    another tool while the module was closed, a merge, or an `mg`
    command on an open `.mod` (no archive is watched) is made right when
    its blueprint next changes, by the palette's More › Refresh Palette
    (that type's, as Aurora's right-click Refresh Palette: its capture
    changed the item palette alone) or by Build. Work recovered after a
    crash counts as opened: its blueprints' entries wait for a change
    too. A module's own categories (`<type>pal.itp`) changed outside are
    taken in as they are, without remaking the palette's categories
    (the palette pulled with them has them); a search typed doesn't open
    the category of a blueprint that arrives after it was typed (typed
    again, it does). (S each)
  - *A category left with an empty `LIST`:* an entry put into a
    category that had no `LIST` field makes one, and the entry taken
    out again (a blueprint added, then deleted, before a save) leaves
    it empty, where the file had none: the same palette to the game,
    not the same bytes. (S)
  - *What moves an entry besides its blueprint:* a faction renamed
    (`repute.fac`) leaves its creatures' `FACTION` until they change; a
    hak's skeleton changed, or a hak's blueprints changed, renamed or
    removed, don't touch the module's palette (the module's own
    skeleton changed remakes its categories, each with its entries; a
    category taken out takes its entries, a hak's too). Aurora's save
    (its Save As, captured 2026-10-29, `captures/wave1/saveas.mod`)
    gives every creature entry its blueprint's `CR` and `FACTION`,
    stale or not, which Moonglow's doesn't; a blueprint of category
    255 or one the palette lacks is left out by its Refresh Palette,
    as Moonglow leaves it out. (S)
  - *An entry put in a category out of order* goes after the last entry
    whose name doesn't come after it (in order, its place); Aurora puts
    the whole category in order at its save. (S)
  - *The command line:* `mg rename` renames a blueprint's entry in
    place, but `mg set`, `mg apply`, `mg import`, `mg update-instances`
    and plugin commands run from `mg plugin` change blueprints without
    touching the palettes; the next save in the GUI or a Build brings
    them up to date. (S–M)
  - *No command takes out one entry* whose blueprint isn't the
    module's (a hak's, the override folder's, one there is no more);
    Build takes out those of the override folder and the ones there
    are no more, and keeps a hak's. (S)
  (El Grillo's and Jasperre's report, October 2026)

## Custom content data (2DAs and talk tables)

- **The 2DA editor** ([its proposal](2da-editor-proposal.md), built
  after 1.21.3: a table's cells in the module, a hak or a file, checked
  as typed, with rows taken from another copy). Not done:
  - A model's cell has no list to choose from and is not checked by the
    row; a talk-table string's cell offers the talk table, not a new
    line in it. (S each)
  - Find References to Row looks in the tables' known columns, not in
    blueprints, areas or scripts. (M)
  - A row both copies changed is taken whole or left: its cells are not
    chosen one by one. (S)
  - A table saved is laid out anew, columns in line or compact as its
    file was (its own widths, its tabs and its row labels are not
    kept: rows are numbered), and an empty row at the end is not
    written. `mg 2da-diff -o` and a plugin's `twoda:write` write
    columns in line only. (S)
  - A table open in the editor is not read again when what it was read
    from changes under it (the module's Undo taking its save back, the
    hak's table replaced, the file changed outside). (S)
  - Column widths (dansen9191's report, 2026-10-21) are kept by the
    table's name and the column's, in the settings (the last 200
    tables): two different tables of one name (two haks' `custom.2da`)
    share them, and there is no way to put a column back to its first
    width but to drag it there. The resource browser's **From** column
    (the layer a row comes from) keeps its width. A double click on an
    edge fits the column to its rows as shown then (Find's rows, the
    strings as text or not), not to all of them. (S each)
  - The resource browser's read-only view (with its scroll bars since
    2026-10-21) has no keys of its own: no cell to select, nor the
    arrows to move with, nor Find. It is drawn by code of its own beside
    the editor's grid, which it copies (the column names and rows'
    numbers that follow the scroll); one grid for both, with the editing
    left out, would serve. (M)
  - Large tables were not timed while edited (a persistent world's
    `appearance.2da`); Find looks through every cell at each frame
    (Take Rows reads the copies once, and works the rows out once for
    a copy). (S)
  - No row is put between others, by decision (Delete Row takes any
    row away since 1.22.1); no sorting, no `DEFAULT:` edited. (—)
  - Asked for by builders on 1.22.0 and 1.22.1 (October 2026), none
    begun:
    - **Columns hidden** from view. (S)
    - **A table's description** (`name.2da.toml`: what each column
      holds, lists from a script's constants, `mg 2da-check`; built
      after 1.22.1 from a builder's workbook,
      [the proposal](2da-editor-proposal.md#columns-that-say-what-they-hold-a-builders-spreadsheet-read)).
      Not done:
      - One description for a family of tables (`loot_*`); the game's
        own tables described the same way. (M)
      - Verify Module reads the descriptions of the module's own
        tables, not those of its haks' (`mg 2da-check` on the hak's
        sources does). (S)
      - A constant whose value is a function's result is left out of a
        list, and said; one made of constants of another script it
        includes is too. (S)
      - A description's folder is walked when it is read (once, and
        again when the file changes; the folders a build fills are
        passed over): a second and a half for a repository of 144,000
        files in a build made for testing, during which the window
        waits. A table of the project that a description names is read
        from its file once, not as it is open in the editor. (S)
      - Without a game install, `mg 2da-check` knows no talk table: a
        `strref` column's custom string is said not to exist; a
        plugin's `faults` does not look strings up at all. (S)
      - A hak's table's description (beside its file in the hak's
        folder) was tried beside the hak's file, not in a folder a hak
        is built from. (S)
    - **A talk-table string chosen from the talk table** in the cell,
      not gone to. (S; with "a new line in it" above)
    - **A class's spell tables checked against `spells.2da`** (each
      spell a class's table names is one the class can learn): Verify
      Module holds the lists a class names to rows that exist, not to
      what those rows allow. (M)
    One builder's view, to weigh the rest by: a spreadsheet does most
    of this already, and the editor earns its place on a module's own
    tables (treasure, spawns), not on the game's.
  - The keys: a letter typed does not begin typing in the cell (Enter,
    F2 or a double click does); a drag selecting cells does not scroll
    the table when it reaches its edge. (S each)
  - Edit… › The Table of a hak, in the resource browser, was tried
    through the functions under it (the hak opened by its file, its
    table opened in it), not by its menu on a module with a hak. (S)
  - The hak editor's Edit Table and the Take Rows window against two
    haks' copies were tried through the functions under them and with
    the game's copy, not by their own buttons. (S)
- **Text put in the talk table as it is written** (a conversation's
  lines, names), in a range set aside for it, and reusing a line that
  already has the same text, asked first: proposed by builders, not
  begun. Move to Talk Table does one string at a time. (L)
  (from builders' reports after 1.19.1)

## Haks in the GUI

- **Not followed by `--usage`** (and What the Module Uses), what is left
  of it after the sprint followed sounds, icons, visual effect
  properties, 2DAs and the heads, portraits and voice sets players
  choose (`mg_preview::usage`):
  - **What scripts reach by a number** (visual effects, spells, item
    properties, `SetCreatureAppearanceType`): a reader of a script's
    calls' arguments (an `int` literal, a stock constant from
    `nwscript.nss`, the module's own `const`s through its includes) by
    the function and parameter that take a row of a table
    (`EffectVisualEffect` visualeffects.2da, `ActionCastSpell*`
    spells.2da, `ItemProperty*` their iprp tables), then each row's
    files (a visual effect's models, progfx.2da's, its sounds; a
    spell's icons, models and sounds; an appearance's look through
    `mg_preview::creature`). In mg-script and `usage.rs`: a day or two
    (L), larger than the rest of the entry was.
  - **The body parts players choose** at character creation (tattoos):
    which part numbers the creation screen offers for a race and
    phenotype isn't known here (parts_*.2da has no column for it, and
    every part number would count every armor part as used). Waits on
    the client batch: read the creation screen's part lists against
    parts_chest.2da and the others.

  A script's string still counts as a use of whatever it spells,
  whatever the string is for (a release cut keeps too much rather than
  too little; said in the manual), and a per-hak list for the release
  cut (a folder of the used assets to pack smaller haks) is not written:
  `--json` has the lists, and writing haks from them goes beyond what
  was asked. (settle?: both proposed by the sprint on 2026-10-29, the
  owner's to say) (L; the body parts wait on the client batch)

- **Viewing in the hak editor:** a model of a hak the game doesn't read
  (not one of the module's, or changed there and not saved) shows as its
  text. Drawing it wants the model viewer to read its models, supermodels
  and textures from the hak over the load order: a source of its own
  (`model_view::Source`, keyed in `model_views`), `compose`'s loader and
  the renderer's `Assets` layered over the hak's bytes. Pictures, sounds,
  2DAs and a model the game reads from the hak are shown (2026-10-29).
  (M)

## Getting files out


## Area visibility


- **A grass tuft's sway:** a tuft is one upright quad turned at random,
  a texture quarter, as tall as wide, as the client's are (seen with
  white opaque tufts, sparse: `reading_grass_tufts`, 2026-10-29). The
  client's sway in the wind, tops moving and feet still (`reading_grass_sway`:
  6% of the field's pixels changed in a second and a half at `WindPower`
  2); Moonglow's stand still with either shader: the game's `vsgrass`
  reads no wind, so the engine moves the vertices itself, and the area
  view would have to as well. How far and how fast isn't read: the
  window's pictures come about 0.4 s apart and seconds late (30 taken
  from the side, `MG_CAMERA=90,4,85 MG_SHOTS=30,0`, 2026-10-29: half of
  them the same as the one before), too few and too loose to time a
  sway by. The reading that would: one tuft, a patched `fsgrass`
  painting its vertices' world place (top against foot, carried as a
  varying) and `worldtimerTimeOfDay` into the picture, as
  `light_uniforms_match_the_client` paints its uniforms, at `WindPower`
  1 and 2; then the tops moved in both of Moonglow's paths (its own
  grass shader, and the game's `vsgrass` fed moved vertices: a pass of
  their own over the whole view's grass, as the engine moves them on
  the CPU). (L: the reading, then the motion in both paths; waits on
  the client)
- **A walkmesh's own node position:** one shipped placeable
  (`ptm_candle02.pwk`, Tyrants of the Moonsea) puts its mesh 72 m from
  itself. Moonglow draws it there; whether the engine does isn't known.
  Tried in the server (2026-10-29, a probe not kept): a creature made
  where the mesh would be, at the placeable or 72 m off, is not moved at
  either, where one made on a wagon is moved off it; with the module's
  own copy of the walkmesh twenty times as wide (3.4 m), the same. So
  the engine ignores this mesh, or a module's copy of it, or making a
  creature doesn't see it: telling which needs a creature's path across
  each place, or the client. (M)
- **Sound ranges:** level circles at the sound's height. Aurora draws
  each distance as a dotted sphere about the sound, with one level
  circle at its maximum (captured 2026-10-29, the probe's sound of
  distances 3 and 12, from above and pitched). The random position's
  rectangle is as Aurora's now (its ranges wide, half each way). (S: the
  spheres, drawn as a few rings of dots)
- **Setup Area Transition's Connection Type:** the window picks a door
  or waypoint on the target area's map or in its lists, as Aurora's
  `TdlgAreaTransition` does, but has none of its Connection Type (Both
  Ways, Current To Target, Target To Current) nor its Target Type
  Trigger: the target linked back to this door or trigger. What Aurora
  writes to the target (its `LinkedTo`, and `LinkedToFlags` where either
  end is a trigger) waits on a capture of the dialog in Aurora; Moonglow
  then writes the same as one step. (S–M, needs Aurora) (what was left of
  "The minimap in Moonglow", 2026-10-29)
- **Fade Geometry, `tilefade` 4:** Never, Object Mode Only and Always
  were compared with Aurora (2026-10-29): its Object Mode Only fades in
  Select Objects and not in Select Terrain, as Moonglow's; the rural
  tiles' trees (`tilefade` 1) go and an interior's black caps (2) stay
  (a City Interior's walls of `tilefade` 1 looked the same in every mode
  at the three cameras tried, so that 1 goes rests on the trees). A
  mesh with `tilefade` 4 wasn't in the tiles captured, so that it goes
  is still taken from the game's tiles and their names. (S) (from
  builders' reports after 1.10.1)
- **An armor lying as plate here and as a tunic in Aurora**, in the
  same builder's picture: not looked into. The model is chosen by the
  torso's parts_chest.2da `ACBONUS` (measured with the game's own
  armors); his armor, or his content's table, may be a case that rule
  misses. Needs the item. (S) (from builders' reports after 1.19.1)

## Editing in the area view

- **Lerps, animation speed and other scopes, how the client draws a
  part's:** Adjust Location shows and sets a transform's animation
  speed and shows a lerp in progress and other scopes' transforms, kept
  as they are; the area view draws a creature's head, tail, wings and
  cloak scopes on those parts (the engine keeps each:
  `the_engine_keeps_part_scopes_animation_speed_and_lerps`). How the
  client draws a part's transform is assumed, not read: the part's
  model placed as the whole object's is (`mg_preview::visual_matrix`),
  about the node it hangs from (`head_g`, the wings' and the tail's
  dummies; the cloak, which hangs from none, about the creature's
  feet). The client batch should read it as `visual_transforms_look`
  reads the whole object's (`client_render.rs`): a part-based creature
  (appearance 6, wings 2) with its head scaled 2, then turned 45° about
  each axis in turn, then moved, and its wings and cloak the same; and
  a creature of one model (the bandit, appearance 269), whose head
  scaled Moonglow draws unchanged. An item's part scopes (255 to 251)
  are shown and not drawn: which of its models each is wants the same
  reading. (S, waits on the client batch) (xtul's report, October 2026)

- **Heed's interiors' lighting, waiting on Heed** (what he sees, a
  picture of it, and his haks): every area of his reports measured
  draws as the client draws it. His armourer (`hss_armourer`, the area
  he sent in October 2026 as still showing it: `tni01`, 60/170 by day,
  black fog of 5, the smithy's tile lights, no placeables) is within
  0.1 to 1.6 of 255 of the client's picture from four cameras, the
  Shaders switch's two pictures the same, his file and the stock one
  built from it alike (`heed_armourer_look`; findings, "An area's
  ambient and diffuse beside its tile lights"); the one difference seen
  (the forge's fire dark for its first seconds after the area opened or
  changed) is fixed. Before it: a stock smithy's tile lights within 2.4
  (`area_lighting_look`; his "tile lights don't show" was his
  placeables' light nodes, since lit); a `tin01` interior set as his
  within 0.7 to 1.5 at 60/170, 60/150, 255/170 and 0/0, the white
  ambient as flat in the client (`heed_interior_look`); his walls' custom
  pair fed `tverts2`/`tverts3` (up to 2.5.3) and lit as `fslit_nm`
  (findings, "What a custom shader gets of its mesh"). Since his area
  draws as the game draws it on stock content, what he sees comes from
  what his module loads over it (a hak's `tni01` models, textures or
  MTRs, a `lightcolor.2da`, a shader include) or is something other than
  the game's look: his own words for it, a picture of the view beside
  the game's, and his haks are what is needed. 2.13.1's lit check on his
  Vulkan (Radeon RX 580, AMD 25.5.1) drew its red mesh [212, 133, 133]
  through the game's `vslit`/`fslit` where this machine draws [204, 56,
  56] (Moonglow's own shader too): exactly the stock program's color
  with `fNdotV` zero, the surface lit as if seen edge on (the Fresnel
  term at a grazing angle, environment specularity 0.29 for 0.04, the
  diffuse unchanged), which no uniform, texture, buffer or uninitialized
  value of Moonglow's here gives (all are written or zeroed; findings,
  "The lit check's pink on Heed's Vulkan"). The check then read the
  programs through the whole load order, his `override/` and
  `development/` included: a builder's `inc_lighting` there draws so
  (`a_builder_s_include_leaves_the_lit_check_to_the_game_s_own`). It
  reads the game's own copies now and its debug-log line names any
  copy the load order puts over them, with each shader's color: that
  line from the next release says which of the two it was. (S) (Heed's
  reports, October 2026)
- **A custom shader's tangents, not every case read:** the client's
  rule (tangents from the model or a render hint, else zeros) was read
  with an ASCII model without tangents, with and without an MTR
  `renderhint`; a model that names `texture1` itself without a hint,
  and a compiled one, under a custom pair, weren't tried, nor the black
  of a normal map without tangents on Windows and macOS (the stock
  include normalizing a zero vector; that half waits on a Windows and a
  Mac client, or the renderer's CI branch). (S) (Heed's report, October
  2026)
- **Lights placed on the fly, the flicker** ([the proposal](lights-proposal.md)):
  way B is built behind its option (Custom Light…, `mg_module::lights`;
  the server reads the hak, finds the model in it and places the row:
  `engine_custom_light.rs`), and the client answered the proposal's two
  questions (2026-10-29, the third client batch): a `LightColor` light
  and a model's light node light creatures and placeables as they light
  tiles, so the window's "Lights creatures and placeables too" starts
  ticked; a draw takes as many lights as the client's Max Lights (32 by
  default). Not built: a flicker (the node's color or multiplier keyed in
  the model's `default` animation), which waits on reading whether the
  client plays that animation on a static placeable. Rows and models no
  light uses any more stay in the hak, as a row's number is what placed
  lights know it by (settle?: proposed by the sprint on 2026-10-29, the
  owner's to say). (S, waits on a client reading) (from builders'
  reports after 1.16.2)
- **Tile Properties on several tiles, in Aurora:** Moonglow writes only
  the controls set into every tile chosen (2.4.0's OK wrote every shown
  value, the first tile's, into all of them); how Aurora writes them for
  several tiles was not looked at on the oracle: it waits on a session
  driving Aurora. (S) (the owner's report on Tile Properties)

## Rendering: creatures, particles, water and shaders

- **Dynamic body parts from custom content (GitHub issue 5):** fixed
  with the reporter's haks over CEP 2.71. Under armor a creature's own
  part shows where the armor's is bare skin (part 1) or none, as the
  game shows a pale master's arm; the rule is taken from that and from
  the reporter's Aurora screenshot, which the previews now match, not
  measured in the client (nor what the game does for armor part 0 over
  a body part). A model whose appearance says full-body but whose
  skeleton has a creature's animations stands in `cpause1` (the
  werebat stood in its rest pose). Left: the report's werejackal wears
  a helmet whose model (`helm_129`) is in neither download, and is
  drawn headless; what the game draws for a missing helmet model isn't
  known. (needs the file) (from builders' reports after 1.10.1)
- **Emitters start as they look once going:** an emitter new to the
  view, an object's or a tile's, is run twenty seconds ahead so that it
  shows as it does once going (`scene.rs`, `PARTICLES_AHEAD`), and the
  nearest 256 objects and 128 tiles with emitters are simulated while
  Animations is on (`nearest`). Compared with the client on 2026-10-29
  (`reading_emitters_from_above` in `client_render.rs`'s `batch2.rs`,
  ttr01, the reference camera, Moonglow's view run ahead beside it): a
  wide fog (`new:Mist`) is as dense as the game's (the ground under it
  brightened by 18 of 255, the client's by 17). Left: Magic Sparks
  (`plc_u03`, `render Motion_Blur`, velocity 3) are long bright streaks
  in the client, from the side and from its steepest camera (pitch 1
  asked, which the client clamps), where Moonglow draws each spark as a dot of its size laid along its path (the ground
  under them brightened by 6 against the client's 13): the client
  stretches a motion-blurred particle by its speed, by how much not yet
  measured (a probe emitter at a few velocities, from the side). A tile's
  emitters take their keys from the animations playing on it as one
  animation as long as the longest (the game's tiles' keys are
  constant; one that changed over a shorter loop would run slow).
  (S, the streaks S–M) (from builders' reports after 1.14.1)
- **The game's shaders, compiled (M0 of [the shaders
  proposal](shaders-proposal.md)):** what the compiler (`mg-shader`) and
  Verify's `shader-compile` check leave out. (from M0, 2026-10-21; Verify
  compiles each program at six settings since 2026-10-29, and keeps what
  compiled, `doctor::settings`)
  - Arrays of samplers are refused (not split) where a view draws the
    program (Verify, which asks glslang alone, takes them), and
    `customShaderGS` (a geometry shader) is not read; no shader seen has
    either. (S)
  - What the engine does is assumed in three places, none checked in the
    client: an indented `#include` is spliced (14 of the stock ones
    are), a missing include fails the program, and splicing stops 16
    deep (`mg_shader::assemble`, `MAX_DEPTH`). (S) (waits on a session
    that runs the client: three programs put in a scratch user
    directory's `override`, as `light_uniforms_match_the_client` puts
    its, one with an indented include, one naming an include that isn't
    there and one at the end of a chain 17 deep, each drawn magenta or
    not)
  - A uniform that is a struct is described as `Other`, without its
    members (none in the stock blocks). (Locations, which glslang numbers
    in the order a stage first uses them, are settled in M1: vertex
    inputs bound by name, varyings matched by name.) (S)
  - The C++ build: glslang is in every program and test binary that
    links `mg-module` (about 40 s of C++ on a cold build, once).
- **The game's lit shaders drawn (M1 of [the shaders
  proposal](shaders-proposal.md)):** what `Renderer::game_shaders`
  leaves out (the Shaders switch, on by default since M3; Moonglow
  Viewer draws them since its 0.2.0). (from M1, 2026-10-21)
  - **The environment map's level under a normal map:** the WGSL port
    takes `_nm`'s derivatives from the normal map's normal before the
    parallax and the bend of a normal facing away, which the game takes
    after (2 of 255 at the 99th percentile). (S)
  - `gl_FragCoord.y` counts from the top in wgpu and from the bottom in
    OpenGL: counted from the bottom (in IR) only in a program that reads
    the framebuffer's copies (`fswater`, `fsparticle`), whose size gives
    the height. Of the other stock ones keyholing (off),
    `fs_shadowplane` and `fs_cg_btn_col` (not drawn) read it, and none
    of The Domains of Dread's 37. A screen-space custom shader that
    reads no copy would see it upside down; its height would come from
    `screenParams`. (S)
  - The height map's parallax direction on a tile's `rotatetexture`
    meshes (`inc_displacement` turns it by `mat2(m_texture)`) is not
    turned in Moonglow's own shader (`displace`; the normal map's is).
    (S)
  - `worldtimerTimeOfDay` counts real milliseconds and
    `sceneCurrentFrame` frames (measured, notes §B.4); Moonglow's clock
    starts both at 0 when the scene's time does. The client's timer is a
    count taken within the module's day (24 × its minutes an hour ×
    60,000 ms), whatever the module's start hour, day or dawn, but the
    count moves: about 17,697,400 as the player entered an area in 16
    runs until after 12:09 on 2026-10-29 (and on 2026-10-21), two hours
    more by 13:11 (`time_uniforms_in_the_client`). It is the phase of
    the falling water's flow and the grass's sway, which are not
    compared with the client frame by frame until it is known. (S: the
    timer read back at a few hours of the day, for what moves it)
  - The game's programs draw on CI's Metal and Direct3D 12 (the
    renderer's CI branch, 2026-10-29: three draws, no failures), but a
    discarding one's depth is not tested there: naga's `discard` from
    the game's, and the alpha discard `keep_alpha` adds (a bare `Kill`),
    run on, and on Metal a discarded fragment that runs on writes its
    depth (`a_shader_returns_where_it_discards` holds Moonglow's own
    shader to returning). (S: a see-through mesh's solid part through
    the game's programs, as `a_see_through_edge_does_not_hide_the_ground_behind_it`
    draws Moonglow's, on the renderer's CI branch)
- **Tier A of the game's shaders (M2 of [the shaders
  proposal](shaders-proposal.md)):** what the Shaders switch leaves out
  or unchecked. (from M2, 2026-10-21)
  - **The framebuffer's copies** (`texFBColor`/`texFBDepth`, [the
    framebuffer proposal](framebuffer-proposal.md)) are made after the
    solid meshes only. A material's `sample_framebuffer` (TDoD's two
    flow MTRs; their programs read no copy) is read and not drawn late,
    after the shadows, as the engine draws it (S); whether the client
    copies again after the water (a see-through mesh or a particle over
    water sees the solid meshes only) wasn't read (S) (waits on a
    client reading: a see-through mesh reading `texFBColor` over water,
    whether the water shows in its copy).
  - The tangents the renderer makes (a mesh with a render hint and none
    of its own) are summed per face (`made_tangents`), where the game's
    compiler copies a vertex whose handedness flips (`engine_tangents`,
    MDL-MESH-043): whether the client makes an ASCII model's so at load
    is not known, and a seam may shade otherwise. (S) (waits on a
    client reading: an ASCII mesh with a mirrored UV seam under a render
    hint, its tangents written as colour through a patched include,
    beside the same model compiled)
  - Pictures already made (the galleries' thumbnails, a hovered row's
    preview) are kept across a flip of the switch, drawn as they were,
    until they are asked for anew: `set_game_shaders` changes the
    renderer only. (S)
- **The frame budget of the game's shaders (M3 of [the shaders
  proposal](shaders-proposal.md)):** what the Shaders switch, on by
  default, leaves over the budget or unchecked. (from M3, 2026-10-21)
  - **A dense area seen whole is still slower with the switch on.**
    TDoD's synthetic woods (24 by 24 `mst04` tiles, 400 placeables,
    33,000 meshes on screen) zoomed out: 15.8 ms a frame on against 13.1
    off (was 34 and 26), turning 28 against 21 (was 51 and 36); the GPU
    3.7 against 3.5 (was 6.1 and 7.3). Both over the budget; the bar (on
    no slower than off) not cleared. The game's draws bind two groups a
    material change and more pipelines (796 changes against 349), write
    their values through each program's plan, and are more draws (a
    swaying program's box grown by a meter joins fewer). `NO_DISCARD` 0
    for every material (notes §M.10) not looked at. ([the frames
    note](research/notes_frames.md), 27 October) (M)
  - **A program that moves its vertices farther than a meter** is batched
    by its box grown by `MOVED` (1 m): TDoD's grass lifted by
    `fTilesetMeshOffsetZ` (2 m) could be blended in another order where
    it reaches past it. Its MTR's parameters could say how far. (S)
  - **An area changes its look a moment after it opens**, the first time
    its programs are compiled (stock: 50–113 ms of the worker's; TDoD:
    250–277 ms) and, with them cached, while naga reads them and the
    driver makes the pipelines (28–40 and 123–146 ms): Moonglow's own
    shader draws until they are ready. Making the pipelines of the
    programs a module needs when it opens, or keeping the drivers'
    pipeline caches (wgpu's `PipelineCache`), would shorten it. (S)
  - `mg` and `mgv`'s previews compile the programs each run: no shader
    cache is set outside the window (`Renderer::set_shader_cache`). (S)
  - A program whose uniforms can't be split (its block used otherwise
    than member by member: none of the 102 seen) falls back to
    Moonglow's shader, reported, rather than drawing with one block. (S)
  - **Pictures drawn once wait for the pipelines** (a gallery's or
    palette's thumbnails, a hovered row's preview: drawn once and kept,
    they can't take the fallback): the first ones a session needs block
    the UI thread for the worker's time (28–113 ms for area092's pairs).
    `frames_at_144` with the default on (nothing over the budget by its
    median) shows the Gallery's worst frames while scrolling at 10 and 49
    ms; not yet compared with the switch off. (S)
  - The tests that compare pictures of the area view and the model
    viewer choose the switch off; only the switch tests and TDoD's wind
    test draw the game's shaders through the window. (S)
- **The engine's choice of program (M4 of [the shaders
  proposal](shaders-proposal.md)):** what M4 left to read in the
  client. (from M4, 2026-10-21)
  - **A static placeable's meshes** sharing a texture seemed to share
    one `NO_DISCARD` in the client (the plain and transparency-hint cards
    of a static copy were alpha-tested beside an `alpha 0.5` card of the
    same texture): the game bakes static geometry into buckets (.37).
    Moonglow decides it per mesh. The cards read as a static and a
    dynamic placeable (2026-10-29, `program_choice_in_the_client` with
    and without `MG_STATIC=1`): three of 36 differ, each one whose
    texture and pair another card shares (`mesh alpha 0.5` and `decal
    red alpha 0.5` alpha-tested static, blended dynamic; `envmap` the
    other way, beside `envmap+mesh alpha`); which mesh of a bucket
    gives its value isn't plain from these (the first of `mgp_plain`'s
    and `mgp_decalr`'s, not of `mgp_env`'s). Not read: tiles. (S)
  - Not read in the client: which map an item on the ground reflects (it
    draws `_sm`, as Moonglow's `chrome1` has it; the band didn't reach
    it). (`cameraFocus` was read back as the player's place, (20, 20, 0)
    with the player there, in four runs on 2026-10-29: what Moonglow
    uploads, its camera's target.) (S) (waits on a client reading: a
    patched include painting `texUnitEnv` on an item's own fragments)
  - Moonglow's own shader draws as the client does since M4 (Settled),
    not yet looked at in the builders' areas: TDoD's DXT1 textures (690)
    with their transparent texels black, and its 21 materials naming
    maps without a render hint. (S) (waits on a client reading: a TDoD
    area with both, beside the client's picture)
  - Against the client, the switch on is as close as off on the stock
    scenes and closer on TDoD's room, but for its far floor (1.0 of 255
    from the client on, 0.4 off), beside a dark wall where the fitted
    camera is a fraction of a pixel off. (S) (waits on a client reading:
    the room with the camera fitted to that wall)
- **The special surfaces (M5 of [the shaders
  proposal](shaders-proposal.md)):** water, grass, the sky, additive
  meshes and particles, read in the client (`surfaces_in_the_client`)
  and drawn with the game's programs or kept Moonglow's. (from M5,
  2026-10-21)
  - **Shiny water is the game's** (`vswater`/`fswater` with the
    framebuffer's copies) with the Shaders switch on: a rural area's far
    water 10.9 of 255 from the client against Moonglow's own 12.6,
    TDoD's fountain 10.6 against 11.7, a standing card 14.6 against
    57.1. Still 10.9 away: not looked into. (S) (waits on the client
    batch: the far water's pixels against Moonglow's, by part: the
    reflection, the refraction's copy, the fog)
  - **Soft particles** fade as the client's with the Shaders switch on
    (`particleSoftness` 10 over the emitter's larger size); what the
    client gives lightning and `Linked` emitters wasn't read (Moonglow
    gives lightning none); a Lighten particle's foot fades a little more
    than the client's (6.2 of 255 away over its last rows). Moonglow's
    own particle shader (the switch off) doesn't fade. (S) (waits on a
    client reading: `particleSoftness` read through the marked
    `fsparticle` for a lightning and a `Linked` emitter)
  - **The sky's fade** (the horizon's band in the fog's colour) stays
    Moonglow's: what the engine draws it with and gives it
    (`globalColor`, its blending) wasn't read; the skybox is the game's
    `vsvt`/`fst`. (S) (waits on a client reading)
  - **Particles' size and turn:** a still emitter's square particles
    are drawn upright, as in the client, 28 pixels a side against the
    client's 30 in `surfaces_in_the_client` (the fitted camera's error,
    or the size: not told apart). A spinning particle's starting angle
    wasn't read. (S) (waits on a client reading)
  - **The grass** is the game's program on Moonglow's tufts: their
    places, sizes and count are Moonglow's (the field's mean colour 0.1
    of 255 from the client's, but per pixel 17.6 against Moonglow's own
    17.0). The material colours the engine uploads (`.set`
    [GRASS]'s) are not read through a patched `vsgrass`. How far it
    draws grass is read (`reading_grass_placement` with
    `MG_GRASS_DISTANCE`, 2026-10-29): the client's Grass Far Render
    Distance (`graphics.grass.render-distance`, 900 by default and at
    the least, to 30,000) is in square meters, the grass fading out over
    the last fifth or so of its root from the eye: at 900 the white
    tufts are whole to about 22 m and gone by 28 to 30 m (the "15 to 20
    m out" seen from the player), at 3,600 they reach the horizon of
    the 4 by 4 area. Moonglow's go on, whole, to the end of the view,
    as the owner asked of an editor's camera on 2026-10-23 (grass seen
    only about its target was what he took for a fault; from an
    editor's height, 30 m from the eye is no grass at all) (settle?:
    proposed by the sprint on 2026-10-29, the owner's to say: kept to
    the view's end, not cut at the client's distance). (S)
  - **Cube maps have one level** (`GpuTexture::upload_cube`), so the
    level Moonglow's shader works out for them (`cube_level`) and the
    bias the game's programs add come to level 0 either way: a rough
    surface's cube reflection isn't blurred. The stored levels would be
    uploaded. (S)
  - **The switch on still costs the water view about 0.9 ms of
    encoding** (3.6 ms against 2.7 off, at 6.5 ms a frame within the
    budget by 0.4): about 0.75 ms the draws' own values (the material's
    template copied, `m_mv`, `m_mvp` and `m_normal` with an inverse each
    draw), the upload 0.09 ms, the submit 0.07 ms. `m_normal` from the
    3×3 part, or values that the camera alone changes kept per instance
    and multiplied in the vertex program, would cut it. (M)
  - The area views saved with `FRAME_SHOTS=1` are not comparable from
    one run to the next where tiles animate (up to 31 of 255 over
    area092's sea): they are drawn at the running clock, where
    `FRAME_PARTS_SHOTS` draws at a fixed one. A fixed clock for the
    shots would make them comparable. (S)
- **Emitters a builder does not see** (GitHub issue 8: a demon's wings,
  braziers, shafts of light, all without particles in his area): not
  reproduced. The game's own balor, fire elemental, campfire and brazier
  show theirs in the area view here (`look_particles_of`), so it is his
  content, a setting (Animations off shows none) or his machine; asked
  of him. (?) (from builders' reports after 1.16.2)
- **A `Linked` ribbon and a flip-book** (a builder's custom fog was
  drawn as bright streaks): what the client draws is in [the
  findings](findings.md) ("Particles: kept, flip-books and ribbons"),
  and the fog shows as an even haze, as in the game. Not looked at: the
  turn of the picture within a ribbon's piece (the client's looked
  mirrored beside a plain particle's), and how the client orders a
  ribbon's pieces among other see-through things. A model with no mesh
  to draw (this fog) has no bounds of its own to frame a view by. (S)
  (from builders' reports after 1.19.1)
- **Client tests' scripted cameras:** the client takes a
  `SetCameraFacing` only when it is set a second after the player
  enters ([findings](findings.md), "An upright particle turns to the
  eye"; the module's `Mod_Entry_Dir_X` and `_Y` turn it too, as the
  program-choice tests do); `particles_look`'s closer camera,
  `grass_and_sky_look`'s
  `MG_CLOSE` and `grass_overrides_look`'s `MG_DISTANCE` and `MG_PITCH`
  set it at once, so their pictures were likely taken from the client's
  own camera. Not changed: the numbers measured from them (in
  `docs/research/notes_models.md`) would be taken again with it, in a
  session that runs the client (the sprint's fix waves of 2026-10-29
  ran none, other sessions having the off-screen display). (S, waits on
  a client session)
- **Builders' reports after 1.21.0:** a creature's body part can be
  set to none (0), as in Aurora; a body part whose mesh is a skin is
  drawn as the client draws it ([findings](findings.md), "A skin mesh
  in a body part or a held item": not where it is compiled, a plain
  mesh where it is text; a held item's never). Not done: a skin
  weighted to nodes of its own part, which the client draws out of
  place, is not drawn; the compiled models tried were `mgv compile`'s,
  not the game's own compiler's. Not reproduced: "unequipping an armor
  sets all body parts to 0": nothing in the code writes a body part on
  unequip, and the preview draws the creature's own parts then; the
  creature it was seen on would tell. (S)
  (from builders' reports and the owner's notes after 1.21.0)
- **See-through meshes** are drawn in two parts: what is nearly opaque
  of them (alpha from 0.95) with the depth written, then the rest over
  everything solid, back to front, writing none. (A builder's report:
  blue-white patches round plants and along shores at a slant, in a
  base-game tileset: a plant's soft edge drawn before the ground behind
  it kept the ground from showing, and the water under it showed. Shown
  in a renderer test; not confirmed on the reporter's area, which isn't
  here; compared with the client on a rural shore, ttr01's tile 7 and
  its water (`reading_shore`, 2026-10-29): no patches in either, the
  water as the client's with the Shaders switch on.) Two see-through surfaces crossing each other still
  sort by their middles. Each see-through mesh costs two draws. On
  macOS the solid part's discarded fragments wrote their depth (the
  plant's edge hid the ground again) until the shader stopped at each
  discard: seen and mended on the CI's Mac, a virtual one; no real Mac
  was looked at. The
  smaller levels made of an uncompressed texture leave transparent
  pixels' colors out; a compressed DDS's own levels are used as stored.
  (S–M)
- **Custom creatures from the Vault and CEP** (a builder's report,
  [issue 3](https://github.com/jadzziaa/moonglow-toolset/issues/3)),
  fixed against the content itself but none of it compared with the
  client: a supermodel's position keys scaled by `setanimationscale`
  (taken as the size against the model whose animation is played; a
  chain of scaled supermodels wasn't tried); a body part drawn only where
  the base has its node; a mesh with `bitmap NULL` given the part's own
  texture; a bitmap name over 16 characters cut to 16; a blank line among
  ASCII skin weights read as a vertex without any (a blank line that is
  only spacing in a list with all its rows would shift the rows; such a
  vertex's faces are not drawn since 2026-10-29, as the game draws none
  of a vertex no bone weighs). The
  reporter's "stopped animating" (Mindwitness) wasn't seen as such: here
  it animated, with its skin torn by the shifted weights. The model
  viewer's own check of names over 16 characters is as it was. The
  report's Mindwitness also poses differently than in Aurora and the
  game; not reproduced without the model. (S; the pose needs the
  files)
- **Shadows:** the area view draws the sun's and moon's shadows
  (2026-10-28: a shadow map standing in for the game's stencil volumes,
  laid as the game's shadow plane lays them, checked against the client
  in `shadows_look`; [the research note](research/notes_shaders.md) §G),
  and creatures' and lights' shadows (the Shadows button's menu, done
  2026-10-28). Left:
  - the lights' shadows fall off with the lamp's distance, where
    Moonglow's darkness is the opacity times the plane's fade alone: an
    ogre's shadow from a lamp post (row 73) 1.5, 3 and 5 m north of it
    darkens the floor just south of its feet (the same place on the
    screen) by 0.50, 0.53 and 0.38 (medians; `reading_light_shadows` in
    `client_render.rs`'s `batch3.rs`, 2026-10-29), and grows longer as
    the lamp goes further (the rule, `shadowradius` or the light's reach,
    not fitted: S);
  - which light of two equally near the client takes isn't the first of
    the area's (Moonglow's): with two lamp posts 2.5 m west and east of
    the ogre, in either order in the area's file, it cast from the west
    one, and of two north and south, from the south one (the ogre facing
    south; the server places all three where they were put, so it likely
    measures from the creature's model, not its feet: not read further);
    how many creatures cast the lights' shadows at once
    (`max-casting-lights` changed nothing with one; Moonglow: 16), and a
    PC's own light at night (Moonglow places no PC) (S) (waits on a
    client reading);
  - what is drawn after the shadows receives none: the area's grass
    (see-through tufts: in `shadows_look` with `MG_SHADOW_GRASS=1` the
    shadowed ground darkens by 0.34 to 0.38, the client's by 0.42, its
    grass sparser and darkened), the solid parts of a tile's see-through
    meshes (alpha-blended leaves), shiny water drawn after the
    framebuffer's copy; the client's volumes darken whatever was drawn
    before its plane (M: those drawn later would read the map
    themselves);
  - with Fade Geometry on, the faded roofs and upper walls cast no
    shadow, where the game's still do (S);
  - a sun low in the west (a GIT's direction,
    `reading_git_sun_direction`): the client leaves a city street lit
    but for thin spikes, where Moonglow lays the buildings' long shadows
    over all of it, light showing through in thin lines. Read further
    (2026-10-29, `reading_low_sun_shadow_length`, a sun from the west 14,
    27 and 45 degrees up): a white pillar's shadow, static or not, 4 m
    high or 8, is as long in the client as in Moonglow at every height
    (33 m for the 8 m pillar under the lowest), and so is a rural tile's
    hill's (ttr01); but tcn01's buildings, whose shadows cover half the
    street at 45 and 27 degrees in both, cast none on it at 14 in the
    client, only the posts' and trees' spikes. So a tile's shadows go as
    the sun sinks, which Moonglow's map (its bias tried at 0.5 m: no
    change) has no reason to do; the client's rule isn't read: its
    stencil volumes of these tiles' meshes (open ones, the wiki's
    "shadow tearing"), or a reach of its tiles' precomputed projections
    (`ComputeStaticProjections`). (M: the reading, a tcn01 building tile
    alone on a flat area at a few heights of the sun, its `shadow`
    meshes looked at, then the rule; waits on the client) (The fade with
    the height between plane and caster that 8193.31 names is not seen:
    armoires raised 0, 1.5 and 3 m darken the ground alike, 0.44 to 0.49,
    and casters above the camera still cast, as in Moonglow:
    `reading_shadows_of_doors_items_and_heights`, 2026-10-29);
  - the previews (blueprints, the model viewer) draw none (S).
  (asked by the Domains of Dread module session and by a builder,
  Troubletcat)
- **Where a cut-out is cut:** at alpha 0.2 and solid where it is not
  cut; that, two-sided faces and decals as the client draws them are in
  [the findings](findings.md) ("Two-sided faces, cut-outs and decals").
  Left: the tiles and the armoires were not
  painted by the probe (another shader path: their value was not
  read); a cut-out that is also see-through as a whole (a mesh's alpha
  under 1, a transparency hint) is still blended, rim and all, which
  was not compared. Cut-outs are no longer sorted back to front with
  the see-through meshes: they write their depth as solid meshes do.
  (S)
  (asked by the Domains of Dread module session; checked in the client,
  2026-10-09)
- **A rippling glow (TXI `proceduretype arturo`) does not look like the
  client's:** `dd_ce3_zombieash`'s `dd_ce3_txhotcoal` (additive;
  `channelscale` 0 0 0 0, `channeltranslate` 1 1 1 1, `distort 1`,
  `arturowidth`/`arturoheight` 32, `distortionamplitude 10`, `speed 16`,
  no `defaultwidth`; its TGA and DDS alike, 256 by 256, alpha 37 of 255
  on average) shows in Moonglow as bright red and yellow patches over
  the body, the texture as it is (`area_creatures_game_shaders` in
  `screens.rs` with `MG_SCREEN_APPEARANCES=28518` and the module's
  `tdod_top` and `tdod_p_ce3`, 2026-10-29), in the client as dim orange
  veins, with either shader (a rippling texture is Moonglow's). Not
  ported, waiting on a client reading: the procedure is the engine's
  own, on the CPU, and the wiki says only that with `distort 1` the
  channels' scale and translation are ignored and the amplitude is in
  texels; Moonglow takes it as a part of `defaultwidth` (64 without one:
  10/64 of the picture here, where 10 of its 256 texels is a quarter as
  far). A ripple moves the picture and doesn't dim it, so the
  difference may be the blending: Moonglow added an additive texture
  whole (One, One) when this was seen; it weighs it by its alpha since
  2026-10-29, as the client does (`reading_additive_blending`), which
  may dim the zombie as the client's (not looked at again). The game's 39
  arturo TXIs all name a scale and a translation; `distort 2` is only
  `fxpa_globe*`'s, `distortangle` only the decal skins'
  (`c_water_skin`, `vdu_tex_shade`). **What the client batch is to
  read**, `creatures_look` with the zombie (`MG_OVERRIDE` with its model,
  textures and appearance.2da, `MG_APPEARANCES=28518,28518`): (a) as it
  is, a few frames apart; (b) its TXI without `proceduretype` and
  `distort` (still dim: the blending, not the ripple); (c) a flat card
  with probe TXIs, one keyword changed at a time (`distort 0` with this
  scale and translation: white or the texture; `distortionamplitude` 10
  against 40, with and without `defaultwidth`; `speed`), for how far and
  how fast it ripples. (The client binary's symbols name the procedure,
  `ArturoTextureController`: reading its code, where the client's
  pictures are what Moonglow reads, is the owner's to say.) (M: the
  reading, then the port)
  (from the Domains of Dread's report on the flickering zombie, 2026-10-22;
  seen in the client, `creatures_look`)

## Models and textures

- **Heed's previews after 1.33.0** (not reproduced): the Drow shields of
  the standard palette shown as bags in their thumbnails and the
  palette's hover preview. In the base game they show whole
  (`item_previews` in `screens.rs`: `x2_adrowshl001`), so his haks are the
  likely cause (a `baseitems.2da` of their own); his hak list or a
  blueprint is needed. (His armor showing only a head and pelvis was
  Beamdog's HD bodies, drawn whole since 1.34.1.) (S–M)
- **Where mg-mdl still differs from nwn-mdl-spec** (August's pointer to
  the shared spec, 2026-10-19; [the comparison](research/notes_mdl_spec.md)):
  its suite passes every probe since 2026-10-29, in Moonglow's tests
  (`the_model_spec_suite_passes`). A skin vertex with no valid bone,
  which the game collapses to clip-space 0 (MDL-SKIN-030), was drawn at
  rest; seen in the client (`reading_wings_without_bones`, 2026-10-29: a
  human with `c_wings2`, whose right wing, 102 vertices, has no bone
  that is a node), the wing isn't there, and Moonglow draws none of such
  a vertex's faces since (`a_skin_s_faces_with_a_vertex_no_bone_weighs_are_not_drawn`).
  That a face with one such vertex among weighed ones goes too is taken
  from the collapse, not seen: of the game's 1,778 skins only that wing
  has any such vertex, all of its own.
  Left: the wing it keeps stands otherwise than the client's (spread
  where the client's hangs, seen from the side; the demon's wings beside
  it alike in both): its pose, not looked into. (S)
- **The model compiler and writers in `mg-mdl`** (moved from the
  viewer's `crates/mgv-mdl` at its 0.1.6, 2026-10-09; in use since
  1.20.5). Left: `mg pack` (and a nasher project's Pack, by its project
  file's `[pack] compile_models`) compiles models only when asked, as
  the game reads text and a pack's bytes should not change unasked: on
  by default later? (settle?: proposed by the sprint on 2026-10-29, the
  owner's to say: kept off by default)
- **Animations bind by part number, not by name:** done (by name
  where either is from text, and for a worn cloak or robe).
  Seen in the viewer session's client pictures (a
  skeleton numbered afresh under the same names leaves its creatures
  unposed), not measured again here; a human and an elf in armor and
  cloak still stand as the client draws them. Of the game's models, 70
  have nodes the two ways move differently, none of them looked at one
  by one. A node an animation numbers −1 is not moved, on the reading
  that the game moves none for it. (S)
  (proposed by the Moonglow Viewer session, 2026-10-07: its
  `docs/PLAN.md` §10.1 has the full text; not checked here beyond that
  the code named is there)

## The main window and its panes

- **What a module's tabs open again without:** a module opens again with
  its tabs, side editors and tools, split windows and views as they were
  (Options › General, `crates/mg-ui/src/open_tabs.rs`), a script's
  bookmarks, the talk table's line and a Tile Properties still waiting
  for its area's view among them. Not kept, each an optional field of
  `open_tabs::View` written by `view_of` and taken at the view's first
  draw (S each): what a side editor's view showed (a hak's selection, a
  2DA's cell, a tileset's page) and a model view's camera. Advanced
  Search's question (the next session's tab is empty) wants its
  conditions kept as text, nine kinds with their rows, parts and ranges
  (`search::Condition` has no serde form) (S–M). What is kept as it is
  by design is Settled. (a builder's report, October 2026)
- **The light theme's own colors:** it is egui's light theme; the script
  editor's syntax colors follow it, but conversation and faction colors
  are the dark theme's (legible on both), and the area view's overlays
  are drawn for the 3D view. A pass with a builder who uses it would
  settle the rest. (S–M) (a senior builder's review, October 2026)
- **Panes folded away** (GitHub issue 10, open: waits on its reporter's
  retest of the one-button rule, built on 1.31.0): a View menu (Module
  Tree, Palettes Panel, Log, ticked while shown; Hide All Panels; Reset
  Layout), with keys (Ctrl+Alt+1, 2, 3 and 0, in Options › Keyboard); a
  pane folded away leaves a strip at its edge that brings it back, as
  wide as it was; the palettes' pane closed by its ✕ counts as folded;
  the state is kept in the settings. Each pane has one button of its
  own to fold it away, by the reporter's rule: a chevron on the edge it
  faces the middle by (the tree's right, the palettes' left, or right
  where they were split off the middle's left, the log's top), centered
  along it, in a thin strip that is all a folded pane leaves, the button
  in the same place. egui_dock's arrow on the palettes' tab bar is
  covered over and its clicks taken (egui_dock has it on every pane or
  none, and windows fold to their bar by it), and the keyboard passes
  it (Tab and Shift+Tab, `the_palettes_covered_arrow_is_passed_by_the_keyboard`).
  Left for the reporter's word, as their rule shapes each (S each): the
  strip names its pane only in the tip (a name along a 16-point strip
  would be drawn turned); the palettes' bar keeps the arrow's blank room
  before its first tab, and the middle's panes keep their arrows
  (egui_dock offers no per-pane choice: a patched egui_dock, or the
  middle's arrows covered the same way); palettes joined to the middle's
  tabs, split above or below it or in a window have no edge button (View
  and their ✕ fold them), and those split above or below leave their
  strip on the right. (from builders' reports after 1.19.1)
- **The menus by the keyboard** (GitHub issue 9), on egui's own menus:
  Alt alone or F10 opens the first menu and closes them (over an area's
  view F10 is Select Tiles', as in Aurora), Left and Right
  go between menus (round the ends), Up and Down between rows (past
  separators and rows that can't be chosen), Enter and Space choose,
  Right and Left open and close a submenu, and the pointer over another
  menu's name opens it while one is open. The marked row is drawn as
  selected. Alt and a menu's first letter (underlined) opens it (unless
  a command was given that key in Options › Keyboard; not AltGr), a
  letter with a menu open goes to the next row that begins with it, and
  Escape closes the submenu the keys are in before the menus; all of it
  under test (`menus_by_the_keyboard`, Alt alone by its modifier
  events). Waits: Alt alone on Windows, where the system may take Alt
  for its own menu (a Windows build). (S) (from builders' reports
  after 1.19.1)
- **The middle with a tab beside the palettes** (issue 11): the middle
  was let go where another tab (the resource browser) shared the
  palettes' pane; the pane's tabs no longer count as the middle's. The
  palettes dropped among the middle's areas are the middle's (issue 20:
  they were taken for the side pane, and the areas pushed to the
  right), and the resource browser's pane alone is a side pane once the
  palettes are closed (issue 23: it took the whole window). Another
  tool docked alone on the right (the talk table, a script) is not
  known as a side pane. Issue 21's drop on a side target is docked
  there from a pane; from a window of its own, the window was seen to
  move and not to dock, by the tests' drag (not confirmed either way in
  the window). (S) (GitHub issues 10 to 24, after 1.20.0; 20, 21 and
  23 after 1.30.2) (the window's drop waits on a drag by hand in the
  running program, the owner's or issue 21's reporter's: the UI tests
  can't make a window of its own, and issue 21 has no answer since
  2026-10-10)
- **The module tree and the palettes by the keyboard** (GitHub issue
  12, after 1.20.0, and a builder: Aurora's arrows go to the parent on
  Left and fold the tree): after a click in a list (the module tree, a
  palette, Module Properties' haks) the arrow keys, Page Up and Down,
  Home and End are that list's wherever the pointer is, until it is in
  an area's view, and a thin outline shows the pane; Right and Left
  open and close a row and go in and out, into an area opened out too;
  Enter opens, Escape hands the keys back; F2 and Delete are for the
  row under the pointer, else the cursor's; both panes' filter fields
  have Ctrl+F, Escape (clear, then leave) and Down. F6 and Shift+F6 give
  the keys to the next pane, each pane's being what its tab in front
  has (the middle's a talk table's lines, a 2DA's cells at its first
  cell, the resource browser's list, else an area's; a palette's cursor
  at its first row); in the Gallery Up and Down go a row of pictures at
  a time (done 2026-10-29). Not done:
  - F2 at a placed object does nothing: its row's menu has no Rename,
    nor has Aurora's object menu (the parity inventory); whether
    Aurora's tree edits an object's label in place on F2 (its
    `tvMainEditing` renames resources) was not watched. Delete there
    deletes it as its menu does. Waits on the Aurora oracle (not used
    in the fix wave of 2026-10-29). (S)
  - With the keys on a category the blueprint selected stays in hand;
    whether Aurora keeps one in hand then was not checked. Waits on the
    Aurora oracle. (S)
  - The log is not among F6's panes: it has no keys of its own (its
    lines are text to select and copy); a command a builder already
    gave F6 to is in conflict with Next Pane (Options › Keyboard lists
    it; the pane's wins until one is changed). (S)
  - Down from the hak editor's, the tileset editor's and the Appearance
    Gallery's Find fields goes nowhere: their lists have no keys of
    their own, nor have the tileset's palette and the Prefabs list.
    Keys for four lists (a cursor, its outline, Enter's action, the
    rows' order for Up and Down in each), about a day: more than the
    wave's S. (M)
- **A long dropdown that grew to the window's edge, not seen growing
  here:** the open list of a dropdown with a Filter field (Area
  Properties › Audio › Ambient Sound, Day) widened over 5 to 10 seconds
  under Windows. On Linux it grew once, by its scroll bar's width, on the
  frame after the one that measures it, and never again (100% to 175%
  interface sizes, frames of 1/144 s, the window at fractional
  positions, the pointer moving and the wheel turning over it); its
  width is now set when it opens, the same every frame, which closes
  the way any growth took. To check under Windows at the builder's
  interface size. The other dropdowns (no Filter field, no list of
  their own inside) keep egui's sizing, which can grow by the scroll
  bar's width once. (S) (Troubletcat's report, October 2026)

## Languages, tokens and loading screens

- **The game's text in the language edited:** the install is read as
  that language when its `lang/<code>/data/dialog.tlk` is there (its
  talk table, and the rest of its `lang` folder); a module's custom
  talk table is one file whatever the language (checked for each of
  the five other languages installed, `the_install_s_language_is_the_one_edited`).
  Whether the feminine table (`dialogf.tlk`) is shown where the game
  would use it was not looked at: the game reads it for a woman's
  character, which the editor has none of. (S) (waits on Aurora: whether
  it ever shows the feminine table's text, a line by StrRef in its
  conversation editor say; if never, this is settled)
- **A module's `encoding.2da`, what is left by choice** (EE 1.87: a
  table of the Unicode character each byte of the game's text stands
  for; a builder who knows the engine pointed it out, after 1.19.1):
  the toolset, plugins and `mg` read and write a module's text by it
  wherever they are given the module (`mg gff` and `mg lsp` with
  `--module`, the name `mg nwsync` publishes), as the server reads it
  (`engine_encoding.rs`: from a hak or `override`, not the module file;
  a missing row the game's language's letter, a Polish game's
  Windows-1250 one; `game.language.codepage` honored). Kept as they are
  by choice, for the owner to settle: a prefab's file keeps the bytes
  (written as Windows-1252 text, so that it is the same object in any
  module); a hak's description is the hak's, not the module's; a nasher
  project's sources are converted by its `nasher.cfg`'s
  `--nwn-encoding`, as nasher's own tools convert them, so that the
  bytes nasher packs are the module's; and the toolset reads the game's
  text by Options › Language, not by the user folder's
  `game.language.codepage`, which the server honors and `mg lsp` reads.
  An exported or imported conversation is spelled over as a whole, its
  punctuation too (of no matter unless a table moves ASCII's own
  characters). Left: a text field already being typed in when the hak
  list changes keeps what it showed until it is left. (S)

## Remappable keys

- **Shift+Insert in the window itself:** it asks the backend for the
  clipboard (`RequestPaste`), and the paste comes a frame later; the UI
  tests stand in for the backend, and the keys were not pressed in the
  running program on Linux, Windows or macOS. Shift+Delete in the module
  tree no longer deletes the resource (it is a cut, which the tree does
  not take). (S) (Sacha's report on 2.5.4) (waits on the keys pressed
  in the running program: the owner's on Linux, Sacha's on his system)

## NUI Creator (the review of its pull request, October 2026)

The NUI Creator came as a contributor's pull request and was brought in
line with the codebase before it was merged: its frames (26 to 35 ms after
an edit, now 1.4 to 2.4), one Undo step a field, the toolset's labels,
headings, name window and OK and Cancel, typed errors and lossless
settings in `mg-nui`, Build through the module's compile path, its tests
where the crates keep theirs, and a client test of what its checks say of
the game. This is what that review left, and after it what the
contribution itself states as not done or not checked.

- **A picture cache of its own** (`crates/mg-ui/src/nui_view/skin.rs`):
  `skin::Assets` keeps its textures beside `images::Loader`
  (`crates/mg-ui/src/images.rs`). Both find a picture through the game's
  resource manager with the module being edited in its place, below its
  haks (`ResMan::get_over` and `texture_over`, since 2026-10-29); their
  orders of types differ on purpose (a window's pictures JPG, TGA, PNG,
  BMP, then DDS, as the client looks for a NUI picture; the loader's DDS,
  then TGA, as for a texture). What is left: the loader keeps a picture by
  name, not read again when the module's bytes of it change, where a
  window's are (a portrait added to the module shows once the game data is
  reloaded); and JPEG and BMP are still read with the `image` crate
  (nothing else here reads them). (S)
  - *15- and 16-bit TGAs*, read with the `image` crate before, come out of
    `mg-image` a level off in places and with their attribute bit as
    alpha: `mg-image` makes a 5-bit channel 8 bits as `(c << 3) | (c >> 2)`
    and takes the top bit as alpha when the descriptor declares an alpha
    bit; the `image` crate's table is c × 255 / 31 rounded (3 is 25, not
    24) and it never took the bit as alpha. Read in the client
    (2026-10-29, `nui_readings_in_the_client` in `client_render.rs`'s
    `nui.rs`: a NUI image of a 16-bit TGA whose descriptor declares an
    alpha bit, at its size): the game draws level c as c × 255 / 31
    rounded down (4 is 32, 8 is 65, 30 is 246, where `mg-image` gives 33,
    66 and 247 and the `image` crate 33, 66 and 247), and the pixels with
    the bit clear opaque, as those with it set: it takes no alpha from the
    bit. `mg-image` differs on both; a model's texture of the same file
    was not read, and `mg-image` decodes it for the renderer too, so the
    fix (the table, and the bit not alpha) waits on that one reading. (S)
- **Where the preview is known to differ from the game** (stated in
  [the manual](manual/18-nui-creator.md);
  `crates/mg-ui/src/nui_view/preview.rs`; `mg nui render` and `layout`
  show the canvas, with these differences; since 2026-10-30, as the
  client was read on 2026-10-29: a control's size and margins whole
  pixels at a UI scale that isn't whole, a row's last button stretched
  to a wider control below it, numbers in text a 32-bit float's digits,
  ties to even):
  - *An encouraged control* shows its hover look and does not pulse; the
    pulse's period is not measured either. (settle?: proposed by the
    sprint on 2026-10-29, the owner's to say: a still preview shows the
    pulse's peak.)
  - *`mg nui layout`'s `overlaps`* come of the canvas's own layout, which
    places neighbors apart: it finds none but where the game would lay the
    window out otherwise, which it can't know. (settle?: proposed by the
    sprint on 2026-10-29, the owner's to say: said in the manual's command
    line chapter, as it is.)
- **Left by The Domains of Dread's NUI Creator asks (October 2026)**
  (AccessKit; the asks are the module project's
  probe, `tdod-modulework/docs/systems/nui-creator-probe.md`; done on
  2026-10-29: a state for one row, variants of several groups, binds held
  to their properties' rules, a list's rows out of view and the title in
  `layout`, the layout's schema, the Creator's Import…, the hand-run
  `a_module_s_windows_are_laid_out_as_json` gone; moved: "`mg` links
  `mg-ui`" to "`mg` builds the GPU stack by default", "the preview is not
  the game" and `layout`'s `overlaps` to "Where the preview is known to
  differ from the game"):
  - *AccessKit doesn't start on newer Linux desktops*: eframe 0.36's
    `accesskit_unix` (0.21) offers the tree only when the session bus's
    `org.a11y.Status.ScreenReaderEnabled` turns true, and at-spi2-core
    2.62 (the development machine's) has only `IsEnabled`, so there it
    never starts; `accesskit_unix` 0.24 watches `IsEnabled`, and comes
    with an egui/eframe whose `accesskit_winit` uses it (0.34; eframe
    0.36.2, the newest on 2026-10-29, has 0.32). Until then the desktop
    agents here can't read the window. (S, with that upgrade)
  - *AccessKit was checked on Linux only*: the GUI checks for Windows
    (`x86_64-pc-windows-gnu`) and eframe's AccessKit for macOS
    (`aarch64-apple-darwin`); CI builds them, and no one has read the
    tree with a screen reader on any of the three, nor in the Flatpak
    (`--talk-name=org.a11y.Bus`, from reading `accesskit_unix`). Its
    cost with an assistive program connected, `FRAME_ACCESSKIT=1` in
    `frame_perf` on the campaign's 117 views: 0.86 ms a frame on
    average without the tree, 0.97 with it, the most 0.55 ms more (a
    long script, 0.73 to 1.28); none over the budget. Without one,
    nothing. (S, waits on a screen reader's user)

## Automation

- **Plugins, beyond API 0.4** (`plugin-proposal.md`; commands and checks
  in sandboxed Luau are built; 0.2 added a file or folder the user
  chooses, haks written with their leave, areas and terrain, pictures;
  0.3 files written into a chosen folder, 2DAs and talk tables as
  values, and events, whose own gaps are below, under "Plugins writing
  into a folder"; 0.4 what is selected and
  open, commands in right-click menus, fixes for findings, a plugin's
  own tests, settings, a project's own plugins, a resource's own events
  and the after-create hook):
  - **A builder's terrain tool (NWN Mapper) as the measure:** it sculpts
    a free height field, paints texture layers and a walkmesh, bakes
    placeables and a lightmap in, and writes a tileset's group (a model
    and walkmesh per tile, textures, materials, a patched `.set`). A
    plugin can now take such an export into a hak and make the area
    (`examples/tileset-import`, tried on the builder's earlier export:
    149 files, an area of one of its groups drawn from the hak). What
    the tool itself does is out of a plugin's reach: there is no view
    to draw in or brush to take input from, and baking textures in
    Luau would be slow. Sculpted terrain inside Moonglow would be a
    feature of its own. (L)
  - **Not tried in the application on a desktop:** the folder and the
    hak question were driven through the UI harness and `mg`, not with
    the real file dialogs (the file dialog of `open_file` filters by
    the extensions asked for; that filter was not seen on a desktop
    either). (S)
  - **A hak is written before the module's edits go in,** and stays if
    they then fail; Undo leaves it too. A job's haks are held in memory
    until it ends, except files copied from a chosen folder. (S)
  - **Painting is a tile at a time:** each call reads the area's tiles
    again, so a plugin that sets every corner of a 32 × 32 area makes
    thousands of small edits (one undo step, but a large one). (S–M)
  - **No list to install or update from:** plugins come as files; the
    proposal has an index with API 1.0. Remove is for what Install from
    File installed; a folder copied in by hand is deleted by hand. (M)
  - **Archives of several plugins** are refused: one plugin to an
    archive. (S)
  - **A project's own plugins** (API 0.4, the Domains of Dread's
    builder proposal A2: `plugins/` of a project, or `[plugins] folder`,
    run once the builder allows each, asked when the project is opened,
    the answer kept in Moonglow's own folder (`project-plugins.toml`, by
    the project's canonical path, the plugin's id and a SHA-256 digest of
    its files, which it runs from), never read from the project; `mg
    verify` as allowed or with `--allow-project-plugins`; `mg plugin list
    --project`, `mg plugin allow`). Not done: a project moved or copied
    elsewhere on disk is asked about again (answers are by its path);
    the question comes when the project is opened (and at Reload), not
    when its plugins' folder changes while it is open (the folder is not
    watched; until then the plugins run as they were read, which is what
    was allowed); a digest says a plugin is the one allowed, not who
    wrote it (no signatures); a no holds for the plugin's id whatever it
    becomes, and there is no Allow All in the window; `mg plugin run` runs a
    plugin's folder it is given whether allowed or not (it is named on
    purpose); a project's plugin and one installed of its id share their
    settings of scope `user`; what a project's plugin's events were
    allowed (a folder, a yes) is kept by its digest, so a changed one
    asks again; a module kept in one file has no project, so none. The
    question and the Plugins window were driven through the UI harness
    and looked at as rendered pictures (one plugin asked about), not
    used on a desktop. (S each)
  - **A plugin's settings** (API 0.4, A5: `[setting]`, `ctx.settings`,
    `ctx.settings:set`, the settings' form, scopes `user`, `project`
    and `user-project`). Not done: five kinds only (no number with a
    fraction, list, color or pattern a text is held to); a `path` is
    text, the plugin reads no file by it (it still asks with
    `ctx.ui:open_file`), and the form's Choose… picks a file, not a
    folder (typed); what the form saves and what a job sets is not part
    of Undo (files outside the module), and a team value saved writes
    the project's `moonglow.toml`, committed with it, without asking;
    what a job set is kept when the job did not fail, even if its edits
    then do not apply; `mg` keeps no settings of scope `user` and reads
    none of Moonglow's (`mg plugin run --setting` for one run; `mg
    verify` takes the project's files and the defaults); a value kept of
    the wrong kind, or of a setting no plugin declares, is said when the
    project is opened and left there, and nothing tidies it. Driven
    through the UI harness, with a rendered picture of the form; not
    used on a desktop. (S each)
  - **More kinds of things to add:** exporters and importers, wizards,
    panels. (L)
  - **Commands on what is selected** (API 0.4, the Domains of Dread's
    builder proposal A1: `ctx.selection`, `ctx.open`, a command's
    `for`). Not done: `for` names a kind for its placed objects and its
    blueprints alike (a command for one only checks `blueprint` itself);
    a selection is objects, blueprints or one area, never tiles, a
    trigger's points, the start location, several areas or several of
    the module tree's rows; the commands are in the area view's, the
    module tree's and the palettes' menus, not in Find References,
    Advanced Search, the Properties editors or the other editors' own;
    an area's commands are in the area view's menu only when nothing is
    selected there; from the Plugins menu or a key the selection is the
    area view shown last, else the palette's, not that of the pane with
    the focus; `ctx.open` is nil for several objects or blueprints edited
    together; a palette's blueprint menu has no Properties to end it, so
    the plugins' commands come last there; a palette's categories and
    the tree's groups have none. Driven through the UI harness, with one
    rendered picture of the area view's menu; not used on a desktop.
    (S each)
  - **Fixes for findings** (API 0.4, the Domains of Dread's builder
    proposal A3: `ctx.fix`, Verify's button beside a finding, `mg verify
    --fix CHECK`, a finding's `fix` in `mg verify --json`). Not done: a
    fix asks nothing (no form: "retag to what?" is a command's); it is
    worked out when Verify runs and refused once a resource it changes is
    not as Verify read it (compared as bytes) or was changed by another
    fix that did more than set fields and take items out, so after
    Undo or a second fix
    there, Verify runs again; Undo of a fix leaves its button saying
    Fixed; the buttons are in Verify Module's log alone (not Build
    Module's results, the 2DA editor's faults or the tileset editor's),
    for the last Verify, with no "fix all of this check" in the window
    (`mg verify --fix CHECK` is that); the doctor has fixes for
    `tag-unique` (the others retagged `TAG_2`…, scripts that look for
    the tag not followed), `area-variables` (moved to the GIT),
    `var-runtime` (removed) and `static-transform` (the transform off)
    only:
    `pc-faction` has none (the builder chooses the faction), nor do the
    references, models, 2DAs and tilesets; the fixes of different
    plugins' findings are made one at a time in the window; `mg verify
    --fix` with a check's id that no finding with a fix has (mistyped,
    or a plugin's not given) only warns, since a plugin's ids are known
    only once its checks ran, so a pipeline reads "nothing to fix" (fail
    instead?). Driven
    through the UI harness, with one rendered picture of the log's
    buttons; not used on a desktop. (S each)
  - **A resource's own events and the after-create hook** (API 0.4, the
    Domains of Dread's builder proposal A4 and A6: `resource-saving`,
    which warns of or refuses what a save would write, with fixes, in
    the Before saving window; `resource-saved`; `object-placed`,
    `object-pasted` and `resource-created`, whose edits join the user's
    own command; a 5-second budget; `mg set` and `mg apply --plugins`;
    `mg plugin run --event --was/--select/--by`; a test's `was`, `by`,
    `warned`, `refused`). Not done: Save As (to another place) asks no
    plugin, nor do the 2DA, talk table and hak editors' own saves, a
    script's own Save, Build Module or `mg watch`; the custom palettes
    Moonglow rebuilds at a save are not among what a handler reads;
    what fires `resource-created` is the Area Wizard, the creature and
    blueprint wizards, Add to Palette, Setup Store and Copy (Copy As,
    Edit Copy), not the Script Wizard, New Script, a NUI window, an
    import or what a plugin makes, and `object-placed` is a palette's
    blueprint, the Placeable Gallery, a light, Create Waypoint and Setup
    Store's store, not a spawn point, the start location or an
    encounter's points; a handler of a placing, a paste or a making can
    only edit and log (no warning beside what was made, no refusing a
    paste); a fix in the Before saving window is made one at a time, the
    save asked again after each; "as it was" is as each file was read
    (a module folder, a project) or, for a module kept in one file, as
    it was at the last save or opening, in this run; `ctx.event.changed`
    says what differs in a GFF only (100 lines at most), not in a
    script's text; the budget is fixed (5 seconds), not a setting, and a
    handler is stopped between steps of its code, not inside a long call
    of the API (a large `ctx.module:gff`); `mg` runs a save's events for
    `mg set` and `mg apply` alone (not `mg areas`, `mg rename`, `mg
    replace`, `mg update-instances`, `mg import`, `mg verify --fix` or a
    plugin command's own save), and a warning there never fails it (no
    `--strict`). Driven through the UI harness (a save refused, warned
    of, its fix made, Save Anyway and Cancel; a paste and a copy edited,
    one Undo), with rendered pictures of the window warning and
    refusing; not used on a desktop. (S each)
  - **A plugin's own tests** (API 0.4, A8: `tests/*.toml`, `mg plugin
    test`). Not done: what a plugin writes outside the module (haks, a
    chosen folder) is not written and not held to anything; a command's
    edits are held by the fields they leave, their count and resources,
    not as an edit file to compare; a finding's message is matched
    whole; `mg plugin test` runs one plugin, not a folder of them, and
    the window does not run them; `mg plugin check` does not read the
    test files; the plugin gets the game's data only where `mg` finds a
    game, so a pipeline without one cannot test what reads `ctx.game`.
    (S each)
  - **The reference is written by hand.** A test holds it to list every
    name of the API, not that each description is right. (S)
  - **The type file was not tried in an editor:** it loads as Luau and a
    test holds it to have every name, but it was not checked with the
    Luau language server. (S)
  - **External programs** as plugins (a protocol over standard input and
    output, for Python or Nim): not started. (M)
  - **The window with plugins was run on Linux only:** the application
    itself was driven on an off-screen display there (plugins installed
    from archives listed and enabled, a command as a background job, a
    form answered, Undo, a check in Verify Module, the enabled plugins
    kept over a restart, `--no-plugins`), and the rest through the UI
    harness. On Windows and macOS the tests pass in CI, but nobody has
    used plugins in the window there; and the file dialog of Install
    from File was not driven anywhere (the harness answers for it). (S)

## NWSync publishing

- **The feminine talk table** isn't published, as `nwn_nwsync_write`
  doesn't. Read in single player (2026-10-29,
  `reading_feminine_talk_table` in `client_render.rs`'s `batch2.rs`: a
  module's `mg_rtlk` and `mg_rtlkf` with other text, in the module and
  in the user directory's `tlk`, the player a woman): the client draws a
  line by StrRef (`FloatingTextStrRefOnCreature`) from the masculine
  table for her, where the server's `GetStringByStrRef` with
  `GENDER_FEMALE` reads the feminine one; so the client's own lookups
  don't use it, and only scripts' would. Whether a client downloading
  by NWSync is the same isn't read: the download is (2026-10-29,
  `joined_nwsync_download`), but the client then ends itself ("We
  disconnected you from the server while NWSync is running", exit 0,
  its own restart not seen in the sandbox), and started again it finds
  the server, shows the character list and ends again; a player in
  the game after a download waits on that restart in
  `tools/nwclient/run-server-and-client.sh`. (S)
- **Compression:** Moonglow's encoder offers only zstd's fastest level,
  so the data is about a quarter bigger than
  `nwn_nwsync_write`'s (9.8 MB against 7.7 MB in `nwsync.rs`): more to
  upload and download. A stronger encoder would be a new dependency. (S)

## Tileset authoring

- **A tileset with holes** (a builder's report: a big old tileset,
  merged by hand, drew no tiles at all) is read tile by tile, as the
  game reads it: a tile without its section or a model draws nothing,
  the area's problems and Verify (`set-hole`) name the holes, a bare-LF
  file is an error (`set-line-ends`: the server never loads the
  module), and a model that is there and can't be read is said to be,
  with why, not "not found". Not done:
  - The builder's own tileset (trf01, 2813 tiles) has no holes and CRLF
    line ends, and Moonglow draws its tiles whose models the game has:
    why it draws none of theirs is not known without their hak (its
    models, or the module and the haks it lists). The log now says,
    for each model left out, whether it was found and why it can't be
    read. (S, once the hak is in hand)
  - A file with lines ending both ways was not run in the engine; only
    one whose every line ends in LF is called an error. (S)
  - The tileset editor opens such a file now (it opened none before);
    how its pages show a hole was not looked at. Painting never chooses
    one. (S)

## The install, starting and crashes

- **The recovery copy written at a panic** holds the work as the frame
  that panicked left it (a change applied in it is in the copy); one in
  a thread of its own has the next frame write it, and one in eframe's
  own code has the application write it as the unwinding drops it
  (2026-10-29, `recovery.rs`'s tests). Not tried in a real crash: on
  macOS a panic unwinding into the system's code aborts (after a
  frame's copy is written; one in eframe's code there may abort before
  the application is dropped), and eframe's unwinding through the event
  loop was read in its code (`run_app_on_demand` on the stack), not
  crashed on Linux, Windows or macOS. (S) (waits on a crash on Windows
  and macOS) (Sacha's report, 2026-10-25)

- **Steam Workshop content** is read (Options › General): each item's
  `override` folder as a layer under the user's own, its `hak` and `tlk`
  folders searched. Which of two items' overrides wins in the game isn't
  known (the lower number here), and that the game puts them under the
  user's override is from how it is described, not measured: the client
  runs here with Steam hidden. An item's other folders (portraits,
  modules, music) aren't read. (S) (from builders' reports after 1.14.1)
- **Finding the game:** Steam's libraries are read from its
  `libraryfolders.vdf` (a game on a second drive), GOG's usual folders
  tried, Steam's own folder read from Windows' registry (`reg query`
  of `SteamPath` and `InstallPath`: held by Wine's `reg` in
  `steam_folder_from_this_machine_s_registry`), and Beamdog's client's
  libraries from its `settings.json` (as neverwinter.nim's
  `findNwnRoot` reads them; else `~/Beamdog Library`), 2026-10-29. Not
  tried where they live: a real Windows with Steam elsewhere (a folder
  of other than ASCII letters, which `reg` prints in the console's
  codepage), Beamdog's client on any system, and the reporter's case (a
  second drive on Linux), covered by the list. (S) (waits on a Windows
  machine and a Beamdog install) (from builders' reports after 1.14.1)
- **Installing updates from the app** (the rest of GitHub issue 28):
  Help › Check for Updates… and the daily check only say that a newer
  release is out and link its page. Not done, for the owner to decide:
  downloading the package for this system and installing it (the
  AppImage replaced in place or through AppImageUpdate and zsync; the
  Windows installer run; the macOS app swapped), restarting after
  asking about unsaved work or installing at the next start, and the
  issue's Options (Install updates automatically, off; Include
  pre-releases, off). Before any of that, verifying what is downloaded:
  the release would publish SHA-256 checksums and a signature checked
  against a public key built into the app (minisign or ed25519), so that
  a tampered download or a compromised GitHub account can't push code
  to every user; the packages aren't signed yet. Building blocks the
  issue names, unevaluated: Velopack (a Rust SDK, Windows, macOS and
  Linux, delta updates), cargo-dist with axoupdater, the `self_update`
  crate over GitHub Releases. Also not done: the release notes shown in
  the window (only the page is linked); the issue's "Install Now, Later"
  buttons (the notice has the page, Don't remind me about this version
  and ×). (L) (GitHub issue 28)
- **The graphics backend** (Options › General, `mg_render::backend`;
  El Grillo's report, October 2026: an AMD Radeon PRO's tiles gone one
  by one through AMD's Windows Vulkan driver, 25.10.37.03, drawn right
  with `WGPU_BACKEND=dx12`; Heed's, since 2.11.0: a Radeon RX 580 on
  Windows 10 drawing nothing of an area through Direct3D 12, right
  through Vulkan). Automatic keeps Direct3D 12 first for AMD on Windows;
  the startup check (`mg_render::probe`) takes the other backend where
  it draws nothing, the lit check (`probe::probe_lit`, once the game's
  data is read) checks a lit, textured mesh by Moonglow's own shader and
  by the game's, and the area view's watch (`mg_render::watch`) offers
  Restart with … where drawing collapses later. Neither case was
  reproduced: each is imitated (`guard::blank_draws`, `fail_on`,
  `fail_program`, `stall_textures`). Left over:
  - Heed's machine: 2.13.0 drew nothing there through Direct3D 12 on
    Automatic and neither check said so (2026-10-28). What could get
    past them was closed after 2.13.0 (frames that never get ready are
    judged after 20 s with nothing finished; tiles whose models failed
    are expected; a look the GPU doesn't give back is given up; the
    pipelines' internal errors are caught, FXC's failures on
    `fslit_nm` and `fslit_sm_nm` among them; the lit check; the watch
    writes what it sees to the debug log), and Direct3D 12 compiles with
    the DXC shipped beside the programs since (`mg_render::dx12`;
    FXC, where `dxcompiler.dll` is missing, can't compile those two nor
    Heed's `fhss_skintile`, a copy of `fslit_nm`). The startup check passed there and draws unlit meshes alone,
    so Moonglow's own unlit pipelines draw on that driver; why the rest
    doesn't is not known. Waiting on Heed, with the next release: the session's
    debug log (`%APPDATA%\Moonglow\logs`: its `graphics` and
    `graphics watch` lines, and any `[WARN` or `[ERROR` ones; the
    `graphics:` line now ends with the compiler taken, which should be
    DXC), any
    `crash-*.txt` or `model-failure-*.txt` in `%APPDATA%\Moonglow` from
    2.13.0 and the next (2.13.0 wrote one where FXC failed a program, as a
    crash though it went on), whether a notice appeared, whether the
    area is drawn with the Shaders button off on Direct3D 12, and whether
    the skin tile is drawn by the game's shaders now (DXC compiling for a
    real driver is checked on CI's software one alone, WARP). (S, needs
    the builder)
  - El Grillo's machine: 2.13.0 renders well on his Radeon PRO
    (2026-10-28), on Automatic's Direct3D 12: that confirms Direct3D 12
    there, not the watch on his Vulkan case. Waiting on him: Vulkan
    chosen in Options, whether the watch's notice appears as the tiles
    go. The owner decides Automatic's order after both. (S, needs the
    builder)
  - Why AMD's Vulkan driver loses the tiles isn't known: Moonglow's use
    of Vulkan may be at fault (a validation run on that driver would
    say), nor why Heed's Direct3D 12 draws nothing, and nothing was
    reported to AMD. Until then every AMD GPU on Windows takes Direct3D 12
    first (the renderer's tests run there on CI, through Microsoft's
    Basic Render Driver, and Metal on Apple's Paravirtual device: the
    2.13.0 run drew the shared buffers at a base vertex, textures read
    on threads, the game's programs written for the tests and the
    framebuffer's copies). (M, needs the hardware)
  - The window's adapter is chosen by Moonglow (egui-wgpu's adapter
    selector), which leaves wgpu's power preference and surface check
    to it: `pick_adapter` repeats wgpu 30's order of adapter kinds and
    the app checks the surface. A later wgpu that orders otherwise is
    not followed by itself: look at it at the next wgpu bump. (S)
  - **OpenGL draws wrong.** Not offered in Options (the owner's
    decision), but wgpu still falls back to it where neither Vulkan,
    Direct3D 12 nor Metal draws, and `WGPU_BACKEND=gl` takes it:
    `WGPU_BACKEND=gl cargo test -p mg-render --test render` fails 20
    of its 44 tests on the development machine (no visible particles,
    the game's programs, the framebuffer's copies, animations...): the
    renderer was never made for wgpu's GL backend. (M)
  - `mg` and the other headless renderers take Automatic (or
    `WGPU_BACKEND`), not the GUI's setting; Moonglow Viewer, which builds
    on mg-render, doesn't use `pick_adapter` yet. (S each)

## Documentation

What the [audit](docs-audit.md) of 2026-10-12 could not put right with a
few proven words needs a decision or a rewrite and is listed there, per
document; the entries here that name the audit are that list in short.

- **The wiki corrections were never submitted** (`wiki-proposal.md`);
  all fifteen pages are unchanged upstream. (S)
  (the documentation audit of 2026-10-12)

## The code itself (the code review of October 2026)

A review of the whole workspace for dead code, clumsy code and code
written twice found more than it changed. What it changed (some forty
commits, with a test for all but one of the bugs it fixed) is in the
history; this is what it left, found by reading and not by running
unless it says so.

- **Never used:** the second half of `ModelView::frame`
  (`model_view.rs`: the model is there exactly when the composed preview
  is). The rest of what this entry listed went on 2026-10-29; this is
  left while the sprint's packages 07 and 08 change that file (waits on
  their merge). (S)

### The review of 2026-10-27: what is left

Six readers over the workspace and clippy's pedantic lints as a finder
found some forty defects and costs; what was done about each is in the
[log](log.md) (2026-10-27). The editors' costs were measured first
(`frame_perf`, with views added at a persistent world's sizes): those
within the 144 fps budget were left as they are (the 2DA editor filtered
on 6,000 rows 1.7 ms a frame, a conversation 1.4, blueprint editors and
an item's Properties page 0.6 to 1.5, palettes, the manual and the model
viewer under 2). What is left:

- **Typing in a very large script** (`nwscript.nss`, 14,000 lines) is
  13 ms a key: egui lays out the whole text again on each change. An
  editor that lays out the lines in sight would be needed. (L)
- **The blueprint picker over a large palette** (inventory, store and
  encounter pages) folds every blueprint once a level of its tree and
  draws every match: searched for what all of the persistent world's
  8,000 custom items have, the store's Inventory page takes 8.0 ms a
  frame (`frame_perf`'s "store inventory, picker searched", 2026-10-29;
  the budget 6.94). The matches kept by palette and search, and only
  the rows in view drawn, would cut it. (M) Dragging objects copies the
  whole area model each frame: not timed, `frame_perf` has no drag of a
  selection (S to add). (The tiles palette searched as it is typed is
  timed since: 5.0 ms at worst over area092's 784 tiles, in budget.)

## Written more than once (the review of 2026-10-21)

Five reviewers read the workspace for logic written more than once, a
builder having said that AI-written code "reimplements a lot of stuff in
minorly different ways instead of building one source of truth". Where
copies had drifted into a wrong result, the fix went through one place
with a test (the log, 2026-10-21). This is what is left: places by file
and line as they were that day, found by reading, not by running unless
it says so. One question settles most of them: which copy is the source
of truth, and does each caller's difference matter.

### To build once (the structure the copies want)

- **GFF paths as strings:** two `Step` enums (`mg-module text.rs:77`,
  `mg-edit lib.rs:36`), hand parsers (`search.rs:255`, `rename.rs:83`,
  `:436`, `verify.rs:38`, `references.rs:489`), hand walkers beside
  `Struct::walk_fields` (`rename.rs:192`, `:209`, `text.rs:207`,
  `doctor.rs:1250`). `GffPath` into mg-gff, carried by `Reference`,
  `Usage` and `Hit`. (L, mechanical)

### Settled in the engine or Aurora first

- **A valid resource name, three rules:** ASCII of 16 (`mg-plugin
  tables.rs:253`), letters, digits and `_` (`talk.rs:237`), those and
  `-` (`hak_edit.rs:114`, `mg-plugin files.rs:247`), lower case alone
  (`mg-nui lib.rs:209`, `talk_view.rs:332`); also `talk.rs:82`,
  `attach.rs:72`, `grass.rs:222`, `doctor.rs:708`,
  `tileset_view.rs:142`. A plugin can write a sound `mg tlk` refuses.
  The game takes any name of up to 16 bytes but NUL, whatever its case
  (found by either, listed in lower case), with `-`, `.`, a space or a
  byte past ASCII (`engine_resource_names.rs`), as `ResRef` does; what
  is left is the one rule for names Moonglow makes, chosen (narrower
  than the game's, for names that are also a project's file names)
  and put in mg-core for the eleven places. (S–M)
- **A copy's resref, what is left:** a blueprint's Copy is named as
  Aurora's Edit Copy names one now (`palette_add::copy_resref_named`,
  captured 2026-10-29: `nw_w1copy007` to `w1copy008`, the one case).
  Aurora's Add to Palette, captured the same day, named copies of
  templates without a number `002` (`w1_held002`, `w1_arrow002`,
  `mgp_utc002`; `captures/wave1/add-palette.mod`, `pal2-addpal.mod`),
  where `new_resref` gives `001`, as the earlier capture's `mgp_utp001`
  (`aurora_palette_add.rs`) has it: what makes the difference is not
  known (S, an Aurora capture or two). `copy_resref`
  (`palette_view.rs`) stays for other resources, with three more
  generators (`store_setup.rs:27`, `blueprints.rs:35`,
  `dialog_view.rs:1612`) and two from a file's stem
  (`dialog_view.rs:1608`, `nui_view/images.rs:91`). (S)
- **The two portrait filters, the Creature Wizard's:** Select Portrait's
  (`situated.rs`: the gender exactly, the module's portraits too, the
  table's order) is Aurora's (captured 2026-10-29: Human and Male lists
  Sex 0 rows alone in row order, the module's own `w1po_` among them;
  Outsider and Male shows nothing there too). The Creature Wizard's
  (`creature_wizard.rs:92`: the gender chosen, then rows of both or
  none, Plot rows left out, not the module's portraits) was not compared
  with Aurora's wizard. (S)

### Smaller, each S

- Left by the one row list (`widgets::row_choice`, 2026-10-21): it lays
  out every row while open, where the 2DA editor's "a row of X.2da" laid
  out only those in sight (a table of tens of thousands of rows, not
  measured); Advanced Search's Value list goes by name past 12 rows,
  where the item editor's cost list keeps the numbers' order (+10 before
  +2 there); the Appearance and loading screen lists have no "≠" mark
  for blueprints edited together, which the other fields' lists have;
  and an item property's parameter list, by name now, was not held to
  Aurora's order (a creature's classes and a door's generic appearance
  are by name in Aurora too; its portraits are in the table's order, as
  Select Portrait's grid: captured 2026-10-29).
- `rename`'s `usages`, `tag_usages` and `mentions` restate what
  `UsageIndex` answers. (`#include`, read three ways, is read one way
  since 2026-10-28: `mg_script::lex::includes`, which knows comments.)

## Claims, protected areas and the project's commands (the builders' proposal, C6 to C8)

The Domains of Dread's builder proposal (its section 3, and the hand-back
proposal's section 8.2 to 8.4): the claims table, claimed areas read-only
or asked about before a save, Claim This Area and Release Claim, merged
areas' Tag and ResRef asked about before they change, `mg claims`, and
File › Project. What is left:

- **The project's commands on Windows** (`cmd /C` with the command's
  text as written, no console window) are tested by CI's runs of the
  UI tests, not on a builder's machine with a real update script. (S,
  needs Windows)

## Release and packaging

- **macOS:** the app and disk image are built by CI but have never been
  run on a Mac. (S, needs a Mac)
- **Windows signing:** the installer isn't code-signed, so SmartScreen
  warns. (S, needs a certificate)

## Questions asked, unanswered

Each waits on its reporter: with no word by 2026-11-12 (a month from
the owner's ruling of 2026-10-12) it is dropped, and said so in the
log.

- **xtul's mirror at full size in the game:** the client draws a
  dynamic placeable with Scale 0.46 at 0.46 (`visual_transforms_look`),
  upright and turned on its stand by X Rotation -50.8; at full size and
  unturned only when it is static. Whether his mirror is static in the
  module the game ran (a build step that makes placeables static, or
  Static ticked after the transform), or the full size was by eye,
  would settle it. (S) (xtul's report, October 2026)

- **The rectangle by two clicks on a builder's Linux:** 1.25.1 made the
  right Alt place it, which was not his trouble; the next release has Alt
  pressed and let go before the click, on the reading that his window
  manager keeps a click with Alt held (the rectangle went when he
  clicked and came back when Alt was pressed again). Not seen on his
  system: his word that it places would settle it. (a builder's report,
  October 2026)
- **Creature Properties' overlapping elements:** reported, not
  reproduced: every page is checked for widgets drawn over each other at
  four window sizes (`blueprint_editor_pages_draw_nothing_over_anything_else`)
  and none are. A screenshot, or the page and the interface size, would
  find it. (S) (a senior builder's review, October 2026)
- **A drag on a number field moving objects in the area behind:**
  reported, not reproduced
  (`a_drag_in_a_window_over_the_area_moves_nothing_there`). (needs the
  steps) (a second builder's review, October 2026)
- **"It kept randomly exiting"** on a persistent world's module (ten
  times before it stayed open), reported without a log. One cause was
  found and fixed: the compiler walks a script by recursion, and a very
  long `else if` chain or expression overflowed the stack of the thread
  it ran on, which ends the program at once and leaves no crash report
  (a 60,000-link chain did it on Linux; Windows' main thread has an
  eighth of that stack). Compiles run on a stack of their own now.
  Whether that was the reporter's crash isn't known: asked for the debug
  log (Options › General) and what was being done. (needs the log)
  (from builders' reports after 1.14.1)
- **A hak's 2DA not seen until a restart** (a builder: a creature
  showed "appearance.2da has no row 8241", from a hak above CEP's whose
  table has more rows; gone after restarting Moonglow): not reproduced.
  Haks and folders that change on disk are read again every few seconds
  and their tables forgotten, which a test holds for the override
  folder. Suspected and not shown: `reload_resources` and `sync_haks`
  give up without a word while anything else holds the game data (a
  job), and try again later; on Windows a hak Moonglow has open cannot
  be replaced by another program at all, so a hak tool may have failed
  to write it. If it comes back: the log around it, and whether Tools ›
  Reload Resources cures it. While
  something else holds the game data, the look is put off and the
  debug log says so ("the game data is held"), which would show this.
  (S–M) (from builders' reports after 1.19.1)
- **"Area transitions can't be selected visually":** not understood;
  asked.

## Settled: as it is, by decision or by design

Nothing is left to do on these; they are kept so that a report of one
is not taken for a new gap.

- **A geometry program beside a material's pair** (`customshaderGS` with
  `customshaderVS` and `FS`): the client runs it (one that emits nothing
  hid its card, `program_choice_in_the_client`, 2026-10-29); wgpu has no
  geometry stage, so Moonglow draws its own shader there and reports it.
  (Named alone, the client ignores it, and so does Moonglow since then.)

- **A creature's Equipment in a narrow window** (about 480 points, as
  a placed creature's Properties open beside the area) shows each slot's
  name over its item's beside a smaller icon, Equip over Remove; the
  longer slot names (Secondary Weapon or Shield, the creature weapons)
  are cut short there, whole in their tips. Kept: Aurora's inventory
  names no slot at all (a paper doll of icons, seen again in the
  capture of October 2026), so a name cut short with its tip says more
  than Aurora does, and the item's name, which matters more, stays
  whole in sight.

- **Seams between tiles:** drawn as instances, a sample on the seam of
  two tiles may show the other tile's floor (12 of 192 pictures, one to
  three pixels each): which of two floors that both cover a sample is
  drawn last was the order of the area's tiles, and is now the order of
  the draws. A mesh at a time (`Renderer::instancing` off) it is as it
  was.
- **Pictures for the rest:** waypoints have their flags as pictures;
  sounds, triggers, encounters and stores say what they are on hover, in
  a line or two (a store's and a sound's markers are the same for every
  one, and the others have no model).
- **Standard palette categories** can't be changed (the game's); only
  custom blueprints move between categories, as in Aurora.
- **Areas by name elsewhere:** areas are by name in the module tree, tab
  titles, Find Instance, the area transition's destinations and the
  references list; the log has ResRefs.
- **The Options tab** opens as wide as the window allows, where the
  other editors leave some of the area showing beside them; Enter is OK
  only while the pointer is over it.
- **Column headings** in forms' tables ("Base", "Total", "Sun", "Moon")
  keep the plain strong style; only section headings were made larger.
- **Scripts and conversations by name:** they have no names; the tree
  lists them by ResRef.
- **A window's bar:** egui_dock gives a folded window the height of its
  tab bar, frame included, so the dock's windows are drawn without a
  frame margin and their tabs' contents are given one instead.
- **Copy… of a script** takes its compiled script along; its debug
  information (`.ndb`) is made when the copy is next compiled.
- **Deleting a prefab** isn't Edit › Undo's: the palette's Undo Delete
  puts back the one deleted last, until Moonglow is closed.
- **Compiling before export** needs the game's data (for nwscript):
  without it scripts go out with the compiled scripts they have, and the
  log says so.
- **Include files** aren't compiled on their own by Compile All, the
  build or Compile in the editor (they were, and one leaning on what its
  includer brings was reported as an error). Watched in Aurora since
  (Build › Build Module, Compile): it compiles every script, says
  nothing of one that only lacks `main`, and reports "Compile Error" for
  an include that leans on what its includer brings, as Moonglow did
  before this change. Skipping includes is Moonglow's own, kinder than
  Aurora and not like it, and kept by decision (2026-10-07); seen so
  again on 2026-10-28 (capture `script-compile`: Compile Error for three
  include files that don't compile alone, nothing for one that does). An include
  is still checked as you type, on its own, so one that leans on its
  includer shows an error there.
- **The grid seen from the area's edge:** red and plain as the view
  comes down (Aurora's is red from every side, and mostly lost in the
  ground from above), and drawn at the area's foot under raised ground.
  Looked at in Aurora on a raised plateau since, at a low pitch: its
  red lines show on top of the raised ground, at that ground's height,
  as Moonglow's do; none could be made out on the low ground in front
  (lost in the ground, as from above). Whether Aurora also draws lines
  at the area's foot under raised tiles could not be told.
- **A .mod beside a module folder on save**, as a builder says Aurora
  writes one for a module directory: on unless Options › General
  switches it off (the user's decision: a folder opened from elsewhere,
  a repository's source tree, gets a `.mod` written beside it unless it
  is off). A nasher project is left to Pack Target. Captured in Aurora
  since (`docs/findings.md`, A module directory beside its .mod): it
  writes the changed files into the folder and packs it into the `.mod`
  beside it, the old one kept as `.BackupMod`, as Moonglow does.
- **Clicking between tiles and objects:** a click on an object while
  tiles are selected goes to objects, and a double click on the ground
  to its tile. So a tile wholly under an object is selected by a box,
  not a click.
- **Behind things:** the rings and arrows are painted over the view, not
  hidden by what stands in front of them (as 3D editors draw theirs:
  left so).
- **A palette's rows are one line** (a builder: names with ResRefs and
  challenge ratings wrapped onto a second line in a narrow pane), cut
  short, whole under the pointer (the module tree scrolls sideways
  instead, as Aurora's does). Not an option.
- **The module tree and the object selected in the area:** the tree
  goes to it only where its area is opened out there; an area left
  closed stays closed. Asked for by the user (it was opened out and
  scrolled to on every selection, written to follow Aurora, whose own
  tree was not captured doing so). With several objects selected, or
  none, it stays where it is; there is no way to turn it off.
  (from builders' reports after 1.19.1)
- **The keypad's − and * in the conversation tree:** both − fold, and *
  typed either way (the keypad's, or Shift+8) opens everything under the
  line. egui 0.36.2 can't tell them apart: egui-winit maps `Minus` and
  `NumpadSubtract` both to `Key::Minus`, has no key for `NumpadMultiply`
  (the keypad's * comes only as the text `*`) and drops a key's location
  on the keyboard (`location: _` in its key event).
- **Names that fold alike, against Windows:** `Arboles`, `arboles` and
  `Árboles` are put in order by their text after the folded name ties
  (`mg_core::text::sort_key`), not as Windows' `CompareString` ties them
  (by accents, then case, at its second and third levels). Aurora's
  order of such names can't be watched here: it runs under Wine, whose
  `CompareString` is Wine's own collation, not Windows'. One order,
  the same whatever the names came in, is what the lists need. (Sacha's
  report, October 2026)
- **Escape in the conversation tree** hands its keys back, as the module
  tree's Escape does; Aurora's Escape does nothing in its tree (watched,
  2026-10-26). Moonglow's lists keep the keys from the area's view, which
  Aurora's tree has no counterpart of: this is the way to give them back
  without pointing at the area.
- **Backspace on macOS** deletes a conversation's line (Delete Line has
  the Mac's delete key, which is Backspace, besides Delete), where it goes
  to the line above on the other systems: a Mac laptop has no Delete key.
- **Bookmarks and folded lines across a change Moonglow didn't make** (a
  plugin's, the file changed outside it) follow their lines as they are
  found again: by each line's text and speaker and those of the lines
  owning it up to Root. Of lines alike in all that, the same number of them
  is taken in order, else only the one keeping its number; a line whose
  text the same change edited (and the lines under it) keeps its number
  only where its kind has as many lines as before. One not found goes, and the debug log says so. The
  editor remembers its own last 256 deletes to follow them exactly; a
  change further back is followed by finding the lines again, so the
  limit stays. Of two deletes that leave the same conversation (either of
  two lines alike), undo and redo follow the one done last.
- **Page keys in a NUI window's preview:** every text field of several
  lines has Page Down and Page Up (`widgets::page_keys`) but those of a
  NUI window tried in the preview (Interact), drawn as the game's: the
  game's own NUI text field of several lines takes neither (its caret
  stays, its text doesn't scroll; the arrow keys move it), tried in the
  client (`docs/findings.md`, "NUI text fields have no page keys").
  (from Sacha's report, 2026-10-21)
- **A script with both kinds of line end, changed a lot:** every line
  not changed keeps its own line end, by a diff of the lines; a run of
  changed lines too big for one table of the diff (2,000 lines each
  side) is cut at the lines each side has once, and the lines between
  are diffed the same way. Lines in a part with no line found once on
  each side and too big for the table, or past the diff's bound of work
  (16 million cells, so it takes a bounded time), are taken as changed
  in place, keeping the ends where they were. Two lines swapped count as
  one kept and one added (the added one takes the text's line end).
  (Jasperre's report, 2026-10-21)
- **Line ends of a line typed into a text of one line** are Aurora's by
  the kind of field (`docs/findings.md`, "Line ends"): plain in a
  localized string (descriptions, names, a conversation line's text,
  journal entries), CRLF in a `CExoString` (comments) and in scripts; a
  text that has line ends gives a line added its own (most of its
  lines'), and a one-line field (a Tag) keeps those its text has. Aurora
  also makes every line end of a blueprint's comment CRLF when it saves
  the blueprint, typed in or not; Moonglow keeps the line ends of what
  is not typed in, so that an untouched text comes back byte for byte.
  A hak's description (the archive's, not a GFF field; Aurora writes no
  hak) gets CRLF. (Jasperre's report, 2026-10-21)
- **Verify's `tag-unique` covers the project's patterns only:** no check
  of every duplicate tag (scenery shares tags by design). (S) (Settled in the triage of 2026-10-28.)
- **Verify's `var-name` with `undescribed` looks at the names only:** a
  variable described nowhere and not named as the project's is not
  said; variables set by scripts are not seen (Verify reads the module's
  resources). (S) (Settled in the triage of 2026-10-28.)
- **Overlapping tufts blend in another order:** a tile's tufts are drawn
  in no order of place (so that the first of them are spread over the
  tile, for the levels), not face by face: the same tufts in the same
  places, but where two overlap their fringes blend the other way round
  (1.3 % of a close view's pixels differ by more than 12 of 255). (S) (Settled in the triage of 2026-10-28.)
- **The camera's height** is each view's and lasts until its tab closes.
  (Lighting and Sound Ranges are kept: a view opens as they were last
  left, and one already open keeps its own.) (S) (Settled in the triage of 2026-10-28.)
- **The start location's marker** is still drawn over the view, not
  hidden by what is in front of it, as the selection's boxes and the
  outlines of triggers and encounters are by design. (S) (Settled in the triage of 2026-10-28.)
- **Plugins writing into a folder, and acting on events** (plugin API
  0.3): `folder:write` into a folder the user chose, with their leave,
  and `mg.on` for a module opened or saved. Not done: a file goes into
  the chosen folder itself, not into a folder inside it (a project's
  `src/` tree of folders is chosen one folder at a time); no copy of a
  replaced file is kept; a script's own Save is no save of the module
  and fires none (a resource's own events, around a module's save, came
  with API 0.4: above); what an event was allowed is kept between
  runs by plugin and version (the settings' `plugin_grants`; a
  plugin's own settings came with API 0.4, above); `mg plugin run
  --event` runs an event by hand, and `mg watch` and `mg build` fire
  none; an event's handler that asks a form asks it at every save. (S
  each) (Settled in the triage of 2026-10-28.)
- **The Spells page against Aurora** (watched in Aurora: see
  [findings](findings.md), "Aurora's Spells page"; Moonglow has its
  Meta-Magic list, casting levels and counts by metamagic now). Left
  different on purpose: the levels show how many the class may have
  ("Level 1: 2 of 3") where Aurora's summary has the bare level, the
  spells known or prepared are beside the list rather than a count on
  every row, and a spell whose casting level would pass 9 can't be
  prepared (Aurora lists it at 10). Not seen: what Aurora writes for a
  spell prepared with two metamagic feats, which its list can't choose.
  (S) (from builders' reports after 1.16.2) (Settled in the triage of 2026-10-28.)
- **Claims are not checked against who commits:** the hand-back's CI
  check (a pull request touching areas its author didn't claim) is the
  team's script's; Moonglow knows the builder by the code they set, not
  by their git identity. A code is not reserved to one builder: one that
  already holds claims is only warned of when set (Builder Code, `mg
  builder-code`). (S) (Settled in the triage of 2026-10-28.)
- **The claims table is a 2DA only** (the hand-back proposal's own
  choice): a TOML or CSV table isn't read; a codepage other than UTF-8
  isn't considered (the table is ASCII in practice). (S) (Settled in the triage of 2026-10-28.)
- **Verify's merged-area checks read the table only:** an area merged
  with no Tag recorded is held to its ResRef alone; the hand-back's
  check against the merge base (a Tag changed since `main`) is the
  team's CI's, as git is not Moonglow's. (S) (Settled in the triage of 2026-10-28.)
- **The conversation tree's letters on Windows:** Aurora was watched
  under Wine, whose tree control is Wine's own (findings, "The
  conversation tree's letters and Escape"): there the same letter typed
  again quickly stayed on the first row beginning with it, and how long a
  pause starts the letters over is Wine's. Moonglow goes round the rows
  beginning with the letter and starts over after a second, as asked;
  Aurora on Windows wasn't watched doing either. (S) (Settled in the triage of 2026-10-28.)
- **Inserting or removing lines in the middle** isn't offered: it
  renumbers every later line, and their users would have to be
  renumbered with it (Find References finds them now: the entry above;
  the renumbering is not written). (M) (Settled in the triage of 2026-10-28.)
- **`mg 2da-diff` and `mg tlk-diff`** compare and merge by row number
  and column name (line number for a talk table). Not done: an edit
  file takes the table whole (`set_resource`), since edit files have no
  change for one cell; a row is its number, so a row inserted in the
  middle reads as every row after it changed, and two rows added under
  one number are a conflict to be settled by hand (no renumbering, which
  would need every reference to the row changed too); a talk table's
  merge takes a line whole (its text, sound and length together); a
  feminine table is compared as a file of its own. (M, M, S, S) (Settled in the triage of 2026-10-28.)
- **The Aurora UI inventory's hand-made parts:** its inputs are made
  from the installed game now (`tools/aurora/uiinv/inputs.py`), but
  twelve StrRef table offsets and the mappings in `mapping.py` were
  settled by eye for one build of the toolset (26c6e573, 89.8193.37):
  another build needs them looked at again (the generator warns). (S,
  when the toolset changes) (the documentation audit of 2026-10-12) (Settled in the triage of 2026-10-28.)
- **A brush's right click is the tile menu,** not Aurora's putting the
  brush down (Escape or the menu's Put Down does); Select Tiles is in
  the Tiles palette too, but isn't turned on by choosing the Tiles
  palette (Aurora's Terrain tab doesn't, and objects are still worked on
  with it shown). Out of Select Tiles, a right click on the ground with
  no object there starts the area's menu with the tile's items, whatever
  objects are selected; Delete there is still the selection's. (S)
  (from a builder's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
- **A click is on the model, not its box:** an object with a model is
  picked where the pointer's ray meets its triangles (at rest: a creature
  mid-stride is picked where it stands still), so a click through the
  empty part of a wide box reaches the trigger or the object behind;
  where the ray meets nothing so, the nearest box passed through is
  taken, so a thin post is still clicked beside. Aurora's own picking
  wasn't measured. Things without a mesh (markers, an emitter alone) are
  picked by their boxes still. (S) (from builders' reports after 1.14.1) (Settled in the triage of 2026-10-28.)
- **A refused tile step out of Select Tiles and with no brush** (the
  area's own right-click menu on the ground) flashes the tiles red and
  says why in the log, not on the status line, which shows the objects'
  line then (a notice there would stay after the brush or the mode that
  set it is gone). (S) (from Jasperre's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
- **An area full of far-reaching lights:** the Txpple pack's light-node
  placeables have multipliers of 6 to 10, so a "15 m" one reaches 300 m
  (as in the game), and every mesh of an area of them is lit by up to 32
  lights, as the client lights it. In The Domains of Dread's woods
  (`game_shaders_frames_at_144`, `FRAME_TDOD_LAMPS`) seen close with the
  game's shaders: 2 lights, 4.7 ms of CPU and 2.3 of GPU a frame; 20 of
  those placeables, 4.3 and 3.0 (within the budget); 100, 7.0 and 8.0
  (over it: each fragment adds 32 lights). A light's reach isn't
  shortened for the frames: what a light past the 32 adds is the
  client's question too. (S–M) (Settled in the triage of 2026-10-28.)
- **An object's light nodes, as it stands:** the light nodes of an
  object's models (its base's and its parts', a held torch's) are taken
  where they are at the start of its idle animation, with the colour,
  radius and multiplier keyed there: a light an animation moves, flickers
  or turns on and off (a lamp's "on" and "off" states keying its light's
  colour) is drawn as it is at that moment. A part's light nodes are taken
  in the part's rest pose. The model viewer's **Lights** shows the base
  model's light nodes (animated) and the appearance's light, not a
  part's. (S) (from Troubletcat's and Heed's reports, October 2026) (Settled in the triage of 2026-10-28.)
- **Moonglow draws the rooms the player can't see:** in an interior the
  client draws only the tiles the player's tile sees (the tileset's
  visibility); the area view draws every tile, as Aurora does. Comparing
  pictures with the client's, the prison's main floor and a `tdm01` mine
  were 13 to 15 of 255 away for this, their lighting alike. (S) (Settled in the triage of 2026-10-28.)
- **Tile appearance and rotation:** Next and Previous Tile Appearance
  step through the tiles that fit by their index in the tileset (Aurora
  has no such step to compare with; its Shift + right click cycles the
  tiles and their turns together, as Refine Tile does); the new tile
  gets fresh lights, as a painted one does, not the old tile's colors.
  Rotate Tile turns a tile only where the same tile still fits, and
  refuses elsewhere: it doesn't turn the terrain under it (that would
  repaint the neighbors), and whether Aurora or the engine has a turn
  for a placed tile was not looked into (the engine draws any
  `Tile_Orientation`; a tile turned against its neighbors' terrain
  would show seams). A tile at any turn, whether it fits or not, is put
  in with Tile Properties' All tiles and its Turn (issue #29); the steps
  themselves stay fit-only. (S) (from a builder's tile feedback,
  October 2026) (Settled in the triage of 2026-10-28.)
- **Tile Properties' lights and loops from the model:** a model not
  found (or not readable) falls back on the tileset's flags, which say
  every light and loop; a tile model's `animloop` animations from a
  supermodel are not looked for (the game's tile models have none).
  Aurora's ✕ discards without asking (seen on the oracle); Moonglow asks,
  as the builder asked. (S) (from Jasperre's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
- **Uniforms a script sets:** a custom shader's uniform that a module's
  scripts set in the game (`SetMaterialShaderUniformInt`,
  `SetMaterialShaderUniformVec4` on an object that isn't static) shows
  the MTR's value in the area view, which runs no scripts. (S) (Heed's
  report, October 2026) (Settled in the triage of 2026-10-28.)
- **Which tiles are a group's is worked out, not recorded:** the area's
  file doesn't say what a group placed, so a tile is a group's when the
  tileset lists it in a group (`TileIndex::is_grouped`), and the group is
  whole where its tiles stand at their places, turned and raised alike
  (`TileIndex::group_at`). What is left of a group cut by an earlier
  edit (or by Aurora's Delete) is still locked tile by tile: only the
  Eraser (which takes such a tile alone) or the option changes it. A
  tile listed in two groups is taken for the first that stands whole
  there. Groups placed with Aurora's Delete applied to them were not
  surveyed. (S) (from Jasperre's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
- **Item costs:** Additional Cost keeps Aurora's limit. The game prices a
  created item itself, as Moonglow computes the stored Cost (see
  [Findings](findings.md#created-items-are-priced-anew)), so nothing seemed
  to need it. (S to check) (Settled in the triage of 2026-10-28.)
- **Animations step 25 times a second** (`ANIMATION_STEP`, 40 ms) while
  only they move; before, they stepped with every frame the window drew
  (20 a second left alone, 144 with the pointer moving). A setting, or
  every frame once the picture is cheap enough. (S) (Settled in the triage of 2026-10-28.)
- **Tokens by language:** a language's own tokens are read from the
  install's `lang/<code>/data/ovr/stringtokens.2da` (Polish has one);
  a hak's or module's `stringtokens.2da` isn't per language. Token…
  puts the token where the caret was last in the text (in place of a
  selection), or at the end if it was never there. (S) (Settled in the triage of 2026-10-28.)
- **Locks kept apart in projects only:** a module folder still writes
  `MG_Locked` into its area files (binary GFF, so no line diff), and a
  `.mod` keeps it as before. A project's locks live on one machine
  (Moonglow's data folder), not in the checkout; an object moved by
  another program or `mg` loses its lock where it can't be found again
  (by list place, blueprint, tag and position). (S) (Settled in the triage of 2026-10-28.)
- **Interact does not run the window** (stated in the manual): handlers
  are NWScript and run only in the game, so event timing, and the
  game's own layout solver, are seen only in Test Module. (L, if at
  all) (Settled in the triage of 2026-10-28.)
- **Nothing short of the game proves a window** (`apps/mg/src/nui.rs`,
  `crates/mg-nui/src/validate.rs`): `mg nui validate` and `generate`,
  like the checks and Build in the editor, check the document and
  compile its scripts; that the game builds the window and draws it as
  previewed is seen only in the client (Test Module). (L, if at all) (Settled in the triage of 2026-10-28.)
- **Resizing leaves bound and template sizes** (stated in the manual;
  `crates/mg-ui/src/nui_view/design.rs`): a resize gesture does not
  replace a dimension that is a bind, and a list template's cell gets
  its width in the cell's settings or the JUI source, not by a handle.
  (S) (Settled in the triage of 2026-10-28.)
- **Portraits, music and ambient sound** in folders (not haks) aren't
  gathered; they go in a hak first, as with `nwn_nwsync_write`. (S) (Settled in the triage of 2026-10-28.)
- **A picture of a model that emits** is drawn four seconds in, framed
  with its particles (a flame is small under its smoke). (S)
  (from builders' reports after 1.10.1) (Settled in the triage of 2026-10-28.)
- **The palette's header, not as drawn in the issue:** "More" is a
  word, and the clear button "×" and the Gallery's "⊞" are the glyphs
  egui's fonts have (the mock-up's were boxes); the icons are the
  toolbar's glyphs, not a drawn set; the tile palette and the prefabs
  keep their own bodies under the same two top rows. Seen in one
  rendered picture, with the dropdown (not with the row of icons). (S)
  (GitHub issue 16) (Settled in the triage of 2026-10-28.)
- **Inventory lists:** a store page or inventory lays out the rows in
  sight and builds their icons only (the persistent world's store, 449
  items on its first page: 30 ms to show where it took 0.4 s, 0.5 ms a
  frame where it took 6.3). A row's height is its icon's, known once
  it has been in sight: until then the rows out of sight take the
  middle height of those drawn, so the scroll bar's size is a guess
  that changes as the rows are seen, and a jump of the scroll bar lands
  among rows by those guesses (the rows in sight are drawn as before).
  The list still goes through every row a frame, to count the room of
  those out of sight. (S) (Settled in the triage of 2026-10-28.)
- **Aurora's keys Moonglow lacks:** Plot Wizard (Ctrl+Alt+P; no plot
  wizard), Refresh (F5: it is the script editor's Bookmark here, and a
  key of the whole window would take it from there; Reload Resources
  can be given a key in Options), and the script editor's Save As
  (Ctrl+Alt+S, now New Script's). (S) (Settled in the triage of 2026-10-28.)
- **Not every key:** text editing, Escape and Enter, Copy, Cut and Paste
  (Ctrl+C, X, V and the classic Ctrl+Insert, Shift+Delete, Shift+Insert),
  Delete in the area view, the script editor's numbered bookmarks, and
  mouse bindings (drags, Ctrl+click) are fixed. (S) (Settled in the triage of 2026-10-28.)
- **A cut-out lit from behind, by Moonglow's own shader:** the game's
  programs at High Quality light a cut-out's kept texels below opaque
  (its edges) through from behind (`inc_lighting`'s translucence: the
  light behind times one minus the alpha), and the client draws the
  light line that makes along a backlit cut edge (the Domains of Dread's
  dire wolf, `dd_n2e_wolfdire`, in the area view; `creatures_look` with
  the wolf, gone with the translucence defined away; held by
  `a_backlit_cut_out_s_edge_is_lit_through_by_the_game_s_programs`).
  Moonglow's own shader makes a cut-out solid before it lights it, so it
  draws no line: it differs from the client there. With The Domains of
  Dread's `inc_lighting` (a wrapped diffuse, N·L never below 0) there is
  no translucence; what shows there is the fur card lit otherwise than
  the normal-mapped body under it, which the client draws too. Also left:
  the game's programs are made for High Quality whatever the user's
  `settings.tml` says (at Performance, the game's default, there is no
  translucence nor light softening). (S)
  (from the Domains of Dread's report on the wolf's edges in the area
  view, 2026-10-22; seen in the client) (Settled in the triage of 2026-10-28.)
- **Compiled on saving the module:** with Automatically Compile Scripts
  on Save, File › Save compiles the scripts whose text it stores (only a
  script's own Save did; a builder found scripts left uncompiled). A
  script saved earlier and never compiled is not caught by it (Compile
  All is); one the external editor saved is compiled when its text is
  next stored here. (S) (from builders' reports after 1.16.1) (Settled in the triage of 2026-10-28.)
- **Find In Currently Open Scripts** searches the scripts whose editors
  hold their text, which includes one closed with text typed and not
  saved. (S) (from builders' reports after 1.16.2) (Settled in the triage of 2026-10-28.)
- **Programs Moonglow starts:** the game (Test Module) and the external
  script editor are waited for on a thread each, so they leave no
  "defunct" process; checked on Linux only. (S) (Settled in the triage of 2026-10-28.)
- **The sample is fixed:** an everyday run sweeps the same sixth of the
  modules and tenth of the resources each time; what only the rest
  holds is seen before a release. A sample that moves with the date
  would cover everything over a week, at the price of failures that
  come and go. (S) (Settled in the triage of 2026-10-28.)
- **Placed objects posed again on each redrawn frame**, static ones too,
  and the animation's node maps made again per pose: 0.3 ms of a frame
  for 416 objects, so left. (S) (Settled in the triage of 2026-10-28.)
- **The Flatpak and the update check:** its sandbox has no network
  (`finish-args` lacks `--share=network`), so Help › Check for Updates…
  in the Flatpak asks nothing and says to update it with Flatpak (the
  owner's choice: no network asked for one request). Builds from source (`cargo run`, a distribution's
  own package) aren't told apart: they check as the AppImage does. (S)
  (GitHub issue 28) (Settled in the triage of 2026-10-28.)
- **Edit on a script that doesn't exist** makes it and says so in the
  log; Aurora shows a message box first ("Resource not found. Creating
  new script."). A mistyped name makes a script (Undo takes it back). (S) (Settled in the triage of 2026-10-28.)
- **The ring over another object:** a click on the ring picks the
  object under it, but a drag from there turns the selection: an object
  under the ring's line can't be dragged from there while the selection
  lasts. (S) (Settled in the triage of 2026-10-28.)
- **Minimap pictures** for a tileset without them are made by the
  tileset editor's Render Minimap Pictures (an area's Export Minimap
  writes the pictures the tileset has): they have Moonglow's renderer's
  look (lit models, as in
  the area view), not the game's painted look; and models must be
  readable (beside the `.set` or in the game data). (S–M) (Settled in the triage of 2026-10-28.)
- **The terrain brush's cancel** leaves out the right click
  (`terrain_mode.rs:543`), where five others take it: a right click
  turns a group there. (S) (Settled in the triage of 2026-10-28.)
- **No one question for every tab with unsaved work** (GitHub issue 15's
  tabs by the keyboard): closing tabs together asks nothing of its own;
  only a hak's and a tileset's editors have work of their own to lose,
  and each asks for itself as it closes. (S) (Settled in the triage of 2026-10-28.)
- **`mg test`'s results outside a test:** a result reported while no
  test is running (a delayed assertion after its test ended, a
  module's own driver) goes under "(outside a test)", since NWScript
  does not say which script is running. (S) (Settled in the triage of 2026-10-28.)
- **`mg serve --restart-on-change`** looks at a file's size and time
  twice a second, and at a folder six levels deep (no file-system
  notifications). (S) (Settled in the triage of 2026-10-28.)
- **`mg serve` and Beamdog's master server:** the server looks the
  master server up at its start even when not listed (`-publicserver
  0`): `mg serve` does not cut its network off, which would take a
  sandbox of its own. (S) (Settled in the triage of 2026-10-28.)
- **An external compiler and what is remembered:** with an external
  compiler chosen in the window, the window and `mg compile` (always
  the built-in compiler) do not share what they remember of the
  scripts they compiled: it holds for one compiler. (S) (Settled in the triage of 2026-10-28.)
- **`nwnsc` and the haks' scripts:** scripts in haks reach
  `nwn_script_comp` through `{haks}` (its `--erfs`); nwnsc has no such
  option. (S) (Settled in the triage of 2026-10-28.)
- **`mg areas`, what it doesn't set:** names, tags and comments, which
  are each area's own (Edit Areas Together doesn't offer them either;
  `mg replace` changes text in names; a name in one language would
  need a language to be given), nor list fields one by one (an ARE's
  tiles and `Expansion_List`: a tile's own lights aren't set;
  variables are `--var`'s, in the GIT where the game reads them). Give
  Every Area This Page's Visual Settings gives an area of another
  scheme the first's tiles' lights picked anew; an area of the same
  scheme keeps its own, and the first area's own picks aren't copied
  tile by tile (the areas' tiles differ). (S) (Settled in the triage of 2026-10-28.)
- **A plugin's talk table and its feminine table** are two files, as the
  game keeps them (`dialogf.tlk`, `NAMEf.tlk`): `mg.tlk` reads and
  writes each as a talk table of its own, and nothing pairs a line with
  its feminine line. (S) (Settled with the rest of "Tables for plugins",
  done in the deferred sprint of 2026-10-28: it is the game's format.)
- **The plugin API's `place_group` ignores Options › Area's freed group
  tiles:** it keeps its documented way (over part of another group, that
  one goes whole), so a plugin's strokes keep groups whole whatever the
  builder's option. (Settled 2026-10-29, when the window's freed strokes
  were checked in the engine.)
- **What stays with Moonglow's own shader with the Shaders switch on:**
  the sky's fade, unlit markers, shiny water without the targets'
  framebuffer copies (thumbnails, previews), textures that only ripple
  (a procedure the engine runs on the CPU) and skins of more than 64
  bones (the game's own `MAX_NUM_BONES`). (M1, 2026-10-21)
- **Keyholing is compiled out** (`KEYHOLING_ENABLED 0`) of the game's
  programs: the editor has no player. (M1)
- **The game's uniforms the editor has no source for** are the game's
  defaults or zero: `vColor`, `vStaticLightDir` (`staticLighting` 0,
  as the client has it at fragment lighting), the weather (clear: the
  game rolls it from the ARE's chances as it runs), wind sources,
  targeting, `scriptable*`; the model viewer and previews have no
  module (noon, two minutes an hour); `cameraViewAngle` is the editor's
  field of view. (M1)
- **A program the renderer can't take falls back** to Moonglow's own
  shader, reported once (the log, `Renderer::game_shader_failures`, the
  log pane): more than one uniform block a stage, storage buffers, 3D or
  array textures, more than 16 textures a stage. (M1)
- **An instance drawn see-through** (a blueprint about to be placed, a
  tile preview) draws with Moonglow's own shader even when its material
  names a pair: the ghost is the editor's, which the game has none of.
  (M2)
- **`texture8`-`texture15` of a material are white** to its programs
  (Moonglow keeps eight slots): no shader of TDoD's reads past
  `texUnit6`, and none of its 18,146 materials, nor the game's 7, names
  one. (M2, checked in M4)
- **The two-part see-through split cuts a custom shader's alpha at
  0.95**, as it does any mesh's: every fragment program of The Domains
  of Dread writes the stock opacity (`fs_plt_hair` its texture's alpha),
  none an alpha meant otherwise (read 2026-10-29). (M2)
- **The model viewer's area for the shaders is outdoors without wind**
  (`areaFlags` 0): it has no area, so an interior-only effect (TDoD's
  roofs) shows as outdoors there. (M2)
- **Moonglow's own shader draws as the client does** since M4 (both
  paths): a texture's `bumpyshinytexture` reflects nothing (water keeps
  it for Moonglow's own water); maps named without a render hint are not
  read; a DXT1's transparent texels are black, opaque; a 32-bit TGA all
  opaque is solid; every lit mesh reflects the area's map where it has
  none of its own; `default` in appearance.2da's `ENVMAP` is the area's
  map (`chrome1` without an area), in placeables.2da's `Reflection`
  `chrome1`. (M4, each read from the client)
- **A pair drawn both with and without `NO_DISCARD`** is compiled twice
  (TDoD: 26 programs, 22 before): the cost of the engine's choice made
  per mesh. (M4)
- **The shadow map stands in for the game's stencil volumes**
  ([notes](research/notes_shaders.md) §G.2): a volume torn by a
  non-manifold caster isn't drawn as the game draws it, and the map's
  edges are a texel soft. (2026-10-28)
- **Lighting off against Aurora:** Aurora's Use Area Lighting off draws
  the textures unshaded at full brightness (no side darker than another;
  shadows still drawn; captured 2026-10-29, `target/test-output` shots
  of the wave 1 probe); Moonglow's working light is white and even with
  a little from the sun's side, so that shapes read: close enough, kept.
- **Static and the transform elsewhere:** ticking Static takes a placed
  placeable's visual transform off in its Properties and Make Static, as
  Aurora does; `mg set`, `mg apply`, a plugin and Update Instances set
  fields as given and keep an object's transform, and Verify warns of a
  static object with one (`--fix static-transform`). Aurora drops a
  visual transform from every blueprint its Edit and OK writes, static
  or not, and from a static placed placeable at every area save
  (captured 2026-10-29, `captures/wave1/static-bp.mod`,
  `pal2-vtdyn.mod`, `area-saved.mod`): Moonglow keeps what files have
  (lossless editing).
- **Delete takes a whole group by default,** where Aurora's Delete takes
  the tile alone and leaves the rest standing (the option gives
  Aurora's). Pasted tiles over a group are refused, as Aurora refuses
  them (captured 2026-10-29: "Pasting tiles to current area failed.
  There was a feature or a group within the region…").
- **The areas' sweeps walked five times:** the corpus tests `areas`,
  `hlsl`, `minimaps`, `object_walkmeshes` and `tiles` each walk every
  module's areas (`area_common::each_module`), sharing the module's
  set-up, not the walk, as the renders hold the GPU and the others read
  each area differently; no walk of the five together. (Settled by the
  owner, 2026-10-29.)
- **The Command Palette's Undo in a script** is the module's (its field
  has the keys), where Edit › Undo, the toolbar's Undo and Ctrl+Z are the
  script's while its editor has the keyboard. (a builder's report on
  1.25.1) (Settled by the owner, 2026-10-29.)
- **A module's files changed outside, read on a thread of their own**
  (`mg_module::outside`), leave the files' memory in that thread's
  allocator arena: the process keeps about 200 MB more after a 970-file
  pull on Linux (25 MB with `MALLOC_ARENA_MAX=1`; it is reused, not
  leaked). (Settled by the owner, 2026-10-29.)
- **Advanced Search's Appearance** covers the 2DA rows a field names
  (creatures, placeables, doors, waypoints, items' base types); items'
  model parts and colors, portraits and sound sets are found through
  Any field. (Settled by the owner, 2026-10-29.)
- **A module's categories in its own order:** the palette and the
  `palcus.itp` built on save keep the order a module's own skeleton has
  (a builder's arrangement, kept until Aurora saves the module). Aurora
  shows every level by name whatever the files' order and writes the
  `palcus.itp` back in name order on Save As (captured 2026-10-29,
  `captures/wave1/pal2-saveas.mod`); Moonglow keeps its own by the
  owner's decision. (Settled by the owner, 2026-10-29.)
- **A retired key's line keeps its text** in the table `mg tlk-alloc`
  writes (a key taken out of the names file): a number saved in a
  character or an item still shows what it said, not an empty line, and
  the line stays the key's for when it is named again. Said in the
  manual's Strings by name. (Settled by the owner, 2026-10-29.)
- **A talk table's JSON is one table's** (`mg tlk-export`, the talk
  table editor's Export JSON), as `nwn_tlk`'s is and a nasher repository
  keeps it: the feminine table is a file of its own there, exported on
  its own (`mg tlk-export TABLEf.tlk`); the CSV carries both. (Settled
  by the owner, 2026-10-29.)
- **Marker models unlit:** stores, sounds and waypoints use the game's
  marker models in their materials' colors, unlit, so that their colors
  tell apart in a dark area. Aurora lights them as any model (captured
  2026-10-29 in the probe made always dark: a waypoint's and a store's
  yellow arrow from 243, 216, 0 by day to 76, 100, 0); Moonglow keeps
  its own by the owner's decision. (Settled by the owner, 2026-10-29.)
- **Alt for the camera and the selection:** Alt + right drag turns the
  selection and Alt + middle drag moves the camera; on a desktop that
  takes Alt + drag, the camera moves by Ctrl + drag and the selection is
  raised by Shift + drag of its blue arrow and turned by its ring or Q
  and E. Aurora turns the selection with Shift + right drag, which turns
  the camera in Moonglow (the handles shown), and there is no key for
  the camera's focus (Alt + middle click): Moonglow keeps its own design
  by the owner's decision. (from builders' reports after 1.10.1)
  (Settled by the owner, 2026-10-29.)
- **Tile Properties' Defaults** sets the lighting scheme's first colors
  and every loop. Aurora's, compared on Interior Evil (2026-10-29,
  `captures/wave1/tile-defaults.mod`), changed nothing shown; Moonglow
  keeps its own by the owner's decision. (from the research notes)
  (Settled by the owner, 2026-10-29.)
- **Exact tile's reach** (issue #29): the Gallery offers a terrain's and
  a crosser's tiles, not a group's pieces (one goes in alone through
  Tile Properties' All tiles); an exact tile takes the replaced tile's
  height; a corner it disagrees on with a neighbor is read as the tile
  to the north or east has it (`Lattice::from_tiles`). The client draws
  such a seam as the area view does (`reading_exact_tile_seams`,
  2026-10-29); the Gallery's reach and the corner rule are Moonglow's
  own design. (Settled by the owner, 2026-10-29.)
- **Tile Height and what stands on a tile** (issue #29): Raise/Lower's
  corner strokes move no object, and the control's ±1024 is Moonglow's
  reading bound, not the engine's; an object over a tile's square goes
  up or down with it, as the server and the client put it
  (`a_tile_s_own_height_stands_where_the_engine_puts_it`,
  `reading_tile_heights`, 2026-10-29). (Settled by the owner,
  2026-10-29.)
- **Escape during a drag** drops it (the drag's other tools, brushes and
  pastes, drop on Escape too). Aurora's ignores it: Escape pressed before
  the button is let go changes nothing, and the object is put down where
  it was dragged (captured 2026-10-29, `captures/wave1/esc-drag.mod`);
  Moonglow keeps its own by the owner's decision. (Settled by the owner,
  2026-10-29.)
- **What a module's tabs don't open again, by design:** a hak never
  saved and a 2DA in one, and a 2DA's Edit a Copy not saved yet (no
  file to open again); a camera moved since the tabs were last written,
  after a crash (they are written when they change, a window is moved,
  the module closes or Moonglow quits, not as the camera moves); a
  placed object moved outside Moonglow with another of its tag and
  blueprint added (neither is taken for it); a kept Options tab making
  its draft as Tools › Options does, which looks for the game's install
  (Steam's library files) as the module opens. (a builder's report,
  October 2026) (Settled by the owner, 2026-10-29.)
- **The round trip** (`mg roundtrip`, Build › Round-Trip Check) compares
  a GFF written anew by its fields, not its bytes: Moonglow lays a GFF
  out its own way, as every edit does. It doesn't ask the game or Aurora
  (the engine and Aurora tests do, on the game's own modules), and the
  formats it doesn't write anew (scripts, models) are held to their
  bytes by the saved copy. (Settled by the owner, 2026-10-29.)
- **Described variables (C2): an object's or a location's value is not
  described** (The Domains of Dread's builder proposal): what a file
  holds of one is an object's id or a place in an area, which no list,
  range or default describes; `values`, `min`, `max` and `default` are
  refused for them, said by line. (Settled by the owner, 2026-10-29.)
- **Reset Factions to Parent Factions in Export** starts unticked.
  Aurora's starts ticked (`xbFactionReset`, the parity inventory and the
  capture of Aurora's Export, 2026-10-29); Moonglow keeps its own by the
  owner's decision. (Settled by the owner, 2026-10-29.)
- **The start location has no Shift axis arrows:** they stay the
  objects'; the marker lies on the ground, where the game puts a player,
  and its drag and Adjust Location place it exactly: left out by the
  owner's decision. (Settled by the owner, 2026-10-29.)
- **Variable sets** hold int, float and string variables only, as the
  Variables window edits: object and location variables mean nothing
  outside their module. A set's Undo Delete lasts while the Variable
  Sets window is open and a rename is put back by renaming it (neither
  is in Edit › Undo, the module's), and `variable-sets` has no folders
  inside it. (Settled by the owner, 2026-10-29: the triage's proposals.)
