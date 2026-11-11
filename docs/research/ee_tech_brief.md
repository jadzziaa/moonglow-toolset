---
type: Research Note
title: 'Moonglow Toolset: NWN:EE technical research brief'
description: Technical brief on NWN:EE 89.8193.37 for the toolset - file formats, GFF resource types, resource loading order, script compilation, Test Module, Aurora's known bugs and limits, each claim tagged with its evidence; tilesets, models and rendering are short sections pointing at the detail notes and the findings.
tags: [nwn-ee, formats, gff, rendering, tilesets, aurora]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-11T00:20:32Z }
---

# Moonglow Toolset: NWN:EE technical research brief

- **Target:** NWN:EE **89.8193.37-17** (Steam, Linux install; `bin/win32/build.txt` commit 26c6e57, 2025-10-06). Compiled 2026-09-30.
- **Evidence tags:**
  - **[V]** verified against shipped game data or binaries in this research
  - **[W]** nwn.wiki (local mirror; URLs are in each section and in Appendix D)
  - **[CL]** the game's changelogs (`lang/en/docs/CHANGELOG.md`, `patchnotes/*.md`, `Neverwinter Nights Enhanced Edition (v74..v79).txt`, `legacy/NWNv169.txt`)
  - **[T]** strings in `nwtoolset.exe` / `nwmain` / `nwserver`
  - **[I]** inference (not verified)
- **Detail notes** (same folder). Sections 4, 5 and 6 of this brief were cut down to pointers at them in October 2026; sections 1–3 and 7–9 and the appendices stand on their own:

  | File | Contents |
  |---|---|
  | `notes_gff_changelog.md` | GFF fields in toolset write order, struct IDs, and about 150 dated EE toolset changelog items |
  | `notes_tilesets.md` | SET keys, verification statistics, path-node table, terrain painting as captured from Aurora |
  | `notes_models.md` | binary MDL byte layout, animation tables, PLT, textures |
  | `notes_shaders.md` | full uniform tables and lighting math |

- **What was measured later** in the game, the client and Aurora is in `../findings.md`; where it differs from this brief, the findings are right.
- The extracts and working scripts of this research (shader, tileset and model extracts, the Python binary MDL reader, the GFF field union) were not kept. Their place is taken by Moonglow's own readers and the corpus tests in `crates/mg-corpus-tests`, and by the generated `gff-observed-schema.md`.

---

## 0. Decision-relevant findings

1. **The Aurora toolset renders with the game's own GLSL.**
   - It is still a 32-bit Windows VCL app (`nwtoolset.exe` is PE32).
   - It loads `.shd` resources through ResMan (restype 2069) and prepends the same `#define` preamble as `nwmain` [T]. The pairs it uses are `vslit/fslit`, `*_nm`, `*_sm`, `vsvt/fst`, grass, particles, shadow volumes and `fsfbpostpr`.
   - All 92 stock shaders ship as readable GLSL 330 source.
   - Moonglow does not run them: its renderer is wgpu with its own shaders, which follow the lighting model read from the stock ones (`notes_shaders.md`) and are compared with the game client (`../PLAN.md` §5.5). Running the stock shaders stays a later option there.
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
   - Every data-level rule is verified against the official areas (439 in `../findings.md` "The tile grid"; this research's first pass listed 441):
     - index → cell mapping
     - CCW quarter-turn orientation
     - absolute corner heights
     - group layout and rotation
     - door hookpoint transform
   - The **interactive painting algorithm is undocumented** (rule propagation, raise/lower footprint, crosser drag, eraser). It was captured by scripting Aurora under Wine and comparing `Tile_List`: `notes_tilesets.md` §B.4a–B.4b, `../findings.md` "Aurora's terrain painting", test `aurora_terrain.rs`.
5. **Models.**
   - 22% of base MDLs are **ASCII**, including player skeletons (`pmh0`…) and many EE tilesets. Moonglow needs ASCII and binary loaders feeding one data model.
   - The binary layout is fully mapped and validated on all 25,597 base binaries:
     - the node type comes only from `flags@+0x6C`
     - orientation is a quaternion
     - mesh alpha is controller 128
   - Animations are found by name through supermodel chains; a compiled model's nodes bind by part number, a text model's by name (`notes_models.md` §B.19).
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
| MDL (2002) | all 3D; ASCII and binary | R (ASCII W optional) | Neverblender (ASCII); vendored **nwnmdlcomp** (binary→ASCII: `~/Projects/neverblender/build/third_party/nwn-tools`); `nwmain compilemodel <resref>` (ASCII→binary) | §5 |
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
| **DLG** | conversation | root: `DelayEntry`, `DelayReply`, `NumWords` (recomputed on save), `EndConversation`, `EndConverAbort`, `PreventZoomIn`, `EntryList`/`ReplyList`/`StartingList`. Nodes: `Speaker` (entries only), `Animation`, `AnimLoop`, `Text` (Loc), `Script`, **`ActionParams` (EE)**, `Delay`, `Comment`, `Sound`, `Quest`, [`QuestEntry`], `RepliesList`/`EntriesList`. Links: `Index`, `Active` (condition), **`ConditionParams` (EE)**, `IsChild`, [`LinkComment`]. Param element = {`Key`, `Value`} CExoString [T; confirmed by saving one in Aurora and running it in the engine: `../findings.md` "Conversation script parameters"] |
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
| UTI | per-part armour colours `APart_<part>_Col_<ch>` | B | runtime (engine string) | not toolset-written; name and type confirmed in the engine (`../findings.md` "An armor's per-part colors") |

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
  - The game looks in the module's haks first, then the module, then user `tlk/` (engine test `engine_tlk.rs`; the wiki's TLK page said a TLK inside a hak is not recognized) [W: TLK https://nwn.wiki/spaces/NWN1/pages/38176005/TLK].
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

A tileset is a SET file (INI text) of tiles with corner terrains, corner heights and edge crossers, plus groups, a palette (`<tileset>palstd.itp`), tile models with walkmeshes, and an `_edge.2da`. An area stores one `Tile_List` entry per cell (tile index, quarter turns counter-clockwise, height, lights, animation loops), row-major from the south-west. The detail is in `notes_tilesets.md`; this section only says where.

| Was here | Now in |
|---|---|
| 4.1 SET format | `notes_tilesets.md` §A (keys, section order, tolerances, base-set counts) |
| 4.2 Corner/edge model | `notes_tilesets.md` §B.1–B.2; `../findings.md` "The tile grid" |
| 4.3 ARE tile data and world mapping | `notes_tilesets.md` §C; `notes_gff_changelog.md` §B2; `../findings.md` "The tile grid", "Door hooks, and the doors Aurora places itself", "Resize Area and Rotate Area", "ARE colors and tile lights, stored and read" |
| 4.4 Painting operations | `notes_tilesets.md` §B.3 (as documented) and §B.4a–B.4b (as captured from Aurora); `../findings.md` "Aurora's terrain painting" |
| 4.5 Palette | `notes_tilesets.md` §F.2 |
| 4.6 Walkmesh and pathing | `notes_tilesets.md` §D |
| 4.7 Lights, animations, edges, minimap | `notes_tilesets.md` §B.5, §E, §F.1; `../findings.md` "Point lights", "Minimaps" |
| 4.8 Wiki claims found wrong | `notes_tilesets.md` §F.3 |
| 4.9 Reverse-engineering checklist | done: `notes_tilesets.md` §B.4a–B.4b; what is left open is in its §I |

Measured later, and different from what the documents say (`../findings.md`): the Eraser does not paint the `Default` terrain; primary rules reach all eight neighbors by relative height and do not chain; Resize does not copy edge tiles; a tileset's `Doors=` and `Count=` values are not to be trusted; the engine forces a tile main light's radius to 10 (ml1) and 5 (ml2).

Moonglow: `crates/mg-set` (SET), `crates/mg-tiles` (the corner lattice and painting), `crates/mg-area` (tile data, resize, rotate).

---

## 5. Models, animations, part-based creatures, textures

The base game ships 32,832 models, 7,235 of them ASCII and 25,597 binary (a 32-bit little-endian memory image), so a reader needs both loaders feeding one data model. Creatures with appearance.2da `MODELTYPE` P are assembled from a skeleton and parts, colored through PLT textures. Textures are TGA, BioWare or standard DDS, and PLT, with TXI and MTR directives. The detail is in `notes_models.md`.

| Was here | Now in |
|---|---|
| 5.1 MDL ASCII | `notes_models.md` §A |
| 5.2 Binary MDL | `notes_models.md` §B (byte layout, controllers, reader recipe in §B.21); `../findings.md` "Binary models from the game's own compiler" |
| 5.3 Animations the editor must play | `notes_models.md` §C (per object type in §C1, supermodels and animation scale in §C2); `../findings.md` "A worn part moves at its wearer's animation scale" |
| 5.4 Part-based creatures | `notes_models.md` §D1–D2; `../findings.md` "A body part is drawn in its own PLT", "A skin mesh in a body part or a held item", "EE \"x\" twin part fields" |
| 5.5 PLT coloring | `notes_models.md` §D3; `../findings.md` "An armor's per-part colors" |
| 5.6 Textures | `notes_models.md` §E (lookup, TGA, DDS, KTX, TXI, MTR), §D4–D5 (items, placeables, doors); `../findings.md` "A TGA's origin bits are ignored", "Two-sided faces, cut-outs and decals" |

Measured later, and different from the first reading (`../findings.md`, `notes_models.md` §B.19–B.20): the game binds a compiled model's animation nodes by part number, not by name; a body part is drawn with the texture of its own name where one exists, whatever its mesh names; the game's compiler stores `materialname` in the texture3 slot and tangents with a render hint; a TGA's origin bits are ignored.

Moonglow: `crates/mg-mdl` (both readers, the compiler and decompiler), `crates/mg-image` (TGA, DDS, PLT, TXI, MTR), `crates/mg-preview` (creature, item, placeable and door composition).

---

## 6. EE lighting and rendering model (from the effective stock shaders)

The game and Aurora render with the game's own GLSL (92 effective `.shd`, restype 2069): lighting in view space, linear and unclamped when enhanced lighting is on, a GGX specular term, environment maps driven by the diffuse alpha, fog applied after the return to gamma space, and one post pass with a color clamp. The lighting model is read from those shaders in `notes_shaders.md`.

**Decision taken:** Moonglow's renderer is wgpu with Moonglow's own shaders, which approximate this model and are compared with screenshots of the game client; running the stock shaders is not a goal (`../PLAN.md` §5.5). The game's shaders are game assets: they are only ever read from the user's install, never shipped.

| Was here | Now in |
|---|---|
| 6.1 Programs and selection | `notes_shaders.md` §A |
| 6.2 Attributes, uniforms, texture units | `notes_shaders.md` §B, §C |
| 6.3 Lighting equations | `notes_shaders.md` §D; the uploaded values in §E.6; `../findings.md` "Point lights", "Colors in the shader" |
| 6.4 Light sources: data → shader | `notes_shaders.md` §E; `../findings.md` "ARE colors and tile lights, stored and read", "Point lights" |
| 6.5 Fog, sky, shadows, post | `notes_shaders.md` §F–§J; `../findings.md` "Fog" |
| 6.6 Recommended Moonglow renderer plan | superseded by `../PLAN.md` §5.5. It recommended loading the stock `vslit`/`fslit` pairs on GL 3.3 with an emulated uniform set; that was not done. Its items to measure in the game (fog start, the color space of uploaded colors, the default sun direction, the main lights' radius) are answered in `../findings.md` and `notes_shaders.md` §E.6, §F, §M |

Moonglow: `crates/mg-render` (`shader.wgsl`, `renderer.rs`, `scene.rs`).

---

## 7. Script compilation

Sources: `~/.local/opt/neverwinter/bin/nwnscriptcomp.h`, nwn.py `nwscript/comp.py`, a local test compile [V], [CL], https://nwn.wiki/spaces/NWN1/pages/109674520/Script+Editor.

### 7.1 libnwnscriptcomp C API
Built as described, with the compiler's source built in rather than loaded as a library: `crates/mg-script/src/compiler.rs` (ABI 1, optimization flags 1).

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
- **Moonglow design** (the launch is built as described, with character selection as an option: `crates/mg-ui/src/test_module.rs`; the server route is `mg serve` and `mg test`, `crates/mg-serve`):
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
- The prototype scripts of this research (a binary MDL reader, SET statistics, a GFF field scan) were not kept.

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
