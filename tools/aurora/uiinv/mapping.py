# Hand-authored semantics for the Aurora Toolset UI inventory generator (gen.py).
# MAP[form][control] = what the control edits (GFF field / 2DA / behavior target).

HEADER = r"""
# Aurora Toolset (nwtoolset.exe, NWN:EE) — UI Feature Inventory

Reimplementation checklist for **Moonglow Toolset**. Generated from the 105 VCL forms (DFM/TPF0 resources) embedded in
`nwtoolset.exe`, cross-referenced with the exe's per-form localisation tables, `dialog.tlk` (EE, English), base-game 2DAs
and sample GFFs. Every form is listed; every user-facing control of every form appears in a table row.

## How to read this document

- **Form heading**: `` `TClassName` — runtime window title ``. Meta line: DFM resource name, design size (client W×H),
  whether a localisation (StrRef) table was resolved, form-level event handlers.
- **Where**: tab › group box › (captioned panel or embedded frame). Wizard steps are hidden tab sheets (`ts…`).
- **Label / caption (runtime)**: the text the user actually sees. For captioned controls (buttons, checkboxes, radios,
  menu items) it is the control's own caption; for edits/combos it is the paired `TLabel` (paired by `FocusControl`,
  else by geometry, else across split "names"/"values" panels). When the runtime (TLK) text differs from the DFM design
  text, the DFM text is shown as `(DFM: '…')`.
- **Maps to**: the GFF field / 2DA / file the control edits (hand-mapped; resref fields are 16 chars, tags 32).
  `ARE`/`GIT`/`IFO`/`UTC`/`UTI`/`UTP`/`UTD`/`UTT`/`UTE`/`UTS`/`UTM`/`UTW`/`DLG`/`JRL`/`FAC`/`ITP` = file type.
- **Notes**: `hidden` (Visible=False at design time — usually dead or shown conditionally), `disabled`, `read-only`,
  `max N` (MaxLength), `spin a…b` (associated TUpDown range), `range a…b` (trackbar), combo `list` (drop-down list)
  vs `editable` (free text + list, used for all script-resref combos), `tip: “…”` (tooltip from TLK/Hint),
  and the VCL event handlers (`OnClick=…`), whose names describe behavior.
- Script-event rows fold their companion buttons: `buttons […] [Edit]` = "browse for script" (resource picker)
  and "open in Script Editor" (creates the script if it does not exist).

## Key findings

1. **Captions are localised from `dialog.tlk`, but not via Tag = StrRef.** Each form/frame has a static table in the
   exe's `.data` section: an array of 12-byte records `{int32 CaptionStrRef, int32 HintStrRef, int32 HelpStrRef}`
   indexed by the component's `Tag` (record 0 is usually `(0,0,0)`; `Tag = 1` usually marks "do not localise" — its
   record is `(-1,-1,-1)` — but some forms use 1 as a real index; `Tag = -1` = never localised; `-1` in a record
   column = keep DFM text / no tooltip). Tags are therefore small per-form indices (repeated Tags share one string,
   e.g. every `...` button on a form). Example (TdlgWaypointEdit, `.data+0x10D980`): Tag 2 → 7492 "Waypoint Properties",
   Tag 7 → (5197 "OK", 7205 "Accept changes", 7206 "Press this button to save all changes"), Tag 11 → 7435
   "Destination Tag" (DFM says "Linked To"), Tag 17 → 7494 "Waypoint Contains a Map Note".
   The toolset strings live mostly in StrRef ranges ~5000–8200 and ~61000–70000 and 83000+ of the base TLK.
   {LOCSTATS} (located by voting DFM caption ↔ TLK text over candidate table offsets, ambiguous ones verified by
   hand); the rest are tiny/generic dialogs (tables ambiguous) or have no Tags at all (item-generator dialogs,
   confirmation, comments, export picker — English-only, never localised).
   **Implication:** Moonglow can reuse `dialog.tlk` for all labels, tooltips (Hint) and status-bar help (Help).
2. The DFM English captions are design-time text; where TLK differs, the TLK text wins at runtime
   (e.g. "Linked To"→"Destination Tag", "Filters"→"Object Filters", "Palette"→"Palettes").
3. Many dialogs carry dead/hidden controls (Visible=False) from 2002-era features: world-map icon, secret door DC,
   "Party Required", item generators, "Save to Savegame", "Compute Static Lighting", module-start event,
   grammar checking. They are listed (marked **hidden**) so the checklist can explicitly decide to skip them.
4. EE-era additions visible in the forms: tabbed area/editor views (`pmEditorTab`, ini `Disable Tabs`), module
   events OnPlayerChat/Target/GuiEvent/TileAction/NuiEvent, module Tag, custom TLK + hak priority list + conflict
   analyser, area Fog Clip Distance + Sky Box + Shadow Opacity, Visual Transforms (scale/rotate/translate) in
   Adjust Location, 8 creature classes, creature Wings/Tail pickers, conversation script parameters
   (`ActionParams`/`ConditionParams`) and "Stop camera zoom in", item "Undroppable" (`Cursed`), placeable "Static",
   OnClick events for doors/placeables/triggers, external script editor, open-module-as-directory.
"""

TRAILER_PRE = r"""
## Appendix A — All keyboard shortcuts declared in the forms

VCL `ShortCut` decoding: low byte = virtual key, 0x2000 = Shift, 0x4000 = Ctrl, 0x8000 = Alt.
Hard-coded shortcuts handled in `OnKeyDown` code (not in DFMs) are listed in the "Hidden/implicit behaviors" section.
"""

TRAILER = r"""
## Hidden / implicit behaviors worth noting

Behaviors that are not (fully) described by DFM properties — inferred from handler names, exe strings, ini keys
(`nwtoolset.ini`), TLK strings and the nwn.wiki toolset manual. Each needs an explicit Moonglow decision.

**Module / file handling**
- Module is edited unpacked in a working directory (`modules/temp0/`, exe string `WORKTEMP`, `temp0`); Save repacks
  the `.mod`. Option *Create Backup Modules* writes `.BackupMod`; conversation auto-backup every N minutes (1–180) to
  temp0. EE can open a module *directory* instead of a `.mod` ("A directory with the same name has been found… open
  this directory as a module?"; option *Always open module directories*).
- MRU list of recent modules appended to the File menu at runtime (`MRUSeparator`).
- Open/Save use a custom module picker (`TdlgModuleSelect`, Normal vs Official Campaign) rather than the Windows
  dialog; `.nwm` official modules can be opened. Option *Show Retail* (ini) toggles listing official modules.
- **Test Module (F9)**: saves, then runs `nwmain.exe -userdirectory "<dir>" +TestNewModule "<module>"`
  (first local-vault character, spawns at start location); option *Minimize Toolset on test module*.
- *Verify module on save* runs the Build/Verify pass automatically. Custom palettes (`*palcus.itp`) can be rebuilt by
  Build › Palettes.
- Title bar: TLK format `BioWare Aurora Neverwinter Nights Toolset v%s.%s.%s-%s` (+ module name; DFM default
  `NeverWinter ToolSet - No Module`); `FormCloseQuery` prompts to save. Window/pane layout (pane widths,
  toolbar dock positions/visibility, script-editor geometry) persists to `nwtoolset.ini`.
- Warnings with "Never warn again" (`TdlgWarning`) are individually suppressible and re-enabled in Options › General:
  reserved blueprint resref namespace (`nw_`, `x0_`, `x2_`…), invalid creature spells, creature inventory,
  32-bit colour depth, character set, resource-in-hak, standard-resource overwrite.
- Duplicate tag / duplicate resref detection (TLK has "Duplicate tag detected", "Duplicate resref detected");
  `eTagChange`/`eResRefChange` handlers on every Tag/ResRef edit validate input while typing (resref: lower-case,
  16 chars, legal chars).

**Area editor (3D viewer, `TfrmViewerArea`)**
- Two selection modes: *Select Terrain* / *Select Objects* (F10 toggles). Terrain mode: click/drag rectangle selects
  tiles (blue); Ctrl+C/X/V copy/cut/paste tile blocks incl. lighting; Delete removes feature tiles;
  Shift+Right-click cycles tile variants/rotation; *Tile Properties* per tile (`TdlgTileProperties`).
- Terrain painting from the palette's Terrain tab: tileset *Features*, *Groups*, terrain types (corner-based,
  4 tiles at once), crossers, *Eraser*, *Raise/Lower* (tileset height). Tile-rule matching from `.set`
  (TERRAIN/CROSSER TYPES, PRIMARY/SECONDARY RULES, GROUPS). *Paint Start Location* sets `Mod_Entry_*`.
- Object placement: pick blueprint in palette, left-click to place (snaps to walkmesh), **Shift+click keeps the
  brush** for repeated placement (and stacks placeables); Right-click/Esc clears the brush. Triggers and encounters
  are drawn as polygons (click points, close polygon); *Redraw Polygon*; encounters get *Add Spawn Point*;
  creatures get *Create Waypoint* (walk waypoints `WP_<tag>_nn`); *Create Set* for waypoints.
- Object manipulation: left-drag moves (X/Y), **Alt+drag** moves Z, **Shift+Right-drag** rotates, **Ctrl+wheel**
  scales (VisualTransform), rubber-band multi-select; Delete removes; Ctrl+C/X/V copy/cut/paste groups with a ghost
  preview (left-click places, right-click cancels), relative positions kept; copy/paste works across open area tabs.
  Camera-control buttons also rotate the selected object (`bGobRotate*`) or randomise facing (`bGobRotateRandom`).
- Camera: wheel zoom (Shift/Ctrl+wheel = fine), Ctrl+Left-drag orbit/move, Ctrl+Right/Middle-drag pan, numpad
  2/4/6/8 move, 7/9/1/3 pan, 5 reset to top-down; optional *Visual Camera Controls* pane (repeat-while-pressed
  buttons: pan L/R/F/B, rotate CW/CCW, pitch up/down, zoom in/out). Areas open facing north.
- Undo/Redo (Ctrl+Z / Ctrl+Shift+Z) with configurable depth (Options › Area › Undo Levels 1–32, restart needed);
  only object/tile operations are undoable, not property-dialog edits.
- Render toggles: grid, shadows, fog, area lighting, *Fade Geometry* (never / object mode only / always — fades roofs
  and occluders near the camera), *Render AABB Nodes* (walkmesh/AABB debug), background colour, encounter spawn-point
  markers (height/width), door orientation arrows, 3D sound listener marker. Object filters (show/hide per type,
  Show All / Show None) affect rendering (and, per nwn.wiki, what can be selected/copied).
- Area audio preview: ambient sound, music, placed sounds (per-sound Mute/Turn On in context menu, saved as
  `PlayInToolset`), 2D/3D bias.
- Door/placeable *Initial State* context submenu animates and sets `AnimationState` (Open/Closed/Destroyed/
  Activated/Deactivated; door Opened Forward/Backward); *Reverse Door* flips a door 180°.
- Hidden developer console in the viewer (`ConsolePanel`/`CommandBox`/`ResponseBox`; the exe also carries engine
  console usage strings such as `savegamesnapshot`).
- *Area Statistics* (`TdlgSystemUsage`): model/texture memory counters (per-type breakdown group hidden).
- *Find Instance* (`TdlgFindInstance`): search all areas by object type(s), area, blueprint resref, tag;
  double-click result jumps camera to it.
- *Object Preview* window (`TfrmPreview`): non-modal; shows 3D/2D preview + read-only key properties of the
  selected palette/contents item; mouse-drag rotates the model.
- *Inaccessible Objects* (`TfrmObjectList`) lists objects that cannot be reached/picked; double-click focuses.
- Module-contents tree: double-click opens area / focuses object (opens area first); in-place rename (`tvMainEdited`);
  custom-drawn nodes; *Create Copy* clones areas/resources with a new resref; *Export Area* exports ARE+GIT+GIC and
  dependencies to ERF.

**Blueprints, palettes & instances**
- Standard palette (`*pal*.itp` from game data, read-only) vs Custom palette (`*palcus.itp` in module); categories come
  from ITP `STRREF`/`ID` trees; placing = instancing a blueprint into GIT; *Edit Copy* clones a standard blueprint into
  the custom palette; *Add to Palette* turns an instance back into a blueprint; *Update Instances* pushes blueprint
  changes to all instances; palette Find (Ctrl+F / F3) and *Refresh Palette*.
- Instance property dialogs are the same editor classes as blueprint editors, but show/hide controls:
  `bUpdateInstancesInArea` (blueprint mode), `ePaletteCategory` (blueprint only), ResRef read-only for instances.
- Multi-select + Properties on several situated objects opens `TdlgSituatedMultiEditor`: only common fields; blank =
  "leave unchanged".
- Variables (`TdlgVarTable`) edit the `VarTable` list (Name / Type / Value; type list filled at runtime) on module,
  areas, all blueprints and instances.
- Localised strings: the `...` next to every name/description opens `TdlgLocString` (per-language × gender grid,
  StrRef support via `TdlgNewEditStringExternal`); plain edit box edits the current editing language only
  (Options › Language).
- Script-set files: *Load/Save Script Set* on every Events tab reads/writes an `.ini` of event→script mappings.
- Creature editor recomputes derived values live (ability modifiers, AC, saves, HP, CR — exe references
  `fractionalcr.2da`, `cls_spgn_*`/`cls_spkn_*` spell tables, `cls_atk_*`), random names from `.ltr` name generators
  (`namefilter.2da` also referenced), *Apply Template* using `crtemplates.2da`.
- Item editor recomputes cost/weight/required level (`itemvalue.2da`, `skillvsitemcost.2da`), stack limits and
  property lists (`itemprops.2da`, `itempropdef.2da`, `iprp_*` tables) per base item.

**Editors**
- Conversation editor: node tree with NPC (red) / PC (blue) / link (grey) nodes, Ctrl+A add, Ctrl+C/X/V, Delete,
  drag to move/reorder, Ctrl+drag = link (configurable direction), *Paste As Link*, Cut → Scrap tab, bookmarks,
  search/replace (current / open / all files, language & gender scope), expand/collapse all, highlight filters,
  speaker portraits, token insertion, spell check, test-run (`TdlgConversationTest`), multiple open files as tabs,
  export to text (string vs character based). Debug menu items `-- REMOVE STRREF --` / `-- TEXT DUMP --` exist.
- Script editor: tabs per script, F7 compile (writes `.ncs`, optional `.ndb` debug), F2 code completion
  (`TSEditCodeCompletionList`), function/constant/variable/template side lists parsed from `nwscript.nss` + includes
  (F9 re-parse), help pane shows function doc comment, bookmarks (F5, Ctrl+Shift+1…9 / Ctrl+1…9), find-in-files,
  print, external-editor hand-off, syntax colours (TfraScriptEditorColor), templates directory.
- Journal editor: category/entry tree, Add/Copy/Cut/Paste/Delete, Apply without closing.
- Faction editor: basic (visual chart of checked factions in an OpenGL panel with mouse handlers) and advanced
  (N×N reputation grid 0–100), global effect flag, parent faction.
- Plot Wizard (plot manager `.ptm`, plot templates `.ptt`): generates conversations, scripts (`nw_i0_plotwizard`),
  journal entries, creatures/items/keys; Plot Manager tree in the palette pane (*Show Plots*).

**Global**
- Localisation: toolset UI language = TLK language (ini `[Display Options] Language=`); editing language separate.
- Help: `TfrmHelp` shows help text; the third column of every StrRef table is a per-control help string
  (e.g. "Press this button to save all changes") — usable for status-bar / context help.
- Spell checker (third-party engine, About box `lNameSpellchecker`) — broken in EE per nwn.wiki; candidate for omission.

## Complexity estimate

| Subsystem | Forms | Size | Why |
|---|---|---|---|
| Main frame, menus, toolbars, docking panes, MRU, ini persistence | TfrmFrame | M | Many actions but mostly plumbing |
| Module Contents tree + context actions | TfraMainInventory | M | Tree model over module resources, rename/clone/export |
| Area 3D editor (render, camera, picking, gizmo-less manipulation, filters, fade, AABB, audio preview, undo) | TfrmViewerArea, TfrmPreview | XL | Needs an Aurora MDL/tileset renderer, walkmesh picking, polygon tools, undo stack |
| Terrain painting (tileset rules, crossers, groups, raise/lower, tile props) | palette Terrain tab, TdlgTileProperties | XL | `.set` rule engine is the hardest non-rendering piece |
| Area properties / environment / wizard / resize-rotate / transitions / location | 7 dialogs | L | Many fields + transition linking UI with target-area minimap |
| Module properties, module wizard, hak/tlk, conflict analysis | TfrmIFOProp, TdlgModuleWizard, TdlgHakPak | M | Straightforward GFF + resman conflict report |
| Shared situated frames + multi-editor | 8 frames + TdlgSituatedEdit/MultiEditor | M | Reused by door/placeable; multi-edit merge semantics |
| Creature editor (+ wizard, level-up, colours, soundset, spells/feats/skills/special abilities, inventory) | 6+ forms | XL | Heavy 2DA-driven rules, CR/HP/save calc, 3D preview, spell lists |
| Item editor (+ property editor, wizard, armour parts, colours) | 4 forms | L | Property tables (itemprops/iprp_*), part/colour previews, cost calc |
| Placeable / door / trigger / encounter / sound / store / waypoint editors (+ wizards) | ~16 forms | L (together) | Individually S–M; sound editor timing/positioning and store restrictions are the larger ones |
| Inventory editor (creature/placeable/store) | TdlgInventory | M | Drag-drop grids, equip slots, store categories |
| Palette / blueprint selection / ITP handling | 6 forms | M | ITP read/write, custom palette mutation, find |
| Resource pickers (portrait, sound, soundset, load screen, resource open, colour) | 8 forms | M | Thumbnails (TGA/DDS/PLT decode), audio playback |
| Conversation editor (+ search, test, tokens, export) | 7 forms | L | Link semantics, scrap, multi-file tabs, params |
| Script editor (+ search, colours, completion, wizard) | 5 forms | L | Editor widget, compiler integration (nwnsc/neverwinter.nim), completion; wizard codegen M |
| Journal editor | 1 | S | |
| Faction editor | 2 | M | Chart visualisation optional |
| Plot wizard / plot manager | 4 | L | Large code/conversation generator; low user value — candidate to defer |
| Import/export (ERF), multi-select | 2 | M | Dependency discovery, overwrite resolution |
| Options | 1 | S | Mostly ini settings |
| Verify / build / test module | 1 | M | Compile all, CR recompute, palette rebuild, missing/unused resource scans |
| Misc (about, welcome, progress, warnings, confirm, help, loc-string, var table, comments) | ~12 | S | |
"""

LOC_OVERRIDE = {   # manually verified with tools/loccheck.py
    'TDLGDOORWIZARD': 183812, 'TDLGCONVERSATIONEDITOR': 32608, 'TFRAMAINPALETTE': 329784, 'TDLGRESOURCESELECTION': 915948,
    'TFRAENCOUNTERCREATURELIST': 185292, 'TFRMTRAP': 994208, 'TFRASITUATEDTRAP': 994208, 'TDLGWIZARD': 17144,
    'TDLGWAYPOINTWIZARD': 1109496, 'TFRMHELP': 569216, 'TDLGPROGRESS': 896036, 'TFRASCRIPTEDITORCOLOR': 943460,
}

NOLOC_NOTE = {
    'TDLGITEMGENERATOREDIT': 'no Tags — never localised (dead feature)',
    'TDLGGENERATORCHOOSER': 'no Tags — never localised (dead feature)',
    'TDLGCOMMENTS': 'no Tags — English only',
    'TDLGCONFIRMATION': 'no Tags — English only; button captions set at runtime',
    'TDLGCONVERSATIONEXPORTPICKER': 'no Tags — English only',
}

PARENT = {
    'TDLGAREAWIZARD': '`TdlgWizard` (Back/Next/Finish/Cancel/Help buttons + hidden-tab page control `pcSteps`).',
    'TDLGBLUEPRINTWIZARD': '`TdlgWizard`. Base for Door/Encounter/Placeable/Sound/Store/Trigger/Waypoint wizards: palette-category step + name step.',
    'TDLGDOORWIZARD': '`TdlgBlueprintWizard` (palette category → name).',
    'TDLGENCOUNTERWIZARD': '`TdlgBlueprintWizard`.',
    'TDLGPLACEABLEWIZARD': '`TdlgBlueprintWizard`.',
    'TDLGSOUNDWIZARD': '`TdlgBlueprintWizard`.',
    'TDLGSTOREWIZARD': '`TdlgBlueprintWizard`.',
    'TDLGTRIGGERWIZARD': '`TdlgBlueprintWizard`.',
    'TDLGWAYPOINTWIZARD': '`TdlgBlueprintWizard`.',
    'TDLGITEMWIZARD': '`TdlgWizard`.',
    'TDLGMODULEWIZARD': '`TdlgWizard`.',
    'TDLGPLOTNODEWIZARD': '`TdlgWizard`.',
    'TDLGSTORESETUPWIZARD': '`TdlgWizard`.',
    'TDLGPLACEABLEEDIT': '`TdlgSituatedEdit` — all tabs/frames of the base dialog (Basic/Lock/Trap/Events/Advanced/Description/Comments) plus the extras below.',
    'TDLGDOOREDIT': '`TdlgSituatedEdit` — base tabs/frames plus Area Transition tab and generic-door appearance.',
    'TDLGRESOPENSOUND': '`TdlgResOpen` — adds Play/Stop for WAV preview.',
    'TFRMVIEWERAREA': '`TfrmViewer` (open/save dialogs, `FormCloseQuery`). Hosted as a tab inside the main frame `pArea`.',
    'TFRMIFOPROP': '`TfrmProperties` (OK/Cancel).',
    'TDLGAREAPROPERTIES': '`TfrmProperties` (OK/Cancel).',
}

# ---------------------------------------------------------------- purposes
PURPOSE = {
 'TFRMFRAME': 'Main application window: menu bar, dockable toolbars (File, Display, Selection Mode, Object Filters, Preview) in a `TControlBar`, left pane = Module Contents (`TfraMainInventory`), centre = tabbed area viewers (`pArea`), right pane = palettes + plot manager (`TfraMainPalette`), bottom = message log + status bar.',
 'TFRAMAININVENTORY': 'Module Contents tree (left pane): Areas (with per-area object lists), Conversations, Scripts, and other module resources. Context menu drives most module-level resource operations.',
 'TFRMVIEWER': 'Base class of document windows (open/save dialogs, close query).',
 'TFRMVIEWERAREA': '3D area editor (one per open area tab): OpenGL viewport, optional visual camera-control pad, per-instance context menu, and an area-specific Edit/Scene menu merged into the main menu (GroupIndex 20/30).',
 'TFRMPREVIEW': 'Non-modal Object Preview window: 3D (or 2D icon) render of the selected blueprint/instance plus read-only summary fields per object type.',
 'TFRMOBJECTLIST': '"Inaccessable Objects" list: objects that cannot be picked in the viewer; double-click focuses.',
 'TFRMPROPERTIES': 'Base class for property dialogs (OK/Cancel).',
 'TFRMIFOPROP': 'Module Properties (module.ifo).',
 'TDLGMODULEWIZARD': 'Module creation wizard: name → create areas (launches Area Wizard repeatedly) → finish.',
 'TDLGAREAPROPERTIES': 'Area Properties (ARE + GIT `AreaProperties`).',
 'TDLGENVIRONMENT': 'Customize Environment (area lighting/fog/weather/skybox), opened from Area Properties › Visual.',
 'TDLGAREAWIZARD': 'Area Wizard: name + tileset → size → finish.',
 'TFRMAREARESIZE': 'Resize Area (rows/columns, preset list) and Rotate Area (90/180/270 CW/CCW) — same form, panel switched.',
 'TDLGTILEPROPERTIES': 'Per-tile lighting/animation properties (ARE `Tile_List` entry).',
 'TDLGAREATRANSITION': 'Setup Area Transition: pick target area and target door/trigger/waypoint on a minimap; links LinkedTo/LinkedToFlags on one or both ends.',
 'TDLGLOCATION': 'Adjust Location: exact position, bearing (with drag dial) and EE visual transform of the selected instance(s).',
 'TDLGADDPOPUPTEXT': 'Add Popup Text: creates a one-line conversation (resref given) and assigns it to the selected object (e.g. placeable examine-speak).',
 'TDLGSYSTEMUSAGE': 'Area Statistics: model/texture memory usage of the current area.',
 'TDLGFINDINSTANCE': 'Find Instance: search placed objects across the module.',
 'TDLGSITUATEDEDIT': 'Base properties dialog for situated objects (doors, placeables) composed of shared frames; also used directly.',
 'TDLGSITUATEDMULTIEDITOR': 'Multi-object editor for several selected situated objects (common subset of fields).',
 'TFRASITUATEDBASIC': 'Shared frame: name, tag, appearance, HP, hardness, saves, plot.',
 'TFRASITUATEDADVANCED': 'Shared frame: faction, template resref, conversation, portrait, interruptable, open state, variables.',
 'TFRASITUATEDLOCK': 'Shared frame: lock settings.',
 'TFRASITUATEDTRAP': 'Shared trap frame (variant; `TfrmTrap` is the one actually hosted in `pTrap`).',
 'TFRMTRAP': 'Trap settings frame hosted in the Trap tab of doors, placeables and triggers.',
 'TFRASITUATEDSCRIPTS': 'Shared frame: event scripts for doors/placeables (panels hidden per object type).',
 'TFRASITUATEDDESC': 'Shared frame: localised description.',
 'TFRASITUATEDCOMMENTS': 'Shared frame: designer comment.',
 'TDLGCREATUREEDIT': 'Creature Properties (UTC blueprint or GIT creature instance), with live 3D preview.',
 'TDLGCREATUREWIZARD': 'Creature Wizard: start → racial type → appearance/portrait/gender → class & level → faction → name → palette category → review → finish.',
 'TFRMCREATURELEVELUPWIZARD': 'Levelup Wizard: add classes/levels to an existing creature (auto-picks feats/skills/spells via packages).',
 'TCOLORPICKER': 'Character/armour colour picker: palette image (PLT colour ramps) with a layer selector.',
 'TDLGINVENTORY': 'Inventory editor for creatures (equipment slots + backpack), placeables (contents) and stores (categorised stock).',
 'TDLGITEMEDIT': 'Item Properties (UTI blueprint or instance), with 3D/2D preview.',
 'TDLGPROPEDIT': 'Item property parameter editor ("Select Property Parameters").',
 'TDLGITEMWIZARD': 'Item Wizard: base item type → name/quality → palette category → finish.',
 'TDLGITEMGENERATOREDIT': 'Random item generator editor (unlocalised, unreachable in retail UI).',
 'TDLGGENERATORCHOOSER': 'Random item generator chooser (unlocalised, unreachable; `mbRandom` button hidden in inventory).',
 'TDLGPLACEABLEEDIT': 'Placeable Object Properties (UTP / GIT placeable).',
 'TDLGPLACEABLEWIZARD': 'Placeable Wizard.',
 'TDLGDOOREDIT': 'Door Properties (UTD / GIT door).',
 'TDLGDOORWIZARD': 'Door Wizard: palette category → name → (appearance list, strength — pages defined but not localised).',
 'TDLGTRIGGEREDIT': 'Trigger Properties (UTT / GIT trigger). Uses a split name/value property-sheet layout.',
 'TDLGTRIGGERWIZARD': 'Trigger Wizard: palette category → trigger type → name.',
 'TDLGENCOUNTEREDIT': 'Encounter Properties (UTE / GIT encounter).',
 'TFRAENCOUNTERCREATURELIST': 'Encounter creature list frame (palette trees ↔ CreatureList grid).',
 'TDLGENCOUNTERWIZARD': 'Encounter Wizard: palette category → creature list → name.',
 'TDLGSOUNDEDIT': 'Sound Properties (UTS / GIT sound).',
 'TDLGSOUNDWIZARD': 'Sound Wizard: palette category → timing → positioning → wave list → name (defaults from sounddefaultspos/stim.2da).',
 'TDLGSTOREEDIT': 'Store (merchant) Properties (UTM / GIT store).',
 'TDLGSTOREWIZARD': 'Merchant Wizard.',
 'TDLGSTORESETUPWIZARD': 'Store Setup Wizard (context menu on a creature): writes shopkeeper conversation + open-store script and links a store.',
 'TDLGWAYPOINTEDIT': 'Waypoint Properties (UTW / GIT waypoint).',
 'TDLGWAYPOINTWIZARD': 'Waypoint Wizard.',
 'TDLGWIZARD': 'Generic wizard base.',
 'TDLGCONVERSATIONEDITOR': 'Conversation Editor (DLG), multi-file (one tab per open conversation + Scrap tab).',
 'TFRACONVERSATIONTREE': 'Conversation tree frame (one per open DLG tab).',
 'TDLGCONVERSATIONINPUT': 'Popup to type new node text (Options: "Show popup when creating a new text entry").',
 'TDLGCONVERSATIONSEARCH': 'Find / Replace in conversations.',
 'TDLGCONVERSATIONTEST': 'Conversation test-run window (click-through of the tree).',
 'TDLGCONVERSATIONEXPORTPICKER': 'Export mode chooser for conversation text export.',
 'TDLGTOKENSELECTOR': 'Insert Token (custom tokens from stringtokens.2da, highlight colour tokens).',
 'TDLGSCRIPTEDITOR': 'Script Editor (NSS), multi-tab, with compiler output/help/bookmarks/search panes.',
 'TDLGSCRIPTSEARCH': 'Script Find / Replace / Find in Files.',
 'TFRASCRIPTEDITORCOLOR': 'Syntax colour options (embedded in Options › Script Editor).',
 'TSEDITCODECOMPLETIONLIST': 'Code-completion popup of the script editor (virtual list).',
 'TDLGSCRIPTWIZARD': 'Script Wizard: generates conversation condition or action scripts from checklists.',
 'TDLGJOURNALEDITOR': 'Journal Editor (module.jrl).',
 'TDLGFACTIONEDITOR': 'Faction Editor (repute.fac).',
 'TDLGFACTIONSELECT': 'Add Faction (name, global, parent).',
 'TDLGPLOTWIZARD': 'Plot Wizard: plot name/template → cast (giver, villains, extras) → props → plot nodes → summary; saves a plot manager file.',
 'TDLGPLOTNODEWIZARD': 'Plot Node Wizard (one sub-plot step).',
 'TFRAPLOTMANAGER': 'Plot Manager tree (bottom of palette pane).',
 'TFRAPROGRESS': 'Wizard progress strip (step list) used by plot wizards.',
 'TFRAMAINPALETTE': 'Right pane: blueprint palettes (Standard/Custom × 9 object types, Terrain), paint-mode toolbar, find bar, plot manager.',
 'TFRABLUEPRINTSELECT': 'Embeddable blueprint picker (Standard/Custom trees + New/Edit Copy) used by wizards (Store Setup, Plot).',
 'TDLGPALETTECHOOSER': '"Select Category": choose palette category (ITP node) for a blueprint.',
 'TDLGRESOURCESELECTION': '"Select Resources": pick blueprints of any type (used by Export and Plot wizard).',
 'TDLGCHOOSER': 'Generic "Select Type" tree chooser (e.g. item base type in Item Wizard / script-editor templates).',
 'TDLGDEFAULTSELECTOR': 'Generic "Select Pattern" list chooser.',
 'TDLGPORTRAIT': 'Select Portrait (portraits.2da thumbnails with filters).',
 'TDLGRESOPEN': 'Open Resource: generic resource browser (type, name filter, location filter); used for scripts, conversations, sounds, etc.',
 'TDLGRESOPENSOUND': 'Open Resource for WAV sounds with preview playback.',
 'TDLGSOUNDSETSELECT': 'Sound Set Select (soundset.2da filtered by gender/type; click previews).',
 'TDLGLOADSCREEN': 'Select Load Screen (loadscreens.2da thumbnails, tileset filter, random).',
 'TDLGRESTYPESELECTOR': 'Resource Type Selection (radio buttons built at runtime).',
 'TDLGCOLORSELECTION': '"Select A Color": fixed palette of 32 colour swatches (tile light colours, lightcolor.2da).',
 'TDLGLOCSTRING': 'String Edit: CExoLocString editor (languages × genders grid).',
 'TDLGNEWEDITSTRINGEXTERNAL': 'Edit String (one language/gender entry or a TLK StrRef).',
 'TDLGVARTABLE': 'Scripting Variables (VarTable) editor.',
 'TDLGCOMMENTS': 'Edit Comments (plain memo).',
 'TDLGIMPORTEXPORT': 'Import / Export resources (ERF): export list, missing-dependency confirmation, overwrite resolution, comments.',
 'TDLGMULTISELECT': 'Generic multi-select list/grid dialog (instructions set by caller).',
 'TDLGHAKPAK': 'Hak Pak Conflict Analysis (Module Properties › Custom Content › Check for Conflicts).',
 'TDLGOPTIONS': 'Toolset Options (tree of pages; values persisted in nwtoolset.ini).',
 'TDLGVERIFYMODULE': 'Build Module / Verify (Build menu): compile, CR/encounter/palette rebuild, missing/unused resource reports, spell check.',
 'TDLGWARNING': 'Suppressible warning box (Yes/No or OK, "Never warn again").',
 'TDLGCONFIRMATION': 'Yes / Yes to All / No / No to All / Cancel confirmation with "Remember this Response".',
 'TDLGPROGRESS': 'Two-bar progress dialog (task + total), optional Cancel.',
 'TDLGABOUT': 'About box (versions, key file string, licence, expansions installed).',
 'TFRMHELP': 'Help popup (TLK help text).',
 'TDLGWELCOME': 'Startup welcome dialog: create new module / open existing (list) / start normally.',
 'TDLGMODULESELECT': 'Open / Save module picker (modules folder or official campaign).',
}

# ---------------------------------------------------------------- field mappings
SCRIPT = 'resref (script)'
MAP = {
 'TFRMFRAME': {
   'mMessages': 'message log (read-only)', 'sbFrame': 'status bar (2 panels)',
   'sbDrawCreatures': 'filter: creatures', 'sbDrawDoors': 'filter: doors', 'sbDrawEncounters': 'filter: encounters',
   'sbDrawItems': 'filter: items', 'sbDrawStores': 'filter: stores', 'sbDrawPlaceables': 'filter: placeables',
   'sbDrawSounds': 'filter: sounds', 'sbDrawTriggers': 'filter: triggers', 'sbDrawWaypoints': 'filter: waypoints',
   'sbDrawStartLocation': 'filter: start location', 'tbDrawAll': 'all filters on', 'tbDrawNone': 'all filters off',
   'sbFindStart': 'camera → start location', 'sbRecenter': 'reset camera', 'sbModeTerrain': 'selection mode',
   'sbModeObject': 'selection mode', 'tbDisplayShadows': 'render option', 'tbFog': 'render option',
   'tbUseAreaLighting': 'render option', 'dlgOpen': '.mod', 'dlgSaveAs': '.mod', 'dlgSaveExport': '.erf export',
   'dlgOpenImport': '.erf import', 'sdSaveToSavegame': '.sav (hidden feature)',
 },
 'TFRAMAININVENTORY': {'tvMain': 'module resources / area objects'},
 'TFRMVIEWERAREA': {'GLPanel': 'area 3D viewport', 'CommandBox': 'debug console input', 'bGobRotateRandom': 'instance Bearing (random)'},
 'TFRMPREVIEW': {
   'apPreview': 'model preview', 'eChallengeRating': 'UTC `ChallengeRating`', 'eCreatureFaction': '`FactionID` name',
   'eDoorTrapType': '`TrapType` (traps.2da)', 'eDoorFaction': '`Faction`', 'eDoorDestinationTag': '`LinkedTo`',
   'xbDoorLocked': '`Locked`', 'eDifficulty': 'UTE `DifficultyIndex`', 'eSpawnOption': 'UTE `SpawnOption`',
   'eEncounterFaction': '`Faction`', 'eItemCost': 'UTI cost', 'rb2D': 'preview mode', 'rb3D': 'preview mode',
   'ePlaceableFaction': '`Faction`', 'ePlaceableTrapType': '`TrapType`', 'xbPlaceableLocked': '`Locked`',
   'eVolume': 'UTS `Volume`', 'xbActive': 'UTS `Active`', 'eTriggerType': 'UTT `Type`', 'eTriggerDestinationTag': '`LinkedTo`',
   'eTriggerFaction': '`Faction`', 'eTriggerTrapType': '`TrapType`', 'eTag': '`Tag`', 'eName': 'name',
   'mComments': '`Comment`', 'eResRef': '`TemplateResRef`',
 },
 'TFRMOBJECTLIST': {'tvObjectList': 'unreachable instances'},
 'TFRMIFOPROP': {
   'eName': 'IFO `Mod_Name` (CExoLocString)', 'bName': '`Mod_Name` all languages (TdlgLocString)', 'eTag': 'IFO `Mod_Tag`',
   'cbStartArea': 'IFO `Mod_Entry_Area` (read-only; set by painting Start Location)',
   'eStartPosX': 'IFO `Mod_Entry_X`', 'eStartPosY': 'IFO `Mod_Entry_Y`', 'eStartPosZ': 'IFO `Mod_Entry_Z`',
   'cbOnClientEnter': 'IFO `Mod_OnClientEntr`', 'cbOnModuleLoad': 'IFO `Mod_OnModLoad`', 'cbOnHeartbeat': 'IFO `Mod_OnHeartbeat`',
   'cbOnUserDefined': 'IFO `Mod_OnUsrDefined`', 'cbOnClientLeave': 'IFO `Mod_OnClientLeav`', 'cbOnActivateItem': 'IFO `Mod_OnActvtItem`',
   'cbOnAcquireItem': 'IFO `Mod_OnAcquirItem`', 'cbOnModuleStart': 'IFO `Mod_OnModStart`', 'cbOnUnAquireItem': 'IFO `Mod_OnUnAqreItem`',
   'cbOnPlayerDeath': 'IFO `Mod_OnPlrDeath`', 'cbOnPlayerDying': 'IFO `Mod_OnPlrDying`', 'cbOnPlayerRespawn': 'IFO `Mod_OnSpawnBtnDn`',
   'cbOnPlayerRest': 'IFO `Mod_OnPlrRest`', 'cbOnPlayerLevelUp': 'IFO `Mod_OnPlrLvlUp`', 'cbOnCutsceneAbort': 'IFO `Mod_OnCutsnAbort`',
   'cbOnPlayerEquipItem': 'IFO `Mod_OnPlrEqItm`', 'cbOnPlayerUnEquipItem': 'IFO `Mod_OnPlrUnEqItm`', 'cbOnPlayerChat': 'IFO `Mod_OnPlrChat`',
   'cbOnNuiEvent': 'IFO `Mod_OnNuiEvent`', 'cbOnPlayerGuiEvent': 'IFO `Mod_OnPlrGuiEvt`', 'cbOnPlayerTarget': 'IFO `Mod_OnPlrTarget`',
   'cbOnPlayerTileAction': 'IFO `Mod_OnPlrTileAct`',
   'eStartMonth': 'IFO `Mod_StartMonth`', 'eStartDay': 'IFO `Mod_StartDay`', 'eStartHour': 'IFO `Mod_StartHour`',
   'eTemplateName': '(dead)', 'eMinPerHour': 'IFO `Mod_MinPerHour`', 'eDawnHour': 'IFO `Mod_DawnHour`', 'eDuskHour': 'IFO `Mod_DuskHour`',
   'tbXPScale': 'IFO `Mod_XPScale`', 'eXPScale': 'IFO `Mod_XPScale` (numeric mirror)', 'cbMovieStart': 'IFO `Mod_StartMovie` (movies/*.bik)',
   'bBrowseMovieStart': 'browse movie', 'eStartYear': 'IFO `Mod_StartYear`', 'bVariablesEdit': 'IFO `VarTable`',
   'mDescription': 'IFO `Mod_Description`', 'bDescription': 'all languages',
   'lbHakFiles': 'IFO `Mod_HakList[].Mod_Hak` (ordered, top = highest priority; legacy `Mod_Hak`)',
   'cbHakFile': 'choose .hak from hak dir(s)', 'bHakAdd': 'append to hak list', 'bHakRemove': 'remove selected hak',
   'bHakMoveUp': 'reorder', 'bHakMoveDown': 'reorder', 'bHakConflicts': 'opens TdlgHakPak', 'cbCustomTlkFile': 'IFO `Mod_CustomTlk` (tlk dir)',
 },
 'TDLGMODULEWIZARD': {'eModuleName': 'IFO `Mod_Name` + module filename', 'bModuleName': 'all languages', 'lbAreas': 'areas created so far', 'bNewArea': 'runs Area Wizard'},
 'TDLGAREAPROPERTIES': {
   'eName': 'ARE `Name`', 'bAreaName': 'ARE `Name` all languages', 'cbTileSet': 'ARE `Tileset` (*.set; editable only on creation)',
   'eLength': 'ARE `Height` (tiles; shown only when creating)', 'eWidth': 'ARE `Width` (tiles)',
   'bCustomize': 'opens TdlgEnvironment', 'mmWarning': 'info text', 'lvLightingSchemes': 'ARE `LightingScheme` (environment.2da presets; applies sun/moon/fog colours)',
   'cbAmbientSoundDay': 'GIT `AreaProperties.AmbientSndDay` (ambientsound.2da)', 'cbAmbientSoundNight': 'GIT `AreaProperties.AmbientSndNight` (ambientsound.2da)',
   'cbMusicBattle': 'GIT `AreaProperties.MusicBattle` (ambientmusic.2da)', 'cbMusicDay': 'GIT `AreaProperties.MusicDay` (ambientmusic.2da)',
   'cbMusicNight': 'GIT `AreaProperties.MusicNight` (ambientmusic.2da)', 'eMusicDelay': 'GIT `AreaProperties.MusicDelay`',
   'cbEnvironmentalAudio': 'GIT `AreaProperties.EnvAudio` (soundeax.2da)', 'tbAmbientSoundDayVolume': 'GIT `AreaProperties.AmbientSndDayVol`',
   'tbAmbientSoundNightVolume': 'GIT `AreaProperties.AmbientSndNitVol`',
   'cbOnEnter': 'ARE `OnEnter`', 'cbOnExit': 'ARE `OnExit`', 'cbOnHeartbeat': 'ARE `OnHeartbeat`', 'cbOnUserDefined': 'ARE `OnUserDefined`',
   'bLoadScriptSet': 'read script set (.ini)', 'bSaveScriptSet': 'write script set (.ini)',
   'eWorldMapIcon': '(dead)', 'bWorldMapIcon': '(dead)', 'eWorldX': '(dead)', 'eWorldY': '(dead)', 'eTime': '(dead)',
   'eTag': 'ARE `Tag`', 'eModifierListenCheck': 'ARE `ModListenCheck`', 'eModifierSpotCheck': 'ARE `ModSpotCheck`',
   'eResRef': 'ARE `ResRef` (renames area files)', 'xbNoRest': 'ARE `NoRest`', 'cbPlayerVsPlayer': 'ARE `PlayerVsPlayer` (pvpsettings.2da)',
   'rbNotInterior': 'ARE `Flags` bit 0x1 = 0', 'rbInterior': 'ARE `Flags` bit 0x1 (interior)', 'rbNatural': 'ARE `Flags` bit 0x4 (natural)',
   'rbNotNatural': 'ARE `Flags` bit 0x4 = 0', 'rbSubterranean': 'ARE `Flags` bit 0x2 (underground)', 'rbNotSubterranean': 'ARE `Flags` bit 0x2 = 0',
   'eLoadScreen': 'ARE `LoadScreenID` (loadscreens.2da)', 'bBrowseAreaTransitionBitmap': 'opens TdlgLoadScreen', 'bVariablesEdit': 'ARE `VarTable`',
   'mComments': 'ARE `Comments`', 'm_bSaveDefault': 'save as default area template', 'm_bLoadDefault': 'load default area template', 'bApply': 'apply without closing',
   'dlgOpen': 'world-map icon (dead)', 'm_iImage': 'tileset/area thumbnail',
 },
 'TDLGENVIRONMENT': {
   'pMoonAmbientColor': 'ARE `MoonAmbientColor` (BGR dword)', 'pMoonDiffuseColor': 'ARE `MoonDiffuseColor`', 'pMoonFogColor': 'ARE `MoonFogColor`',
   'eMoonFogAmount': 'ARE `MoonFogAmount`', 'tbMoonFogAmount': 'ARE `MoonFogAmount` (old slider)', 'xbMoonShadows': 'ARE `MoonShadows`',
   'pSunAmbientColor': 'ARE `SunAmbientColor`', 'pSunDiffuseColor': 'ARE `SunDiffuseColor`', 'pSunFogColor': 'ARE `SunFogColor`',
   'eSunFogAmount': 'ARE `SunFogAmount`', 'tbSunFogAmount': 'ARE `SunFogAmount` (old slider)', 'xbSunShadows': 'ARE `SunShadows`',
   'eLightning': 'ARE `ChanceLightning` (%)', 'eRain': 'ARE `ChanceRain` (%)', 'eSnow': 'ARE `ChanceSnow` (%)', 'tbWindPower': 'ARE `WindPower` (0 none,1 weak,2 strong)',
   'rbDayNightCycle': 'ARE `DayNightCycle`=1', 'rbAlwaysDay': 'ARE `DayNightCycle`=0, `IsNight`=0', 'rbAlwaysNight': 'ARE `DayNightCycle`=0, `IsNight`=1',
   'eShadowOpacity': 'ARE `ShadowOpacity` (0–100)', 'eFogClipDistance': 'ARE `FogClipDist` (EE)', 'cbSkyBox': 'ARE `SkyBox` (skyboxes.2da)',
   'bApply': 'preview in viewer', 'dlgColor': 'colour chooser',
 },
 'TDLGAREAWIZARD': {'eName': 'ARE `Name` + resref/tag derived', 'lbTileSets': 'ARE `Tileset` (installed *.set)', 'lbSize': 'size presets (Tiny…Huge)',
                    'eWidth': 'ARE `Width`', 'eHeight': 'ARE `Height`', 'xbLaunchAreaDialog': 'open Area Properties after', 'xbOpenNewArea': 'open viewer tab after'},
 'TFRMAREARESIZE': {'eRows': 'ARE `Height`', 'eColumns': 'ARE `Width`', 'lbSize': 'size presets', 'rbCounterClockwise90': 'rotate area',
                    'rbCounterClockwise180': 'rotate area', 'rbCounterClockwise270': 'rotate area', 'rbClockwise90': 'rotate area', 'rbClockwise180': 'rotate area', 'rbClockwise270': 'rotate area'},
 'TDLGTILEPROPERTIES': {'xbAnimLoop1': 'ARE `Tile_AnimLoop1`', 'xbAnimLoop2': 'ARE `Tile_AnimLoop2`', 'xbAnimLoop3': 'ARE `Tile_AnimLoop3`',
   'pMainLight1': 'ARE `Tile_MainLight1` (lightcolor.2da index)', 'pMainLight2': 'ARE `Tile_MainLight2`', 'pSourceLight1': 'ARE `Tile_SrcLight1`', 'pSourceLight2': 'ARE `Tile_SrcLight2`',
   'pCustomColor1': '(dead)', 'pCustomColor2': '(dead)', 'bDefaults': 'reset to tileset defaults'},
 'TDLGAREATRANSITION': {'cbAreas': 'target area', 'rbBothWays': 'link both ends', 'rbCurrentToTarget': 'link source only', 'rbTargetToCurrent': 'link target only',
   'rbTargetDoor': 'target kind → `LinkedToFlags`=1', 'rbTargetWaypoint': 'target kind → `LinkedToFlags`=2', 'rbTargetTrigger': 'target kind (trigger↔trigger)',
   'lbAreaDoors': 'target object → `LinkedTo` = its tag', 'lbAreaTriggers': 'target object', 'lbAreaWaypoints': 'target object', 'apArea': 'target-area minimap (click to pick)'},
 'TDLGLOCATION': {'ePositionX': 'instance `XPosition`/`X`', 'ePositionY': 'instance `YPosition`/`Y`', 'ePositionZ': 'instance `ZPosition`/`Z`',
   'udPositionX': 'nudge X', 'udPositionY': 'nudge Y', 'udPositionZ': 'nudge Z', 'pOrientationAngle': 'bearing dial (drag)', 'eBearing': 'instance `Bearing` or `XOrientation`/`YOrientation`',
   'udBearing': 'nudge bearing', 'eScale': 'EE VisualTransform scale', 'eRotationX': 'EE VisualTransform rotate X', 'eRotationY': 'EE VisualTransform rotate Y', 'eRotationZ': 'EE VisualTransform rotate Z',
   'eTranslationX': 'EE VisualTransform translate X', 'eTranslationY': 'EE VisualTransform translate Y', 'eTranslationZ': 'EE VisualTransform translate Z',
   'udScale': 'nudge', 'udRotationX': 'nudge', 'udRotationY': 'nudge', 'udRotationZ': 'nudge', 'udTranslationX': 'nudge', 'udTranslationY': 'nudge', 'udTranslationZ': 'nudge', 'bApply': 'apply without closing'},
 'TDLGADDPOPUPTEXT': {'mText': 'DLG single entry `Text`', 'eFileName': 'new DLG resref', 'bText': 'all languages'},
 'TDLGSYSTEMUSAGE': {},
 'TDLGFINDINSTANCE': {'xlbTypes': 'object types to search', 'cbArea': 'area filter (or all)', 'eTemplate': '`TemplateResRef` filter', 'eTag': '`Tag` filter', 'lvResults': 'results (dbl-click = go to)', 'bClear': 'reset criteria'},
 # ---- situated frames
 'TFRASITUATEDBASIC': {'eName': '`LocName`', 'bName': '`LocName` all languages', 'eTag': '`Tag`', 'cbAppearanceType': '`Appearance` (placeables.2da / doortypes.2da)',
   'eFort': '`Fort`', 'eHP': '`HP` (+`CurrentHP`)', 'eWill': '`Will`', 'eRef': '`Ref`', 'eHardness': '`Hardness`', 'xbPlot': '`Plot`'},
 'TFRASITUATEDADVANCED': {'cbFaction': '`Faction` (repute.fac)', 'eTemplate': '`TemplateResRef`', 'cbConversation': '`Conversation`', 'bEditConversation': 'open in Conversation Editor',
   'bBrowseConversation': 'pick .dlg', 'imgPortrait': '`PortraitId` preview', 'ePortrait': '`PortraitId` (portraits.2da)', 'bPortrait': 'opens TdlgPortrait',
   'bUpdateInstancesInArea': 'push blueprint to instances', 'xbInterrupt': '`Interruptable` (inverted)', 'cbOpenState': '`AnimationState` (initial open/closed/destroyed…)', 'bVariables': '`VarTable`'},
 'TFRASITUATEDLOCK': {'xbAutoRemoveKey': '`AutoRemoveKey`', 'eKeyName': '`KeyName`', 'eOpenLockDC': '`OpenLockDC`', 'xbKeyRequired': '`KeyRequired`',
   'xbLocked': '`Locked`', 'xbLockable': '`Lockable`', 'eCloseLockDC': '`CloseLockDC`'},
 'TFRASITUATEDTRAP': {'eDisarmDC': '`DisarmDC`', 'eDetectionDC': '`TrapDetectDC`', 'cbTrapType': '`TrapType` (traps.2da)', 'cbOnDisarm': '`OnDisarm`', 'cbOnTrapTriggered': '`OnTrapTriggered`',
   'eDetectDCMod': 'traps.2da `DetectDCMod` (read-only)', 'eDisarmDCMod': 'traps.2da `DisarmDCMod` (read-only)', 'eSetDC': 'traps.2da `SetDC` (read-only)',
   'xbTrapDisarmable': '`TrapDisarmable`', 'xbTrapOneShot': '`TrapOneShot`', 'xbTrapDetectable': '`TrapDetectable`'},
 'TFRMTRAP': {'eDisarmDC': '`DisarmDC`', 'eDetectionDC': '`TrapDetectDC`', 'cbTrapType': '`TrapType` (traps.2da)', 'cbOnDisarm': '`OnDisarm`', 'cbOnTrapTriggered': '`OnTrapTriggered`',
   'eDetectDCMod': 'traps.2da `DetectDCMod` (read-only)', 'eDisarmDCMod': 'traps.2da `DisarmDCMod` (read-only)', 'eSetDC': 'traps.2da `SetDC` (read-only)',
   'xbTrapDisarmable': '`TrapDisarmable`', 'xbTrapOneShot': '`TrapOneShot`', 'xbTrapDetectable': '`TrapDetectable`'},
 'TFRASITUATEDSCRIPTS': {'cbOnClosed': '`OnClosed`', 'cbOnDamaged': '`OnDamaged`', 'cbOnDeath': '`OnDeath`', 'cbOnLock': '`OnLock`', 'cbOnMeleeAttacked': '`OnMeleeAttacked`',
   'cbOnOpen': '`OnOpen`', 'cbOnSpellCastAt': '`OnSpellCastAt`', 'cbOnUnlock': '`OnUnlock`', 'cbOnUserDefined': '`OnUserDefined`', 'cbOnHeartbeat': '`OnHeartbeat`',
   'cbOnClick': '`OnClick` (EE)', 'cbOnInvDisturbed': 'UTP `OnInvDisturbed`', 'cbOnUsed': 'UTP `OnUsed`', 'cbOnFailToOpen': 'UTD `OnFailToOpen`',
   'bLoadScriptSet': 'read script set', 'bSaveScriptSet': 'write script set'},
 'TFRASITUATEDDESC': {'mDescription': '`Description`', 'bDescription': 'all languages'},
 'TFRASITUATEDCOMMENTS': {'mComments': '`Comment`'},
 'TDLGSITUATEDEDIT': {'apAppearance': 'model preview (drag rotate, wheel zoom)', 'xbTrapFlag': '`TrapFlag`', 'bDefaults': '(hidden) reset'},
 'TDLGSITUATEDMULTIEDITOR': {},
 # ---- creature
 'TDLGCREATUREEDIT': {
   'bRandomName': 'random `FirstName` (race .ltr)', 'bRandomLastName': 'random `LastName`', 'bUniqueTag': 'generate unique `Tag`',
   'eFirstName': 'UTC `FirstName`', 'bLocFirstName': 'all languages', 'eLastName': 'UTC `LastName`', 'bLocLastName': 'all languages',
   'cbRace': 'UTC `Race` (racialtypes.2da)', 'cbPhenotype': 'UTC `Phenotype` (phenotype.2da)', 'cbGenders': 'UTC `Gender` (gender.2da)',
   'eDescription': 'UTC `Description`', 'bLocDescription': 'all languages', 'cbAppearance': 'UTC `Appearance_Type` (appearance.2da)',
   'eTag': 'UTC `Tag`', 'eCRBasic': 'UTC `ChallengeRating` (display)', 'ePaletteCategory': 'UTC `PaletteID` (creaturepal.itp)', 'bPaletteCategory': 'opens TdlgPaletteChooser',
   'imgPortrait': 'portrait preview', 'bPortrait': 'opens TdlgPortrait', 'ePortrait': 'UTC `PortraitId` (portraits.2da) / `Portrait`',
   'sbEditConversation': 'open DLG in editor', 'cbConversation': 'UTC `Conversation`', 'bBrowseConversation': 'pick .dlg', 'xbNoInterrupt': 'UTC `Interruptable` (inverted)',
   'eStrength': 'UTC `Str`', 'eDexterity': 'UTC `Dex`', 'eConstitution': 'UTC `Con`', 'eIntelligence': 'UTC `Int`', 'eWisdom': 'UTC `Wis`', 'eCharisma': 'UTC `Cha`',
   'eStrengthBonus': 'derived', 'eDexterityBonus': 'derived', 'eConstitutionBonus': 'derived', 'eIntelligenceBonus': 'derived', 'eWisdomBonus': 'derived', 'eCharismaBonus': 'derived',
   'eRacialStrMod': 'racialtypes.2da `StrAdjust`', 'eRacialDexMod': 'racialtypes.2da `DexAdjust`', 'eRacialConMod': 'racialtypes.2da `ConAdjust`',
   'eRacialIntMod': 'racialtypes.2da `IntAdjust`', 'eRacialWisMod': 'racialtypes.2da `WisAdjust`', 'eRacialChaMod': 'racialtypes.2da `ChaAdjust`',
   'eStrTotal': 'derived', 'eDexTotal': 'derived', 'eConTotal': 'derived', 'eIntTotal': 'derived', 'eWisTotal': 'derived', 'eChaTotal': 'derived',
   'eNaturalAC': 'UTC `NaturalAC`', 'eACBase': 'derived (10)', 'eACBonus': 'derived', 'eACSize': 'appearance.2da `SIZECATEGORY` → creaturesize.2da', 'eACTotal': 'derived',
   'eSaveReflex': 'classes.2da save tables', 'eSaveWill': 'classes.2da save tables', 'eSaveFortitude': 'classes.2da save tables',
   'eSaveFortitudeBonus': 'UTC `fortbonus`', 'eSaveReflexBonus': 'UTC `refbonus`', 'eSaveWillBonus': 'UTC `willbonus`',
   'eSaveFortitudeTotal': 'derived', 'eSaveReflexTotal': 'derived', 'eSaveWillTotal': 'derived', 'eFortModifier': 'racial/feat mods', 'eReflexModifier': 'racial/feat mods', 'eWillModifier': 'racial/feat mods',
   'eHitPoints': 'UTC `HitPoints` (+`CurrentHitPoints`,`MaxHitPoints`)', 'eHPBonus': 'derived', 'eHPTotal': 'derived',
   'cbMovementRate': 'UTC `WalkRate` (creaturespeed.2da)', 'bColor': 'opens TColorPicker → `Color_Skin`,`Color_Hair`,`Color_Tattoo1`,`Color_Tattoo2`',
   'cbThighRight': 'UTC `BodyPart_RThigh` (parts_legs)', 'cbShinRight': 'UTC `BodyPart_RShin`', 'cbFootRight': 'UTC `ArmorPart_RFoot` (sic)', 'cbThighLeft': 'UTC `BodyPart_LThigh`',
   'cbShinLeft': 'UTC `BodyPart_LShin`', 'cbFootLeft': 'UTC `BodyPart_LFoot`', 'cbBicepRight': 'UTC `BodyPart_RBicep`', 'cbForearmRight': 'UTC `BodyPart_RFArm`',
   'cbHandRight': 'UTC `BodyPart_RHand`', 'cbBicepLeft': 'UTC `BodyPart_LBicep`', 'cbForearmLeft': 'UTC `BodyPart_LFArm`', 'cbHandLeft': 'UTC `BodyPart_LHand`',
   'cbHead': 'UTC `Appearance_Head`', 'cbNeck': 'UTC `BodyPart_Neck`', 'cbTorso': 'UTC `BodyPart_Torso`', 'cbPelvis': 'UTC `BodyPart_Pelvis`',
   'cbWings': 'UTC `Wings_New` (wingmodel.2da)', 'cbTail': 'UTC `Tail_New` (tailmodel.2da)',
   'cbAlignment': 'UTC `GoodEvil` + `LawfulChaotic` (9 presets)',
   'cbClass1': 'UTC `ClassList[0].Class` (classes.2da)', 'cbClass2': '`ClassList[1].Class`', 'cbClass3': '`ClassList[2].Class`', 'cbClass4': '`ClassList[3].Class`',
   'cbClass5': '`ClassList[4].Class`', 'cbClass6': '`ClassList[5].Class`', 'cbClass7': '`ClassList[6].Class`', 'cbClass8': '`ClassList[7].Class`',
   'eClass1Level': '`ClassList[0].ClassLevel`', 'eClass2Level': '`ClassList[1].ClassLevel`', 'eClass3Level': '`ClassList[2].ClassLevel`', 'eClass4Level': '`ClassList[3].ClassLevel`',
   'eClass5Level': '`ClassList[4].ClassLevel`', 'eClass6Level': '`ClassList[5].ClassLevel`', 'eClass7Level': '`ClassList[6].ClassLevel`', 'eClass8Level': '`ClassList[7].ClassLevel`',
   'cbStartingPackage': 'UTC `StartingPackage` (packages.2da)', 'bCreatureLevelupWizard': 'opens TfrmCreatureLevelupWizard',
   'sgSkills': 'UTC `SkillList[].Rank` (skills.2da order)',
   'cbOnHeartbeat': 'UTC `ScriptHeartbeat`', 'cbOnPerception': 'UTC `ScriptOnNotice`', 'cbOnSpellCast': 'UTC `ScriptSpellAt`', 'cbOnMeleeAttacked': 'UTC `ScriptAttacked`',
   'cbOnDamaged': 'UTC `ScriptDamaged`', 'cbOnDisturbed': 'UTC `ScriptDisturbed`', 'cbOnEndCombatRound': 'UTC `ScriptEndRound`', 'cbOnDialogue': 'UTC `ScriptDialogue`',
   'cbOnSpawnIn': 'UTC `ScriptSpawn`', 'cbOnRested': 'UTC `ScriptRested`', 'cbOnDeath': 'UTC `ScriptDeath`', 'cbUserDefine': 'UTC `ScriptUserDefine`', 'cbOnBlocked': 'UTC `ScriptOnBlocked`',
   'bLoadScriptSet': 'read script set', 'bSaveScriptSet': 'write script set',
   'cbTreasurePile': 'UTC `BodyBag` (bodybag.2da)', 'eDecayTime': 'UTC `DecayTime` (ms in GFF, seconds in UI)', 'xbLootable': 'UTC `Lootable`', 'cbDisarmable': 'UTC `Disarmable`',
   'xbPlot': 'UTC `Plot`', 'xbNoPermanentDeath': 'UTC `NoPermDeath`', 'xbImmortal': 'UTC `IsImmortal`',
   'ePositionX': '(dead)', 'ePositionY': '(dead)', 'ePositionZ': '(dead)', 'eOrientationX': '(dead)', 'eOrientationY': '(dead)',
   'cbFaction': 'UTC `FactionID` (repute.fac)', 'sbFactionEditor': 'opens Faction Editor',
   'eCRAdjustment': 'UTC `CRAdjust`', 'eCRCalculated': 'derived CR', 'eChallengeRating': 'UTC `ChallengeRating` (= calc + adjust)',
   'bChooseTemplates': 'shows Template tab', 'eSubRace': 'UTC `Subrace`', 'eDeity': 'UTC `Deity`',
   'eSoundSet': 'UTC `SoundSetFile` (soundset.2da)', 'bEditSoundSet': 'opens TdlgSoundSetSelect', 'cbPerceptionRange': 'UTC `PerceptionRange` (ranges.2da)',
   'bVariables': 'UTC `VarTable`', 'eResRef': 'UTC `TemplateResRef`', 'bUpdateInstancesInArea': 'push blueprint to instances',
   'sgFeatsTable': 'UTC `FeatList[].Feat` (feat.2da; granted/available/selected)', 'cbFeatsFilter': 'feat list filter', 'meFeatsSummary': 'derived summary', 'bFeatsHelp': 'help',
   'lvSpellTable': 'UTC `ClassList[c].KnownList{L}` / `MemorizedList{L}` (`Spell`,`SpellFlags`,`SpellMetaMagic`)', 'sgSpellTable': '(legacy grid)',
   'cbSpellLevels': 'spell level 0–9 filter', 'cbMetamagic': '`SpellMetaMagic` of selected entry', 'rbClass1': 'class selector for spell list', 'rbClass2': 'class selector',
   'rbClass3': 'class selector', 'rbClass4': 'class selector', 'rbClass5': 'class selector', 'rbClass6': 'class selector', 'rbClass7': 'class selector', 'rbClass8': 'class selector',
   'meSummary': 'derived summary', 'bClearSpellList': 'clear class spells', 'bSaveSpellSet': 'write spell list file', 'bLoadSpellSet': 'read spell list file', 'bSpellsHelp': 'help',
   'sgSpecialAbilitiesTable': 'UTC `SpecAbilityList[]` (`Spell`,`SpellCasterLevel`,`SpellFlags`)', 'cbSpecialAbilitiesFilter': 'filter', 'meSpecialAbilitiesSummary': 'summary', 'bSpecialAbilitiesHelp': 'help',
   'lbTemplates': 'crtemplates.2da rows', 'bApplySelectedTemplates': 'apply creature template(s)', 'bCancelTemplates': 'leave Template tab',
   'mComments': 'UTC `Comment`', 'apCreature': 'model preview', 'bInventory': 'opens TdlgInventory (`Equip_ItemList`, `ItemList`)',
   'bOK': 'save', 'dlgOpenFile': '(unused .dlg picker)',
 },
 'TDLGCREATUREWIZARD': {'tvPaletteSelector': 'UTC `PaletteID`', 'cbAppearance': 'UTC `Appearance_Type`', 'imgPortrait': 'UTC `PortraitId`', 'bPortraitSelect': 'opens TdlgPortrait',
   'cbGender': 'UTC `Gender`', 'mSummary': 'review text', 'xbLaunchCreatureEditor': 'open Creature Properties after', 'eCreatureName': 'UTC `FirstName`', 'bRandom': 'random name',
   'eLastName': 'UTC `LastName`', 'bRandomLastName': 'random last name', 'lbFactions': 'UTC `FactionID`', 'lbRacialType': 'UTC `Race` (+default appearance/abilities)',
   'xbDoNotShowAgain': '(hidden) ini', 'lbClasses': 'classes.2da (player + NPC classes)', 'bClassAdd': 'add selected class (max 8)',
   'eClass1': '`ClassList[0].Class`', 'eClass1Level': '`ClassList[0].ClassLevel`', 'apAppearance': 'model preview'},
 'TFRMCREATURELEVELUPWIZARD': {'lbClasses': 'classes.2da', 'bClassAdd': 'add class', 'eClass1': 'class 1', 'eClass1Level': 'levels to add'},
 'TCOLORPICKER': {'Image': 'PLT colour ramp image (click = select index)', 'TexLayerList': 'layer: skin/hair/tattoo1/tattoo2 or cloth1/2, leather1/2, metal1/2'},
 'TDLGSOUNDSETSELECT': {'rbMale': 'filter soundset.2da `GENDER`', 'rbFemale': 'filter', 'rbMaleFemale': 'filter', 'cbTypes': 'filter soundset.2da `TYPE` (soundsettype.2da)',
   'lbStrRefs': 'soundset.2da rows (click = play sample)'},
 'TDLGINVENTORY': {
   'mbNew': 'Item Wizard', 'tvItems': 'standard item palette (drag source)', 'tvCustomItems': 'custom item palette (drag source)',
   'cbInfinite': 'UTM `StoreList[].ItemList[].Infinite`', 'cbPickpocketable': 'UTC `ItemList[].Pickpocketable`', 'cbDropable': 'UTC `ItemList[].Dropable`/`Equip_ItemList` droppable',
   'meSelectedName': 'selected item name',
   'imPrimary': 'equip slot RIGHTHAND (0x10)', 'imArmor': 'equip slot CHEST (0x2)', 'imSecondary': 'equip slot LEFTHAND (0x20)', 'imCloak': 'equip slot CLOAK (0x40)',
   'imHelmet': 'equip slot HEAD (0x1)', 'imBoots': 'equip slot BOOTS (0x4)', 'imGauntlets': 'equip slot ARMS (0x8)', 'imAmulet': 'equip slot NECK (0x200)',
   'imBelt': 'equip slot BELT (0x400)', 'imArrows': 'equip slot ARROWS (0x800)', 'imBolts': 'equip slot BOLTS (0x2000)', 'imBullets': 'equip slot BULLETS (0x1000)',
   'imRing1': 'equip slot LEFTRING (0x80)', 'imRing2': 'equip slot RIGHTRING (0x100)', 'imCWeapon1': 'creature weapon L (0x4000)', 'imCWeapon2': 'creature weapon R (0x8000)',
   'imCWeapon3': 'creature weapon B / special (0x10000)', 'imCArmor': 'creature hide (0x20000)',
   'dgBackpack': '`ItemList[]` (`Repos_PosX/Y`) — all', 'dgArmor': 'store cat. armor (StoreList 0)', 'dgWeapons': 'store cat. weapons (StoreList 4)', 'dgPotions': 'store cat. potions/scrolls (StoreList 2)',
   'dgRings': 'store cat. rings/amulets (StoreList 3)', 'dgMisc': 'store cat. misc (StoreList 1)', 'imDrop': 'drop target', 'mbRandom': '(hidden) random item generators', 'imTrash': 'drag here to delete',
 },
 # ---- item
 'TDLGITEMEDIT': {
   'meCost': 'UTI `Cost` (derived)', 'meWeight': 'baseitems.2da `TenthLBS` + props', 'meDamage': 'baseitems.2da dice', 'meCritical': 'baseitems.2da `CritThreat`/`CritHitMult`',
   'eDamageType': 'baseitems.2da `WeaponType`', 'meArmor': 'armor AC (armor.2da via `ArmorPart_Torso` parts_chest.2da `ACBONUS`)', 'eCharges': 'UTI `Charges` (0–250)',
   'cbStolen': 'UTI `Stolen`', 'ePaletteCategory': 'UTI `PaletteID` (itempal.itp)', 'bPaletteCategory': 'opens TdlgPaletteChooser', 'cbPlotItem': 'UTI `Plot`',
   'eAddCost': 'UTI `AddCost`', 'eStackSize': 'UTI `StackSize` (1…baseitems `Stacking`)', 'meLevel': 'itemvalue.2da required level', 'meLore': 'skillvsitemcost.2da required lore',
   'eACType': 'armor.2da type', 'eACCheck': 'armor.2da `ACCHECK`', 'eArcaneFailure': 'armor.2da `ARCANEFAILURE%`', 'eMaxDexBonus': 'armor.2da `DEXBONUS`',
   'eName': 'UTI `LocalizedName`', 'bLocName': 'all languages', 'eResRef': 'UTI `TemplateResRef`', 'eTag': 'UTI `Tag`', 'eBaseName': 'UTI `BaseItem` name (baseitems.2da; fixed after creation)', 'bUniqueTag': 'generate unique tag',
   'icItem': 'inventory icon preview', 'cbSimple': 'UTI `ModelPart1` (simple items)', 'lvSimple': 'UTI `ModelPart1` (icon grid)',
   'cbWeaponTop': 'UTI `ModelPart3` model', 'cbWeaponMiddle': 'UTI `ModelPart2` model', 'cbWeaponBottom': 'UTI `ModelPart1` model',
   'cbWeaponTopColor': 'UTI `ModelPart3` colour digit', 'cbWeaponMiddleColor': 'UTI `ModelPart2` colour digit', 'cbWeaponBottomColor': 'UTI `ModelPart1` colour digit',
   'cbForearmRight': 'UTI `ArmorPart_RFArm`', 'cbHandRight': 'UTI `ArmorPart_RHand`', 'cbBicepRight': 'UTI `ArmorPart_RBicep`', 'cbBicepLeft': 'UTI `ArmorPart_LBicep`',
   'cbForearmLeft': 'UTI `ArmorPart_LFArm`', 'cbHandLeft': 'UTI `ArmorPart_LHand`', 'cbNeck': 'UTI `ArmorPart_Neck`', 'cbThighs': 'UTI `ArmorPart_LThigh`+`RThigh`',
   'cbShins': 'UTI `ArmorPart_LShin`+`RShin`', 'cbFeet': 'UTI `ArmorPart_LFoot`+`RFoot`', 'cbShoulderLeft': 'UTI `ArmorPart_LShoul`', 'cbShoulderRight': 'UTI `ArmorPart_RShoul`',
   'cbTorso': 'UTI `ArmorPart_Torso` (parts_chest.2da → AC)', 'cbBelt': 'UTI `ArmorPart_Belt`', 'cbPelvis': 'UTI `ArmorPart_Pelvis`', 'cbRobe': 'UTI `ArmorPart_Robe` (parts_robe.2da)',
   'meArmor2': 'armour AC (derived)', 'rb2D': 'preview mode', 'rb3D': 'preview mode', 'rbColor': '(hidden)', 'rbModel': '(hidden)', 'rbFemale': 'preview body gender', 'rbMale': 'preview body gender',
   'bColorChooser': 'TColorPicker → `Cloth1Color`,`Cloth2Color`,`Leather1Color`,`Leather2Color`,`Metal1Color`,`Metal2Color` (also helmets/cloaks)',
   'tvMaster': 'available properties (itemprops.2da column for base item; itempropdef.2da names)', 'bAdd': 'add property (→ TdlgPropEdit if params)',
   'bRemove': 'remove property', 'tvAssigned': 'UTI `PropertiesList[]` (`PropertyName`,`Subtype`,`CostTable`,`CostValue`,`Param1`,`Param1Value`,`ChanceAppear`)',
   'meCost2': 'UTI `Cost`', 'bEdit': 'opens TdlgPropEdit', 'eMin': 'property-count info', 'eMax': 'cast-spell use limit info', 'cbIdentified': 'UTI `Identified`',
   'xbCursed': 'UTI `Cursed` (EE label "Undroppable")',
   'meDescUnidentified': 'UTI `Description`', 'meDescIdentified': 'UTI `DescIdentified`', 'bDescUnidentified': 'all languages', 'bDescIdentified': 'all languages',
   'meDescType': 'baseitems.2da `Description` (read-only)', 'meDescStats': 'generated property text', 'bVariables': 'UTI `VarTable`', 'mComments': 'UTI `Comment`',
   'apItem': 'model preview', 'bInventory': '(hidden) container contents', 'bUpdateInstancesInArea': 'push blueprint to instances', 'bDefault': '(hidden)',
 },
 'TDLGPROPEDIT': {'eProperty': '`PropertyName` (itempropdef.2da)', 'eSubProperty': '`Subtype` (itempropdef `SubTypeResRef` table)', 'eCostParam': '`CostTable`/`CostValue`',
   'eParam1': '`Param1`/`Param1Value`', 'bCostSelect': 'pick from tvCostParam', 'bParam1Select': 'pick from tvParam1', 'tvCostParam': 'iprp_costtable.2da → cost table rows',
   'tvParam1': 'iprp_paramtable.2da → param table rows', 'eChance': '`ChanceAppear` (0–100)'},
 'TDLGITEMWIZARD': {'pSubChooser': 'base item type chooser (TdlgChooser tree; itmwiz*.2da)', 'eName': 'UTI `LocalizedName` + resref', 'rgItemLevel': 'item level band (disabled)',
   'rgItemQuality': 'quality (disabled)', 'xbMagical': 'magical flag (adds props)', 'cbLaunch': 'open Item Properties after', 'tvPaletteSelector': 'UTI `PaletteID`'},
 'TDLGITEMGENERATOREDIT': {'eName': 'generator name', 'bName': 'all languages', 'eTag': 'tag', 'dgContents': 'item templates in generator (drag from palette)', 'cbScaleByLevel': 'scale by level',
   'cbTreasureValue': 'treasure value (treasurescale.2da?)', 'eResRef': 'template resref', 'mComments': 'comment', 'tvItems': 'item palette (drag source)', 'mbNew': 'new item', 'imTrash': 'drag to delete'},
 'TDLGGENERATORCHOOSER': {'sgAssigned': 'assigned generator templates', 'tvGeneratorPalette': 'generator palette', 'bAssign': 'assign', 'bRemove': 'remove', 'bRemoveAll': 'remove all'},
 # ---- placeable / door
 'TDLGPLACEABLEEDIT': {'xbUseable': 'UTP `Useable`', 'xbInventory': 'UTP `HasInventory`', 'ePaletteCategory': 'UTP `PaletteID`', 'bPaletteCategory': 'opens TdlgPaletteChooser',
   'xbStatic': 'UTP `Static`', 'cbTreasurePile': 'UTP `BodyBag` (bodybag.2da)', 'bInventory': 'opens TdlgInventory (`ItemList`)', 'fraSituatedBasic1': 'UTP basic fields', 'fraSituatedAdvanced1': 'UTP advanced fields'},
 'TDLGDOOREDIT': {'cbGenericType': 'UTD `GenericType_New` (genericdoors.2da)', 'ePaletteCategory': 'UTD `PaletteID`', 'bPaletteCategory': 'opens TdlgPaletteChooser',
   'eLinkedTo': 'UTD `LinkedTo`', 'xbPartyRequired': '(dead)', 'rbLinkedToWaypoint': 'UTD `LinkedToFlags`=2', 'rbLinkedToDoor': 'UTD `LinkedToFlags`=1', 'rbLinkedToNothing': 'UTD `LinkedToFlags`=0',
   'bSetupAreaTransition': 'opens TdlgAreaTransition', 'eLoadScreen': 'UTD `LoadScreenID`', 'bBrowseAreaTransitionBitmap': 'opens TdlgLoadScreen', 'eSecretDoorDC': '(dead)',
   'fraSituatedBasic1': 'UTD basic fields (`cbAppearanceType` = doortypes.2da tileset door)'},
 # ---- trigger
 'TDLGTRIGGEREDIT': {'eDisplayName': 'UTT `LocalizedName`', 'bDisplayName': 'all languages', 'eTag': 'UTT `Tag`', 'cbTriggerType': 'UTT `Type` (0 generic, 1 area transition, 2 trap)',
   'ePaletteCategory': 'UTT `PaletteID`', 'bPaletteCategory': 'opens TdlgPaletteChooser', 'eLinkedTo': 'UTT `LinkedTo`', 'rbLinkedToWaypoint': '`LinkedToFlags`=2', 'rbLinkedToDoor': '`LinkedToFlags`=1',
   'rbLinkedToNothing': '`LinkedToFlags`=0', 'xbPartyRequired': '(dead)', 'bSetupAreaTransition': 'opens TdlgAreaTransition', 'bBrowseAreaTransitionBitmap': 'opens TdlgLoadScreen', 'eLoadScreen': 'UTT `LoadScreenID`',
   'cbOnEnter': 'UTT `ScriptOnEnter`', 'cbOnExit': 'UTT `ScriptOnExit`', 'cbOnHeartbeat': 'UTT `ScriptHeartbeat`', 'cbOnUserDefine': 'UTT `ScriptUserDefine`', 'cbOnClick': 'UTT `OnClick`',
   'bLoadScriptSet': 'read script set', 'bSaveScriptSet': 'write script set', 'eTemplateName': 'UTT `TemplateResRef`', 'cbFaction': 'UTT `Faction`', 'eKeyName': 'UTT `KeyName` (transition key)',
   'xbAutoRemoveKey': 'UTT `AutoRemoveKey`', 'cbCursor': 'UTT `Cursor` (cursors.2da, icons drawn)', 'imgPortrait': 'UTT `PortraitId` preview', 'bPortraitSelect': 'opens TdlgPortrait',
   'ePortrait': 'UTT `PortraitId`', 'eHighlightHeight': 'UTT `HighlightHeight`', 'udHighlightHeight': 'nudge HighlightHeight', 'bVariablesEdit': 'UTT `VarTable`', 'mComments': 'UTT `Comment`',
   'sbMain': 'status bar', 'bUpdateInstancesInArea': 'push blueprint to instances', 'bDefault': '(hidden)'},
 # ---- encounter
 'TDLGENCOUNTEREDIT': {'eDifficulty': '(hidden) UTE `Difficulty`', 'eMaxCreatures': 'UTE `MaxCreatures`', 'eRecCreatures': 'UTE `RecCreatures`', 'eTag': 'UTE `Tag`',
   'cbSpawnOption': 'UTE `SpawnOption` (0 continuous, 1 single shot)', 'eLocName': 'UTE `LocalizedName`', 'bLocName': 'all languages', 'cbDifficulty': 'UTE `DifficultyIndex` (encdifficulty.2da; also sets `Difficulty`)',
   'ePaletteCategory': 'UTE `PaletteID`', 'bPaletteCategory': 'opens TdlgPaletteChooser', 'bRecalculateCRs': '(hidden) refresh `CreatureList[].CR`',
   'cbOnEnter': 'UTE `OnEntered`', 'cbOnExhausted': 'UTE `OnExhausted`', 'cbOnUserDefined': 'UTE `OnUserDefined`', 'cbOnExit': 'UTE `OnExit`', 'cbOnHeartbeat': 'UTE `OnHeartbeat`',
   'bLoadScriptSet': 'read script set', 'bSaveScriptSet': 'write script set', 'eResetTime': 'UTE `ResetTime` (s)', 'xbReset': 'UTE `Reset`', 'cbFaction': 'UTE `Faction`',
   'eTemplate': 'UTE `TemplateResRef`', 'xbActive': 'UTE `Active`', 'eRespawns': 'UTE `Respawns`', 'xbPlayerTriggeredOnly': 'UTE `PlayerOnly`', 'xbRespawnInfinite': 'UTE `Respawns`=-1',
   'bVariables': 'UTE `VarTable`', 'mComments': 'UTE `Comment`', 'bUpdateInstancesInArea': 'push blueprint to instances', 'bDefaults': '(hidden)'},
 'TFRAENCOUNTERCREATURELIST': {'bAddCreature': 'append to CreatureList', 'bRemoveCreature': 'remove row', 'tvStandardCreatures': 'standard creature palette', 'tvCustomCreatures': 'custom creature palette',
   'sgCreatureList': 'UTE `CreatureList[]` (`ResRef`,`CR`,`Appearance`,`SingleSpawn`="Unique")'},
 'TDLGENCOUNTERWIZARD': {'pCreatureListFrame': 'hosts TfraEncounterCreatureList'},
 # ---- sound
 'TDLGSOUNDEDIT': {'bPlay': 'preview selected', 'lbSounds': 'UTS `Sounds[].Sound`', 'bRemoveSound': 'remove', 'bAdd': 'opens TdlgResOpenSound (multi-add)', 'bMoveDown': 'reorder', 'bMoveUp': 'reorder', 'bStop': 'stop preview',
   'ePaletteCategory': 'UTS `PaletteID`', 'bPaletteCategory': 'opens TdlgPaletteChooser', 'eLocName': 'UTS `LocName`', 'bLocName': 'all languages', 'eTag': 'UTS `Tag`', 'tbVolume': 'UTS `Volume` (0–127)',
   'eComments': '(hidden)', 'bComments': '(hidden)', 'mComments': 'UTS `Comment`',
   'rbOmnipresent': 'UTS `Positional`=0 (area-wide)', 'rbPositionalRandom': 'UTS `Positional`=1,`RandomPosition`=1', 'rbPositional': 'UTS `Positional`=1,`RandomPosition`=0',
   'eDistanceMax': 'UTS `MaxDistance`', 'eDistanceMin': 'UTS `MinDistance`', 'tbDistanceMin': '(hidden)', 'eRandomRangeX': 'UTS `RandomRangeX`', 'eRandomRangeY': 'UTS `RandomRangeY`',
   'eElevation': 'UTS `Elevation`', 'tbElevation': '(hidden)', 'eTemplateResRef': 'UTS `TemplateResRef`', 'ePitchVariation': 'UTS `PitchVariation`', 'tbPitchVariation': 'UTS `PitchVariation`',
   'bUpdateInstancesInArea': 'push blueprint to instances', 'xbActive': 'UTS `Active`', 'rbTimeAlways': 'UTS `Times`=3', 'rbTimeNight': 'UTS `Times`=2', 'rbTimeDay': 'UTS `Times`=1', 'rbTimeSpecific': 'UTS `Times`=0 + `Hours`',
   'rbLooping': 'UTS `Looping`=1', 'rbContinuous': 'UTS `Continuous`=1,`Looping`=0', 'rbOnce': 'UTS `Continuous`=0', 'rbSequential': 'UTS `Random`=0', 'rbRandom': 'UTS `Random`=1',
   'eInterval': 'UTS `Interval` (ms; UI in s)', 'eIntervalVariation': 'UTS `IntervalVrtn`', 'tbVolumeVariation': 'UTS `VolumeVrtn`', 'bVariables': 'UTS `VarTable`', 'bDefaults': '(hidden)',
   'rbContinuousRandom': '(hidden legacy)', 'rbOnceRandom': '(hidden legacy)', 'rbContinuousSequential': '(hidden legacy)', 'rbOnceSequential': '(hidden legacy)',
   'rbContinuousSequentialSingle': '(hidden legacy)', 'rbOnceSequentialSingle': '(hidden legacy)', 'rbOneShot': '(hidden legacy)',
   **{f'xb{h}{ap}': f'UTS `Hours` bit {(h % 12) + (12 if ap == "PM" else 0)}' for h in range(1, 13) for ap in ('AM', 'PM')},
 },
 'TDLGSOUNDWIZARD': {'rbLooping': 'timing preset (sounddefaultstim.2da Looping)', 'rbSingleShot': 'timing preset (sounddefaultstim.2da)', 'rbAreaWide': 'sounddefaultspos.2da AreaWide',
   'rbPositionalRandom': 'sounddefaultspos.2da PositionalRandom', 'rbPositional': 'sounddefaultspos.2da Positional', 'lbWaves': 'UTS `Sounds[]`', 'bAddWaves': 'opens TdlgResOpenSound', 'bRemoveWave': 'remove'},
 # ---- store
 'TDLGSTOREEDIT': {'eName': 'UTM `LocName`', 'bName': 'all languages', 'eTag': 'UTM `Tag`', 'bInventory': 'opens TdlgInventory (store mode: `StoreList[0..4].ItemList`)',
   'cbBlackMarket': 'UTM `BlackMarket`', 'eBMMarkDown': 'UTM `BM_MarkDown` (%)', 'eMarkUp': 'UTM `MarkUp` (%)', 'eMarkDown': 'UTM `MarkDown` (%)', 'eIdentifyPrice': 'UTM `IdentifyPrice`',
   'xbWillIdentify': 'UTM `IdentifyPrice` ≠ -1', 'ePaletteCategory': 'UTM `ID` (palette id)', 'bPaletteCategory': 'opens TdlgPaletteChooser', 'bUniqueTag': 'unique tag',
   'eMaxBuyPrice': 'UTM `MaxBuyPrice`', 'xbHasMaxBuyPrice': 'UTM `MaxBuyPrice` ≠ -1', 'xbHasLimitedGold': 'UTM `StoreGold` ≠ -1', 'eStoreGold': 'UTM `StoreGold`',
   'eResRef': 'UTM `ResRef` (template resref)', 'cbOnOpenStore': 'UTM `OnOpenStore`', 'bOnOpenStore': 'browse', 'bOnOpenStoreEdit': 'edit', 'cbOnStoreClosed': 'UTM `OnStoreClosed`',
   'bOnStoreClosed': 'browse', 'bEditOnStoreClosed': 'edit', 'bUpdateInstancesInArea': 'push blueprint to instances', 'bVariables': 'UTM `VarTable`',
   'rbWillNotBuy': 'UTM `WillNotBuy[]` mode', 'rbWillOnlyBuy': 'UTM `WillOnlyBuy[]` mode', 'lbRestrictedItems': 'UTM `WillNotBuy`/`WillOnlyBuy` `BaseItem` list (baseitems.2da)',
   'bRestrictedItemsAdd': 'add base item', 'bRestrictedItemsRemove': 'remove', 'bRestrictedItemsRemoveAll': 'clear', 'bRestrictedItemsAddAll': '(hidden)', 'mComments': 'UTM `Comment`'},
 'TDLGSTORESETUPWIZARD': {'mGreeting': 'DLG NPC entry text', 'eReplyYes': 'DLG PC reply (opens store)', 'eReplyNo': 'DLG PC reply (exit)', 'eConversationResRef': 'new DLG resref → creature `Conversation`',
   'eOpenStoreScriptResRef': 'generated NSS resref (nw_ / gplotAppraiseOpenStore)', 'xbUseAppraiseCheck': 'script uses appraise-adjusted OpenStore', 'pPalettesStore': 'hosts TfraBlueprintSelect (stores)',
   'pPalettesShopkeeper': 'hosts TfraBlueprintSelect (creatures)', 'cbFaction': 'shopkeeper `FactionID` override', 'xbUseSuggestedFaction': 'apply faction change', 'eOldFaction': 'current faction (info)'},
 # ---- waypoint
 'TDLGWAYPOINTEDIT': {'eTag': 'UTW `Tag`', 'eLocName': 'UTW `LocalizedName`', 'eLinkedTo': '(hidden) UTW `LinkedTo`', 'bLocName': 'all languages', 'ePaletteCategory': 'UTW `PaletteID`',
   'bPaletteCategory': 'opens TdlgPaletteChooser', 'cbAppearance': 'UTW `Appearance` (waypoint.2da)', 'eMapNote': 'UTW `MapNote`', 'bMapNote': 'all languages', 'cbHasMapNote': 'UTW `HasMapNote`',
   'cbMapNoteEnabled': 'UTW `MapNoteEnabled`', 'eResRef': 'UTW `TemplateResRef`', 'bUpdateInstancesInArea': 'push blueprint to instances', 'bVariables': 'UTW `VarTable`',
   'mDescription': 'UTW `Description`', 'bDescription': 'all languages', 'mComments': 'UTW `Comment`'},
 'TDLGWAYPOINTWIZARD': {'tvPaletteSelector': 'UTW `PaletteID`', 'xbLaunchWaypointDialog': 'open properties after', 'eName': 'UTW `LocalizedName`', 'eTag': 'UTW `Tag`', 'cbAppearance': 'UTW `Appearance`'},
 'TDLGBLUEPRINTWIZARD': {'tvPaletteSelector': '`PaletteID`', 'eName': 'name → resref/tag derived'},
 'TDLGDOORWIZARD': {'xbLaunchPropertiesDialog': 'open properties after', 'lbGenericAppearances': 'UTD `GenericType_New`'},
 'TDLGPLACEABLEWIZARD': {'xbLaunchPropertiesDialog': 'open properties after'},
 'TDLGTRIGGERWIZARD': {'lbTriggerType': 'UTT `Type`', 'xbLaunchPropertiesDialog': 'open properties after', 'tvPaletteSelector': 'UTT `PaletteID`', 'eName': 'UTT `LocalizedName`'},
 'TDLGSTOREWIZARD': {'xbLaunchPropertiesDialog': 'open properties after', 'tvPaletteSelector': 'UTM `ID`', 'eName': 'UTM `LocName`'},
 # ---- conversation
 'TDLGCONVERSATIONEDITOR': {
   'RightViewStatusBar': 'status (4 panels; dbl-click)', 'fraConversationTree1': 'Scrap tree (cut nodes)', 'tbFilterComment': 'highlight nodes with `Comment`', 'tbFilterAction': 'highlight nodes with `Script`',
   'tbFilterQuest': 'highlight nodes with `Quest`', 'tbFilterAnimation': 'highlight nodes with `Animation`', 'tbFilterSound': 'highlight nodes with `Sound`',
   'tbExpandAll': 'expand tree', 'tbCompressAll': 'collapse tree', 'bOK': 'close editor',
   'bConditionWizard': 'Script Wizard (condition)', 'xbOnceOnly': '(hidden)', 'cbCondition': 'DLG link `Active` (StartingList/RepliesList/EntriesList struct)',
   'vlConditionParameters': 'DLG link `ConditionParams[]` (Key/Value, EE)', 'bAddConditionParameter': 'add param', 'bRemoveConditionParameter': 'remove param', 'mConditionScript': 'NSS preview',
   'bActionWizard': 'Script Wizard (action)', 'cbActionScript': 'DLG node `Script`', 'vlActionParameters': 'DLG node `ActionParams[]` (EE)', 'bAddActionParameter': 'add param',
   'bRemoveActionParameter': 'remove param', 'mActionScriptPreview': 'NSS preview', 'cbAnimation': 'DLG node `Animation`', 'cbSound': 'DLG node `Sound`', 'bSoundPlay': 'preview',
   'cbQuest': 'DLG node `Quest` (journal tag)', 'cbQuestEntry': 'DLG node `QuestEntry`', 'bJournalEditor': 'opens Journal Editor', 'xbAnimLoop': '(hidden) `AnimLoop`',
   'mComment': 'DLG node `Comment`', 'cbEndConvScript': 'DLG `EndConversation`', 'cbEndConvAbortScript': 'DLG `EndConverAbort`', 'bScriptBrowse1': 'browse', 'bScriptBrowse2': 'browse',
   'bScriptEditor1': 'edit', 'bScriptEditor2': 'edit', 'mFileScriptPreview': 'NSS preview', 'xbFileZoomIn': 'DLG `PreventZoomIn` (EE)',
   'bEditText': 'node `Text` all languages', 'RightTextDelayEdit': '(hidden) node `Delay`', 'bInsertToken': 'opens TdlgTokenSelector', 'imSpeaker': 'speaker portrait',
   'mText': 'DLG node `Text` (CExoLocString)', 'cbSpeakers': 'DLG entry `Speaker` (tags of creatures in module)', 'bAddNPCTag': 'add tag to speaker list',
   'lbBookmarks': 'bookmarked nodes', 'lbSearchResults': 'search hits', 'ExportSaveDialog': 'text export',
   'bConditionEdit': 'edit condition script', 'bConditionBrowse': 'browse', 'bEditScript': 'edit action script', 'bBrowseActionScript': 'browse',
 },
 'TFRACONVERSATIONTREE': {'tvMain': 'DLG `StartingList` → `EntryList`/`ReplyList` graph (links = `IsChild`=1)'},
 'TDLGCONVERSATIONINPUT': {'TextMemo': 'new node `Text`'},
 'TDLGCONVERSATIONSEARCH': {'cbSearch': 'find text (history)', 'cbReplace': 'replace text (history)', 'rbCurrentLanguage': 'scope', 'rbAllLanguages': 'scope', 'xbMale': 'gender scope', 'xbFemale': 'gender scope',
   'xbCaseSensitive': 'option', 'xbWholeWord': 'option', 'rbCurrentFile': 'scope: current file', 'rbCurrentlyOpen': 'scope: open files', 'rbAllFiles': 'scope: all module DLGs'},
 'TDLGCONVERSATIONTEST': {'bBack': 'step back'},
 'TDLGCONVERSATIONEXPORTPICKER': {'rbStringBased': 'export mode', 'rbCharBased': 'export mode'},
 'TDLGTOKENSELECTOR': {'lbTokens': 'stringtokens.2da tokens', 'lbExamples': 'token example expansions', 'rbHighlightAction': '<StartAction>…</Start>', 'rbHighlightSkillCheck': '<StartCheck>…</Start>',
   'rbHighlight': '<StartHighlight>…</Start>', 'eHighlightAction': 'text to wrap', 'eHighlight': 'text to wrap', 'cbHighlightSkill': 'skill for [Skill] check prefix'},
 # ---- script
 'TDLGSCRIPTEDITOR': {'eFilter': 'filter side lists', 'lbFunctions': 'nwscript.nss + include functions', 'lbVariables': 'globals in current script', 'lbConstants': 'constants',
   'lbTemplates': 'script templates dir', 'pcEditor': 'one tab per open .nss', 'bOK': 'close', 'dlgPrint': 'print'},
 'TDLGSCRIPTSEARCH': {'xbBackwards': 'direction', 'xbSelectedOnly': '(hidden)', 'xbPrompt': 'confirm each replace', 'xbReplaceAll': 'replace all', 'rbFindInOpenFiles': 'find-in-files scope',
   'rbFindInAllFiles': 'find-in-files scope (module)', 'cbFind': 'find text (history)', 'cbReplace': 'replace text (history)', 'xbMatchCase': 'option', 'xbWholeWord': 'option', 'xbFindInFiles': 'switches to find-in-files panel'},
 'TFRASCRIPTEDITORCOLOR': {'lbColors': 'element: text/selected/margin/comment/directive/identifier/keyword/number/string/error (ini Color*)', 'pColor': 'current colour', 'bChangeColor': 'colour dialog', 'pScriptColorPreview': 'preview'},
 'TSEDITCODECOMPLETIONLIST': {'lbCompletionCandidates': 'matching identifiers'},
 'TDLGSCRIPTWIZARD': {'lbCondFeats_All': 'feat.2da', 'lbCondFeats_Required': 'required feats → GetHasFeat', 'lbCondGender': 'gender.2da', 'lbCondItem': 'item tags → GetItemPossessedBy',
   'eCondItem_Tag': 'item tag', 'lbCondRace_Player_Accept': 'racialtypes.2da (player races)', 'lbCondRace_Other_Accept': 'racialtypes.2da (other)', 'lbCondRace_Player_Reject': 'rejected', 'lbCondRace_Other_Reject': 'rejected',
   'lbCondSkills_All': 'skills.2da', 'lbCondSkills_Required': 'required skills → GetHasSkill', 'lbCondSkillcheck_All': 'skills.2da', 'lbCondSkillCheck_Checks': 'skill checks (skill, DC band)',
   'eActionRewards_Gold': 'GiveGoldToCreature', 'eActionRewards_XP': 'GiveXPToCreature', 'eActionRewards_Item': 'CreateItemOnObject resref', 'eActionActions_StoreTag': 'store tag → OpenStore',
   'tbActionActions_ModFaction': 'AdjustReputation amount', 'eCondRandom_1': 'Random numerator', 'eCondRandom_2': 'Random denominator', 'eActionTake_Gold': 'TakeGoldFromCreature', 'eActionTake_XP': 'SetXP (reduce)',
   'eActionTake_Item': 'item tag to take', 'rbConditionScript': 'generate `int StartingConditional()`', 'rbActionScript': 'generate `void main()`', 'eDone_ScriptName': 'output .nss resref', 'xbDone_StartEditor': 'open in Script Editor'},
 # ---- journal / faction
 'TDLGJOURNALEDITOR': {'tvMain': 'JRL `Categories[]` / `EntryList[]`', 'mCatComments': 'JRL category `Comment`', 'cbPriority': 'JRL `Priority` (0 highest…4 lowest)', 'eTag': 'JRL `Tag`',
   'bCatEdit': '`Name` all languages', 'eCatName': 'JRL `Name`', 'eXP': 'JRL `XP`', 'eEntryID': 'JRL entry `ID`', 'xbEntryFinish': 'JRL entry `End`', 'mEntryText': 'JRL entry `Text`',
   'bEntryTextEdit': '`Text` all languages', 'bApply': 'save without closing'},
 'TDLGFACTIONEDITOR': {'xbShowInverse': 'chart detail mode', 'lbBasic': 'FAC `FactionList` (check = show in chart)', 'xbBasicGlobal': 'FAC `FactionGlobal`', 'OpenGLPanel1': 'reputation chart (drag to edit)',
   'sgAdvanced': 'FAC `RepList` (`FactionID1`,`FactionID2`,`FactionRep` 0–100)', 'lbAdvanced': 'factions shown in grid', 'xbAdvGlobal': 'FAC `FactionGlobal`', 'MainStatusBar': 'status'},
 'TDLGFACTIONSELECT': {'eName': 'FAC `FactionName`', 'xbGlobal': 'FAC `FactionGlobal`', 'lbParent': 'FAC `FactionParentID` (copy reps from parent)'},
 # ---- plot
 'TDLGPLOTWIZARD': {'ePlotName': 'plot name', 'lbBasic_Templates': 'plot templates (.ptt)', 'ePlot_JournalName': 'JRL category `Name`', 'ePlot_JournalTag': '(hidden) JRL `Tag`',
   'sgCast_PlotGiver': 'plot giver creature (Name/Tag/Blueprint)', 'sgCast_Villain': 'villains', 'sgCast_Extras': 'extras', 'sgProps': 'items/placeables used', 'lbPlotNodes': 'sub-plot nodes (ordered)',
   'bSave': 'write .ptm + generated resources', 'mHelp': 'context help'},
 'TDLGPLOTNODEWIZARD': {'cbCastType': 'cast member kind', 'cbCreatures': 'cast creature', 'cbDoors': 'cast door', 'cbPlaceables': 'cast placeable', 'ePlotNodeName': 'node name', 'mComments': 'node comment',
   'cbItemKeyTags': 'key item tag', 'cbPlaceableUnlocker': 'unlocking placeable', 'cbTakeItemFromPlayer': 'item taken', 'cbGiveItemToPlayer': 'item given', 'cbLootItem': 'loot item',
   'eGoldToTakeFromPlayer': 'gold taken', 'eGoldToGiveToPlayer': 'gold given', 'mGreeting': 'generated DLG text', 'eAccept': 'generated DLG text', 'eReject': 'generated DLG text', 'mAction': 'generated DLG text',
   'cbCompletionPlotNode': 'prerequisite node', 'mJournalEntry': 'JRL entry `Text`', 'xbJournal': 'create JRL entry', 'xbEnd': 'JRL entry `End`', 'eExperience': 'XP reward'},
 'TFRAPLOTMANAGER': {'tvMain': 'plots (.ptm) and nodes'},
 'TFRAPROGRESS': {'dgMain': 'step list (current step highlighted)'},
 # ---- palette / selection
 'TFRAMAINPALETTE': {
   **{f'tvStandard{t}': f'standard {t.lower()} palette (`{p}pal.itp`, read-only)' for t, p in [('Creatures','creature'),('Doors','door'),('Encounters','encounter'),('Items','item'),('Placeables','placeable'),('Sounds','sound'),('Stores','store'),('Triggers','trigger'),('Waypoints','waypoint')]},
   **{f'tvCustom{t}': f'custom {t.lower()} palette (`{p}palcus.itp`, editable, in-place rename)' for t, p in [('Creatures','creature'),('Doors','door'),('Encounters','encounter'),('Items','item'),('Placeables','placeable'),('Sounds','sound'),('Stores','store'),('Triggers','trigger'),('Waypoints','waypoint')]},
   'tvTerrain': 'tileset Features / Groups / terrain types, Eraser, Raise/Lower', 'sbStartLocation': 'paint start location → IFO `Mod_Entry_*`', 'sbTerrain': 'terrain paint mode',
   'cbPalettes': '(hidden) old palette selector', 'xbShowPlot': 'show Plot Manager', 'eFindText': 'palette find', 'fraPlotManager1': 'plot manager'},
 'TFRABLUEPRINTSELECT': {'tvStandard': 'standard palette', 'tvCustom': 'custom palette', 'bNew': 'new blueprint (wizard)', 'bEditCopy': 'clone into custom palette'},
 'TDLGPALETTECHOOSER': {'tvPalette': 'ITP category (`PaletteID`)'},
 'TDLGRESOURCESELECTION': {'tvStandard': 'standard blueprints', 'tvCustom': 'custom blueprints'},
 'TDLGCHOOSER': {'tvDisplay': 'choice tree'},
 'TDLGDEFAULTSELECTOR': {'lbDefaults': 'choice list'},
 # ---- pickers
 'TDLGPORTRAIT': {'imgPortrait': 'large preview (po_*.tga/dds)', 'lvPortraits': 'portraits.2da thumbnails', 'cbRace': 'portraits.2da `Race` filter', 'xbGender': 'enable gender filter',
   'cbGender': 'portraits.2da `Sex` filter', 'xbRace': 'enable race filter', 'rbInanimate': 'portraits.2da `InanimateType` set', 'rbCreature': 'creature portraits',
   'xbInanimateCategory': 'enable category filter', 'cbInanimateCategory': '`InanimateType` category', 'rbPlotCharacters': '`Plot` portraits'},
 'TDLGRESOPEN': {'lvResources': 'resources (in-place rename, context menu)', 'cbResType': 'resource type', 'cbResRef': 'resource name (type-ahead)', 'rbAllResources': 'location filter',
   'rbGlobalResourcesOnly': '(hidden)', 'rbHakPakResourcesOnly': 'location filter', 'rbModuleResourcesOnly': 'location filter', 'cbResRefFilter': 'name prefix filter'},
 'TDLGRESOPENSOUND': {'bPlay': 'play WAV', 'bStop': 'stop', 'lvResources': 'WAV resources', 'cbResType': 'resource type (wav)', 'cbResRef': 'sound resref'},
 'TDLGLOADSCREEN': {'imgLoadScreen': 'large preview', 'lvLoadScreens': 'loadscreens.2da thumbnails → `LoadScreenID`', 'cbTileSet': 'loadscreens.2da tileset filter', 'xbFilterTileSet': 'enable filter', 'xbRandom': '`LoadScreenID`=0 (random)'},
 'TDLGRESTYPESELECTOR': {},
 'TDLGCOLORSELECTION': {**{f'Panel{i}': f'colour index {i} (tile light / lightcolor.2da)' for i in range(32)}},
 # ---- misc
 'TDLGLOCSTRING': {'sgStrings': 'CExoLocString entries (Language × Gender) / StrRef', 'mString': 'selected entry text', 'bEdit': 'opens TdlgNewEditStringExternal', 'bReset': 'clear entry', 'bApply': 'commit'},
 'TDLGNEWEDITSTRINGEXTERNAL': {'rbMale': 'gender', 'rbFemale': 'gender', 'cbLanguage': 'language id', 'eStrRef': 'CExoLocString `StringRef` (TLK)', 'StringMemo': 'substring text'},
 'TDLGVARTABLE': {'lvVariables': '`VarTable[]` (`Name`,`Type`,`Value`)', 'eName': 'var `Name`', 'eValue': 'var `Value`', 'cbType': 'var `Type` (1 int, 2 float, 3 string)', 'bReplace': 'update selected', 'bAdd': 'add', 'bDelete': 'delete'},
 'TDLGCOMMENTS': {'mComments': '`Comment`'},
 'TDLGIMPORTEXPORT': {'lbExportList': 'resources to export', 'bSelectForExport': 'opens resource selection', 'mMissingResources': 'missing dependencies', 'lbOverwriteCandidates': 'resources to overwrite',
   'mComments': 'ERF description (`DescriptionStrRef`/localized string list)', 'xbFactionReset': 'export: reset instance factions to parent faction'},
 'TDLGMULTISELECT': {'sgItems': 'multi-column choice', 'lbItems': 'single-column choice'},
 'TDLGHAKPAK': {'lbHakFiles': 'IFO `Mod_HakList` (same as Module Properties)', 'cbHakFile': 'hak to add', 'bReport': 'save text report', 'lvConflicts': 'resources in >1 hak',
   'lvResources': 'every resource across haks', 'lvOverrides': 'haks overriding base-game resources'},
 'TDLGOPTIONS': {
   'tvOptions': 'page selector', 'mInfo': '(hidden) option info', 'pBackgroundColor': 'ini `Background Color`', 'xbAreaShowSpawnpointMarkers': 'ini `Show Spawnpoint Markers`', 'eAreaSpawnpointMarkerHeight': 'ini `Spawnpoint Marker Height`',
   'eAreaSpawnpointMarkerWidth': 'ini `Spawnpoint Marker Width`', 'xbAreaShowDoorOrientation': 'ini `Show Door Orientation`', 'eUndo': 'ini `UndoStackSize`',
   'xbShowWelcome': 'ini `Show Welcome`', 'xbVerifyOnSave': 'ini `Verify On Save`', 'xbWarnReservedNameSpace': 'ini `NameSpace Warning`', 'xbWarnCreatureSpells': 'ini `Creature Spells`',
   'xbWarnInventory': 'ini `Inventory`', 'xbWarnColorDepth': 'ini `ColorDepth`', 'xbWarnCharacterSet': 'ini `Character Set`', 'xbUseEnvironmentMapping': 'ini `UseEnvironmentMapping`',
   'xbWarnResourceInHak': 'ini `ResourceInHak`', 'xbWarnStandardResource': 'ini `Standard Resource Warning`', 'xbCreateBackupModules': 'ini `Create Backup Modules`',
   'xbMinimizeOnTest': 'ini `MinimizeOnTest`', 'xbOpenModuleDir': 'ini `Always Open Module Directory`',
   'eScriptTemplateDir': 'ini `Templates`', 'bScriptTemplateDir': 'browse dir', 'eScriptFontName': 'ini `FontName`', 'eScriptFontSize': 'ini `FontSize`', 'bScriptFont': 'font dialog',
   'pScriptEditorColors': 'hosts TfraScriptEditorColor', 'xbAutoCompile': 'ini `AutoCompile`', 'xbScriptGenerateDebugInfo': 'ini `GenerateDebugInfo` (.ndb)',
   'bScriptExternalEditor': 'browse exe', 'eScriptExternalEditor': 'ini `ExternalEditor`',
   'ePlrDefaultDelay': '(hidden) ini `PlrDefaulDelay`', 'eNPCDefaultDelay': '(hidden) ini `NPCDefaulDelay`', 'chbTextPopup': 'ini `TextPopup`', 'chbShowName': 'ini `ShowNames`',
   'pPlrTextColor': 'ini `PlayerTextColor`', 'pNPCTextColor': 'ini `NPCTextColor`', 'rbSrcToDest': 'ini `ConvLinkPasteMode`', 'rbDestToSrc': 'ini `ConvLinkPasteMode`',
   'rbSrcToDest2': 'ini `ConvLinkLinkMode`', 'rbDestToSrc2': 'ini `ConvLinkLinkMode`', 'xbAutoBackup': 'ini `AutoBackup`', 'eAutoBackup': 'ini `AutoBackupInterval` (min)', 'pConvBackColor': '(hidden) ini `BackgroundColor`',
   'xbEnableSpellCheck': 'ini `EnableSpellCheck`', 'xbAutoSpellCheckLocEdit': 'ini `AutoSpellCheckLoc`', 'pTextColorSpellingError': 'ini `TextColorSpellingError`',
   'xbAutoSpellCheckConvEdit': 'ini `AutoSpellCheckConv`', 'xbInteractiveCheckOnEnterConvEdit': 'ini `InteractiveCheckOnEnterConv`', 'xbInteractiveCheckOnExitConvEdit': 'ini `InteractiveCheckOnExitConv`',
   'rbSpellCheckClearTempLex': 'ini `ClearIgnoreChangeAllLists`', 'rbSpellCheckKeepTempLex': 'ini `ClearIgnoreChangeAllLists`',
   'lbSound3DProviders': '(hidden) ini `3D Provider`', 'xbSoundsInArea': 'ini `Enable Area Sounds`', 'xbAreaAmbientSound': 'ini `Enable Area Ambient Sound`', 'xbAreaMusic': 'ini `Enable Area Music`',
   'tbVolumeMusic': 'ini `Ambient Music Volume`', 'tb2D3DBias': 'ini `2D3D Bias`', 'xbShowListener': 'ini `Show Listener Position`',
   'rbLanguageUseSpecified': 'ini `UseDefault`=0', 'rbLanguageDefault': 'ini `UseDefault`=1', 'lbLanguages': 'ini `LanguageID` (editing language)',
 },
 'TDLGVERIFYMODULE': {'lbResults': 'result lines (dbl-click = open offending object)', 'xbAdvanced': 'show filter panel', 'sdResults': 'export results'},
 'TDLGWARNING': {'xbNoDisplay': 'suppress this warning (ini)'},
 'TDLGCONFIRMATION': {'xbRememberResponse': 'remember answer for this session'},
 'TDLGPROGRESS': {'pbTask': 'current task', 'pbTotalProgress': 'overall'},
 'TDLGABOUT': {'mLicense': 'EULA text', 'mExpansionPacks': 'installed expansions'},
 'TFRMHELP': {'mHelp': 'TLK help text of focused control'},
 'TDLGWELCOME': {'rbCreateNew': 'Module Wizard / new module', 'rbOpenExisting': 'open selected', 'lbModuleFiles': 'modules/*.mod', 'xbShowAtStartup': 'ini `Show Welcome`', 'rbDoNothing': 'start with no module'},
 'TDLGMODULESELECT': {'lbModules': 'modules list', 'eModuleName': 'module filename', 'rbNormal': 'modules/ folder', 'rbOfficial': 'data/nwm official modules'},
}

# per-control extra notes (prefixed into Notes column)
NOTE = {
 'TFRMFRAME': {'sbUndo': 'enabled when viewer has undo', 'sbRedo': 'enabled when viewer has redo'},
 'TDLGCREATUREEDIT': {'Button1': 'debug button', 'bRestoreDefaults': 'dead'},
 'TDLGSOUNDEDIT': {'lbSounds': 'Del key removes'},
}

# ---------------------------------------------------------------- form-level notes
NOTES = {
 'TFRMFRAME': [
   'Menu captions come from the Actions (`Action=` property); runtime text from TLK (e.g. *View › Toolbars › Object Filters*, *Interface Panels › Palettes*).',
   'The area viewer merges its own `Edit` (GroupIndex 20) and `Scene` (30) menus; frame menus use GroupIndex 90 so they stay.',
   'Disabled-by-default items (`Undo`, `Redo`, `Resize/Rotate Area`, `Find Instance`, `Module/Area Properties`, render toggles, `Verify Area`, `Area Statistics`) are enabled when a module/area is open.',
   'Hidden/dead: `File › Save To Savegame` (`actSaveToSavegame` + `sdSaveToSavegame`), `Wizards › Module Wizard` (visible via Welcome dialog), `Window` menu, `actComputeStaticLighting`, `actWindowSpeed`, `actAreaImport`, `actTemplateDelete`, `actCloseSilent`.',
   'Toolbars live in a `TControlBar` (drag to re-dock); visibility via View › Toolbars and toolbar popup `pmToolbars`; state persisted in nwtoolset.ini (`FileVisible`, `FileDocked`, `FileTop`…).',
   'Panes: Module Contents (left), Palettes (right), Message Log (bottom), Visual Camera Controls (in viewer); widths persisted (`ContentsWidth`, `PaletteWidth`, `MessagesHeight`).',
   '`pArea` hosts area viewers as tabs (EE); `pmEditorTab` (Close) on tab right-click; ini `Disable Tabs` reverts to single area.',
   'Form handles `KeyPreview` + `FormKeyDown/Up` (camera keys, F10 toggle) and mouse-wheel (zoom) for the active viewer.',
   '`miTestModule` (F9) has no action: saves then launches the game (see implicit behaviors). `Help › Neverwinter Nights Website` opens http://nwn.beamdog.com.',
 ],
 'TFRAMAININVENTORY': [
   'Context menu is filtered per node type (area node vs object node vs resource node). *Delete* removes areas/resources (not undoable).',
   'Area nodes lazily populate object children (`tvMainExpanding`), grouped by type; labels = name (creatures/items/placeables) or tag.',
   'Double-click (`actGotoExecute`) opens the area tab or focuses camera on an object. *Build* compiles/validates the node (e.g. scripts, area).',
   'In-place label editing (`tvMainEditing/Edited`) renames resources.',
 ],
 'TFRMVIEWERAREA': [
   'Context menu items are shown/hidden per clicked object type in `pmViewerAreaPopup`: tiles → Tile Properties; creatures → Create Waypoint, Conversation, Inventory, Levelup Wizard, Setup Store (for merchants), Add Popup Text; doors → Reverse Door, Initial State (door states); placeables → Initial State (placeable states), Inventory; triggers/encounters → Redraw Polygon, Add Spawn Point; waypoints → Create Set; sounds → Mute / Turn On; all → Add To Palette, Adjust Location, Delete, Variables, Properties.',
   '`pCameraControls` buttons act while held (`MouseDown`/`MouseUp`, also keyboard repeat).',
   '`AreaMenu.Edit.Copy1` has no handler here: copy is routed through the frame action `actCopy`.',
 ],
 'TFRMPREVIEW': ['One property tab per object type is shown (tabs hidden); `pCommon` shows Name/Tag/ResRef/Comments for all. Mouse on `apPreview` rotates/zooms the model; items can switch 2D icon / 3D model.'],
 'TFRMIFOPROP': [
   'Split name/value panel layout; start location fields are informational (paint Start Location in an area to change).',
   'Event list includes EE events (OnPlayerChat, OnPlayerTarget, OnPlayerGuiEvent, OnPlayerTileAction, OnNuiEvent); OnModuleStart (`Mod_OnModStart`) row is hidden.',
   'XP Scale slider and edit mirror each other (0–200). Start-year/month/day/hour and dawn/dusk validated on exit.',
   '*Custom Content* tab: ordered hak list (top overrides lower), add from dropdown of available .hak files, move up/down, remove, conflict analysis, custom TLK dropdown (tlk folder).',
   'Not exposed in UI but written by the toolset: `Mod_ID`, `Mod_UUID`, `Mod_Creator_ID`, `Mod_Version`, `Mod_MinGameVer`, `Expansion_Pack`, `Mod_Area_list`, `Mod_CacheNSSList`, `Mod_Expan_List`, `Mod_CutSceneList`, `Mod_GVar_List`, `Mod_PartyControl`, `Mod_DefaultBic`.',
 ],
 'TDLGAREAPROPERTIES': [
   'Tileset and dimensions are likely only editable for a new area (`pHideAreaDimensions` panel covers them otherwise); resizing an existing area uses Resize Area.',
   'Visual tab: lighting-scheme presets (environment.2da) + *Customize Environment* sub-dialog; warning memo (TLK): selecting a scheme discards custom area lighting/weather **and custom tile lighting**.',
   '`m_bLoadDefault` / hidden `m_bSaveDefault`: area-property defaults template. `bApply` applies to the open viewer without closing.',
   'Hidden/dead: World Map icon/X/Y, Time.',
 ],
 'TDLGENVIRONMENT': ['Colour swatches open the Windows colour dialog (`actBrowseColorExecute`); fog amount sliders are hidden (EE uses numeric edits 0–200 and Fog Clip Distance 25–999 m). `Apply` previews in the viewer.'],
 'TFRMAREARESIZE': ['Resize anchors at the south-west tile; shrinking deletes objects on removed tiles; growing repeats edge tiles. Rotation is loss-less.'],
 'TDLGTILEPROPERTIES': ['Colour swatches open `TdlgColorSelection` (fixed tile light palette). Applies to all selected tiles.'],
 'TDLGAREATRANSITION': ['`apArea` renders the target area top-down with doors/triggers/waypoints; clicking an object selects it in the lists. Status label under the map. Linking writes `LinkedTo` = target tag and `LinkedToFlags` on source (and target when *Both Ways*).'],
 'TDLGLOCATION': ['Keyboard up/down on edits nudges values (`OnEditUpDownKeyDown`). Applies to all selected instances. Visual Transform group has no Tags (EE, English only).'],
 'TDLGSITUATEDEDIT': ['Tab set shared by doors/placeables; frames: Basic=`TfraSituatedBasic`, Lock=`TfraSituatedLock`, Trap=`xbTrapFlag` + `TfrmTrap` created into `pTrap` at runtime, Events=`TfraSituatedScripts`, Advanced=`TfraSituatedAdvanced`, Description=`TfraSituatedDesc`, Comments=`TfraSituatedComments`. 3D preview (`apAppearance`) supports mouse rotate/zoom.'],
 'TDLGSITUATEDMULTIEDITOR': ['Shows a subset of each frame (Basic: name/tag/appearance; Advanced: faction/conversation/portrait; all scripts; description; lock; comments). Fields left blank/indeterminate are not written.'],
 'TFRASITUATEDSCRIPTS': ['Panels are shown per object type: doors hide OnInvDisturbed/OnUsed; placeables hide OnFailToOpen (EE shows OnClick for both).'],
 'TFRASITUATEDBASIC': ['Hardness hidden in the frame; shown by TdlgSituatedEdit. Saves/HP validated on exit.'],
 'TFRASITUATEDADVANCED': ['Template ResRef read-only for instances; `cbOpenState` items depend on object type (door: open/closed; placeable: default/open/closed/destroyed/activated/deactivated).'],
 'TDLGCREATUREEDIT': [
   'Right side: live 3D preview (`apCreature`: drag rotate, wheel zoom); bottom buttons: Inventory, OK, Cancel.',
   'Statistics recompute on every change (`OnAbilityDependentKeyUp`): ability totals include racial adjustments; AC = 10 + dex + size + natural; saves = class tables + ability + bonus; HP total = base + Con bonus × levels.',
   'Challenge Rating = calculated (classes, HD, abilities, feats/specials, `fractionalcr.2da`) + CR Adjustment; shown on Basic and Advanced tabs.',
   'Classes: up to 8 (EE) with levels 1–60; `Package` drives Levelup Wizard auto-picks.',
   'Skills grid: editable ranks, edit mask numeric; Feats grid: owner-drawn check column + filter (all / assigned / available); Spells: per class radio, per level list with Prepared/Name/Level/Caster-level columns, metamagic combo, save/load/clear class spell list; Special Abilities grid (spell, caster level, uses) with filter.',
   'Template tab (`crtemplates.2da`: Half-Dragon, Lich, Vampire, Half-Fiend…) — applying modifies stats/feats/abilities.',
   'Hidden/dead: Position/Orientation groups, `Restore`, `Button1`, `dlgOpenFile`, `bUpdateInstancesInArea` (shown in blueprint mode).',
 ],
 'TDLGCREATUREWIZARD': ['Implemented with panels (not tabs) switched by Next/Back; wizard auto-derives tag/resref, default portrait by race/gender, abilities by race+class, and calls the level-up logic to fill feats/skills/spells.'],
 'TFRMCREATURELEVELUPWIZARD': ['Label says "1 to 3 classes" (legacy) but 8 class slots exist.'],
 'TDLGINVENTORY': [
   'Mode-dependent: creature (Standard Equipment + Natural Equipment tabs, Dropable/Pickpocketable), placeable (contents only), store (category tabs + Infinite + price preview "Store buys/sells/BlackMarket buys for").',
   'Drag & drop from palette trees to slots/grid; drag to trash image deletes; double-click equips/edits; popup: Edit / Edit Copy on custom items.',
   'Equip slot validation against baseitems.2da `EquipableSlots`; warnings for invalid inventory (Options: creature inventory warning).',
 ],
 'TDLGITEMEDIT': [
   'Appearance tab switches panel by base item `ModelType` (0 simple → `lvSimple` icon grid; 1 layered/2 composite weapon → top/middle/bottom model+colour; 3 armour → 18 part combos + robe). Colour button for PLT-coloured items.',
   'Properties tab: available tree filtered by base item (itemprops.2da column), assigned tree; double-click adds/edits; cost recalculated (`meCost2`).',
   'Description tab: unidentified/identified descriptions + read-only base-type description and generated statistics text.',
   'Stack size spin validated against baseitems.2da `Stacking`; charges 0–250.',
 ],
 'TDLGPLACEABLEEDIT': ['Category is a captioned panel + read-only edit; `xbInventory` enables the Inventory button; `xbStatic` disables most scripts/interaction; Treasure Model only when HasInventory.'],
 'TDLGDOOREDIT': ['Appearance = tileset door type (doortypes.2da) *or* Generic Appearance (genericdoors.2da, `GenericType_New`); Area Transition tab like triggers.'],
 'TDLGTRIGGEREDIT': ['Property-sheet layout with draggable splitters (`splitProperties`, `splitEvent`); Area Transition tab visible for type Area Transition; Trap tab hosts `TfrmTrap` for type Trap. `cbTriggerType` items in DFM are placeholders; runtime list = Generic / Area Transition / Trap. `cbFaction`/`cbCursor` placeholder items false/true replaced at runtime.'],
 'TDLGENCOUNTEREDIT': ['Creature List tab hosts `TfraEncounterCreatureList` in `pCreatureListFrame`. Encounter geometry & spawn points are edited in the area viewer (Redraw Polygon / Add Spawn Point).'],
 'TFRAENCOUNTERCREATURELIST': ['Grid columns set at runtime (creature name, CR, Unique). Double-click palette = add; double-click grid = remove; popup on palette: Add / Edit Creature / Edit Copy.'],
 'TDLGSOUNDEDIT': ['Volume/pitch/volume-variation sliders with min/mid/max tick labels; radii diagram images; hour checkboxes enabled only for *Specific Hours*; interval edits disabled for Looping. Priority (`Priority`) is derived from prioritygroups.2da (not shown). Legacy hidden "play options" radio groups remain.'],
 'TDLGSTOREEDIT': ['Mark-up/down spinners use `OnChangingEx` validation; black-market mark-down enabled only with *Buy Stolen Goods*; Restrictions tab moves base items between a base-item chooser (`pBaseItemChooser`, runtime tree) and the restricted list.'],
 'TDLGWAYPOINTEDIT': ['Map-note edits enabled only when *Waypoint Contains a Map Note* is checked.'],
 'TDLGCONVERSATIONEDITOR': [
   'Top: toolbar (file + filters), tabbed trees (`pcMainText`: one tab per open DLG + Scrap); bottom: node text (speaker combo, `mText`, `...` languages, token button, portrait) and Data tabs (Text Appears When… / Actions Taken / Other Actions / Comments / Current File), plus Bookmarks and Search result tabs.',
   'Condition/action script panes show a read-only preview of the NSS source. EE parameter grids (`TValueListEditor`) with +/− add key/value pairs passed to scripts.',
   'Speaker combo lists creature tags in the module; `Add` stores extra tags. Quest combo lists journal categories, entry combo lists entries.',
   'Hidden: `xbOnceOnly`, `RightTextDelayEdit`, `xbAnimLoop`, `lUnique`; actions without UI: `actExportDialog`, `actImportDialog`, `actTest`, `actRemoveNPCTagFromList`.',
   'Conditional popup `pmConditional` → *Unique Script* (generate a node-specific script name).',
 ],
 'TFRACONVERSATIONTREE': ['Custom-drawn nodes (colours from Options; link nodes grey; highlight filters); drag & drop move/link (`tvMainDragDrop`); timer `tMain` for auto-backup/scroll. Keyboard: Ctrl+A add, Ctrl+C/X/V, Delete, arrows.'],
 'TDLGSCRIPTEDITOR': [
   'Editor component is a custom syntax-highlighting memo (`TSEdit…`, completion list `TSEditCodeCompletionList`); tabs in `pcEditor`; `sHorizontalSplitter` between editor and info pane.',
   'Bottom info tabs are panels filled at runtime: Compiler (errors, double-click jumps to line), Help (doc comment of selected function), Bookmarks, Search Results.',
   'Side lists: double-click inserts identifier/template; hover shows prototype (`ShortcutListMouseMove`).',
   'Keyboard (from nwn.wiki): F2 completion, Ctrl+Shift+1…9 set / Ctrl+1…9 jump bookmark, Tab/Shift+Tab indent, Ctrl+Y redo, Ctrl+A select all.',
 ],
 'TDLGSCRIPTWIZARD': ['Condition steps: abilities (op + value), class/level (any/specific, allow any/only), feats, gender, items in inventory (tags), local variables (lhs type/name op rhs type/value), race (player/other accept/reject), skills, skill checks (easy/medium/hard), random (x in y), alignment. Action steps: rewards (gold/XP/items, to party), set locals, actions (open store by tag with appraise, attack, adjust faction −100…100), take (gold/XP/items; destroy/keep). Done: script name + open editor.'],
 'TDLGJOURNALEDITOR': ['Tree of categories → entries; tab sheet shown per selected node type (category vs entry). Edits save back to the tree on change (`actSaveFromControls`). Priority list: Highest/High/Medium/Low/Lowest.'],
 'TDLGFACTIONEDITOR': ['Basic tab: check factions to plot their mutual reputations in the OpenGL chart (mouse handlers present). Advanced tab: editable N×N grid (row = faction feeling about column). The standard factions (PC, Hostile, Commoner, Merchant, Defender) cannot be removed.'],
 'TDLGPLOTWIZARD': ['Uses `TfraProgress` step strip; Next/Back navigate; `mHelp` shows context help per step; cast grids with New/Edit/Browse/Delete; "Edit non-plot-related Conversation".'],
 'TDLGPLOTNODEWIZARD': ['Pages shown depend on plot type (conversation / villain / open object) and conversation type (quest / exchange items / single statement); first use of a cast member asks for Plotless/Undefined greetings (`tsConvOther`). Generates DLG nodes, scripts and journal entries.'],
 'TFRAMAINPALETTE': [
   'Top: paint-mode toggle buttons (one per blueprint type + Terrain + Start Location) switch the visible palette tree; Standard/Custom tabs; bottom: Plot Manager (toggle *Show Plot*).',
   'Standard trees read-only; custom trees support in-place rename, Del key, popup (Edit, Edit Copy, New, Delete, Update Instances, hidden Export/Import/Restore Default, Find Text Ctrl+F, Find Next F3, Refresh Palette).',
   'Selecting an item arms placement in the viewer (`tvChange`); `pFind` find bar appears on Ctrl+F.',
 ],
 'TDLGRESOPEN': ['Location filter: module / hak paks / all (global hidden). List view in-place rename (module resources only); popup Delete (and hidden Export…). Name filter combo with prefixes. Count label.'],
 'TDLGPORTRAIT': ['Thumbnails generated from portraits.2da `BaseResRef` + size suffix; filters: creature vs inanimate (placeables/doors) vs plot characters, race, gender, inanimate category.'],
 'TDLGLOADSCREEN': ['Thumbnails of loadscreens.2da entries; *Use Random Loading Screen* stores 0.'],
 'TDLGIMPORTEXPORT': ['Wizard-like pages: Export list → (Missing resources prompt) → Overwrite candidates (select all/none) → Comments (+ reset factions). Import = choose .erf, resolve overwrites.'],
 'TDLGOPTIONS': ['Tree (`tvOptions`) selects hidden tab sheets: Area, General, Script Editor, Conversation Editor, Spell Check, Sound, Language. Undo levels change requires restart.'],
 'TDLGVERIFYMODULE': ['Build = compile scripts (+ unused), recompute creature CR, encounter CRs, rebuild palettes, report missing resources by type, unused resources, spell check. Results list exportable to .txt; double-click opens the offending object. Filter checkboxes enabled in groups (`actEnableFiltersExecute`).'],
 'TDLGHAKPAK': ['Three report list views (Resource / Type / Hak Paks containing resource), sortable by column; *Report Resources and Conflicts…* saves a text report.'],
 'TDLGWELCOME': ['Shown at startup unless disabled; module list from modules folder.'],
}

# ---------------------------------------------------------------- sections
SECTIONS = [
 {'title': '1. Main frame, menus, toolbars, actions', 'forms': ['TFRMFRAME', 'TFRAMAININVENTORY', 'TFRMVIEWER', 'TFRMPROPERTIES']},
 {'title': '2. Module properties & module wizard', 'forms': ['TFRMIFOPROP', 'TDLGMODULEWIZARD']},
 {'title': '3. Area editor, area properties, area wizard, resize/rotate, tiles, transitions, placement', 'forms': [
   'TFRMVIEWERAREA', 'TDLGAREAPROPERTIES', 'TDLGENVIRONMENT', 'TDLGAREAWIZARD', 'TFRMAREARESIZE', 'TDLGTILEPROPERTIES', 'TDLGCOLORSELECTION',
   'TDLGAREATRANSITION', 'TDLGLOCATION', 'TDLGADDPOPUPTEXT', 'TDLGFINDINSTANCE', 'TDLGSYSTEMUSAGE', 'TFRMPREVIEW', 'TFRMOBJECTLIST']},
 {'title': '4. Shared "situated" frames and dialogs (doors, placeables, multi-edit, traps)', 'forms': [
   'TDLGSITUATEDEDIT', 'TFRASITUATEDBASIC', 'TFRASITUATEDLOCK', 'TFRMTRAP', 'TFRASITUATEDTRAP', 'TFRASITUATEDSCRIPTS', 'TFRASITUATEDADVANCED', 'TFRASITUATEDDESC', 'TFRASITUATEDCOMMENTS', 'TDLGSITUATEDMULTIEDITOR']},
 {'title': '5. Creature editor (+ wizard, level-up, colours, sound set, inventory)', 'forms': [
   'TDLGCREATUREEDIT', 'TDLGCREATUREWIZARD', 'TFRMCREATURELEVELUPWIZARD', 'TCOLORPICKER', 'TDLGSOUNDSETSELECT', 'TDLGINVENTORY']},
 {'title': '6. Item editor (+ property editor, wizard, generators)', 'forms': ['TDLGITEMEDIT', 'TDLGPROPEDIT', 'TDLGITEMWIZARD', 'TDLGITEMGENERATOREDIT', 'TDLGGENERATORCHOOSER']},
 {'title': '7. Placeable editor', 'forms': ['TDLGPLACEABLEEDIT', 'TDLGPLACEABLEWIZARD']},
 {'title': '8. Door editor', 'forms': ['TDLGDOOREDIT', 'TDLGDOORWIZARD']},
 {'title': '9. Trigger editor', 'forms': ['TDLGTRIGGEREDIT', 'TDLGTRIGGERWIZARD']},
 {'title': '10. Encounter editor', 'forms': ['TDLGENCOUNTEREDIT', 'TFRAENCOUNTERCREATURELIST', 'TDLGENCOUNTERWIZARD']},
 {'title': '11. Sound editor', 'forms': ['TDLGSOUNDEDIT', 'TDLGSOUNDWIZARD']},
 {'title': '12. Store (merchant) editor', 'forms': ['TDLGSTOREEDIT', 'TDLGSTOREWIZARD', 'TDLGSTORESETUPWIZARD']},
 {'title': '13. Waypoint editor', 'forms': ['TDLGWAYPOINTEDIT', 'TDLGWAYPOINTWIZARD']},
 {'title': '14. Wizard base classes', 'forms': ['TDLGWIZARD', 'TDLGBLUEPRINTWIZARD']},
 {'title': '15. Conversation editor', 'forms': ['TDLGCONVERSATIONEDITOR', 'TFRACONVERSATIONTREE', 'TDLGCONVERSATIONINPUT', 'TDLGCONVERSATIONSEARCH', 'TDLGCONVERSATIONTEST', 'TDLGCONVERSATIONEXPORTPICKER', 'TDLGTOKENSELECTOR']},
 {'title': '16. Script editor & script wizard', 'forms': ['TDLGSCRIPTEDITOR', 'TDLGSCRIPTSEARCH', 'TFRASCRIPTEDITORCOLOR', 'TSEDITCODECOMPLETIONLIST', 'TDLGSCRIPTWIZARD']},
 {'title': '17. Journal editor', 'forms': ['TDLGJOURNALEDITOR']},
 {'title': '18. Faction editor', 'forms': ['TDLGFACTIONEDITOR', 'TDLGFACTIONSELECT']},
 {'title': '19. Plot wizard & plot manager', 'forms': ['TDLGPLOTWIZARD', 'TDLGPLOTNODEWIZARD', 'TFRAPLOTMANAGER', 'TFRAPROGRESS']},
 {'title': '20. Palettes & blueprint selection', 'forms': ['TFRAMAINPALETTE', 'TFRABLUEPRINTSELECT', 'TDLGPALETTECHOOSER', 'TDLGRESOURCESELECTION', 'TDLGCHOOSER', 'TDLGDEFAULTSELECTOR']},
 {'title': '21. Resource pickers (portrait, sound, sound set, load screen, resource open)', 'forms': ['TDLGPORTRAIT', 'TDLGRESOPEN', 'TDLGRESOPENSOUND', 'TDLGLOADSCREEN', 'TDLGRESTYPESELECTOR'],
  'intro': 'Sound-set picker (`TdlgSoundSetSelect`) is in §5, colour pickers in §3/§5.'},
 {'title': '22. Localised strings, variables, comments', 'forms': ['TDLGLOCSTRING', 'TDLGNEWEDITSTRINGEXTERNAL', 'TDLGVARTABLE', 'TDLGCOMMENTS']},
 {'title': '23. Import / export', 'forms': ['TDLGIMPORTEXPORT', 'TDLGMULTISELECT']},
 {'title': '24. Hak paks & custom TLK', 'forms': ['TDLGHAKPAK'],
  'intro': 'Hak list and custom TLK are edited in Module Properties › Custom Content (§2); this dialog is the conflict analyser.'},
 {'title': '25. Options', 'forms': ['TDLGOPTIONS']},
 {'title': '26. Build / verify / test module', 'forms': ['TDLGVERIFYMODULE'],
  'intro': 'Test Module (F9) has no dialog — see §1 notes and the implicit-behaviors section.'},
 {'title': '27. Misc dialogs (startup, about, help, progress, warnings)', 'forms': ['TDLGWELCOME', 'TDLGMODULESELECT', 'TDLGABOUT', 'TFRMHELP', 'TDLGPROGRESS', 'TDLGWARNING', 'TDLGCONFIRMATION']},
]

# ---------------------------------------------------------------- manual label overrides (geometry pairing fails)
_AB = [('Strength','Str'),('Dexterity','Dex'),('Constitution','Con'),('Intelligence','Int'),('Wisdom','Wis'),('Charisma','Cha')]
LABEL = {
 'TDLGCREATUREEDIT': {
   **{f'e{a}Bonus': f'{a} — Bonus' for a, _ in _AB},
   **{f'eRacial{b}Mod': f'{a} — Racial Modifier' for a, b in _AB},
   **{f'e{b}Total': f'{a} — Total' for a, b in _AB},
   'eSaveFortitude': 'Fortitude — Base', 'eSaveReflex': 'Reflex — Base', 'eSaveWill': 'Will — Base',
   'eSaveFortitudeBonus': 'Fortitude — Bonus', 'eSaveReflexBonus': 'Reflex — Bonus', 'eSaveWillBonus': 'Will — Bonus',
   'eFortModifier': 'Fortitude — Racial/Ability Modifier', 'eReflexModifier': 'Reflex — Racial/Ability Modifier', 'eWillModifier': 'Will — Racial/Ability Modifier',
   'eSaveFortitudeTotal': 'Fortitude — Total', 'eSaveReflexTotal': 'Reflex — Total', 'eSaveWillTotal': 'Will — Total',
   **{f'cbClass{i}': f'Class {i}' for i in range(1, 9)}, **{f'eClass{i}Level': f'Class {i} level' for i in range(1, 9)},
   'cbFaction': '⟨Faction⟩', 'eSoundSet': '⟨Sound Set⟩', 'cbPerceptionRange': '⟨Perception Range⟩', 'cbConversation': '⟨Conversation⟩', 'ePortrait': '⟨Portrait⟩',
 },
 'TDLGCREATUREWIZARD': {**{f'eClass{i}': f'Class {i}' for i in range(1, 9)}, **{f'eClass{i}Level': f'Class {i} level' for i in range(1, 9)}},
 'TFRMCREATURELEVELUPWIZARD': {**{f'eClass{i}': f'Class {i}' for i in range(1, 9)}, **{f'eClass{i}Level': f'Class {i} level' for i in range(1, 9)}},
 'TDLGPLACEABLEEDIT': {'ePaletteCategory': 'Category', 'cbTreasurePile': 'Treasure Model'},
 'TDLGDOOREDIT': {'ePaletteCategory': 'Category', 'cbGenericType': 'Generic Appearance', 'eSecretDoorDC': 'Secret Door Detection DC'},
 'TDLGSCRIPTWIZARD': {
   **{f'cbCondAbility_{b}_Operator': f'{a} operator (> < =)' for a, b in _AB}, **{f'eCondAbility_{b}_Value': f'{a} value' for a, b in _AB},
   'cbCondClass_Classes': 'Class', 'sgCondClass_All': 'Class restrictions (class, level)', 'eCondClass_Level': 'Specific level',
   'lbCondLocal': 'Local Expressions', 'cbCondLocal_LHandType': 'Variable type (int/float/string)', 'eCondLocal_LHand': 'Variable name',
   'cbCondLocal_Operator': 'Operator', 'cbCondLocal_RHandType': 'Value type', 'eCondLocal_RHand': 'Value',
   'lbActionRewards': 'Rewards list', 'eActionLocal_Name': 'Variable name', 'eActionLocal_Value': 'Value', 'lbActionLocal': 'Local Expressions',
   'lbActionTake_Item': 'Items to take', 'cbActionLocal_LHandType': 'Set local variable (type)',
 },
 'TDLGCONVERSATIONEDITOR': {'mText': 'Text', 'cbQuestEntry': 'Journal entry', 'mComment': '⟨Comments⟩'},
 'TDLGPLOTNODEWIZARD': {'ePlotNodeTag': 'Plot Node Tag'},
}

DEFAULT_MAP = {
 'TDLGSCRIPTWIZARD': 'script-generator input (emits NSS)',
 'TDLGPLOTNODEWIZARD': 'plot-generator input (DLG/NSS/JRL)',
 'TDLGVERIFYMODULE': 'build/verify pass option',
 'TDLGCREATUREWIZARD': 'new UTC `ClassList[n]`',
 'TFRMCREATURELEVELUPWIZARD': 'classes/levels to add',
 'TDLGPLOTWIZARD': 'plot-generator input',
 'TDLGPLACEABLEWIZARD': '`PaletteID`', 'TDLGENCOUNTERWIZARD': '`PaletteID` / wizard option', 'TDLGSOUNDWIZARD': '`PaletteID` / wizard option',
 'TDLGRESOPENSOUND': 'location filter',
}
LABEL['TDLGSOUNDEDIT'] = {f'xb{h}{ap}': f'{h} {ap}' for h in range(1, 13) for ap in ('AM', 'PM')}

MAP['TDLGPLACEABLEEDIT'].update({'xbTrapFlag': 'UTP `TrapFlag`', 'pTrap': 'hosts `TfrmTrap` (runtime)', 'apAppearance': 'model preview'})
MAP['TDLGDOOREDIT'].update({'xbTrapFlag': 'UTD `TrapFlag`', 'pTrap': 'hosts `TfrmTrap` (runtime)', 'apAppearance': 'model preview'})
MAP['TDLGSITUATEDEDIT'].update({'pTrap': 'hosts `TfrmTrap` (runtime)'})

SHORTCUT_NAMES = {'actSelectToggle': 'Toggle Select Terrain / Select Objects'}

TABNAME = {
 'TDLGOPTIONS': {'tsArea': 'Area', 'tsGeneral': 'General', 'tsScriptEditor': 'Script Editor', 'tsConvEditor': 'Conversation Editor',
                 'tsSpellCheck': 'Spell Checking', 'tsSound': 'Sounds', 'tsLanguage': 'Language'},
 'TDLGIMPORTEXPORT': {'tsExportList': 'Export list', 'tsMissing': 'Missing resources', 'tsOverwrite': 'Overwrite', 'tsComments': 'Comments'},
 'TDLGJOURNALEDITOR': {'tsCategory': 'Category (node selected)', 'tsEntry': 'Entry (node selected)'},
 'TFRMPREVIEW': {'tsCreature': 'Creature', 'tsDoor': 'Door', 'tsEncounter': 'Encounter', 'tsItem': 'Item', 'tsPlaceable': 'Placeable',
                 'tsSound': 'Sound', 'tsStore': 'Store', 'tsTrigger': 'Trigger', 'tsWaypoint': 'Waypoint', 'tsTerrain': 'Terrain'},
}
LABEL['TDLGOPTIONS'] = {'pBackgroundColor': 'Background Color (click panel to change)', 'pPlrTextColor': 'Player Text Color', 'pNPCTextColor': 'NPC Text Color',
  'pConvBackColor': 'Background colour', 'eAutoBackup': 'Backup interval (minutes)', 'pScriptEditorColors': 'Colours', 'eScriptFontName': 'Font (name)',
  'eScriptFontSize': 'Font (size)', 'ePlrDefaultDelay': 'Default delay for player speech', 'eNPCDefaultDelay': 'Default delay for NPC speech',
  'pTextColorSpellingError': 'Color of Text with Spelling Errors', 'pTextColorGrammaticalError': 'Color of Text with Grammatical Errors',
  'pTextColorSpellingAndGrammaticalError': 'Color of Text with both Spelling and Grammatical Errors', 'tbVolumeAmbientSound': 'Ambient sound volume',
  'tb2D3DBias': '2D/3D Bias (2D … 3D)', 'mLanguageHelp': 'help text', 'mLocEditSpellCheckOptions': 'info text'}

MAP['TDLGOPTIONS'].update({'xbCheckSpellingLocEdit': '(hidden) ini `CheckSpellingLoc`', 'xbCheckGrammarLocEdit': '(hidden) ini `CheckGrammarLoc`',
  'xbInteractiveCheckOnEnterLocEdit': '(hidden) ini `InteractiveCheckOnEnterLoc`', 'xbInteractiveCheckOnExitLocEdit': '(hidden) ini `InteractiveCheckOnExitLoc`',
  'pTextColorGrammaticalError': '(hidden) ini `TextColorGrammaticalError`', 'pTextColorSpellingAndGrammaticalError': '(hidden) ini `TextColorSpellingAndGrammaticalError`',
  'mLocEditSpellCheckOptions': 'info text', 'xbCheckSpellingConvEdit': '(hidden) ini `CheckSpellingConv`', 'xbCheckGrammarConvEdit': '(hidden) ini `CheckGrammarConv`',
  'tbVolumeAmbientSound': '(hidden) ambient sound volume', 'mLanguageHelp': 'help text (read-only)', 'dlgColor': 'colour picker', 'dlgFileOpen': 'file/dir picker', 'dlgFont': 'font picker'})
TRAILER += r"""
## Provenance / regeneration

Working files (scratchpad `uiinv/`): `forms.json` (parsed TPF0 trees), `rep/*.txt` (condensed per-form dumps),
`locbases.json` (StrRef table offsets), `tools/dfmparse.py` (binary DFM → tree), `tools/matchloc.py` (table voting),
`tools/loccheck.py` (per-form DFM-vs-TLK comparison), `tools/gen.py` + `tools/mapping.py` (this document; GFF mappings
and notes are hand-authored in `mapping.py`). Regenerate: `python3 uiinv/tools/gen.py research/aurora_ui_inventory.md`.
GFF field names were checked against base-game blueprints (`nw_*.ut?`), the Prelude module (`module.ifo`, `.are`,
`.git`, `.jrl`, `.dlg`) and field-name strings in the exe (e.g. `Mod_OnPlrTileAct`, `Mod_UUID`, `VisTransformList`).
"""
