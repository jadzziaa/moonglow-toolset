---
type: Manual Page
title: Modules
description: Modules - opening and saving, where things are used and renaming, find and replace, advanced search, nasher projects, merging in git (mg merge-gff as the merge driver), the project file (moonglow.toml), your builder code, described variables (variables.toml), claimed and protected areas (the claims table), the project's commands (File › Project, update and submit), recovering unsaved work, Module Properties, haks and talk tables, the hak editor, the 2DA editor, import and export.
tags: [manual, modules, haks, nasher]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-22T20:30:53Z }
---

# Modules

## Opening and saving

Moonglow opens modules as `.mod` archives, folders (a module unpacked into
a directory), nasher projects (see below) and the game's campaign files
(`.nwm`, read as modules).

- **File › Open Module…** (Ctrl+O), **File › Open Folder…** (a module
  folder or a nasher project), **File › Recent Modules**, or a module named
  on the command line (`moonglow path/to/module.mod`; with `--user-dir
  FOLDER`, another user folder for that run). A module opens on
  the area you opened last in it; the first time, or when that area is
  gone, on the first the module tree lists (Options › General switches
  this off).
- **File › Save** (Ctrl+S) writes the module where it came from, as an
  archive or a folder. **Save As…** writes it as a `.mod` elsewhere. A new
  module is offered as `<name>.mod` in the user folder's `modules`, where
  the game (and Test Module) finds it.
- In a module folder a resource is one file, named in lower case
  (`bread.uti`). A file Aurora named otherwise (`bread.UTI`) is read all
  the same and renamed at the next save; where a folder has both, the
  one changed last is the resource, and the save leaves that one file.
- **A module folder is worked on in place**, as in Aurora: keep its
  scripts open in an editor of your own while the module is open here.
  Every few seconds (Options › General: reloading), files saved, added
  or deleted in the folder by another program are read again (in the
  background, as a nasher project's are), and the log says which. A save never writes over them: a file changed outside
  that you did not change here stays as the other program left it, and
  a file new to the folder is not removed. Where a file was changed
  outside and here both, yours is kept, Save writes nothing and says
  which files, and **Changed Outside Moonglow** asks which to keep. A
  script read again is compiled if **Automatically Compile Scripts on
  Save** is on (Options › Script Editor), as one saved here is, and so
  are the scripts that include it (more than 24 at once are compiled
  in the background, and the log says so). The script editor's external editor
  opens the folder's own file.
- Saving is safe. A module archive is written to a temporary file, then
  put in place, so a failure never leaves a half-written module. The
  previous version is kept beside it (`mymodule.mod.bak`). With **Create
  backups of modules** (Options › General), each save also keeps the
  module as it was in `<name>.BackupMod`.
- **File › Close** closes the module, asking first if there are unsaved
  changes; so does quitting.

Moonglow keeps everything in a module it does not understand: fields with
no editor, unknown resources, unusual orderings. An untouched module saves
as it was.

## Where things are used, and renaming

**Find References** shows everywhere the module names a script, area,
conversation or blueprint (the first question in a large module reads
all of it; the next ones, for a resource or a tag, only what changed
since). A number typed there that is no resource's name is a talk-table
line's StrRef: the module's names, descriptions, conversation lines and
2DAs that name that line are listed. It's in the right-click menus of the module
tree and palettes, the script editor's toolbar and **Edit › Find
References…**.
- **Places are readable**, such as `keep › creature GUARD › OnSpawn` or
  `guard_talk › NPC line 3 “Halt!” › action`. Click one to go there: the
  area with the object selected, the conversation line, the script line.
- **Tags work too.** Type a tag in the tab's field to find the objects with
  it, and the transitions, locked doors and conversation lines (journal
  categories) that name it.
- **Strings in scripts** that spell the name are listed separately, such as
  `ExecuteScript("guard_spawn", …)` or `CreateObject(…, "guard", …)`. A
  script may build names at run time, so a name can be used where no list
  shows it.

**Rename…** (in the same menus, or the References tab) renames a
resource and everything that names it, as one step that **Undo** takes
back:
- **What moves with it:** a script's compiled code, and an area's objects
  and comments.
- **What follows the new name:** objects' and areas' events, conversation
  lines, the module's area list and start area, objects placed from a
  blueprint, inventories and stores, `#include` lines.
- **Strings in scripts** change too if you tick **Also change … in
  scripts**. Check them in Find References first: a string may also be a
  tag or something else (Neverwinter Chess compares tags to `"pawn_b"`, its
  pawns' blueprint name). The log names strings left spelling the old
  name.
- **Recompiling:** scripts whose text changed, and every script that
  includes them, are compiled again.

## Find and Replace Text

**Edit › Find and Replace Text…** (Ctrl+H) finds text players read: the
module's names, descriptions, conversation lines, the journal and the rest
(map notes and such), in every language each string has.
- **What's searched:** choose the kinds, whether case matters and whether
  only whole words count.
- **What's found** is listed by place, as in Find References. Click one to
  go there; untick any to leave alone.
- **Replace** changes the ticked strings as one step that **Undo** takes
  back, then lists what's left.

Text from the game's talk table (a string number with no text of the
module's own) isn't searched: it isn't the module's to change. Scripts
have their own **Find in Files**; tags, variables and the rest are found
by **Advanced Search** (below).

## Advanced Search

**Edit › Advanced Search…** finds the module's blueprints, placed
objects, areas and conversations by what they hold, not by their tag or
name. It opens as a tab of its own, which docks beside the others as
any tab does and keeps what it found while you go to each. Choose what
to look for:
- **Variables:** by name, `*` matching any run of letters (`*Deidad*`
  finds `sDeidad` and `nDeidadBonus`), case ignored; and, if given, by
  type and value.
- **Item properties:** a property (Skill Bonus), and if chosen its
  subtype (Hide), value (+5) and parameter, named as the game names them.
- **Creatures by class, feat, spell or skill:** a class (with the
  level each has), a feat, a spell known, memorized or used as a special
  ability, a skill with at least so many ranks.
- **Appearances:** of creatures, placeables, tileset doors, generic
  doors, waypoints, or items' base types.
- **Scripts attached:** by name, `*` as in variables: objects' and
  areas' events, the module's, conversation lines' conditions and
  actions.
- **Any field:** by label (`*` as in variables), by value, or both. A
  value is text that strings, resrefs and every language of names and
  descriptions contain, or with **Whole number** ticked a number that
  fields of every integer type hold exactly: `103` finds every field
  holding 103, an appearance, a portrait or a conversation's link alike.

**Look at** keeps the search to one kind of object (Items include
those in inventories, equipment and stores) and to blueprints or placed
objects. The lists come from the game's 2DAs and the module's haks; with
no game data loaded, choose rows by number.

What's found is listed by place, as in Find References: `keep ›
creature GUARD_01 › item OLD_SWORD`, `sword.uti`, `keep (area)`, with
what matched beside it (`Skill Bonus: Hide +5`, `sDeidad (string) =
"Ezra"`). Click one to go there. A creature, placeable or store
blueprint's inventory names its items' blueprints: an item blueprint
found is also listed where they carry it (`chest.utp › carries
sword.uti`). The command line has the same: `mg search`
([Command-line tools](11-command-line.md)).

## nasher projects (version control)

A `.mod` is one binary file, so version control (git) can't show what
changed in it or merge two people's work. Many teams keep their module as
a [nasher](https://github.com/squattingmonk/nasher) project instead: a
folder of text files, one per resource (`module.ifo.json`,
`area001.git.json`, `my_script.nss`), described by a `nasher.cfg`.
Moonglow opens such a project and saves into it directly, with no
unpacking or packing between the toolset and git.

- **Open a project:** **File › Open Folder…** and choose the project's
  folder (the one with `nasher.cfg`). Moonglow edits the target that packs
  a module (the default target if it does).
- **Save** writes only the files of the resources you changed, exactly as
  `nasher unpack` would, so `git diff` shows your changes and nothing
  else:
  - new resources go where the project's rules put them;
  - deleted ones lose their files;
  - other files in the folder are left alone.

  A new file the project's sources (`include` in `nasher.cfg`) don't take
  in is written all the same, and the log says so: nasher would not pack
  it, and the project would not read it back when opened again. Add its
  extension to the include (a NUI window's `jui` and `txt`, say).
- **Files the include leaves out are named.** Opening a project, and when
  a file is moved to a folder the include doesn't look in, the log lists
  the files in the source folders that Moonglow would read and the
  include leaves out (a NUI window's `.txt` only beside its `.jui`; what
  an `exclude` leaves out is left be). **Build › Add Left-Out Files to
  nasher.cfg** adds a line for their kinds beside the include they lie
  under (`include = "src/**/*.{jui,txt}"`), and they are read: nasher
  packs them from then on too.
- **Changes made elsewhere are picked up while the project is open.**
  Every few seconds (Options › General, the reloading switch) Moonglow
  looks for source files that another program changed, added or deleted:
  an editor, a `git checkout` that discards a change, a `git pull`. Those
  are read again, the log names them, and open editors show them. A file
  moved within the project is read from where it is now. It is
  not unsaved work. If the undo history had changes to one of them, the
  history is cleared (Undo could not take them back from the new file).
- **A large pull doesn't hold the window.** Moonglow waits for a burst
  of changes to end (a moment with nothing more written), then reads the
  changed files in the background while you go on working; the status
  bar says *Reading files changed outside Moonglow…* while it does, and
  they come into the module all at once when they have been read. On a
  project of 21,000 files, a pull of nearly a thousand is in within a
  few seconds of its end, without a frame of the window over the
  144 fps budget. A file you save while it is being read, or one
  deleted and put back meanwhile, is left for the next look, and one
  written again meanwhile is read again then.
- **Where you have unsaved changes to a file that also changed
  elsewhere**, Moonglow keeps yours and the **Changed Outside Moonglow**
  window asks: **Keep Moonglow's** (the next Save writes yours over the
  file), **Take the Files'** (yours are lost) or **Later**.
- **Changes made elsewhere are never overwritten unasked.** Until you
  answer (and with reloading off), a save that would replace or delete a
  file changed on disk since it was read writes nothing, and the log
  names the files.
- **Areas added on another branch** join the module's area list when you
  open the project.
- **Start a project from any module** with **File › Save As nasher
  Project…**, choosing an empty folder. Moonglow writes a `nasher.cfg` as
  `nasher init` does, with everything under `src/`, its sources taking in
  scripts, GFF files as JSON, NUI windows, and the module's other kinds
  of file (2DAs, pictures).
- **Build › Pack** *file* writes the module the project builds, as `nasher
  pack` does. **Test Module** (F9) packs it into the game's `modules`
  folder and starts the game on it.
- **Compiled scripts are build output.** A project keeps scripts as `.nss`
  only, and Moonglow compiles them when it packs. A compiled script
  without its source can't be kept in a project; Moonglow names any it
  leaves out.
- **Locked objects stay out of the files.** A lock (the area view's
  **Lock**) is a selection aid the game ignores, so a project's area files
  don't carry it, and locking never shows in a diff. Moonglow keeps a
  project's locks in its own data folder (`locks/`, a file a project) and
  puts them back on the objects when the project is opened, and when an
  area changed elsewhere is read again. Each lock finds its object by its
  place in the area's list, its blueprint, tag and position: an object
  moved or renumbered by another program (or `mg`) is found where it can
  be, and loses its lock where it can't. A lock lives on the machine it
  was made on; a teammate's checkout has none. An area file that carries
  the lock already (written by an earlier Moonglow) keeps it on each
  object that stays locked; unlocking that object takes it out of the
  file. A `.mod` and a module folder keep their locks in their files, as
  before.

nasher rounds the module's numbers to 4 decimal places (its
`truncateFloats` setting), so objects in a project may sit up to a
ten-thousandth of a meter from where they were in a `.mod`. In the game
that can show as a turn of up to a degree. Moonglow uses the project's
own settings (`.nasher/user.cfg`): the number of places, the codepage and
whether the area list is kept up to date. It reads projects that keep
resources as JSON, nasher's default; NWNT format isn't supported yet.

`mg init` and `mg build` do the same from a terminal or a build pipeline
(see [Command-line tools](11-command-line.md)).

## Merging in git

Git merges text line by line, and that goes wrong for a module's files.
Two builders who each place an object in one area, or each add an area,
change the same lines of the area's `.git.json` or of
`module.ifo.json`, and git calls that a conflict. A `.mod`'s own GFF
files (`.are`, `.git`, `.utc`…) are binary, and git can't merge them at
all. `mg merge-gff` merges these files by what is in them: object by
object, area by area, field by field. Git can run it as a *merge
driver*: in every merge, on every file you assign it to.

**Set it up once per clone.** Put this in the repository's
`.gitattributes` (it is committed, so every builder has it):

```
# nasher projects: GFF kept as JSON
*.are.json merge=mg-gff
*.git.json merge=mg-gff
*.gic.json merge=mg-gff
*.ifo.json merge=mg-gff
*.ut?.json merge=mg-gff
*.dlg.json merge=mg-gff
*.jrl.json merge=mg-gff
*.fac.json merge=mg-gff
*.itp.json merge=mg-gff
# a module kept as a folder of GFF files
*.are -text merge=mg-gff
*.git -text merge=mg-gff
*.gic -text merge=mg-gff
*.ifo -text merge=mg-gff
*.ut? -text merge=mg-gff
*.dlg -text merge=mg-gff
*.jrl -text merge=mg-gff
*.fac -text merge=mg-gff
*.itp -text merge=mg-gff
```

The driver itself is named in each clone's own settings (git doesn't
take commands from a repository it clones), once:

```sh
git config merge.mg-gff.name "Moonglow GFF merge"
git config merge.mg-gff.driver "mg merge-gff %O %A %B --path %P"
```

(`mg` must be on the PATH, or named with its whole path. `git config
--global` sets it up for all your repositories at once.) Then `git
merge`, `git pull`, `git rebase` and `git cherry-pick` merge these files
with it.

**What it does with each kind of change:**

- A value one side changed is taken. Both changed it the same way: fine.
  Both changed it, each to something else: a **conflict**.
- **An area's placed objects** (creatures, doors, placeables, waypoints,
  triggers, encounters, sounds, stores, items on the ground) are merged
  object by object. Objects added by both sides are all kept. An object
  one side removed and the other left alone is removed. One side removed
  and the other changed it: a conflict. Both edited one object: merged
  field by field. GFF gives an object no id, so an object is recognized
  by these, in order:
  1. it is the same in every field;
  2. the same `ObjectId`, where both have one;
  3. the same blueprint (`TemplateResRef`), tag and position;
  4. the only object left with its blueprint and tag (so a moved object is
     still that object);
  5. the only object left with its blueprint and position (so a retagged
     object is too).

  An object that was moved *and* retagged on one side is taken as
  removed and another added. If the other side changed it, that is a
  conflict, not a lost edit.
- **The module's area list** is a union: the areas the base had, in its
  order, without those a side removed, then ours' new ones, then theirs'.
- **Variables** merge by name and type (an int and a string of one name
  are two variables, as the game keeps them), **haks** and cached scripts
  by name, a creature's feats, classes and spells by what they are,
  equipped items by slot, a journal's quests by tag and their entries by
  number, a faction file's standings by the pair of factions.
- **Every other list** (tiles, skills, inventories, conversation nodes,
  item properties) is merged by place: an item is its number in the list,
  and two items added at one place are a conflict.
- A list only one side changed is that side's list as it is, order and
  all. Where both changed one, the base's order holds; one side's moving
  of an item is not kept.

**The merge is written over your file (`%A`) in its own form.** A GFF is
written as a GFF and JSON as `nasher unpack` writes it: keys sorted,
floats rounded to the project's places, text in its codepage (read from
the nasher project the file is in). A merge that equals your file leaves
it untouched, byte for byte; one that equals theirs is their file.

**On a conflict** the command exits with an error and writes nothing.
It prints each conflict with its path, for example `CONFLICT
/Placeable List[plc_chest "CHEST" at 10, 4.5]/Tag: "CHEST" -> ours
"RED", theirs "BLUE"`. Git then marks the file as unmerged and leaves
your version in it. To settle it, take one side's for every conflict in
that file from git's three copies:

```sh
git show :1:src/area1.git.json > base.json
git show :2:src/area1.git.json > ours.json
git show :3:src/area1.git.json > theirs.json
mg merge-gff base.json ours.json theirs.json --prefer theirs -o src/area1.git.json
git add src/area1.git.json
```

or take one side's file whole (`git checkout --ours FILE` or `--theirs`),
or open the module in Moonglow and make the change there.

The command needs no game install, and `--json` prints the conflicts for
a script or a bot. NWNT files (nasher's other text form) aren't merged
yet.

## The project file (moonglow.toml)

A team's rules for Moonglow are kept with its module, in version
control: a `moonglow.toml` beside `nasher.cfg` in a nasher project, or
at the root of a module folder. Moonglow reads it when the module opens,
and again when it changes (with reloading on, Options › General), and
the log says so. A module kept in one file (`.mod`) has none: its folder
holds other modules too. Without the file, Moonglow does what it does
for any module.

```toml
[make_static]                       # Make Placeables Static leaves dynamic:
keep_variables = ["tdod_*"]         # a placeable with a variable so named
keep_tags = ["LAMP_*", "TORCH"]     # a placeable whose tag matches
keep_resrefs = ["tdod_*"]           # one placed from a blueprint so named

[tags]
unique = ["TDOD_START_*", "KZ_*"]   # on one object of the module only
unique_in_area = ["*_STORE_*"]      # on one object of each area only
paste_warning = "custom"            # which duplicates a paste says it made

[store]                             # Setup Store
open_script = "tdod_vendor_open"    # the reply that opens the store runs it
tag = "{area}_STORE_{n}"            # the store's tag
blueprint = "tdod_vendor"           # the store blueprint chosen at first
keeper_variable = "tdod_store"      # the shopkeeper's variable: the store's tag

[resrefs]
pattern = "{code}_{name}"           # a new ResRef, by each builder's code

[claims]
file = "builders/claims.2da"        # who is building which area
others = "warn"                     # another builder's area: "warn" before a save, or "read-only"
unclaimed = "edit"                  # an area no one claimed: "edit", or "warn"
protected = ["kz_*"]                # areas whose Tag and ResRef are kept, besides those merged

[commands]                          # File › Project
update = "bin/update"
submit = { run = "bin/submit", windows = "bin\\submit.cmd", label = "Submit for Review" }

[plugins]
folder = "plugins"                  # the project's own plugins (as without it)

[plugins."tdod.rules"]              # a plugin's settings the team shares
claims = "data/claims.toml"
```

Names are matched by globs, whatever their case: `*` any characters,
`?` one, `[abc]` one of them, `{a,b}` either.

| Key | What it does | Without it |
| --- | --- | --- |
| `[make_static] keep_variables` | a placeable with a variable these name stays dynamic; `[]`: variables don't keep one | any variable keeps it |
| `[make_static] keep_tags` | a placeable whose tag these name stays dynamic | none |
| `[make_static] keep_resrefs` | a placeable placed from a blueprint these name (its `TemplateResRef`) stays dynamic | none |
| `[tags] unique` | tags that must be on one area or placed object only: Verify Module warns of two (`tag-unique`) | none |
| `[tags] unique_in_area` | tags that must be on one object of each area only (an area's own tag counts as in it), as a store's tag is when a script looks for the nearest of it: another area may have one too. Verify Module warns of two in one area (`tag-unique`), and a paste counts them in the area it pasted into. A tag these name is held to them alone, though a glob of `unique` names it too (`unique = ["KZ_*"]` with `unique_in_area = ["*_STORE_*"]`: `KZ_SQUARE_STORE_1` once in each area). Since Moonglow 2.2.0 | none |
| `[tags] paste_warning` | which tags a paste or a copy says it put on more than one object: `"custom"` (those a builder set, not the object's blueprint's own tag, and those of `unique` and `unique_in_area`), `"any"`, `"unique"` (those of `unique` and `unique_in_area` only) or `"off"`; never a static placeable's, and one of `unique_in_area` only when it is twice in one area | `"custom"` |
| `[store] open_script` | the script the shopkeeper's reply runs to open the store; none is written | an `openstoreNNN` that calls `OpenStore` |
| `[store] tag` | the store's tag: `{area}` the area's tag, `{n}` the first number no object has | the blueprint's tag, with a number where another object has it (with a `[store]` section; Aurora's, the blueprint's, without one) |
| `[store] blueprint` | the store blueprint chosen at first | none chosen |
| `[store] keeper_variable` | a String variable set on the shopkeeper to the store's tag | none |
| `[resrefs] pattern` | how a builder's code goes into the ResRefs Moonglow proposes: `{code}` and `{name}` once each, the rest letters, digits and `_` (`"{name}_{code}"`, `"t{code}{name}"`) | `"{code}_{name}"`, where a builder has a code |
| `[claims] file` | the team's [claims table](#claimed-areas), from the project's root | no claims |
| `[claims] others` | an area another builder claimed: `"warn"` (it says so, and a save asks before it writes a change to it) or `"read-only"` (no change to it is taken) | `"warn"` |
| `[claims] unclaimed` | an area no one claimed: `"edit"` (as without a table) or `"warn"` (as another's) | `"edit"` |
| `[claims] protected` | areas (ResRefs, globs) whose Tag and ResRef are [kept](#protected-areas), besides those the table says are merged | none |
| `[commands]` | the project's [commands](#the-projects-commands), each by its name (letters, digits, `_`, `-`): its text, or `{ run = "…", windows = "…", unix = "…", label = "…" }` (`windows` and `unix` in place of `run` on those systems; `label` its name in the menu) | none |
| `[plugins] folder` | the folder of the project's own plugins, from the project's root, inside it ([A project's own plugins](15-plugins.md#a-projects-own-plugins)) | `plugins` |
| `[plugins."<plugin id>"]` | the values of a plugin's settings of scope `project`, by setting (the id in quotes: it has dots in it). The Plugins window's Settings… and the plugin itself write them here, the rest of the file as written | the settings' defaults |

See [Make Placeables Static](04-areas.md#static-placeables-many-at-once),
[Duplicate tags](04-areas.md#duplicate-tags), [Setup
Store](04-areas.md#setup-store), [claimed areas](#claimed-areas) and
[the project's commands](#the-projects-commands).

**What the file doesn't understand** is said in the log (and by `mg`) by
file and line, and the rest is read: a key or a section it doesn't know,
a value of the wrong kind (`moonglow.toml:12: "colour" is not a key of
[store]`). A file that is no TOML at all is an error, by line, and its
rules are left out until it is mended.

**Each builder's own settings** go in `.moonglow/user.toml` in the same
folder, never in the file the team shares. Moonglow makes the folder
with a `.gitignore` in it holding `*`, so that it stays out of version
control without a line in the project's own (the log warns of a
`.moonglow` made by hand without one). It holds `[builder] code`, your
builder code (below), and the values of plugins' settings of scope
`user-project` (`[plugins."<plugin id>"]`, as in the team's file). A
team's section put there, or `[builder]` put in the team's file, is said
and left out. Which of the project's commands you allowed to run is
never kept in the project's files ([below](#the-projects-commands)).

```toml
# .moonglow/user.toml
[plugins."tdod.rules"]
code = "ab"
```

**Which of the project's own plugins may run is never in its files.**
A project's files come with its checkout, and a repository could carry
answers of its own, its plugins "allowed" before anyone was asked. What
you answer is kept in Moonglow's own folder
([A project's own plugins](15-plugins.md#a-projects-own-plugins)); a
`[project_plugins]` section in either file is said in the log, and
nothing in it allows anything. No setting allows anything either.

### Your builder code

Two builders whose wizards both propose `merchants001` make one file of
two in version control. With a builder code, the ResRefs Moonglow
proposes for new resources begin with yours: `ab_merchants001` for one
builder, `cd_merchants001` for the other. **Edit › Builder Code…** sets
it (letters, digits and `_`, at most 8, kept in lower case), shows what
it makes of a name, and writes it into `.moonglow/user.toml`, keeping
the rest of that file as it is written; **No Code** takes it out. It
is there for a module folder or a nasher project, not a module kept in
one file (`.mod`). From a terminal, `mg builder-code PROJECT CODE` does
the same ([Command-line tools](11-command-line.md)).

```toml
# .moonglow/user.toml
[builder]
code = "ab"
```

Where a name would make the ResRef too long, the name is cut and the
code kept (16 characters: `ab_areatransi001`). A name that begins with
your code already doesn't get it twice: a copy of `ab_chicken001` is
`ab_chicken002`; another builder's code is kept as part of the name. The
team's `[resrefs] pattern` says where the code goes. Without a code,
ResRefs are made from names alone, as Aurora makes them. The code is
used by:

- the Area Wizard (which then shows the ResRef under the name), the
  Creature Wizard and every blueprint wizard (the ResRef field's hint);
- **Add to Palette** (`ab_wswls003` for a placed `nw_wswls001`);
- **Setup Store**'s conversation and script (`ab_store001`,
  `ab_openstore001`);
- **Copy…** and **Edit Copy…** (`ab_nw_chicken001`);
- the Script Wizard's names (`ab_sc_001`, `ab_at_001`).

A name typed in a wizard's ResRef field, or in **New Script…** and the
other windows that start with an empty name, is taken as typed.

### Described variables

A module's scripts read local variables by name and type, and a wrong
one fails without a word: a variable read as another type is that
type's empty value (an Int read as a String is `""`, a String read as
an Int is 0), and a name is matched in its case only (`tdod_lit` is not
`TDOD_LIT`), as the game's server shows. A team describes its
variables in `variables.toml`, beside its `moonglow.toml` (or alone),
as a table's columns are described beside it: each variable's type, the
values it takes, the objects it goes on and a line of help.

```toml
undescribed = ["tdod_*"]        # a variable so named, described nowhere here, is said

[tdod_workbench]
type = "string"
on = ["placeable"]
values = { forge = "Smithing", tanner = "Leatherworking", loom = "Tailoring", carpenter = "Woodworking", machinist = "Machining", jeweler = "Jewelrymaking", alchemist = "Alchemy", enchanter = "Enchanting", herbalist = "Herblore", fire = "a kitchen, for Cooking", water = "a fishing spot, for Fishing" }
help = "The craft the bench serves; a player works within 5 m of it"

[tdod_harvest]
type = "string"
on = ["placeable"]
values = { herb = "Herblore", ore = "Smithing", timber = "Woodworking" }
help = "What the node yields, once a rest to each character"

[tdod_harvest_dc]
type = "int"
on = ["placeable"]
min = 1
max = 60
help = "The Survival check's DC (15 without it)"

[tdod_light]
type = "int"
on = ["placeable"]
values = { 1 = "lit" }
help = "A natural light against delirium, within 3.33 m"

[tdod_crier]
type = "int"
on = ["creature", "placeable"]
values = { 1 = "a crier" }
help = "Players within 5 m say !cry MESSAGE"

[tdod_settlement]
type = "string"
on = ["area"]
help = "The settlement's key: every street of a town has the same"

[tdod_rental_term]
type = "string"
on = ["creature", "placeable"]
values = { long = "30 days, paid ahead to 90", short = "7 days" }

[tdod_kind]
type = "string"
on = ["creature"]
values = ["aberration", "beast", "construct", "fiend", "fey", "flora", "humanoid", "ooze", "shapechanger", "undead", "vermin"]
help = "What the creature is, where its race misleads"

["tdod_rental_*"]               # every variable so named that isn't described above
type = "string"
on = ["creature", "placeable"]

["tdod_*_d20"]                  # the scripts' own: never kept in a file
type = "int"
runtime = true
help = "A test's hook: the next d20 rolled (the tests set it on the module)"
```

| Key | What it says | Without it |
| --- | --- | --- |
| `[name]` | a variable of that name (in its case); a glob (`["tdod_rental_*"]`, in quotes) describes every variable it names that has no description of its own | |
| `type` | `"int"`, `"float"`, `"string"`, `"object"` or `"location"` | any type (said in the log) |
| `on` | what it goes on: `module`, `area`, `creature`, `door`, `encounter`, `item`, `placeable`, `sound`, `store`, `trigger`, `waypoint` (blueprints, placed objects and the items they hold alike) | anything |
| `values` | the values it takes: `["herb", "ore"]`, or each with what it means, `{ forge = "Smithing" }` | any |
| `constants`, `prefix` | its values are a script's constants whose names begin so (`constants = "tdod_kind_inc"`, `prefix = "TDOD_KIND_"`): the project's script of that name (any folder of it), the module's, else the game's | |
| `table`, `column` | its values are a table's rows: an Int's is a row's number (named by its `Label`), a String's a cell of `column` | |
| `other = true` | a value not listed is right too (the list is a help) | only those listed |
| `min`, `max` | an Int's or a Float's range | any |
| `help` | a line of help, shown with it | |
| `runtime = true` | the scripts set it while the server runs (their state, a test's hook), and it is never kept in a file: no `undescribed` says it, the Variables window doesn't offer it, and Verify says where one is kept (since Moonglow 2.2.0) | kept in the module's files |
| `undescribed` (at the top) | names (globs) that are the project's: a variable so named and described nowhere is said, with the described name nearest it (a typo) | not said |

What the file doesn't understand is said in the log by line, as for
`moonglow.toml`, and the rest is read; it is read again when it
changes.

**In the Variables window** (an object's, a blueprint's, an area's, the
module's), the name being typed offers the described variables that go
on that object, each with its type and what it goes on; one chosen (or
typed whole) takes its type, a value is chosen from its values, and its
help is the name's tip. Under the list, the window says what the
description finds wrong (another type, a value not listed, the wrong
kind of object, a name in another case or like the project's and
described nowhere, one only the scripts set); OK writes it all the
same, as you have it.

**Verify Module** (and `mg verify`) holds every variable of the module
to the descriptions:

| Check | Finds | |
| --- | --- | --- |
| `var-type` | a described variable of another type: the scripts read that type's empty value | an error: Verify fails |
| `var-value` | a value none of its values (a value in another case is named: the scripts compare as written), out of its range, or no row of its table | a warning |
| `var-object` | a variable on another kind of object than it says | a warning |
| `var-name` | a described name in another case, and (with `undescribed`) a variable named as the project's that is described nowhere | a warning |
| `var-runtime` | a variable described as the scripts' own (`runtime = true`) kept in the module: there from the start, on every server (a test's hook set so fixes every roll) | an error: Verify fails |

`mg vars MODULE` lists the descriptions, with the values a script's
constants and a table's rows give (`--on placeable`: those that go on
one); `mg areas --var NAME=VALUE` gives a described variable its
described type ([Command-line tools](11-command-line.md)).

### Claimed areas

Two builders who change one area at once make the one conflict version
control can't merge for them. A team that keeps its module in git can
keep a **claims table**, a file of the checkout that says who is building
which area, and name it in `moonglow.toml` (`[claims] file`). It is a
2DA, so the team's checks can read it too:

```text
2DA V2.0

    Area             Builder  Status   Date        Tag
0   kz_square        ab       claimed  2026-10-22  ****
1   kz_abbey_gate    ****     merged   2026-10-01  KZ_ABBEY_GATE
2   kz_bridge        cd       merged   2026-10-20  KZ_BRIDGE
```

| Column | What it says |
| --- | --- |
| `Area` | the area's ResRef |
| `Builder` | the [builder code](#your-builder-code) of the one who claimed it; `****`: no one |
| `Status` | `merged` once the area is in the game (its Tag and ResRef are then [kept](#protected-areas)); `claimed` or `free` otherwise |
| `Date` | when it was last claimed or released |
| `Tag` | the Tag it was merged with ([Verify](#protected-areas) holds it to it) |

`Area` and `Builder` are needed; the others are read where the table has
them, and a column of the team's own (a `World` marking the areas of the
game world) is kept as it is. A row the table can't read is said in the
log by line, as the project file's are.

**In the window**, with your code set (**Edit › Builder Code…**):

- the module tree marks each claimed area: ✔ and your code for yours,
  🔒 and the other's code for another's (in the warning color), 📌 for a
  merged one; on hover, who claimed it and since when;
- an area another builder claimed says so above its view and its
  Properties, as does an unclaimed one in a project that warns of those
  (`unclaimed = "warn"`), with **Claim This Area** beside it;
- where the project keeps others' areas read-only (`others =
  "read-only"`), no change to one is taken (from any window, a rename
  elsewhere that would change it, or a plugin): the log says why;
- where it warns (the default), you may change it, and a save that
  would write a change to it asks first (**Areas Not Yours**: **Save
  Anyway**, or Cancel to leave it unsaved), as does one of an unclaimed
  area in a project that warns of those.

**Claim This Area** and **Release Claim**, in the area's menu in the
module tree and on its tab, write your row into the table (made, with
its folder, if it isn't there yet): one line changed or added at the
end, every other line as it was, so that two builders claiming two
areas at once make changes git merges by itself. Commit the table with
your work so that the others see it. An area another builder claimed is
theirs to release. Without a builder code, Claim is there and off.

`mg claims PROJECT` lists every area of the table and the module and
whose each is; `mg claims PROJECT claim AREA…` and `release AREA…` do
what the menus do (`--force` takes or releases another's claim, for the
one who keeps the table), and `merged AREA…` marks areas merged, with
their Tags as the module has them ([Command-line tools](11-command-line.md)).

### Protected areas

The game keeps a character's position by its area's Tag, and its first
visits by the area's ResRef. Once an area is in the game, a new Tag or
ResRef loses both for everyone who has been there. An area the claims
table says is merged, or one `[claims] protected` names, is protected:
before its Tag changes (Area Properties, or any other window), before it
is renamed (**Rename…**) or deleted, Moonglow asks (**Protected Area**),
and offers **Make a Copy Instead…**: a copy under a new ResRef and Tag is
a new area, and loses nothing. **Change It Anyway** (or Rename, or
Delete) goes ahead.

**Verify Module** (and `mg verify`) holds the merged areas to the table:

| Check | Finds | |
| --- | --- | --- |
| `claims-merged` | an area the table says is merged that the module doesn't have (renamed or deleted) | an error: Verify fails |
| `claims-tag` | a merged area whose Tag is not the one the table records | an error: Verify fails |

### The project's commands

A team whose module is a git checkout keeps its own scripts for bringing
in the others' work and handing its own back (an update and a submit,
which commit, pull, merge and push). `moonglow.toml` names them in
`[commands]`, and **File › Project** runs them, in place of closing
Moonglow to run them in a terminal:

```toml
[commands]
update = "bin/update"
submit = { run = "bin/submit", windows = "bin\\submit.cmd", label = "Submit for Review" }
```

Choosing one:

1. **Saves the module first**, asking as on closing (Save, Don't Save,
   Cancel), and about [areas not yours](#claimed-areas) as any save does.
2. **Asks you to allow it**, the first time and whenever its text
   changes: **Run a Project Command?** shows what it runs and where. A
   project's files can name any program, and it runs as you, with
   everything you may do: allow it only if you trust where the project
   came from. What you allowed is kept in Moonglow's own settings (in
   your configuration folder), each command by the project folder's
   full path, its name and its exact text: never in the project's
   files, which a repository could carry. An `[allowed_commands]` found
   in `moonglow.toml` or `.moonglow/user.toml` is not read (the log
   says so). The same project in another folder asks again. A command
   never runs when a module opens.
3. **Lets the project's files go**: it stops looking at them for changes
   made outside, and closes the module's haks, so that the command may
   write over any of them (Windows refuses to replace a file another
   program has open). Moonglow keeps no other file of the project open.
4. **Runs it** in the project's folder, by the system's shell (`sh -c` on
   Linux and macOS, `cmd /C` on Windows), with `MOONGLOW_PROJECT` (the
   folder) and `MOONGLOW_BUILDER` (your code, where you have one) set.
   Its output comes into the log as it is written (its errors in the
   warning color), its last line in the window that shows while it
   runs; **Cancel** stops it. The log says how it ended: done, or its
   exit code.
5. **Reads the project's files again**: the haks are opened again, and
   what the command changed is read as any [change made
   outside](#nasher-projects-version-control) is (a file you had changed too waits in
   **Changed Outside Moonglow**), with the project file and the claims
   table.

The menu is there and off for a module without commands (a `.mod`, or a
project whose `moonglow.toml` names none); a command without a text for
this system is there and off.

## Recovering unsaved work

While a module has unsaved changes, Moonglow writes a recovery copy of it
every 5 minutes (Options › General sets how often, or turns it off). The
copies live in Moonglow's own data folder, never next to the module or in
the game's folders, and are removed when you save or discard the changes.
If Moonglow (or the computer) stops first, the next start offers the copy
in **Recover Unsaved Work**: open it as an unsaved module, then save it
where you want. As in Aurora, open conversations are also backed up as
`<name>.bak` every 5 minutes.

## Module Properties

**Edit › Module Properties** (or the top of the module tree), with
Aurora's pages:

- **Basic**: name, tag, and the start area and location (the start
  location marker in the area viewer moves it).
- **Events**: the module's event scripts, each with Edit to open it.
- **Advanced**: the starting date and hour, minutes per hour, dawn and
  dusk hours, the experience scale, the starting movie, Variables.
- **Description**: the module's description.
- **Custom Content**: the haks the module uses, highest priority first
  (where two have the same resource, the higher one wins; changes apply
  when the module is reopened), and its custom talk table.

Text fields players see (names, descriptions) are localized strings. The
field shows the language chosen in Options › Language. The **…** button
opens **String Edit**, where each language and gender has its own text, or
a talk-table reference.

## Haks and custom talk tables

Haks listed in Custom Content are searched before the game's own files, in
their order, as the game does: their tilesets, models, 2DA tables and
blueprints appear in the area viewer, the palettes and the editors. Haks
are looked for in the user folder's `hak`, then the game's `data/hk`. A
change to the list (adding, removing, reordering, an undo) takes effect at
once.

**A language the game has no letters for.** A hak can have an
`encoding.2da`: a table of the Unicode character each of the 256 bytes of
the game's text stands for, shipped with fonts drawn to match (a Turkish
module gives the bytes of "ð" and "þ" to "ğ" and "ş"). Moonglow reads the
table as the game does and shows and writes the module's text by it:
names, tags, descriptions, conversations, the journal, scripts and the
talk table show the table's letters, and a letter you type is saved as
its byte. A letter the table has no byte for is a "?" (or is refused,
where a field says so). The table must be in a hak on the list: the game
does not read one kept in the module itself. A module without one is
read as ever (Windows-1252; Polish text Windows-1250). Plugins' text
follows the table too, and so does the command line where it is given a
module (`mg set`, `apply`, `find`, `info`, `areas`, `replace`, `dialog-export`, `dialog-import`):
it needs the game install to find the hak. `mg gff` on a file of its own
is Windows-1252.

**The hak list** (Custom Content) has the module's haks numbered from the
top: where two haks have a resource of the same name, the game takes the
one from the hak higher in the list. A hak that is in none of the hak
folders is marked **not found**: the game will not load the module until
it is there, or taken off the list.
- **Choose** a row with a click; Ctrl + click adds or takes out a row,
  Shift + click chooses from the last one clicked to this one.
- **Move** what is chosen by dragging a row (by its name or the dots
  before it) to where the line shows, with the arrows under the list, or
  with Alt + Up and Alt + Down. Rows chosen together move together.
- **Remove** takes the rows chosen off the list; so does Delete.
- **The keys** work after a click in the list, with the pointer over it:
  Up and Down choose the row above or below (with Shift, more), Home and
  End the first and the last.
- Each move or removal is one step to undo.

**Add Haks and Talk Table…** (Custom Content) attaches downloaded content
in one step. Choose the haks and a talk table wherever they are, and put
the haks in order (highest priority first). Moonglow copies them into the
user folder's `hak` and `tlk` (asking before replacing different files of
the same names), lists the haks at the top of the module's list and names
the talk table. Undo takes back the module's changes; the copies stay.
`mg attach` does the same from the command line.

A custom talk table (`.tlk`) gives the strings numbered from 16,777,216
up. The game looks for it in the module's haks, then the module, then the
user folder's `tlk` (Moonglow also looks in the game's `data/tlk`, where
the premium campaigns keep theirs). Its feminine table, `<name>f.tlk`, is
looked for the same way. If the game can't find the table the module
names, it won't load the module; Verify reports it. Name it without
`.tlk`, in lower case: on Linux the file's name must match exactly.

### What the module uses of its haks

`mg haks MODULE --usage` says, hak by hak, which of its resources the
module reaches and which nothing does: for cutting a release (or a
player's download) to what is used. A line a hak, and one for them all:

```
tdod_p_n2e	129 of 4882 used (80.4 MB of 650.1 MB); 4753 unused, 0 shadowed, 0 not judged
```

With `--json`, every resource used with the module resource whose walk
reached it first (`from`), and the lists of those unused, shadowed and
not judged. The walk starts at every resource of the module and follows
what each names, wherever in the load order that is (a hak's blueprint,
the game's script), and what that names in turn:

- **References**, as Verify follows them: scripts (the compiled script
  and its source), conversations, blueprints (a placed object's too),
  sounds, tilesets, areas, portraits (every size), and a script's
  `#include`s.
- **A script's strings**: a string that is the name of a script,
  blueprint, conversation, sound, music, 2DA, model or texture reaches
  it. What the string is passed to isn't read, so a string that happens
  to spell a name counts.
- **An area**: its tileset (the `.set` and its palette), and of the
  tiles, those the area places, each with its model, walkmesh and map
  image; the tileset's grass and environment map; the skybox's models,
  the loading screen, the ambient sounds and the music (with its
  stingers).
- **An object, as the area view draws it**: a placeable's or door's
  model and sounds (placeableobjsnds.2da); a creature's body (head, body
  parts, wings, tail, what it wears), its portrait and sound set (the
  `.ssf` and its sounds); an item's models and their inventory icons;
  armor's parts and cloaks for every gender, part-based race and
  phenotype that has a model of them, as anyone may wear them.
- **A model's own files**: walkmeshes, textures (the DDS or TGA the game
  picks, and a PLT), materials and TXIs and what they name, its
  emitters' models and its supermodels.

Each hak resource is then **used** (reached, and this hak's copy is the
one the game loads), **shadowed** (a hak higher in the list, or the
module, has it: this copy is never loaded), **not judged** or
**unused**. Not judged are what the game reads by itself (2DAs other
than those the walk reads, GUI, fonts, shaders a material doesn't name,
palettes, NUI windows) and a hak's replacements of the game's own
resources, which the game's own content may use. Not followed at all:
what scripts reach by a number (visual effects, spells, item
properties, appearances set by a script) and what players choose
(heads, portraits, sound sets and parts at character creation): a hak of player heads can
show as unused and still be needed. The game is taken as a player has
it: the user's `override` and `development` folders are left out, the
haks are found where the game finds them. On The Domains of Dread's
module and 143 haks (29.5 GB) it takes about 5 seconds.

### The talk table editor

**Tools › Talk Table** (or **Edit…** beside Custom TLK) edits the
module's talk table:
- **The lines** are listed by the StrRef the game knows them by
  (16777216 and up), with the feminine text beside them when there's a
  feminine table. **Find** finds lines by words or by StrRef. **Go to**
  takes a StrRef or a line's number: press Enter and the list goes to
  that line, among all the others, and it is chosen. **Find References**
  lists where the module names the chosen line (a hak's 2DAs are not
  looked in).
- **Choosing a line:** click anywhere on its row. The keys are the
  list's from then: Up and Down (Page Up and Page Down, Home and End)
  choose a line, and Enter, or a double click, goes to its text.
- **A line**: choose it to edit its text, its feminine text, and the
  sound spoken with it and its length. **Copy** copies its StrRef, to put
  in a 2DA or a script. The text boxes take the room under the list; a
  text longer than its box scrolls in it, and the sound's row stays in
  sight. A larger window gives them more.
- **Add Line** adds one at the end. **Remove Last Line** removes only the
  last, since removing another would renumber the lines after it.
- **Undo** and **Redo** in its toolbar undo the table's changes, as do
  Ctrl+Z and Ctrl+Y with the pointer over the editor. (The table is a
  file of its own: elsewhere, and in the main toolbar, Undo is the
  module's.)
- **Export CSV…** writes the lines to a file for a spreadsheet or a
  translator: each line's StrRef, text (and feminine text), sound and
  sound length. **Import CSV…** reads such a file back: each row sets
  the line of its StrRef, which keeps what the file has no column for;
  rows past the table's end add lines. Nothing changes unless every row
  can be read, and one Undo takes the import back.
- **Export JSON…** and **Import JSON…** do the same with the JSON that
  neverwinter.nim's `nwn_tlk` writes and a nasher repository keeps: the
  table's language and the lines that have text, each with its `id` (the
  line's number in the table). An import sets the lines the file has and
  leaves the others; the feminine table is a file of its own there and
  is not touched. (A line's sound length is `soundLen`, as `nwn_tlk` has
  it; files written before 1.20.5 named it `soundLength` and are still
  read.) From a terminal: `mg tlk-export` and `mg tlk-import`.
- **Only lines with text** leaves the empty lines out of the list, for a
  table with reserved ranges. The selected line stays selected: switch it
  off again and the list is at that line, with the empty ones after it
  to fill.
- **Open File…** edits a `.tlk` file anywhere (with its feminine table,
  if a file named with an `f` after its name is beside it) and **New
  File…** makes an empty one: with or without a module open. **Module's
  Table** goes back to the table the module names.
- **Saving**: **Save** saves the table, as does saving the module.

With no talk table, the editor makes one in the user folder's `tlk` (with
a feminine table if you ask) and names it in Module Properties. A table in
a hak or in the module is shown read-only: change it where it is built.

**Move to Talk Table** in the String Edit window (the **…** beside a name
or description) adds the text to the talk table as a new line and puts its
StrRef in the string in place of the text. That's for translations, and
for text a server sends many times.

**Resources in haks**: when the module and a hak have the same resource,
the hak's wins in the game. Moonglow warns in the log when you add such a
resource, or one that replaces the game's own (Options › General). The same
warnings come however the resource arrives: made in an editor, imported,
copied from the palette or renamed. An import that brings many says the
first few and how many more.

## The hak editor

**Tools › Haks** opens a hak: **New Hak**, **Open Hak…** (a `.hak` or an
`.erf`), or **Build Hak from Folder…**, which makes a new hak from a
folder's files, subfolders included, to look over and save.
- **The list:** every resource with its size, and for files just added,
  where they come from. **Find** narrows it by name or type. Click to
  select, Ctrl+click to add to the selection, Shift+click to select from
  the one clicked last to this one; right-click for **View**,
  **Rename…**, **Extract…** and **Remove**. The list is by name, by type
  or by size (the largest first).
- **Viewing:** **View**, or a double click, shows a resource under the
  list: a GFF's fields, a text file's text, a compiled model as the
  text it compiles from, else its first bytes.
- **Compile Models** compiles the hak's models kept as text (ASCII),
  each against its supermodel from this hak, the open module's haks or
  the game, as one step to undo (a hak of many models takes a while:
  the window shows how far it is, and Cancel leaves the hak as it was). A model that doesn't compile, has
  errors Verify Module would name, or whose supermodel isn't found stays
  as text, and the log says why. (`mg pack --compile-models` does the
  same for a folder.) The game reads models as text too: compiling
  makes them smaller and quicker to load.
- **Update from Folder:** a hak built with Build Hak from Folder
  remembers the folder (from one session to the next, once saved):
  **Update from Folder** takes the folder's files again in place of the
  hak's, as they are now.
- **Adding:** **Add Files…** and **Add Folder…** add files. Where the hak
  already has some of them, it asks first, listing them, as Aurora's hak
  editor does: **Replace** them, **Skip Those** and add the rest, or
  **Cancel**. A file the game couldn't read by its name
  is left out, and the log says why: a name over 16 characters (other hak
  tools silently cut it short), or a type the game doesn't know. Hidden
  files, `Thumbs.db` and `desktop.ini` are skipped.
- **Extracting:** **Extract…** and **Extract All…** write resources as
  files into a folder.
- **Undo** and **Redo** in its toolbar, and Undo's and Redo's keys
  with the pointer over the editor (the hak's steps, not the module's).
  **Description** is the hak's own note, for people.
- **Saving:** **Save** writes the hak beside the old one, one resource at
  a time, then moves it over the old one. A hak of gigabytes needn't fit
  in memory, and a failed save leaves the old one whole. It warns when
  resources would start past 2 GiB, which the game can't read. Saving the
  module saves open haks too.
- **Reloading:** saving a hak the module uses reads it again.
- **The game's own haks** (in its `data/hk`) aren't written: **Save As…**
  keeps your changes in a copy.

Closing a hak with unsaved changes asks first: **Save**, **Don't
Save** or **Cancel**.

## The 2DA editor

A 2DA is edited where it lives, and the game's own tables are never
changed:

- **A table of one of the module's haks:** open the table in the
  resource browser (Tools › Resource Browser, and its name) and choose
  **Edit… › The Table of *name*.hak**: the hak opens in the hak editor
  and its table in the 2DA editor. (Or, with the hak open in the hak
  editor already, right-click the `.2da` there and choose **Edit
  Table**.) Save puts the table into the hak (a step of the hak's own
  Undo); the hak's own **Save** writes the file, and **Tools › Reload
  Resources** shows the module the change. The game's own haks are not
  offered.
- **The module's table, or a copy of any table the game reads:**
  **Edit…** in the resource browser also offers the module's own table
  as it is, or a copy in the module, in a hak open in the hak editor, or
  in a file. A copy is not there until it is saved.
- **A project's or a folder's tables, and a file anywhere:** **Tools ›
  2DA File…** lists the module's own 2DAs and every `.2da` file of its
  nasher project or module folder, whatever folder each is in (a hak's
  sources kept in the repository too), with a name to find them by;
  Enter opens the first listed. **Browse…** there takes a file anywhere
  else (an `override` or `development` folder); with no module open, it
  is asked for at once.

The table is a grid: the column names stay at the top and the rows'
numbers at the left as it scrolls.

- **A click** selects a cell; a double click, Enter or F2 types into it
  (Enter keeps what was typed and goes to the row below, Escape leaves
  the cell as it was); Delete empties it (`****`). The arrow keys, Tab,
  Page Up and Down, Home and End move from cell to cell.
- **Several cells** are selected as a rectangle: Shift and the arrow
  keys, Shift and a click, or a drag over the cells. Delete empties them
  all in one step.
- **Ctrl+C** copies what is selected, **Ctrl+X** empties it too, and
  **Ctrl+V** pastes cells from the selection's first cell on: text with
  a line for each row and tabs between cells, as a spreadsheet copies
  them, so rows go from one table to another (or to and from a
  spreadsheet). Rows are added at the end where the paste runs past it;
  where the table has fewer columns than what is pasted, the first ones
  that fit are taken.
- **Add Row** puts an empty row at the end, or as many as the number
  beside it says (padding, up to the row your own begin at), in one
  step. No row is put between others.
- **Empty Row** empties the selected row, which stays: the rows after it
  keep their numbers. **Delete Row** takes it away, and every row after
  it is then one number lower. A row's number is what other tables,
  blueprints, areas and scripts know it by, and nothing is changed to
  follow: delete a row others come after only in a table whose rows
  nothing names yet (**Find References to Row** shows the tables that
  do), and empty it otherwise.
- **A column's width** is set by dragging the edge at the right of its
  name (the pointer turns into a double arrow over it); a double click
  there makes the column as wide as its widest cell, as a spreadsheet
  does. Widths are kept for the table's name, between runs too: another
  copy of `appearance.2da`, and the game's own read in the resource
  browser, show the columns as wide.
- **Add Column…** adds one at the end. A column is moved by dragging
  its name onto another column's (a line shows where it will go), or by
  **Move Left** and **Move Right** in the menu a right click on its name
  opens; its cells go with it. The same menu renames the column or
  removes it, with its cells (Undo puts it back where it was).
- **Undo** and **Redo** in the editor's own row are the table's steps,
  and so are Undo's and Redo's keys (Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z, or
  those set in Options › Keyboard) with the pointer over the editor or
  the keys in its grid (after a click on a cell): there they take back a
  paste or a typed cell, not the module's last change.
- **Find** narrows the rows to those with the text in any cell (Ctrl+F
  goes to it, Escape clears it); **Go to** takes a row's number.

**Cells are checked as you type**, as Verify Module checks a hak's
tables: a cell that names a row of another table which isn't there (a
feat's required feat, a spell's feat, a package's class, and the like),
or a talk-table string that isn't, is outlined in red and says why
under the pointer; the count above the grid goes to each in turn. Under
the grid, the selected cell is shown by what its column holds: a
talk-table string as its text (**Strings as text** shows them so in the
grid too), a row of another table as that row's name, with a list to
choose another from.

**Find References to Row** lists the cells, in every table the game
reads, of the columns known to name a row of this table: who would be
left pointing at nothing if the row were emptied.

**Take Rows…** is for a table that hides another copy of itself (your
hak's `appearance.2da` over another hak's, or over the game's). It lists
the rows the other copy has and yours lacks, and those it has
differently; against a copy that has one below it, it tells the rows
only the other changed from those you both changed. Tick rows one by
one, or all of a kind with the **New** (those yours lacks) and
**Differ** (those it has differently) boxes; **Clear Selections**
unticks them all. **Take** brings each ticked row in whole, as one
step.

Saving lays the table out as the game's own files are (columns in line,
rows numbered from 0), or, with **Compact** ticked, with one space
between cells: a smaller file with less for the game to pass over as it
reads, where columns in line are easier to read and to copy from. The
box begins as the file was (one space or one tab between its cells:
compact), and ticking it is a change to save. An empty row at the very
end is not written: a table has no empty rows after its last.

Under the table are **OK** and **Cancel**: OK saves the table and closes
its window; Cancel closes it without saving, asking first where there
are unsaved changes (**Save**, **Don't Save** or **Cancel**), as the
window's × does (a table's changes are its own, and the module's Undo
does not bring them back).

### A table's description

A table of your own (a spawn table, a treasure table, a table an AI
reads) can say what its columns hold, in a file of text beside it:
`loot.2da.toml` beside `loot.2da`. The table stays what it is: a cell
keeps its value, and the description gives the value its name, a list
to choose it from, and a check.

![A described table: values by name, cells that are wrong, a flags cell ticked](../images/2da-table-described.png)

```toml
rows = "spells"                 # each row says more of the same row of spells.2da

[columns.Level]
kind = "int"                    # a whole number
min = -1
max = 10
note = "The spell's level"      # shown with the column

[columns.DamageType]
kind = "flags"                  # several values at once: bits of the cell
constants = "nwscript"          # the constants of this script...
prefix = "DAMAGE_TYPE_"         # ...whose names begin so

[columns.Shape]
kind = "list"                   # one of these values
values = { Sphere = 0, Wall = 1, Cone = "0x03" }
other = true                    # or a value typed by hand

[columns.ClassRequired]
kind = "row"                    # a row's number
table = "classes"

[columns.Name]
kind = "strref"                 # a talk-table string's number

[[rules]]                       # a cell that must be filled...
when = { column = "Effect", is = "CURE" }
require = "Cures"               # ...where another says so
```

| Kind | The cell holds | Wrong when |
| --- | --- | --- |
| `text` (or no kind) | anything | never |
| `int`, with `min` and `max` if you like | a whole number, decimal or `0x` | it is no whole number, or out of the range |
| `float`, the same | a number with a fraction | it is no number, or out of the range |
| `list` | one of the values | it is none of them (unless `other = true`) |
| `flags` | any of the values together, each a bit | a bit no value has is set, or it is no number |
| `row`, with `table` | a row's number in that table | the table has no such row |
| `strref` | a talk-table string's number | there is no such string |

- **A list's values** are written `values = { NAME = 1, OTHER = 2 }`
  (a value in the table's own way of writing it goes in quotes:
  `"0x0010"`), or read from a script: `constants` names the script and
  `prefix` what the names begin with. The script is one of your project's
  (any folder of it) or the game's; its constants are read when the
  table is opened, so the list is never behind the script. A constant
  made of others of its script (`A | B`, `1 << 3`) is worked out; one
  that cannot be (a function's result) is left out, and said. Both may
  be given: the written values come first. A cell with several flags
  shows each by its name, and not a name for them all together too.
- **`rows`** says the table's rows are another table's, row for row (the
  AI's facts about each spell beside `spells.2da`): the other table's
  row is shown with the cell, and a row past the other's last is wrong.
- **A rule** wants one cell filled where another holds a value (its name
  or the value itself).
- A table the description names is your project's own if it has one of
  that name, else the game's. The project is the folder above the table
  that has a `nasher.cfg` or a `.git`; else the table's own folder.

In the 2DA editor, a described table says so under its buttons, and:

- cells that are wrong by the description are marked like the others,
  and counted on the button that goes to the next;
- a list's and a flags' column shows each value's name (**Values as
  names**, on by default; the value is in the cell's tip, and is what
  typing in the cell edits);
- the selected cell is chosen from its list, ticked from its flags (a
  cell with none is empty; the value is written as the cell wrote it,
  hexadecimal or decimal) or chosen from the other table's rows by name;
- **Find** looks through the names shown as well as the values;
- the file is read again when it changes, so the description and the
  table can be written side by side.

Where the description is looked for:

| The table | Its description |
| --- | --- |
| A file opened in the editor | beside the file |
| The module's, in a nasher project or a module folder | beside the table's file |
| The module's, in a `.mod` file | beside the `.mod` file |
| A hak's, in a hak built from a folder | beside the table's file in that folder |
| A hak's, otherwise | beside the hak's file |

**Verify Module** (and `mg verify`) holds the module's own described
tables to their descriptions, as the check `2da-described`. From a
terminal, `mg 2da-check` does it for every described table of a folder,
a hak's sources too ([Command-line tools](11-command-line.md)); a
plugin reads a description with `mg.twoda.describe`
([the plugin API](17-plugin-api.md)).

## Import and export

- **File › Export…** writes chosen resources to an `.erf` archive.
  Two options add the module resources they use (an area its blueprints,
  scripts and conversations, a conversation its scripts…), and move
  creatures out of the module's own factions into the standard ones.
  Right-click a resource in the tree to export it. The window lists the
  module kind by kind (areas, conversations, scripts, the blueprints,
  then the rest by extension); **Kind** lists one kind alone, **Filter**
  those whose names hold what is typed, and **Select the Listed** and
  **Clear Them** tick and untick what the two leave. With **Add to the
  file if it exists**, choosing an archive that is already there adds
  the resources to it (those of the same names are replaced, the rest
  and its description stay) instead of replacing the file.
- **An export says what it was made from.** After the description, the
  archive holds a stamp: the module's git commit, if the module is in a
  git checkout (a nasher project's folder, a module folder, or the
  folder of a `.mod`), and whether its files had changes not committed;
  Moonglow's version; the module's name and tag. In the description:

  ```
  Moonglow-Export: 2.0.0
  Base: git 5f96447… with uncommitted changes
  Module: The Domains of Dread (TDOD)
  ```

  The game and Aurora pass an archive's description by (Aurora's
  Import shows it), so the archive imports as before. `mg export` stamps
  its archives the same way.
- **Export as Files…** (in the Export window, and on a resource's
  right-click menu in the module tree) writes resources as loose files,
  `name.ext`, into a folder you choose: as they are now in the toolset,
  saved or not. A script goes with its compiled script (`.ncs`, and
  `.ndb` if it has one), an area with its `.git` and `.gic`. From the
  window, **Include the resources they use** applies too.
- **Export Files…** (over the picture of a placeable, door, item or
  creature in its editor, and in the model viewer) writes the files the
  game draws it with into a folder you choose: its models, their
  walkmeshes (`.pwk`, `.dwk`), textures, materials and their settings
  (`.mtr`, `.txi`), the models its emitters throw and the model its
  animations come from, taken from the haks and wherever else in the
  load order they are. No digging through haks to hand a broken
  placeable to whoever mends it. The game's own files are left out,
  unless all of it is the game's; the log lists each file and the hak
  it came from. Where the folder has files of those names already, it
  asks first: replace them, skip those, or cancel. `mg model-files` does
  the same from a terminal.
- **Copy to Scratch Folder** (the same right-click menu; **To Scratch** in
  the script editor and the area view) writes them into the scratch
  folder: one you choose the first time, kept from then on (Tools ›
  Options › Folders changes it). Make it the game's or a server's
  `development` folder, which the game loads before the module's own
  resources: compile a script, send it to scratch, and a running server
  or game has the fix. (Aurora users pick such files out of its `temp0`
  folder; Moonglow keeps a module in memory, so it hands them over
  instead.) A script changed since it was compiled is compiled
  first, wherever it is sent from, and the log names it; one that no
  longer compiles goes with the compiled script it has, and the log says
  that too.
- A module opened as a folder (**File › Open Folder…**) or kept as a
  nasher project has its resources as files already.
- **File › Import…** adds an `.erf`'s resources to the module; for those
  the module already has, you choose which to overwrite. It also lists
  what the imported resources refer to that neither the module nor the
  game has. An archive Moonglow exported says what it was made from, and
  where the module's git checkout is at another commit now, the window
  (and the log) warns: the archive was made from **older work** (the
  module is that many commits on: copied in, it undoes what changed
  since; import it on a branch from the commit it names and merge), from
  **other work**, or from a commit the checkout doesn't have; and it says
  when the archive was made with changes not committed. `mg import` says
  the same.

## Areas

**Wizards › Area Wizard…** makes an area from a name, a tileset and a
size (2 to 32 tiles each way). It opens with the first tileset chosen
(Aurora chooses none). Its ResRef is made from the name, after [your
builder code](#your-builder-code) where you have one (the wizard then
shows it). **Edit › Resize Area…** and **Rotate Area…**
change the area shown last, as in Aurora. See [Areas](04-areas.md).
