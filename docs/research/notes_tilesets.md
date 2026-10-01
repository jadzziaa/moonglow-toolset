# NWN:EE tileset system: research notes for Moonglow (area editor / terrain painting)

Game build checked: 89.8193.37-17 (Steam install, read-only). Date: 2026-09-30.

Evidence tags used below:
- **[V]** verified against real game data in this session (scripts listed at the end)
- **[D]** documented: nwn.wiki page, BioWare Aurora PDF, nwscript.nss or EE patch notes (source given)
- **[I]** inference, plausible but not verified
- **[?]** open question / needs reverse-engineering (Aurora under Wine)

Scratch artifacts (reusable): `SP=/tmp/claude-1000/-home-august-Projects-moonglow-toolset/42266be1-7a5b-4aaa-92bd-b9aa74640348/scratchpad`
- `$SP/tilesets/`: all 33 base `.set`, their `*palstd.itp` (+ `.json`), all `*_edge.2da`, `doortypes.2da`, `genericdoors.2da`, `lightcolor.2da`, `tilecolor.2da`, `surfacemat.2da`, `loadscreens.2da`, `footstepsounds.2da`, `areag.ini`
- `$SP/mod_*/`: ARE/GIT JSON from Prelude, Chapter1, Dark Dreams of Furiae, Darkness over Daggerford, Tyrants of the Moonsea (441 areas)
- `$SP/tilemdl/`: sample tile `.mdl`/`.wok`, door `.dwk`, placeable `.pwk`, minimap TGAs
- `$SP/research/bioware_are.txt`, `bioware_itp.txt`, `door.txt`: text of the BioWare Aurora ARE/GIT, ITP and Door/Placeable PDFs (github.com/kucik/nwn-docs, nwn.wiki attachments)
- Scripts: `setstat.py` (tolerant SET parser + stats), `setstat2/3.py`, `itpcheck.py`, `arecheck.py`, `groupcheck.py`, `doorcheck.py`, `sigs.py`

Test oracle for SET parsing: nwn.py `~/.local/opt/neverwinter/venv/lib/python3.14/site-packages/nwn/tileset.py`
(`read_set()`; dataclasses Set/SetGrass/Terrain/Crosser/Rule/Tile/Door/Group). It parses all 33 base SETs, and its counts match mine exactly [V].
Its gaps: it drops VisibilityNode, VisibilityOrientation, DoorVisibilityNode, DoorVisibilityOrientation, SelectorHeight, UnlocalizedName (general and terrain), GrassTextureName, Group StrRef and SECONDARY RULES. It uses configparser, so key case is lost. It attaches `[TILEnDOORm]` sections whatever `Doors=` says.
A second reference is Neverblender's `~/Projects/neverblender/tools/e2e/build_test_tileset.py`, which writes a minimal one-tile tileset (set, mdl, wok, minimap, hak). It is useful as a test fixture.

---

## A. SET file format

### A.1 General syntax
- Windows INI text. Base files use CRLF line endings and start with `; NEVERWINTER NIGHTS TILESET FILE` / `; DO NOT EDIT MANUALLY - UNLESS YOU KNOW WHAT YOU ARE DOING` [V]. The toolset binary holds both comment lines as strings, so Aurora has SET-writing code [V strings].
- Comments are whole lines starting with `;`. Trailing comments are not allowed [D Tilesets].
- The toolset reads the file **sequentially**. Sections must come in this order or it may crash: GENERAL, GRASS, TERRAIN TYPES, TERRAINn…, CROSSER TYPES, CROSSERn…, PRIMARY RULES, PRIMARY RULEn…, SECONDARY RULES, TILES, TILEn (each followed by its TILEnDOORm), GROUPS, GROUPn [D SET, Tileset Construction Tutorial]. Write in this order; read tolerantly.
- UNIX (LF) line endings reportedly crash the toolset on area creation [D Tilesets]. A trailing newline after the last group is needed on some systems (Wine 9) [D SET]. **Moonglow: write CRLF plus a trailing newline.**
- Section and key names are case-sensitive as written (upper-case sections, CamelCase keys). Value comparisons for terrain and crosser names are case-insensitive [V: base data mixes `Floor` and `floor`, e.g. tdm01 defines `Floor` while its tiles say `floor`; tcn01 has `Border=building`].
- Numbers: some integers are empty (`TopLeftHeight=` appears in trm02/trs02); treat empty as 0 [V]. Orientation-type values show up as `0`, `90`, `-90`, `270`, `-180`, `-270`, and door Orientation also as float strings (`270.0`, `62.1776`) [V].
- Full key vocabulary the toolset reads, from nwtoolset.exe strings [V strings]:
  GENERAL/GRASS/TERRAIN TYPES/TERRAIN/CROSSER TYPES/CROSSER/PRIMARY RULES/PRIMARY RULE/SECONDARY RULES/SECONDARY RULE/TILES/GROUPS/GROUP/SOUND, `Count, Crosser, Orientation, DisplayName, UnlocalizedName, Version, Transition, SelectorHeight, Interior, EnvMap, Border, Default, Floor, DefaultAreaProperties, Grass, Density, Height, HasHeightTransition, AmbientRed/Green/Blue, DiffuseRed/Green/Blue, Columns, Image, StrRef, Adjacent, AdjacentHeight, Changed, ChangedHeight, Placed, PlacedHeight, Terrain, Bottom, BottomLeft, BottomLeftHeight, BottomRight, BottomRightHeight, Doors, Sound, Sounds, Model, VisibilityOrientation, DoorVisibilityOrientation, PathNode, VisibilityNode, DoorVisibilityNode, Right, TopLeft, TopLeftHeight, TopRight, TopRightHeight, WalkMesh, ImageMap2D, MainLight1, MainLight2, SourceLight1, SourceLight2, ReplaceTexture, AnimLoop1..3`.
  Keys absent from all base SETs: `DefaultAreaProperties`, `Image`, `ReplaceTexture`, `Sound`/`SOUND`, `Terrain`/`Crosser` as keys, `SECONDARY RULEn`. Their meaning is unknown [?].
  (`GrassTextureName` did not show up in the string dump, but base files use it and it is documented since 1.69 [D 1.69 notes].)
- Toolset load-error strings [V strings]: `Invalid Tile, %s`, `Invalid Crosser, %s`, `Blank Parameter`, `Blank parameter found for TopLeft terrain type in %s` (and TopRight, BottomLeft, BottomRight), `Invalid Group: %s.  Check tile count and image.`, `Tile group '%s' has no entry for tile #%d!`, `The toolset will treat this as a NULL tile, which may or may not be what the tileset creator intended.`, `Please inform the tileset creator of this error message.`

### A.2 `[GENERAL]`
Two key orders occur in base data: `Name,Type,Version,Interior,HasHeightTransition,EnvMap,Transition,[SelectorHeight],DisplayName,[UnlocalizedName],Border,Default,Floor` [V].

| Key | Type | Meaning |
|---|---|---|
| Name | str | Tileset name, usually the resref upper-cased (`TTR01`); also lower case [V] |
| Type | `SET` | constant |
| Version | `V1.0` | constant |
| Interior | 0/1 | Interior tileset. Should match `Interior` in areag.ini [D 1.69 notes]. It sets the initial area flags [I] |
| HasHeightTransition | 0/1 | Enables the Raise/Lower tool [D]. Toolset 1.80.8193.14 fix: "Repaired DAG01.set to NOT offer unsupported height transitions" [D patch notes] |
| EnvMap | resref | Default environment-map texture for the tileset (`ttr01__ref01`) |
| Transition | number | Metres per height step. Base values: 5 (11 sets), 3 (19), 4 (tcn01), 2 (tno01), 1 (twc03) [V]. Floats allowed in EE [D Tutorial]. Must be ≥1, because Transition=0 breaks group placement next to other tiles [D Common Issues] |
| SelectorHeight | number | Height (m) of the tile-selector box in the toolset. Defaults to Transition when absent [D 1.69 notes]. Found in 7 sets: 3, 4, 8, 10, 15 [V] |
| DisplayName | int StrRef | Tileset name in the area wizard. -1 = none. Values ≥ 16777216 are custom-TLK refs [D Tilesets] |
| UnlocalizedName | str | Used when DisplayName = -1 (EE sets: `Medieval City 2`, `TNO: Exterior`). An optional `name:description` split on the colon is documented [D Tilesets] |
| Border | terrain name | Terrain of the vertices on the area's outer boundary [D, V: e.g. tcn01 Border=building, Default=cobble] |
| Default | terrain name | Terrain the Eraser paints, and the fill for a new area [D ITP PDF: "The eraser paints down the tileset's default terrain type"] |
| Floor | terrain name | "Main floor" terrain. Interior sets have Floor ≠ Default (tdm01 Default=Wall, Floor=Floor). A new tdm01 area is "mostly walled up … except for a small space in the center" [D BioWare Toolset Tutorial]. So Floor is likely what the area wizard seeds in the centre [I] |

### A.3 `[GRASS]`
Keys: `Grass` (0/1), `GrassTextureName` (resref, default `grass`; e.g. `ttz_grass`, `tcm02_grass3d`; added in 1.69), `Density` (float, e.g. 5.0/6.0/3.0), `Height` (m, 0.5–1.5), `AmbientRed/Green/Blue`, `DiffuseRed/Green/Blue` (0–1) [V, D]. When `Grass=0` the section is often just `Grass=0` (19 sets) [V]. Grass grows on walkmesh faces with surface material 3 (Grass) [D]. `ReloadAreaGrass()` and `SETTILE_FLAG_RELOAD_GRASS` exist (87.8193.35) [D nwscript].

### A.4 `[TERRAIN TYPES]` / `[TERRAINn]`, `[CROSSER TYPES]` / `[CROSSERn]`
- `Count=N`, then `[TERRAIN0]`…`[TERRAIN{N-1}]` holding `Name` (no whitespace) and an optional `StrRef`. Crossers are the same (`[CROSSERn]`). Keys found: `Name` or `Name,StrRef` [V]. `UnlocalizedName` is documented [D Tilesets] but unused in base data.
- Terrains and crossers are referenced **by name** (case-insensitive) in tiles, rules and the ITP, never by index [D, V].
- Some tiles use names that are not defined: `Room` in tib01, `padding` in trs02, crosser `path` in tcm02/trm02/trs02 [V]. Those tiles can never match a paint request, so they only appear through groups [I].

### A.5 `[PRIMARY RULES]` / `[PRIMARY RULEn]`, `[SECONDARY RULES]`
- Rule keys: `Placed, PlacedHeight, Adjacent, AdjacentHeight, Changed, ChangedHeight` [V]. The notation below is `terrain@height`.
- Meaning [D Tilesets, I refined]: when the brush sets a vertex to `Placed@PlacedHeight` and a neighbouring vertex holds `Adjacent@AdjacentHeight`, that neighbour is rewritten to `Changed@ChangedHeight`. When Changed equals Adjacent, the rule only records that the pair is legal.
- Base sets list the full product of (terrain × height) squared: ttr01 has 28 rules over 3 terrains and 2 heights. Examples:
  - ttr01 `Water@0 next to Water@1 → Grass@1`: raised water has to be ringed by raised grass.
  - ttr01 `Water@1 next to Grass@0 → Grass@1`: placing raised water raises the neighbours.
  - tic01 `Storage@0 next to Rich@0 → Wall@0`: two different rooms are always split by Wall.
  - tcn01 `goodcastle@0 next to water@0 → cobble@0`.
  The "buffer" terrain these rules convert to is always the Default/base terrain [V].
- Counts: ttr01 28, tts01 28, tts02 33, tcn01 76, tno01 192, tcm02 51, trm02 50, trs02 41, tic01 36, tin01/tni01 25, tss13 16, tdr01 14, the tdc01/tde01/tds01/tdm01/tdt01/tsw01/ttd01/ttf01/ttf02 family 9 (all "=" identity), ttu01 3. Zero rules: ttz01, twc03, tni02, tid01, tii01, tib01, tbw01, tti01, tms01, dag01 [V]. **ttz01 has height transitions and 0 rules**, so painting must work with no rules at all, using only the tile-matching search [V/I].
- `[SECONDARY RULES] Count=0` in 23 sets. The section is absent in 10 (tcn01, tdm01, tdt01, tni01, tni02, tno01, tsw01, ttz01, twc03, tbw01) [V]. Its semantics are unknown [?]. The toolset parses `SECONDARY RULE` sections [V strings]. The wiki calls them "not used".

### A.6 `[TILES]` / `[TILEn]`
`Count=N`, then `[TILE0]`…`[TILE{N-1}]` in index order. **The Tile index is what ARE `Tile_ID` stores** [V]. Never renumber tiles in a tileset that existing areas use.
Key order in base data: `Model, WalkMesh, TopLeft, TopLeftHeight, TopRight, TopRightHeight, BottomLeft, BottomLeftHeight, BottomRight, BottomRightHeight, Top, Right, Bottom, Left, MainLight1, MainLight2, SourceLight1, SourceLight2, AnimLoop1, AnimLoop2, AnimLoop3, Doors, Sounds, PathNode, Orientation, [VisibilityNode, VisibilityOrientation], [DoorVisibilityNode, DoorVisibilityOrientation], ImageMap2D` [V; 5 variants, 12,342 tiles].

| Key | Meaning |
|---|---|
| Model | Tile MDL resref (≤16 chars). A matching `.mdl` and `.wok` exist for every base tile except tcm02_b92_02 and trs02_m00_00 [V] |
| WalkMesh | `msb01` in 11,710 tiles. Junk values also occur (`dude`, `rina`, `jasa`, `bates`, `jurf`, empty). Unused; the walkmesh is `<Model>.wok` [V/D] |
| TopLeft/TopRight/BottomLeft/BottomRight | Corner terrain names. Top = +Y (north), Right = +X (east) in model space |
| *Height | Corner height in Transition units, relative to the tile's own base. Base data only has 0 or 1 [V]: 9,388 flat tiles, 2,938 with mixed 0/1 corners, 16 with all corners at 1 (trm02/tcm02) |
| Top/Right/Bottom/Left | Crosser on that edge. Empty means none [V] |
| MainLight1/2, SourceLight1/2 | "Tile has this light" flag. Mostly 1; junk values 16/205/121/7/116/104/4 occur [V]. Treat as nonzero = present [I]. Presence really comes from the model nodes `<model>ml1/ml2/sl1/sl2` [D ARE PDF] |
| AnimLoop1..3 | "Tile has animloop n" flag (mostly 1) [V] |
| Doors | Number of `[TILEnDOORm]` sections. 0–4 [V]. **Mismatches exist in 50 tiles**: Doors=0 with a DOOR0 section (dag01 tiles 2/6/11/15/17/23, tno01, tni01, trm02) and Doors=1 with no section (tcm02 1227/1337, tin01 298). Official DoD areas **do have doors placed on the dag01 hookpoints whose Doors=0** [V]. So either the loader enumerates sections, or the doors predate an edit [?] |
| Sounds | Always 0 [V]. The sub-section format is unknown [?] |
| PathNode | A–Z, a–x (case-sensitive), orientation-independent definition (§D.4) |
| Orientation | Rotation of the PathNode, in degrees: 0/90/180/270/-90/-180/-270. 90 = CCW [D Tilesets] |
| VisibilityNode / VisibilityOrientation | Path-node-style graph for line of sight. Defaults to PathNode when absent [D Tutorial]. Present on 4,098 tiles [V] |
| DoorVisibilityNode / DoorVisibilityOrientation | Visibility graph used while a door on the tile is closed [D]. Only 336 tiles have it; 2,373 tiles with Doors>0 lack it [V] |
| ImageMap2D | Minimap texture resref, **stored explicitly** [V]. Conventions: `mi_<model>` (EE-era sets), `mi<tileset-without-t>_<series>` shared by variants (`mitr01_A01`, `MICN01_B08`), `mall_a01`, `mi_temp01` (tms01). TGA, 16×16 (old) or 32×32 (EE) [V] |

`[TILEnDOORm]` (no spaces; m from 0) has `Type, X, Y, Z, Orientation` [V, 3,486 sections]:
- `Type`: doortypes.2da row. `0` = **generic hookpoint**: the builder picks any genericdoors.2da appearance, and the door is stored with `Appearance=0` and `GenericType`=row. Nonzero values are tileset-specific doors, e.g. 5005 = `tcm02_WarehouseDoor`, 204 = `CastelRich*IC` [V doortypes.2da, GIT].
  The BioWare Door PDF words it as "generic hookpoint entry contains an index into genericdoors.2da". Base data only uses 0 for generic, so treat 0 as generic [V].
  The toolset also matches door appearances through the TileSet, TileSet0, TileSet1 and TileSet3 columns of doortypes.2da [D 1.69 notes].
- `X, Y, Z`: offset in metres from the **tile centre** (not the corner) in unrotated tile space [D Adding Doors, V].
- `Orientation`: door facing in degrees, in tile space [V].
- Doorway crosser edges almost always have a hookpoint at the edge midpoint (±5,0) or (0,±5), facing inward: 293 of 312 checked in tin01/tic01/tdm01/tdc01 [V]. Two tiles sharing a doorway edge therefore both offer a hookpoint at the same spot [I].

### A.7 `[GROUPS]` / `[GROUPn]`
Keys: `Name`, optional `StrRef`, `Rows`, `Columns`, `Tile0`…`Tile{R*C-1}` [V]. Group sizes in base data: 1×1 (1,063, the "features"), 2×2 (252), 1×2 (196), 2×1 (105), 3×1 (87), up to 9×8 and 7×7 [V].
- **Layout, verified on placed groups in official areas** [V: 770 placements match fully; 24 match partially, which are partly deleted groups or shared tiles; the column-major hypothesis fails]: index k gives column `c = k % Columns` (+X, east) and row `r = k / Columns` (+Y, north). Tile0 is the bottom-left (south-west) tile. Rows = Y extent, Columns = X extent. Example: ttr01 Windmill_2x2 is Tile0=u11, Tile1=v11, Tile2=u10, Tile3=v10.
- **Rotation** [V]: every tile of a placed group gets the same `Tile_Orientation o`, and the offset (c,r) of each tile relative to Tile0's cell is rotated CCW by o·90°: (c,r) → (−r,c) per step. The pivot is Tile0's cell (data-level description; the pivot the UI uses is [?]).
- `TileK=-1`: that cell is left to terrain-matched random tiles [D SET]. Found 40 times [V], and even as **Tile0** in tcm02 GROUP72 `CornerGate1`. The wiki says Tile0=-1 crashes the toolset, and such a group cannot be reached from the ITP.
- Tiles used in any group, and any tile whose model equals a group's Tile0 model, are excluded from random terrain and crosser matching [D Tutorial, Tilesets].
- The ITP identifies a group by **the model resref of its Tile0**, so Tile0 must be unique per group. Duplicates exist: tni01 tile 186 is Tile0 of 3 groups, tno01 tile 353 of 2 [V]. Sources disagree on which group wins: the BioWare ITP PDF says "the first tilegroup", nwn.wiki Tilesets says "the last" [D, conflict → ?].
- Group painting "locks" the tiles inside the group (at the centre) and skips template verification for them, which lets 3×3+ groups with null tiles work [D 1.69 notes]. On area load the toolset checks for valid tile-group placement [D 1.69].

### A.8 Counts (base game) [V]
| Set | Tiles | Terr | Cross | PrimRules | Groups | Door sections | Transition | HT | Interior |
|---|---|---|---|---|---|---|---|---|---|
| ttr01 Rural | 283 | 3 | 4 | 28 | 67 | 38 | 5 | 1 | 0 |
| tcn01 City Ext | 408 | 5 | 4 | 76 | 89 | 104 | 4 | 1 | 0 |
| tic01 Castle Int | 179 | 6 | 2 | 36 | 34 | 97 | 3 | 0 | 1 |
| tdm01 Mines | 248 | 3 | 5 | 9 | 40 | 92 | 3 | 0 | 1 |
| ttf01 Forest | 168 | 3 | 4 | 9 | 28 | 13 | 3 | 0 | 0 |
| tno01 Castle Ext Rural | 1287 | 7 | 9 | 192 | 198 | 288 | 2 | 1 | 0 |
| tcm02 Medieval City 2 | 1872 | 7 | 5 | 51 | 204 | 1175 | 5 | 1 | 0 |
| trm02 Medieval Rural 2 | 1644 | 7 | 6 | 50 | 161 | 138 | 5 | 1 | 0 |
| tms01 Microset | 16 | 1 | 1 | 0 | 2 | 0 | 5 | 1 | 0 |
| tss13 Sea Ships | 404 | 4 | 1 | 16 | 132 | 96 | 5 | 0 | 0 |
All 33 base sets: dag01 tbw01 tcm02 tcn01 tdc01 tde01 tdm01 tdr01 tds01 tdt01 tib01 tic01 tid01 tii01 tin01 tms01 tni01 tni02 tno01 trm02 trs02 tss13 tsw01 ttd01 ttf01 ttf02 tti01 ttr01 tts01 tts02 ttu01 ttz01 twc03.
Paintable (non-group) tiles and distinct rotation-canonical signatures: ttr01 151/108 (the flat-grass signature has 12 variants), tcn01 147/93, tic01 135/99, tdm01 180/125, ttf01 103/59, tno01 843/563, tcm02 1470/720 (up to 26 variants per signature) [V].

Tile naming convention: `<ts>_<row letter><NN>_<variant NN>`. a–e terrains, f–k crossers (g streams, h roads, i walls), l–n extra terrains, o–y groups/features, z edge tiles, `tall_*` shared across sets [D Tilesets]. Informative only, the engine does not care.

---

## B. Tile corner/edge model and painting

### B.1 Geometry [V unless noted]
- A tile is 10×10 m, model origin at the tile centre, corners at (±5,±5). Z=0 is height level 0 of that tile [D Tutorial; V wok verts].
- Corners carry (terrain, height). Edges carry a crosser name or empty.
- **Rotation**: `Tile_Orientation o ∈ {0,1,2,3}` means o × 90° **counter-clockwise** [D nwscript.nss SetTile, BioWare ARE PDF].
  With corners listed clockwise as [TL, TR, BR, BL] and edges as [T, R, B, L], one CCW step gives `new[i] = old[(i+1) % 4]`. So new TL = old TR, new TR = old BR, new BR = old BL, new BL = old TL, and new T = old R, new R = old B, new B = old L, new L = old T.
  Verified: 100% corner agreement over **~161,000 shared corner and edge comparisons in 439 official areas** (Prelude, Chapter1, Furiae, DoD, ToM). The CW hypothesis and the column-major hypothesis both fail.
- **Absolute vertex height** = SET corner height + `Tile_Height`. Adjacent tiles agree on absolute heights, not relative ones [V]. Official areas reach Tile_Height 19 (tno01, Transition 2 m), with values 2–19 common in tno01, trm02 and trs02 [V].
  So a tile's base height is the minimum of its absolute corner heights and the relative corners are 0/1 [I, consistent with the data]. Heights can never be negative [D ARE PDF; D Tutorial].
- Consistency rule the toolset maintains [V]: for every pair of neighbouring tiles, shared corner terrain (case-insensitive) and absolute height are equal (0 violations), and the shared edge crosser is equal. There were 13 crosser violations in 161k checks: 8 tss13 gangplank group edges, 4 tni01 doorway edges, 1 tno01 sandbank. Groups are matched against their neighbours too.

### B.2 Vertex grid model (recommended internal representation) [I, consistent with data]
- An area of W×H tiles has (W+1)×(H+1) vertices, each holding (terrain, absolute height). Each tile additionally has 4 edges holding a crosser.
- Tile (x,y) uses vertices (x,y)=BL, (x+1,y)=BR, (x,y+1)=TL, (x+1,y+1)=TR.
- A "tile signature" is (4 corners relative to min height, 4 edges). Candidate tiles for a cell are all non-group tiles t and orientations o such that rot(t,o) equals the target signature. Build an index from canonical signature to [(tile, orientation)] at tileset load.
- Tiles with rotational symmetry match under several orientations. Official areas show near-uniform orientation distributions for such tiles, so the toolset randomises orientation among equivalent matches [V distribution, I cause].

### B.3 Painting operations: what is documented
- **Terrain brush**: acts on a vertex. "Base terrain is done affecting 4 tiles at once … centred between tiles" [D Area Editor]. Left-click paints. Painting the same spot again re-randomises the 4 tiles [D].
- **Shift + click** (or Shift + right-click on a tile in select mode) cycles through the matching tiles **in order** instead of randomly. Ground tiles cycle through each variant × 4 rotations [D Area Editor, Advanced Tips].
- **Eraser** (`eraser` palette node) paints the SET `Default` terrain. Used on a single tile it re-randomises the appearance. Erasing a group tile removes the group [D ITP PDF, BioWare tutorial, Area Editor].
- **Raise/Lower** (`raiselower` node, only when HasHeightTransition=1): left-click raises, right-click lowers "a portion of the area" [D BioWare tutorial]. You cannot go below 0 [D Tutorial]. EE areas show multi-level heights, so repeated raising stacks [V].
- **Crossers** (roads, streams, walls, corridors, doorways, bridges…): painted by **dragging** from tile to tile. The cursor turns red while invalid. A bridge appears when a road is dragged across a stream and ends on grass [D Vault "Module Mapping 102"]. Crossers can cross other crossers to form bridges [D Area Editor].
- **Groups/features**: a ghost follows the mouse; right-click rotates it 90° [D BioWare tutorial]. Deleting one selected tile of a placed group leaves a partial group (EE) [D Advanced Tips]. BioWare 1.x said group tiles cannot be selected on their own [D tutorial]. Removing a group should remove its doors but may leave them behind [D Tutorial].
- **Delete** on selected tiles re-randomises terrain tiles and removes feature/crosser tiles [D Area Editor].
- **Copy/paste tiles**: same tileset only. Pasting a region behaves like a group, and the toolset may change surrounding tiles so they fit ("grass tiles need a water edge so it might take over water") [D Advanced Tips]. Tile lights are copied too [D].
- **Resize Area** (Edit menu): adds or removes rows and columns at the **top (north) and right (east)**, keeping (0,0) at the south-west. Shrinking deletes objects there. Enlarging copies the edge tiles [D BioWare tutorial, Area Editor]. **Rotate Area** exists and is non-destructive [D].
- Status bar shows mouse x,y, grid (row, column) and the resref of the tile under the mouse [D BioWare tutorial].
- Tile lights: MainLight colours are **chosen randomly when painting a tile or generating the initial tiles**, from environment.2da columns `MAIN1_COLOR1..4`, `MAIN2_COLOR1..4` and `SECONDARY_COLOR1..4` of the area's LightingScheme [D BioWare ARE PDF §2.2].

### B.4 Painting algorithm (reconstruction; **[I]** unless tagged)
1. **Terrain paint at vertex v with terrain T** (height kept = current absolute height of v):
   a. Set v := (T, h).
   b. Apply primary rules outward. For each vertex u that shares a tile with v (the 8 neighbours), if a rule `(Placed=T@hRel, Adjacent=u.terrain@uRel)` exists, set u := Changed@(ChangedRel). The rules are expressed in relative heights (0/1) [?: how absolute heights map onto the 0/1 in rules].
   c. Whether rule changes propagate further (a changed u triggering rules on its own neighbours) is [?]. The 1.69 fix about "locking corners in the interior of a tilegroup" suggests vertices can be locked and that propagation is bounded by locks.
   d. For every tile touching a changed vertex (4 per vertex), pick a random candidate (tile, orientation) whose rotated signature matches. Crossers on the edges of those tiles: [?] whether they are kept (needs tiles with that terrain + crosser) or dropped when no tile matches.
   e. If some tile has no candidate, the toolset refuses the change (red cursor / no-op) or falls back to Default terrain [?]. ttz01 has 0 rules, so the matching search alone must be enough for normal painting.
2. **Raise/lower at vertex v**: v.h ± 1, then the same rules (Placed@h+1 vs neighbours) and matching. Rules like `Water@1 next to Grass@0 → Grass@1` show raising water drags neighbours up. `Grass@0 next to Water@1 → Grass@1` shows the reverse: water can't sit next to a lower tile.
   Brush footprint: the tutorial says "a portion of the area has been raised", which may mean one vertex (4 tiles) or more [?].
3. **Crosser drag** from tile A to tile B (orthogonal neighbours): set the shared edge crosser to C and the crossers leading into each tile's centre. That means a tile gets C on the edges the drag passes through; a path "end" tile gets C on one edge [I from tutorial pictures: "3 basic tiles to make a continuous road with a termination"]. Rematch those tiles. Crossing an existing different crosser selects a bridge tile if one exists.
4. **Group placement** at cell (x,y) with orientation o: write each non-(-1) tile at its rotated offset with orientation o and the group's Tile_Height. Lock the group-interior vertices and edges. The group's outer corners and edges then act as constraints; neighbours are re-matched, and cells with -1 are filled by matching [D 1.69 notes + I].
   Whether a group can be placed on raised terrain (Tile_Height>0) is [?]. Transition=0 breaks it, which implies height maths are involved [D].
5. **Tile choice among candidates**: random, weighting [?]. There is no weight key in SET. Base sets provide duplicates (variants) of common signatures, so uniform random over (tile, orientation) pairs reproduces the look [I]. Shift-click order is probably tile index order × orientation 0..3 [?].
6. **New area** (wizard): all vertices = Default at height 0, with Border terrain on the outer ring [I], and interior sets get a Floor patch in the centre [D tutorial wording + I]. Tiles are chosen randomly, lights from environment.2da [D]. areag.ini supplies the other area defaults (music, flags…) [D areag.ini].

What must be reverse-engineered (Aurora under Wine, scripted paint sequences, diff the ARE `Tile_List`): rule application order and neighbourhood (4 vs 8 vertices), propagation depth, handling of height in rules, crosser persistence under terrain paint, the RNG and shift-cycle order, the raise brush footprint, the new-area seed, what happens when no tile fits, border-ring behaviour, and SECONDARY RULES.

### B.4a Painting as Aurora does it (captured, 89.8193.37) [V]
Verified by `tools/aurora/capture_terrain.py`: 54 scripted steps in ttr01, ttz01, tic01 and tdm01 areas, replayed by `crates/mg-corpus-tests/tests/aurora_terrain.rs` against `mg_tiles::paint`. This settles most of §B.4 and items 1–3 of §I.
- **Terrain brush**: sets one corner's terrain (height kept). Primary rules then apply to its **eight** neighbours (diagonals too: Storage painted diagonally beside Rich turns the Rich corner into Wall). Rule heights are relative: with `base = min(h_placed, h_adjacent)`, the rule for `(Placed, h_placed − base, Adjacent, h_adjacent − base)` applies, and `Changed@k` puts the neighbour at `base + k`. Pairs more than a step apart match no rule. Rules don't chain from the corners they change.
- **Raise/Lower**: one corner a step. Neighbours (eight) follow so that no two neighbouring corners differ by more than a step, recursively (raising a corner three times makes rings at 3, 2, 1). Rules then apply as if the corner's terrain were painted at its new height (raised water lifts the grass around it). Lowering a corner at 0 changes nothing but re-picks the tiles around it. Border and area-corner vertices can be raised.
- **Re-picking**: only the tiles touching a changed corner, plus the four around the painted corner, get new tiles (random among the fits). The others keep theirs. A re-picked tile gets new random lights from the lighting scheme (environment.2da `MAIN1/MAIN2/SECONDARY_COLORn`; `Tile_SrcLight1` = `Tile_SrcLight2`).
- **Refusal**: if any affected cell would have no fitting tile, the whole stroke does nothing. That explains brushes that seem to have no effect, e.g. trees next to a straight road, or water on a raised corner when the ring of grass around it can't be raised.
- **Crossers**: the brush works in quarter-cells. Each cell is split by its diagonals into four quarters, one per edge. With the button down, the crosser goes on the edge of every quarter the pointer passes through, including the one it was in at the press. So a drag that starts at a cell's centre, coming from the north, leaves a stub on the north edge. A click without movement re-picks the cell. A road dragged across a stream picks the crossing (bridge) tile by ordinary matching.
- **Eraser**: acts on the tile under the pointer, not a corner, and doesn't paint the Default terrain (contrary to the ITP documentation). It clears the crossers on that tile's edges and re-picks it and the tiles across the cleared edges. A tile that then fits nothing loses its other kinds of crosser (erasing a road beside a road/stream crossing removes the stream), or else all of them, outward. With no crossers it just re-picks the tile.
- Aurora's area view tracks the pointer only through motion events: after selecting a brush in the palette, a crosser drag starts from wherever the cursor last moved inside the view. The capture script glides onto every target.

### B.4b Groups, resizing and rotation (captured, 89.8193.37) [V]
- **Groups**: the ghost's first tile (Tile0, south-west) sits under the pointer. Each right click turns it a quarter counter-clockwise about that tile. A click places it and clears the brush. The tiles go in at the height of the ground under the first tile (min of its corners: on a plateau Tile_Height 1, over a single raised corner 0, which flattens that corner). Their corners and edges replace the terrain (a water corner on the group's edge becomes grass), and the tiles around whose corners changed are re-picked.
- **Doors**: a placed tile whose door hook has a type ≠ 0 gets a door. Aurora builds it from doortypes.2da `TemplateResRef` as it places a door from the palette, sets `Appearance` = the type, and places it at the hook (barn: `nw_door_ttr_04`, tag `Barn1Door`). Generic hooks (type 0) get none.
- **Resize Area** (rows, columns, Tiny/Small/Medium/Large; anchored at the south-west):
  - Growing continues the terrain at the old edge (new corners copy the nearest old corner, new edges the nearest old edge across) and picks the new tiles at random among the fits. Edge tiles are not copied as tiles.
  - Shrinking drops the north and east rows and columns and replaces what remains of groups the new edge cuts with terrain tiles (their doors go too). Then it warns "Some objects were deleted as a result of this operation. You may recover these objects by undoing the previous action."
- **Rotate Area** (CCW or CW 90/180/270): tile (x, y) of an area H high goes to (H − 1 − y, x), orientation + 1. Points (x, y) go to (H·10 − y, x). Door bearings turn +90° into [−π, π), so a door turned to face west reads −π.
- Not yet checked: Tile Properties' Defaults with a non-black scheme. Aurora's tile selection didn't respond to the capture script in an interior area.

### B.5 Area border
- "Border" terrain is the terrain of boundary vertices [I from name; D Tilesets "default terrain type used around the border of your map"]. It is unclear whether the toolset forces boundary vertices to Border or only initialises them [?].
- Beyond the area the **game** (not the ARE) draws edge tiles 5 tiles deep (50 m), using `<tileset>_edge.2da` [D Tutorial, Render Distance page]. Columns: `Corner1, Edge, Corner2, Height, Model` [V].
  Example ttr01_edge rows: `Grass **** Water 0 TTR01_Z05_01`, `Grass+ **** Grass 0 TTR01_Z06_01` (`+` = raised corner), `Grass+ **** Grass+ 1 TTR01_Z01_01`, `Grass Road Grass 0 TTR01_Z12_01` (a crosser leaving the map), `Grass **** **** 0 TTR01_Z01_01` (one-corner piece).
  Inference [I]: for each boundary edge, look up (boundary vertex 1, crosser on the boundary-perpendicular edge, boundary vertex 2). `Corner2=****` rows serve outer area corners, and Height is the height level of the edge piece. Corner1/Corner2 order and orientation are [?].
  Every base set ships an `_edge.2da` (EE added dag01, tcm02, trm02, trs02, ttf02, tts02 in 1.81.8193.16 and tss13 in 1.81.8193.17/1.82.8193.20) [V, D]. `ReloadAreaBorder()` and `SETTILE_FLAG_RELOAD_BORDER` exist [D]. Whether the Aurora area view draws edge tiles is [?].

---

## C. ARE / GIT tile data

### C.1 ARE top-level fields that matter here [V from dumps; D BioWare ARE PDF]
`Tileset` (CResRef, the .set resref), `Width` (INT, tiles along X/east), `Height` (INT, tiles along Y/north), `Tile_List` (List of StructID 1, length W·H), `Flags` (DWORD: 0x1 interior, 0x2 underground, 0x4 natural), `LightingScheme` (BYTE, environment.2da), `Version` (DWORD save counter; **missing in all 31 Furiae areas**, present in 410 others), `Creator_ID`/`ID` (deprecated; observed -842150451 = 0xCDCDCDCD in Prelude), `Expansion_List` (deprecated, empty), plus weather, fog, sun/moon colours (BGR DWORD), `SkyBox`, `FogClipDist` (FLOAT, EE), `LoadScreenID`, scripts, `Tag`, `ResRef`, `Name`, `Comments`.

### C.2 `Tile_List` struct (StructID 1) as written [V]
| Field | GFF type (actual) | Meaning |
|---|---|---|
| Tile_ID | INT | Index of `[TILEn]` in the SET |
| Tile_Orientation | INT | 0–3, CCW quarter turns |
| Tile_Height | INT | Base height in Transition steps, ≥0 (observed 0–19) |
| Tile_MainLight1/2 | BYTE | lightcolor.2da row 0–31 (0 = off/black) |
| Tile_SrcLight1/2 | BYTE | 0 = off, 1–15 = colour/animation (see E) |
| Tile_AnimLoop1/2/3 | **BYTE** (the PDF says INT) | 1 = play `animloop0n`. Official data has 1 even on tiles without the animation, although the PDF says the field should be 0 unless the animation exists |
- **Index → cell**: `x = i % Width`, `y = i / Width` [D ARE PDF, nwscript SetTileJson comment, V]. Row-major from the **south-west (bottom-left)**; x grows east, y grows north. Example for 3×3: top row `6 7 8`, bottom row `0 1 2`.
- **World coordinates** [V via doors]: tile (x,y) centre = (10x+5, 10y+5). Tile base Z = Tile_Height × Transition. The area spans [0,10W]×[0,10H].
- **Door hookpoint → GIT door** [V: 1,854 doors match to <1e-5 m]: `pos = centre + Rz(90°·o)·(X,Y) + (0,0, Tile_Height·Transition + Z)`. `Bearing = radians(DoorOrientation + 90·o)`, or +π when the builder flipped the door (413 of 1,854). Appearance = SET Type when Type≠0. Type 0 gives Appearance 0 with GenericType chosen by the builder (the GIT uses `GenericType_New` in EE). A door moved from a generic to a unique hook keeps its GenericType [D Door PDF].
- Max size: the area wizard offers up to 32×32 [D Area page, Tutorial "maximum area size is 32 wide or long"]. The largest official area seen is 30×28 [V]. No EE change to the maximum is documented [?].
- Scripted tile changes (EE 87.8193.35): `SetTile(loc, id, orientation, height, flags)`, `SetTileJson(area, [{index,tileid,orientation,height,animloop1..3}], flags, sTileset)`, `GetTileID/Orientation/Height`, `SetTileAnimationLoops`, `ReloadAreaGrass`, `ReloadAreaBorder`. Flags: `SETTILE_FLAG_RELOAD_GRASS=1`, `RELOAD_BORDER=2`, `RECOMPUTE_LIGHTING=4` [D nwscript.nss].
  Older API: `SetTileMainLightColor/SetTileSourceLightColor/GetTileMainLight1Color…`, where "the vector part is the tile grid (x,y) coordinate" [D].

---

## D. Walkmesh and pathing

### D.1 Tile walkmesh `.wok` [V sample, D]
- ASCII text: `#MAXWALKMESH ASCII`, `beginwalkmeshgeom <MODEL>`, then `node aabb <name>` with `parent <MODEL>`, `position`, `orientation`, `wirecolor`, `verts N` (x y z), `faces N` (`v1 v2 v3 smoothgroup t1 t2 t3 material`), an `aabb` tree, `endnode`, `endwalkmeshgeom`.
  The vertex positions are relative to the node position (ttr01_a01_01: position z 2.5, verts ±2.5 → z 0 and 5 = Transition).
- AABB tree lines: `minx miny minz maxx maxy maxz faceIdx`, where faceIdx = -1 for inner nodes. Depth-first, left subtree before right. Neverblender's builder (`~/Projects/neverblender/neverblender/nvb_aabb.py`) splits on the longest axis at the average centroid.
- The MDL also embeds the AABB for single-player use. In network play the server's WOK is preferred [D Tutorial]. The aurorabase must be `classification tile` / Type Tile [D].
- Face material = surfacemat.2da row [D]. Rows: 0 NotDefined, 1 Dirt, 2 Obscuring, 3 Grass (tile grass), 4 Stone, 5 Wood, 6 Water (walkable), 7 Nonwalk, 8 Transparent, 9 Carpet, 10 Metal, 11 Puddles, 12 Swamp, 13 Mud, 14 Leaves, 15 Lava, 16 BottomlessPit, 17 DeepWater, 18 Door, 19 Snow, 20 Sand, 21 Barebones, 22 StoneBridge, 23–29 Temp, 30 Trigger.
  Columns: `Label, Walk, WalkCheck, LineOfSight, Sound, Name (footstepsounds column), IsWater, Visual, Act1..8_Strref/Icon`. Up to 64 rows since 1.84.8193.29 (was 32). Hak-overridable since 1.85.8193.30 [D surfacemat.2da].
- Rules [D Tutorial, Walkmesh Notes]: at most 8 faces per vertex; no overlapping or duplicate or degenerate faces; walkable faces point up; one AABB node, parented to the aurorabase, at (0,0,0), with no children; no vertical faces (offset ≥1 cm); edges exactly at ±5.0 so they meet the neighbour. A missing walkmesh or gap makes creatures float at 15 m with material 0.
- Moonglow needs walkmeshes for object Z snapping, walkability display and surface materials [I]. EE debug: `renderaabb`, `rendertilepathnodes` [D].

### D.2 Door walkmesh `.dwk` [D DWK, V sample t_door01.dwk]
- ASCII, `#NWmax DWKMESH ASCII`. Nodes are parented to `<Model>_DWK`: dummy `XX_DWK_dp_closed_01/_02`, `XX_DWK_dp_open1_01`, `XX_DWK_dp_open2_01` (use points), and trimesh `XX_DWK_wg_closed`, `XX_DWK_wg_open1`, `XX_DWK_wg_open2` (blockers per state).
- The real file has `closed_01` with an underscore, where the wiki writes `closed01`.
- A closed door's DWK blocks the doorway. Opening hands control back to the tile WOK and changes path nodes and visibility (DoorVisibilityNode) [D].

### D.3 Placeable walkmesh `.pwk` [D PWK, V sample plc_a01.pwk]
`#MAXDOOR ASCII`, trimesh `<MODEL>_wg` (material 7) plus dummies `<model minus 4 chars>_pwk_use01/use02` (only use01 and use02 are read).

### D.4 Tile path nodes [D Tile Path Nodes, engine source quoted there]
- Coarse graph used by long-distance pathing; a pathnode says "you can/can't go from this exit to that exit". Tile coordinates run from (-5,-5) to (5,5). WASD movement ignores them.
- Per type: region nodes (x,y), exits (x,y), and the region id of each exit. Exits in the same region are connected. Types: A–Z and a–x.
  f–l were added in 1.6x; m–p in 1.69; **q–x in EE 87.8193.35** [D patch notes].
- Rotated by SET `Orientation` (degrees CCW) [D]. Whether the effective rotation also adds `Tile_Orientation` is [?], but it almost certainly does [I].
- Compact table (regions; exits as (x,y)r=region), from the wiki's engine code:
```
A 1 (0,0) | (0,5)r0 (-5,0)r0 (5,0)r0 (0,-5)r0
B 2 (4,4)(-2.5,-2.5) | (0,5)r0 (-5,2.5)r0 (-5,-2.5)r1 (-2.5,-5)r1 (2.5,-5)r0 (5,0)r0
C 2 (-2.5,0)(2.5,0) | (-2.5,5)r0 (-5,0)r0 (-2.5,-5)r0 (2.5,-5)r1 (5,0)r1 (2.5,5)r1
D 4 | (-2.5,5)r0 (-5,2.5)r0 (-5,-2.5)r1 (-2.5,-5)r1 (2.5,-5)r2 (5,-2.5)r2 (5,2.5)r3 (2.5,5)r3
E 3 | (-2.5,5)r0 (-5,2.5)r0 (-5,-2.5)r1 (-2.5,-5)r1 (2.5,-5)r2 (5,0)r2 (2.5,5)r2
F 1 | (-2.5,5) (-5,0) (-2.5,-5) (2.5,-5) (5,0) (2.5,5) all r0
G 1 | (-2.5,5) (-5,0) (0,-5) (5,0) (2.5,5) all r0
H 1 | (-5,0) (0,-5)          I 1 | (0,5) (-5,0) (0,-5)
J 2 | (-5,0)r0 (0,-5)r0 (5,0)r1 (0,5)r1
K 3 | (-2.5,5)r0 (-5,0)r0 (0,-5)r1 (5,-2.5)r1 (5,2.5)r2 (2.5,5)r2
L 1 | (0,5) (0,-5)          M 2 | (-2.5,5)r0 (-2.5,-5)r0 (2.5,5)r1 (2.5,-5)r1
N 1 | (0,5)                 O 3 | (-2.5,5)r0 (-5,2.5)r0 (-5,-2.5)r1 (-2.5,-5)r1 (2.5,-5)r2 (5,-2.5)r2 (5,2.5)r1 (2.5,5)r1
P 0 (no exits)            Q 2 | (-2.5,5)r0 (-5,0)r0 (0,-5)r1 (5,0)r1 (2.5,5)r1
R 2 | (-2.5,-5)r0 (-5,0)r0 (0,5)r1 (5,0)r1 (2.5,-5)r1
S 2 | (-2.5,5)r0 (-5,0)r0 (2.5,5)r1 (5,0)r1      T 0 (no exits)
U 2 | (-5,0)r0 (0,-5)r1 (0,5)r1 (5,0)r1          V 2 | (0,5)r0 (0,-5)r1 (5,0)r1
W 2 | (0,5)r0 (0,-5)r1 (-5,0)r1                  X 2 | (0,5)r0 (-5,0)r1
Y 2 | (0,5)r0 (0,-5)r1                           Z 1 | (-5,2.5) (-5,-2.5) (-2.5,-5) (2.5,-5) (5,0) (0,5)
a 2 | (-2.5,5)r0 (-5,0)r0 (0,-5)r0 (5,0)r0 (2.5,5)r1
b 2 | (-2.5,5)r0 (-5,0)r1 (0,-5)r1 (5,0)r1 (2.5,5)r1
c 1 | (-4,5) (-5,0) (0,-5) (5,0) (4,5) (0,5)
d 2 | (-4,5)r0 (-5,0)r0 (0,-5)r0 (5,0)r0 (4,5)r0 (0,5)r1
e 3 | (0,5)r0 (-5,4)r0 (-5,0)r1 (-5,-4)r2 (-4,-5)r2 (0,-5)r1 (4,-5)r0 (5,0)r0
f 3 | (-4,5)r2 (-5,0)r2 (-4,-5)r2 (0,-5)r1 (4,-5)r0 (5,0)r0 (4,5)r0 (0,5)r1
g 2 | (-4,5)r1 (-5,0)r1 (-4,-5)r1 (0,-5)r1 (4,-5)r0 (5,0)r0 (4,5)r0 (0,5)r1
h 2 | (0,5)r0 (-5,4)r0 (-5,0)r0 (-5,-4)r1 (-4,-5)r1 (0,-5)r0 (4,-5)r0 (5,0)r0
i 3 | (0,5)r1 (-5,4)r1 (-5,0)r2 (-5,-4)r1 (0,-5)r1 (5,-4)r1 (5,0)r0 (5,4)r1
j 4 | (0,5)r1 (-5,4)r1 (-5,0)r2 (-5,-4)r3 (0,-5)r3 (5,-4)r3 (5,0)r0 (5,4)r1
k 4 | (-4,5)r2 (-5,4)r2 (-5,0)r1 (-5,-4)r3 (-4,-5)r3 (0,-5)r1 (4,-5)r0 (5,0)r0 (4,5)r0 (0,5)r1
l 5 | (-4,5)r2 (-5,4)r2 (-5,0)r0 (-5,-4)r3 (-4,-5)r3 (0,-5)r0 (4,-5)r4 (5,-4)r4 (5,0)r0 (5,4)r1 (4,5)r1 (0,5)r0
m 5 | like l without (-5,0),(5,0)
n 6 | (-4,5)r2 (-5,4)r2 (-5,-4)r3 (-4,-5)r3 (0,-5)r4 (4,-5)r5 (5,-4)r5 (5,4)r1 (4,5)r1 (0,5)r0
o 4 | (-4,5)r2 (-5,4)r2 (-5,-4)r3 (-4,-5)r3 (0,-5)r1 (4,-5)r0 (5,0)r0 (4,5)r0 (0,5)r1
p 4 | (-4,5)r2 (-5,4)r2 (-5,0)r1 (-5,-4)r3 (-4,-5)r3 (4,-5)r0 (5,0)r0 (4,5)r0
q 3 | (-4,5)r1 (-5,4)r1 (-5,0)r0 (-5,-4)r2 (-4,-5)r2 (4,-5)r0 (5,0)r0 (4,5)r0
r 8 | each of the 12 exits in its own region except corner pairs: (-4,5)(-5,4)r0 (-5,0)r1 (-5,-4)(-4,-5)r2 (0,-5)r3 (4,-5)(5,-4)r4 (5,0)r5 (5,4)(4,5)r6 (0,5)r7
s 7 | as r without (0,5)
t 5 | (-4,5)(-5,4)r1 (-5,-4)(-4,-5)r2 (0,-5)r0 (4,-5)(5,-4)r3 (5,0)r0 (5,4)(4,5)r4
u 5 | as t plus (0,5)r0
v 6 | (-4,5)(-5,4)r2 (-5,0)r1 (-5,-4)(-4,-5)r3 (0,-5)r0 (4,-5)(5,-4)r4 (5,0)r0 (5,4)(4,5)r5 (0,5)r0
w 4 | (0,5)r0 (-5,0)r1 (5,0)r2 (0,-5)r3
x 3 | (-2.5,5)(-5,2.5)r0 (-5,-2.5)(-2.5,-5)r1 (2.5,-5)(5,-2.5)r2
```
  The wiki says its ASCII art is not authoritative. For exact region-node coordinates use `$SP` script output or the wiki page. Base-data usage: A 3702, I 1655, H 1124, B 765, N 745 … [V].
- Visibility nodes use the same types and govern line of sight across tiles [D].

---

## E. Tile lights, animations, sounds

- **Main lights** [D ARE PDF, Model Special Nodes]: a light node named `<MODEL>ml1` / `<MODEL>ml2` (case-insensitive) in the tile model; real models use `TTR01_A01_01ml1` [V]. If the node is absent the toolset disables the control.
  `Tile_MainLight*` indexes `lightcolor.2da` rows 0–31 (Black, DimWhite, White, BrightWhite, PaleDarkYellow…Orange), which match the `TILE_MAIN_LIGHT_COLOR_*` constants 0–31 [V].
  The game uses RED/GREEN/BLUE (0–2); the toolset colour picker uses TOOLSETRED/GREEN/BLUE (0–1). Adding rows to lightcolor.2da breaks the toolset Tile Properties ("List Index Out of Bounds") [D 2da Files, Common Errors].
  The engine overrides ml1 with shadow 1, affectdynamic, priority 4, shadowradius 12, radius 10; ml2 gets radius 5, shadowradius 8 [D Model Special Nodes].
- **Source lights**: a dummy `<MODEL>sl1/sl2`. The engine spawns `fx_flame01.mdl` there and plays the animation named after the value ("1"–"15"). The colour shown is lightcolor.2da row **2·value** (e.g. 1 → White row 2), matching `TILE_SOURCE_LIGHT_COLOR_*` 0–15. lightcolor.2da does not change the engine's flame colour [D ARE PDF, V constants].
- tilecolor.2da (16 RGB rows) is "used in the toolset, not by the game" [D 2da Files]. Its purpose is unknown (terrain colours on the minimap or selector?) [?]. The tutorial's claim that lights come from tilecolor.2da contradicts the BioWare PDF.
- Toolset tile-light default colours come randomly from environment.2da `MAIN1_COLOR1..4` / `MAIN2_COLOR1..4` / `SECONDARY_COLOR1..4` for the area's lighting scheme, both when painting and on area creation [D ARE PDF].
- **Animations** on the tile model: `animloop01`, `animloop02`, `animloop03` (toggled by Tile_AnimLoop1..3 / SetTileAnimationLoops), `day`, `night`, `day2night`, `night2day` (run at dusk and dawn), `tiledefault` [D Animations page, ARE PDF uses "AnimLoop01"].
  The Tutorial's "animloop1" is wrong (two-digit form confirmed by the PDF and the EE patch notes "animloop01 toggle").
  Animated geometry must hang under an "a-node" dummy `<MODEL>a` parented to the tile base, because tile trimeshes are otherwise static. A-nodes returned in 8193.35 [D].
- Other tile-model features: `tilefade 0/1/2/3` per mesh (off/fade/base/neighbour) with toolset "Tile Fade" UI modes, and `rotatetexture 1` (UVs counter-rotated with the tile so ground textures don't seam). The EE 85.8193.32 fix makes it work with normal maps [D MDL ASCII, patch notes].
- **Tile sounds**: `Sounds=` is always 0 in base data and the sub-format is unknown [?]. Area ambient and music defaults come from areag.ini per tileset [D].
- Toolset: after changing tile light properties the static lighting is recalculated (1.80.8193.14). F5 recomputes shadows [D].

---

## F. Minimap and palette (ITP)

### F.1 Minimap [V/D]
- Each tile's `ImageMap2D` TGA is drawn in its cell, rotated with the tile [I]. Sizes are 16×16 or 32×32, 24-bit. Toolset and game read the SET value; don't derive the name.

### F.2 Tileset palette `<tileset>palstd.itp` (GFF "ITP ") [D BioWare ITP PDF §3, V 33 files]
- The top-level struct has `MAIN` (list of category structs). Each category has `ID` (BYTE), `STRREF` (DWORD), `LIST`.
  - ID **0 = Features** (STRREF 63261): leaves `RESREF` = model of Tile0 of a 1-tile group.
  - ID **1 = Groups** (STRREF 63262): leaves `RESREF` = model of Tile0 of a multi-tile group.
  - ID **2 = Terrain** (STRREF 8282): leaves `RESREF` = lower-case terrain or crosser name. Terrains are searched before crossers, first match wins. The special leaves are `eraser` (STRREF 63291) and `raiselower` (63292, only if HasHeightTransition).
- Leaves carry `RESREF` plus `STRREF`, or `RESREF` plus `NAME` (CExoString, EE 1.80.8193.6+). STRREF overrides NAME [D].
- Base data [V]: 1,681 leaves use RESREF+STRREF and 615 use NAME+RESREF. **Subfolders** (struct with `LIST` + `STRREF`, no ID) appear inside the Feature and Group branches of tcm02, trm02, trs02, tts02 and tss13 (nested 2 deep).
  tno01 has no ID 0/1 branches. Instead it has ID 2 plus top-level branches **ID 3, 4, 5, 6**, which mix features and groups by theme. tib01's ID 0 branch has no LIST.
  So the toolset evidently resolves any non-terrain leaf by "find the group whose Tile0 model == RESREF", whatever the branch ID [I].
- Cross-check [V]: every non-terrain leaf in base palettes resolves to a group Tile0 model, except about 12 leaves that name a non-group model (e.g. tdc01_a02_07, tno01_m40_11) or a missing model (trm02_r22_07, trs02_r22_07). Those are dead entries or single-tile placement [?].
  All terrain leaves resolve to a SET terrain or crosser name. dag01 lists `raiselower` although HasHeightTransition=0.
- `TILESETRESREF` exists only in the skeleton palettes (`<ts>pal.itp`) [D].

### F.3 Checking the "Claude Code" wiki subtree (SET File Format / Tileset Operations / ITP Palette Format, 2026-03)
| Claim | Verdict |
|---|---|
| ImageMap2D is "always mi_ + model name" | **False** for 3,825 of 12,342 base tiles (`mitr01_A01`, `MICN01_B08`, `mall_a01`, `mi_temp01`) |
| Primary/secondary rules "not used in practice… set Count=0" | **Misleading**. 23 base sets use primary rules (up to 192). Only SECONDARY is always 0. Rules matter for the Aurora-compatible paint behaviour |
| ITP subfolder `ID=2` = paint folder, `ID=3` = placement-only folder "(#1 cause of failure)" | **Not supported by base data or the BioWare spec**. Base subfolders carry no ID, IDs 0/1/2 are the top-level categories, and tno01 uses top-level IDs 3–6 as ordinary feature/group branches. The claim may describe one custom tileset; treat as unverified |
| "Door Type is an index into doortypes.2da" | True; 0 = generic hookpoint |
| DoorVisibilityNode "required when Doors > 0" | **False in practice**: 2,373 door tiles in base data lack it |
| Border terrain must be shared across "families" | Plausible (Border/Default hold single names) but unverified [I] |
| Reorder tiles alphabetically and renumber | **Dangerous**: ARE Tile_ID stores indices, so renumbering breaks every existing area. No base SET is sorted |
| Every Model must have .mdl and .wok | True as a rule; 2 base tiles break it (tcm02_b92_02, trs02_m00_00) |
| SelectorHeight optional | True; it defaults to Transition [D 1.69] |
| MDL/WOK internal names include the model name, lights `…ml1/ml2` | True [V] |
| Section order; `Count` must match | Consistent with other sources |
Other wiki inconsistencies:
- nwn.wiki SET page: group Tile0=-1 "crashes the toolset", yet tcm02 GROUP72 has it.
- Duplicate Tile0: the ITP PDF says the first group wins, the wiki says the last.
- The Tutorial says doortypes rows are "1-based". In fact row 0 is "Generic", so tileset doors start at 1.
- The Tutorial names the light colour table tilecolor.2da; the PDF and constants say lightcolor.2da.
- ARE PDF says Tile_AnimLoop is INT; the files hold BYTE.

---

## G. EE changes relevant to tilesets (by version)
- 1.69 and earlier (baseline): GrassTextureName; SelectorHeight; path nodes f–l and m–p; tileset limit 24 → 50 → 100; group-painting locks interior tiles (3×3+ groups with null tiles); load-time group validation; `renderaabb`/`rendertilepathnodes` debug; edge 2DA typo fix [D 1.69 page].
- 1.74.8156: `GetSurfaceMaterial`, `GetGroundHeight`. 1.74.8154: tile exploration functions [D].
- 1.79.8193: memleak fix loading tile walkmeshes. 1.79.8193.1: Tile Fade UI fixed [D].
- 1.80.8193.6: ITP `NAME` CExoString instead of STRREF for custom tileset palettes [D].
- 1.80.8193.7–.13: mass tileset repairs (walkmeshes, drive-through transitions, doorcap feature tiles for TNI01/TNI02, animloops so animations can be shut off, TNO01.SET pathfinding/orientation/visibility fixes, TTZ01_EDGE entry); tile light colour picker fix [D].
- 1.80.8193.14 / 1.81.8193.15: lighting and grass rendering overhaul; DoD/ToM content; new tilesets dag01, tcm02, trm02, trs02, ttf02, tts02, tss13; DAG01.set height fix; toolset recomputes static lighting after tile light edits [D].
- 1.81.8193.16/.17, 1.82.8193.20: new `_edge.2da` files (dag01, tcm02, trm02, trs02, ttf02, tts02, tss13) [D].
- 1.84.8193.29: surfacemat.2da up to 64 rows plus the OnPlayerTileAction radial. 1.85.8193.30: surfacemat.2da hak-overridable [D].
- 85.8193.32: tile shadow volume clipping uses the tile bounding box; `rotatetexture` works with normal/displacement maps; toolset "second story tile fade Always" fix; crash fix right-clicking a tile [D CHANGELOG].
- 85.8193.33: tile source lights shown red after save/load fixed [D].
- 86.8193.34.1: facelift tts02/tcm02 fixes [D].
- **87.8193.35**: 8 new path nodes **q–x**; `GetTileID/Orientation/Height`, `SetTile`, `SetTileJson`, `SetTileAnimationLoops`, `ReloadAreaGrass`, `ReloadAreaBorder`; rock/chasm crosser in trm02; missing tcm02 tiles; refined path node debug render; A-nodes working again [D CHANGELOG, Tutorial].
- 88.8193.36: fix for malformed tile walkmesh causing levitation [D].
- **89.8193.37-13**: modules no longer limited to 100 in-use tilesets; toolset standard palette tileset-specific door list fixes (names, refresh on area tab change, no duplicate "Door" rows); tileset kept in memory in MP [D CHANGELOG].
- No documented change to area size limits or the SET/ARE formats in EE beyond the above [V grep].

---

## H. Citations
nwn.wiki (local mirror; URLs from front matter):
- SET: https://nwn.wiki/spaces/NWN1/pages/38175567/SET
- Adding Doors to a Tile: https://nwn.wiki/spaces/NWN1/pages/38175683/Adding+Doors+to+a+Tile
- Tilesets: https://nwn.wiki/spaces/NWN1/pages/38175063/Tilesets
- Tileset Construction Tutorial: https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial
- Common Issues with Tiles and Tilesets: https://nwn.wiki/spaces/NWN1/pages/72417367/Common+Issues+with+Tiles+and+Tilesets
- Tile Path Nodes: https://nwn.wiki/spaces/NWN1/pages/139689996/Tile+Path+Nodes
- Walkmesh Notes: https://nwn.wiki/spaces/NWN1/pages/179077218/Walkmesh+Notes
- Community Tileset Prefixes: https://nwn.wiki/spaces/NWN1/pages/65470588/Community+Tileset+Prefixes
- Area Tilesets Information: https://nwn.wiki/spaces/NWN1/pages/38175219/Area+Tilesets+Information
- Area: https://nwn.wiki/spaces/NWN1/pages/38174902/Area
- Area Editor: https://nwn.wiki/spaces/NWN1/pages/60982635/Area+Editor
- Advanced Area Creation Tips: https://nwn.wiki/spaces/NWN1/pages/26738846/Advanced+Area+Creation+Tips
- Area Transitions: https://nwn.wiki/spaces/NWN1/pages/38176214/Area+Transitions
- Resource Limits: https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits
- Day/night tile animations: https://nwn.wiki/spaces/NWN1/pages/60984838
- MDL ASCII: https://nwn.wiki/spaces/NWN1/pages/12027273/MDL+ASCII
- DWK: https://nwn.wiki/spaces/NWN1/pages/91324443/DWK
- PWK: https://nwn.wiki/spaces/NWN1/pages/60985376/Placeable+Walkmesh+PWK
- surfacemat.2da: https://nwn.wiki/spaces/NWN1/pages/38176553/surfacemat.2da
- surfacemat uneditable: https://nwn.wiki/spaces/NWN1/pages/38174817
- areag.ini: https://nwn.wiki/spaces/NWN1/pages/72417335/areag.ini
- Model Special Nodes: https://nwn.wiki/spaces/NWN1/pages/38176272/Model+Special+Nodes
- Animations: https://nwn.wiki/spaces/NWN1/pages/38175170/Animations
- 2da Files: https://nwn.wiki/spaces/NWN1/pages/38174875/2da+Files
- Common Errors: https://nwn.wiki/spaces/NWN1/pages/38175555
- Render Distance: https://nwn.wiki/spaces/NWN1/pages/38175000
- Placeables and the Toolset: https://nwn.wiki/spaces/NWN1/pages/26738852
- Vault Tilesets: https://nwn.wiki/spaces/NWN1/pages/129237024/Vault+Tilesets
- Tileset page (prefix and name): https://nwn.wiki/spaces/NWN1/pages/179077122
- 1.69 notes: https://nwn.wiki/spaces/NWN1/pages/38174729/1.69
- "Claude Code" subtree: SET File Format https://nwn.wiki/spaces/NWN1/pages/195362833/SET+File+Format, Tileset Operations https://nwn.wiki/spaces/NWN1/pages/195362837/Tileset+Operations, ITP Palette Format https://nwn.wiki/spaces/NWN1/pages/195362835/ITP+Palette+Format

BioWare specs (github.com/kucik/nwn-docs; also nwn.wiki attachments 327727): Bioware_Aurora_AreaFile_Format.pdf, Bioware_Aurora_PaletteITP_Format.pdf, Bioware_Aurora_DoorPlaceableGFF.pdf. BioWare "Aurora Neverwinter Nights Toolset Module Construction Tutorial" (amethyst-dragon.com/neverwinternightsinfo/Downloads/AuroraToolsetTutorial.pdf). Vault: "Module Mapping 102" (neverwintervault.org/article/tutorial/module-mapping-102-basic-terrain-mapping-aurora-neverwinter-nights-toolset).
Game docs: `$G/lang/en/docs/CHANGELOG.md`, `patchnotes/*.md`, `Neverwinter Nights Enhanced Edition (v7x).txt`. nwscript.nss via `nwscript-docs`.

---

## I. Open questions / reverse-engineering checklist (priority order)
1. Terrain paint algorithm: neighbourhood for primary rules (4 or 8 vertices), recursion/propagation depth, the order rules apply in, how relative rule heights relate to absolute vertex heights, and the fallback when no tile matches (refuse, red cursor, or force Default).
2. Crosser drag semantics: which edges get set on start, middle and end tiles; whether terrain painting preserves crossers; bridge/crossing selection; the invalid-drag rule.
3. Raise/Lower footprint and propagation (one vertex or several; multi-level stacking; the interaction with rules when heights differ by more than 1).
4. RNG: distribution over (tile variant × orientation), and whether orientations of symmetric tiles are deduplicated. Shift-cycle order.
5. New-area seed: Border ring vs Default fill, the centre Floor patch size, initial light colours (environment.2da MAIN*_COLORn).
6. Group placement pivot under rotation in the UI; validity checks against neighbours and heights; behaviour at area edges; partial-group deletion.
7. Border/edge: whether the toolset forces boundary vertices to `Border`; exact `_edge.2da` lookup (corner order, `+` and Height meaning for multi-level heights); whether Aurora renders edge tiles.
8. SECONDARY RULES semantics and the unused keys (`DefaultAreaProperties`, `Image`, `ReplaceTexture`, SOUND sections, `Sounds>0` format).
9. Door hookpoints when `Doors` ≠ number of `[TILEnDOORm]` sections: which one wins.
10. Duplicate group Tile0 in the ITP: first or last group; handling of the tcm02 group with Tile0=-1; ITP leaves that point at non-group models (single-tile placement?).
11. tilecolor.2da use in the toolset.
12. PathNode effective rotation = SET Orientation + Tile_Orientation (probably).
13. Area size limit in the EE wizard (1..32?) and whether the engine accepts larger areas.
Method: run nwtoolset.exe under Wine with scripted input on small areas of ttr01 (rules + heights), ttz01 (no rules), tic01 (rooms/walls) and tdm01 (crossers), then diff ARE Tile_List against Moonglow using the verified formulas above (index, rotation, heights).
