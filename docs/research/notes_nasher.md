# nasher source trees: what nasher writes and reads

nasher 1.1.3 (August 2026) with neverwinter.nim 2.3.1, read from their
sources (`src/nasher/utils/nwn.nim`, `unpack.nim`, `utils/target.nim`, and
neverwinter.nim's `gffjson.nim`) and checked against nasher's output.
`crates/mg-corpus-tests/tests/nasher.rs` holds Moonglow to it: nasher's
unpack of all 28 shipped modules and campaigns (18,777 GFF files) is, file
for file, the text Moonglow writes, and each file reads back to a resource
Moonglow writes as the same text.

## A GFF resource's text

`nasher unpack` runs `nwn_gff -k json` on each GFF, then:

1. **Fields it drops:**
   - `module.ifo`: `Mod_ID`, which `nwn_gff` sometimes can't read back.
   - An area's `.are`: `Version`, which Aurora increments at every save.
2. **Keys are sorted** in every object, comparing them ignoring ASCII case.
   The sort is stable, so keys equal but for case keep the file's order.
   `__data_type` and `__struct_id` sort first, because `_` comes before the
   letters once lowercased.
3. **Floats are rounded** to `truncateFloats` places (default 4):
   - Each value is formatted with that many decimals, trailing zeros are
     trimmed, and the result is read back.
   - Under a key named `Bearing` or `Orientation`, a value that rounds to
     -π becomes +π.
   - Integers are left alone.
4. **The text is written** with Nim's `pretty()` and a final newline:
   - two spaces of indent and `"key": value`;
   - `{}` and `[]` when empty;
   - Nim's string escapes: `\n \b \f \t \r \" \\`, `\u000b` for vertical tab,
     and other control characters as `\u00XX` with uppercase hex. `/` and
     non-ASCII characters are written as they are, in UTF-8.
5. **Floats print with C's `%.16g`**, as nasher's build of Nim does:
   - 16 significant digits, trailing zeros dropped;
   - scientific notation below 1e-4 and from 1e16, with an exponent of at
     least two digits (`1e-05`, `1e+20`);
   - `.0` added when the result has neither a point nor an exponent.

   So 95.9779 prints as `95.97790000000001`. `nwn_gff` itself, built with a
   newer Nim, prints the shortest digits instead (`95.9779`, `1e+17`, `1e-8`),
   but nasher re-prints its output.

What `nwn_gff` puts into the JSON:
- **Field types:** each field is `{"type": ..., "value": ...}`.
- **Voids** are base64 in `value64`.
- **Localized strings** are `{"<language*2+gender>": text, "id": strref}`,
  without `id` when there is no strref.
- **Struct ids:** a struct field carries `__struct_id` beside `type`, and list
  items carry their own. A struct id of 0xFFFFFFFF is left out. Ids are
  printed as signed 32-bit numbers.
- **The file type** is in `__data_type`.
- **Text** is decoded from windows-1252, unless `nwn_gff --nwn-encoding`
  (passed through nasher's `gffFlags`) names another codepage.

## Other files

- **Scripts (`.nss`) and anything that isn't a GFF** are copied unchanged.
- **Compiled scripts (`.ncs`) are build output.** Unpack never writes them,
  and pack compiles them from the `.nss`.
- **TLKs** become JSON through `nwn_tlk` when the target is a `.tlk`.

## Where files go (`unpack`)

For each file in the archive:
- **Already in the tree:** if the target's source tree has a file for it
  (`name.ext`, or `name.ext.json` for a GFF), it is written there.
- **More than one place:** `onMultipleSources` decides (by default nasher
  asks).
- **New files:** the first `[rules]` pattern matching the file name decides
  its folder; a destination of `/dev/null` discards the file. Patterns are
  globs on the name alone (`"*" = "src"`). Without a match the file goes to
  `unknown/`.
- **Deleted resources:** with `--removeDeleted`, their source files are
  deleted.

## `nasher.cfg`

The format is Nim's parsecfg, not TOML: keys may repeat (`include` twice).
- **Sections:**
  - `[package]`, with `[package.sources]`, `[package.rules]` and
    `[package.variables]`;
  - then one or more `[target]` sections, each with its own `.sources`,
    `.rules` and `.variables`;
  - `[package]` must come first.
- **Keys:** a target takes `name`, `file`, `description`, `default`,
  `parent`, `group`, `flags`, `branch`, `modName`, `modMinGameVersion`, and
  `modDescription`. Sources take `include`, `exclude`, `filter` and
  `skipCompile`.
- **Unknown keys** in a `[target]` or `[package]` section are treated as
  unpack rules.
- **Inheritance:** a target inherits each missing field from its `parent`
  target, or from `[package]` if it has none. Lists (includes, rules and so
  on) are inherited whole, not merged. Variables are merged.
- **Variables** are written `$name` or `${name}`, looked up in the target's
  variables and then the environment.
  - `$target` is the target's name.
  - `$ext` is left as it is (in a rule's destination, it is the file's
    extension).
- **The default target** is the one with `default = true`, or the one named
  by `[package] default`, or else the first.
- **Source files:**
  - The `include` globs are walked from the package root, in order. Hidden
    files and directory links are skipped.
  - Files that the `exclude` globs also walk are removed, and duplicates are
    dropped.
- **Glob syntax:** `*` doesn't cross `/`, `**` spans folders, and `?`,
  `[...]` and `{a,b}` work as usual. Matching ignores case on Windows.
- **Per-user settings** live in `.nasher/user.cfg` (package) and
  `~/.config/nasher/user.cfg` (global): `gffFormat` (`json` or `nwnt`),
  `gffFlags`, `truncateFloats`, `removeUnusedAreas`, `onMultipleSources`,
  `installDir`.

## What Moonglow does differently

- **The area list:** on opening a project, Moonglow adds to `module.ifo`'s
  area list the areas the tree has but the list misses (after a merge, say),
  as nasher's pack would.
  - It keeps listed areas the tree lacks, which nasher's pack drops. They
    may be in a hak: Kingmaker, ShadowGuard and Witch's Wake keep all their
    areas in haks.
  - The change is counted as read, so `module.ifo.json` is rewritten only
    with other changes to it.
- **Conflicts:** a save never replaces or deletes a file that changed on
  disk since it was read. It writes nothing and names those files.
- **Settings:** `gffFormat = nwnt` isn't read yet.

## In the engine

nasher's rounding is visible in the game. The shipped modules, packed from
their nasher trees, present the same areas and objects as the originals
(6,389 objects in eight modules, `engine_modules.rs`), but objects may sit
a hundredth of a meter away. They may also face a degree differently:
`GetFacing` reports whole degrees, and a bearing just below 0 that rounds
to -0 reads as 360. A larger `truncateFloats` keeps more precision.

## Packing

- GFF JSON goes back through `nwn_gff`, and scripts are compiled, skipping
  `skipCompile`.
- `module.ifo`'s area list is then rebuilt from the `.are` files present, in
  file-system order, unless `removeUnusedAreas = false`. An area without a
  `.git` gets a warning.
- `modName`, `modDescription` and `modMinGameVersion` are applied.
- Files matching `filter` are left out, and `nwn_erf` packs the rest.
