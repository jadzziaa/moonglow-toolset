---
type: Research Note
title: NWN:EE GFF resources authored by the toolset + EE toolset changelog — research notes for Moonglow Toolset
description: Field reference for the GFF resource types the toolset authors (NWN:EE 89.8193.37), toolset-only data in a module, the EE-added fields, and the EE toolset's changes in order and by topic, with citations and open questions.
tags: [gff, fields, ee, changelog]
generated: { by: claude-code/claude-opus-5-5, at: 2026-09-30T13:38:36Z }
---

# NWN:EE GFF resources authored by the toolset + EE toolset changelog — research notes for Moonglow Toolset

Game: 89.8193.37-17 (bin/win32/build.txt: commit 26c6e57, 2025-10-06). nwtoolset.exe = **PE32 i386** (still 32-bit; Delphi UI + C++ engine code), nwmain = PE32+ x64.
`$G` = `~/.local/share/Steam/steamapps/common/Neverwinter Nights`.

## 0. Method and evidence tags

- **[D]** = confirmed in real data. Scanned every GFF resource in all 28 stock modules (`$G/data/nwm/*.nwm` 20, `$G/data/mod/*.mod` 8) plus all GFF resources in the base-game resman (keys/bifs, no user dir): 5 743 UTI, 5 543 UTC, 4 183 UTP, 2 854 DLG, 1 493 UTS, 1 462 GIT, 993 ARE, 990 GIC, 516 UTE, 400 UTW, 331 ITP, 326 UTT, 323 UTD, 137 UTM, 28 IFO, 28 FAC, 26 JRL, 7 PTM, 4 PTT, 70 BIC (`data/lcv`, `data/dmv`). Tools: nwn.py 3.14 venv + neverwinter.nim 2.3.1 (`nwn_erf`, `nwn_gff`). Scripts/outputs in `scratchpad/gff/` (`scan.py`, `union.json`, `analysis.txt`).
- **Ground truth for "what the current toolset writes"**: *Doom of Icewind Dale* (`DoIWD`, ERF build date 2025-03-16, `Mod_MinGameVer` "1.89") — the only stock module saved by an 89-era toolset. Field **write order** given below is DoIWD's (the toolset writes fields in a fixed order; the game doesn't care about order, but matching it makes diffs clean).
- **Pre-EE baseline [D-old]**: 5 modules with 2002-2004 ERF dates saved by 1.2x-1.62 toolsets (Contest of Champions, Neverwinter Chess, Winds of Eremor = 1.2x; Dark Ranger's Treasure, To Heir is Human = `Mod_MinGameVer` 1.62). Beamdog's 2019-2021 re-saves (campaign .nwm = 1.77/1.78, Kingmaker/ShadowGuard/Witch's Wake = 1.79, DDF/XP1-Ch2 = 1.80, DoD 2021-07, ToM 2021-10) show intermediate EE states.
- **[T]** = label found in `strings nwtoolset.exe` (read-only). The toolset keeps each GFF writer's labels as a contiguous string run in the write order, followed by the reader's run; unknown labels in those runs were checked (e.g. DLG param children `Key`/`Value`, visual transform members `ValueFrom`/`LerpDuration`/`LerpProgress`).
- **[S]** = label in `strings nwserver-linux` (engine reader; includes savegame-only runtime state the toolset never writes).
- **[Doc]** = changelog / nwn.wiki (URLs in §F). **[K]** = from the 1.69-era Bioware GFF specs as I know them — the PDFs are linked from the wiki but **not mirrored locally**, so [K] items are unverified here.
- **"EE"** = added after 1.69. Pre-EE introduction versions come from `$G/lang/en/docs/legacy/NWNv169.txt` (1.18-1.69 patch notes).

Type abbreviations: B=BYTE(0) C=CHAR(1) W=WORD(2) S=SHORT(3) DW=DWORD(4) I=INT(5) DW64(6) I64(7) F=FLOAT(8) D=DOUBLE(9) Str=CExoString(10) RR=CResRef(11) Loc=CExoLocString(12) V=VOID(13) St=Struct(14) L=List(15).

---

## A. GFF container basics

- Header 56 bytes [D, parsed module.ifo]: `FileType` 4 chars (e.g. `"IFO "`), `FileVersion` `"V3.2"`, then 12 DWORDs: StructOffset (=56), StructCount, FieldOffset, FieldCount, LabelOffset, LabelCount, FieldDataOffset, FieldDataCount(bytes), FieldIndicesOffset, FieldIndicesCount(bytes), ListIndicesOffset, ListIndicesCount(bytes). Example DoIWD module.ifo: `IFO V3.2, 56, 74, 944, 128, 2480, 57, 3392, 1870, 5262, 220, 5482, 312`.
- Struct entry 12 bytes: struct ID (DWORD), DataOrDataOffset (field index if 1 field, else byte offset into FieldIndices), FieldCount. Root struct ID = 0xFFFFFFFF [D all files].
- Field entry 12 bytes: Type, LabelIndex, DataOrDataOffset (inline for types 0-5 and 8; offset into FieldData for 6,7,9-13; struct index for 14; offset into ListIndices for 15). Labels: 16-byte, NUL-padded, **max 16 chars** — engine derives labels by prefixing and truncating to 16 (EE `xArmorPart_LBice`, `xArmorPart_Pelvi` [D]).
- Complex data [K]: CExoString = DW length + bytes (no NUL). CResRef = 1 byte length + ≤16 chars. CExoLocString = DW total size, DW StringRef (0xFFFFFFFF = none), DW count, then {DW id, DW len, bytes}; id = language*2 + gender. VOID = DW length + bytes. List = DW count + DW struct indices.
- **Language IDs in real data** [D]: 0-5 (en fr de it es pl) and **256-263 = languages 128-131 (Korean, Chinese trad., Chinese simp., Japanese) × gender**, found in DoD (c1ar0500*.git) and ToM (tm_cr_marel*.utc, area053.git). nwn.py 3.x's reader **crashes** on them ("129 is not a valid Language"); neverwinter.nim keeps them. Moonglow must accept any id.
- Strings are 8-bit, windows-1252 for English [Doc: nwn_gff `--nwn-encoding` default]; EE added `encoding.2da` (87) [Doc].
- FileType strings seen [D]: `IFO ` `ARE ` `GIT ` `GIC ` `UTC ` `UTD ` `UTE ` `UTI ` `UTM ` `UTP ` `UTS ` `UTT ` `UTW ` `DLG ` `JRL ` `FAC ` `ITP ` `BIC ` `PTM ` `PTT ` `GUI `, all `V3.2`. Toolset also knows [T] `UTG ` (item generator, unused), `GFF ` (ExportInfo), and legacy blueprint magics `BTC BTD BTE BTG BTI BTM BTP BTS BTT` (pre-release; read-only compatibility, toolset maps them to UT*). The toolset binary also contains `"V2.0"` checks next to DLG/JRL/FAC/ARE/PTM/PTT readers (old-version compatibility).
- **No EE change to the GFF container itself**: no new field types (0-15 only), still V3.2, 16-char labels [D+Doc: nothing in any changelog]. EE-only GFF *resource types*: `.caf` (ARE+GIT(+GIC) combined, RESTYPE 2082, 85.8193.32) used by TemplateToJson/JsonToObject — not toolset-authored. SQLite databases ride inside GFF as VOID data (NWCompressedBuf magic `SQL3`) in BIC/savegame IFO [Doc SQLite, nwn_gff `--out-sqlite`; label in engine: `SQLite` [S]].
- Tolerance notes from real data: ToM ERF contains an entry with empty resref (`'.res'`, nwn_erf renames it `invalid_4042.res`); `Repute.fac` is stored with capital R by the toolset (every stock module except DDF/KM/SG/WW/WCoC), `Module.ifo` capitalised in Neverwinter Chess — resref lookups must be case-insensitive; files are written lowercase except these legacy names.

---

## B. Per-resource-type field reference

Legend for columns: **Written** = present in DoIWD (89 toolset) output [D]. **EE** marks EE additions (version). "cond." = written only when non-empty/non-default. Meaning column is [K] unless marked.

### B1. IFO — `module.ifo` (module properties). FileType `IFO `.
Write order (DoIWD) and meaning:

| # | Label | Type | Meaning / notes |
|---|---|---|---|
| 1 | Mod_ID | V (16 bytes) | module identifier blob (random/hash); some Beamdog nwm have all zeros [D] |
| 2 | Mod_MinGameVer | Str | "1.89" in DoIWD. Toolset stamps its compat version; old toolsets refuse newer modules [Doc wiki "How to Change the Module Version Id"]. Pre-EE field (1.62 modules have it) [D-old]. Toolset binary holds constants "1.89" and "1.22" next to it [T] |
| 3 | Mod_Creator_ID | I | always 2 [D] |
| 4 | Mod_Version | DW | always 3 [D] |
| 5 | Expansion_Pack | W | required expansions bitmask (1 SoU, 2 HotU [K]); 3 in all EE saves, 0 in 1.2x [D] |
| 6 | Mod_Name | Loc | |
| 7 | Mod_Tag | Str | |
| 8 | Mod_Description | Loc | also copied into the ERF header localized-string table [D DoIWD ERF header; Doc 1.61] |
| 9 | Mod_IsSaveGame | B | 0 in modules |
| 10 | Mod_CustomTlk | Str | custom TLK name without extension ("ossian", "tyrants", "dla") [D] |
| 11-16 | Mod_Entry_Area RR, Mod_Entry_X/Y/Z F, Mod_Entry_Dir_X/Y F | | start location (area resref + position + facing vector) |
| 17 | Mod_Expan_List | L | children [T]: `Expansion_Name`, `Expansion_ID` (always empty in data) |
| 18-24 | Mod_DawnHour B, Mod_DuskHour B, Mod_MinPerHour B, Mod_StartMonth B, Mod_StartDay B, Mod_StartHour B, Mod_StartYear DW | | calendar |
| 25 | Mod_XPScale | B | 0-200 % |
| 26-43 | Mod_OnHeartbeat, Mod_OnModLoad, Mod_OnModStart, Mod_OnClientEntr, Mod_OnClientLeav, Mod_OnActvtItem, Mod_OnAcquirItem, Mod_OnUsrDefined, Mod_OnUnAqreItem, Mod_OnPlrDeath, Mod_OnPlrDying, Mod_OnPlrEqItm, Mod_OnPlrLvlUp, Mod_OnSpawnBtnDn (OnPlayerRespawn), Mod_OnPlrRest, Mod_OnPlrUnEqItm, Mod_OnCutsnAbort, **Mod_OnPlrChat** | RR | event scripts. OnPlayerChat = **1.69** (not EE) [Doc NWNv169] |
| 44 | **Mod_OnPlrTarget** | RR | **EE** OnPlayerTarget — event 1.80.8193.14 (script-only), toolset UI 87.8193.35 [Doc] (controls `bBrowse/bEditOnPlayerTarget` etc. [T]) |
| 45 | **Mod_OnPlrGuiEvt** | RR | **EE** OnPlayerGuiEvent — event 1.85.8193.30, toolset UI 87 [Doc] |
| 46 | **Mod_OnPlrTileAct** | RR | **EE** OnPlayerTileAction — event 1.85.8193.30, toolset UI 87 [Doc] |
| 47 | **Mod_OnNuiEvent** | RR | **EE** OnNuiEvent — NUI 85.8193.31/32, toolset UI 87 [Doc] |
| 48 | Mod_StartMovie | RR | |
| 49 | **Mod_DefaultBic** | RR | **EE 87.8193.35**: default character .bic shipped inside the .mod; game launches straight into the module with it; changelog says "no GUI option yet" [Doc]; the 89 toolset reads+writes it (empty) [D,T] but has no UI control for it [T]. Read by nwmain only (not nwserver) [S] |
| 50 | **Mod_UUID** | Str | **EE**. Written empty by the toolset [D DoIWD]. 88.8193.36 fixed "toolset clobbering module UUID when the author manually embedded one with a GFF editor" → toolset must **preserve** an existing value [Doc]. Not in any ≤2021 module [D]; engine reads it [S] |
| 51 | **Mod_PartyControl** | I | **EE 87.8193.35**: 0 = server default, 1 = enabled, 2 = disabled; overrides server setting; "no GUI option yet" [Doc]; read+written (0) by the 89 toolset, no UI control found [D,T] |
| 52 | Mod_CutSceneList | L | children [T] `CutScene_Name`, `CutScene_ID` (empty in data) |
| 53 | Mod_GVar_List | L | global vars; children [T] `GVar_Name`, `GVar_Data` (typed variants) (empty in data) |
| 54 | Mod_Area_list | L (struct id 6) | `Area_Name` RR — every area in the module (DoIWD lists a `testarea` first) |
| 55 | Mod_HakList | L (struct id 8) | `Mod_Hak` Str — hak names without extension, **priority order (first = highest)**. Multiple haks = 1.28; list form present in 1.62 files [D-old] |
| — | Mod_Hak | Str | legacy single hak (1.2x files) — read, converted to Mod_HakList [T writer code has `Mod_Hak` in both] |
| — | Mod_CacheNSSList | L (struct id 9) | `ResRef` RR — legacy "Cached Scripts" (1.30). **Dropped**: toolset UI removed 1.80.8193.14 [Doc]; not written by DoIWD, not in toolset strings [D,T]; present (4-159 entries) in 1.77-1.80 files. Preserve if present; don't author |
| — | VarTable | L | module local variables (Name/Type/Value) — cond. (only when variables set; DoD, PotSC, WCoC) [D] |

Default scripts the toolset fills for a new module [T]: x2_mod_def_load, x3_mod_def_enter, x2_mod_def_act, x2_mod_def_aqu, x2_mod_def_unaqu, nw_o0_death, nw_o0_dying, x2_mod_def_equ, nw_o0_respawn, x2_mod_def_rest, x2_mod_def_unequ.
Engine-only (savegame) IFO labels [S] — never toolset-authored, preserve if found: Mod_IsNWMFile, Mod_NWMResName, Mod_Effect_NxtId, Mod_NextCharId0/1, Mod_NextObjId0/1, Mod_Transition, Mod_StartMinute/Second/MiliSec, Mod_MaxHenchmen, Mod_Tokens(+Number/Value), Mod_TlkOverrides(+Number/Value) (EE SetTlkOverride), Mod_TURDList (TURD_* fields), Mod_PlayerList, Mod_CommntyName, Mod_FirstName, Mod_LastName, Mod_IsPrimaryPlr, Mod_MapAreasData/MapAutoExplores/MapData/MapDataList/MapNumAreas, NWSync/NWSyncAdvertLUT, SQLite blob.
No NWSync hash / hak-hash field exists in module.ifo [D DoIWD, S]. NWSync metadata lives in server-side repositories, not the module.
Not real (prompt guesses checked and absent everywhere [D,T,S]): `Mod_Cache`, `Mod_HakList` as a single string, `UseTweakedStuff`, `Door Tlk`, creature `DisplayName`.

### B2. ARE — area static data (`<area>.are`). FileType `ARE `.

| Label | Type | Meaning |
|---|---|---|
| ID, Creator_ID | I | -1 in toolset output [D] |
| Version | DW | save counter, increments each save (8-32 seen) [D] |
| Tag Str, Name Loc, ResRef RR | | ResRef = own resref |
| Comments | Str | area comment (Area Properties > Comments) |
| Expansion_List | L | children [T] write `Expansion_Name`/`Expansion_ID`, read `Name`/`ID`; empty in data |
| Flags | DW | bit0 interior, bit1 underground, bit2 natural [K] (values 0,1,3,4,5,7 seen [D]) |
| ModSpotCheck, ModListenCheck | I | |
| MoonAmbientColor, MoonDiffuseColor, MoonFogColor, SunAmbientColor, SunDiffuseColor, SunFogColor | DW | 0x00BBGGRR |
| MoonFogAmount, SunFogAmount | B | 0-15 |
| MoonShadows, SunShadows | B | bool |
| IsNight | B | for DayNightCycle=0 |
| LightingScheme | B | environment.2da row |
| ShadowOpacity | B | 0-100 |
| FogClipDist | F | EE: now also stored in savegames (85.8193.32 fix) [Doc] |
| SkyBox | B | skyboxes.2da row (EE: list sorted alphabetically, row 0 "None", 89) |
| DayNightCycle | B | |
| ChanceRain, ChanceSnow, ChanceLightning | I | 0-100 |
| WindPower | I | 0 none, 1 weak, 2 strong (toolset wind presets documented on wiki 1.80.8193.14: NONE mag 0; LIGHT mag 1.0 yaw 100 pitch 3; HEAVY mag 2.0 yaw 150 pitch 5) |
| LoadScreenID | W | loadscreens.2da |
| PlayerVsPlayer | B | pvpsettings |
| NoRest | B | |
| Width, Height | I | tiles |
| OnEnter, OnExit, OnHeartbeat, OnUserDefined | RR | the only area events (no EE area event slots) |
| **TileBrdrDisabled** | B | **EE** — disables the tile border (area edge) ring; script API SetAreaTileBorderDisabled 88.8193.36 [Doc]; toolset reads+writes it (0) [D DoIWD 70/72 areas; T reader+writer runs]; no Area Properties control found in the toolset's form strings [T] → preserve-only in the stock toolset |
| Tileset | RR | .set resref |
| Tile_List | L (struct id 1) | width×height entries, row-major from bottom-left [K] |

Tile_List struct: `Tile_ID` I (index into .set TILES), `Tile_Orientation` I (0-3 ×90°), `Tile_Height` I, `Tile_MainLight1/2` B (lightcolor.2da; 0 = none), `Tile_SrcLight1/2` B (0 = off, else lightcolor), `Tile_AnimLoop1/2/3` B.
ARE write order (DoIWD): ID, Creator_ID, Version, Tag, Name, ResRef, Comments, Expansion_List, Flags, ModSpotCheck, ModListenCheck, MoonAmbientColor, MoonDiffuseColor, MoonFogAmount, MoonFogColor, MoonShadows, SunAmbientColor, SunDiffuseColor, SunFogAmount, SunFogColor, SunShadows, IsNight, LightingScheme, ShadowOpacity, FogClipDist, SkyBox, DayNightCycle, ChanceRain, ChanceSnow, ChanceLightning, WindPower, LoadScreenID, PlayerVsPlayer, NoRest, Width, Height, OnEnter, OnExit, OnHeartbeat, OnUserDefined, TileBrdrDisabled, Tileset, Tile_List.
Engine/savegame-only ARE labels [S] (EE runtime state, preserve): `Tile_ReplaceTex`, `ChanceFog`, `AreaEffectList`, `GrassDefDisabled`, `GrassOverrides`/`GrassOvrList` {GrassMaterialId, GrassTexture, GrassDensity, GrassHeight, GrassAmbientX/Y/Z, GrassDiffuseX/Y/Z} (EE SetAreaGrassOverride), `MoonDirectionX/Y/Z`, `SunDirectionX/Y/Z` (EE area light direction), `WindDirectionX/Y/Z`, `WindMagnitude`, `WindYaw`, `WindPitch` (EE SetAreaWind), `CurrentWeather`, `WeatherStarted`.
Area *local variables* are not in the ARE: they go in the **GIT root `VarTable`** [D + T Area Properties has a Variables button].
Area defaults (lighting schemes, ambient/music presets) come from `areag.ini`/environment.2da [T strings LIGHT_AMB_RED…, DAYNIGHT, SKYBOX; wiki areag.ini]; DFT files unused.

### B3. GIT — area instances (`<area>.git`). FileType `GIT `.
Root [D]: `AreaProperties` St, `Creature List` L, `Door List` L, `Encounter List` L, `List` L (items on the ground), `Placeable List` L, `SoundList` L, `StoreList` L, `TriggerList` L, `WaypointList` L, `VarTable` L (area variables, cond.). Engine savegames add `AreaEffectList` [S] (toolset never writes it).
List struct IDs [D]: Creature 4, Door 8, Encounter 7, List (item) 0, Placeable 9, SoundList 6, StoreList 11, TriggerList 1, WaypointList 5; AreaProperties struct id 100 (14 in some older files).

`AreaProperties` (write order): AmbientSndDay I, AmbientSndNight I, AmbientSndDayVol I, AmbientSndNitVol I, EnvAudio I, MusicBattle I, MusicDay I, MusicNight I, MusicDelay I (ms). (`Comment` Str appears in 3 modules [D]; not written by toolset [T].)

Instances = the full blueprint field set **minus** `PaletteID` and (except creatures) `Comment`, **plus** position/orientation (+ EE visual transforms). Instance-only fields [D DoIWD, blueprint vs GIT diff]:

| List | Instance-only fields (types) | Blueprint-only |
|---|---|---|
| Creature List | XPosition, YPosition, ZPosition, XOrientation, YOrientation (F; orientation = facing vector) written **first**; `VisTransformList` (EE) | PaletteID (Comment is kept in GIT for creatures [D]) |
| Door List | X, Y, Z, Bearing (F, radians) written **last**; `VisTransformList` (EE); `VarTable` | Comment, PaletteID |
| Placeable List | X, Y, Z, Bearing; `VisTransformList` (EE); `ItemList` (inventory, cond.) | Comment, PaletteID |
| Encounter List | XPosition, YPosition, ZPosition; `Geometry` L {X,Y,Z F; struct id 1} (polygon, relative); `SpawnPointList` L {X,Y,Z,Orientation F; struct id 2}; Comment is here (GIC encounter structs are empty) | PaletteID |
| List (items) | XPosition, YPosition, ZPosition, XOrientation, YOrientation first; model/colour fields only for the base item's model type (e.g. a simple item writes only ModelPart1/xModelPart1) | Comment, PaletteID |
| SoundList | GeneratedType DW, XPosition, YPosition, ZPosition | Comment, PaletteID (+ `PlayInToolset` in GIC) |
| StoreList | XPosition, YPosition, ZPosition, XOrientation, YOrientation first | Comment, `ID` (store palette id) |
| TriggerList | XPosition, YPosition, ZPosition, XOrientation, YOrientation, **ZOrientation**, `Geometry` L {PointX, PointY, PointZ F; struct id 3} | Comment, PaletteID |
| WaypointList | XPosition, YPosition, ZPosition, XOrientation, YOrientation | Comment, PaletteID |

All instances keep `TemplateResRef` (source blueprint; door TemplateResRef save/load fixed 1.80.8193.14 [Doc]). `VarTable` per instance cond.
Nested instance inventories (creature `Equip_ItemList`/`ItemList`, placeable `ItemList`, store `StoreList[].ItemList`) contain **full item structs** in the GIT (not just resrefs as in blueprints): item fields + XPosition…YOrientation + Repos_PosX/Repos_Posy (inventory grid) + Dropable/Pickpocketable (creature) / Infinite (store) [D].

**EE visual transforms** (toolset "Adjust Location" dialog + Ctrl+wheel scaling, 1.79; creatures, items, doors, non-static placeables; reset when Static is set [Doc v79]):
- 1.79-1.8x format [D WCoC door, ToM placeable]: `VisualTransform` St (struct id 6) {ScaleX, ScaleY, ScaleZ F (…other members presumably only when non-default)}. Current toolset still **reads** this label [T].
- Current format [D DoIWD, T, S]: `VisTransformList` L (struct id 6), one element per scope: `Scope` I (OBJECT_VISUAL_TRANSFORM_DATA_SCOPE_*: 0 base; creature 254 head, 253 tail, 252 wings, 243 cloak; item parts 255-251 [nwscript.nss]), then structs (id 0) `AnimationSpeed`, `ScaleX`, `ScaleY`, `ScaleZ`, `RotateX`, `RotateY`, `RotateZ`, `TranslateX`, `TranslateY`, `TranslateZ`, each {`TimerType` I, `ValueTo` F, `LerpType` I (OBJECT_VISUAL_TRANSFORM_LERP_* 0-7)} plus, per [T] writer, `ValueFrom` F, `LerpDuration`, `LerpProgress` when a lerp is in progress (not seen in data). Lerps added 1.83.8193.21 [Doc]. The version that switched VisualTransform → VisTransformList (scopes) is not stated in the changelogs (between 2021-10 and 2025).

Engine-only instance labels [S] (runtime, savegames/ObjectToJson; preserve): Material, ShaderParams {Float1-4}, TextureReplace {OldTexture, NewTexture}, AnimationReplace {OldAnimation, NewAnimation}, MiscVisuals, MouseCursor, HiliteColorR/G/B, VisibleDistance, UiDiscoverMask, TextBubbleType/Text, SecretDoorDC, OnDialog, TrapRecoverable/Active/Faction/Creator, DieWhenEmpty, GroundPile, LightState, IsBodyBag, ExpressionList…, UsesPerDay.

### B4. GIC — area instance comments (`<area>.gic`). FileType `GIC `. Toolset-only.
Same 9 lists as GIT, same struct IDs, one struct per GIT instance **in the same order**: `Comment` Str. SoundList structs also carry `PlayInToolset` B (preview sound in the area editor) — write order PlayInToolset, Comment. Encounter List structs are **empty** (encounter comments live in the GIT) [D DoIWD]. `List[]` (ground items) Comment appears only in newer files [D].

### B5. UTC — creature blueprint. FileType `UTC `. Palette field `PaletteID` B.
Write order (DoIWD most common, 75/275):
TemplateResRef RR, Race B, FirstName Loc, LastName Loc, Appearance_Type W, Gender B, Phenotype I, PortraitId W, Description Loc, Tag Str, Conversation RR, IsPC B, FactionID W, Disarmable B, Subrace Str, Deity Str, **Wings_New DW, Tail_New DW** (1.69: wing/tail/genericdoor 2da rows > 255 [Doc NWNv169]), SoundSetFile W, Plot B, IsImmortal B, Interruptable B, Lootable B (1.61), NoPermDeath B, BodyBag B, StartingPackage B, DecayTime DW (1.61, ms), then body parts, each BYTE immediately followed by its **EE WORD twin**: ArmorPart_RFoot B / **xArmorPart_RFoot W**, BodyPart_LFoot/xBodyPart_LFoot, RShin, LShin, LThigh, RThigh, Pelvis, Torso, Belt, Neck, RFArm, LFArm, RBicep, LBicep, RShoul, LShoul, RHand, LHand, Appearance_Head B / **xAppearance_Head W**; Color_Skin, Color_Hair, Color_Tattoo1, Color_Tattoo2 B; Str, Dex, Con, Int, Wis, Cha B; WalkRate I (creaturespeed.2da); NaturalAC B; HitPoints S, CurrentHitPoints S, MaxHitPoints S; refbonus, willbonus, fortbonus S; GoodEvil B, LawfulChaotic B (0-100); ChallengeRating F, CRAdjust I; PerceptionRange B (ranges.2da; EE 89 fix: custom ranges saved/loaded); ScriptHeartbeat, ScriptOnNotice, ScriptSpellAt, ScriptAttacked, ScriptDamaged, ScriptDisturbed, ScriptEndRound, ScriptDialogue, ScriptSpawn, ScriptRested, ScriptDeath, ScriptUserDefine, ScriptOnBlocked RR; SkillList L, FeatList L, TemplateList L, SpecAbilityList L, ClassList L, [VarTable L cond.], [ItemList L cond.], Equip_ItemList L, PaletteID B, Comment Str.
- General placement of the conditional `VarTable` [D DoIWD+ToM]: UTC after ClassList; UTP after OnClick; UTI after PropertiesList; UTT/UTW right before PaletteID. UTP `ItemList` goes after OnUsed.
- Note the Bioware quirk: right foot is `ArmorPart_RFoot` (not BodyPart_RFoot) [D].
- **EE x-parts** (87.8193.35 raised body-part/armor-part ids 255 → 999; 88.8193.36 fixed parts > 255 in the toolset [Doc]). The changelog never names the fields; data shows WORD `x`+label twins written right after each BYTE [D]. In all 25 891 DoIWD pairs the two values are equal (no id > 255 in stock data), so what the BYTE holds for ids > 255 is unobserved (changelog: older clients "default to part 0 if given a part greater than 255"). The labels are built at runtime as "x" + label, not stored as strings in any binary [T,S]. Only DoIWD has them among stock modules.
- Nested: `ClassList` (struct 2; up to **8 entries in EE**, ruleset.2da MULTICLASS_LIMIT, toolset wizards 89) {Class I, ClassLevel S, KnownList0-9 L, MemorizedList0-9 L (struct 3) {Spell W, SpellMetaMagic B, SpellFlags B}}; `SkillList` (struct 0) {Rank B} — one per skills.2da row, index = skill; `FeatList` (struct 1) {Feat W}; `SpecAbilityList` (struct 4) {Spell W, SpellFlags B, SpellCasterLevel B} (EE v74: cast at the builder-set caster level); `TemplateList` (struct 5) {TemplateID W} (unused); `Equip_ItemList` (struct id = equipment slot bit 1…131072) {EquippedRes RR, [Dropable B, Pickpocketable B]}; `ItemList` (struct id = index) {InventoryRes RR, Repos_PosX W, Repos_Posy W, Dropable B, [Pickpocketable B]}; `VarTable`.
- Legacy read-only [T/D]: `Wings` B, `Tail` B (pre-1.69), `Portrait` RR (BASE blueprints).
- BIC-only fields (read-only for the toolset) listed in B18.

### B6. UTD — door blueprint. FileType `UTD `. `PaletteID` B.
Write order: Tag Str, LocName Loc, Description Loc, TemplateResRef RR, AutoRemoveKey B, CloseLockDC B, Conversation RR, Interruptable B, Faction DW, Plot B, KeyRequired B, Lockable B, Locked B, OpenLockDC B, PortraitId W, TrapDetectable B, TrapDetectDC B, TrapDisarmable B, DisarmDC B, TrapFlag B, TrapOneShot B, TrapType B (traps.2da), KeyName Str, AnimationState B, Appearance DW (doortypes.2da; 0 = use generic), HP S, CurrentHP S, Hardness B, Fort B, Ref B, Will B, OnClosed, OnDamaged, OnDeath, OnDisarm, OnHeartbeat, OnLock, OnMeleeAttacked, OnOpen, OnSpellCastAt, OnTrapTriggered, OnUnlock, OnUserDefined, OnClick RR (area-transition click), LinkedTo Str (destination tag), LinkedToFlags B (0 none, 1 door, 2 waypoint [K]; EE 87: transitions may target any tag), LoadScreenID W, GenericType_New DW (genericdoors.2da, 1.69), OnFailToOpen RR, PaletteID B, Comment Str, [VarTable cond.].
Legacy read-only: `GenericType` B, `Portrait` RR [D BASE, T]. The toolset reader also knows `Invulnerable` [T] (legacy name; not in data).

### B7. UTP — placeable blueprint. FileType `UTP `. `PaletteID` B.
Write order: Tag, LocName, Description, TemplateResRef, AutoRemoveKey, CloseLockDC, Conversation, Interruptable, Faction DW, Plot, KeyRequired, Lockable, Locked, OpenLockDC, PortraitId W, TrapDetectable, TrapDetectDC, TrapDisarmable, DisarmDC, TrapFlag, TrapOneShot, TrapType, KeyName, AnimationState B, Appearance DW (placeables.2da), HP S, CurrentHP S, Hardness, Fort, Ref, Will, OnClosed, OnDamaged, OnDeath, OnDisarm, OnHeartbeat, OnLock, OnMeleeAttacked, OnOpen, OnSpellCastAt, OnTrapTriggered, OnUnlock, OnUserDefined, **OnClick** (1.67), [VarTable cond.], HasInventory B, BodyBag B, Static B, Type B, Useable B, OnInvDisturbed RR, OnUsed RR, [ItemList L {InventoryRes, Repos_PosX, Repos_Posy} cond.], PaletteID, Comment.

### B8. UTE — encounter blueprint. FileType `UTE `. `PaletteID` B.
Tag Str, LocalizedName Loc, TemplateResRef RR, Active B, Difficulty I, DifficultyIndex I (encdifficulty.2da), Faction DW, MaxCreatures I, PlayerOnly B, RecCreatures I, Reset B, ResetTime I, Respawns I (-1 infinite [K]), SpawnOption I (0 continuous, 1 single-shot [K]), OnEntered, OnExit, OnExhausted, OnHeartbeat, OnUserDefined RR, CreatureList L (struct 0) {Appearance I, CR F, ResRef RR, SingleSpawn B}, PaletteID, Comment, [VarTable cond.]. Geometry/SpawnPointList only in GIT.

### B9. UTI — item blueprint. FileType `UTI `. `PaletteID` B.
Write order: TemplateResRef RR, BaseItem I (baseitems.2da), LocalizedName Loc, Description Loc, DescIdentified Loc, Tag Str, Charges B (EE v74: max 50 → 250), Cost DW, Stolen B, StackSize W, Plot B, AddCost DW, Identified B, Cursed B (1.61 undroppable), then model fields by baseitems ModelType: simple = ModelPart1 B + **xModelPart1 W**; layered = + Leather/Cloth/Metal1/2Color B; composite = ModelPart1-3 (+x twins); armor = ArmorPart_{RFoot,LFoot,RShin,LShin,LThigh,RThigh,Pelvis,Torso,Belt,Neck,RFArm,LFArm,RBicep,LBicep,RShoul,LShoul,RHand,LHand,Robe} B (Robe 1.61) + **x twins W** (`xArmorPart_LBice`, `_LShou`, `_LThig`, `_Pelvi`, `_RBice`, `_RShou`, `_RThig` — truncated to 16 chars) + 6 colours; PropertiesList L, PaletteID B, Comment Str, [VarTable cond.].
PropertiesList struct (id 0): PropertyName W (itempropdef.2da), Subtype W, CostTable B, CostValue W, Param1 B, Param1Value B, ChanceAppear B (legacy, 100). Runtime adds UsesPerDay/Useable [S].

### B10. UTM — store blueprint. FileType `UTM `. **Palette field is `ID` B (not PaletteID)** [D,T].
ResRef RR (= TemplateResRef equivalent), LocName Loc, Tag Str, MarkUp I, MarkDown I, BlackMarket B, BM_MarkDown I, IdentifyPrice I, MaxBuyPrice I, StoreGold I (-1 unlimited [K]), OnOpenStore RR, OnStoreClosed RR (these 5 are 1.61), WillNotBuy L, WillOnlyBuy L (struct id 0x17E4D = 97869) {BaseItem I}, StoreList L (struct id = page 0 armor, 1 misc, 2 potions, 3 rings, 4 weapons [K]) {ItemList L {InventoryRes RR, Repos_PosX W, Repos_Posy W, [Infinite B]}}, ID B, Comment Str, [VarTable cond.].

### B11. UTS — sound blueprint. FileType `UTS `. `PaletteID` B.
Tag, LocName, TemplateResRef, Active B, Continuous B, Looping B, Positional B, RandomPosition B, Random B, Elevation F, MaxDistance F, MinDistance F, RandomRangeX F, RandomRangeY F, Interval DW (ms), IntervalVrtn DW, PitchVariation F, Priority B, Hours DW (24-bit mask), Times B (0 specific hours, 1 day, 2 night, 3 always [K]), Volume B, VolumeVrtn B, Sounds L (struct 0) {Sound RR}, PaletteID, Comment. Instance adds GeneratedType DW (GIT) and PlayInToolset B (GIC). EE 89: sound objects gained ObjectToJson/TemplateToJson support (runtime).

### B12. UTT — trigger blueprint. FileType `UTT `. `PaletteID` B.
Tag, TemplateResRef, LocalizedName, AutoRemoveKey B, Faction DW, Cursor B, HighlightHeight F, KeyName Str, LinkedTo Str, LinkedToFlags B, LoadScreenID W, PortraitId W, Type I (0 generic, 1 area transition, 2 trap [K]), TrapDetectable, TrapDetectDC, TrapDisarmable, DisarmDC, TrapFlag, TrapOneShot, TrapType B, OnDisarm, OnTrapTriggered, OnClick, ScriptHeartbeat, ScriptOnEnter, ScriptOnExit, ScriptUserDefine RR, [VarTable], PaletteID, Comment. (Order from ToM .utt files [D]; DoIWD has no UTT.) Legacy in BASE only: `PartyRequired` B, `Portrait` RR (not read by the 89 toolset [T]).

### B13. UTW — waypoint blueprint. FileType `UTW `. `PaletteID` B.
Appearance B (waypoint appearance), LinkedTo Str, TemplateResRef, Tag, LocalizedName Loc, Description Loc, HasMapNote B, MapNote Loc, MapNoteEnabled B, [VarTable], PaletteID, Comment (order from ToM .utw [D]).

### B14. DLG — conversation. FileType `DLG `.
Root write order: DelayEntry DW, DelayReply DW, NumWords DW (word count, recomputed on save), EndConversation RR, EndConverAbort RR, PreventZoomIn B (**1.27**, not EE), EntryList L, ReplyList L, StartingList L.
- EntryList[] (NPC lines; struct id = index): Speaker Str (tag; "" = owner), Animation DW, AnimLoop B (legacy), Text Loc, Script RR (action), **ActionParams L (EE)**, Delay DW, Comment Str, Sound RR, Quest Str (journal tag), [QuestEntry DW — only when Quest set], RepliesList L.
- ReplyList[] (PC lines): same minus Speaker; children list `EntriesList`.
- Link structs (RepliesList[] / EntriesList[]): Index DW (into the other list), Active RR (condition script), **ConditionParams L (EE)**, IsChild B (1 = link, "grey" node), [LinkComment Str — only when set].
- StartingList[]: Index DW, Active RR, **ConditionParams L (EE)**.
- **EE script parameters** (1.80.8193.14 dev / 1.81.8193.15 stable; read with GetScriptParam) [Doc]: `ActionParams`/`ConditionParams` are written on every node/link (empty list when unused) [D DoD, ToM, DoIWD]. Element struct = {`Key` Str, `Value` Str}: labels `Key` and `Value` sit directly after the DLG writer/reader string runs in nwtoolset.exe [T]; no non-empty example in stock data — **verify by saving a DLG with a parameter**.
- `DisplayInactive` is read by the engine's dialog loader [S] but never written by the toolset [T] and not documented; treat as preserve-only.
- Struct IDs of Entry/Reply/Starting/link structs = list index in toolset output (0 mismatches in 9 288 DoIWD structs), but arbitrary values occur in old files [D] — write indices, don't rely on them when reading.

### B15. JRL — `module.jrl` (journal). FileType `JRL `.
Categories L (struct id = index) {Name Loc, XP DW, Priority DW (0 highest … 4 lowest [K]), Picture W (unused, 65535 [K]), Comment Str, Tag Str, EntryList L {ID DW, End W (1 = quest finished), Text Loc}}. (EE 1.80.8193.14 fixed an off-by-one when adding journal entries [Doc].) Absent in modules with no journal (Contest of Champions, Chess).

### B16. FAC — `Repute.fac` (factions). FileType `FAC `.
FactionList L (struct id = faction index) {FactionParentID DW (0xFFFFFFFF = none), FactionName Str, FactionGlobal W}; RepList L {FactionID1 DW, FactionID2 DW, FactionRep DW (0-100: how ID2 feels about ID1 [K])}. First 5 factions are the standard PC, Hostile, Commoner, Merchant, Defender. EE 1.79 fixed reputation editing in the Faction Editor [Doc].

### B17. ITP — palettes. FileType `ITP `. Three kinds [D + wiki]:
1. **Skeleton** `<type>pal.itp` (base game; defines the category tree + IDs): root `MAIN` L, `NEXT_USEABLE_ID` B, `RESTYPE` W (2027 UTC, 2044 UTP, 2013 SET for tilesets …), tileset skeletons add `TILESETRESREF` RR. Nodes (struct id 1): `STRREF` DW, `DELETE_ME` Str (folder name; historically the display fallback), `TYPE` B (0/1/2 seen — display flags, e.g. "ASSIGN TO NEW CATEGORY" = TYPE 0, "Familiars" TYPE 1, "Special" TYPE 2 with Custom 1-5), `ID` B (category id = the blueprint's PaletteID; leaf categories), `LIST` L (sub-nodes).
2. **Standard** `<type>palstd.itp` (base game blueprints; tileset `tXXnnpalstd.itp` for tiles/groups/terrain): MAIN tree (struct id 0) with `STRREF`/`ID`/`LIST`; leaves {`RESREF` RR, `NAME` Str or `STRREF` DW}; creature leaves add `CR` F and `FACTION` Str.
3. **Custom** `<type>palcus.itp` (itempalcus, creaturepalcus, encounterpalcus, doorpalcus, soundpalcus, triggerpalcus, waypointpalcus, placeablepalcus, storepalcus) — written by the toolset into the module (9 in DoIWD) from the module's own blueprints sorted by PaletteID; used by the DM client Creator, not by the toolset itself [wiki Toolset Palette ITP]. PaletteID 255 hides a blueprint from the custom palette [wiki].
- **EE 1.79** [Doc]: category nodes may carry a plain `NAME` CExoString instead of STRREF (renamed from `DELETE_ME`); 1.80.8193.6 extends this to custom tileset palettes.
- Tileset palette branches: MAIN ID 0 Features (STRREF 63261), 1 Groups (63262), 2 Terrain paint (8282); sub-folder ID 2 = paintable terrain (solver), ID 3 = specific tile placement [wiki ITP Palette Format].
- Legacy `Cat`/`CatName`/`Node`/`LName`/`LRes` format exists in 7 unused base files (standard.itp, stdcre.itp…) [D]; not read by the toolset [T].

### B18. BIC — player character (read-only for the toolset). FileType `BIC `.
UTC fields plus runtime/character state [D 70 pregens]: Age, Experience, Gold, Portrait RR, SkillPoints, LvlStatList {LvlStatClass, LvlStatHitDie, LvlStatAbility, EpicLevel, FeatList, SkillList, KnownList0-9, KnownRemoveList*}, QBList (quickbar, 36 × QB* fields), ClassList adds Domain1/2, School, SpellsPerDayList; FeatList {Feat, Uses}; CombatInfo, CombatRoundData, PerceptionList, positions/orientation, MasterID, IsDM, FamiliarType/Name, CompanionType/Name, ArmorClass, BaseAttackBonus, saves (C), MovementRate, CreatureSize, FootstepType … Equip items have `ObjectId` and odd `UArmorPart_*` twins. EE adds `DataMigration` (1.79) and the player SQLite DB (1.80.8193.14). The toolset only needs BIC for `Mod_DefaultBic` (a .bic stored in the module, EE 87).

### B19. PTM / PTT — plot wizard (toolset-only, legacy). FileType `PTM `/`PTT ` (reader also accepts V2.0).
Root: ID I, Name Loc, JournalName Loc, JournalTag Str, WizCreated B, ResList L {ResRef, ResType}, Cast_List L (struct 4) {Type I, ResType I, IsToBeKilled B, CastID I, GreetPlotless Loc, GreetUndefined Loc, Creature/Placeable/Item St (PTT)}, Prop_List L {Item St}, Node_List L {Name Loc, Comments Str, CastID I, Killable B, Enemy B, PreReqID I, PreReqIsConv B, ItemGiveID, ItemGivePickable, ItemTakeID, ItemLootID, ItemLootPickable, GoldGive I, GoldTake I, JournalEntry Loc, JournalEnd B, XP I, HasConv B, ConvGreet/ConvAccept/ConvAction/ConvReject Loc, ConvIsSingle B, ConvIsFreeForm B} [D 7 PTM (one module), 4 PTT (base)]. Toolset writer also has `UnlockerID`, `UnlockerConv`, `ScriptCondGreet/Accept`, `ScriptActAction/Reject`, `ScriptCHGreet/Accept`, `ScriptAHAction/Reject` [T]. Low priority (Plot Wizard is rarely used).

### B20. Common struct: VarTable (all blueprints, instances, GIT root, IFO)
VarTable L (struct 0) {Name Str, Type DW (1 int, 2 float, 3 string, 4 object, 5 location [K]), Value (I / F / Str by Type)}. The toolset variable editor supports int, float, string only [T: writer has 3 Value variants]. Variables on items/placeables/etc. date from 1.3x-1.61 (pre-EE). `Comment` inside VarTable entries seen in WCoC [D] — not toolset-written. v74: toolset keeps trigger/encounter variables when their polygon is repainted [Doc].

### B21. Non-GFF companions
- **NSS**: script source (text, cp1252). Stored in the .mod next to the NCS by the toolset (DoIWD: 1 106 nss + 1 106 ncs).
- **NCS**: compiled bytecode; header `NCS V1.0` + `B` + DW size [D compiled test]. Compiler = the game's (open-sourced in 88: github.com/niv/neverwinter.nim `nwn_script_comp`).
- **NDB**: debug info, text, first line `NDB V1.0` then counts, `N00 name`, struct `s`, fields `sf`, functions `f`, vars `v`, line map `l` [D test compile with `-g`]. Written only if "Generate Debug Information When Compiling Scripts" is on (1.30 option). Used by the NWScript Debugger (revived 1.83.8193.23).
- **ERF (.mod/.hak/.erf)**: `MOD V1.0` header 160 bytes: LanguageCount, LocalizedStringSize, EntryCount, OffsetToLocalizedString (160), OffsetToKeyList, OffsetToResourceList, BuildYear (since 1900), BuildDay, DescriptionStrRef (0xFFFFFFFF), 116 reserved bytes. Key 24 bytes = 16-char resref + DW ResID + W ResType + 2 unused; resource list {DW offset, DW size}. Toolset-written keys are sorted by **upper-cased** resref (so `_` sorts after letters: `con_tm_henint9` < `con_tm_hen_cnt_1`), ResID = index, resref case preserved (`DisorganizedScri`, `Repute`) [D DoIWD]. All 71 stock ERFs (14 ERF, 29 HAK, 28 MOD) are V1.0 [D]. neverwinter.nim can also write an `E1` data version with optional zstd compression; nothing in the game changelogs mentions it and no stock file uses it — **don't emit it**.
- **Exported .erf** carries `ExportInfo.gff` (FileType `GFF `): Mod_MinGameVer, Expansion_Pack, Comments, `Top`, `Dependencies`, `Missing` [T]; toolset uses EXPORTTEMP/IMPORT work dirs.
- Other module-resident types the toolset writes/keeps: `.itp` (palcus), `Repute.fac`, `module.jrl`, `.gic`; legacy RESTYPEs DFT (area defaults, unused), UTG (item generator, unused), SSF, PTM/PTT [wiki Modules]. EE: a .mod may hold any resource type except HAK/ERF/MOD/SAV/KEY/BIF; 89 toolset saves "all valid file types" into the .mod [Doc].

---

## C. Toolset-side-only data in a module

| Item | Where | Notes |
|---|---|---|
| Instance comments + sound preview flag | `<area>.gic` | B4. Game ignores (but CopyArea/CAF include it) |
| Blueprint `Comment`, `PaletteID` (UTM: `ID`) | every UT* | stripped from GIT instances (creature Comment kept) |
| Custom palettes | 9 × `*palcus.itp` | regenerated from blueprints; DM Creator uses them |
| Journal | `module.jrl` | B15 |
| Factions | `Repute.fac` | B16 |
| Script sources / debug | `.nss`, `.ndb` | .ndb only with the debug option |
| Plot wizard | `.ptm`, `.ptt` | legacy |
| Module-level toolset fields | `Mod_Area_list` (area list for the tree), `Mod_Entry_*` (start location), `Mod_HakList`, `Mod_CustomTlk`, `Mod_MinGameVer`, `Mod_CacheNSSList` (legacy, dropped in 1.80.8193.14), `Mod_Expan_List`, `Mod_CutSceneList`, `Mod_GVar_List` | Mod_* read by the engine too |
| Area editor state | none in GFF | tabs/view state are in nwtoolset.ini / settings |
| Work dirs | `modules/temp0/` (unpacked module), `.BackupMod` (backup), `WORKTEMP`, `EXPORTTEMP` | [T]. Module folder mode: a folder `modules/<modname>/` is opened instead of unpacking (1.75; "always open module directories" setting 1.80.8193.12); prompt text "A directory with the same name has been found. Do you want to open this directory as a module?" [T]; on save the folder is packed into the .mod |
| Resman for the toolset | base keys, `development/` (read since 1.83.8193.21), `nwsync/` [T strings `\development`, `\nwsync`], haks from Mod_HakList (highest first), override | toolset loads development folder but may not live-reload [Doc] |

---
## B-summary. Master list of EE-added GFF fields (toolset-relevant)

| File | Field(s) | Type | Version | Evidence |
|---|---|---|---|---|
| IFO | Mod_OnPlrTarget | RR | event 1.80.8193.14; GFF/UI 87.8193.35 (persisted to saves 1.83.8193.26) | D (DoIWD), T, S, Doc |
| IFO | Mod_OnPlrGuiEvt, Mod_OnPlrTileAct | RR | events 1.85.8193.30; UI 87 | D, T, S, Doc |
| IFO | Mod_OnNuiEvent | RR | NUI 85.8193.31/32; UI 87 | D, T, S, Doc |
| IFO | Mod_DefaultBic | RR | 87.8193.35 ("no GUI yet") | D, T, Doc |
| IFO | Mod_PartyControl | I (0/1/2) | 87.8193.35 ("no GUI yet") | D, T, S, Doc |
| IFO | Mod_UUID | Str | undocumented; toolset preserve-fix 88.8193.36 | D, T, S, Doc(indirect) |
| IFO | Mod_CacheNSSList | L | **removed** from toolset 1.80.8193.14 (legacy 1.30) | D, T(absent), Doc |
| ARE | TileBrdrDisabled | B | ≤89 (script API 88.8193.36) | D, T, S |
| GIT | VisualTransform {ScaleX/Y/Z…} | St | 1.79 (toolset visual transforms) | D (WCoC, ToM), T(read) |
| GIT | VisTransformList {Scope, AnimationSpeed/Scale*/Rotate*/Translate* {TimerType, ValueTo, LerpType, [ValueFrom, LerpDuration, LerpProgress]}} | L | lerps 1.83.8193.21; scopes later (≤89) | D (DoIWD), T, S |
| UTC / GIT creatures | xAppearance_Head, xBodyPart_* (17), xArmorPart_RFoot | W | 87.8193.35 limit 255→999; toolset fix 88.8193.36 | D (DoIWD) |
| UTI/GIT items | xModelPart1-3, xArmorPart_* (19, truncated labels) | W | same | D (DoIWD) |
| DLG | ActionParams (Entry/Reply), ConditionParams (links, StartingList) {Key, Value} | L | 1.80.8193.14 / 1.81.8193.15 | D (lists), T (Key/Value), Doc |
| ITP | NAME (Str) on category nodes (was DELETE_ME) | Str | 1.79 (tileset palettes 1.80.8193.6) | Doc, wiki |
| BIC | DataMigration | B? | 1.79 | Doc, S |
| BIC/IFO(save) | SQLite blob (`SQL3`) | V | 1.80.8193.14 | Doc, S |

Pre-EE fields that look new because they're missing from 1.2x/1.62 files, with their real origin [Doc NWNv169]: Mod_OnPlrChat (1.69), Wings_New/Tail_New/GenericType_New (1.69, 2DA rows > 255), placeable OnClick (1.67), Pickpocketable (≤1.66), Lootable/DecayTime, ArmorPart_Robe, Cursed, store IdentifyPrice/MaxBuyPrice/StoreGold/OnStoreClosed/WillNotBuy/WillOnlyBuy (1.61), Mod_HakList (multiple haks 1.28), PreventZoomIn (1.27), Mod_CacheNSSList (1.30), VarTable (toolset variables ≤1.61).

Checked and **not** EE fields (don't exist anywhere [D,T,S]): creature `DisplayName`, `UseTweakedStuff`, "Door Tlk", `Mod_Cache`. No new per-object or area event slots in EE (object script fields identical to 1.69 [D]). No NWSync field in module.ifo.

---

## D. Chronological list of EE toolset changes (features / fixes / known issues)

Compiled by a sub-pass over `$G/lang/en/docs/CHANGELOG.md` (= patchnotes/*.md, identical text), `$G/lang/en/docs/Neverwinter Nights Enhanced Edition (v74…v79).txt`, `legacy/NWNv169.txt`, and the nwn.wiki patch pages. Format: `[build] (F)eature/(X)fix/(K)nown issue/(C)hanged — paraphrase — source/section`. `[inf]` = inference.
### D sources and abbreviations

- `CL` = `$G/lang/en/docs/CHANGELOG.md` (covers 85.8193.32 .. 89.8193.37-17; `##` = release, `###` = Added/Changed/Fixed...). `$G` = `~/.local/share/Steam/steamapps/common/Neverwinter Nights`.
- `$G/lang/en/docs/patchnotes/*.md` are byte-for-byte the same text as the CL sections (verified by diff; only a missing newline differs), so they are not cited separately.
- `v74.txt` .. `v79.txt` = `$G/lang/en/docs/Neverwinter Nights Enhanced Edition (v74..v79).txt` (cp1252, CRLF). v74.txt lines 1-524 = EE v74 summary; lines 526+ = embedded copy of the 1.69 and older patch notes. v75..v78 are short per-version files; v79.txt = the 1.79.8193 stable notes.
- `W:<page>` = local nwn.wiki mirror `~/.local/opt/neverwinter/wiki/pages/NWN1/<file>`; its URL is given once per page in the per-release header.
- GAP in local game docs: nothing between v79.txt (1.79.8193, 2019-12) and CL 85.8193.32 (2021-09). The builds in between (1.79.8193.1 .. 1.85.8193.31) come ONLY from the wiki. The wiki has no pages for 1.75-1.78 builds (8168-8187ish); those come only from v75-v78.txt (undated, no build numbers).
- Version scheme (W:26738926 Patches NWN:EE, https://nwn.wiki/spaces/NWN1/pages/26738926/Patches+NWN+EE): `1.80.8193.14` = Edition 1 / "Toolset Version" 80 / Major 8193 / Minor 14. The 1.xx part is the toolset/module compatibility version; "A 1.80 module cannot be opened in a 1.79 client". Since 85.8193.32 the version is written without the leading "1." (85 = 1.85 ...). 89.8193.37-13 etc: "-NN" is the build suffix of the .37 line.

Legend: (F)=feature, (X)=fix, (K)=known issue, (C)=changed behaviour. "[inf]" marks my inference, not stated by the source.

---

### Pre-EE baseline: see "D-baseline" at the end of section E

### 1.74.8154 — 2018-01-12 (Head Start/pre-beta)
W: https://nwn.wiki/spaces/NWN1/pages/38174746/1.74.8154
- No toolset items. (Area instancing CopyArea/CreateArea fixes, SetDescription limit 8KB->128KB [game side]).

### 1.74.8155 — 2018-01-18
W: https://nwn.wiki/spaces/NWN1/pages/38174748/1.74.8155
- [8155] (X) crash when custom content has empty scripts in the "caching list" (nullptr) — engine, relates to module "script caching" list [inf: Mod_CacheNSSList] — W 1.74.8155 "v74.8155 Fixes"
- [8155] (X) TXI `rotatetexture` handled wrongly by new shaders (city interior carpets misaligned) — renderer; relevant for toolset area rendering of tiles — W 1.74.8155 "Regression Fixes"

### 1.74.8156 — 2018-02-02
W: https://nwn.wiki/spaces/NWN1/pages/38174750/1.74.8156
- No explicit toolset items (PLT reading crash fix for area-placed items; skinmesh armour parts; experimental normal/specular maps).

### 1.74.8157/8158 — 2018-02-09
W: https://nwn.wiki/spaces/NWN1/pages/38174752/1.74.8158
- No explicit toolset items. MDL: normals and tangents read from ASCII; animeshes support TSB.

### 1.74.8159 — 2018-02-15 (first public beta PC/Mac/Linux)
W: https://nwn.wiki/spaces/NWN1/pages/38174744/1.74.8159
- No explicit toolset items. Introduces the `.mtr` material concept (fields customshaderVS/FS, texture0..texture14, `parameter float|int`), mesh `materialname`; mesh per-vertex `colors` stream; extra UV streams `tverts1..3`. Steam Workshop thumbnail.jpg/png in mod root.

### 1.74.8160 — 2018-02-23 ("Toolset fixed, MP relays")
W: https://nwn.wiki/spaces/NWN1/pages/38174754/1.74.8160
- [8160] (X) Toolset memory leak after every script compile — W 1.74.8160 "Fixes"
- [8160] (X) Toolset crash when static-lighting a mesh with zero vertices (custom tilesets, "humongous bell") — "Regression Fixes"
- [8160] (X) Toolset crash on Intel GPUs — "Regression Fixes"
- [8160] (F) `.mtr` material format supported (engine; toolset renders same models [inf])
- [8160] (F) per-vertex static lighting works with normal mapping (stream of brightest-light direction per vertex) — relevant to toolset static lighting/"Recompute Static Lighting" [inf]

### 1.74.8161 — 2018-02-28
W: https://nwn.wiki/spaces/NWN1/pages/38174756/1.74.8161
- [8161] (X) huge shadows-related memleak in toolset when loading areas — "Fixes"
- [8161] (F) Toolset remembers window position between starts — "Fixes"
- [8161] (F) Toolset performance when loading/accessing placeable-related functions improved — "Features"
- [8161] (F) Toolset performance with object tree in area view improved — "Features"

### 1.74.8162 — 2018-03-05
W: https://nwn.wiki/spaces/NWN1/pages/38174758/1.74.8162
- Roll-up of 8160+8161 notes (same toolset items repeated: compile memleak, zero-vertex static-light crash, Intel GPU crash, area-load shadow memleak, window position, placeable perf, object tree perf).

### 1.74.8163 — 2018-03-07
W: https://nwn.wiki/spaces/NWN1/pages/38174760/1.74.8163
- [8163] (X) Toolset can now properly select WBM movies instead of BIK (module/area movie pickers) — "Fixes"
- [8163] (X) Water static lighting problems fixed — "Fixes" [engine; likely shared with toolset static lighting, inf]

### 1.74.8164 — 2018-03-16
W: https://nwn.wiki/spaces/NWN1/pages/38174762/1.74.8164
- [8164] (X) Toolset crash when re-opening a conversation from search results — "Fixes"
- [8164] (X) Toolset runs on Wine again — "Fixes"
- [8164] (X) Toolset script compiler no longer crashes on `#include` of a missing script — "Fixes"
- [8164] (C) Toolset defaults to the NWN user directory when importing/exporting ERFs — "Fixes"
- [8164] (X) Toolset crash when deleting an area with an uppercase resref — "Fixes"
- [8164] (X) further toolset memleaks fixed — "Features"
- [8164] (F) Toolset script editor accepts cyrillic/non-ASCII input — "Features"
- [8164] (F) Toolset script compiler identifier limit 8K -> 16K (error was "IDENTIFIER LIST FULL") — "Features"
- [8164] (F) Toolset keeps local variables on triggers and encounters when repainting their polygon — "Features"
- [8164] (F) Toolset much faster opening object properties — "Features"
- [8164] (F) Area editor: selected objects don't move on single click; properties dialog opens on double click; undo/redo for object move — "Toolset area editor improvements"
- [8164] (F) NWScript GetEventScript/SetEventScript (runtime; toolset-set event slots can be changed at runtime)
- [8164] (F) Steam Workshop tags via `tags.txt` in project root

### 1.74.8165 — 2018-03-22 (Linux libs only) — no toolset items.
W: https://nwn.wiki/spaces/NWN1/pages/38174764/1.74.8165

### 1.74.8166 — 2018-03-23 (+hotfix 2018-03-26); EE retail launch 2018-03-27
W: https://nwn.wiki/spaces/NWN1/pages/38174769/1.74.8166
- [8166] (X) "Paste (as in copy/paste) has been fixed" — game-wide text input [inf: not toolset]
- No explicit toolset items.

### 1.74.8167 — 2018-04-10 (Development build) — no toolset items (player name 32 -> 127 chars).
W: https://nwn.wiki/spaces/NWN1/pages/38174771/1.74.8167

### v74 summary (v74.txt, "Neverwinter Nights: Enhanced Edition (v74)") — roll-up of 1.74 builds
- (C) WARNING line at top: all modules created/saved with the v74 toolset are tagged as requiring v74+ of game or toolset — v74.txt "Patch Details" [=> module.ifo Mod_MinGameVer; inf]
- (F) toolset much faster (opening/editing objects); numerous crashes/memleaks fixed; no longer moves objects down to walkmesh z on click/edit; undo/redo object moving; double-click to edit; keeps vars on triggers/encounters on polygon repaint; script compiler identifier limit 16K (from 8K) — v74.txt "Toolset Improvements"
- (F) normal + specular maps on any texture; `.mtr` materials per mesh incl. VS/FS shaders — v74.txt "New materials and content authoring tools"
- (X) crash: entries longer than 255 chars in .2DA files; crash on case-sensitive FS (Linux) with non-lowercase resources in override — v74.txt "Crashes Fixed"
- (C) Triggers and doors no longer run default.nss for events if no script declared in Aurora Toolset — v74.txt "Other Issues"
- (C) Max item charges 50 -> 250 — v74.txt "Other Issues" (item editor charges field range [inf])
- (C) Spell-like abilities from the creature Special Abilities tab are now cast at the builder-set caster level — v74.txt "Other Issues"
- (C) module HAK/TLK loaded before chargen in MP — v74.txt "Other Issues"
- (X) crash related to "script caching" — v74.txt "Other Issues"
- (C) SetDescription limit 8KB -> 128KB — v74.txt "Other Issues"
- (C) SetTransitionTarget doc: "toolset-configured destination tag is used for a lookup only once" — v74.txt "New Scripting Commands"

### 1.75 (v75.txt; undated, no build number stated locally)
- (F) The toolset can open unpacked modules: unpack into a directory with the same name as the .mod (without extension) — v75.txt "Content Creation Changes / Features"
- (F) Toolset: move objects along Z by holding Alt and moving the mouse — v75.txt "Content Creation Changes / Features"
- (F) Materials: `.mtr` can specify `renderhint`; meshes need no texture if .mtr set; mesh `bitmap`/`texture0` can name the material instead of `materialname` — v75.txt "Client Changes / Features"
- (F) nwscript: `\"` escape in strings — v75.txt "Content Creation Changes / Features" (compiler)
- (F) Object visual transforms script API (scale/rotate/translate/anim speed) — runtime only in 1.75; toolset support comes in 1.79 — v75.txt
- (X) crash in script compiler with many nested includes — v75.txt "Content Creation Changes / Fixes"
- (X) custom content tiles could not be laid down in the toolset (notably Seasonal Forest) — v75.txt CC Fixes
- (X) "Stereo WAV in 3D Positional Sound" popping up erroneously in the toolset — v75.txt CC Fixes
- (X) toolset not displaying spells or feats on character sheets — v75.txt CC Fixes
- (X) "sqrt DOMAIN error" on binary models without valid tangent data — v75.txt CC Fixes
- (F) Utils: nwhak.exe and gffeditor.exe re-added to the torrent; nwhak.exe understands new restypes (e.g. .mtr) — v75.txt CC Fixes
- (F) racialtypes.2da new `Icon` column (race icon) — v75.txt Client Features
- (C) nw_g0_conversat is no longer assigned by engine to doors/placeables/creatures with no script set; used only as temporary default — v75.txt Server Fixes

### 1.76 (v76.txt; undated)
- (X) Script Compiler: allows escaping backslashes (`\\`) — v76.txt "Content Creation Changes / Fixes"
- (F) racialtypes.2da name-generator tables configurable — v76.txt Client Features

### 1.77 (v77.txt; undated)
- (F) Toolset: script editor shows variable and constant declarations on double click — v77.txt CC Features
- (F) Toolset: recent modules list expanded to 10 entries — v77.txt CC Features
- (F) Toolset: compiler include limit bumped to 128 — v77.txt CC Features
- (X) Toolset: area ambient sounds and music brought back to life — v77.txt CC Fixes
- (X) Toolset: Z-axis translate (Alt+LMB) no longer requires the mouse over the object — v77.txt CC Fixes
- (X) Toolset: tile properties light select color UI no longer truncated with Windows DPI scaling — v77.txt CC Fixes

### 1.78 (v78.txt; undated)
- (F) NWSync introduced (server-side manifests; github.com/Beamdog/nwsync) — no toolset item — v78.txt "New Feature: NWSync"
- (F) normal DDS without header mangling; KTX texture containers — v78.txt Client Features

### 1.79.8192 — 2019-10-10 (preview/dev towards 1.79 stable; = toolset 1.5.0.2 beta)
W: https://nwn.wiki/spaces/NWN1/pages/60982625/1.79.8192
- (C) "Module compatibility has been bumped to 1.79 due to the added script commands and the above itp change" — W "Features"
- (F) DM Creator palettes (ITP) may contain plain CExoString `NAME` field instead of STRREF (node previously named `DELETE_ME`) — useful for tileset authors — W "Features"
- Toolset section says "brings it in line with patch version 1.5.0.2 as seen in the Beta Toolset release thread"
- Toolset features/fixes = same list as 1.79.8193 below (+ "Additional fixes for 'sqrt domain' errors on fancymapped content")

### 1.79.8193 — 2019-12-03 stable (RC 2019-12-02). First 64-bit-only release.
W: https://nwn.wiki/spaces/NWN1/pages/60982619/1.79.8193 ; local v79.txt "Toolset Changes"
- (F) Visual transforms in toolset: new options in Adjust Location dialog; Ctrl+Mousewheel scales; only creatures, items, doors and non-static placeables — v79 "Toolset Changes"
- (F) Multiple areas in tabs: off by default, Toolset settings -> Area editor -> "Open areas in tabs" (restart); right-click tab -> Close — v79
- (F) Inventory editor key bindings: arrows move selection; 'I' sets Infinite flag; Delete deletes; after delete selects next item — v79
- (F) Build Module and Update Instances significantly optimized; expanding a not-open area faster — v79
- (K) Toolset built with a newer C++ compiler; regressions vs the 1.78 toolset possible — v79
- (X) reputation editing in Faction Editor — v79
- (X) renaming areas in the main area list — v79
- (X) Z translation saved when it's the only transform defined; "Fixed Z translations not saving" — v79
- (X) access violation dragging equipped items to trash — v79
- (X) toolbar icons becoming disabled on double-click of objects in area list — v79
- (X) Unicode issues in Creature properties dialog; Unicode issues in Token Selector dialog — v79
- (X) undo issues with tabs; fog rendering with tabs — v79
- (C) Visual transforms reset when Static flag set on a placeable — v79
- (X) tab order for rotation fields in Adjust Location — v79
- (X) "sqrt domain" errors on some fancymapped content — v79
- (X) Update Instances dialog layout — v79
- (X) memory leak when switching between areas — v79
- (X) Test Module passes `-userdirectory` to the game — v79
- (F) engine: object UUIDs "persisted to GFF" (items, creatures, placeables, triggers, doors, waypoints, stores, encounters, areas) — v79 "Further Features"
- (F) engine: user-dir `development/` folder (top of resource search path, hot reload for uncached types) — v79 "Further Features"
- (F) engine: ResMan rewritten — v79
- (F) engine: music/ambient sounds loaded through ResMan (can be in hak/nwsync); plain mp3 read if renamed .bmu — v79
- (F) DM Creator palettes CExoString `NAME` — v79
- (X) script compiler confusing functions where one name is a prefix of another ("Action" vs "ActionTwo") — v79 "Fixes"
- (F) new GFF field `DataMigration` in CreatureStats struct (BIC; Dragon Disciple migration) — v79 "Content Creation"
- (F) new 2DA: ruleset.2da; classes.2da `StatGainTable`; racialtypes.2da several new columns (extra skill points/feats at 1st level, point buy, feat progression, `SkillPointModifierAbility`) — v79 "Content Creation"

### 1.79.8193.1 — 2019-12-06 (hotfix)
W: https://nwn.wiki/spaces/NWN1/pages/60982617/1.79.8193.1
- (X) "We fixed the toolset build." — W "Fixes"

### 1.79.8193.2 — 2019-12-09
W: https://nwn.wiki/spaces/NWN1/pages/60982614/1.79.8193.2
- (X) "development" alias now works on dedicated servers too — W "Fixes" (no toolset item)

### 1.79.8193.3 — 2019-12-10 (stable)
W: https://nwn.wiki/spaces/NWN1/pages/60982612/1.79.8193.3
- (X) Toolset sound and ambient music "raised from the dead" — W "Fixes"

### 1.79.8193.4 — 2019-12-13
W: https://nwn.wiki/spaces/NWN1/pages/60982610/1.79.8193.4
- (X) Toolset no longer tries to deallocate the starting location when closing an area that doesn't have it — W "Fixes"

### 1.79.8193.5 — 2019-12-19
W: https://nwn.wiki/spaces/NWN1/pages/60982606/1.79.8193.5
- (X) Toolset no longer truncates Description and Comments fields — W "Fixes"
- (X) ResMan priority restored for haks vs modules — W "Fixes"
- (X) SP 2da cache not cleared between module runs with different CC — W "Fixes"
- (F) ResMan debug UI buttons; config option to log failed resource lookups — W "Fixes"

### 1.80.8193.6 — 2020-01-13 (development patch; module version 1.80 starts here [inf from page title])
W: https://nwn.wiki/spaces/NWN1/pages/60982603/1.80.8193.6
- (F) Toolset allows selecting heads from custom slots 50-99 — W "Toolset"
- (X) Up/down arrow keys in Adjust Position now progress in perfect sequence — W "Toolset"
- (X) double-clicking an area entry could grey out parts of the toolbar — W "Toolset"
- (X) truncation issue in script editor search & replace — W "Toolset"
- (F) custom tileset palettes may use a `NAME` CExoString field instead of a STRREF — W "Toolset"
- (X) object descriptions > 2000 chars no longer truncated — W "Toolset"
- (X) crash moving the start location between areas open in multiple tabs — W "Toolset"
- (X) a group of raised objects retains z-level when moved — W "Toolset"
- (F) baseitems.2da new columns WeaponFocusFeat, EpicWeaponFocusFeat, WeaponSpecializationFeat, EpicWeaponSpecializationFeat, WeaponImprovedCriticalFeat, EpicWeaponOverwhelmingCriticalFeat, EpicWeaponDevastatingCriticalFeat, WeaponOfChoiceFeat — W
- (F) iprp_visualfx.2da (custom weapon VFX) — W
- (F) classes.2da custom caster columns: MemorizesSpells, SpellbookRestricted, PickDomains, PickSchool, LearnScroll, Arcane, ASF, SpellcastingAbil, SpellTableColumn, CLMultiplier, MinCastingLevel, MinAssociateLevel, CanCastSpontaneously; ruleset COMPANION_LEVELS_STACK — W
- (F) `.lod` files (LOD model swapping, up to 3 levels) — W
- (F) 50 more custom animation slots (LOOPING_CUSTOM21..70) — W
- (F) ResMan >256MB cache; "priorities of userpatch, modules, haks, and override have been restored" — W "ResMan Priorities"
- (X) shader: tile borders in "Dungeon" tileset showed illumination when meant black; mipmaps for uncompressed textures — W

### 1.80.8193.7 — 2020-02-27 (development)
W: https://nwn.wiki/spaces/NWN1/pages/60982599/1.80.8193.7
- (X) Script editor no longer freezes when trying to compile `/**/` [shown as **/** in wiki markdown; inf: a comment-only token sequence] — W "Toolset"
- (X) Shield equipping logic in the creature editor fixed — W "Toolset"
- (C) Tileset data: 250+ tile collision/walkmesh fixes, 150+ light flag fixes, emitter/anim fixes, all 192 doors checked; new "Doorcap, Interior" feature tile for TNI01/TNI02; negative values in SET files fixed; "ovr/ has been cleaned out into the keyfiles" — W "Tileset changes"/"Game Features" (toolset tile painting data changed [inf])
- (F) ruleset.2da chargen constants (CHARGEN_*); chargenclothes.2da — W
- (F) renderhint `NormalTangents` (MTR) — W "Perf"
- (X) tile path nodes not showing up; ground triggers disappearing or losing tint — W "Game Fixes" (engine rendering)
- (F) Support for >17 bones — W "Game Fixes"

### 1.80.8193.8 — date not given (development, between 02-27 and 03-17)
W: https://nwn.wiki/spaces/NWN1/pages/60982595/1.80.8193.8
- No toolset items. ruleset ALLOW_CUSTOM_PORTRAITS; tileset walkmesh repairs; WCoC/PotSC premium modules now include script sources; "Removed duplicate/conflicting SET, ITP, 2DA and TGA files" (WCoC).

### 1.80.8193.9 — 2020-03-17 (STABLE "Patch 1.80"; cumulative since 1.79/8193.5)
W: https://nwn.wiki/spaces/NWN1/pages/60982592/1.80.8193.9
- (C) "Module compatibility has been bumped to 1.80." — W "Misc"
- Toolset section = union of 8193.6 + 8193.7 toolset lists (heads 50-99; Adjust Position arrows; toolbar greying; search&replace truncation; tileset palette NAME CExoString; >2000-char descriptions; start-location crash with tabs; raised group z-level; `/**/` freeze; shield equip logic) — W "Toolset"
- (X) tileset data regression in 2 water tiles (Blacklake District) — W "Tileset changes"

### 1.80.8193.10 — 2020-04-09 (development)
W: https://nwn.wiki/spaces/NWN1/pages/38176092/1.80.8193.10
- (X) Access violation when compiling scripts with nested structs — W "Toolset"
- (X) Tile data: doors on area edges fixed (#13); TWC03_C43_02 SET X-position negative scientific number -> 0.0 — W "Tileset Changes" (SET parser robustness [inf])
- (X) NWSync offline modules: CURRENTGAME resman priority shuffled (offline NWSync modules get lower HAK priority) — W "Fixes"
- (F) GetPlayerBuildVersionMajor/Minor — W

### 1.80.8193.11 — 2020-04-24 (development)
W: https://nwn.wiki/spaces/NWN1/pages/38176090/1.80.8193.11
- (X) [NVIDIA] Toolset: no longer necessary to force "threaded optimisation" off manually — W "Fixes"
- (F) Toolset: Music and Ambient Sounds can be loaded from ResMan (haks) if listed in the 2da — W "Fixes"
- (C) Server: GFF validation performance optimised (large BICs with many local vars) — W "Fixes"

### 1.80.8193.12 — 2020-05-06 (development)
W: https://nwn.wiki/spaces/NWN1/pages/38176083/1.80.8193.12
- (C) Removed the warning dialog shown when opening a module with haks — W "Toolset"
- (F) Setting to always open module directories (i.e. unpacked module folders) — W "Toolset"
- (C) Outdated/unused registry read + warning UI purged — W "Toolset"
- (F) Conversation editor opening time optimised — W "Toolset"
- (F) Open-module list sorted alphabetically; last-opened module selected by default — W "Toolset"
- (F) "Modern UI skinning has been enabled" — W "Toolset"
- (C) Tabbed area UI now on by default — W "Toolset"
- (X) crash pasting objects from a closed area — W "Toolset"
- (X) placeable properties UI opened the wrong dialogue (conversation) file — W "Toolset"
- (F) nwhak.exe: support for ktx, ttf, sql, tml, sq3, lod file types — W "Fixes" (ERF restype list [inf])
- (X) VM: script commands not accessing area UUIDs (#41) — W "Fixes"
- (X) renderaabb not showing on all tiles (#43) — W "Fixes"
- (C) TTZ01_EDGE.2da new entry (elevated stream with grass); TN_SDOOR_19 reclassified Type=Door — W "Art"

### 1.80.8193.13 — 2020-05-13 (STABLE; includes .10 .11 .12)
W: https://nwn.wiki/spaces/NWN1/pages/38176086/1.80.8193.13
- (X) Creature Wizard: stray "-1" text removed (#50) — W "Toolset"
- (X) Area tile light color picker no longer shows blank colors (#49) — W "Toolset"
- (X) Inventory window renders icon backgrounds properly (#47) — W "Toolset"
- (X) `.lod` parser handles files without trailing newline — W "Fixes"
- (+ repeats .10/.11/.12 toolset lists)

### 1.80.8193.14 — dev 2020-08-07 (big feature patch; wiki .15 page: ".15 is the stable patch version of 1.80.8193.14")
W: https://nwn.wiki/spaces/NWN1/pages/38174859/1.80.8193.14
- (F) Conversation editor: script parameters can be specified for conversation scripts; read with GetScriptParam(); SetScriptParam() before ExecuteScript — W "Conversation script parameters" (DLG GFF fields not named in the note; see E)
- (F) New module event OnPlayerTarget (EnterTargetingMode, GetTargetingModeSelectedObject/Position, GetLastPlayerToSelectTarget). (K) "The toolset cannot currently configure this module event. You need to set it via SetEventScript() at module load." — W "Scripted mouse targeting mode"
- (F) Modules and HAKs (ERF) can contain more than 16k resources (was 16356 limit) — W "Modules and HAKs can now contain more than 16k items"
- (F) SetAreaWind(): note lists the toolset's predefined wind presets: NONE dir(1,1,0) mag 0 yaw 0 pitch 0; LIGHT mag 1.0 yaw 100 pitch 3; HEAVY mag 2.0 yaw 150 pitch 5 — W "Scripted wind management"
- (F) New lighting engine (PBR, tone mapping, per-pixel, up to 32 dynamic lights); new water rendering with reflections incl. tile lights ("builders using tile lights will now see them in the reflections"); grass sorted by distance — W "New Lighting Engine"/"New Water Rendering"/"Grass Rendering" (affects how areas look; toolset renderer not mentioned)
- (F) Ossian content imported: Medieval City, Medieval Rural, Mountain Snow tilesets (Zwerkules), Lizardfolk Interior + Seaships microsets, doorway tiles, 476 placeables, 13 creature models, skybox set, 42 music, 54 loadscreens, 25 ambient sounds, 5 soundsets, and "Palettes and blueprints for all imported Ossian content" — W "Content import from Ossian premium modules"
- (F) progfx.2da (unhardcoded ProgFX); walk anims walk_002+; custom mouse cursors (MOUSECURSOR_CUSTOM_00); VFX may use PLT; SetObjectHiliteColor; SetObjectMouseCursor — W
- (F) SQLite: Module DB persisted to savegames; Player DB saved into the .bic — W "Scripted access to SQLite databases" (BIC/sav content change; not toolset)
- (C) Texture pack support removed; all texture-pack content merged into nwn_base.key — W "Miscellaneous Improvements"
- (F) DDS BC4/BC5 support — W "Miscellaneous Improvements"
- (F) data build ships 2da.zip — W "Miscellaneous Improvements"
- (X) TemplateResRef now stored/loaded from GFF for doors (GetResRef no longer "") (#100) — W "Fixes"
- (X) Object hilite state properly persisted to GFF — W "Fixes"
- (X) off-by-one when adding journal entries — W "Fixes"
- (C) nwhak.exe moved from util/ to bin/ — W "Fixes"
- (X) Toolset: bearing not saved for placeables and doors — W "Toolset"
- (C) Toolset: texture pack selector removed — W "Toolset"
- (X) Toolset: errant "OK" label in Area Properties removed (#59) — W "Toolset"
- (X) Toolset: portrait backgrounds in object properties — W "Toolset"
- (X) Toolset: update static lighting after changing tile light properties — W "Toolset"
- (X) Toolset: Bearing float no longer flips sign when saving the area GFF — W "Toolset"
- (F) Toolset: script editor responsiveness + save/load module performance improved — W "Toolset"
- (F) Toolset: area view settings applied to all open tabs — W "Toolset"
- (F) Toolset: menu option to toggle AABB rendering — W "Toolset"
- (C) Toolset: no longer selects last-opened module in the save UI — W "Toolset"
- (C) Toolset: UI for caching scripts removed ("underlying system has been disabled for a while") — W "Toolset" [=> module.ifo Mod_CacheNSSList no longer edited; inf]
- (X) Toolset linked to 32-bit OpenAL-soft (sound woes) — W "Toolset" [=> toolset still a 32-bit exe in 2020; inf]
- (C) Art: DAG01.set no longer offers unsupported height transitions (#70); TTF01.set typo kept minimap from showing (#77) — W "Art Changes"

### 1.81.8193.15 — 2020-09-15 (STABLE of the .14 feature set; "Patch 1.81")
W: https://nwn.wiki/spaces/NWN1/pages/38174976/1.81.8193.15
- (C) "Singleplayer modules made with this new patch 1.81 cannot be played on older versions" — W "Greetings, friends!" (module version bump to 1.81 [inf: Mod_MinGameVer "1.81"])
- Toolset list identical to 8193.14 (bearing save for placeables/doors; texture pack selector removed; Area Properties "OK" label; portrait backgrounds; static lighting refresh after tile light change; Bearing sign flip; script editor + save/load perf; area view settings to all tabs; AABB toggle menu; save UI no preselect; script-caching UI removed; 32-bit OpenAL-soft) — W "Toolset"
- (F) Conversation editor script parameters; OnPlayerTarget module event (K: toolset cannot configure it yet) — same as .14
- (F) Tileset facelifts for Forest (TTF02) and Rural Winter (TTS02) by Zwerkules; config toggle (default on) substitutes them in campaigns/official DLC; custom modules can use them as new tilesets; "As the tile layout is compatible, you could also just open the .are file in GFFEditor and replace the tileset reference." — W "Content import from Ossian premium modules" [=> ARE `Tileset` resref swap is layout-compatible between tts01/tts02, ttf01/ttf02; inf]
- (F) weathertypes.2da (unhardcoded weather types; page typo "weatherypes.2da") — W "Weather Types Unhardcoded"
- (X) Art: TSS13 (Seaships) boats causing a toolset crash (#142) — W "Art Changes"
- (C) Art: TNO01 restored 3 thatch houses that never made the palette; new doors added to doortypes.2da (tn_sdoor_03, tn_sdoor_25) — W "Art Changes"
- (F) Shaders: defines BUILD_VERSION and BUILD_REVISION — W "Miscellaneous Improvements"
- (X) Renderer: needless recalculation of static lights in full dynamic light mode (perf with some static tile lights) — W "Fixes"
- (X) wind direction variation offset by 120 degrees at peak — W "Fixes"

### 1.81.8193.16 — 2020-09-16 (hotfix, client-side only)
W: https://nwn.wiki/spaces/NWN1/pages/38174991/1.81.8193.16
- (X) Music tracks added by the DoD/TotM content import now show properly in the toolset (not "Bad Strref") — W "Fixes"
- (X) game no longer reads the N: drive when loading a non-existent supermodel — W "Fixes"
- (K) "Hosting a module from within the DM client does not update/show the creator palettes correctly" — W "Known Issues"
- Wiki summary: 2DAs updated vs .13: ambientmusic, ambientsound, appearance, doortypes, genericdoors, loadscreens, placeables, portraits, ruleset, skyboxes, soundset, tailmodel, visualeffects; new: dag01_edge, progfx, tcm02_edge, trm02_edge, trs02_edge, ttf02_edge, tts02_edge, weathertypes — W "Summary" (links Finaldeath/NWNEEGameData diff)

### 1.81.8193.17 — date "na" (dev)
W: https://nwn.wiki/spaces/NWN1/pages/38175058/1.81.8193.17
- (X) Toolset: shadow rendering with fog disabled — W "Toolset"
- (X) Toolset: VFX emitters not rendering properly (#162) — W "Toolset"
- (X) Toolset: some shadows not rendering properly — W "Toolset"
- (C) Config "Hide secondary story tiles" disabled by default (game) — W "Changes"
- (C) Model loader: mesh ambient and diffuse default to vec3(1.0) — W "Fixes" (affects model rendering defaults; toolset renderer parity [inf])
- (C) Art/data: TSS13_edge.2da added; doortypes.2da blueprint refs for TTF02 elven doors; loadscreens.2da default tileset entries; placeables.2da fixes; PlaceablePalStd.ITP new entries/bogus entries removed; portraits.2da entries; tailmodel.2da 5000 "Half-Dragon"; DoD/TotM placeable use nodes renamed to strict 13-char length — W "Art Fixes"
- (F) nwserver -moduleurl / -modulehash (server boots module from NWSync repo written with --with-module) — W "Using NWSync Serverside"

### 1.82.8193.20 — 2020-12-15 (STABLE; .18 and .19 were previews/betas with no wiki page)
W: https://nwn.wiki/spaces/NWN1/pages/38175678/1.82.8193.20
- (X) Toolset: some shadows not rendering properly; VFX emitters not rendering properly — W "Toolset"
- (X) Content: "Fixed some tile models resulting in Access Violation in the toolset" — W "Content Updates"
- (C) Content: TRM02.SET missing crosser entry added; TCM02.SET tile name typos fixed; TSS13_edge.2da; PlaceablePalStd.ITP changes; doortypes/loadscreens/placeables/portraits/tailmodel 2da changes — W "Content Updates"
- (C) "Increased module description size to avoid cutting off text in the multiplayer browser" — W "Fixes" [unclear whether this is a GFF/Mod_Description limit or only the MP advert; inf: MP advert]
- (F) nwscript.nss: added missing tileset resref constants (TILESET_RESREF_*) — W "Other Updates"
- (C) Material filenames and params now allow underscore — W "Other Updates"
- (C) Cutscenes: "Hide Second Story Tiles" save/restore fixed — W "Other Updates"
- (C) Normal maps read as two-channel textures (BC5) — W "Renderer Improvements"

### 1.83.8193.21 — 2021-03-12 (beta/dev)
W: https://nwn.wiki/spaces/NWN1/pages/38176139/1.83.8193.21
- (X) Toolset: floating point drifting for X/YOrientation fixed — W "Fixes" [=> GIT XOrientation/YOrientation floats of doors/placeables etc.; inf]
- (F) Toolset now reads `DEVELOPMENT:` (the user-dir development/ folder) "but it might not live-reload the same as the game would" — W "Fixes"
- (F) ResMan: Movies can be stored/played from override, ERF (hak, mod) and NWSync — W "Other Features"
- (F) Renderer: material `parameter float DisplacementOffset` (heightmap base level); default vs/fslit_nm/sm shaders usable for all standard PBR setups — W "Other Features"
- (C) Renderer: TXI `decal` now actually disables lighting — W "Fixes"
- (F) Renderer: material files added for icy tileset; old human heads — W "Other Features"
- (X) Renderer: tile lights added to BSP before radii set — W "Fixes"
- (C) VM: ExecuteScriptChunk no longer writes `!chunk.ndb` to override — W "Fixes"
- (F) New script type `cassowary` (GetLocalCassowary/SetLocalCassowary/DeleteLocalCassowary, Cassowary* functions) — W "New script commands" (compiler must know the type; see E)
- (F) GetCurrentlyRunningEvent() returns EVENT_SCRIPT_* — W "New script commands"
- (F) SetTlkOverride(); ItemPropertyCustom() — W
- (F) TLK files get a local cache — W "Other Features"
- (F) SQLite builtin functions for NWCompressedBuf compression, base64, hashing (see SQLite_README) — W "Other Features"

### 1.83.8193.23 — 2021-05-20 (STABLE of the .21 line; wiki note "Final version of 1.81.8193.21" [sic])
W: https://nwn.wiki/spaces/NWN1/pages/38176502/1.83.8193.23
- (X) Toolset: floating point drifting for X/YOrientation — W "Fixes" (repeat of .21)
- (F) Toolset: reads `DEVELOPMENT:` (may not live-reload like the game) — W "Fixes" (repeat of .21)
- (X) Toolset: some backgrounds not filling the full icon pane (#240) — W "Fixes"
- (F) NWScript Debugger revived: debugger binary back in `bin/win32/`; config keys in UI; address parsing fixed (non-localhost works); heap overflow parsing NDB files fixed; renders CGameEffect / CScriptEvent internals; renders `cswysolver` (cassowary) and `sqlquery` types; no longer launched from nwmain, run by hand (works under wine on Linux/Mac) — W "NWScript Debugger" (relevant: toolset option "Generate Debug Information When Compiling Scripts" writes .ndb)
- (X) Renderer: `bumpshinytexture` check restored (meshes transparent/invisible regression #298); static lights not updating properly (#290) — W "Fixes"
- (C) Savegames: when the NWSync manifest is missing, load newest manifest "of the same UUID" (upgrade path for New Game UI modules) — W "Fixes" [module UUID concept; inf]
- (F) NWSync client downloads .wbm movies in manifest — W "Other Feature Changes"
- (C) Art: ttf02 forest facelift referred to missing envmap (transparent metal) fixed — W "Art Changes"
- (K) Steam Workshop modules not shown in the new game launcher — W "Known Issues"

### 1.83.8193.26 — 2021-06-24
W: https://nwn.wiki/spaces/NWN1/pages/38176504/1.83.8193.26
- (X) Toolset: fixed not rendering bounding boxes — W "Other Fixes"
- (X) Game: OnPlayerTarget now actually persisted to save games — W "Other Fixes" [=> module.ifo field; inf]
- (X) Renderer: disabling a tile source light with TILE_SOURCE_LIGHT_COLOR_BLACK now removes the light — W "Renderer"
- (C) Renderer: Early-Z re-enabled; custom shaders that set alpha must supply a TXI with `alphamean` < 1.0 — W "Renderer"
- (C) Renderer: TSB computed at runtime for legacy compiled models; env map fallback to default chrome map; TXI may use "default" env map — W "Renderer"
- (F) Start Game UI: modules can show as published-but-unreleased via future date in the repository version entry — W "Curated Content in Multiplayer" (repository.json, not toolset)
- (C) Tyrants of the Moonsea DLC: content already in base game removed from DLC download — W "Premium Modules"

### 1.84.8193.29 — 2021-07-08 (beta/dev)
W: https://nwn.wiki/spaces/NWN1/pages/38176496/1.84.8193.29
- (C) "Modules saved with the toolset of this patch are flagged as Compat 1.84, due to some new script commands." — W "Summary"
- (C) Create/CopyArea get auto-generated resrefs in the `nw_` namespace; "nw_ is also the same namespace that the toolset reserves for game-builtin content"; savegames now store a serialised image of the .are data; CopyArea can set tag and name — W "Tech" (VM)
- (F) baseitems.2da new columns `IsMonkWeapon`, `WeaponFinesseMinimumCreatureSize`; shields take AC from `BaseAC` column — W "Game, Logic and Rules" (item editor data [inf])
- (F) "Neverwinter Chess" demo module added to game data — W "Game, Logic and Rules"
- (F) EffectRunScript, EffectIcon, HideEffectIcon, GetLastRunScriptEffect(ScriptType) — W "New Script Commands"
- (C) GetCurrentlyRunningEvent gains bInheritParent — W

### 1.85.8193.30 — 2021-07-23 (dev)
W: https://nwn.wiki/spaces/NWN1/pages/48988173/1.85.8193.30
- (C) Module compatibility bumped to 1.85 (tile/area radial actions and GUI modification script commands) — W "Summary"
- (F) New module event OnPlayerTileAction (custom radial actions on tiles; GetLastTileActionId 1..8, GetLastTileActionPosition, GetLastPlayerToDoTileAction) — "new module event you can access via SetEventScript" (no toolset UI yet) — W "Tile/Radial actions, surfacemat.2da"
- (F) New module event OnPlayerGuiEvent (GUIEVENT_*, GetLastGuiEvent*, SetGuiPanelDisabled) — also only via SetEventScript at this point — W "New Script Commands"
- (C) surfacemat.2da supports 64 materials (was 32) and is reloaded as part of custom content — W "Tile/Radial actions, surfacemat.2da" (walkmesh material ids; toolset walkmesh/material UI [inf])
- (K) "The toolset tilefade setting 'Always' doesn't work correctly." — W "Known Issues" (fixed in .31/.32)
- (X) Renderer: static placeables breaking animations of non-static placeables with the same model — W "Fixes"

### 1.85.8193.31 — 2021-09-16 (beta; content = CL 85.8193.32 minus "Changes over build .31")
W: https://nwn.wiki/spaces/NWN1/pages/48988175/1.85.8193.31
- Same toolset list as 85.8193.32 below.

### 85.8193.32 — 2021-09-22 (CL `## [85.8193.32] - 2021-09-22`; wiki says 2021-09-21)
W: https://nwn.wiki/spaces/NWN1/pages/48988177/1.85.8193.32
- (F) Toolset: support for custom caster classes — CL "Other Features"
- (X) Toolset: inconsistencies with shadow rendering — CL "Fixes"
- (X) Toolset: second story tile fade in "Always" mode — CL "Fixes"
- (X) Toolset: areas considered modified when undo stack changes — CL "Fixes"
- (F) Toolset: undo works for mouse wheel object scaling — CL "Fixes"
- (F) Toolset: undo works for Adjust Location dialog — CL "Fixes"
- (X) Toolset: crash right-clicking a tile with a recently-selected creature on it — CL "Fixes"
- (X) Toolset: Replace All not working in backwards search mode — CL "Fixes"
- (F) Script compiler: `\xFF` escape sequences in string literals (`\x00` terminates the string) — CL "Smaller Changes"
- (C) ovr/ cleaned up; everything except scripts moved into keybif — CL "Smaller Changes"
- (F) new `json` NWScript type; `.caf` combined area format (ARE+GIT, RESTYPE_CAF = 2082); nw_inc_gff.nss; TemplateToJson for CAF/ARE/GIT/GIC/UTC/UTI/UTT/UTP/UTD/UTW/UTE/UTM — CL "NWScript JSON Datatype Support"/"NWScript API Additions"
- (F) Scriptable UI (NUI); `.jui` resources (RESTYPE_JUI; its ID was wrong in .31, fixed in .32) — CL "Scriptable UI"/"Changes over build .31"
- (F) PLT textures can be phenotype-specific — CL "Other Features"
- (X) Game: fog clipping distance of areas not stored in savegames — CL "Fixes"
- (C) Renderer: tiledata bounding box used for lower clip of shadow volumes; `rotatetexture 1` breaking normal/displacement maps fixed — CL "Renderer Improvements"

### 85.8193.33 — 2021-09-30 (CL)
W: https://nwn.wiki/spaces/NWN1/pages/48988179/1.85.8193.33 (header typo "1.83.8193.33"; beta 2021-09-28)
- No toolset items. (X) area tile source lights showing red after save/load when they were off — CL "Patch notes".

### 86.8193.34.1 — 2021-12-08 (CL; wiki "1.86.8193.34")
W: https://nwn.wiki/spaces/NWN1/pages/53670803/1.86.8193.34
- No toolset items. (X) facelift tilesets tts02/tcm02 texture/model issues; (F) NWSync module versions `localalias` field (backreference for StartNewModule) — CL "Fixes"/"QoL Improvements". New VM: Get2DAColumn, Get2DARowCount, GetScriptInstructionsRemaining, JsonArrayTransform/JsonFind/JsonArrayGetRange/JsonSetOp — CL "New Script Commands".

### 87.8193.35-40 — CL date 2023-05-11 (wiki: dev/beta 2023-03-16, final 2023-05-25)
W: https://nwn.wiki/spaces/NWN1/pages/91324440/1.87.8193.35
- (F) Toolset: external script editor (Options -> Script Editor -> External Script Editor) — CL "Added / Toolset"
- (F) Toolset: right-click a script in the left pane -> build that script — CL "Added / Toolset"
- (F) Toolset: Module Properties gets new module events OnPlayerTarget, OnPlayerGuiEvent, OnPlayerTileAction, OnNuiEvent — CL "Added / Toolset"
- (C) Toolset: Build Module observes changes in temp0/ when compiling scripts — CL "Added / Toolset"
- (C) Toolset: no longer artificially restricts which item/body part ranges are available — CL "Added / Toolset"
- (F) module.ifo `Mod_DefaultBic` resref: default character (.bic shipped in the .mod); launches immediately with it; "no GUI option yet" — CL "Added"
- (F) module.ifo `Mod_PartyControl` INT: 0=server default, 1=enabled, 2=disabled; "No GUI option yet" — CL "Added"
- (F) up to 8 multiclasses per creature (ruleset MULTICLASS_LIMIT) — CL "Added" (toolset support only in 37)
- (F) 8 new tile pathnodes 'q'..'x' — CL "Added"
- (F) rock/chasm crosser in Medieval Rural 2; missing tiles in Medieval City 2 — CL "Added"
- (C) Script Compiler: string constants in `case` (see HashString); identifiers 16384 -> 65536; max string constant 512 -> 8192; max include files 128 -> 512 — CL "Changed / Building and Scripting"
- (F) built-in constants LOCATION_INVALID, JSON_FALSE, JSON_TRUE, JSON_OBJECT, JSON_ARRAY, JSON_STRING — CL "Added"
- (F) CompileScript() VM function — CL "82 new NWScript functions"
- (C) body part and armor variation limit 255 -> 999 (older clients use part 0 for >255) — CL "Changed / Custom Content"
- (F) MTR modes `transparency 1`, `twosided 1`, `sample_framebuffer 1/2`, `volumetric 1` — CL "Changed / Custom Content"
- (C) Renderer now OpenGL 3.3; models of classification other than Character cast shadows only if not transparent / all project shadows — CL "Changed / Renderer" & "Custom Content" (toolset renderer parity [inf])
- (C) Area transitions can target any tag (doors and waypoints have priority) — CL "Changed / Building and Scripting" (toolset transition target picker [inf])
- (F) 2DA: iprp_damagetype.2da + damagehitvisual.2da base-damage row; iprp_damagetypes.2da `VisualFX`; spells.2da `TargetShape`, `TargetSizeX`, `TargetSizeY`, `Flags`; skills.2da `HideFromLevelUp`; classes.2da `SkipSpellSelection`; racialtypes.2da `FavoredEnemtyFeat` [sic]; encoding.2da; custom damage types (up to 32) — CL
- (F) dynamic area lighting (nw_dynlight.nss), Get/SetAreaLightColor/Direction; Get/SetTile*, SetTileJson, SetTileAnimationLoops, ReloadAreaGrass/Border — CL "82 new NWScript functions"
- (X) NWScript Debug UI could not run 16-character-long script names — CL "Fixed"
- (X) cubemap env maps specified in TXI; creatures facing east on `_POST` waypoints; skybox fade blending — CL "Fixed"
- (X) several crashes loading broken CC (internal node/texture names too long) — CL "Fixed"
- (F) nui_skin.tml global and per-module — CL "NUI additions"
- (C) NWSync single file max 15MB -> 64MB — CL "Changed / Custom Content"
- (K) wiki: as of preview v.87.8193.35[dd1322cd], custom shaders copying uniforms from inc_water/fs_water give "redefinition error #198" in the toolset — W Shader Engine Support https://nwn.wiki/spaces/NWN1/pages/14614573/Shader+Engine+Support

### 88.8193.36-11 — 2024-02-06 (CL; wiki: dev 2023-12-05, final 2024-02-15)
W: https://nwn.wiki/spaces/NWN1/pages/123797539/1.88.8193.36
- (F) Toolset Area Properties dialog: load/save script sets — CL "Added"
- (X) Toolset: structures passed via `?:` caused bad compiler state — CL "Fixed / Toolset"
- (X) Toolset clobbered module UUID when author manually embedded one with a GFF editor — CL "Fixed / Toolset"
- (X) Toolset: scriptset changes not preserved after loading a scriptset into a Trigger instance — CL "Fixed / Toolset"
- (X) Toolset: armor parts over 255 not working — CL "Fixed / Toolset"
- (X) Toolset: PLT textures not updating correctly when the color is changed — CL "Fixed / Toolset"
- (F) nwscript raw string literals `r"..."` / `R"..."`, may span lines — CL "Added"
- (C) Script Compiler: for loops accept non-integer expressions in init/increment; float literals `0f`, `.0`, `.42f`; `const` may be any constant expression incl. previous consts — CL "Changed / Script Compiler"
- (F) Script compiler open source + standalone (github.com/niv/neverwinter.nim) — CL "Highlights"
- (C) compile-time evaluation of expressions; second bytecode pass melding instructions — CL "Performance / Scripting"
- (F) cachedmodels.2da — CL "Added"
- (C) TemplateToJson/JsonToTemplate also support DLG, UTS, IFO, FAC, ITP, JRL, GUI, GFF — CL "Changed / VM functions"
- (C) GetResRef works on encounters — CL "VM functions"
- (C) 2DA: damagetypes.2da `DamageRangedProjectile`; ammunitiontypes.2da `AmmunitionType`, `DamageRangedProjectile`; packages.2da values > 255; vfx_persistent.2da > 255 for mobile AOEs; classes MemorizesSpells=0 with SpellbookRestricted=0 valid — CL "2DA changes"
- (C) Removed legacy reading of pre-EE encrypted premium modules — CL "Removed"
- (X) server crash on area load due to bad GFF data; client crash loading a module with missing movie; corrupt TGA crash — CL "Crash Fixes"
- (C) on module corruption, game logs the relevant area resref — CL "Logging"
- (X) malformed tile walkmesh causing levitation — CL "Gameplay"
- (X) model part combining ignored differing `materialname` between nodes — CL "Graphics"
- (X) encounter support in ObjectToJson/CopyObject/SqlBindObject/StoreCampaignObject; CopyItem hidden-when-equipped field — CL "Scripting"
- (X) cutscene camera settings use 1.69 values — CL "Misc"
- (C) perf: area loading with many placeables; triggers/encounters memory footprint — CL "Performance / Misc"

### 88.8193.36-12 — 2024-02-23 (hotfix) — no toolset items.

### 89.8193.37-13 — 2025-01-07 (CL; wiki: dev 2024-10-01, final 2025-01-09, hotfix 2025-01-11)
W: https://nwn.wiki/spaces/NWN1/pages/149291011/1.89.8193.37
- (F) Key features list "Toolset quality of life improvements" and "Script compiler improvements from the open source release" — CL "Key Features"
- (F) Toolset: Creature Level Up Wizard, Creature Wizard, Creature Editor support up to 8 classes — CL "Added"
- (F) Toolset: version info in main window banner — CL "Added"
- (F) Toolset: label column fallback for placeabletypes.2da rows without strref — CL "Added"
- (F) Toolset: Palette right-click menu shows Find Next (F3) — CL "Added"
- (F) Toolset: Palette right-click shows Find shortcut (Ctrl-F) — CL "Added"
- (F) Toolset: Label column fallback for strref column in soundset.2da — CL "Added"
- (F) scriptcomp: constant folding of unary `-`, `!`, `~`; `__FUNCTION__`, `__FILE__`, `__LINE__`, `__DATE__`, `__TIME__`; more info in error messages; hashed string literals `h"..."`/`H"..."` — CL "Added"
- (C) Toolset: Creature Editor Properties GUI enlarged (Spells tab) — CL "Changed"
- (C) Toolset: shows model part ID in addition to toolset ordered ID — CL "Changed"
- (C) Toolset: sorts Creature->Appearance->Tail and ->Wings alphabetically; Area->Visual->Skybox alphabetically; Door/Placeable->Advanced->Portraits->Category alphabetically — CL "Changed"
- (C) first row of skyboxes.2da / tailmodel.2da / wingmodel.2da changed so "None" stays at top of the toolset list — CL "Changed"
- (C) new keytable nwn_retail.key (+bifs) with former ovr/ contents; ovr/nwscript.nss is only for human reference, not read by the game — CL "Changed"
- (C) Modules no longer limited to 100 in-use tilesets — CL "Changed"
- (C) Sound objects: TemplateToJson for sound templates; ObjectToJson/JsonToObject/DestroyObject for sound objects — CL "Changed"
- (C) effecticons.2da usable past line 255 — CL "Changed"
- (X) Toolset scales down oversized item icons to final icon size — CL "Fixed" (wiki Icons/Textures/DDS pages: toolset previously cropped oversized icons; 37 supports larger TGA icons)
- (X) compiler stack overflow when folding constants — CL "Fixed"
- (C) no longer auto-merges parts on model level (game-compiled models) — CL "Fixed"
- (C) nw_inc_nui.nss has toolset-compatible function comments — CL "Fixed"
- (X) Toolset loads non-human PLT textures (previously defaulted to human) — CL "Fixed"
- (X) customised palette files `pal_XXXyy.tga` now loaded from haks and nwsync per module — CL "Fixed"
- (X) Toolset: all valid file types are now saved into the .mod on save — CL "Fixed" (wiki Modules page: EE .mod may contain any type except HAK/ERF/MOD/SAV/KEY/BIF; https://nwn.wiki/spaces/NWN1/pages/155516929/Modules)
- (X) scriptcomp: Unidentified Identifier errors now show the identifier name — CL "Fixed"
- (X) crash from static (unenhanced) lighting in certain tilesets — CL "Fixed"
- (X) some music crashing the toolset when played — CL "Fixed"
- (X) TLK: German Y and Z axis labels swapped in Toolset Adjust Location — CL "Fixed"
- (X) Toolset Standard Palette tileset-specific door list updates when switching area tabs; lists door names instead of "Door"; regression vs 1.69 with extra "Door" entries; handling of Standard Palette for Doors — CL "Fixed" (4 entries)
- (X) custom monster perception ranges saved/loaded properly — CL "Fixed" (UTC/GIT PerceptionRange [inf])
- (X) GetFirstInPersistentObject errors for nPersistentZone != 0 on encounters — CL "Fixed"
- (C) Toolset caches door-related 2das — CL "Performance Improvements"
- (C) scriptcomp: optimised `for (i = 0; i < N; ++i)`; dead branches removed for constant `if` — CL "Performance Improvements"
- (F) ContentIndex repository modules can specify a minimum required game version — CL "Added"
- (C) removed log spam about UUIDs missing for locally installed modules — CL "Removed"
- (C) all texturepack (txpk/) code stripped — CL "Removed"
- (F) savingthrowtypes.2da + new column in iprp_saveelement.2da; SAVING_THROW_TYPE_PARALYSIS in nwscript.nss; ruleset ENCOUNTERS_ON_ENTER_FIRE_BEFORE_SPAWN etc. — CL "Added"

### 89.8193.37-14 — 2025-01-11 — no toolset items.

### 89.8193.37-15 — 2025-02-22 (not on wiki page)
- (X) some model parts appeared rotated when rendered in the toolset — CL "Fixed"
- (C) scriptcomp: disabled a for-loop optimisation that could produce an infinite loop — CL "Changed"

### 89.8193.37-16 — 2025-08-17 (not on wiki page)
- (F) scriptcomp: binary `0b` and octal `0o` integer literals; unary plus; `!!x` without parenthesising the second negation — CL "Added"
- (X) script compiler sometimes generated incorrect bytecode; scripts compiled on 37-15 need recompiling only if they misbehave — CL "Fixed"

### 89.8193.37-17 — 2025-10-06 — no toolset items (crash reports end in .txt).

### Wiki-only (non-changelog) toolset facts worth knowing
- Toolset is still a 32-bit Windows application (runs under Wine); DPI scaling best at 100/150/200% — W Aurora Toolset https://nwn.wiki/spaces/NWN1/pages/26738699/Aurora+Toolset
- Hidden nwtoolset.ini keys: `[Start Up] Disable Tabs=0`; `[Display Options] Language=en` (de/en/es/fr/it/pl) — W Toolset Options https://nwn.wiki/spaces/NWN1/pages/60982804/Toolset+Options
- Open Module Folder: rename temp0 to the .mod's base name; toolset prompts "A directory with the same name has been found. Do you want to open this directory as a module?"; can be made default in Toolset Options; on save the folder is packed into the .mod — W https://nwn.wiki/spaces/NWN1/pages/38176075/Open+Module+Folder
- Mod_MinGameVer in module.ifo controls which toolset may open a module (e.g. 1.83 blocks the 1.81 toolset); workaround is editing it with GFFEditor — W https://nwn.wiki/spaces/NWN1/pages/38175645/How+to+Change+the+Module+Version+Id
- ERF resource limit: before 1.80.8193.14 16,384 (14 bits) per ERF; after: no limit, but toolset (and nwsync) do not read past 2GB per hak — W Resource Limits https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits
- DM palette 1MB limit of 1.69 lifted in EE — same page
- Discord overlays (2025+) crash nwtoolset.exe — W https://nwn.wiki/spaces/NWN1/pages/38175555/Common+Errors+and+Their+Causes
- F9 "Test Module" causes timer/AI issues and module corruption risk — W https://nwn.wiki/spaces/NWN1/pages/38176945/Toolset+F9+Testing+Issues
- Content load order: user `development/` folder priority 71M (above NWSync 40M, haks 31M/30M, module 20M, override 12M); wiki row says toolset now loads development (version not known to the page author; changelog says 1.83.8193.21) — W https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order

---

## E. Format / field / event / resman / rendering / compiler changes by topic

### E0. Additions and corrections from the data pass (supersede the changelog-pass notes below where they conflict)
- **DLG param element layout resolved (strong evidence, not yet observed in data)**: `Key` Str + `Value` Str — the two labels follow directly after the DLG reader/writer label runs in nwtoolset.exe (offset 0x7abd63). Engine reads `ActionParams`, `ConditionParams` and also an undocumented `DisplayInactive` in DLG [S].
- **x-part fields**: exact labels (UTC) `xAppearance_Head`, `xArmorPart_RFoot`, `xBodyPart_{LFoot,RShin,LShin,LThigh,RThigh,Pelvis,Torso,Belt,Neck,RFArm,LFArm,RBicep,LBicep,RShoul,LShoul,RHand,LHand}`; (UTI) `xModelPart1..3`, `xArmorPart_{RFoot,LFoot,RShin,LShin,LThig,RThig,Pelvi,Torso,Belt,Neck,RFArm,LFArm,RBice,LBice,RShou,LShou,RHand,LHand,Robe}` — all WORD, each written right after its BYTE twin [D DoIWD]. Label = "x"+old label truncated to 16 chars.
- **TileBrdrDisabled** is written by the 89 toolset for every re-saved area (70/72 DoIWD areas; value 0) and is in the toolset's ARE writer run [T]; UI exposure undocumented.
- **Mod_CacheNSSList**: last seen in 1.80-era files (DDF 2020-04, XP1-Ch2, InfD, PotSC, WCoC, KM/SG/WW); absent from DoD (2021-07), ToM (2021-10), DoIWD — consistent with the 1.80.8193.14 removal.
- **Mod_UUID**: absent from every file before DoIWD (incl. ToM 2021-10); DoIWD writes it as an empty string.
- **Visual transforms**: 2020-2021 files use `VisualTransform` (single struct, id 6, ScaleX/Y/Z seen); DoIWD (2025) uses `VisTransformList` with `Scope`. The 89 toolset reads both labels [T].
- **Mod_MinGameVer in stock data**: 1.23/1.62 (old), 1.77/1.78 (campaign re-saves), 1.79 (KM/SG/WW), 1.80 (DDF, XP1-Ch2), 1.78 (DoD/ToM despite 2021 build dates — so these weren't stamped by a 1.8x toolset save, or the value is not always bumped), 1.89 (DoIWD).
- **GIT root `VarTable`** = area local variables (ARE has none) [D].
- **Object UUID label**: no `UUID`-like label exists in nwserver besides `Mod_UUID` [S] — how object UUIDs (v79 "persisted to GFF") are serialised remains open.
- **Engine-only runtime fields** (savegames / ObjectToJson / CopyArea; the toolset never writes them but must round-trip them if a user imports such a file): see B1, B2 and B3 lists.

### E-changelog pass (topics E1-E11; from the changelog sub-pass, [data] = its own spot checks)

Notation as in D. "[data]" = I checked stock game files read-only with `nwn_erf`/`nwn_gff` (neverwinter.nim, `~/.local/opt/neverwinter/bin`): mainly `data/nwm/Neverwinter Nights - Doom of Icewind Dale.nwm` (Mod_MinGameVer "1.89", i.e. saved with a 37-era toolset), compared with Chapter1.nwm (1.77), Tyrants of the Moonsea (1.78) and Dark Dreams of Furiae (1.80). These are observations, not changelog text.

### E1. Module / area / object event script slots
- [1.30, baseline] Module Properties "Cached Scripts" tab (server script caching) — NWNv169.txt "Patch details for v1.30". [data] GFF `Mod_CacheNSSList` (present in Chapter1 1.77 and DDoF 1.80 module.ifo; absent in DoIWD 1.89).
- [8155] (X) crash when the caching list has empty script names — W 1.74.8155.
- [1.80.8193.14 / 1.81.8193.15] (C) Toolset "UI for caching scripts has been removed … underlying system has been disabled for a while" — W .14/.15 "Toolset". => a new toolset need not edit Mod_CacheNSSList (may preserve it) [inf].
- [v74] (C) triggers and doors no longer run default.nss for events with no script set in the toolset — v74.txt "Other Issues".
- [1.75] (C) engine no longer assigns nw_g0_conversat to doors/placeables/creatures with no script; only a temporary default — v75.txt.
- [1.74.8164] (F) Get/SetEventScript: event handlers can be changed at runtime on any object incl. PCs — W 1.74.8164 / v74.txt.
- [1.80.8193.14 dev, 1.81.8193.15 stable] (F) module event **OnPlayerTarget** (EnterTargetingMode etc.); (K) toolset could not configure it; use SetEventScript at module load — W .14/.15 "Scripted mouse targeting mode".
- [1.82.8193.20] (C) EnterTargetingMode also fires on cancel (target OBJECT_INVALID) — W .20 "Other Updates".
- [1.83.8193.26] (X) OnPlayerTarget "now actually persisted to save games" — W .26.
- [1.85.8193.30] (F) module events **OnPlayerTileAction** and **OnPlayerGuiEvent**, both "via SetEventScript" only — W .30.
- [1.85.8193.31/85.8193.32] (F) Scriptable UI (NUI) with events; the module-level **OnNuiEvent** slot is not named in the .31/.32 notes (first named in 87.8193.35 toolset list) [open question when the slot itself appeared].
- [87.8193.35] (F) Toolset Module Properties exposes **OnPlayerTarget, OnPlayerGuiEvent, OnPlayerTileAction, OnNuiEvent** — CL 87.8193.35 "Added / Toolset". GFF labels are NOT given in the changelog; [data] module.ifo labels (all resref): `Mod_OnPlrTarget`, `Mod_OnPlrGuiEvt`, `Mod_OnPlrTileAct`, `Mod_OnNuiEvent`. Pre-existing 1.69 slots seen in both old and new data: Mod_OnAcquirItem, Mod_OnActvtItem, Mod_OnClientEntr, Mod_OnClientLeav, Mod_OnCutsnAbort, Mod_OnHeartbeat, Mod_OnModLoad, Mod_OnModStart, Mod_OnPlrChat, Mod_OnPlrDeath, Mod_OnPlrDying, Mod_OnPlrEqItm, Mod_OnPlrLvlUp, Mod_OnPlrRest, Mod_OnPlrUnEqItm, Mod_OnSpawnBtnDn, Mod_OnUnAqreItem, Mod_OnUsrDefined (OnPlayerChat is therefore pre-EE, not an EE addition).
- [88.8193.36] (F) NuiCreate/NuiCreateFromResRef get `sEventScript` (per-window event script; runtime, not a GFF slot) — CL 36 "VM functions".
- [88.8193.36] (F) Toolset Area Properties: load/save "script sets" (a UI feature; script sets for object types existed since 1.30) — CL 36 "Added". (X) Trigger instance scriptset changes not preserved — CL 36 "Toolset".
- [1.83.8193.21] (F) GetCurrentlyRunningEvent() returns EVENT_SCRIPT_* ids (confirms a per-object event id table) — W .21.
- No changelog adds new AREA or per-object (creature/door/placeable/trigger/encounter/store) event slots in EE. [data] GIT instance script fields in DoIWD match the 1.69 set (e.g. placeables: OnClick, OnClosed, OnDamaged, OnDeath, OnDisarm, OnHeartbeat, OnInvDisturbed, OnLock, OnMeleeAttacked, OnOpen, OnSpellCastAt, OnTrapTriggered, OnUnlock, OnUsed, OnUserDefined; doors add OnFailToOpen; triggers: OnClick, OnDisarm, OnTrapTriggered, ScriptHeartbeat, ScriptOnEnter, ScriptOnExit, ScriptUserDefine). 

### E2. New GFF fields / object properties
- module.ifo `Mod_DefaultBic` (resref; default character .bic shipped inside the .mod; launches immediately; "no GUI option yet") — CL 87.8193.35 "Added". [data] present (empty) in DoIWD.
- module.ifo `Mod_PartyControl` (INT: 0 server default, 1 enabled, 2 disabled; overrides server setting; "No GUI option yet") — CL 87.8193.35 "Added". [data] present (0) in DoIWD.
- module.ifo `Mod_UUID` — not named in any changelog; changelog only says toolset used to clobber a module UUID "manually embedded … with a GFF editor" (CL 88.8193.36 "Fixed / Toolset") and removes "spam about UUIDs missing for locally installed modules" (CL 37 "Removed"). [data] `Mod_UUID` cexostring (empty) in DoIWD. [inf: tied to the 1.83 game launcher / NWSync savegame-manifest lookup "of the same UUID" (W 1.83.8193.23)].
- module.ifo `Mod_MinGameVer` (cexostring) — pre-EE field; EE bumps: v74 (all saved modules require v74+; v74.txt), 1.79 (W 1.79.8192 "Module compatibility has been bumped to 1.79"), 1.80 (W 1.80.8193.9 Misc), 1.81 (W 1.81.8193.15 "Singleplayer modules made with this new patch 1.81 cannot be played on older versions"), 1.83 (wiki example: a module saved by the .21 preview toolset had 1.83; W How to Change the Module Version Id), 1.84 (W 1.84.8193.29), 1.85 (W 1.85.8193.30). No changelog statement for 1.86-1.89; [data] DoIWD = "1.89", DDoF = "1.80", TotM = "1.78", Chapter1 = "1.77".
- DLG conversation script parameters (toolset Conversation Editor; GetScriptParam/SetScriptParam) — W 1.80.8193.14 / 1.81.8193.15. Field labels not in changelog; [data] `ActionParams` (list) on EntryList/ReplyList node structs and `ConditionParams` (list) on link structs (RepliesList, EntriesList, StartingList). Element struct layout not observed (all empty in sample) [open question; likely Key/Value CExoStrings, inf].
- Visual transforms stored by the toolset (Adjust Location; creatures, items, doors, non-static placeables) — v79 / W 1.79.8192/8193. (C) reset when placeable Static set; (X) Z-only translation saved. [data] GIT `VisTransformList` (list; struct id 6) on Creature/Door/Placeable instances with members `Scope` (int) and structs `AnimationSpeed`, `RotateX/Y/Z`, `ScaleX/Y/Z`, `TranslateX/Y/Z`, each {`LerpType` int, `TimerType` int, `ValueTo` float}. Lerp support added 1.83.8193.21 (OBJECT_VISUAL_TRANSFORM_LERP_*; W .21).
- Object UUIDs "persisted to GFF" for Items, Creatures, Placeables, Triggers, Doors, Waypoints, Stores, Encounters, Areas — v79 "Further Features"/"New Script Commands". GFF label not given; [data] no UUID field in DoIWD GIT instances [inf: written only when a UUID was assigned at runtime (savegames/serialised objects)].
- BIC `DataMigration` field in CreatureStats struct (Dragon Disciple stat migration) — v79 "Content Creation".
- Door `TemplateResRef` now stored/loaded (#100) — W 1.80.8193.14 / 1.81.8193.15 "Fixes".
- Object hilite state persisted to GFF (label not given) — W .14/.15 "Fixes".
- Door/placeable `Bearing` save + sign-flip fixes — W .14/.15 "Toolset". X/YOrientation drift — W 1.83.8193.21.
- Body part / armor variation ids up to 999 (was 255) — CL 87.8193.35; toolset restriction lifted (CL 35 Toolset); toolset >255 armor parts fixed (CL 36). [data] creature GIT has WORD fields `xAppearance_Head`, `xBodyPart_*` (Belt, LBicep, LFArm, LFoot, LHand, LShin, LShoul, LThigh, Neck, Pelvis, RBicep, RFArm, RHand, RShin, RShoul, RThigh, Torso), `xArmorPart_RFoot` alongside the legacy byte fields; items have `xModelPart1` [inf: similar x-fields for armor ArmorPart_*].
- Area `FogClipDist` now stored in savegames — CL 85.8193.32 "Fixes". [data] ARE also has `TileBrdrDisabled` (byte) [inf: pairs with SetAreaTileBorderDisabled, CL 36; no changelog about a toolset control].
- SetShaderUniformVec 4th float clobbering 3rd "when saving as gff" — CL 36 and CL 37 "Fixed".
- Savegame/serialisation: Module SQLite DB saved to savegames and Player DB saved into the .bic (W 1.80.8193.14 SQLite; [data/wiki] RESTYPE_SQ3 in .sav, W Modules page); savegames store serialised .are and instanced areas use `nw_` resrefs (W 1.84.8193.29); AoE objects store all data to saves; polymorph saves pre-polymorph BIC state; custom perception ranges saved/loaded (CL 37 "Fixed").
- ITP palettes: `NAME` CExoString node allowed instead of STRREF (node formerly named `DELETE_ME`) — W 1.79.8192, v79, W 1.80.8193.6 ("custom tileset palettes").
- repository.json (NWSync/ContentIndex) fields: `localalias` (CL 86.8193.34.1), minimum required game version per module (CL 37 "Added"), release date in version entry (W 1.83.8193.26), website/web links (W 1.84.8193.29).

### E3. GFF / ERF / resource format changes
- ERF can hold more than 16,384 (14-bit) resources (modules and haks) since 1.80.8193.14 dev / 1.81.8193.15 stable — W .14/.15; W Resource Limits. Toolset (and nwsync) do not read haks past 2GB (wiki).
- `.caf` combined area format (ARE+GIT), RESTYPE_CAF = 2082; JSON serialisation compatible with neverwinter.nim — CL 85.8193.32.
- `.jui` NUI layout resource (RESTYPE_JUI id fixed in .32) — CL 85.8193.32 "Changes over build .31".
- `.lod` level-of-detail files (1.80.8193.6/.9); `.mtr` (1.74.8159/8160); KTX + plain DDS (1.78); `.bmu` may be plain mp3 (1.79); movies `.wbm` (toolset picks WBM not BIK, 1.74.8163); DDS BC4/BC5 (8193.14); `.sq3` in saves; `.ttf`, `.tml`, `.sql` (nwhak support list, 8193.12).
- nwhak.exe: understands new restypes incl. .mtr (1.75); adds ktx, ttf, sql, tml, sq3, lod (1.80.8193.12); moved util/ -> bin/ (8193.14).
- .mod may contain "any filetype except other container formats (HAK and ERF), other modules (MOD), save games (SAV) and KEY and BIF" — W Modules; CL 37: toolset now saves all valid file types into the .mod.
- Compression: NWCompressedBuf helpers in SQLite (1.83.8193.21); NWSync data always zstd (CL 37 "Removed"). No changelog mentions compressed ERF, ERF "V1.1", or 32-character resrefs. [tool evidence only] local `nwn_erf` offers `--data-version V1|E1` and `--data-compression none|zstd`; all stock .mod/.nwm files are "MOD V1.0" [data].
- Resref length: nothing in changelogs changes the 16-char resref limit; CL 87.8193.35 fixes the NWScript Debug UI for "16-character long script files" and CL 37 "Optimized ResRef handling" [inf: still 16 chars].
  Wiki GFF page (https://nwn.wiki/spaces/NWN1/pages/38175366/GFF) still documents CResRef as max 16 characters, null-padded; it also lists `.caf` as an EE-specific GFF type and notes the toolset writes an "ExportInfo.gff" into exported ERFs. (That page swaps the GIT/GIC descriptions; GIT = instances, GIC = comments.)
- Pre-EE encrypted premium module reading removed — CL 36 "Removed". Legacy 1.61: ERF header Strings section mirrors Module.ifo Description (NWNv169.txt 1.61).
- "Removed duplicate/conflicting SET, ITP, 2DA and TGA files" (WCoC) and "invalid resref '.res'" style issues exist in shipped data [data, corrected in the data pass: it is Tyrants of the Moonsea.nwm (entry 4042, empty resref) — Neverwinter Chess has none] — robustness note.
- Server crash on area load with bad GFF data fixed; BIC GFF validator (CL 36); GFF validation perf (W 8193.11).

### E4. Module folder mode (unpacked modules)
- [1.75] toolset opens unpacked modules from a directory named like the .mod without extension — v75.txt.
- [1.80.8193.12 dev / .13 stable] toolset setting "always open module directories" — W .12/.13.
- [87.8193.35] Build Module observes changes in temp0/ when compiling — CL 35 Toolset.
- Wiki workflow (rename temp0; prompt text; packs folder back into .mod on save) — W Open Module Folder.

### E5. Module UUID / object UUID
- [1.79] object UUID API + persistence (see E2) — v79.
- [1.80.8193.12] VM: script commands not accessing area UUIDs (#41) — W .12.
- [1.83.8193.23] savegame NWSync manifest fallback "of the same UUID" — W .23.
- [88.8193.36] toolset no longer clobbers a manually embedded module UUID — CL 36.
- [89.8193.37] removed log spam about UUIDs missing for locally installed modules — CL 37.

### E6. NWSync and related module fields (hak list, custom TLK, min version)
- [v74] module HAK and TLK loaded before MP chargen — v74.txt. [1.75] HAK/TLK preloading checked at connect — v75.txt.
- [1.78] NWSync introduced (Beamdog/nwsync tooling) — v78.txt.
- [1.79] NWSync management UI; manifests purged by group_id; offline modules (experimental); music/ambient through ResMan so they can be in hak/nwsync — v79.txt.
- [1.79.8193.5] hak vs module ResMan priority restored — W .5.
- [1.80.8193.6/.9] NWSync offline module repositories (`nwsync_write --with-module --group-id N`, modules.json); Storage Manager tabs — W .6/.9.
- [1.80.8193.10] offline NWSync modules: lower HAK priority (CURRENTGAME) — W .10.
- [1.80.8193.12] toolset removed the warning dialog when opening a module with haks — W .12. (1.67 baseline had nwtoolset.ini "Display Hak Warning=1".)
- [1.81.8193.17 / 1.82.8193.20] nwserver `-moduleurl` / `-modulehash` (repo written `--with-module`; `--no-latest`) — W .17/.20.
- [1.83.8193.21/.23] curated content repository + game launcher; NWSync downloads .wbm — W.
- [1.84.8193.29] repository CExoLocStr fields must be UTF-8 — W .29.
- [86.8193.34.1] `localalias` — CL.
- [87.8193.35] NWSync single file max 15MB -> 64MB — CL.
- [89.8193.37] ContentIndex modules may declare a minimum required game version; pal_XXXyy.tga loaded from haks/nwsync per module; nwsync always zstd — CL.
- No changelog adds a new module.ifo NWSync-hash field; [data] no such field in DoIWD module.ifo. `Mod_CustomTlk` and `Mod_HakList` are pre-EE [data].

### E7. development/ folder, override, ResMan load order (toolset-relevant)
- [1.79] user-dir `development/` at top of resource search path, hot-reload for uncached types; ResMan rewritten; override sits below ERF/haks — v79.txt.
- [1.79.8193.2] "development" alias works on dedicated servers — W.
- [1.79.8193.5] resman hak/module priority restored; option to log failed lookups — W.
- [1.80.8193.6/.9] "priorities of userpatch, modules, haks, and override have been restored"; ResMan cache > 256MB — W.
- [1.80.8193.11] toolset loads music/ambient sounds from haks if listed in the 2da — W.
- [1.80.8193.14] texture packs removed (merged into nwn_base.key); movies search user dir before install — W.
- [1.83.8193.21] toolset reads `DEVELOPMENT:`; movies from override/ERF/NWSync — W.
- [85.8193.32] ovr/ moved into keybif except scripts — CL. [89.8193.37] new nwn_retail.key with ovr/ contents; ovr/nwscript.nss only a human reference — CL.
- Priority constants (from engine source quoted on wiki): USER_DEVELOPMENT 71M, INSTALL_DEVELOPMENT 70M, RIM 60M, USER_MANIFEST 40M, USER_HAK 31M, INSTALL_HAK 30M, CURRENTGAME 23M, SAVEGAME 22M, SAVEGAME_MANIFEST 21M, MODULE 20M, USER_PATCH 13M, USER_OVERRIDE 12M, SOURCE_OVERRIDE 11M, INSTALL_OVERRIDE 10M, music/ambient 9-6M, INSTALL_PATCH 3M, TEXTUREPACK 2M, KEYTABLE 1M — W Content Load Order.

### E8. Rendering / MTR / PBR / textures relevant to toolset area & tile rendering
- [8159/8160] `.mtr` materials (customshaderVS/FS, texture0..14, parameter float/int), mesh `materialname`, vertex `colors`, `tverts1..3`; per-vertex static lighting with normal maps — W.
- [8163] water static lighting fixed; part combining with new streams — W. [8164] skybox centred on character — W.
- [1.75] `.mtr` `renderhint`; meshes need no texture with a material; mesh `bitmap`/`texture0` can name the material — v75.txt.
- [1.78] DDS without header mangling; KTX — v78.txt. [1.79] shader `#include` (verbatim replace) — v79.txt.
- [1.80.8193.6/.9] dungeon tile border illumination; mipmaps for uncompressed textures; `.lod` — W. [8193.7] renderhint `NormalTangents` — W.
- [1.80.8193.14 / 1.81.8193.15] PBR lighting engine, tone mapping, per-pixel lights, up to 32 dynamic lights; water reflects tile lights; grass sorting; VFX models fogged; BC4/BC5; toolset updates static lighting after tile light edits — W.
- [1.81.8193.17 / 1.82.8193.20] mesh ambient/diffuse default 1.0; normal maps two-channel; material filenames/params may contain underscore; toolset shadow and emitter rendering fixes — W.
- [1.83.8193.21/.23] material `parameter float DisplacementOffset`; unified PBR shaders (vs/fslit_nm/sm); TXI `decal` disables lighting; icy tileset materials; tile lights inserted into BSP before radius fix; bumpshinytexture regression; static lights update fix — W.
- [1.83.8193.26] early-Z: custom alpha shaders need TXI `alphamean` < 1.0; env map fallback/"default"; TILE_SOURCE_LIGHT_COLOR_BLACK really disables tile light — W.
- [1.85.8193.30] static placeables no longer break animations of non-static placeables sharing a model — W.
- [85.8193.32] tiledata bounding box for shadow clip; `rotatetexture 1` normal/displacement fix; debug rendering of bounding boxes/pivots/light ranges — CL.
- [87.8193.35] OpenGL 3.3; MTR `transparency`, `twosided`, `sample_framebuffer`, `volumetric`; dynamic area lighting; cubemap env maps from TXI; skybox fade blending; up to 3x more vertices per mesh; debug rendering of tile path nodes refined — CL.
- [88.8193.36] HDR bloom; model validation of normals/tangents on first load; PLT-on-GPU; part combining respects materialname — CL.
- [89.8193.37] bbox shader uniforms (bboxMin/bboxMax); static (unenhanced) lighting crash in some tilesets; no auto-merge of parts on model level; toolset renders non-human PLTs and fixed rotated parts (37-15) — CL.

### E9. New/changed 2DA columns and 2DAs the toolset reads (as far as changelogs say)
- racialtypes.2da `Icon` (1.75); name-generator tables (1.76); first-level skill/feat, point-buy, feat progression, `SkillPointModifierAbility` (1.79); `FavoredEnemtyFeat` [sic] (35).
- classes.2da `StatGainTable` (1.79); caster columns MemorizesSpells, SpellbookRestricted, PickDomains, PickSchool, LearnScroll, Arcane, ASF, SpellcastingAbil, SpellTableColumn, CLMultiplier, MinCastingLevel, MinAssociateLevel, CanCastSpontaneously (1.80.8193.6/.9) -> toolset custom caster support (85.8193.32); `SkipSpellSelection` (35).
- baseitems.2da weapon-feat columns WeaponFocusFeat .. WeaponOfChoiceFeat (8193.6/.9); `IsMonkWeapon`, `WeaponFinesseMinimumCreatureSize`, shield `BaseAC` used (8193.29).
- New 2DAs: ruleset.2da (1.79), iprp_visualfx.2da (8193.6), chargenclothes.2da (8193.7), progfx.2da + weathertypes.2da (8193.15), tileset edge 2das dag01_edge/tcm02_edge/trm02_edge/trs02_edge/ttf02_edge/tts02_edge (.16) and tss13_edge (.17/.20), encoding.2da (35), cachedmodels.2da (36), savingthrowtypes.2da (37).
- spells.2da `TargetShape`, `TargetSizeX`, `TargetSizeY`, `Flags`; skills.2da `HideFromLevelUp`; iprp_damagetypes.2da `VisualFX`; base-damage row in iprp_damagetype/damagehitvisual (35). damagetypes.2da `DamageRangedProjectile`; ammunitiontypes.2da `AmmunitionType`, `DamageRangedProjectile`; packages.2da > 255; vfx_persistent.2da > 255 (36). iprp_saveelement.2da new column; effecticons.2da > 255 (37).
- surfacemat.2da up to 64 materials, reloaded with CC (8193.30).
- Toolset-specific 2DA handling: music/ambient names without TLK (wiki Major Changes; 1.25 baseline `DisplayName`); toolset music from haks (8193.11); DoD/TotM music "Bad Strref" fix (.16); placeabletypes.2da and soundset.2da Label fallback (37); skyboxes/tailmodel/wingmodel first row "None" (37); toolset caches door 2das (37); appearance Label fallback when no StrRef (1.23 baseline); loadscreens.2da Beamdog rows have no StrRef and toolset uses Label (wiki loadscreens.2da page).
- Updated stock 2das list in 1.81.8193.16 summary (ambientmusic, ambientsound, appearance, doortypes, genericdoors, loadscreens, placeables, portraits, ruleset, skyboxes, soundset, tailmodel, visualeffects).

### E10. Tileset / area features
- Tile data fixes shipped in keybif: doors/transition meshes, Doorcap Interior feature tile, SET negative and scientific-notation values (8193.7/.10), DAG01.set height transitions, TTF01.set minimap typo (8193.14), TRM02.SET crosser, TCM02.SET names (8193.20) — W. => SET parser must tolerate odd numeric formats [inf].
- New tilesets: Ossian Medieval City/Rural, Mountain Snow, Lizardfolk Interior, Seaships (TSS13), facelifts TTF02/TTS02 (layout-compatible with originals, swap `Tileset` in .are) — W .14/.15; rock/chasm crosser (Medieval Rural 2), missing tiles (Medieval City 2) — CL 35.
- Tile path nodes 'q'..'x' (35).
- Tile/area runtime API: SetTileExplored etc (1.74), GetSurfaceMaterial/GetGroundHeight (8156), SetTile/SetTileJson/SetTileAnimationLoops/GetTileID/Orientation/Height, ReloadAreaGrass/Border (35), SetAreaGrassOverride/RemoveAreaGrassOverride/SetAreaDefaultGrassDisabled, SetAreaTileBorderDisabled, Get/SetAreaNoRestFlag (36), dynamic area light color/direction (35), SetAreaWind with toolset presets (8193.14), weathertypes.2da (8193.15), tile radial actions (8193.30).
- Modules no longer limited to 100 in-use tilesets (37).
- Toolset: tile light color picker fixes (1.77, 8193.13); static lighting refresh after tile light change (8193.14); second story tile fade "Always" (known issue .30, fixed .32); fog rendering with tabs (1.79); skybox list sorted (37).
- Area skybox / fog / wind: skybox centred on character (8164); "Icy, Clear sky" time-of-day skybox (8193.14); black skybox model (.17/.20); FogClipDist saved (32); wind direction offset fix (.15). No changelog adds new ARE fields for tile-level lighting/fog/skybox beyond these; per-area lighting is runtime-scriptable (35).

### E11. Script compiler / language changes (toolset compiler = game compiler)
- [1.74.8164] identifiers 8K -> 16K ("IDENTIFIER LIST FULL"); no crash on missing #include — W / v74.txt.
- [1.75] `\"` escapes; crash with many nested includes fixed — v75.txt.
- [1.76] `\\` escapes — v76.txt.
- [1.77] include limit 128 — v77.txt.
- [1.79] function-name prefix confusion ("Action"/"ActionTwo") fixed; ExecuteScriptChunk (JIT) — v79.txt.
- [1.80.8193.7] editor freeze compiling `/**/` — W.
- [1.80.8193.10/.13] access violation with nested structs — W.
- [1.80.8193.14] `sqlquery` type with SQLite API [inf: type name not stated in the note; SQLite doc referenced]; GetScriptParam/SetScriptParam — W.
- [1.83.8193.21/.23] `cassowary` engine type (GetLocalCassowary …) — W.
- [85.8193.32] `json` type; `\xFF` escapes (`\x00` terminates) — CL.
- [87.8193.35] string constants in `case` (HashString); identifiers 65536; string constants 8192 chars; 512 includes; built-in constants LOCATION_INVALID, JSON_FALSE/TRUE/OBJECT/ARRAY/STRING; CompileScript() — CL.
- [88.8193.36] raw strings `r"…"`/`R"…"` (multi-line); float literals `0f`, `.0`, `.42f`; `const` = any constant expression; non-integer for-loop init/increment; `?:` with structs fixed; compile-time evaluation; bytecode melding pass; compiler open-sourced (neverwinter.nim) — CL.
- [89.8193.37] unary constant folding; `__FUNCTION__`, `__FILE__`, `__LINE__`, `__DATE__`, `__TIME__`; hashed strings `h"…"`/`H"…"`; better error info; identifier name in Unidentified Identifier errors; for-loop optimisation; dead-branch elimination; stack overflow in folding fixed; instruction limit 1024k (VM) — CL.
- [89.8193.37-15] for-loop optimisation disabled (infinite-loop bug) — CL. [37-16] `0b`/`0o` literals, unary `+`, `!!x`; incorrect bytecode fix — CL.
- NOT found in any local changelog or wiki patch page: `#pragma`, vector arithmetic operators, `switch` on arbitrary strings beyond "string constants in case statements", struct/nested-include semantic changes other than the fixes above.
- Debug info: toolset option "Generate Debug Information When Compiling Scripts" writes .ndb (1.30 baseline; W Toolset Options); debugger revived 1.83.8193.23; ExecuteScriptChunk no longer writes `!chunk.ndb` to override (8193.21).


---

### D-baseline: 1.69 and older toolset items (pre-EE, NWNv169.txt; brief)
- 1.69: new modules get OnClientEnter = "x3_mod_def_enter"; toolset fixes for non-human cloaks, phenotype >= 10 textures, "Bad Strref" for padded classes.2da lines in creature/levelup wizards; horse dismount triggers on palette; nwtoolset.ini CPU affinity option — "Neverwinter Nights v1.69 / Aurora Toolset".
- 1.68: modules saved tagged as requiring 1.68 (same WARNING pattern for 1.67, 1.66, 1.65, 1.64, 1.62, 1.30, 1.28, 1.27, 1.25) [=> Mod_MinGameVer mechanism is old].
- 1.67: nwtoolset.ini `[Start Up] Display Hak Warning=1`; random facing button; new appearance/trident models; SavingThrowTable column crash.
- 1.66: Build -> "Test Module"; command-line compiler; switch-in-do/while stack underflow fix.
- 1.64: SetSkyBox; 1.62: custom TLK in MP.
- 1.61: custom palette refresh includes hak blueprints; ERF header Strings = module.ifo Description; faster ERF I/O; toolset language from dialog.tlk; year-2038 ERF date crash.
- 1.31: hak loading speed/progress; warnings when editing resources that exist in a hak or base game; Palette "Find Text".
- 1.30: script caching ("Cached Scripts" tab), script/spell set save-load, temp0-only temp dir handling, `utils\clcompile.exe`, "Generate Debug Information When Compiling Scripts".
- 1.28: multiple hak paks; palette reload; toolset refuses module with missing hak; nwhak merge/"Build Hak from directory".
- 1.27: Hak Pak editor restricts file names to 16 characters. 1.25: ambientmusic/ambientsound `DisplayName` column.
- 1.23: appearance Label fallback when no StrRef; Shift-Del in script editor.

---

## F. Citations

Local game files (read-only):
- `$G/lang/en/docs/CHANGELOG.md` (85.8193.32 … 89.8193.37-17); `$G/lang/en/docs/patchnotes/*.md` (same text); `$G/lang/en/docs/Neverwinter Nights Enhanced Edition (v74).txt` … `(v79).txt`; `$G/lang/en/docs/legacy/NWNv169.txt` (1.18-1.69 patch notes; field introduction versions); `$G/lang/en/docs/SQLite_README.txt`.
- Real data: `$G/data/nwm/*.nwm` (20), `$G/data/mod/*.mod` (8), base-game keys/bifs via nwn.py resman (palettes, standard blueprints), `$G/data/lcv/*.bic`, `$G/data/dmv/dungeonmaster.bic`. Ground truth = `$G/data/nwm/Neverwinter Nights - Doom of Icewind Dale.nwm`.
- Binaries (strings only): `$G/bin/win32/nwtoolset.exe` (build 26c6e57, 2025-10-06), `$G/bin/linux-x86/nwserver-linux`, `$G/bin/linux-x86/nwmain-linux`; `nwscript.nss` from resman (visual transform constants).
- Scratch artefacts: `scratchpad/gff/union.json` (every field path × type × source module), `analysis.txt`, `labelregions2.txt` (toolset label runs), `server_strings.txt`, `x/doiwd/` and `x/tym/` (unpacked DoIWD and ToM).
- Changelog sub-pass notes (unabridged): `scratchpad/research/notes_changelog_part.md`.

nwn.wiki (local mirror `~/.local/opt/neverwinter/wiki/pages/NWN1/`):
- https://nwn.wiki/spaces/NWN1/pages/38175366/GFF (note: swaps the GIT/GIC descriptions; says CResRef max 16 chars)
- https://nwn.wiki/spaces/NWN1/pages/327727/File+Format+Specification+Bioware (links the Bioware PDFs: 2DA, AreaFile (ARE/GIT/GIC), CommonGFFStructs, Conversation, Creature, DoorPlaceableGFF, Encounter, ERF, Faction, GFF, IFO, Item, Journal, KeyBIF, LocalizedStrings, PaletteITP, SoundObject, SSF, Store, TalkTable, Trigger, Waypoint — PDFs not mirrored locally)
- https://nwn.wiki/spaces/NWN1/pages/26738834/Toolset+Palette+ITP ; https://nwn.wiki/spaces/NWN1/pages/195362835/ITP+Palette+Format
- https://nwn.wiki/spaces/NWN1/pages/26738928/Major+Changes+in+NWN+EE ; https://nwn.wiki/spaces/NWN1/pages/26738699/Aurora+Toolset ; https://nwn.wiki/spaces/NWN1/pages/60982732/Toolset+User+Manual ; https://nwn.wiki/spaces/NWN1/pages/60982804/Toolset+Options
- https://nwn.wiki/spaces/NWN1/pages/38176075/Open+Module+Folder ; https://nwn.wiki/spaces/NWN1/pages/38175645/How+to+Change+the+Module+Version+Id ; https://nwn.wiki/spaces/NWN1/pages/155516929/Modules ; https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits ; https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order
- https://nwn.wiki/spaces/NWN1/pages/72417335/areag.ini ; https://nwn.wiki/spaces/NWN1/pages/60982751/Conversation+Editor ; https://nwn.wiki/spaces/NWN1/pages/38176021/Conversations
- Patch pages: index https://nwn.wiki/spaces/NWN1/pages/26738926/Patches+NWN+EE ; per-build URLs are given in each section-D release header (e.g. https://nwn.wiki/spaces/NWN1/pages/38174859/1.80.8193.14, https://nwn.wiki/spaces/NWN1/pages/91324440/1.87.8193.35, https://nwn.wiki/spaces/NWN1/pages/123797539/1.88.8193.36).

---

## G. Open questions

1. **Values > 255 in x-part fields**: what the toolset writes into the legacy BYTE twin (0? value & 0xFF? 255?) when an x-part is ≥ 256 — no stock example. Test by saving a creature/armor with part 300 in the 89 toolset.
2. **DLG params**: confirm `{Key, Value}` CExoString structs (and their struct ID) by saving one parameter in the toolset. Also what `DisplayInactive` (engine-read DLG field) does and where it sits (node or link).
3. **VisTransformList**: exact version that replaced `VisualTransform`; whether the toolset UI can set non-base scopes; when `ValueFrom`/`LerpDuration`/`LerpProgress` get written; default `TimerType`/`LerpType` values (stock data only shows the three basic members).
4. **Mod_UUID**: who generates it (launcher/NWSync/ContentIndex?) and whether the toolset ever fills it; only the "preserve existing value" rule is documented (88.8193.36).
5. **Mod_MinGameVer rule**: does the 89 toolset always stamp "1.89", or max(existing, own)? DoD/ToM (2021 builds) still say 1.78. Binary contains "1.89" and "1.22" constants next to the IFO writer.
6. **TileBrdrDisabled / Mod_DefaultBic / Mod_PartyControl UI**: changelogs say "no GUI option yet" (87). The 89 toolset both reads and writes them (labels in its IFO/ARE reader and writer runs [T]), so values set with a GFF editor survive a save, but its form resources contain no control names for them (searched: party/bic/border/uuid) — while the 4 new module events do have controls (`bBrowseOnPlayerTarget`, `bEditOnNuiEvent`, …) and the DLG editor has `bAddActionParameter`/`bAddConditionParameter` [T]. So: most likely no UI; confirm visually.
7. **Object UUIDs** (v79 "persisted to GFF"): label/serialisation unknown; no UUID label in nwserver except `Mod_UUID`.
8. **ITP `TYPE` values** (0/1/2 on skeleton palette nodes) exact semantics — Bioware PaletteITP PDF not available locally.
9. **ExportInfo.gff** structure (`Top`, `Dependencies`, `Missing` list layouts) — only label names known [T].
10. **Bioware 1.69 spec PDFs** are not mirrored locally; [K] meanings (Flags bits, LinkedToFlags, Times, SpawnOption, FactionRep direction, JRL Priority/Picture, store page IDs) should be re-checked against the PDFs (download needs user approval).
11. From the changelog pass (kept): exact builds/dates for 1.75-1.78; when the IFO slots for OnPlayerTarget/GuiEvent/TileAction/NuiEvent became GFF-persisted before the 87 UI; module description size change in 1.82.8193.20 (MP advert vs GFF); `sqlquery` type introduction date; wiki/CL date mismatches; missing wiki pages for 8193.18/.19/.22/.24/.25/.27/.28 and 37-15..17.
12. E1 ERF (zstd) support in game/toolset: only neverwinter.nim offers it; no changelog mention; all stock ERFs are V1.0. 16-char resref limit unchanged per all sources.
