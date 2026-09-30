# Moonglow Toolset: NWN:EE technical research brief

- **Target:** NWN:EE **89.8193.37-17** (Steam, Linux install; `bin/win32/build.txt` commit 26c6e57, 2025-10-06). Compiled 2026-09-30.
- **Evidence tags:**
  - **[V]** verified against shipped game data or binaries in this research
  - **[W]** nwn.wiki (local mirror; URLs are in each section and in Appendix D)
  - **[CL]** the game's changelogs (`lang/en/docs/CHANGELOG.md`, `patchnotes/*.md`, `Neverwinter Nights Enhanced Edition (v74..v79).txt`, `legacy/NWNv169.txt`)
  - **[T]** strings in `nwtoolset.exe` / `nwmain` / `nwserver`
  - **[I]** inference (not verified)
- **Detail notes** (same folder; this brief condenses them):

  | File | Contents |
  |---|---|
  | `notes_gff_changelog.md` | GFF fields in toolset write order, struct IDs, and about 150 dated EE toolset changelog items |
  | `notes_tilesets.md` | SET keys, verification statistics, path-node table |
  | `notes_models.md` | binary MDL byte layout, animation tables, PLT, textures |
  | `notes_shaders.md` | full uniform tables and lighting math |
  | `draft_mine.md` | draft of sections 3, 7 and 8 |

- **Extracts and scripts** (all under the scratchpad, `../`):
  - `shaders/` (92 effective `.shd`)
  - `tilesets/` (33 SETs, palettes, edge 2DAs)
  - `models/` (decompiled ASCII, PLT decodes)
  - `gff/union.json` (every field path × type in stock modules)
  - `scripts/mdlbin.py` (binary MDL reader, validated on all 25,597 base binaries)

---

## 0. Decision-relevant findings

1. **The Aurora toolset renders with the game's own GLSL.**
   - It is still a 32-bit Windows VCL app (`nwtoolset.exe` is PE32).
   - It loads `.shd` resources through ResMan (restype 2069) and prepends the same `#define` preamble as `nwmain` [T]. The pairs it uses are `vslit/fslit`, `*_nm`, `*_sm`, `vsvt/fst`, grass, particles, shadow volumes and `fsfbpostpr`.
   - All 92 stock shaders ship as readable GLSL 330 source.
   - So Moonglow can compile the **stock shaders unchanged** with an emulated uniform/define set. That gives the best parity, and hak custom shaders and MTR `customshaderVS/FS` work for free. Porting the formulas in §6 is the fallback.
2. **ResMan priorities differ from 1.69 intuition.**
   - `development/` outranks everything.
   - Haks outrank the module.
   - `override/` sits **below** both haks and the module.
   - DDS and TGA interleave per location.
   - Copy the engine's numeric priority table (§3).
3. **The GFF container is unchanged** (V3.2, 16-char labels and resrefs, types 0–15).
   - EE added a small set of fields (§2.4):
     - WORD `x`-twins of body/armor part bytes (part ids up to 999)
     - GIT `VisTransformList`
     - DLG `ActionParams`/`ConditionParams`
     - 4 module event slots
     - `Mod_UUID`, `Mod_DefaultBic`, `Mod_PartyControl`
     - ARE `TileBrdrDisabled`
   - Moonglow must round-trip unknown and savegame-only fields losslessly.
4. **Tile painting.**
   - Every data-level rule is verified against 441 official areas:
     - index → cell mapping
     - CCW quarter-turn orientation
     - absolute corner heights
     - group layout and rotation
     - door hookpoint transform
   - The **interactive painting algorithm is undocumented**: rule propagation, raise/lower footprint, crosser drag and RNG. It has to be reverse-engineered by scripting Aurora under Wine and diffing `Tile_List` (checklist in §4.9).
5. **Models.**
   - 22% of base MDLs are **ASCII**, including player skeletons (`pmh0`…) and many EE tilesets. Moonglow needs ASCII and binary loaders feeding one data model.
   - The binary layout is fully mapped and validated on all 25,597 base binaries:
     - the node type comes only from `flags@+0x6C`
     - orientation is a quaternion
     - mesh alpha is controller 128
   - Animations bind **by name** through supermodel chains.
6. **Scripts.** The official compiler `libnwnscriptcomp` has a tiny C ABI (v1), ships for Linux/macOS/Windows on x64 and arm64, and is the same compiler the game and toolset use. It stops at the first error. Set optimisation flags = 1 for toolset-identical bytecode.
7. **F9.** Aurora runs `nwmain.exe -userdirectory "<dir>" +TestNewModule "<module>"`. This ports directly to `nwmain-linux` and the macOS app.
8. **Most known Aurora crashes come from unvalidated content**, e.g.:
   - SET counts that don't match the sections
   - missing tile, placeable or appearance models
   - extra `lightcolor.2da` rows
   - MTR texture names over 16 characters
   - baseitems.2da with more than 255 rows
   - open GitHub issues on hak reload, area rotate, white tile polygons and TLK strref clobbering

   Validate on load, degrade gracefully, and never lose data (§9).

---

## 1. File formats the toolset must read and/or write

Restype IDs are from nwn.py `res.py`.

| Format (restype) | Role | Moonglow | Oracle / validator | Notes |
|---|---|---|---|---|
| KEY (9999) / BIF (9998) `V1` | base-game index and archives | R | `nwn_key_unpack`, `nwn_key_shadows`, `nwn_resman_{grep,cat,extract,stats,diff}`; `nwn.key` | Two keys: `nwn_base.key` (113,483 res) and `nwn_retail.key` (93 res, added 37-13: newest 2DAs, 33 `.shd`, `nwscript.nss`, fonts) [V]. neverwinter.nim can also write `E1` (zstd) KEY/BIF; no shipped file uses it |
| ERF family `V1.0`: MOD 2011, HAK 2061, ERF 9997, NWM 2062, SAV 2057 | module container, haks, `.erf` export/import, SAV-as-MOD | R/W (.mod, .erf); R .hak (hak builder optional) | `nwn_erf`, `nwn.erf` (V1.0 only; default `max_entries=65535`), nasher | 160-byte header; key = 16-char resref + ResID + type. Aurora sorts keys by **upper-cased** resref and keeps legacy case (`Repute.fac`) [V]. `E1.0` strings exist in nwmain and nwtoolset, but no stock ERF uses E1: **don't emit**. No 16k-entry limit since 1.80.8193.14; keep haks < 2 GB (32-bit toolset) [W: Resource Limits] |
| Module folder `modules/<name>/`, `modules/temp0/` | open-as-directory (1.75+), work dir, `.BackupMod` | R/W | nasher | §3 |
| GFF `V3.2`: IFO ARE GIT GIC UTC UTD UTE UTI UTM UTP UTS UTT UTW DLG JRL FAC ITP BIC PTM PTT, `GFF ` (ExportInfo) | all authored data | R/W | `nwn_gff` (lossless JSON), `nwn_nwnt`, `nwn.gff` | nwn.py crashes on CExoLocString ids 256–263 (Korean/Chinese/Japanese, present in DoD and ToM). Use nim as the oracle there [V] |
| CAF (2082) | ARE+GIT combined (EE script JSON) | optional R | `nwn_gff` | not toolset-authored |
| 2DA `2DA V2.0` (2017) | all lookups | R (W if 2DA editing is added) | `nwn_twoda`, `nwn.twoda` | Lenient parsing: whitespace-separated; quoted cells ≤256 chars; `****` = blank; optional `DEFAULT:`; row labels ignored (rows are sequential) [W: 2da Files] |
| TLK `V3.0` (2018) | `lang/<xx>/data/dialog.tlk` (+`dialogf.tlk`), one custom TLK | R (+W for a custom-TLK editor) | `nwn_tlk`, `nwn.tlk` | custom strref = index + 0x01000000 [W: TLK] |
| SSF (2060) | soundset preview | R | `nwn_ssf`, `nwn.ssf` | 49 lines of resref + strref [W: SSF] |
| NSS (2009) | script source (cp1252 text) | R/W | – | stored in the .mod next to the NCS |
| NCS (2010) `NCS V1.0` | bytecode | W (via compiler) | `nwn_asm -d`, `nwn.nwscript.vm` | §7 |
| NDB (2064) `NDB V1.0` | debug info (text) | W (option) | `nwn.nwscript.ndb` | |
| SET (2013) | tileset (INI text) | R (W only for a tileset editor) | `nwn.tileset.read_set` (parses all 33 base sets; drops some keys) | §4 |
| MDL (2002) | all 3D; ASCII and binary | R (ASCII W optional) | Neverblender (ASCII); vendored **nwnmdlcomp** (binary→ASCII: `~/Projects/neverblender/build/third_party/nwn-tools`); `nwmain compilemodel <resref>` (ASCII→binary); prototype `scripts/mdlbin.py` | §5 |
| WOK (2016) / PWK (2053) / DWK (2052) | tile / placeable / door walkmesh (ASCII MDL subset) | R | Neverblender | §4.6 |
| TGA (3) | textures, icons, portraits, minimaps, `pal_*.tga` | R | Pillow / ImageMagick | Types 2, 10 (RLE), 3 (grey); 24/32 bpp; nearly all bottom-left; NPOT exists [V] |
| DDS (2033) | textures: BioWare 20-byte header (6,736 files) or standard `DDS ` (1 file) | R | texconv/ImageMagick for standard DDS; write your own BioWare decoder (DXT1/DXT5) | §5.6 |
| KTX (2073) | ETC2 (mobile) | optional R | – | none ship on desktop |
| PLT (6) `PLT V1  ` | layered colourable textures and icons | R | – | §5.5 |
| TXI (2022), MTR (2072) | texture and material directives (text) | R | – | §5.6 |
| SHD (2069) | GLSL shaders | R | glslang (compile check) | §6 |
| LOD (2078) | LOD chain (text) | ignore / optional | – | none ship |
| WAV (4), BMU (8) | sound/voice/ambient (WAV or MP3 with a `BMU V1.0` prefix); music (MP3) | R (preview) | ffprobe | [W: Sounds and Music] |
| WBM (2071) | movies (WebM) | reference only | – | module/area movie pickers |
| INI / TML | `areag.ini` (area-wizard defaults per tileset), `nwtoolset.ini`, `nwn.ini`, `settings.tml` | R | – | the toolset ignores `userpatch.ini` [W] |
| BIC (2015) | `Mod_DefaultBic`, vault characters (F9 picks the first) | R | `nwn_gff` | |
| ITP (2030) | palettes: skeleton, standard, custom, tileset | R/W (`*palcus.itp`) | `nwn_gff` | §2.2 |
| LTR (2036) | random-name Markov tables | optional | nwnltr | [W: LTR] |
| NWSync manifests / sqlite | server content | optional R | `nwn_nwsync_print`, resman tools via `nwsync-view.sh` | Toolset binary has an `NWSYNC` alias string; mounting is unconfirmed [T] |
| JUI, GUI, TTF, SQ3, SQL | runtime UI and DB | not needed | | |

Encoding:
- All GFF, 2DA, TLK and NSS strings are 8-bit, **windows-1252** for Western languages.
- EE 87 added `encoding.2da` (UTF-8 ↔ codepage mapping, used together with custom fonts) [W: encoding.2da].
- Aurora mangles some Polish letters (nwn-issues#534). Moonglow should be explicit about codepage per language.

---

## 2. GFF resource types authored by the toolset

Sources: real data (all 28 stock modules; the ground truth is *Doom of Icewind Dale*, the only module saved by the 89 toolset), toolset writer label runs [T], nwserver reader labels, and [CL]. Full tables in write order are in `notes_gff_changelog.md` §B. Wiki: https://nwn.wiki/spaces/NWN1/pages/38175366/GFF (it swaps the GIT/GIC descriptions). BioWare PDFs are linked at https://nwn.wiki/spaces/NWN1/pages/327727/File+Format+Specification+Bioware.

### 2.1 Container rules
- **Header:** 56 bytes. `FileType` (4 chars, e.g. `"IFO "`), `"V3.2"`, then 12 DWORDs (struct, field, label, field-data, field-index and list-index offset/count pairs).
- **Structs:** 12 bytes. Root struct ID = 0xFFFFFFFF.
- **Fields:** 12 bytes.
- **Labels:** 16 bytes, max 16 characters. The engine builds EE labels as `"x"+label` truncated to 16, e.g. `xArmorPart_Pelvi`.
- **Types 0–15:** BYTE, CHAR, WORD, SHORT, DWORD, INT, DWORD64, INT64, FLOAT, DOUBLE, CExoString, CResRef (≤16), CExoLocString, VOID, Struct, List. No EE additions.
- **CExoLocString language ids:** `lang*2+gender`. Real data has 0–5 and **256–263**.
- **Write order and struct IDs:**
  - The game ignores field order. Matching Aurora's order (in the notes) gives clean diffs.
  - List-element struct IDs are meaningful in some lists:
    - `Equip_ItemList` element ID = slot bit
    - UTM `StoreList` element ID = page
    - `Mod_Area_list` = 6, `Mod_HakList` = 8
    - GIT lists: creature 4, door 8, encounter 7, item 0, placeable 9, sound 6, store 11, trigger 1, waypoint 5
    - DLG node struct ID = index
- **Robustness:**
  - The ToM ERF contains an entry with an empty resref.
  - `Module.ifo` and `Repute.fac` appear capitalised.
  - Unknown labels must be preserved (nwn-issues#528: Aurora drops unknown GFF fields; Moonglow should not).

### 2.2 Per resource type: key fields
(B = BYTE, W = WORD, I = INT, DW = DWORD, F = FLOAT, RR = CResRef, Loc = CExoLocString, L = List.)

| Type | File / role | Key fields |
|---|---|---|
| **IFO** | `module.ifo` | `Mod_ID` (V16), `Mod_MinGameVer` ("1.89"; gates which toolset can open it), `Mod_Name`/`Mod_Description` (Loc; the description is mirrored into the ERF header), `Mod_Tag`, `Mod_CustomTlk` (name w/o ext), `Mod_Entry_Area/X/Y/Z/Dir_X/Dir_Y`, calendar (`Mod_DawnHour`, `DuskHour`, `MinPerHour`, `StartMonth/Day/Hour/Year`), `Mod_XPScale`, 18 legacy events (`Mod_OnHeartbeat` … `Mod_OnPlrChat`), **EE:** `Mod_OnPlrTarget`, `Mod_OnPlrGuiEvt`, `Mod_OnPlrTileAct`, `Mod_OnNuiEvent`, `Mod_DefaultBic`, `Mod_UUID`, `Mod_PartyControl`; `Mod_StartMovie`, `Mod_Area_list` {`Area_Name`}, `Mod_HakList` {`Mod_Hak` Str; **first = highest priority**}, `Mod_Expan_List`, `Mod_CutSceneList`, `Mod_GVar_List`, `VarTable`. Legacy: `Mod_Hak` (single), `Mod_CacheNSSList` (preserve, don't author). Savegame-only `Mod_*` fields: preserve |
| **ARE** | area static data | `Tag`, `Name`, `ResRef`, `Comments`, `Version` (save counter), `Flags` (1 interior, 2 underground, 4 natural), sun/moon `Ambient/Diffuse/FogColor` (DWORD **0x00BBGGRR**), `Sun/MoonFogAmount` (B 0–15), `Sun/MoonShadows`, `IsNight`, `DayNightCycle`, `LightingScheme` (environment.2da), `ShadowOpacity` (0–100), `FogClipDist` (F), `SkyBox` (skyboxes.2da), `ChanceRain/Snow/Lightning`, `WindPower`, `LoadScreenID`, `PlayerVsPlayer`, `NoRest`, `ModSpot/ListenCheck`, `OnEnter/OnExit/OnHeartbeat/OnUserDefined`, **`TileBrdrDisabled` (EE)**, `Tileset` (RR), `Width`, `Height`, `Tile_List` (struct 1: see §4.3) |
| **GIT** | area instances | root: `AreaProperties` (struct 100: `AmbientSndDay/Night(+Vol)`, `EnvAudio`, `MusicBattle/Day/Night/Delay`), `Creature List`, `Door List`, `Encounter List`, `List` (ground items), `Placeable List`, `SoundList`, `StoreList`, `TriggerList`, `WaypointList`, `VarTable` (= **area** local variables) |
| **GIC** | toolset comments | same 9 lists, one struct per GIT instance in the same order: `Comment`; sounds add `PlayInToolset`; encounter structs are empty (their comment lives in the GIT) |
| **UTC** | creature | `TemplateResRef`, `FirstName`/`LastName`, `Appearance_Type` (W), `Gender`, `Phenotype` (I), `PortraitId`, `Tag`, `Conversation`, `FactionID`, `Wings_New`/`Tail_New` (DW), `SoundSetFile`, flags (Plot, IsImmortal, Lootable, NoPermDeath, Disarmable, Interruptable), `BodyBag`, `DecayTime`, body parts (B) **each followed by an EE WORD `x…` twin** (`ArmorPart_RFoot` [sic], `BodyPart_*`, `Appearance_Head`), `Color_Skin/Hair/Tattoo1/2`, abilities, `WalkRate`, `NaturalAC`, HP (3 fields), save bonuses, alignment, `ChallengeRating`, `CRAdjust`, `PerceptionRange`, 13 `Script*` events, `SkillList`, `FeatList`, `SpecAbilityList`, `ClassList` (**up to 8 classes in EE**; `KnownList0-9`/`MemorizedList0-9`), `Equip_ItemList` {`EquippedRes`}, `ItemList` {`InventoryRes`, `Repos_PosX/Posy`}, `PaletteID`, `Comment` |
| **UTD** | door | `LocName`, `Description`, `Appearance` (DW, doortypes.2da; 0 = generic), `GenericType_New` (DW genericdoors.2da; legacy `GenericType` B), `AnimationState`, lock/trap/HP/save fields, `LinkedTo`, `LinkedToFlags`, `LoadScreenID`, 14 events incl. `OnClick`, `OnFailToOpen`, `PaletteID`, `Comment` |
| **UTP** | placeable | as UTD plus `Appearance` (placeables.2da), `AnimationState`, `HasInventory`, `Static`, `Useable`, `Type`, `BodyBag`, `OnUsed`, `OnInvDisturbed`, `ItemList` |
| **UTE** | encounter | `Active`, `Difficulty(Index)`, `Faction`, `MaxCreatures`, `RecCreatures`, `PlayerOnly`, `Reset(Time)`, `Respawns`, `SpawnOption`, events, `CreatureList` {`Appearance`, `CR`, `ResRef`, `SingleSpawn`}. GIT adds `Geometry` {X,Y,Z} and `SpawnPointList` {X,Y,Z,Orientation} |
| **UTI** | item | `BaseItem` (I), names (Loc), `DescIdentified`, `Charges` (≤250 EE), `Cost`, `AddCost`, `StackSize`, `Plot`, `Stolen`, `Cursed`, `Identified`, model fields by baseitems `ModelType` (`ModelPart1-3` + **`xModelPart1-3`**; armor `ArmorPart_*` ×19 + **x-twins with truncated labels**; 6 colours), `PropertiesList` {`PropertyName`, `Subtype`, `CostTable`, `CostValue`, `Param1`, `Param1Value`, `ChanceAppear`} |
| **UTM** | store | `MarkUp/MarkDown`, `BlackMarket`, `BM_MarkDown`, `IdentifyPrice`, `MaxBuyPrice`, `StoreGold`, `WillNotBuy`/`WillOnlyBuy` {`BaseItem`}, `StoreList` (struct ID = page 0–4) {`ItemList` {…, `Infinite`}}. **Palette field is `ID`, not `PaletteID`** |
| **UTS** | sound | `Active`, `Continuous`, `Looping`, `Positional`, `Random(Position)`, `Elevation`, `Min/MaxDistance`, `RandomRangeX/Y`, `Interval(Vrtn)`, `PitchVariation`, `Priority`, `Hours` (24-bit mask), `Times`, `Volume(Vrtn)`, `Sounds` {`Sound`}. GIT adds `GeneratedType` |
| **UTT** | trigger | `Type` (generic / transition / trap), `Cursor`, `HighlightHeight`, `LinkedTo(Flags)`, `LoadScreenID`, trap fields, `OnClick`, `ScriptOnEnter/Exit/Heartbeat/UserDefine`. GIT adds `Geometry` {`PointX/Y/Z`} and `ZOrientation` |
| **UTW** | waypoint | `Appearance`, `LinkedTo`, `HasMapNote`, `MapNote`, `MapNoteEnabled`, `Description` |
| **DLG** | conversation | root: `DelayEntry`, `DelayReply`, `NumWords` (recomputed on save), `EndConversation`, `EndConverAbort`, `PreventZoomIn`, `EntryList`/`ReplyList`/`StartingList`. Nodes: `Speaker` (entries only), `Animation`, `AnimLoop`, `Text` (Loc), `Script`, **`ActionParams` (EE)**, `Delay`, `Comment`, `Sound`, `Quest`, [`QuestEntry`], `RepliesList`/`EntriesList`. Links: `Index`, `Active` (condition), **`ConditionParams` (EE)**, `IsChild`, [`LinkComment`]. Param element = {`Key`, `Value`} CExoString [T; not yet seen non-empty in data] |
| **JRL** | `module.jrl` | `Categories` {`Name`, `XP`, `Priority`, `Picture` (unused), `Comment`, `Tag`, `EntryList` {`ID`, `End`, `Text`}} |
| **FAC** | `Repute.fac` | `FactionList` {`FactionParentID`, `FactionName`, `FactionGlobal`}, `RepList` {`FactionID1`, `FactionID2`, `FactionRep`}. The first 5 factions are fixed (PC, Hostile, Commoner, Merchant, Defender) |
| **ITP** | palettes | (1) skeleton `<type>pal.itp` (category tree: `STRREF`/`NAME`, `ID` = PaletteID, `TYPE`, `LIST`, `NEXT_USEABLE_ID`, `RESTYPE`); (2) standard `*palstd.itp` (leaves `RESREF` + `NAME`/`STRREF`; creatures add `CR`, `FACTION`); (3) **custom `*palcus.itp`**: 9 files regenerated by Aurora from module blueprints (used by the DM Creator); `PaletteID` 255 hides a blueprint. EE 1.79: `NAME` string allowed instead of `STRREF` (tileset palettes since 1.80.8193.6) |
| **BIC** | character (read-only) | UTC fields + runtime state; EE `DataMigration`, player SQLite blob |
| **PTM/PTT** | plot wizard (legacy) | preserve only |

Common `VarTable` = {`Name`, `Type` (1 int, 2 float, 3 string, 4 object, 5 location), `Value`}. The toolset edits int, float and string only.

### 2.3 Blueprint vs GIT instance
- Instances contain the full blueprint struct **minus** `PaletteID` and (except creatures) `Comment`.
- They **add**:
  - creatures, items, stores, waypoints: `XPosition/YPosition/ZPosition`, `XOrientation/YOrientation` (facing vector), written first
  - doors and placeables: `X/Y/Z` + `Bearing` (radians), written last
  - EE: `VisTransformList`
- Instance inventories (`Equip_ItemList`, `ItemList`, store pages) embed **full item structs**, not resrefs.
- `TemplateResRef` links an instance to its blueprint ("Update Instances").

### 2.4 EE-added fields (toolset-relevant)

| File | Field | Type | Since | Notes |
|---|---|---|---|---|
| IFO | `Mod_OnPlrTarget` | RR | event 1.80.8193.14; UI 87.8193.35 | |
| IFO | `Mod_OnPlrGuiEvt`, `Mod_OnPlrTileAct` | RR | 1.85.8193.30; UI 87 | |
| IFO | `Mod_OnNuiEvent` | RR | 85.8193.31/32; UI 87 | |
| IFO | `Mod_DefaultBic` | RR | 87.8193.35 | "no GUI yet"; Aurora preserves it |
| IFO | `Mod_PartyControl` | I 0/1/2 | 87.8193.35 | no GUI |
| IFO | `Mod_UUID` | Str | ≤89 | written empty; **must preserve** an existing value (fix in 88.8193.36) |
| ARE | `TileBrdrDisabled` | B | ≤89 | script `SetAreaTileBorderDisabled` (88) |
| GIT | `VisTransformList` {`Scope`, `AnimationSpeed`, `ScaleX/Y/Z`, `RotateX/Y/Z`, `TranslateX/Y/Z` each {`TimerType`, `ValueTo`, `LerpType`, [`ValueFrom`, `LerpDuration`, `LerpProgress`]}} | L | 1.79 (lerps 1.83.8193.21) | older files use the single `VisualTransform` struct: read both |
| UTC/GIT | `xAppearance_Head`, `xBodyPart_*` (17), `xArmorPart_RFoot` | W | 87.8193.35 (limit 255→999); toolset fix 88 | what the BYTE twin holds for ids > 255 is unobserved |
| UTI/GIT | `xModelPart1-3`, `xArmorPart_*` (19, truncated labels) | W | same | |
| DLG | `ActionParams`, `ConditionParams` {`Key`, `Value`} | L | 1.80.8193.14 / 1.81.8193.15 | written as empty lists on every node and link |
| ITP | `NAME` (Str) on nodes | Str | 1.79 / 1.80.8193.6 | |
| UTI | per-part armour colours `APart_<part>_Col_<ch>` | B | runtime (engine string) | not toolset-written; verify |

Not EE (1.69 or older): `Mod_OnPlrChat`, `Wings_New/Tail_New/GenericType_New`, placeable `OnClick`, `Mod_HakList`, `PreventZoomIn`. Checked and nonexistent: creature `DisplayName`, `Mod_Cache`, `UseTweakedStuff`. EE added **no new area or per-object event slots**.

### 2.5 Toolset-only data and module conventions
- `.gic`, blueprint `Comment`/`PaletteID`, `*palcus.itp`, `module.jrl`, `Repute.fac`, `.nss` (always saved with the `.ncs`), `.ndb` (optional), `.ptm/.ptt`.
- Exported `.erf` carries `ExportInfo.gff` (`Mod_MinGameVer`, `Expansion_Pack`, `Comments`, `Top`, `Dependencies`, `Missing`) [T].
- The 89 toolset saves **all valid file types** into the .mod (37-13). A .mod may contain anything except HAK/ERF/MOD/SAV/KEY/BIF [W: Modules https://nwn.wiki/spaces/NWN1/pages/155516929/Modules].
- `Mod_MinGameVer`: Aurora stamps its compatibility version (1.89), and older toolsets refuse newer modules [W: https://nwn.wiki/spaces/NWN1/pages/38175645/How+to+Change+the+Module+Version+Id]. Moonglow should write `max(existing, target)` and let the user choose.
- `nw_` resref namespace is reserved for game content (1.84 notes).

---

## 3. Resource loading order

Sources: https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order (quotes the engine's priority constants), https://nwn.wiki/spaces/NWN1/pages/53670235/HAK, https://nwn.wiki/spaces/NWN1/pages/53671463/userpatch.ini, [CL].

### 3.1 Game ResMan (highest wins; values × 1e6)

| Prio | Location | Notes |
|---|---|---|
| 99 | TEMP | |
| 91 / 90 | user `portraits/` / `data/prt` | any type (quirk: shadows haks) |
| 81 / 80 | user vaults / `data/lcv` | |
| **71 / 70** | user / install **`development/`** | top of normal content (v79). Hot-reloads uncached types (scripts, GFF), not textures |
| 60 | RIM | |
| 40 | NWSync manifest | |
| **31 / 30** | user `hak/` / `data/hk` | in `Mod_HakList` order, first = highest |
| 23 / 22 / 21 | current game / save game / savegame manifest | |
| **20** | **module** (.mod/.nwm; one only) | **below haks**: a module copy never overrides a hak copy |
| 13 | userpatch.ini haks | the toolset does not load these |
| **12** | user **`override/`** | **below haks and module** ("canonically below hak") |
| 11 | SOURCEOVERRIDE | alias present in game and toolset; undocumented |
| 10 | install `ovr/` | now only `nwscript.nss`, a human-reference copy (CRLF; identical content); **not read** |
| 9–6 | ambient/music (user, install) | via ResMan since v79 |
| 3 / 2 / 1 | install patch bifs / texture packs (removed in 37) / **KEY tables** | `nwn_base.key` then `nwn_retail.key` |

Special cases:
- **DDS vs TGA:** interleaved per tier: dev/override DDS > dev/override TGA > nwsync DDS > nwsync TGA > ERF DDS > ERF TGA > BIF DDS > BIF TGA. A TGA in a hak beats a BIF DDS. KTX position unknown.
- **TLK:**
  - `lang/<xx>/data/dialog.tlk` (+`dialogf.tlk`).
  - One custom TLK per module (`Mod_CustomTlk`), found in user `tlk/` (or NWSync).
  - Custom strref = index | 0x01000000.
  - The wiki is inconsistent on whether a TLK inside a hak works; treat `tlk/` as canonical [W: TLK https://nwn.wiki/spaces/NWN1/pages/38176005/TLK].
- **Startup-cached content:** some 2DAs and fonts are read at game start and are not reloaded from module haks.
- **Names:** lookups are case-insensitive; resrefs ≤ 16 characters; lowercase file names strongly advised (Linux).
- **Priority churn:** the ResMan was rewritten in v79. Hak/module/userpatch/override priorities were "restored" in 1.79.8193.5 and 1.80.8193.6.

### 3.2 Toolset vs game
- **Aliases present in `nwtoolset.exe`** [T]: DEVELOPMENT, OVERRIDE, OVERRIDELOCINSTALL, SOURCEOVERRIDE, HAK, TLK, PATCH, PORTRAITS, MUSIC/AMBIENT, vaults, WORK/WORKTEMP, EXPORT/EXPORTTEMP/IMPORT. NWSync mounting is unconfirmed.
- **Loads:**
  - KEY tables
  - `override/`
  - `development/` (since 1.83.8193.21; "may not live-reload like the game")
  - the open module's haks in `Mod_HakList` order
  - custom TLK
  - music/ambient from haks when listed in the 2DAs (1.80.8193.11)
  - the module, extracted to `modules/temp0/` or opened from `modules/<name>/`
- **Does not load:** userpatch.ini.
- **Hak changes** require re-opening the module. Hak reload is buggy (nwn-issues#346).
- **Consequences in the script editor** [W: Script Editor]:
  - saving a script whose name exists in a hak "takes no effect";
  - base-game scripts can be overridden by module copies.
- **Conflict report:** Module Properties → Custom Content → "Check for Conflicts" lists duplicate resources across haks and "Overriden standard resources" [T/W].
- **Recommendation for Moonglow:**
  - Implement one ResMan with the numeric priorities above.
  - Show per-resource "effective source" (like `ResManGetAliasFor`) and a conflict report.
  - Watch `development/` and the module folder for hot reload.
  - Optionally mount NWSync read-only.
  - Oracle: `nwn_resman_*` with `--userdirectory <fixture>`, or `nwn.resman.create(include_user=False)`.

---

## 4. Tileset system

Sources:
- https://nwn.wiki/spaces/NWN1/pages/38175567/SET
- https://nwn.wiki/spaces/NWN1/pages/38175063/Tilesets
- https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial
- https://nwn.wiki/spaces/NWN1/pages/72417367/Common+Issues+with+Tiles+and+Tilesets
- https://nwn.wiki/spaces/NWN1/pages/139689996/Tile+Path+Nodes
- https://nwn.wiki/spaces/NWN1/pages/179077218/Walkmesh+Notes
- https://nwn.wiki/spaces/NWN1/pages/60982635/Area+Editor
- https://nwn.wiki/spaces/NWN1/pages/26738846/Advanced+Area+Creation+Tips
- "Claude Code" subtree (https://nwn.wiki/spaces/NWN1/pages/195362833/SET+File+Format, /195362837/Tileset+Operations, /195362835/ITP+Palette+Format). These were checked against data, and **four claims are wrong** (see 4.8).
- BioWare ARE/ITP/Door PDFs.

Verified against all 33 base SETs and 441 official areas (Prelude, Ch1, Furiae, DoD, ToM).

### 4.1 SET format (INI, CRLF)
- **Section order** (Aurora reads sequentially; write in this order, read tolerantly):
  1. `[GENERAL]`
  2. `[GRASS]`
  3. `[TERRAIN TYPES]` + `[TERRAINn]`
  4. `[CROSSER TYPES]` + `[CROSSERn]`
  5. `[PRIMARY RULES]` + `[PRIMARY RULEn]`
  6. `[SECONDARY RULES]`
  7. `[TILES]` + `[TILEn]`, each followed by its `[TILEnDOORm]`
  8. `[GROUPS]` + `[GROUPn]`
- **Formatting:** LF line endings reportedly crash Aurora; write a trailing newline. Tolerate empty integers, `-90/-180/-270` angles, float door orientations and scientific notation.
- **`[GENERAL]`:**

  | Key | Meaning |
  |---|---|
  | `Name`, `Type=SET`, `Version=V1.0` | identity |
  | `Interior` | 0/1 |
  | `HasHeightTransition` | enables Raise/Lower |
  | `EnvMap` | default env-map texture |
  | `Transition` | metres per height step: 1–5 in base; must be ≥1, 0 breaks group placement |
  | `SelectorHeight` | defaults to Transition |
  | `DisplayName` | StrRef (−1 → `UnlocalizedName`) |
  | `Border` | terrain of the area's outer vertices |
  | `Default` | eraser and new-area fill |
  | `Floor` | interior "main floor" |

- **`[GRASS]`:** `Grass`, `GrassTextureName` (default `grass`), `Density`, `Height`, `Ambient/DiffuseRed/Green/Blue`. Grass grows on walkmesh faces with surfacemat 3.
- **Terrains and crossers:** `Count`, then `Name` (+`StrRef`). Always referenced **by name**, case-insensitive; base data mixes `Floor`/`floor`.
- **`[PRIMARY RULEn]`:** `Placed, PlacedHeight, Adjacent, AdjacentHeight, Changed, ChangedHeight`.
  - Semantics: placing `Placed@h` next to `Adjacent@h'` rewrites the neighbour to `Changed@h''`.
  - 23 base sets use rules; tno01 has 192. ttz01 has height transitions and **zero** rules, so pure tile matching must suffice.
  - `SECONDARY RULES` always `Count=0` (semantics unknown).
- **`[TILEn]`:**

  | Key(s) | Meaning |
  |---|---|
  | `Model` | tile MDL; `.wok` of the same name |
  | `WalkMesh` | junk (`msb01`), ignore |
  | `TopLeft/TopRight/BottomLeft/BottomRight` (+`…Height` 0/1) | corner terrains; top = +Y (north), right = +X |
  | `Top/Right/Bottom/Left` | edge crossers |
  | `MainLight1/2`, `SourceLight1/2`, `AnimLoop1..3` | "has" flags (junk values occur; real presence = model nodes) |
  | `Doors` | door section count (50 mismatches in base data) |
  | `Sounds` | always 0 |
  | `PathNode` (A–Z, a–x) + `Orientation` (deg CCW) | pathing |
  | `VisibilityNode/Orientation`, `DoorVisibilityNode/Orientation` | line-of-sight graphs (optional in practice) |
  | `ImageMap2D` | minimap TGA, **stored explicitly**, not always `mi_<model>`; 16² or 32² |

  **The tile index is what ARE `Tile_ID` stores: never renumber.**
- **`[TILEnDOORm]`:** `Type` (doortypes.2da row; **0 = generic hookpoint**, the builder picks genericdoors), `X,Y,Z` offset from the **tile centre**, `Orientation` (deg).
- **`[GROUPn]`:** `Name`, `StrRef`, `Rows`, `Columns`, `Tile0..Tile(R·C−1)`.
  - Layout: k → column `k % Columns` (+X), row `k / Columns` (+Y); **Tile0 = south-west**.
  - `-1` = a cell filled by matching (even Tile0 exists: tcm02 GROUP72).
  - Tiles used in groups are excluded from random matching.
- **Base-set counts:** ttr01 283 tiles / 3 terrains / 4 crossers / 28 rules / 67 groups; tno01 1,287 / 7 / 9 / 192 / 198; tcm02 1,872 / 7 / 5 / 51 / 204.
- **Oracle:** `nwn.tileset.read_set` (counts match; it drops the visibility keys, `SelectorHeight`, `UnlocalizedName`, `GrassTextureName`, group `StrRef` and secondary rules).

### 4.2 Corner/edge model
- **Tile geometry:** 10×10 m, origin at the tile centre, corners at (±5, ±5).
- **Corners** carry (terrain, height); **edges** carry a crosser or nothing.
- **`Tile_Orientation o` = o × 90° counter-clockwise.** With corners [TL, TR, BR, BL] and edges [T, R, B, L], one CCW step gives `new[i] = old[(i+1) % 4]`. Verified: 100% agreement over about 161,000 shared corner/edge checks.
- **Absolute vertex height** = SET corner height (0/1) + `Tile_Height`. Neighbours agree on terrain and absolute height with **zero** violations; crossers disagreed 13 times (group edges).
- **Recommended internal model:**
  - A (W+1)×(H+1) vertex grid of (terrain, absolute height), plus per-tile edge crossers.
  - Candidate tiles for a cell are all (non-group tile, orientation) pairs whose rotated signature matches. Precompute a signature → [(tile, orientation)] index at load.
  - Randomise among candidates. Symmetric tiles show near-uniform orientation use.

### 4.3 ARE tile data and world mapping
- **`Tile_List` element** (struct 1):

  | Field | Type | Meaning |
  |---|---|---|
  | `Tile_ID` | I | SET index |
  | `Tile_Orientation` | I | 0–3 |
  | `Tile_Height` | I | ≥0; official data reaches 19 |
  | `Tile_MainLight1/2` | B | lightcolor.2da row 0–31; 0 = off |
  | `Tile_SrcLight1/2` | B | 0 = off, 1–15 |
  | `Tile_AnimLoop1/2/3` | **B** | the PDF says INT |

- **Index → cell:** `x = i % Width`, `y = i / Width`, row-major from the **south-west**. Tile centre = (10x+5, 10y+5); base Z = `Tile_Height × Transition`.
- **Door hookpoint → GIT door** (1,854 doors match to < 1e-5 m):
  - `pos = centre + Rz(90°·o)·(X,Y) + (0, 0, Tile_Height·Transition + Z)`
  - `Bearing = radians(Orientation + 90·o)`, or +π when flipped.
  - `Appearance` = SET `Type` when ≠ 0.
- **Area size:** the wizard allows up to **32×32** (largest official area 30×28). No EE change is documented.
- **Resize** adds or removes rows and columns at the north/east (origin stays SW). Rotate Area exists and is non-destructive, but crashes are reported in the current Aurora (#766).
- **New tile light colours** are picked randomly from environment.2da `MAIN1_COLOR1..4`, `MAIN2_COLOR1..4` and `SECONDARY_COLOR1..4` of the area's `LightingScheme` [BioWare ARE PDF].
- **EE runtime API** (useful for "test in game" tooling): `SetTile`, `SetTileJson`, `SetTileAnimationLoops`, `GetTileID/Orientation/Height`, `ReloadAreaGrass/Border`.

### 4.4 Painting operations (documented behaviour)
- **Terrain brush:** acts on a **vertex**, affecting 4 tiles. Repainting re-randomises. Shift+click cycles through matching tiles × rotations.
- **Eraser:** paints `Default` terrain; on one tile it re-randomises; on a group it removes the group.
- **Raise/Lower:** left/right click; never below 0; stacks multiple levels.
- **Crossers:** dragged tile to tile; the cursor turns red when invalid; crossing another crosser picks bridge tiles.
- **Groups/features:** a ghost follows the mouse; right-click rotates it 90°. Every tile gets the group's orientation, and offsets rotate CCW about Tile0 (verified). Deleting one tile leaves a partial group.
- **Copy/paste regions:** same tileset only; surrounding tiles may be changed to fit.

### 4.5 Palette (`<tileset>palstd.itp`)
- `MAIN` categories: ID **0 Features** (leaf `RESREF` = Tile0 model of a 1-tile group), **1 Groups** (Tile0 model of a multi-tile group), **2 Terrain** (leaf = terrain or crosser name; special `eraser`, `raiselower`).
- tno01 uses extra top-level IDs 3–6 for themed feature and group branches, and nested sub-folders exist. Resolve any non-terrain leaf by "group whose Tile0 model == RESREF", whatever the branch ID.
- Duplicate Tile0 across groups exists. Sources disagree whether the first or the last wins.

### 4.6 Walkmesh and pathing
- **`.wok`:** ASCII `#MAXWALKMESH` / `beginwalkmeshgeom`; one `aabb` node (verts; faces `v1 v2 v3 smooth t1 t2 t3 material`; AABB tree lines `min max leafFace|-1`, depth-first). The same AABB is embedded in the tile MDL.
- **Face material** = surfacemat.2da row (0–29 used; 64 rows since 1.84.8193.29): `Walk`, `LineOfSight`, `Sound`, `IsWater`, grass = 3.
- **Walkmesh rules:** ≤ 8 faces per vertex, edges at exactly ±5, no vertical, overlapping or degenerate faces, a single AABB at the origin.
- **Door `.dwk`:** state meshes `*_DWK_wg_closed/open1/open2` + use points `*_DWK_dp_closed_01/_02`, `open1_01`, `open2_01` (the real names have the underscore that the wiki omits).
- **Placeable `.pwk`:** `<model>_wg` (+ `_pwk_use01/02`).
- **Tile path nodes:** A–Z, a–x (q–x added in 87.8193.35). Coarse per-tile exit/region graph, rotated by SET `Orientation` [I: plus `Tile_Orientation`]. Full table in `notes_tilesets.md` §D.4 and on the wiki.
- **Moonglow needs walkmeshes** for Z-snapping of placed objects, walkability and material overlays, and grass placement.

### 4.7 Lights, animations, edges, minimap
- **Main lights:** `light` nodes `<model>ml1/ml2`; colour = lightcolor.2da `RED/GREEN/BLUE` (HDR up to 2.4). The toolset swatches use `TOOLSETRED/GREEN/BLUE`. **Adding rows to lightcolor.2da crashes Aurora's Tile Properties.**
- **Source lights:** dummies `<model>sl1/sl2` (77 are wrongly `light` nodes). They spawn `fx_flame01.mdl` and play animation `"1"`…`"15"`; UI colour ≈ lightcolor row 2n.
- **Tile animations:**
  - `animloop01..03` (two digits; toggled by `Tile_AnimLoop1..3`)
  - `day`, `night`, `day2night`, `night2day`, `tiledefault`
  - Animated geometry sits under the **a-node** `<model>a`.
- **Edge tiles:** the game draws 5 tiles (50 m) of border from `<tileset>_edge.2da` (`Corner1, Edge, Corner2, Height, Model`), unless `TileBrdrDisabled`. Whether Aurora renders them is unknown.
- **Minimap:** one TGA per tile from `ImageMap2D`, rotated with the tile [I].

### 4.8 Wiki claims found wrong or contradicted by data
- "Claude Code" pages:
  - "ImageMap2D always `mi_`+model": false for 3,825 of 12,342 tiles.
  - "Primary rules unused": false.
  - ITP sub-folder ID 2 = paint / ID 3 = placement: unsupported by data or spec.
  - "DoorVisibilityNode required": false.
  - "Sorting tiles by name": breaks existing areas.
- Tutorial: `animloop1` → really `animloop01`; tile light colours come from lightcolor.2da, not tilecolor.2da (tilecolor.2da is a 16-row toolset-only table with an unknown purpose).

### 4.9 Reverse-engineering checklist (run Aurora under Wine, script input, diff `Tile_List`)
1. Primary-rule neighbourhood (4 vs 8 vertices), propagation depth and order, relative-vs-absolute height in rules, and the no-match fallback (refuse / red cursor / Default).
2. Crosser drag semantics (start/middle/end tiles); whether terrain paint preserves crossers; bridge selection.
3. Raise/lower footprint and stacking beyond ±1.
4. RNG over (variant × orientation); Shift-cycle order.
5. New-area seed (Border ring, Default fill, interior Floor patch); initial light colours.
6. Group pivot, validity checks near edges and heights, partial deletion.
7. Border/edge handling and `_edge.2da` lookup; `SECONDARY RULES`; `Doors=` vs section-count precedence; duplicate group Tile0.

Test sets: ttr01 (rules + heights), ttz01 (no rules), tic01 (rooms/walls), tdm01 (crossers).

---

## 5. Models, animations, part-based creatures, textures

Sources:
- https://nwn.wiki/spaces/NWN1/pages/12027273/MDL+ASCII
- https://nwn.wiki/spaces/NWN1/pages/38175669/MDL
- https://nwn.wiki/spaces/NWN1/pages/53671005/Model+Table+of+Parameters
- https://nwn.wiki/spaces/NWN1/pages/38176272/Model+Special+Nodes
- https://nwn.wiki/spaces/NWN1/pages/38175170/Animations
- https://nwn.wiki/spaces/NWN1/pages/49447501/Part-Based+Models+Creatures
- https://nwn.wiki/spaces/NWN1/pages/60985168/Part-Based+Models+Items
- https://nwn.wiki/spaces/NWN1/pages/14618045/PLT
- https://nwn.wiki/spaces/NWN1/pages/38174941/appearance.2da
- https://nwn.wiki/spaces/NWN1/pages/38174935/baseitems.2da
- https://nwn.wiki/spaces/NWN1/pages/38174958/Textures
- https://nwn.wiki/spaces/NWN1/pages/3473496/DDS, /53672779/TGA, /38174929/TXI, /12027232/MTR, /133988392/KTX, /26738916/LOD+-+Level+of+Detail

Binary layout from the nwnmdlcomp source vendored in neverblender, verified on the whole corpus. Byte-level tables are in `notes_models.md` §B.

### 5.1 MDL ASCII (7,235 of 32,832 base models)
- **Structure:**
  ```
  newmodel <name>
  setsupermodel <name> <super|NULL>
  classification <…>
  [ignorefog]
  setanimationscale <f>
  beginmodelgeom … node <type> <name> … endnode … endmodelgeom
  newanim <anim> <model> length/transtime/animroot/event … doneanim
  donemodel
  ```
  - `classification` is one of tile, character, effect(s), door, gui, item, other. The binary stores 0 none, 1 effect, 2 tile, 4 character, 8 door.
  - The first node is a dummy named after the model.
- **Node types:** dummy, trimesh, danglymesh, skin, animmesh, emitter, light, aabb, reference. `camera`, `patch` and `pwk` are treated as dummy.
- **Common keys:** `parent`, `position`, `orientation` (**axis-angle**), `scale`.
- **Trimesh keys:**

  | Key(s) | Notes |
  |---|---|
  | `ambient`, `diffuse` | default 1 |
  | `specular`, `shininess` | ignored in EE |
  | `selfillumcolor` | also misspelt `setfillumcolor`: 2,388 shipped |
  | `alpha`, `render`, `shadow`, `beaming`, `transparencyhint` | |
  | `tilefade` | 0/1/2/3; **4 undocumented but used 1,877×** |
  | `rotatetexture` | |
  | `bitmap`/`texture0`, `texture1-3`, `materialname` (EE), `renderhint NormalAndSpecMapped\|NormalTangents` (EE) | textures and materials |
  | `verts`, `tverts[1-3]`, `faces` | faces are `v1 v2 v3 smoothgroup t1 t2 t3 material`, with **separate vertex and UV indices**: de-index |
  | `colors`, `normals`, `tangents` (EE) | |

- **Other node types:**
  - **danglymesh:** + `displacement`, `tightness`, `period`, `constraints`.
  - **skin:** + `weights` (≤4 bone-name/weight pairs; ≤64 bones per skin since 8193.21).
  - **animmesh:** `sampleperiod`, `animverts`, `animtverts`.
  - **light:** `radius`, `multiplier`, `color`, `ambientonly`, `ndynamictype`/`isdynamic`, `affectdynamic`, `shadow`, `lightpriority` 1–5, `fadinglight`, flares, `shadowradius`, `verticaldisplacement`.
  - **emitter:** about 60 params (update, render and blend modes; sizes/colours/alpha start-mid-end; birthrate; xgrid/ygrid; chunkName; p2p needs a child `reference`).
  - **reference:** `refmodel`, `reattachable`.
  - **aabb:** walkmesh.
- **Limits:**
  - node names ≤ 32 characters
  - resrefs ≤ 16
  - animation names ≤ 16 (longer ones don't play)
  - faces/mesh ≤ 21,845 since 87.8193.35 (was about 10,922)
  - ≤ 4 weights per vertex
- **Parser robustness:** be case-insensitive; accept `true/false` booleans, 4-component colours, both `filedependancy`/`filedependency`, `positionkey N` with or without `endlist`, and wrong counts in old files.
- The game compiles ASCII at load time and generates normals and tangents.

### 5.2 Binary MDL (25,597 files; 32-bit little-endian memory image)

- **File header:** `u32 0` (the binary marker), `u32 rawOffset`, `u32 rawSize`.
  - Model data base `M = 12`; raw data base `R = 12 + rawOffset`.
  - `Ptr` = u32 offset from M (0 = null). `RPtr` = offset from R (**0xFFFFFFFF = null**).
  - `Array` = {offset, count, alloc}.
- **Geometry header (112 B):** name `char[64]` @8; root node @0x48; node count @0x4C (includes supermodel nodes); type @0x6C (2 = model, 5 = animation).
- **Model header (+0x70):** classification @0x72, fog @0x73, animations `Array<Ptr>` @0x78, bbox @0x88/0x94, radius @0xA0, **animscale @0xA4**, supermodel name `char[64]` @0xA8.
- **Animation header:** length @0x70, transtime @0x74, animroot `char[64]` @0x78, events `Array<{f32 t; char[32]}>` @0xB8.
- **Node header (112 B):** part number @0x1C, name `char[32]` @0x20, children `Array<Ptr>` @0x48, controller keys @0x54 (12 B: `i32 type, i16 rows, i16 timeOff, i16 dataOff, i8 cols`), controller data `f32[]` @0x60, **flags @0x6C = node type**:

  | Flags | Type | Struct size |
  |---|---|---|
  | 0x001 | dummy | 112 |
  | 0x003 | light | 204 |
  | 0x005 | emitter | 328 |
  | 0x011 | reference | 180 |
  | 0x021 | trimesh | 624 |
  | 0x061 | skin | 724, or an EE variant of 944 |
  | 0x0A1 | animmesh | 680 |
  | 0x121 | danglymesh | 648 |
  | 0x221 | aabb | 628 |

  Function pointers, parent pointers and geometry back-pointers are **garbage in EE-compiled files**: rebuild parents from the children arrays.
- **Controllers** (IDs depend on node type):
  - all nodes: position 8 (3 cols), **orientation 20 (quaternion x,y,z,w)**, scale 36
  - mesh: selfillumcolor 100, **alpha 128**
  - light: color 76, radius 88, shadowradius 96, verticaldisplacement 100, multiplier 140, EE-only 144 (unknown, always 1.0)
  - emitter: IDs 80–216 (birthrate 88, colorStart 108, sizeStart 168, …), detonate 228
  - Geometry-level controllers are a single row at t = 0 (the rest pose).
- **Mesh header (+0x70):**

  | Offset | Field |
  |---|---|
  | +0x78 | faces (32 B: plane normal + d, surface id, 3 adjacent, **3 u16 vertex indices**) |
  | +0xAC / 0xB8 / 0xC4 | diffuse / ambient / specular |
  | +0xD4 / 0xD8 / 0xDC / 0xE0 | shadow / beaming / render / transparencyhint |
  | +0xE8 | texture0–3 as `char[64]` ×4 |
  | +0x1E8 | tilefade |
  | +0x22C | verts (RPtr) |
  | +0x230 | vertex count (u16) |
  | +0x234 | tverts0–3 |
  | +0x244 | normals |
  | +0x248 | colors (u32 RGBA) |
  | +0x265 | rotatetexture |

  Vertices are **per corner**: all streams share one index. Smoothing groups are not stored.
- **Skin:** weights `f32[4n]` (RPtr) @+0x27C; bone refs `i16[4n]` @+0x280; node→bone map **i16** @+0x284 (+count); qbone (w,x,y,z) and tbone arrays per node. The node index is the DFS pre-order of the model's own tree. Derive the bone list by inverting the map.
- **animmesh sets:** vertex-major `vertex*sets + set`, stored on the animation node.
- **danglymesh:** `constraints f32[]`, displacement, tightness, period.
- **AABB entries:** 40 B (min, max, left, right, leaf face, plane).
- **light:** flare arrays, priority, ambientonly, dynamic type, affectdynamic, shadow, generateflare (always 0), fadinglight.
- **emitter:** deadspace, blast, xgrid/ygrid, spawntype, update/render/blend strings, texture, chunkname, loop, renderorder, flags (p2p, affectedByWind, tinted, bounce, random, inherit…).
- **EE specifics:** the same layout as 1.69, but with garbage pad/pointer fields. `materialname`/`renderhint` storage is unknown (possibly the texture3 slot). No stored tangents: compute them at load. Always use unaligned, bounds-checked reads.
- **Reader recipe:** `notes_models.md` §B.21. Prototype: `scripts/mdlbin.py` (0 errors over the corpus).

### 5.3 Animations the editor must play

| Object | Play |
|---|---|
| Tile | `tiledefault` (if present), then `day` or `night` (from ARE `IsNight`/`DayNightCycle`), plus each enabled `animloop01..03` looping. Some animloops have no keys: no-op. Emitters (birthrate) and lights are keyed too |
| Placeable | by `AnimationState`: 0 `default`, 1 `open`, 2 `close`, 3 `dead`, 4 `on`, 5 `off` (1-frame poses; transitions are `close2open`, `open2close`, `off2on`, `on2off`). Fall back to `default`, then the bind pose. Static placeables cannot animate |
| Door | 0 `closed`, 1 `opened1`, 2 `opened2`; `trans` (transition highlight mesh alpha). Previews: `opening1/2`, `closing1/2` |
| Creature | appearance.2da `MODELTYPE` P/F → `pause1` (from the a_ba chain); S/L → `cpause1`; wing/tail sub-models → `pause1` \| `cpause1` \| `creadyl` (toolset order [T]) |
| Item | none (optionally `default`) |
| VFX | `impact` → loop `duration` → `cessation`; `vco_*` `conjure01`; `vca_/var_*` `cast01`; `vpr_*` `travel01`; `fx_flame01` `"1"`–`"15"` |

Aurora's own preview menu [T]: doors Closed / Closing1/2 / Opened1/2 / Opening1/2; placeables Activate / Closed / Deactivated / Default / Destroyed / Open.

- **Supermodels:**
  - Look up the animation by name in the model, then walk the supermodel chain until NULL.
  - Player chain: `pmh0 → a_ba → a_ba_non_combat → a_ba_med_weap → a_ba_custom → a_ba_casts`.
  - Other races: female `a_fa`; dwarf/gnome/**half-orc male** `a_da`; female dwarf/half-orc `a_dfa`; phenotype 2 → `a_*2`; mounted 3/5 `h_ba`/`h_fa`; joust 6/8 `j_ba`.
  - Bind animation nodes to instance nodes **by case-insensitive name**; the binary part numbers confirm name resolution against the direct supermodel. Nodes missing from the ancestor are not driven.
- **animroot / overlays:** an overlay animation drives only the subtree under its animroot, and the deeper animroot wins. `transtime` = cross-fade.
- **animscale:** e.g. pmh halfling 0.65, gnome 0.62, goblin 0.58. [I] Multiply inherited **position** keys by the instance's scale.
- **Events:** only `detonate` matters in the editor (it fires Explosion emitters).

### 5.4 Part-based creatures (appearance.2da `MODELTYPE` P)
- **Skeleton:** `p` + gender (`m`/`f`) + RACE letter (A halfling, D dwarf, E elf, G gnome, H human/half-elf, O half-orc) + phenotype (0 normal, 2 large, 3/5 mounted, 6/8 joust; fallback via phenotype.2da `DefaultPhenoType`). Examples: `pmh0`, `pfd2`.
- **Parts:** capart.2da (19 slots) → `{skel}_{mdlname}{NNN}` attached at the slot's node:

  | Part | Node | Part | Node |
  |---|---|---|---|
  | footr/footl | rfoot_g/lfoot_g | shinr/shinl | rshin_g/lshin_g |
  | legl/legr | lthigh_g/rthigh_g | pelvis | pelvis_g |
  | chest | torso_g | belt | belt_g |
  | neck | neck_g | forer/forel | rforearm_g/lforearm_g |
  | bicepr/bicepl | rbicep_g/lbicep_g | shor/shol | rshoulder_g/lshoulder_g |
  | handr/handl | rhand_g/lhand_g | robe | root |

  Part number = equipped armor `ArmorPart_*` (WORD `x` twin if > 255), else creature `BodyPart_*`. The exact per-slot rule when the armor part is 0 is open.
- **Robe:** a skinmesh with supermodel `pmh0` (bones via the coat chain); `parts_robe.2da` `HIDE*` columns hide slots.
- **Head:** `{skel}_head{NNN}` at `head_g`. A helmet (`helm_NNN`, scaled by `HELMET_SCALE_M/F`) replaces it.
- **Cloak:** cloakmodel.2da → `{skel}_cloak_{MODEL:03}` with texture `cloak_{TEXTURE:03}.plt`, hiding wings/tail/shoulders as flagged.
- **Wings and tails:** `Wings_New`/`Tail_New` → wingmodel/tailmodel.2da `MODEL` at the `wings`/`tail` dummies, scaled by `WING_TAIL_SCALE`. They play the same animation names as the body.
- **Weapons:** `rhand`/`lhand`; shields at `lforearm`; scaled by `WEAPONSCALE`. Composite weapons are 3 parts `<class>_{b,m,t}_NNN`, all attached at one origin.
- **Texture = the mesh `bitmap`**, which is often another part's or race's PLT. PLT fallback: phenotype-specific → pheno 0 → human same gender → `pmh0`. 37-13 fixed Aurora loading only human PLTs (and #429: only pmh0/pfh0 for skinmesh parts).
- **Env map:** appearance/cloak/wing/tail `ENVMAP`: `default` = area/tileset env map (Aurora uses `Chrome1`), `****` = none (alpha = transparency).

### 5.5 PLT colouring
- **Header:** `"PLT V1  "`, u32 (10; 6 in one file; ignore), u32 0, u32 width, u32 height. Then w·h pairs of (grey 0–255, layer 0–9). Rows are **bottom-up**.
- **Layers:**

  | Layer | Colour source | Palette |
  |---|---|---|
  | 0 skin | UTC `Color_Skin` | `pal_skin01` |
  | 1 hair | `Color_Hair` | `pal_hair01` |
  | 2 metal1 | UTI `Metal1Color` | `pal_armor01` |
  | 3 metal2 | UTI `Metal2Color` | `pal_armor02` |
  | 4/5 cloth1/2 | `Cloth1/2Color` | `pal_cloth01` |
  | 6/7 leather1/2 | `Leather1/2Color` | `pal_leath01` |
  | 8/9 tattoo1/2 | `Color_Tattoo1/2` | `pal_tattoo01` |

- **Colour** = `palette[layer][row = colour index 0–175, counted from the top][column = grey]`, alpha included. Palettes are 256×176 TGA (bottom-left stored: row k = stored row 175−k).
- Grey 255 has alpha 0 in most palettes → transparent, or fully env-mapped when an env map applies.
- Custom `pal_*.tga` load from haks/NWSync (37-13).
- GPU reference: `fs_pltgen.shd` (`texture(pal, vec2(grey, PLTscheme[layer]))`).
- Metal/cloth/leather on a creature come from its **equipped armor**.

### 5.6 Textures
- **Lookup for a mesh texture name:** `name.mtr` first. Then image KTX > DDS > PLT > TGA *within* a location; DDS/TGA are interleaved across locations (§3). The `.txi` of the same name is always applied. A missing texture renders white.
- **TGA:** types 2, 10 (RLE), 3 (grey), 24/32 bpp. Upload bottom-left rows as stored; flip only top-left-origin files (10 files). NPOT exists (palettes 256×176, minimaps). 789 32-bpp files declare 0 alpha bits: treat the 4th byte as alpha [I].
- **BioWare DDS** (no magic):
  - header `u32 w, u32 h, u32 channels (3 = DXT1, 4 = **DXT5**), u32 top-mip size, f32 alphamean`
  - full mip chain; non-square allowed; up to 2048
  - data stored **bottom-row first**: upload as stored
- **Standard DDS** (`DDS `, since v78): BC1/2/3/4/5; BC5 normal maps use RG only (green-up); BC7/DX10 undocumented → unsupported.
- **KTX1 ETC2:** mobile only; optional.
- **TXI keywords:**

  | Keyword(s) | Effect |
  |---|---|
  | `mipmap`, `filter`, `clamp`, `downsample*` | sampler / quality |
  | `alphamean` | alpha hint (needed < 1 for custom alpha shaders since 1.83.8193.26) |
  | `blending default\|additive\|punchthrough` | blend mode |
  | `decal` | unlit |
  | `envmaptexture <tex\|default>`, `bumpyshinytexture` | env map |
  | `bumpmaptexture shinywater` | fancy water |
  | `cube 1` + `filerange 6` | 6-file cubemap `name0..5` |
  | `proceduretype cycle` + `numx numy fps` | flipbook |
  | `proceduretype arturo`/water | CPU distortion |
  | font keys | fonts only |

- **MTR directives:**

  | Directive | Meaning |
  |---|---|
  | `texture0..10` / `null` | 0 diffuse, 1 normal, 2 specular, 3 roughness, 4 height, 5 self-illum, 6–10 custom |
  | `renderhint` | normal-mapped shader + tangents |
  | `customshaderVS/FS/GS` | custom GLSL |
  | `parameter float\|int <name> …` | stock: Specularity, Roughness, Metallicness, DisplacementOffset/Multiplier, CustomSpecularColor |
  | `transparency 1`, `twosided 1`, `sample_framebuffer 1\|2`, `volumetric 1` | render-queue modes (87) |

  For PLT meshes, omit `texture0`. Only 7 MTRs ship.
- **Placeables:** placeables.2da `ModelName`, `LightColor` + `LightOffset*` (spawns `fx_placeable01`), `ShadowSize` (blob), `Reflection` (env map), `Static`.
- **Doors:** `Appearance≠0` → doortypes.2da `Model`, else genericdoors.2da[`GenericType_New`]. Tileset doors come from SET hookpoints.
- **Icons:** item icons are TGA/DDS (PLT for layered/armor), `i<class>_<NNN>`. Aurora needs a **TGA** icon to list a model number and composites the b/m/t icons. It scales oversized icons since 37-13; DDS icons are still flaky (#455).

---

## 6. EE lighting and rendering model (from the effective stock shaders)

Sources:
- 92 effective `.shd` (base_shaders.bif, overridden by `nwn_retail.key`/`ovr.bif`, 37-13)
- `nwmain`/`nwtoolset.exe` strings
- https://nwn.wiki/spaces/NWN1/pages/38175899/Enhanced+Lighting+Engine+and+PBR
- https://nwn.wiki/spaces/NWN1/pages/60981936/Shaders
- https://nwn.wiki/spaces/NWN1/pages/14614573/Shader+Engine+Support
- https://nwn.wiki/spaces/NWN1/pages/65470710/Shaders+and+Area+Flags
- https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting
- https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes
- https://nwn.wiki/spaces/NWN1/pages/38174773/Environment+Maps+and+Cubemaps
- https://nwn.wiki/spaces/NWN1/pages/49447442/Model+Shadows
- https://nwn.wiki/spaces/NWN1/pages/38175898/Standard+material+inputs

Full uniform tables and formulas: `notes_shaders.md`.

### 6.1 Programs and selection
- **Engine preamble** (identical in nwmain and nwtoolset):
  ```
  #version 330 core
  MAX_NUM_LIGHTS            // client setting 3..128, default 32
  MAX_NUM_BONES 64
  GAMMA_CORRECTION          // "enhanced lighting"
  FRAGMENT_LIGHTING
  SHADER_QUALITY_MODE       // 0..2
  KEYHOLING_ENABLED
  SHADER_DEBUG_MODE
  BUILD_VERSION / BUILD_REVISION
  NO_DISCARD                // per material: 1 = opaque, keeps early-Z
  POSTPROCESSING_TYPES_ENABLED
  ```
  plus compatibility macros (`varying`, `texture2D`, `gl_FragColor → compat_glFragColor`).
  - `#include "x"` is a verbatim splice.
  - A compile failure renders `vs_invalid`/`fs_invalid` (magenta).
- **Hard-coded pairs:**

  | Pair | Use |
  |---|---|
  | **`vslit`/`fslit`** | default lit textured mesh: tiles, placeables, creatures |
  | `vslit_nm`/`fslit_nm` | + normal/spec/rough/height/self-illum (renderhint) |
  | `vslit_sm`/`fslit_sm` | + env map ("sphere/env mapped", *not* shadow or specular) |
  | `vslit_sm_nm` | both of the above |
  | `vslitnotex`, `vslitnt_sm` | no UVs |
  | `vsvt/fst` | unlit textured: GUI, **skyboxes** |
  | `vsvtc/fstc`, `vsvc/fsc`, `vsv/fs`, `vsglu/fsglu` | vertex colour / flat / debug |
  | `vsgrass/fsgrass` | grass |
  | `vsparticle/fsparticle` | particles |
  | `vs_shadowvol/fs_shadowvol`, `vs_shadowplane`, `vs_beamvol` | shadows and beams |
  | `vs_pltgen/fs_pltgen` | GPU PLT |
  | `vswater/fswater` | fancy water: TXI `bumpmaptexture shinywater` + setting |
  | `vsfbpostpr/fsfbpostpr` | single post pass (always runs) |
  | `fsFBSSAO`, `fsFBBLOOM` | multi-pass post effects |

  Skinning is a runtime uniform (`skinmesh`), not a permutation.
- **The toolset uses** the lit, unlit, grass, particle, shadow, beam, PLT and post programs. It does **not** use water, SSAO, bloom, DoF or toon. It references a `vso` shader that exists nowhere in the data.
- **Selection:** MTR `customshaderVS`+`FS` if given. Otherwise from mesh traits: lit? UVs? env map (TXI `envmaptexture`/`bumpyshinytexture`/cube, appearance `ENVMAP`, placeables `Reflection`, tileset `EnvMap`) → `_sm`; `renderhint` → `_nm`; TXI `decal` → unlit; GUI → `vsvt/fst`.
- **Permutation defines:** `LIGHTING, FOG, KEYHOLING, NORMAL_MAP, SPECULAR_MAP, ROUGHNESS_MAP, HEIGHT_MAP, SELF_ILLUMINATION_MAP, ENVIRONMENT_MAP, VERTEX_COLOR, NO_TEXTURE(_COORDS), FORCE_VERTEX_LIGHTING, …`. `SPECULAR_LIGHT = LIGHTING && (FRAGMENT_LIGHTING || FRAGMENT_NORMAL)`.
- **`inc_config` constants:** `COLOR_CORRECTION_TYPE 4` ("legacy balanced" clamp), `SPECULAR_DISTRIBUTION_MODEL 1` (GGX); Schlick geometric term and Fresnel at quality 2.

### 6.2 Attributes, uniforms, texture units
- **Attributes:**

  | Attribute | Meaning |
  |---|---|
  | `vPos` | since 37-13, static geometry is **baked in world space** |
  | `vTcIn`, `vTcIn1..3` | UV sets |
  | `vNormal`, `vTangent`, `fHandedness` | tangent space |
  | `vIndex`, `vWeight` | 4 bone indices and weights |
  | `vColor` | **engine-baked static vertex lighting**, not MDL colours |
  | `vCustomColor` | MDL `colors` (unused) |
  | `vStaticLightDir` | brightest static light direction |
  | `fProjectionFrontal` | shadow volume cap flag |

- **Uniforms:**
  - transforms: `m_mvp`, `m_mv`, `m_proj`, `m_view(_inv)`, `m_texture` (only `mat2`: `rotatetexture`), `m_bones[64]`
  - lights: `numLights`, `lightPosition[i]` (view space), `lightColor[i]` (**a = cutoff², negative ⇒ ambient-only**), `lightMaxIntensityInv`, `lightFalloffFactor`, `staticLighting`
  - area: `lightAreaAmbient`, `lightAreaDiffuse`, `lightAreaDiffuseDirection`
  - material: `materialFrontAmbient/Diffuse` (diffuse.a = mesh alpha), `materialFrontEmissive` (selfillum), `fAlphaDiscardValue` (default ≈0.2)
  - fog: `fogParams` (enabled, start, end, 1/(end−start)), `fogColor` (**gamma space**)
  - keyhole: `keyhole*`, player/camera uniforms
  - time/area: `worldtimerTimeOfDay`, `areaFlags`, `areaGlobalWind`
  - MTR params, `scriptable*` (per player)
- **Texture units:**

  | Unit | Content |
  |---|---|
  | 0 | diffuse (pow 2.2; alpha = opacity, or **1 − reflectivity** when env-mapped) |
  | 1 | normal (RG, z reconstructed, GL green-up) |
  | 2 | specular (R) |
  | 3 | roughness (R) |
  | 4 | height (parallax + AO) |
  | 5 | self-illumination (pow 2.2) |
  | 6–10 | custom |
  | 11/12 | framebuffer colour/depth (the wiki contradicts itself on which is which) |
  | 13 | env cube |
  | 14 | env 2D |
  | 15 | noise `solid_noise.dds` (BC4) |

  The env-map fallback is **`chrome1`**.

### 6.3 Lighting equations (FRAGMENT_LIGHTING path, view space)
```
lin(x)=sign(x)|x|^2.2 ; gam(x)=x^(1/2.2)                  // manual; no GL sRGB. Only when GAMMA_CORRECTION=1
tex = lin(texture0)  [env: envLevel = 1-tex.a, alpha not transparency]; discard if a <= fAlphaDiscardValue
N = two-sided normal (flip for back faces); normal map: n.xy = tex1.rg*2-1, z = sqrt(1-|xy|^2), TBN
Ambient = lightAreaAmbient ; Diffuse = lightAreaDiffuse * max(N·L_sun,0)
for each light i:  d² = |Lpos−P|², R² = |lightColor.a|; skip if d²>R²
    f = d²/R² ; att = (1−f) / (lightMaxIntensityInv + lightFalloffFactor·f)
    a<0 → Ambient += rgb·att (no N·L)   else Diffuse += rgb·att·max(N·L,0)  (+softening/translucency at q2)
    Specular += rgb·att · GGX: 1/den², den = NdotH²(α²−1)+1, α = roughness (used directly), ·Schlick-G(k=r)·F(VdotH) at q2
Specular *= α²/4 · G(NdotV)                                   // Cook-Torrance without 1/π; Lambert without 1/π
Env = (Ambient+Diffuse) ; envSpec = mix(F(NdotV), spec0, sqrt(r)) ; Specular += Env · lin(envSample(lod=r*30−1)) · envSpec
Total = emissive + (1−envSpec)·(Ambient·matAmbient + Diffuse·matDiffuse)   // HDR, unclamped when enhanced
rgb = albedo·Total (+ lin(selfIllumMap)) + specColor·Specular   (spec not divided by alpha)
rgb = gam(rgb) ; rgb = mix(rgb, fogColor, fog)                 // fog AFTER gamma revert
```
- **Material defaults when there are no maps:**
  - specularity 0.04, roughness 0.55
  - env-mapped legacy content derives spec/rough from `envLevel` (`spec0 = mix(0.04, 0.98, min(8·envLevel, 1))`)
  - metallic = `clamp(10·spec − 0.4)`
  - specular colour = white scaled by albedo brightness for non-metals, albedo for metals
- **Vertex-lit path** (quality 0 or enhanced lighting off): per-vertex lighting plus the baked static light `vColor`; env maps lerp the texture (the classic 1.69 chrome look).
- **Height maps:** iterative parallax (16/32 steps); AO from height multiplies ambient and env specular.
- **Attenuation constants:** the settings are max lights 32, cutoff-range multiplier 2.0, max intensity 1.5, intensity at range 0.2. [I] Derived uniforms: `R_cut = radius·2`, `lightMaxIntensityInv = 1/1.5`, `lightFalloffFactor ≈ 12.33`. The in-shader formula is exact; this mapping is inferred.

### 6.4 Light sources: data → shader
- **Sun/moon:**
  - ARE `Sun*`/`Moon*` `AmbientColor`/`DiffuseColor` (0x00BBGGRR /255; [I] linearised in enhanced mode).
  - `DayNightCycle=1` → day uses Sun, night uses Moon, with transitions; else `IsNight` picks.
  - Legacy/toolset form: `gidy_sun.mdl` lights `gidy_sun_amb` (ambient-only) and `gidy_sun_diff`, recoloured via `controlpart … color r g b` [T]. Default direction ≈ normalize(4000, 4500, 7000) [I].
  - Scripts can move the sun (`SetAreaLightDirection`, `nw_dynlight`).
- **Tile main lights:** `Tile_MainLight1/2` → lightcolor.2da RGB applied to nodes `<tile>ml1/ml2`. The node's radius and multiplier are kept (base data: radius 14 / 5, shadow 0; the wiki claims engine overrides, which is unverified). Point lights in enhanced mode; baked into `vColor` in vertex mode (Aurora: "Compute Static Lighting").
- **Tile source lights:** `Tile_SrcLight1/2` → `fx_flame01.mdl` at `sl1/sl2`, playing animation n (its light: radius 7, colour ≈ lightcolor row 2n).
- **Placeables:** placeables.2da `LightColor` → `fx_placeable01.mdl` (radius 10) at `LightOffset`.
- **MDL light nodes:** `radius`, `multiplier`, `color`, `ambientonly` (→ negative a), `isdynamic`, `affectdynamic`, `shadow`, `shadowradius`, `lightpriority` (1 sun … 5 others; used to trim to MAX_NUM_LIGHTS), `fadinglight`.
- The player always carries one light (progfx.2da).

### 6.5 Fog, sky, shadows, post
- **Fog:**
  - linear by view depth: `f = clamp((−z − start)/(end − start))`, applied in gamma space
  - `end ≈ FogClipDist` (default 45), also the tile/static render distance
  - a skybox adds +90 m of render distance
  - dynamic placeables/doors 45 m, creatures 35 m
  - `start` comes from `Sun/MoonFogAmount` (0–15): **the formula is unknown** (measure in game)
- **Skybox:** skyboxes.2da DAWN/DAY/DUSK/NIGHT models via `vsvt/fst` (never fogged), centred on the viewer; `skyfade1.mdl` blends fog at the horizon.
- **Shadows:** stencil shadow volumes from `shadow 1` trimeshes (often `render 0` proxies) extruded on the GPU, plus a darkening plane (`ShadowOpacity`). Casters: the sun (static projections), plus ≤3 dynamic `shadow 1` lights. `beaming` meshes render as translucent light shafts. **Optional for an editor.**
- **Post pass** (`fsfbpostpr`, always runs): sharpen → DoF → toon → dynamic contrast → vibrance → gamma (`pow(rgb, Gamma/2.2)`, 2.2 = identity) → **ColorClamp tonemap**:
  ```
  if M = max(rgb) > 1: rgb = M − (M − rgb)(1 − ((M−1)/M)²)   (+ overflow into alpha for transparents)
  ```
  SSAO and HDR bloom are separate passes (game only).
- **Keyholing:** a screen-space dissolve circle around the player (noise, dark rim). Not needed in an editor; distinct from MDL `tilefade` (Hide Second Story, used by Aurora's Tile Fade modes).
- **Grass:** engine-generated quads on material-3 walkmesh faces; lit without N·L (× 1/π); SET `[GRASS]` params.
- **Water:** noise-based waves, wind, SSR/refraction (game only; the toolset uses ordinary shaders).
- **Texture animation:** TXI procedural textures (cycle flipbooks, arturo distortion) are CPU-side.
- **Transparency:**
  - alpha test 0.2 by default; mesh `alpha`; TXI `blending additive/punchthrough`
  - `transparencyhint` 1–9 orders static transparent meshes (no auto-sort)
  - a-node children render dynamic/late
  - MTR `transparency`/`twosided`/`sample_framebuffer`

### 6.6 Recommended Moonglow renderer plan
1. **Phase 1:**
   - GL 3.3 core (or Vulkan/WebGPU via cross-compiled GLSL). Load stock `vslit/fslit(_nm/_sm)` from ResMan with the engine preamble, `GAMMA_CORRECTION=1`, `FRAGMENT_LIGHTING=1`, `SHADER_QUALITY_MODE=2`, `KEYHOLING_ENABLED=0`, `MAX_NUM_LIGHTS=32`.
   - Supply the uniforms in §6.2 (view-space lights).
   - Render linear, gamma out, fog in gamma space, ColorClamp.
2. **Lights:** area sun/moon, tile ml1/ml2 + sl1/sl2 (fx_flame01), placeable lights, MDL lights; per-object top-32 by priority and distance. Offer "day/night" and "use area lighting" toggles, like Aurora's `miUseAreaLighting`.
3. **Later:** grass, stencil shadows, skybox + skyfade, emitters, water.
4. **Shader licensing:** the shaders are game assets. Load them from the user's install at runtime; never redistribute them.

Open items to measure in game: fog amount → start; colour space of uploaded ARE/lightcolor/material colours; default sun direction; whether Aurora uses enhanced or vertex static lighting; cubemap face order; the ml1/ml2 override question.

---

## 7. Script compilation

Sources: `~/.local/opt/neverwinter/bin/nwnscriptcomp.h`, nwn.py `nwscript/comp.py`, a local test compile [V], [CL], https://nwn.wiki/spaces/NWN1/pages/109674520/Script+Editor.

### 7.1 libnwnscriptcomp C API
- **Build:** neverwinter.nim 2.3.1 (ABI **1**).
- **Platforms:** nwn.py bundles `linux/macos/windows × x86_64/aarch64`.
- **Dependencies:** only libc, libm, libpthread, libdl.
- **Licence:** open-sourced by Beamdog in 88.8193.36 inside neverwinter.nim (an MIT repo); confirm the compiler's own licence file before bundling.
- **Threading:** not thread-safe; use one instance per thread.

| Call | Notes |
|---|---|
| `int32 scriptCompApiGetABIVersion()` | must be 1 |
| `CScriptCompiler* scriptCompApiNewCompiler(int src=2009, int bin=2010, int dbg=2064, ResManWriteToFile, ResManLoadScriptSourceFile)` | |
| `void scriptCompApiInitCompiler(c, "nwscript", bool writeDebug, int maxIncludeDepth=16, const char* graphvizOut=NULL, const char* outputAlias="scriptout")` | immediately requests langspec `nwscript` (restype 2009) through the load callback; it must be served |
| `NativeCompileResult scriptCompApiCompileFile(c, "resref")` | name **without** extension. Returns `{code, str}`: 0 = OK, else a negative TLK strref and a static message, e.g. `-622 "t_bad.nss(1): ERROR: UNDEFINED IDENTIFIER (UndefinedFn) [...]"`. **First error only** (same as Aurora) |
| `bool (*ResManLoadScriptSourceFile)(const char* resref, RESTYPE)` | call `scriptCompApiDeliverFile(c, data, size)` inside it; return false if not found |
| `int32 (*ResManWriteToFile)(const char* name, RESTYPE, const uint8* data, size_t, bool binary)` | called only on success: NCS (binary) and NDB (text); return 0 |
| `Get/SetOptimizationFlags` | bitmask 1 dead functions, 2 meld instructions, 4 dead branches. Header default = everything; **`-O1` (dead code only) is "as used by the game and toolset"** → set 1 explicitly |
| `SetGenerateDebuggerOutput(bool)` | NDB on/off |
| `SetRequireEntryPoint(bool)` | false = validate include files (no code generated) |
| `DestroyCompiler` | |

- **Output formats:** `NCS V1.0` binary; `NDB V1.0` text (functions, structs, variables, line map).
- **Limits** (87.8193.35): 65,536 identifiers; 8,192-char string constants; 512 include files; include depth 16.
- **EE language additions** for the editor's highlighter, linter and completion:

  | Version | Addition |
  |---|---|
  | 85 | `\xFF` escapes |
  | 87 | string `case` labels (HashString) |
  | 88 | `r"…"`/`R"…"` raw strings (multi-line); `0f`/`.42f` floats; `const` constant expressions; non-int for-loop clauses |
  | 89 | `h"…"` hashed strings; `__FUNCTION__ __FILE__ __LINE__ __DATE__ __TIME__`; unary constant folding |
  | 37-16 | `0b`/`0o` literals; unary `+` |
  | engine types | `effect, event, location, talent, itemproperty, sqlquery, cassowary, json` |

  Recompile if bytecode was produced by 37-15 (miscompile fixed in 37-16).

### 7.2 nwscript.nss and includes
- **Source:** the effective langspec comes from `nwn_retail.key`. `ovr/nwscript.nss` is a reference copy (same text, CRLF). **Never** resolve from `nwn_base.key` alone (1.69-era signatures).
- **37-17 contents:** 1,187 engine functions (action IDs 0–1186), 6,201 constants, 8 `#define ENGINE_STRUCTURE_n`. Oracle parser: `nwn.nwscript.langspec.read()`.
- **Aurora's script editor** uses it for:
  - the Functions/Constants lists and help (the `//` block above each prototype)
  - F2 autocomplete
  - bold custom symbols
- Includes resolve through ResMan; hak scripts beat module copies.
- New conversation conditionals default to `int StartingConditional()`. The templates tab reads text files from a user directory (examples ship in `data/scr`).
- EE options: an external editor (87) and "build this script" from the tree (87).
- **Runtime:** `CompileScript()` (87) and `ExecuteScriptChunk()` (v79) use the same compiler in game.
- **Oracles:** `nwn_script_comp`, `nwn.nwscript.comp.Compiler`, `nwn_asm -d`, `nwn.nwscript.vm`.

---

## 8. Test Module (F9)

Sources: https://nwn.wiki/spaces/NWN1/pages/60982744/Test+Module, https://nwn.wiki/spaces/NWN1/pages/38176945/Toolset+F9+Testing+Issues, https://nwn.wiki/spaces/NWN1/pages/139689989/NWMain, [T], [CL].

- **Aurora's launch line:** `nwmain.exe -userdirectory "%s" +TestNewModule "%s"` (format string in `nwtoolset.exe`; `-userdirectory` pass-through fixed in v79).
- **`+TestNewModule "<module>"`:** loads straight into single player with the **first local-vault character**, at the module start location. "Test here" = temporarily move `Mod_Entry_*`.
- **`+LoadNewModule "<module>"`:** stops at character select (to test chargen/classes).
- Neither works with `-dmc`.
- **Argument:** the wiki pages disagree (module *name* vs *file name*). Use the .mod basename (as for `StartNewModule`) and verify empirically.
- **Other nwmain flags** [T/W]: `-dmc`, `+connect ip:port`, `+password`, `+connect_lobby`, `+gogfriend`, `-noaliases`, `-userdirectory`, `compilemodel <resref>` (ASCII→binary MDL into the `MODELCOMPILER` alias directory; not skin meshes).
- **Headless alternative:** `nwserver -module <name> -userdirectory DIR [-port -servervault -maxclients -pvp -difficulty -reloadwhenempty -interactive -quiet -moduleurl -modulehash …]`, then `nwmain +connect 127.0.0.1:5121`.
- **Known F9 problems:**
  - combat/AI timer "lag" and AI-update overflow warnings
  - a crash during F9 can corrupt the module (save *before* F9, not from the prompt)
  - performance with a full-screen client (#314)
  - community advice: keep a windowed client and restart via `StartNewModule(GetName(GetModule()))`
- **Moonglow design:**
  1. Save and pack.
  2. Copy/symlink the .mod into `<userdir>/modules/`.
  3. Spawn the platform binary: `bin/linux-x86/nwmain-linux`, `bin/linux-arm64/…`, `bin/macos/nwmain.app/…`, or `bin/win32/nwmain.exe`.
  4. Pass a user-selected `-userdirectory` (default: a test user dir, not the player's).
  5. Offer "test at camera" (temporary `Mod_Entry_*`), "test with character select", and "run in nwserver + connect".
  6. Tail the game log for script errors.

---

## 9. Known Aurora bugs and limitations, and resource limits

Sources: https://nwn.wiki/spaces/NWN1/pages/38175555/Common+Errors+and+Their+Causes, https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits, https://nwn.wiki/spaces/NWN1/pages/26738699/Aurora+Toolset, Toolset User Manual subpages, https://nwn.wiki/spaces/NWN1/pages/26738852/Placeables+and+the+Toolset, github.com/Beamdog/nwn-issues (**open** issues labelled toolset, queried 2026-09-30), [CL].

### 9.1 Limits to respect

| Limit | Value | Source |
|---|---|---|
| Resref (any resource, hak and TLK names) | ≤ 16 chars, lowercase advised; 17 can crash the game | W: HAK |
| GFF label | ≤ 16 chars | [V] |
| ERF entries | unlimited since 1.80.8193.14 (was 16,384); 1.69 toolset silently lost resources | W: Resource Limits |
| Hak size | < 2 GB (32-bit toolset, nwsync) | W |
| NWSync single file | 64 MB (was 15 MB) | CL 87 |
| Body/armor part ids | 0–999 (x-fields) | CL 87/88 |
| Classes per creature | 8 (ruleset `MULTICLASS_LIMIT`; toolset since 37-13) | CL |
| In-use tilesets per module | no limit since 37-13 (was 100) | CL |
| Area size (wizard) | 32×32 tiles | W, [V] |
| Tile lights | 2 main + 2 source per tile | W |
| Script compiler | 65,536 identifiers, 8,192-char strings, 512 includes, depth 16 | CL 87 |
| Bones | 64 per skin node, 4 weights per vertex | CL 1.83 |
| Faces per mesh | 21,845 (87+); models with more than 10k faces crash Aurora tile painting | CL, W |
| Node / animation names | ≤ 32 / ≤ 16 chars | W |
| Dynamic lights | 32 default (3–128); 0–3 shadow casters | W: Area Lighting |
| surfacemat.2da | 64 rows | CL 1.84 |
| Item charges | 250 | v74 |
| Descriptions | Aurora truncated > 2,000 chars before 1.80.8193.6 | CL |
| Walkmesh | ≤ 8 faces per vertex; tile walkmesh must stay within ±5 m | W |
| DM palette | 1 MB limit of 1.69 removed | W |
| Custom TLK | exactly one per module, from `tlk/` | W: TLK |

### 9.2 Aurora bugs and limitations (what Moonglow should do instead)

**Architecture and UX**
- 32-bit, Windows-only VCL app, runs under Wine/Proton. Modal single-object property dialogs (you can't open two creatures or journal + conversation at once).
- Area tabs are unstable. The camera is anchored at z = 0, with no WASD. The mouse wheel moves the camera regardless of cursor position (#458). Creature properties open behind the main window (#230).
- DPI scaling works only at 100/150/200%. Discord/Logitech overlays crash it. Nvidia "threaded optimisation" and AMD vsync cause crashes and camera speed problems (older builds).

**Data safety**
- F9 can corrupt the module. Crash recovery only via `temp0`/`.BackupMod`.
- Unknown GFF fields are dropped (#528). TLK strrefs are overridden by text (#370). Changes in Area Properties are forgotten (#688).
- Scroll-wheel scaling didn't mark the area dirty (probably fixed in 85.32, where it gained undo and "modified on undo-stack change"). Opening a SAV as MOD wipes wizard known-spell lists.
- The spell checker **wipes the text box** (disable it). Conversation undo is almost useless; scrap and bookmark lists are lost on close and bookmarks show only the first letter.
- Cloning an area duplicates tags, which confuses transitions. Deleting a blueprint can remove its instances.

**Crash-on-bad-content** (validate and report instead):
- SET: TILE/GROUP counts vs sections ("Range Check" → AV); group tile index out of range; a missing tile model when painting a group; `Transition=0`; group Tile0 not unique; missing custom tileset doors in doortypes.2da.
- An ARE that references an uninstalled tileset ("List Index Out of Bounds (60)").
- Extra lightcolor.2da rows (Tile Properties AV). baseitems.2da > 255 rows or malformed rows. `MinRange=100` (#658).
- MTR texture names > 16 chars (EEFFACE → pure virtual call → AV in soft_oal.dll).
- Missing placeable/creature models (subsequent objects vanish from the view); a creature with an invalid class.
- EEFFACE while scrolling creature appearances (memory leak, #369); a hak open in nwhak (#82); rotating areas (#766); TTS02 dragon-skeleton group removal (#608).

**Rendering and preview**
- White polygons on some tiles (#765). Crash when an area is shown on some GPUs (#696, #791).
- Only pmh0/pfh0 textures for skinmesh creatures (#429). Helmets on the floor render white (#253). Lights on player part models crash (#255).
- PLT layered item icons are wrong (#456); icons don't load from DDS and need a "valid" TGA (#455). The PLT colour preview needs "Use Environment Mapping on Creatures and Items" enabled.
- SourceLights defined as `light` nodes stay on. Model parts appeared rotated (fixed 37-15).

**Editors and data**
- Hak reload is broken (#346). Invalid tiles when resizing the rural tileset (#324); Medieval City 2 door/terrain issues (#329); Barrows geometry missing after undo (#271); fortress stairs door hookpoints (#214); Castle Interior 2 gate missing (#88).
- The start point can move outside area bounds (#639). Focusing a creature in an unopened area highlights a random placeable (#365).
- Creature wizard class list sorting and `--invalid--` entries (#587). Custom spellcasters' spells cannot be assigned (#566). Epic multiclass saves are computed wrongly (#816). The custom damage-type VFX doesn't show (#530).
- Inventory is very slow (#368); odd potion stack sizes (#375); Find doesn't work (#381); F1 help in the Properties tab (#436); a freeze on area properties (#374).
- Ambient music stops on zoom (#206). Polish letters are truncated (#534). Windows' "UTF-8 beta" setting garbles the script editor.
- `Mod_DefaultBic`, `Mod_PartyControl` and `TileBrdrDisabled` have **no UI** (preserve-only). Placeable visual transforms reset when Static is set. Scaling only works on non-static placeables.

**Recommended Moonglow policies**
- Never drop unknown fields.
- Validate SET/2DA/MDL/MTR on load with actionable errors.
- Autosave/journal edits.
- Real undo everywhere.
- Multi-window editing.
- UTF-8 UI ↔ codepage conversion per language.
- Hot reload of haks and `development/`.
- Expose the EE-only fields.
- A deterministic "Update Instances" diff preview.

### 9.3 Content quirks to tolerate (seen in shipped data)
- SET: junk `WalkMesh` and light flags; `Doors=` ≠ door sections (50 tiles); undefined terrain names; LF vs CRLF.
- MDL: `setfillumcolor`, 4-component colours, `true/false`, garbage pointers and pads, `tilefade 4`, source lights as light nodes, ASCII models with wrong counts.
- GFF: language ids 256–263, empty-resref ERF entry, `Repute.fac` / `Module.ifo` capitalisation, `VisualTransform` vs `VisTransformList`, legacy `Mod_Hak`, BTC/BTI… blueprint magics.
- Textures: 32-bpp TGAs with 0 declared alpha bits, grey TGAs, NPOT images, the BioWare DDS "4 channels" being DXT5.

---

## Appendix A. Aurora feature inventory (action names in `nwtoolset.exe`) [T]
A parity checklist:

| Area | Actions |
|---|---|
| File / module | New, Open, Save, SaveAs, SaveAll, Build, **Test**, VerifyModule, VerifyArea, FileImport/FileExport (ERF), ExportResource, CopyResource, DeleteResource, UpdateResources, SaveToSavegame |
| Area editor | AreaResize, AreaRotate, AreaImport/Export, TileProperties, **ComputeStaticLighting**, SelectTerrain, SelectObjects, SelectToggle, PolygonRedraw, LocationAdjust (visual transforms), InstanceProperties/Delete, NewTemplateFromInstance, AddToPalette, ApplyTemplate, RescanInstances (Update Instances), ShowAreaStats, WaypointCreateSet, EncounterAddSpawnPoint, CreatureAddWaypoint, DoorReverse, ViewFullScreen, camera and pane toggles |
| Previews | AnimateClosed/Closing1/2/Opened1/2/Opening1/2 (doors), AnimatePlaceable{Activate, Closed, Deactivated, Default, Destroyed, Open}, AnimateStop, PreviewAmbientMusic/Sound, PreviewPlacedSounds, SoundTurnOn/Off |
| Wizards | Area, Creature, CreatureLevelup, Door, Encounter, Item, Module, Placeable, **Plot**, Script, Sound, Store, Trigger, Waypoint |
| Editors | Conversation (Add, Cut, Copy, Paste, **PasteAsLink**, Action/Condition scripts, EndConv/Abort scripts, tokens, bookmarks, collapse/expand, import/export dialog), Script (Compile, FindInFiles, bookmarks, templates), Faction (Add/Remove/Rename), Journal, Inventory, Variables, SpellCheck |

## Appendix B. 2DAs referenced by `nwtoolset.exe` [T]
Names intersected with base-game 2DAs:

actions, ambientmusic, ambientsound, appearance, armor, baseitems, bodybag, capart, categories, classes, cloakmodel, cls_atk_1..3, cls_spgn_*, cls_spkn_*, creaturesize, creaturespeed, crtemplates, cursors, domains, doortypes, encdifficulty, environment, feat, fractionalcr, gender, genericdoors, iprp_* (alignment, ammotype, costtable, damagecost, meleecost, paramtable, spelllvcost, spells, visualfx), itempropdef, itemprops, itemvalue, itmwiz* (ammo, armor, helmet, melee, potion, ranged, rods, scroll, staves, throw, trap, wands), **lightcolor**, loadscreens, metamagic, namefilter, packages, parts_* (belt, bicep, chest, foot, forearm, hand, legs, neck, pelvis, robe, shin, shoulder), phenotype, placeables, placeabletypes, portraits, prioritygroups, pvpsettings, racialtypes, ranges, replacetexture, repute, skills, skillvsitemcost, skyboxes, soundcatfilters, sounddefaultspos, sounddefaultstim, soundeax, soundset, soundsettype, soundtypes, spells, statescripts, stringtokens, surfacemat, tailmodel, **tilecolor**, traps, treasurescale, visualeffects, waypoint, wingmodel.

Plus the per-tileset `<ts>_edge.2da` files (game), `areag.ini` and the `*pal*.itp` palettes.

## Appendix C. Tools and oracles available locally
- **neverwinter.nim 2.3.1** (`~/.local/opt/neverwinter/bin`): `nwn_gff`, `nwn_erf` (V1/E1), `nwn_key_*`, `nwn_twoda`, `nwn_tlk`, `nwn_ssf`, `nwn_script_comp`, `nwn_asm`, `nwn_resman_*`, `nwn_nwsync_*`, `nwn_compressedbuf`, `libnwnscriptcomp.so` + `nwnscriptcomp.h`.
- **nwn.py 0.0.23** (venv): `gff`, `erf` (V1.0 only), `key`, `twoda`, `tlk`, `ssf`, `tileset` (SET), `nwscript.{comp, asm, ndb, langspec, vm}`, `resman`, `environ`.
  - Caveat: it crashes on language ids 256–263.
- **Also:**
  - nasher 1.1.3 and nwn_nwnt: module ↔ text trees.
  - Neverblender (`~/Projects/neverblender`): ASCII MDL I/O, AABB builder, a one-tile test tileset generator (`tools/e2e/build_test_tileset.py`), and vendored nwnmdlcomp for binary→ASCII.
  - Game binaries: `nwmain compilemodel` (ASCII→binary).
- **Scratchpad prototypes from this research:** `scripts/mdlbin.py` (binary MDL reader), `tilesets/*.py` (SET statistics, area/door/group verification), `gff/scan.py` (field union).

## Appendix D. nwn.wiki pages cited
Tables: GFF https://nwn.wiki/spaces/NWN1/pages/38175366/GFF

- **Toolset:**
  - Aurora Toolset /26738699/Aurora+Toolset
  - Toolset User Manual /60982732/Toolset+User+Manual
  - Area Editor /60982635/Area+Editor
  - Conversation Editor /60982751/Conversation+Editor
  - Journal Editor /109674497/Journal+Editor
  - Module Creation Wizard /60982735/Module+Creation+Wizard
  - Script Editor /109674520/Script+Editor
  - Test Module /60982744/Test+Module
  - Toolset Options /60982804/Toolset+Options
  - Toolset Palette ITP /26738834/Toolset+Palette+ITP
  - Toolset Import and Export /38175055/Toolset+Import+and+Export
  - Area Lighting /38174907/Area+Lighting
  - Resource Limits /26738887/Resource+Limits
  - Common Errors /38175555/Common+Errors+and+Their+Causes
  - F9 Testing Issues /38176945/Toolset+F9+Testing+Issues
  - Open Module Folder /38176075/Open+Module+Folder
  - Placeables and the Toolset /26738852/Placeables+and+the+Toolset
  - Advanced Area Creation Tips /26738846/Advanced+Area+Creation+Tips
- **Loading:**
  - Content Load Order /38174823/Content+Load+Order
  - HAK /53670235/HAK
  - userpatch.ini /53671463/userpatch.ini
  - TLK /38176005/TLK
  - Modules /155516929/Modules
  - NWMain /139689989/NWMain
- **Formats:**
  - File Format Specification (Bioware) /327727/File+Format+Specification+Bioware
  - 2da Files /38174875/2da+Files
  - SSF /53670746/SSF
  - LTR /38176121/LTR
  - encoding.2da /98074626/encoding.2da
  - Sounds and Music /38174892/Sounds+and+Music
- **Tilesets:**
  - SET /38175567/SET
  - Tilesets /38175063/Tilesets
  - Tileset Construction Tutorial /72417345/Tileset+Construction+Tutorial
  - Common Issues with Tiles /72417367/Common+Issues+with+Tiles+and+Tilesets
  - Tile Path Nodes /139689996/Tile+Path+Nodes
  - Walkmesh Notes /179077218/Walkmesh+Notes
  - Area Tilesets Information /38175219/Area+Tilesets+Information
  - areag.ini /72417335/areag.ini
  - DWK /91324443/DWK
  - PWK /60985376/Placeable+Walkmesh+PWK
  - Claude Code subtree: /195362833/SET+File+Format, /195362837/Tileset+Operations, /195362835/ITP+Palette+Format
- **Models and textures:**
  - MDL /38175669/MDL
  - MDL ASCII /12027273/MDL+ASCII
  - Model Table of Parameters /53671005/Model+Table+of+Parameters
  - Model Special Nodes /38176272/Model+Special+Nodes
  - Models /38175602/Models
  - Animations /38175170/Animations
  - Model Shadows /49447442/Model+Shadows
  - LOD /26738916/LOD+-+Level+of+Detail
  - Part-Based Models: Creatures /49447501/Part-Based+Models+Creatures
  - Part-Based Models: Items /60985168/Part-Based+Models+Items
  - Layered PLT Items /60985278/Layered+PLT+Items
  - PLT /14618045/PLT
  - Textures /38174958/Textures
  - TGA /53672779/TGA
  - DDS /3473496/DDS
  - KTX /133988392/KTX
  - TXI /38174929/TXI
  - MTR /12027232/MTR
  - appearance.2da /38174941/appearance.2da
  - phenotype.2da /38176344/phenotype.2da
  - baseitems.2da /38174935/baseitems.2da
  - placeables.2da /38175210/placeables.2da
  - genericdoors.2da /38175673/genericdoors.2da
  - cloakmodel.2da /53671311/cloakmodel.2da
  - Door /38174806/Door
- **Rendering:**
  - Enhanced Lighting Engine and PBR /38175899/Enhanced+Lighting+Engine+and+PBR
  - Shaders /60981936/Shaders
  - Shader Engine Support /14614573/Shader+Engine+Support
  - Shaders and Area Flags /65470710/Shaders+and+Area+Flags
  - Environment Maps and Cubemaps /38174773/Environment+Maps+and+Cubemaps
  - Standard material inputs /38175898/Standard+material+inputs
  - Render Distance with Fog and Skyboxes /38175000/Render+Distance+with+Fog+and+Skyboxes
  - skyboxes.2da /53670635/skyboxes.2da
- **Patches:**
  - Patches NWN:EE /26738926/Patches+NWN+EE
  - Major Changes in NWN:EE /26738928/Major+Changes+in+NWN+EE
  - per-build pages are listed in `notes_gff_changelog.md` §D

All paths are relative to `https://nwn.wiki/spaces/NWN1/pages`.
