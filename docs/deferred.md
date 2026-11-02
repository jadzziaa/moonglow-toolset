---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What finished work left undone or unchecked, filed by area of the program (an entry's last words say which report, issue or review it came from), each awaiting a verdict - fix, add or drop; what is settled by decision or design is gathered at the end.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-02T09:47:15Z }
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

- **`mg merge-gff`, what it leaves out** (the [manual](manual/03-modules.md#merging-in-git)
  has the rules):
  - **Conversations** merge their entry and reply lists by place, and the
    links name nodes by number: two builders adding nodes to one
    conversation conflict, where renumbering one side's new nodes (and
    the links to them: `RepliesList`, `EntriesList` and `StartingList`
    `Index`) would keep both; a node deleted on one side renumbers every
    node after it, so matching by place pairs the wrong nodes too, and
    nodes would first need matching by what they are. (M)
  - **On a conflict nothing is written** and git leaves ours in the file;
    there is no file of the conflicts to edit, and no way to take one
    side for one conflict and the other for the next but `-o` from
    git's stages. A conflict file (the edits each side wants, for `mg
    apply`) or a Moonglow window would. (M)
  (The Domains of Dread's builder proposal, C9)

## Test loop

- **`mg serve` on Windows:** not tried there: the server's console on
  pipes, the links for haks, and Ctrl+C (the server in a console
  process group of its own, `CREATE_NEW_PROCESS_GROUP`, told to `exit`
  by `mg`). (S) (waits on a Windows machine)
- **Test Module and Steam:** the game started for a test is told which
  Steam game it is (`SteamAppId`), so that a Steam copy reaches a running
  Steam and loads the Workshop's content, as when started from Steam;
  started bare it had none of it (a builder's "F9 won't load the stuff
  correctly either"). Not tried: the client is only ever run here with
  Steam hidden. With Steam running, the test then shows in Steam as the
  game being played. (needs a builder's word)
  (from builders' reports after 1.14.1)

## Persistent-world scale

- **Other machines:** budgets were measured on the development machine
  alone: a Ryzen 9 9950X with a Radeon RX 9070 XT and, since the
  [performance audit](performance-audit.md) of 2026-11-01, its CPU's
  integrated Radeon (2 compute units, weaker than most laptops') as the
  stand-in for weaker hardware. Not measured: an Intel integrated GPU,
  Direct3D 12's heaps on Windows, macOS. Not seen either: the game
  client's black of a custom shader's normal map without tangents on
  Windows and macOS (the stock include normalizing a zero tangent; read
  on Linux, `reading_tangents`; Moonglow draws the same black on CI's
  Direct3D 12 and Metal). (S, waits on a builder's machine)

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
  once each. Tyrants of the Moonsea's area084 (676 tiles, 16,000 meshes)
  with the wheel zooming over all of it: 9.8 to 11 ms (`frame_perf`'s
  `sound_ranges_frames_at_144`, named in its `KNOWN_OVER`). Taken apart
  on 30 October (`render_area`'s `MG_TIME_RENDER`, the frame test's
  size, the game's shaders): the scene made in 0.5 ms, its draws encoded
  in 8.0 ms (7.5 since the lights a mesh took are kept in place), the
  GPU's 2.6 ms; 16,651 meshes in 9,680 draws of 1,606 materials, 4,779
  casting the sun's shadows. Of the encoding (its stacks sampled): wgpu's
  `finish` a third (each draw's groups and buffers bound, their uses
  tracked, the driver's calls), the draws' values worked out and written
  a fifth, the lights a mesh takes and the batcher a tenth each, the
  sun's shadows 1.3 ms (their map fitted to the view, so drawn anew as it
  moves). Ruled out: leaving out small meshes (632 of the 12,149 in sight
  are under 2 px across), merging far tiles' meshes (a draw's 32 lights
  are chosen for its mesh: merged, for the whole, and the picture
  changes), and keeping the draws' values while the camera moves (the
  game's programs take `m_mv` and `m_mvp` a draw, and the lights in view
  space). What is left is wgpu's work a draw: fewer draws (one material's
  meshes of many tiles in one, each mesh's lights found by its vertices)
  or render bundles for what doesn't move. ([the frames
  note](research/notes_frames.md), 27 and 30 October) (L)
  (After 3.0.0, the owner's word, 2026-11-01: 10th by complexity.)
- **The performance audit's leftovers** ([the audit](performance-audit.md)
  of 2026-11-01; beyond what it was asked, filed by the owner's word the
  same day):
  - **A setting for MSAA:** 4× is fixed (`model_view::SAMPLES`). Off
    saves about a tenth of an integrated GPU's picture and 107 MB of
    targets a view at 3440×1400 (27 MB at 50% resolution); the renderer
    is made anew on a change. (M)
  - **TGA and PLT textures are uncompressed** (RGBA8: Tyrants of the
    Moonsea's 70 of 96 MB of textures). Compressing them on the CPU as
    they are read would take a quarter to an eighth of that, at a cost
    in reading time and an encoder of its own. (M)
  - **The animation step could follow what a picture costs:** a slow GPU
    stepping less often while the window is in front, measured with the
    GPU's timestamps (not on every backend). (M)
  - **An area's opening to its last texture** is about 50 ms longer since
    3.0.0 (area092: 112 to 165 ms); not taken apart. (S)

## Blueprint editors and inventories

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
  longsword, a dagger and a hand axe, a torch in the secondary, and the
  primary's own dragged there staying in the primary; the Creature
  Wizard's categories without Familiars, as Aurora's: 2026-10-30; new
  strings written to a talk-table range, 2026-11-02: [Text typed into the
  talk table](typed-text-proposal.md)). Left:
  - **Several items at once:** the inventory selects several (Ctrl,
    Shift), but a drag, Copy and Paste carry one: `Dragged` and
    `item_clip` hold one item each place; carrying the selection is a
    drag of many rows, and where each lands (slots, grid places). (M)
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
  design of its own for the marks and the guard. (L) (After 3.0.0, the
  owner's word, 2026-10-30: 6th of 9 by complexity.)
- **Advanced Search, what it doesn't do** (from builders' asks on
  Discord, October 2026; done: changing what it found, ticked or all, in
  one step: a variable set or removed, an event's script, a field;
  Edit Together and Edit Areas Together from its results; conditions
  together, a class by its levels, number ranges and floats; an item
  blueprint followed through bags in bags, and a creature blueprint
  into the encounters that spawn it, since 2026-10-29; a spell by the
  class that knows it, a conversation's lines kept by several
  conditions, and the haks' and the game's blueprints searched
  read-only, since 2026-11-02). Changes are a variable, a script or a
  field: item properties aren't added or removed on the items found,
  nor list entries (feats, classes, inventory) on the creatures; a
  field is set only where the object has it (none is added). Edit
  Together takes blueprints of one type and objects of one kind placed
  in one area; items held by placed objects, and conversations, open
  one by one. (S–M; shares its design with editing lists across several
  blueprints, and is left to be done with that entry, beyond what it is
  alone: the deferred sprint, 2026-11-01)

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
  the editing language, its names seen right and one saved. (S)
  (from builders' reports after 1.19.1)

## Custom content data (2DAs and talk tables)


## Area visibility


- **Setup Area Transition's Connection Type:** the window picks a door
  or waypoint on the target area's map or in its lists, as Aurora's
  `TdlgAreaTransition` does, but has none of its Connection Type (Both
  Ways, Current To Target, Target To Current) nor its Target Type
  Trigger: the target linked back to this door or trigger. Measured in
  Aurora (2026-10-30, `captures/wave2/transitions.mod`, door to door in
  another area): Both Ways writes `LinkedTo` and `LinkedToFlags` 1 on both
  doors; Current To Target on the current one alone; Target To Current
  wrote nothing on either (twice); it saves the current area first, and
  asks before going on when the current object's tag is its
  blueprint's. Moonglow's window writes the current one's, as Current To
  Target; the Connection Type is to add, the target's fields written in
  its area's GIT in the same step. A trigger as the target was not
  captured (the probe's trigger stopped Aurora's dialog: "List index out
  of bounds (-1)"); a trigger painted from the palette would show it.
  (S–M) (what was left of "The minimap in Moonglow", 2026-10-29)
- **An armor lying as plate here and as a tunic in Aurora**, in the
  same builder's picture. The model is chosen by the torso's
  parts_chest.2da `ACBONUS` (`mg_preview::item::item_placed`, measured
  with the game's own armors): a torso the table lacks, or whose
  `ACBONUS` is blank, lies as cloth (`gi_armor01`), so plate here means
  the table Moonglow read gives his torso 6 or more; that Aurora shows a
  tunic says it read another table or another rule. The game's own
  armors of each `ACBONUS`, 0 to 8, lie in Aurora as Moonglow lays them
  now (captured 2026-10-30: 0 and 1 the tunic, 2 and 3 leather, 4 and 5
  chain, 6 to 8 plate; padded armor, of 1, lay as leather before), so no
  stock item shows it. Waits on the builder's item (its torso's number)
  and his haks' parts_chest.2da. (S) (from builders' reports after
  1.19.1)
## Editing in the area view

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
- **Lights placed on the fly, the flicker** ([the proposal](lights-proposal.md)):
  way B is built behind its option (Custom Light…, `mg_module::lights`;
  the server reads the hak, finds the model in it and places the row:
  `engine_custom_light.rs`), and the client answered the proposal's two
  questions (2026-10-29, the third client batch): a `LightColor` light
  and a model's light node light creatures and placeables as they light
  tiles, so the window's "Lights creatures and placeables too" starts
  ticked; a draw takes as many lights as the client's Max Lights (32 by
  default). Not built: a flicker (the node's color or multiplier keyed in
  the model's `default` animation). Read for it (2026-10-30, the fourth
  client batch, `reading_static_placeable_default_animation`): the
  client plays a dynamic placeable's `default` (a light's color keyed
  red, blue, red over 2 s cycled, the floor with it) and not a static
  one's (it stayed red), so a flickering light is a dynamic placeable;
  the area view holds a static placeable still since, and plays a
  dynamic one's light node keys and its meshes' self-illumination keys
  since 2026-10-30 (`point_lights_at`, `idle_keys_meshes`; the fifth
  client batch). Left: the window's flicker itself (Custom Light…
  writing the keyed `default` and a dynamic placeable). (S) (from
  builders' reports after 1.16.2)

## Rendering: creatures, particles, water and shaders

- **The game's shaders, compiled (M0 of [the shaders
  proposal](shaders-proposal.md)):** what the compiler (`mg-shader`) and
  Verify's `shader-compile` check leave out. (from M0, 2026-10-21; Verify
  compiles each program at six settings since 2026-10-29, and keeps what
  compiled, `doctor::settings`)
  - The C++ build: glslang is in every program and test binary that
    links `mg-module` (about 40 s of C++ on a cold build, once).
- **The game's lit shaders drawn (M1 of [the shaders
  proposal](shaders-proposal.md)):** what `Renderer::game_shaders`
  leaves out (the Shaders switch, on by default since M3; Moonglow
  Viewer draws them since its 0.2.0). (from M1, 2026-10-21)
  - The Domains of Dread's fountain is 10.6 of 255 from the client on
    average with the Shaders switch on (the game's `fswater` with the
    framebuffer's copies; 11.7 with Moonglow's own water; 13.7 with
    Moonglow's own shader), with the area's fog or without. Not the
    falling water's phase: compared at the client's own clock
    (2026-11-02, `batch7_fountain_phase`: the debug `inc_standard`
    showing each picture's `worldtimerTimeOfDay`, Moonglow drawn at it)
    it is 10.57, 10.54 and 10.57, and 10.57, 10.53 and 10.56 half the
    flow's 5-second cycle off (Moonglow's own pictures half a cycle
    apart differ by under 1 there). What differs is the basin's water:
    11 to 12 of 255 over it, its mean color within 2, the client showing
    the basin's floor through the water, its pattern and all, where
    Moonglow's surface is smooth. Not read: what the client's refraction
    takes there (the copy's detail, what is drawn before the copy). (S–M:
    `fswater` over a patterned floor, the client's beside Moonglow's)

- **Tier A of the game's shaders (M2 of [the shaders
  proposal](shaders-proposal.md)):** what the Shaders switch leaves out
  or unchecked. (from M2, 2026-10-21)
  - **The framebuffer's copies** (`texFBColor`/`texFBDepth`, [the
    framebuffer proposal](framebuffer-proposal.md)) are made after the
    solid meshes only (a material's `sample_framebuffer` drawn late,
    after the shadows, since 2026-10-30). Over water (read 2026-10-30,
    `reading_framebuffer_copy_over_water`: cards painting the copy above
    them): a `sample_framebuffer` mesh's copy holds no water in the
    client, as Moonglow's; a solid or `transparency 1` mesh's holds the
    water in the client (likely the frame before's last copy: the camera
    was still), where Moonglow gives it the solid meshes' copy (and so a
    soft particle over water fades against what lies under it). Giving
    those the frame before's picture: a second pair of copies (30 to 40
    MB at the area view's size), filled from the targets as the frame
    begins where the frame before was of the same view; a picture drawn
    once has no frame before. (S–M)
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
  - **An area changes its look a moment after it opens**, the first time
    its programs are compiled (stock: 50–113 ms of the worker's; TDoD:
    250–277 ms) and, with them cached, while naga reads them and the
    driver makes the pipelines (28–40 and 123–146 ms): Moonglow's own
    shader draws until they are ready. Keeping the driver's pipeline
    cache (wgpu's `PipelineCache`, Vulkan's alone) was tried on
    2026-11-02 and measured to shorten nothing here: Mesa's own on-disk
    cache already keeps the driver's part (area092's 21 pipelines 65 ms
    of the worker's without it, 67 with it; TDoD's woods 129–165 and
    135), and with Mesa's cache off the area took 319 ms with wgpu's
    cache read against 371 with neither (the programs' cache alone); it
    was not kept. What is left is naga's and wgpu's own part (reading
    the programs, writing them for the backend). Making the pipelines of
    the programs a module needs when it opens, before its areas are, was
    looked at on 2026-11-02 and left for a decision: a pipeline is keyed
    by its mesh's traits as well as its programs (skinned, tangents,
    two-sided, the pass, and `NO_DISCARD`, which the texture's alpha
    decides), so knowing them means reading every area's models and their
    textures when the module opens (The Domains of Dread: hundreds of
    areas), a real cost in memory and time where today an area reads its
    own when it opens. Two narrower ways, each the owner's to choose:
    the keys an earlier session made for the module remembered in the
    shader cache and sent to the worker as the module opens (no models
    read; a key whose programs a hak no longer has fails silently), or
    an area's pipelines asked for when its row is hovered in the module
    tree. (S–M, waits on the choice)
  - A program whose uniforms can't be split falls back to Moonglow's
    shader, reported. Since 2026-11-02 a block a program names besides
    its loose uniforms (`uniform Name { … };`) is split with them; what
    is left is an array of named blocks read by a variable's index (none
    of the 102 programs seen has one). Drawing that would need its blocks
    written for each draw (its own, its lights', its bones' and the
    frame's values in one) and bound at the draw's own offset, instancing
    off for it: M, for a program not seen. (M)
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
  - **Static geometry's buckets** (a static placeable's meshes and a
    tile's; read on 2026-11-02, `batch7_bucket_order`; [the
    findings](findings.md), "Static geometry is drawn in buckets"): in a
    static placeable and in a tile alike, the meshes that share a
    texture are drawn with the first one's (in the model's order)
    `NO_DISCARD` and opacity. Three pairs of cards (plain, a red decal,
    an environment map), each an opaque mesh and the same with `alpha
    0.5`: opaque first, every card `NO_DISCARD` 1 and drawn solid, the
    `alpha 0.5` one too; see-through first, every card `NO_DISCARD` 0
    and drawn half see-through, the opaque one too; each card keeps its
    own program. Moonglow decides each mesh's alone. The fix (mg-render's
    `game::no_discard` and the blending of `Renderer::slots`, another
    package's this round): a tile's meshes, and a static placeable's,
    that name the same textures (their names, case not counted) draw
    with the first such mesh's alpha (blended below 1, else opaque) and
    `NO_DISCARD`. Whether a bucket gathers meshes of several models (two
    static placeables of one texture) wasn't read: each row of cards
    had textures of its own. (S)
  - Moonglow's own shader in the builders' areas (TDoD's 690 DXT1
    textures with transparent texels black, its 21 materials naming maps
    without a render hint) waits on those areas' files (`tdod.mod` here
    holds three staging areas). (A DDS with the rippling procedure is
    drawn flat gray as the client draws it since 2026-11-02:
    [findings](findings.md), "A DDS with a rippling procedure loses its
    picture".) (S)
- **The special surfaces (M5 of [the shaders
  proposal](shaders-proposal.md)):** water, grass, the sky, additive
  meshes and particles, read in the client (`surfaces_in_the_client`)
  and drawn with the game's programs or kept Moonglow's. (from M5,
  2026-10-21)
  - **Soft particles** fade as the client's with the Shaders switch on
    (`particleSoftness` 10 over the emitter's larger size, a `Linked`
    emitter's and a lightning bolt's too: read on 2026-10-30,
    `reading_soft_particles_of_lightning_and_linked`, and Moonglow's bolt
    made soft); a Lighten particle's foot fades a little more than the
    client's (6.2 of 255 away over its last rows). (S)
  - **Particles' size:** a still emitter's square particles are drawn
    upright, as in the client, 28 pixels a side against the client's 30
    in `surfaces_in_the_client` (the fitted camera's error, or the size:
    not told apart); a spinning one starts upright and turns by
    `particleRot` radians a second, read and drawn on 2026-10-30. (S)
  - **The switch on still costs the water view about 0.9 ms of
    encoding** (3.6 ms against 2.7 off, at 6.5 ms a frame within the
    budget by 0.4): about 0.75 ms the draws' own values (the material's
    template copied, `m_mv`, `m_mvp` and `m_normal` with an inverse each
    draw), the upload 0.09 ms, the submit 0.07 ms. `m_normal` from the
    3×3 part, or values that the camera alone changes kept per instance
    and multiplied in the vertex program, would cut it. (M)
- **Emitters a builder does not see** (GitHub issue 8: a demon's wings,
  braziers, shafts of light, all without particles in his area): not
  reproduced. The game's own balor, fire elemental, campfire and brazier
  show theirs in the area view here (`look_particles_of`), and a static
  brazier's fire and smoke run in the client as a dynamic one's, as in
  the area view (`reading_static_placeable_emitters`, 2026-10-30), so it
  is his content, a setting (Animations off shows none) or his machine;
  asked of him. (?) (from builders' reports after 1.16.2)
- **A `Linked` ribbon and a flip-book** (a builder's custom fog was
  drawn as bright streaks): what the client draws is in [the
  findings](findings.md) ("Particles: kept, flip-books and ribbons"),
  and the fog shows as an even haze, as in the game. The picture's turn
  within a piece is Moonglow's (read on 2026-10-30,
  `reading_particle_turn_and_birth`: the newest piece shows the cell
  turned half round in both, its pieces drawn newest last). Read on
  2026-10-30 (`reading_ribbon_pieces_and_order`, the fifth client batch,
  a texture whose green is its v): a client piece starts its cell at its
  older particle and stretches it past the next, which starts again; the
  rule, fitted on 2026-11-01 (`batch6_ribbon_sizes`, sizes 0.15 to 1.2
  m; [the findings](findings.md), "A `Linked` ribbon's cell runs half a
  particle past the next"): the cell reaches the next particle at
  `gap / (gap + size / 2)` (within 0.01 at a metre's gap; at half a
  metre it reaches 0.85 and 0.67 at 0.3 and 0.6 m, where the rule says
  0.77 and 0.63), where Moonglow's ran from one particle to the next
  exactly. Since 2026-11-02 a piece's v runs to `gap / (gap + size /
  2)` at its newer particle, as the client's. Left, among
  see-through meshes: a mesh of `alpha 0.5` in front of a ribbon hides
  it entirely in the client, of the ribbon's own model or of another
  (2026-11-01: a placeable's card 0.6 m before another's ribbon), where
  Moonglow draws the ribbon over it whole, its see-through part writing
  no depth (`alpha 0.5` meshes writing depth is the see-through meshes
  entry's question too: M). A model with no mesh to draw (this fog) has
  no bounds of its own to frame a view by. (S) (from builders' reports
  after 1.19.1)
- **Builders' reports after 1.21.0:** a creature's body part can be
  set to none (0), as in Aurora; a body part whose mesh is a skin is
  drawn as the client draws it ([findings](findings.md), "A skin mesh
  in a body part or a held item": not where it is compiled, a plain
  mesh where it is text; a held item's never). Not done: a skin
  weighted to nodes of its own part, which the client draws out of
  place, is not drawn (S). The compiled skins tried were `mgv
  compile`'s: the game's own compiler (`nwmain compilemodel`) compiles
  none (2026-10-30, `reading_game_compiler_skins` in the fifth client
  batch: a part as a plain mesh written, 1,400 bytes, the same as a skin
  nothing), so a compiled skin is always another compiler's. Not
  reproduced: "unequipping an armor sets all body parts to 0": nothing
  in the code writes a body part on unequip, and the preview draws the
  creature's own parts then (waits on the creature it was seen on).
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
  sort by their middles, where the client draws a model's meshes of
  mesh `alpha` under 1 in the model's order, each writing its depth: a
  card listed after another and behind it is hidden where they overlap,
  and a ribbon behind one is hidden too (read on 2026-10-30,
  `reading_see_through_crossing` and `reading_ribbon_pieces_and_order`
  in the fifth client batch; across models not read). Drawing them so
  (by model order, with depth, and the soft edges of a texture's alpha
  kept as they are) (M). Each see-through mesh costs two draws. On
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
  (taken as the size against the model whose animation is played); a
  body part drawn only where the base has its node; a mesh with `bitmap
  NULL` given the part's own texture; a bitmap name over 16 characters
  cut to 16; a blank line among ASCII skin weights read as a vertex
  without any (its faces not drawn since 2026-10-29, as the game draws
  none of a vertex no bone weighs), and since 2026-10-30 as spacing only
  in a list that has all its rows without it. The reporter's "stopped
  animating" (Mindwitness) wasn't seen as such: here it animated, with
  its skin torn by the shifted weights. An animation through a chain
  of scaled supermodels plays at each link's scale but the owner's, as
  the client's (since 2026-11-02; [the findings](findings.md), "An
  animation played through a chain of supermodels is scaled by each
  link"). Waiting: the model
  viewer's own check of names over 16 characters (the Moonglow Viewer
  session's); and the Mindwitness's pose, other than in Aurora and the
  game (the builder's model). (S)
- **Shadows:** the area view draws the sun's and moon's shadows
  (2026-10-28: a shadow map standing in for the game's stencil volumes,
  laid as the game's shadow plane lays them, checked against the client
  in `shadows_look`; [the research note](research/notes_shaders.md) §G),
  and creatures' and lights' shadows (the Shadows button's menu, done
  2026-10-28); since 2026-10-30 also as the game's own stencil volumes
  (Shadows quality High: [the notes](research/notes_shaders.md) §G.2),
  beside the client in `high_shadows_beside_the_client`. Left:
  - High's first frame in an area makes its casters' volume meshes in
    the frame: area084's opening's longest frame 22 ms with Fast, 48 to
    51 ms with High (`frame_perf`, 2026-10-30); its frames after are
    within the budget (a frame's GPU time 0.1 to 1 ms more than Fast's,
    3.3 ms more in area026 seen close, 6.7 ms of 6.9). Making them
    apart, as textures are read, goes beyond what was asked (M);
  - every creature in a lamp's reach casts at once (eight ogres in a
    row, each its shadow: `batch6_light_shadow_casters`), as Moonglow
    casts up to 16; and a player's own light casts the shadows of the
    creatures about the player (`batch6_player_light_at_night`), which
    the toolset, placing no player, has nothing of. (A light's shadow
    darkens by the light's share of the light there, as the client's,
    since 2026-11-02: [the findings](findings.md), "A light's shadow
    takes the light's share".);
  - what is drawn after the shadows receives none: the area's grass
    (see-through tufts: in `shadows_look` with `MG_SHADOW_GRASS=1` the
    shadowed ground darkens by 0.34 to 0.38, the client's by 0.42, its
    grass sparser and darkened), the solid parts of a tile's see-through
    meshes (alpha-blended leaves), shiny water drawn after the
    framebuffer's copy; the client's volumes darken whatever was drawn
    before its plane (M: those drawn later would read the map
    themselves);
  - a face casts the sun's shadow from its front only (read on
    2026-11-02; [the findings](findings.md), "A face casts the sun's
    shadow from its front only"): why the client leaves a city street lit
    under a sun low in the west (`reading_low_sun_shadow_length` with
    `MG_TILESET=tcn01`), where Moonglow lays the buildings' long shadows
    over it. The area's tiles taken one by one (`batch7_low_sun_city_tiles`),
    two alone darken Moonglow's street and not the client's, both on the
    area's west edge, turned to face into it (`tcn01_c02_07` and
    `c02_08`), their walls single faces with their backs to the sun;
    turned to face the sun, the first casts its long shadow in the
    client too. Of two single white walls (`batch7_wall_sides`), the one
    facing the sun casts, the one with its back to it doesn't, where
    Moonglow's Fast and High cast both. Sight has nothing to do with it
    (tcn01's house out of sight casts in both). At a cliff (case `cliff`
    of `high_shadows_beside_the_client`, now the tileset's own cliff
    tiles, `raise_north`): a placeable's shadow at the cliff's top
    reaches down onto the ground below, as Moonglow's (whose volume ends
    at the lowest receiver seen); the ground at the cliff's foot is lit
    in the client, where Moonglow darkens it: the rock face, its front
    to the south, casts nothing (0.62 of the shadows' pixels agree, Fast
    and High alike). The fix (mg-render's `shadow.rs` and `volume.rs`,
    another package's this round): the sun's and moon's casters cast
    from the faces whose front faces the light only: the shadow map
    drawn with the faces turned from the light culled, and High's
    volumes without a prism for a face turned from the light (the open
    mesh's "both sides" dropped; a closed mesh's lit faces cover it as
    before). (S–M) (The fade with the height between plane and caster
    that 8193.31 names is not seen: armoires raised 0, 1.5 and 3 m
    darken the ground alike, 0.44 to 0.49, and casters above the camera
    still cast, as in Moonglow: `reading_shadows_of_doors_items_and_heights`,
    2026-10-29);
  - the previews (blueprints, the model viewer) draw none: they have no
    ground to lay a shadow on (a creature or a dynamic placeable casts
    only onto tiles and static placeables in the game), so a shadow there
    means a ground under the model (or one that only takes shadows), a
    change to every thumbnail, gallery and hover preview: the owner's
    call (S, waits on it). (With Fade Geometry on, the faded roofs and
    upper walls cast their shadows, as the game's, since 2026-11-02.)
  (asked by the Domains of Dread module session and by a builder,
  Troubletcat)

## Models and textures

- **Heed's previews after 1.33.0** (not reproduced): the Drow shields of
  the standard palette shown as bags in their thumbnails and the
  palette's hover preview. In the base game they show whole
  (`item_previews` in `screens.rs`: `x2_adrowshl001`). What makes a bag
  (2026-10-30): the Drow shields are large shields (base item 56,
  `ItemClass` `AShLw`, `ModelPart1` 33: `ashlw_033`), and baseitems.2da
  row 56's `DefaultModel` is `it_bag`, which `mg_preview::item_parts`
  falls to where the class and number name no model in the load order:
  a hak's `baseitems.2da` whose row 56 names another `ItemClass`, or a
  load order without `ashlw_033`. Waits on his hak list or a blueprint.
  (His armor showing only a head and pelvis was Beamdog's HD bodies,
  drawn whole since 1.34.1.) (S–M)

## The main window and its panes

- **The light theme's own colors:** it is egui's light theme; the script
  editor's syntax colors and the conversation editor's line colors
  follow it, the Factions' sliders keep Aurora's red and blue (legible
  on both: `conversation_and_factions_in_both_themes`), and the area
  view's overlays are drawn over the 3D view, whatever the theme. What
  else a builder who uses it finds wanting waits on such a pass. (S) (a
  senior builder's review, October 2026)
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
  at its first row), the log last, which they scroll (done 2026-10-30);
  in the Gallery Up and Down go a row of pictures at a time (done
  2026-10-29). Not done:
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
  and the after-create hook; 0.5 (3.3.0) a batch of strokes, more kinds
  of setting, more of what a command is for and the menus it is in, a
  fix that asks, archives of several plugins and the rest of this
  round's gaps):
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
    feature of its own. (L) (After 3.0.0, the owner's word, 2026-10-30:
    9th of 9 by complexity.)
  - **Not tried in the application on a desktop:** the folder and the
    hak question were driven through the UI harness and `mg`, not with
    the real file dialogs (the file dialog of `open_file` filters by
    the extensions asked for; that filter was not seen on a desktop
    either); nor were a project's plugins' question (and Allow All, the
    folder looked at while open), the settings' form (the kinds of 0.5,
    Choose… for a folder, Tidy Up; rendered as `plugin-settings` and
    `plugin-settings-kinds`), the menus of 0.5 (tiles, categories,
    groups, tabs, the script editor, Find References, Advanced Search),
    Verify's fixes (a fix that asks, Fix All, Undo) and the events of
    3.3.0 (Save As, the makers, Manage Plugins' time): all driven
    through the UI harness. Waits on a desktop session. (S)
  - **No list to install or update from:** plugins come as files; the
    proposal has an index with API 1.0. Remove is for what Install from
    File installed; a folder copied in by hand is deleted by hand. (M)
  - **More kinds of things to add:** exporters and importers, wizards,
    panels. (L) (After 3.0.0, the owner's word, 2026-10-30: 8th of 9 by
    complexity.)
  - **The 2DA editor's faults with fixes:** the 2DA editor is another
    package's this round; its faults offer no fix yet. Waits on that
    work. (S)
  - **The type file was not tried in an editor:** it loads as Luau, and
    tests hold it to have every name and the reference's parameters
    (`the_type_file_has_the_whole_api`, `the_reference_says_what_the_code_has`),
    but it was not checked with the Luau language server: neither
    `luau-lsp` nor `luau-analyze` is on the development machine, and the
    Luau that mlua builds in has no type checker (no `Analysis`). Waits on
    a download of `luau-lsp` (or `luau-analyze`), the owner's to allow.
    (S)
  - **External programs** as plugins (a protocol over standard input and
    output, for Python or Nim): designed (the
    [proposal](plugin-proposal.md#external-programs-the-design-2026-11-01):
    JSON-RPC over standard input and output, the reference's names as
    its methods, marked as running with the user's rights), not built:
    left for its own round, after this one's S items, in the owner's
    order of the brief ("last, if room remains"). (M)
  - **The window with plugins was run on Linux only:** the application
    itself was driven on an off-screen display there (plugins installed
    from archives listed and enabled, a command as a background job, a
    form answered, Undo, a check in Verify Module, the enabled plugins
    kept over a restart, `--no-plugins`), and the rest through the UI
    harness. On Windows and macOS the tests pass in CI, but nobody has
    used plugins in the window there; and the file dialog of Install
    from File was not driven anywhere (the harness answers for it). (S)
  - **A table's description's `faults` looks no strings up**
    (`mg.twoda.describe`): a `strref` column's string is not held to a
    talk table, which would come from the plugin host. The last of the
    2DA editor's entry (the rest done in the deferred sprint,
    2026-11-02); left because `crates/mg-plugin` was another package's
    that round, not begun. (S)

## Tileset authoring

- **A tileset with holes** (a builder's report: a big old tileset,
  merged by hand, drew no tiles at all) is read tile by tile, as the
  game reads it: a tile without its section or a model draws nothing,
  the area's problems and Verify (`set-hole`) name the holes, the line
  ends and lengths the server refuses are errors and a mix it reads a
  warning (`set-line-ends`, the rule read in the engine 2026-10-30), the
  tileset editor shows the holes, and a model that is there and can't
  be read is said to be, with why, not "not found". Not done:
  - The builder's own tileset (trf01, 2813 tiles) has no holes and CRLF
    line ends, and Moonglow draws its tiles whose models the game has:
    why it draws none of theirs is not known without their hak (its
    models, or the module and the haks it lists). The log now says,
    for each model left out, whether it was found and why it can't be
    read. (S, waits on the builder's hak)

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
  `override` folder as a layer under the user's own, its `portraits`,
  `ambient` and `music` under the user's and over the game's, its `hak`
  and `tlk` folders searched. Which of two items' wins in the game isn't
  known (the lower number here), and where the game puts them against
  the user's own folders is from how it is described, not measured: the
  client runs here with Steam hidden, never started, so it waits on a
  builder's reading with Steam. (S) (from builders' reports after 1.14.1)
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
  and ×). (L) (GitHub issue 28) (After 3.0.0, the owner's word,
  2026-10-30: 7th of 9 by complexity.)
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
  all fifteen pages are unchanged upstream (the mirror's sync of
  2026-10-26). Each page's edit is ready to paste in
  [wiki-corrections/](wiki-corrections/index.md) (the passage, its
  replacement, an edit summary, the evidence); submitting them waits on
  the owner. (S) (the documentation audit of 2026-10-12)

## The code itself (the code review of October 2026)

A review of the whole workspace for dead code, clumsy code and code
written twice found more than it changed. What it changed (some forty
commits, with a test for all but one of the bugs it fixed) is in the
history; what it left is done, the last of it (typing in a very large
script) after 3.0.0, on 2026-11-02 (the [log](log.md)).

## Written more than once (the review of 2026-10-21)

Five reviewers read the workspace for logic written more than once, a
builder having said that AI-written code "reimplements a lot of stuff in
minorly different ways instead of building one source of truth". Where
copies had drifted into a wrong result, the fix went through one place
with a test (the log, 2026-10-21). This is what is left: places by file
and line as they were that day, found by reading, not by running unless
it says so. One question settles most of them: which copy is the source
of truth, and does each caller's difference matter.


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

- **The grass is drawn across the whole area view, not faded with the
  camera's distance as in the game** (the owner's word, 2026-11-02): the
  client gives the grass's diffuse an alpha that falls with the tufts'
  distance from its camera, about `(32 − d) / 16` (0.94 at 17 m, 0.71 at
  20, 0.49 at 24: `batch6_grass_material`, [the findings](findings.md),
  "The grass's material"), its grass ending at its render distance; the
  area view's camera stands farther off (often 40 m and more), so the
  rule would leave most of the view without grass. Its places, sizes
  and count are Moonglow's own too (the field's mean color 0.1 of 255
  from the client's). Said in the manual's
  [differences](manual/12-differences.md).
- **The game client's clock's start** (`worldtimerTimeOfDay`; the
  shaders proposal's M1): Moonglow's clock starts at 0, by design: the
  client's starts from the machine it runs on, so no client's value is
  there to match. Read on 2026-11-02 in the client's code
  (`CWorldTimer`) and in runs with its clocks moved
  (`batch7_timer_origin`, `tools/nwclient/timeshift.c`; [the
  findings](findings.md), "The client's clock starts from the
  machine's"): its world timer is made as the game's services start (a
  second or so before the player enters the area) at 23:59:56 of a day
  of 5 minutes an hour (10,496,000 ms of 7,200,000) against
  `CLOCK_MONOTONIC`, and the module's minutes an hour, set as the module
  loads, change the day's length and not that offset: the timer is
  `(10,496,000 + 7,200,000 × ⌊the boot clock then / 7,200,000⌋ + the
  milliseconds since) mod the module's day`; the earlier batches' two
  values two hours apart were steps of the boot clock. A comparison
  reads the client's from its picture (`batch7_fountain_phase`).
  (settled in the deferred sprint, 2026-11-02)
- **The feminine talk table over NWSync** isn't published, as
  `nwn_nwsync_write` doesn't publish it, by design: the client never
  reads it for its own lookups. Read in single player (2026-10-29,
  `reading_feminine_talk_table`) and over NWSync (2026-11-02,
  `joined_nwsync_download` with `MG_NWSYNC_CLICK=1` and
  `MG_NWSYNC_FEMININE=1`: the feminine table published and downloaded
  beside the masculine one, the player a woman in the game): the client
  draws a line by StrRef from the masculine table for her, while the
  server's `GetStringByStrRef` with `GENDER_FEMALE` reads the feminine
  one from its own `tlk`. Getting there needed the server to keep its
  module loaded when the last player left: [the findings](findings.md),
  "A module reloaded when empty is advertised without NWSync".
  (settled in the deferred sprint, 2026-11-02)

- **The 2DA editor puts no row between others** (Delete Row takes any
  row away since 1.22.1), sorts no rows and edits no `DEFAULT:`, by
  decision: a row's number is what every other table, blueprint and
  script names it by, so a row put between others would move every row
  after it under what names them (settled in the deferred sprint,
  2026-11-01).

- **The custom palettes where a save differs from Aurora's, or sees no
  change** (settled in the deferred sprint, 2026-11-01; the rest of the
  entry was fixed then: the log). By choice: a save puts in order only
  the categories an entry went into and keeps the fields it doesn't
  know, where Aurora's puts every category in order and drops them, so
  that a palette no change reached is the same bytes (a commit shows
  only what the builder changed); a new entry for a talk-table name is
  its `STRREF`, as Aurora's Build writes it, where Aurora's Edit Copy
  writes the text; a module without a `palcus.itp` gets one made in
  full at a save, where Aurora writes the categories alone, empty (of
  use to no one until a Refresh). As in Aurora, by design: an entry
  made stale where Moonglow saw no blueprint change while the module
  was open (another tool while it was closed, a merge, an `mg` command
  without a game install, work recovered after a crash) stays until
  its blueprint next changes, Refresh Palette or Build (finding it would
  mean reading every blueprint at every save, which is what Refresh
  does); a module's own categories changed outside are taken in as
  they are; a hak's skeleton changed, or a hak's blueprints renamed or
  removed, leave the module's palette as its file is (Refresh Palette
  and Build make it anew; Remove from Palette takes one entry out); a
  blueprint of category 255 or of one the palette lacks is left out by
  Refresh Palette, as Aurora leaves it out. (El Grillo's and
  Jasperre's report, October 2026)
- **A project's plugin is allowed by a digest of its files, not by a
  signature** (the deferred list's "Plugins, beyond API 0.4", 2026-11-01):
  the digest says the plugin is the very one the builder allowed, which
  is what the question is about; who wrote it is the team's checkout,
  which the builder chose to open. Signing would need keys for every
  team and a way to trust them, for no more than the digest already
  holds the builder to. Kept with it, by the same decision: a no holds
  for the plugin's id in that project whatever it becomes (Manage
  Plugins allows it later); what its events were allowed (a folder, a
  yes) is kept by its digest, so a changed one asks again; `mg plugin
  run` runs the folder it is given, allowed or not (it is named on
  purpose); a project's plugin and one installed of its id share their
  settings of scope `user` (it stands in for that plugin, and the
  user's own values are the user's); a module kept in one file has no
  project, so no project plugins.

- **Which findings have fixes, and where** (the deferred list's
  "Plugins, beyond API 0.4", 2026-11-01): a fix is worked out when
  Verify runs and held to the resources being as Verify read them, so a
  change since asks for Verify again (a fix made on what nobody looked
  at would land elsewhere); its Undo now offers it again. The doctor's
  fixes are those whose answer is plain (`tag-unique`, `area-variables`,
  `var-runtime`, `static-transform`): `pc-faction`'s faction, a missing
  reference's resource, a model's or a tileset's repair are the
  builder's to choose, and a plugin's check may offer one that asks. Build
  Module's results are missing and unused resources, which have no fix
  but Mark as Used, there already; the tileset editor's faults are of a
  tileset being made, edited in its own fields. Fixes of different
  checks are made one at a time, or a check's all at once (Fix All).

- **What a resource's own events are not for** (the deferred list's
  "Plugins, beyond API 0.4", 2026-11-01): a save of what is no resource
  of the module (a hak's, a talk table's, a 2DA file's own editor) asks
  no plugin, whose events are of the module's resources; a script's Save
  puts its text into the module, and the module's save is asked; Build
  Module and `mg watch` write a build and a development folder, not the
  module; the custom palettes Moonglow makes again at a save are made
  from the blueprints a handler reads already. What a plugin makes
  itself fires none (or plugins would answer each other), and spawn
  points, the start location and an encounter's points are no placed
  objects. A handler of a placing, a paste or a making edits and logs:
  warning of or refusing what the user just did is the save's to do,
  where the Before saving window asks. A fix there is made one at a
  time, the save asked again on what it now is; "as it was" is as each
  file was last read or saved (a module kept in one file: as it was
  saved or opened, in this run), what the window can know. A handler is
  stopped between steps of its code: a call of the API is Rust code that
  runs to its end, and the time is looked at when it returns.

- **How a plugin's tests hold a job** (the deferred list's "Plugins,
  beyond API 0.4", 2026-11-01): a command's edits are held by the fields
  they leave, their count and the resources they change, not compared
  as an edit file (whose order and paths change with any refactoring of
  the plugin that makes the same module); a finding's message is matched
  whole, as a test of a check's words is meant to be; the game's data is
  the game `mg` finds, the one at a test's `root`, or the test's own
  `game/` folder, which is how a pipeline without the game tests what
  reads `ctx.game` (the game's files are never in a repository).

- **What a plugin's command is not offered on** (the deferred list's
  "Plugins, beyond API 0.4", 2026-11-01): a trigger's or an encounter's
  points and the start location are no selection in the window (a
  polygon is redrawn whole, Redraw Polygon; the start location is set
  where the area is right-clicked), so there is nothing of them to hand
  a plugin: it edits a trigger's `Geometry` through the trigger, and the
  start location through `module.ifo`. A conversation's line is not an
  entry of `ctx.selection` either: the conversation's tab menu has the
  commands for `dlg`. A palette's blueprint menu has no Properties to
  end it, so there the plugins' commands come last, by design.

- **A plugin's settings outside Undo, and a `path` it reads no file by**
  (the deferred list's "Plugins, beyond API 0.4", 2026-11-01): what the
  settings' form saves, and what a job sets, is not a step of Undo: the
  values are files outside the module (Moonglow's own, the project's
  `moonglow.toml` and `.moonglow/user.toml`), as Options are, and Save in
  the form, or the plugin's own command that sets one, is the asking (the
  form says beside each where it is kept; the log says what was set, and
  where). A `path` stays text to the plugin, which reads no file by it
  and still asks with `ctx.ui:open_file`: a value of the team's comes
  with the checkout, not from the user, and a plugin reading a file by
  it would read what the user never chose.

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
  Moonglow's own shader lights it so too since 2026-11-02 (a cut-out's
  alpha, the mesh's times the texel's, weighs the light from behind
  though it is drawn solid: `batch6_cut_out_lighter`'s disc of mesh
  `alpha 0.5` 156 of 255 as the client's, where it was 126). With The Domains of
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
- **The shadows in two qualities** ([notes](research/notes_shaders.md)
  §G.2): Fast (the default), a shadow map standing in for the game's
  stencil volumes, its edges a texel of the map soft (stepped up close
  in a large area seen to its far side); and High (Options › Area ›
  Shadows quality, and the Shadows button's menu), the game's own
  stencil shadow volumes, as the owner asked on 2026-10-30. Settled
  2026-10-28 as the map alone; rewritten 2026-10-30 for both. In
  either, a caster that isn't closed (an edge of three faces or more)
  casts its faces' shadow, no spike, as the client (89.8193.37) drew
  the wiki's "finhead".
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
- **Tile Properties on several tiles** writes only the controls set
  into every tile chosen. Aurora's OK writes every value shown, the
  first tile's, into all of them, and the source lights and loops it
  shows disabled as 0 (captured 2026-10-30, `captures/wave2/tiles-one.mod`:
  Main Light 1 changed alone on two tiles of lights 5, 6, 1, 2 and 10,
  11, 3, 4 left both 27, 6, 0, 0, their loops 0); Moonglow keeps its own,
  as the owner's report on Tile Properties asked. (Settled by the
  owner, 2026-10-30.)
- **The game's text in the language edited, and the feminine talk
  table:** the install is read as that language when its
  `lang/<code>/data/dialog.tlk` is there (its talk table, and the rest
  of its `lang` folder); a module's custom talk table is one file
  whatever the language (`the_install_s_language_is_the_one_edited`).
  The feminine table is not shown: Aurora shows a line by
  its StrRef from the masculine table alone (a custom table's line 1 in
  its conversation editor, an item named by line 0 in its palette:
  "W2 MASCULINE", with `w2f.tlk` beside it saying "W2 FEMININE",
  2026-10-30), as Moonglow does: the feminine table is the game's for a
  woman's character. (Settled in Aurora.)
- **`mg` builds the GPU stack by default** for `mg nui render` (drawn
  with wgpu) and `mg nui layout` (a frame of the NUI Creator's canvas,
  egui, no GPU), both of mg-ui: 229 crates, 2 min 42 s cold (`cargo
  build --release -p mg`, 4 jobs), 37.8 MB. Without the feature
  `nui-preview` (`--no-default-features`) `mg` is 140 crates, 87 s and
  22.7 MB, and has neither command (clap says "unrecognized
  subcommand"). The packages keep the default, so only builds of `mg`
  from source would gain from the canvas in a crate of its own, with
  egui and no wgpu, which was looked into on 2026-10-29 (the deferred
  sprint's package m11) and is several days: the crate would take
  `nui_view/preview/` (the canvas, about 3,400 lines), `skin.rs`
  (1,400), the painting half of `draw.rs`, `layouts.rs`'s variant in its
  window and `interaction.rs`'s session, and the 25 or so fields of
  `nui_view::State` the canvas reads; and what the canvas reaches into
  the editor for would become hooks (`design::node_name` and `Insert`,
  `structure::select`, `drag_target` and `Drag`, `widgets::field_label`,
  `widgets::dropdown` and `fields::hinted`). Kept as it is while no one
  builds `mg` alone. (Settled by the owner, 2026-10-30.)
- **`mg merge-gff`: an object both moved and retagged on one side** is
  taken as removed and another added (a conflict where the other side
  changed it): matched by its blueprint alone, an object one side
  removed while it placed another of the blueprint elsewhere would take
  the other side's change (the merge review's ruling, 2026-10-30; the
  chest A / chest B test). (Settled by the owner, 2026-10-30.)
- **A module's files changed outside, read on a thread of their own**
  (`mg_module::outside`; their reads' allocator arena is settled above):
  a reload that touches resources the undo history has changes to
  clears the whole history (undoing an earlier change to them could not
  be done from what they are now; a command's other edits are not taken
  apart from it); a script open in the editor with typing not yet
  applied counts as changed here (it is unsaved work, and the window
  asks which to keep); the haks and the `override`, `development` and
  portrait folders (and the custom talk table) are read again on the
  window's thread after a change, measured on an `override` of 20,000
  files at 20 ms each time something in it changes (the look that finds
  the change 7 ms; the machine's load 2.2 on 32 CPUs): three frames at
  144 a second, once a change. The three kept as they are. (Settled by
  the owner, 2026-10-30.)
- **An inventory's item made from the blueprint, between a placed object
  and a blueprint:** an item carried (dragged, pasted) between them is
  made from its blueprint where it lands, keeping its flags, as a
  blueprint's inventory holds only a blueprint's name. (a second
  builder's review, October 2026) (Settled by the owner, 2026-10-30.)
- **An inventory's moved row keeps its grid place:** a row dragged
  within a list takes its place in the list, and keeps its `Repos_PosX`
  and `Repos_Posy` (where the game's inventory grid shows it), as
  Moonglow shows a list, not Aurora's grid. (a second builder's review,
  October 2026) (Settled by the owner, 2026-10-30.)
- **A Polish name in the English text, with another editing language:**
  String Edit reads and writes the Windows-1252 languages' text as
  Windows-1250 when Polish is the editing language (`text.rs`'s
  `entry_codepage`); with another editing language a Polish name kept
  in the English text is read as Windows-1252, as an English game and
  Aurora (the system's codepage) show it: Moonglow has nothing else to
  know the text's codepage by. (issue 13's other half, from builders'
  reports after 1.19.1) (Settled by the owner, 2026-10-30.)
- **What the Module Uses, a script's strings and the release cut**
  (`--usage`, `mg_preview::usage`): a script's string counts as a use of
  whatever it spells, whatever the string is for (a release cut keeps
  too much rather than too little; said in the manual), and a per-hak
  list for the release cut (a folder of the used assets to pack smaller
  haks) is not written: `--json` has the lists, and writing haks from
  them goes beyond what was asked. (Settled by the owner, 2026-10-30.)
- **Custom lights' rows and models no light uses any more** stay in the
  hak (Custom Light…, `mg_module::lights`), as a row's number is what
  placed lights know it by. (from builders' reports after 1.16.2)
  (Settled by the owner, 2026-10-30.)
- **A tile's emitters' keys:** a tile's emitters take their keys from
  the animations playing on it as one animation as long as the longest
  (`scene.rs`); the game's tiles' keys are constant, and one that
  changed over a shorter loop would run slow, but no tile of the game's
  has such keys. (from builders' reports after 1.14.1) (Settled by the
  owner, 2026-10-30.)
- **Grass to the end of the view:** the client's Grass Far Render
  Distance (`graphics.grass.render-distance`, 900 by default and at the
  least, to 30,000) is in square meters, the grass fading out over the
  last fifth or so of its root from the eye: at 900 the white tufts are
  whole to about 22 m and gone by 28 to 30 m (`reading_grass_placement`
  with `MG_GRASS_DISTANCE`, 2026-10-29), at 3,600 they reach the horizon
  of the 4 by 4 area. Moonglow's go on, whole, to the end of the view,
  as the owner asked of an editor's camera on 2026-10-23 (grass seen
  only about its target was what he took for a fault; from an editor's
  height, 30 m from the eye is no grass at all): not cut at the
  client's distance. (Settled by the owner, 2026-10-30.)
- **`mg pack` compiles models only when asked** (and a nasher project's
  Pack, by its project file's `[pack] compile_models`; the compiler and
  writers in `mg-mdl`, moved from the viewer's `crates/mgv-mdl` at its
  0.1.6): the game reads text and a pack's bytes should not change
  unasked, so it stays off by default. (Settled by the owner,
  2026-10-30.)
- **A module's `encoding.2da`, the four kept by choice** (EE 1.87: a
  table of the Unicode character each byte of the game's text stands
  for; the toolset, plugins and `mg` read and write a module's text by
  it wherever they are given the module, as the server reads it,
  `engine_encoding.rs`): a prefab's file keeps the bytes (written as
  Windows-1252 text, so that it is the same object in any module); a
  hak's description is the hak's, not the module's; a nasher project's
  sources are converted by its `nasher.cfg`'s `--nwn-encoding`, as
  nasher's own tools convert them, so that the bytes nasher packs are
  the module's; and the toolset reads the game's text by Options ›
  Language, not by the user folder's `game.language.codepage`, which
  the server honors and `mg lsp` reads. An exported or imported
  conversation is spelled over as a whole, its punctuation too (of no
  matter unless a table moves ASCII's own characters). (a builder who
  knows the engine, after 1.19.1) (Settled by the owner, 2026-10-30.)
- **Where the NUI preview is known to differ from the game** (stated in
  [the manual](manual/18-nui-creator.md);
  `crates/mg-ui/src/nui_view/preview.rs`; `mg nui render` and `layout`
  show the canvas): an encouraged control shows its hover look and does
  not pulse (the pulse's period not measured): a still preview shows
  the pulse's peak; and `mg nui layout`'s `overlaps` come of the
  canvas's own layout, which places neighbors apart, so it finds none
  but where the game would lay the window out otherwise, which it can't
  know: said in the manual's command line chapter, as it is. (the
  review of the NUI Creator's pull request, October 2026) (Settled by
  the owner, 2026-10-30.)
- **F6 against a builder's own binding:** a command a builder already
  gave F6 to is in conflict with Next Pane; Options › Keyboard lists
  the conflict, and the pane's wins until one of them is changed: the
  builder's binding is theirs to change. (Settled by the owner,
  2026-10-30.)
