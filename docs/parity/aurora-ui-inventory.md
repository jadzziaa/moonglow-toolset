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
   The StrRef table was resolved for 83 of 105 forms (located by voting DFM caption ↔ TLK text over candidate table offsets, ambiguous ones verified by
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

## Form index

| § | Class | Runtime title | DFM | Size | Controls (approx.) |
|---|---|---|---|---|---|
| 1 | `TfrmFrame` | BioWare Aurora Neverwinter Nights Toolset v%s.%s.%s-%s | TFRMFRAME | 859×596 | 35 |
| 1 | `TfraMainInventory` |  | TFRAMAININVENTORY |  | 1 |
| 1 | `TfrmViewer` | frmViewer | TFRMVIEWER |  | 0 |
| 1 | `TfrmProperties` | Properties | TFRMPROPERTIES | 280×376 | 2 |
| 2 | `TfrmIFOProp` | Module Properties | TFRMIFOPROP | 330×527 | 108 |
| 2 | `TdlgModuleWizard` | Module Wizard | TDLGMODULEWIZARD | 328× | 9 |
| 3 | `TfrmViewerArea` |  | TFRMVIEWERAREA | 619×494 | 14 |
| 3 | `TdlgAreaProperties` | Area Properties | TDLGAREAPROPERTIES | 543×515 | 65 |
| 3 | `TdlgEnvironment` | Environment Options | TDLGENVIRONMENT | 395×460 | 26 |
| 3 | `TdlgAreaWizard` | Area Wizard | TDLGAREAWIZARD | 366×387 | 14 |
| 3 | `TfrmAreaResize` | Resize Area | TFRMAREARESIZE | 338×250 | 13 |
| 3 | `TdlgTileProperties` | Tile Properties | TDLGTILEPROPERTIES | 306×193 | 6 |
| 3 | `TdlgColorSelection` | Select A Color | TDLGCOLORSELECTION | 350×174 | 2 |
| 3 | `TdlgAreaTransition` | Area Transition | TDLGAREATRANSITION |  | 13 |
| 3 | `TdlgLocation` | Adjust Location | TDLGLOCATION | 434×303 | 25 |
| 3 | `TdlgAddPopupText` | Add Popup Text | TDLGADDPOPUPTEXT | 306×239 | 5 |
| 3 | `TdlgFindInstance` | Find Instance | TDLGFINDINSTANCE | 412×371 | 8 |
| 3 | `TdlgSystemUsage` | Resources Used | TDLGSYSTEMUSAGE | 133×93 | 1 |
| 3 | `TfrmPreview` | Preview | TFRMPREVIEW |  | 26 |
| 3 | `TfrmObjectList` | Inaccessable Objects | TFRMOBJECTLIST |  | 1 |
| 4 | `TdlgSituatedEdit` | Situated Object Properties | TDLGSITUATEDEDIT | 558×367 | 70 |
| 4 | `TfraSituatedBasic` |  | TFRASITUATEDBASIC |  | 15 |
| 4 | `TfraSituatedLock` |  | TFRASITUATEDLOCK |  | 9 |
| 4 | `TfrmTrap` |  | TFRMTRAP |  | 17 |
| 4 | `TfraSituatedTrap` |  | TFRASITUATEDTRAP |  | 17 |
| 4 | `TfraSituatedScripts` |  | TFRASITUATEDSCRIPTS |  | 44 |
| 4 | `TfraSituatedAdvanced` |  | TFRASITUATEDADVANCED |  | 11 |
| 4 | `TfraSituatedDesc` |  | TFRASITUATEDDESC |  | 2 |
| 4 | `TfraSituatedComments` |  | TFRASITUATEDCOMMENTS |  | 1 |
| 4 | `TdlgSituatedMultiEditor` | Door Properties | TDLGSITUATEDMULTIEDITOR | 373×359 | 59 |
| 5 | `TdlgCreatureEdit` | Creature Properties | TDLGCREATUREEDIT | 788×640 | 230 |
| 5 | `TdlgCreatureWizard` | Creature Wizard | TDLGCREATUREWIZARD | 590×431 | 53 |
| 5 | `TfrmCreatureLevelupWizard` | Creature Levelup Wizard | TFRMCREATURELEVELUPWIZARD | 427×448 | 35 |
| 5 | `TColorPicker` | Character Colors | TCOLORPICKER | 254×323 | 3 |
| 5 | `TdlgSoundSetSelect` | Sound Set | TDLGSOUNDSETSELECT | 254×338 | 7 |
| 5 | `TdlgInventory` | Inventory Contents | TDLGINVENTORY | 634×545 | 16 |
| 6 | `TdlgItemEdit` | Item Properties | TDLGITEMEDIT | 787×455 | 84 |
| 6 | `TdlgPropEdit` | Select Property Parameters | TDLGPROPEDIT | 517×319 | 12 |
| 6 | `TdlgItemWizard` | Item Wizard | TDLGITEMWIZARD |  | 10 |
| 6 | `TdlgItemGeneratorEdit` | Item Generator Editor | TDLGITEMGENERATOREDIT | 634×388 | 12 |
| 6 | `TdlgGeneratorChooser` | Random Item Generator Chooser | TDLGGENERATORCHOOSER | 446×301 | 7 |
| 7 | `TdlgPlaceableEdit` | Placeable Object Properties | TDLGPLACEABLEEDIT |  | 83 |
| 7 | `TdlgPlaceableWizard` | Placeable Wizard | TDLGPLACEABLEWIZARD | 326×331 | 7 |
| 8 | `TdlgDoorEdit` | Door Properties | TDLGDOOREDIT | 608×405 | 81 |
| 8 | `TdlgDoorWizard` | Door Wizard | TDLGDOORWIZARD | 326× | 7 |
| 9 | `TdlgTriggerEdit` | Trigger Properties | TDLGTRIGGEREDIT | 357×394 | 47 |
| 9 | `TdlgTriggerWizard` | Trigger Wizard | TDLGTRIGGERWIZARD | 415×342 | 9 |
| 10 | `TdlgEncounterEdit` | Encounter Properties | TDLGENCOUNTEREDIT | 609×371 | 47 |
| 10 | `TfraEncounterCreatureList` |  | TFRAENCOUNTERCREATURELIST |  | 5 |
| 10 | `TdlgEncounterWizard` | Encounter Wizard | TDLGENCOUNTERWIZARD | 605×376 | 7 |
| 11 | `TdlgSoundEdit` | Sound Properties | TDLGSOUNDEDIT | 557×543 | 83 |
| 11 | `TdlgSoundWizard` | Sound Wizard | TDLGSOUNDWIZARD |  | 10 |
| 12 | `TdlgStoreEdit` | Merchant Properties | TDLGSTOREEDIT | 363×376 | 39 |
| 12 | `TdlgStoreWizard` | Store Wizard | TDLGSTOREWIZARD |  | 3 |
| 12 | `TdlgStoreSetupWizard` | Store Setup Wizard | TDLGSTORESETUPWIZARD |  | 14 |
| 13 | `TdlgWaypointEdit` | Waypoint Properties | TDLGWAYPOINTEDIT | 309×235 | 19 |
| 13 | `TdlgWaypointWizard` | Waypoint Wizard | TDLGWAYPOINTWIZARD | 372×271 | 10 |
| 14 | `TdlgWizard` | Wizard | TDLGWIZARD | 499×370 | 5 |
| 14 | `TdlgBlueprintWizard` | Blueprint Wizard | TDLGBLUEPRINTWIZARD | 405×319 | 7 |
| 15 | `TdlgConversationEditor` | Conversation Editor | TDLGCONVERSATIONEDITOR | 939×660 | 68 |
| 15 | `TfraConversationTree` |  | TFRACONVERSATIONTREE |  | 1 |
| 15 | `TdlgConversationInput` | Input Text | TDLGCONVERSATIONINPUT | 361×220 | 3 |
| 15 | `TdlgConversationSearch` | Search | TDLGCONVERSATIONSEARCH | 394×249 | 13 |
| 15 | `TdlgConversationTest` | Conversation Test | TDLGCONVERSATIONTEST | 258×319 | 2 |
| 15 | `TdlgConversationExportPicker` | Select Mode | TDLGCONVERSATIONEXPORTPICKER | 179×120 | 4 |
| 15 | `TdlgTokenSelector` | Select Token | TDLGTOKENSELECTOR | 370×290 | 10 |
| 16 | `TdlgScriptEditor` | Script Editor | TDLGSCRIPTEDITOR |  | 23 |
| 16 | `TdlgScriptSearch` | Find Text | TDLGSCRIPTSEARCH | 392×220 | 13 |
| 16 | `TfraScriptEditorColor` |  | TFRASCRIPTEDITORCOLOR |  | 2 |
| 16 | `TSEditCodeCompletionList` |  | TSEDITCODECOMPLETIONLIST | 390×162 | 1 |
| 16 | `TdlgScriptWizard` | Script Wizard | TDLGSCRIPTWIZARD | 509×389 | 137 |
| 17 | `TdlgJournalEditor` | Journal Editor | TDLGJOURNALEDITOR |  | 18 |
| 18 | `TdlgFactionEditor` | Faction Editor | TDLGFACTIONEDITOR |  | 14 |
| 18 | `TdlgFactionSelect` | Select Faction | TDLGFACTIONSELECT | 238×256 | 5 |
| 19 | `TdlgPlotWizard` | Plot Wizard | TDLGPLOTWIZARD | 602×452 | 44 |
| 19 | `TdlgPlotNodeWizard` | Plot Node Wizard | TDLGPLOTNODEWIZARD | 692×509 | 70 |
| 19 | `TfraPlotManager` |  | TFRAPLOTMANAGER |  | 1 |
| 19 | `TfraProgress` |  | TFRAPROGRESS |  | 1 |
| 20 | `TfraMainPalette` |  | TFRAMAINPALETTE |  | 34 |
| 20 | `TfraBlueprintSelect` |  | TFRABLUEPRINTSELECT |  | 4 |
| 20 | `TdlgPaletteChooser` | Select Category | TDLGPALETTECHOOSER | 258×332 | 3 |
| 20 | `TdlgResourceSelection` | Select Resource | TDLGRESOURCESELECTION | 224×426 | 13 |
| 20 | `TdlgChooser` | Select Type | TDLGCHOOSER | 253×201 | 3 |
| 20 | `TdlgDefaultSelector` | Select Patttern | TDLGDEFAULTSELECTOR | 187×242 | 3 |
| 21 | `TdlgPortrait` | Select Portrait | TDLGPORTRAIT | 632×453 | 12 |
| 21 | `TdlgResOpen` | Select Resource | TDLGRESOPEN | 632×453 | 10 |
| 21 | `TdlgResOpenSound` | dlgResOpenSound | TDLGRESOPENSOUND |  | 8 |
| 21 | `TdlgLoadScreen` | Loading Screen | TDLGLOADSCREEN | 769×525 | 6 |
| 21 | `TdlgResTypeSelector` | Resource Type Selection | TDLGRESTYPESELECTOR | 275×96 | 2 |
| 22 | `TdlgLocString` | String Edit | TDLGLOCSTRING | 368×343 | 7 |
| 22 | `TdlgNewEditStringExternal` | Edit String | TDLGNEWEDITSTRINGEXTERNAL | 346×247 | 7 |
| 22 | `TdlgVarTable` | Variables | TDLGVARTABLE | 555×375 | 9 |
| 22 | `TdlgComments` | Edit Comments | TDLGCOMMENTS |  | 3 |
| 23 | `TdlgImportExport` | Import Resources | TDLGIMPORTEXPORT | 255×495 | 19 |
| 23 | `TdlgMultiSelect` | dlgMultiSelect | TDLGMULTISELECT | 323×490 | 6 |
| 24 | `TdlgHakPak` | Hak Pak Conflict Analysis | TDLGHAKPAK | 781×548 | 11 |
| 25 | `TdlgOptions` | Options | TDLGOPTIONS | 632×453 | 73 |
| 26 | `TdlgVerifyModule` | Build Module | TDLGVERIFYMODULE | 542×559 | 38 |
| 27 | `TdlgWelcome` | BioWare Aurora Neverwinter Nights Toolset | TDLGWELCOME | 370×396 | 7 |
| 27 | `TdlgModuleSelect` | Open | TDLGMODULESELECT | 345×432 | 6 |
| 27 | `TdlgAbout` | About | TDLGABOUT | 464×419 | 3 |
| 27 | `TfrmHelp` | NWToolset Help | TFRMHELP | 312×213 | 2 |
| 27 | `TdlgProgress` | BioWare Aurora Neverwinter Nights Toolset | TDLGPROGRESS | 409×137 | 3 |
| 27 | `TdlgWarning` | dlgWarning | TDLGWARNING | 414×86 | 4 |
| 27 | `TdlgConfirmation` | Confirm Action | TDLGCONFIRMATION | 427×97 | 6 |

## 1. Main frame, menus, toolbars, actions

#### `TfrmFrame` — BioWare Aurora Neverwinter Nights Toolset v%s.%s.%s-%s
*DFM `TFRMFRAME` · 859×596 · StrRef table .data+0x7EECC (86/129 captions matched) · form events: OnActivate=FormActivate, OnCloseQuery=FormCloseQuery, OnDestroy=FormDestroy, OnDeactivate=FormDeactivate, OnKeyDown=FormKeyDown, OnKeyUp=FormKeyUp, OnMouseWheelDown=FormMouseWheelDown, OnMouseWheelUp=FormMouseWheelUp, OnResize=FormResize · DFM caption 'NeverWinter ToolSet - No Module'*

**Purpose:** Main application window: menu bar, dockable toolbars (File, Display, Selection Mode, Object Filters, Preview) in a `TControlBar`, left pane = Module Contents (`TfraMainInventory`), centre = tabbed area viewers (`pArea`), right pane = palettes + plot manager (`TfraMainPalette`), bottom = message log + status bar.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mMessages` | memo |  | message log (read-only) | read-only |
|  | `sbFrame` | status bar |  | status bar (2 panels) |  |
|  | `fraInventory` | embedded frame | `TfraMainInventory` |  | see frame section |
|  | `fraPalettes` | embedded frame | `TfraMainPalette` |  | see frame section |
| Object Filters | `sbDrawCreatures` | tool button | Show Creatures | filter: creatures | toggle (down); tip: “Show Creatures”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawDoors` | tool button | Show Doors | filter: doors | toggle (down); tip: “Show Doors”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawEncounters` | tool button | Show Encounters | filter: encounters | toggle (down); tip: “Show Encounters”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawItems` | tool button | Show Items | filter: items | toggle (down); tip: “Show Items”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawStores` | tool button | Show Merchants | filter: stores | toggle (down); tip: “Show Merchants”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawPlaceables` | tool button | Show Placeables | filter: placeables | toggle (down); tip: “Show Placeables”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawSounds` | tool button | Show Sounds | filter: sounds | toggle (down); tip: “Show Sounds”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawTriggers` | tool button | Show Triggers | filter: triggers | toggle (down); tip: “Show Triggers”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawWaypoints` | tool button | Show Waypoints | filter: waypoints | toggle (down); tip: “Show Waypoints”; `OnClick=sbDrawModeChange` |
| Object Filters | `sbDrawStartLocation` | tool button | Show Start Location | filter: start location | toggle (down); tip: “Show Start Location”; `OnClick=sbDrawModeChange` |
| Object Filters | `tbDrawAll` | tool button | Show All | all filters on | tip: “Show All”; `OnClick=tbDrawAllClick` |
| Object Filters | `tbDrawNone` | tool button | Show None | all filters off | tip: “Show None”; `OnClick=tbDrawNoneClick` |
| Display | `sbFindStart` | tool button | Go to Start Location | camera → start location | tip: “Go to Start Location”; `OnClick=actFindStartExecute` |
| Display | `sbRecenter` | tool button | Reorient Camera | reset camera | tip: “Reorient Camera”; `OnClick=sbRecenterClick` |
| File | `sbFileNew` | tool button | New |  | tip: “Create a new Module”; `Action=actNew` |
| File | `sbFileOpen` | tool button | Open |  | tip: “Open an existing Module”; `Action=actOpen` |
| File | `sbFileSave` | tool button | Save |  | tip: “Save”; `Action=actSave` |
| File | `sbUndo` | tool button | Undo |  | enabled when viewer has undo; disabled; tip: “Undo” |
| File | `sbRedo` | tool button | Redo |  | enabled when viewer has redo; disabled; tip: “Redo” |
| Selection Mode | `sbModeTerrain` | tool button | Select Terrain | selection mode | toggle (down); tip: “Select Terrain”; `OnClick=sbModeChange` |
| Selection Mode | `sbModeObject` | tool button | Select Objects | selection mode | toggle; tip: “Select Objects”; `OnClick=sbModeChange` |
| Preview | `tbDisplayShadows` | tool button | Display Shadows | render option | disabled; toggle; tip: “Display Shadows” |
| Preview | `tbFog` | tool button | Fog | render option | disabled; toggle; tip: “Fog” |
| Preview | `tbUseAreaLighting` | tool button | Use Area Lighting | render option | disabled; toggle; tip: “Use Area Lighting” |
| Preview | `tbAmbientSound` | tool button | Play Ambient Sound |  | toggle; tip: “Play ambient sound in area”; `Action=actPreviewAmbientSound` |
| Preview | `tbAmbientMusic` | tool button | Play Ambient Music |  | toggle; tip: “Play ambient music in area”; `Action=actPreviewAmbientMusic` |
| Preview | `tbPlacedSounds` | tool button | Play Placed Sounds |  | toggle; tip: “Play placed sound objects in area”; `Action=actPreviewPlacedSounds` |
| Preview | `tbPreview` | tool button | Show Preview Window |  | toggle; tip: “Show Preview Window”; `Action=actShowPreviewWindow` |
|  | `dlgOpen` | file dialog | Open | .mod | filter `NeverWinter Nights Modules (*.mod)/*.mod` ext .mod |
|  | `dlgSaveAs` | file dialog | Save As | .mod | filter `NeverWinter Nights Modules (*.mod)/*.mod` ext .mod |
|  | `dlgSaveExport` | file dialog | Export | .erf export |  |
|  | `dlgOpenImport` | file dialog | Import | .erf import |  |
|  | `sdSaveToSavegame` | file dialog | Save To Savegame | .sav (hidden feature) | filter `Save games/*.sav/All Files/*.*` ext .sav |

**Action list `alFrame`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actClose` | Close |  | `actCloseExecute` | disabled | Close the current file |
| `actNew` | New | Ctrl+N | `actNewExecute` |  | Create a new Module |
| `actOpen` | Open | Ctrl+O | `actOpenExecute` |  | Open an existing Module |
| `actSave` | Save | Ctrl+S | `actSaveExecute` | disabled | Save |
| `actSaveAs` | Save As... |  | `actSaveAsExecute` | disabled | Save As |
| `actFindStart` | Go to Start Location |  | `actFindStartExecute` |  | Go to Start Location |
| `actAbout` | About |  | `actAboutExecute` |  |  |
| `actOptions` | Options |  | `actOptionsExecute` |  |  |
| `actTemplateDelete` | Delete |  | `actTemplateDeleteExecute` |  |  |
| `actEditorConversation` | Conversation Editor | Ctrl+Alt+V | `actEditorConversationExecute` |  |  |
| `actEditorFaction` | Faction Editor | Ctrl+Alt+F | `actEditorFactionExecute` |  |  |
| `actEditorScript` | Script Editor | Ctrl+Alt+S | `actEditorScriptExecute` |  |  |
| `actVerifyModule` | Build Module |  | `actVerifyModuleExecute` |  |  |
| `actAreaImport` | Import Area... |  | `actAreaImportExecute` |  |  |
| `actFileExport` | Export... |  | `actFileExportExecute` |  |  |
| `actFileImport` | Import... |  | `actFileImportExecute` |  |  |
| `actJournalEdit` | Journal Editor | Ctrl+Alt+J | `actJournalEditExecute` |  |  |
| `actCopy` | Copy | Ctrl+C | `actCopyExecute` | disabled |  |
| `actCut` | Cut | Ctrl+X | `actCutExecute` | disabled |  |
| `actPaste` | Paste | Ctrl+V | `actPasteExecute` | disabled |  |
| `actWizardArea` | Area Wizard | Ctrl+Alt+A | `actWizardAreaExecute` |  |  |
| `actWizardModule` | Module Wizard | Ctrl+Alt+M | `actWizardModuleExecute` |  | New |
| `actWizardCreature` | Creature Wizard | Ctrl+Alt+C | `actWizardCreatureExecute` | disabled |  |
| `actWizardDoor` | Door Wizard |  | `actWizardDoorExecute` |  |  |
| `actWizardEncounter` | Encounter Wizard |  | `actWizardEncounterExecute` |  |  |
| `actWizardItem` | Item Wizard | Ctrl+Alt+I | `actWizardItemExecute` |  |  |
| `actWizardPlaceable` | Placeable Wizard |  | `actWizardPlaceableExecute` |  |  |
| `actWizardScript` | Script Wizard |  | `actWizardScriptExecute` |  |  |
| `actWizardSound` | Sound Wizard |  | `actWizardSoundExecute` |  |  |
| `actWizardStore` | Merchant Wizard |  | `actWizardStoreExecute` |  |  |
| `actWizardTrigger` | Trigger Wizard |  | `actWizardTriggerExecute` |  |  |
| `actWizardWaypoint` | Waypoint Wizard |  | `actWizardWaypointExecute` |  |  |
| `actCloseSilent` | Close |  | `actCloseSilentExecute` |  |  |
| `actSaveToSavegame` |  |  | `actSaveToSavegameExecute` |  |  |
| `actShowAreaStats` | Area Statistics |  | `actShowAreaStatsExecute` |  |  |
| `actWindowSpeed` |  |  |  |  |  |
| `actComputeStaticLighting` | Compute Static Lighting |  |  |  |  |
| `actShowToolbarFile` | File |  | `actShowToolbarFileExecute` | checked |  |
| `actShowToolbarDisplay` | Display |  | `actShowToolbarDisplayExecute` | checked |  |
| `actShowToolbarSelectMode` | Selection Mode (DFM 'Select Mode') |  | `actShowToolbarSelectModeExecute` | checked |  |
| `actShowToolbarFilters` | Object Filters (DFM 'Filters') |  | `actShowToolbarFiltersExecute` | checked |  |
| `actShowToolbarPreview` | Preview |  | `actShowToolbarPreviewExecute` | checked |  |
| `actShowPaneModuleContents` | Module Contents |  | `actShowPaneModuleContentsExecute` | checked |  |
| `actShowPaneMessageLog` | Message Log |  | `actShowPaneMessageLogExecute` | checked |  |
| `actShowPaneVisualCameraControls` | Visual Camera Controls |  | `actShowPaneVisualCameraControlsExecute` |  |  |
| `actShowPanePalette` | Palettes (DFM 'Palette') |  | `actShowPanePaletteExecute` | checked |  |
| `actShowPaneToolbar` | Toolbar |  | `actShowPaneToolbarExecute` | checked |  |
| `actViewFullScreen` | Full Screen | F11 | `actViewFullScreenExecute` |  |  |
| `actAreaResize` | Resize Area |  | `actAreaResizeExecute` |  |  |
| `actVerifyArea` | Verify Area |  | `actVerifyAreaExecute` |  |  |
| `actShowPreviewWindow` | Paste (DFM 'Object Preview') |  | `actShowPreviewWindowExecute` |  | Show Preview Window |
| `actAreaRotate` | Rotate Area |  | `actAreaRotateExecute` |  |  |
| `actSelectTerrain` | Select Terrain |  | `actSelectTerrainExecute` | checked | Select Terrain |
| `actSelectObjects` | Select Objects |  | `actSelectObjectsExecute` |  | Select Objects |
| `actSelectToggle` |  | F10 | `actSelectToggleExecute` |  |  |
| `actPreviewAmbientSound` | Play Ambient Sound |  | `actPreviewAmbientSoundExecute` |  | Play ambient sound in area |
| `actPreviewAmbientMusic` | Play Ambient Music |  | `actPreviewAmbientMusicExecute` |  | Play ambient music in area |
| `actPreviewPlacedSounds` | Play Placed Sounds |  | `actPreviewPlacedSoundsExecute` |  | Play placed sound objects in area |
| `actWizardPlot` | Plot Wizard |  | `actWizardPlotExecute` |  |  |

**Main menu `mmFrame`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miFile` | File |  |  |  |
| &nbsp;&nbsp;`miFileNew` | New | Ctrl+N | `actNew` |  |
| &nbsp;&nbsp;`miFileOpen` | Open | Ctrl+O | `actOpen` |  |
| &nbsp;&nbsp;`miFileClose` | Close |  | `actClose` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miFileSave` | Save | Ctrl+S | `actSave` |  |
| &nbsp;&nbsp;`miFileSaveAs` | Save As |  | `actSaveAs` |  |
| &nbsp;&nbsp;`miFileSaveToSG` | Save To Savegame |  | `actSaveToSavegame` | hidden, disabled |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miFileImport` | Import... |  | `actFileImport` |  |
| &nbsp;&nbsp;`miFileExport` | Export... |  | `actFileExport` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miFileExit` | Exit | Alt+X |  `miFileExitClick` |  |
| `Edit` | Edit |  |  |  |
| &nbsp;&nbsp;`miUndo` | Undo | Ctrl+Z |  | disabled |
| &nbsp;&nbsp;`miRedo` | Redo | Ctrl+Shift+Z |  | disabled |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miEditCopy` | Copy | Ctrl+C | `actCopy` |  |
| &nbsp;&nbsp;`miCut` | Cut | Ctrl+X | `actCut` |  |
| &nbsp;&nbsp;`miPaste` | Paste | Ctrl+V | `actPaste` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miResize` | Resize Area |  | `actAreaResize` | disabled |
| &nbsp;&nbsp;`miRotateArea` | Rotate Area |  | `actAreaRotate` | disabled |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miFindInstance` | Find Instance |  |  `miFindInstanceClick` | disabled |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miViewProperties` | Module Properties |  |  `miViewPropertiesClick` | disabled |
| &nbsp;&nbsp;`miEditAreaProperties` | Area Properties |  |  | disabled |
| &nbsp;&nbsp;`miObjectProperties` | Object Properties |  |  | hidden |
| `miView` | View |  |  `miViewClick` |  |
| &nbsp;&nbsp;`miSelectMode` | Selection Mode |  |  |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miSelectTerrain` | Select Terrain |  | `actSelectTerrain` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miSelectObjects` | Select Objects |  | `actSelectObjects` |  |
| &nbsp;&nbsp;`miViewToolbars` | Toolbars |  |  |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miToolbarsModule` | File |  | `actShowToolbarFile` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miToolbarsDisplay` | Display |  | `actShowToolbarDisplay` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`SelectMode1` | Selection Mode |  | `actShowToolbarSelectMode` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miToolbarsFilters` | Object Filters |  | `actShowToolbarFilters` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miToolbarsPreview` | Preview |  | `actShowToolbarPreview` |  |
| &nbsp;&nbsp;`miViewControls` | Interface Panels |  |  |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFeedback` | Message Log |  | `actShowPaneMessageLog` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miModuleContents` | Module Contents |  | `actShowPaneModuleContents` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miPalette` | Palettes |  | `actShowPanePalette` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`Toolbar1` | Toolbar |  | `actShowPaneToolbar` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miCamera` | Visual Camera Controls |  | `actShowPaneVisualCameraControls` | disabled |
| &nbsp;&nbsp;`miObjectFilters` | Object Filters |  |  `miObjectFiltersClick` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterAll` | Show All |  |  `miFilterAllClick` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterNone` | Show None |  |  `miFilterNoneClick` |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterCreatures` | Show Creatures |  |  `miFilterCreaturesClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterDoors` | Show Doors |  |  `miFilterDoorsClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterEncounters` | Show Encounters |  |  `miFilterEncountersClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterItems` | Show Items |  |  `miFilterItemsClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterStores` | Show Merchants |  |  `miFilterStoresClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterPlaceables` | Show Placeables |  |  `miFilterPlaceablesClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterSounds` | Show Sounds |  |  `miFilterSoundsClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterTriggers` | Show Triggers |  |  `miFilterTriggersClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterWaypoints` | Show Waypoints |  |  `miFilterWaypointsClick` | checked |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFilterStartLocation` | Go to Start Location |  |  `miFilterStartLocationClick` | checked |
| &nbsp;&nbsp;`miFullScreen` | Full Screen | F11 | `actViewFullScreen` |  |
| &nbsp;&nbsp;`miShowPreviewWindow` | Show Preview Window |  | `actShowPreviewWindow` |  |
| `miScene` | Environment |  |  |  |
| &nbsp;&nbsp;`miRefresh` | Refresh | F5 |  | disabled |
| &nbsp;&nbsp;`miDisplayGrid` | Display Grid |  |  | disabled, checked |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miDisplayShadows` | Display Shadows |  |  | disabled, checked |
| &nbsp;&nbsp;`miFog` | Fog |  |  | disabled |
| &nbsp;&nbsp;`miUseAreaLighting` | Use Area Lighting |  |  | disabled, checked |
| &nbsp;&nbsp;`miFadeGeometry` | Fade Geometry |  |  | disabled |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFadeGeometryNever` | Never |  |  | checked, radio |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFadeGeometryObjectModeOnly` | Object Mode Only |  |  | radio |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFadeGeometryAlways` | Always |  |  | radio |
| &nbsp;&nbsp;`miRenderAABB` | Render AABB Nodes |  |  | disabled |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miAmbientSound` | Play Ambient Sound |  | `actPreviewAmbientSound` |  |
| &nbsp;&nbsp;`miAmbientMusic` | Play Ambient Music |  | `actPreviewAmbientMusic` |  |
| &nbsp;&nbsp;`miPlacedSounds` | Play Placed Sounds |  | `actPreviewPlacedSounds` |  |
| `Build1` | Build |  |  |  |
| &nbsp;&nbsp;`miVerifyArea` | Verify Area |  | `actVerifyArea` | disabled |
| &nbsp;&nbsp;`miVerifyModule` | Build Module |  | `actVerifyModule` |  |
| &nbsp;&nbsp;`miTestModule` | Test Module | F9 |  `miTestModuleClick` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miAreaStats` | Area Statistics |  | `actShowAreaStats` | disabled |
| `miTools` | Tools |  |  |  |
| &nbsp;&nbsp;`miConvEditor` | Conversation Editor | Ctrl+Alt+V | `actEditorConversation` |  |
| &nbsp;&nbsp;`miFactionEditor` | Faction Editor | Ctrl+Alt+F | `actEditorFaction` |  |
| &nbsp;&nbsp;`miScriptEditor` | Script Editor | Ctrl+Alt+S | `actEditorScript` |  |
| &nbsp;&nbsp;`JournalEditor1` | Journal Editor | Ctrl+Alt+J | `actJournalEdit` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`actOptions1` | Options |  | `actOptions` |  |
| `Wizards` | Wizards |  |  |  |
| &nbsp;&nbsp;`miModuleWizard` | Module Wizard | Ctrl+Alt+M | `actWizardModule` | hidden |
| &nbsp;&nbsp;`miAreaWizard` | Area Wizard | Ctrl+Alt+A | `actWizardArea` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miCreatureWizard` | Creature Wizard | Ctrl+Alt+C | `actWizardCreature` |  |
| &nbsp;&nbsp;`miDoorWizard` | Door Wizard |  | `actWizardDoor` |  |
| &nbsp;&nbsp;`miEncounterWizard` | Encounter Wizard |  | `actWizardEncounter` |  |
| &nbsp;&nbsp;`miItemWizard` | Item Wizard | Ctrl+Alt+I | `actWizardItem` |  |
| &nbsp;&nbsp;`miStoreWizard` | Merchant Wizard |  | `actWizardStore` |  |
| &nbsp;&nbsp;`miPlaceableWizard` | Placeable Wizard |  | `actWizardPlaceable` |  |
| &nbsp;&nbsp;`miSoundWizard` | Sound Wizard |  | `actWizardSound` |  |
| &nbsp;&nbsp;`miTriggerWizard` | Trigger Wizard |  | `actWizardTrigger` |  |
| &nbsp;&nbsp;`miWaypointWizard` | Waypoint Wizard |  | `actWizardWaypoint` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miWizardPlot` | Plot Wizard | Ctrl+Alt+P | `actWizardPlot` |  |
| `miMainWindow` | Window |  |  | hidden |
| `miHelp` | Help |  |  |  |
| &nbsp;&nbsp;`miHelpBWWeb` | Neverwinter Nights Website |  |  `miHelpBWWebClick` |  |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`miHelpAbout` | About |  | `actAbout` |  |

**Popup menu `pmToolbars`** (OnPopup=pmToolbarsPopup)

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miShowToolbarFile` | File |  | `actShowToolbarFile` |  |
| `miShowToolbarDisplay` | Display |  | `actShowToolbarDisplay` |  |
| `miShowToolbarSelectMode` | Selection Mode |  | `actShowToolbarSelectMode` |  |
| `miShowToolbarFilters` | Object Filters |  | `actShowToolbarFilters` |  |
| `miShowToolbarPreview` | Preview |  | `actShowToolbarPreview` |  |

**Popup menu `pmEditorTab`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miEditorTabClose` | Close |  |  `miEditorTabCloseClick` |  |

**Notes / behaviors:**
- Menu captions come from the Actions (`Action=` property); runtime text from TLK (e.g. *View › Toolbars › Object Filters*, *Interface Panels › Palettes*).
- The area viewer merges its own `Edit` (GroupIndex 20) and `Scene` (30) menus; frame menus use GroupIndex 90 so they stay.
- Disabled-by-default items (`Undo`, `Redo`, `Resize/Rotate Area`, `Find Instance`, `Module/Area Properties`, render toggles, `Verify Area`, `Area Statistics`) are enabled when a module/area is open.
- Hidden/dead: `File › Save To Savegame` (`actSaveToSavegame` + `sdSaveToSavegame`), `Wizards › Module Wizard` (visible via Welcome dialog), `Window` menu, `actComputeStaticLighting`, `actWindowSpeed`, `actAreaImport`, `actTemplateDelete`, `actCloseSilent`.
- Toolbars live in a `TControlBar` (drag to re-dock); visibility via View › Toolbars and toolbar popup `pmToolbars`; state persisted in nwtoolset.ini (`FileVisible`, `FileDocked`, `FileTop`…).
- Panes: Module Contents (left), Palettes (right), Message Log (bottom), Visual Camera Controls (in viewer); widths persisted (`ContentsWidth`, `PaletteWidth`, `MessagesHeight`).
- `pArea` hosts area viewers as tabs (EE); `pmEditorTab` (Close) on tab right-click; ini `Disable Tabs` reverts to single area.
- Form handles `KeyPreview` + `FormKeyDown/Up` (camera keys, F10 toggle) and mouse-wheel (zoom) for the active viewer.
- `miTestModule` (F9) has no action: saves then launches the game (see implicit behaviors). `Help › Neverwinter Nights Website` opens http://nwn.beamdog.com.

#### `TfraMainInventory` — (no caption)
*DFM `TFRAMAININVENTORY` · StrRef table .data+0x4CACC (14/15 captions matched)*

**Purpose:** Module Contents tree (left pane): Areas (with per-area object lists), Conversations, Scripts, and other module resources. Context menu drives most module-level resource operations.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `tvMain` | tree |  | module resources / area objects | `OnChange=tvMainChange OnDblClick=actGotoExecute OnEdited=tvMainEdited OnKeyDown=tvMainKeyDown OnMouseDown=tvMainMouseDown` |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actGoto` | Go To |  | `actGotoExecute` |  |  |
| `actRemove` | Remove (DFM 'Delete') |  | `actRemoveExecute` |  |  |
| `actNew` | New |  | `actNewExecute` |  |  |
| `actEdit` | Edit |  | `actEditExecute` |  |  |
| `actRescanInstances` | Refresh Area |  | `actRescanInstancesExecute` |  |  |
| `actAddToPalette` | Add to Palette |  | `actAddToPaletteExecute` |  |  |
| `actCopy` | Copy |  | `actCopyExecute` |  |  |
| `actCut` | Cut |  | `actCutExecute` |  |  |
| `actAreaExport` | Export Area |  | `actAreaExportExecute` |  |  |
| `actCopyResource` | Create Copy |  | `actCopyResourceExecute` |  |  |
| `actRefreshConversations` |  |  | `actRefreshConversationsExecute` |  |  |
| `actLocationAdjust` | Adjust Location |  | `actLocationAdjustExecute` |  |  |
| `actVariables` | Variables |  | `actVariablesExecute` |  |  |
| `actBuild` | Build |  | `actBuildExecute` |  |  |

**Popup menu `pmMainArea`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miNew` | New |  | `actNew` |  |
| `miObjectFocus` | Focus on Object |  | `actGoto` |  |
| `miLocationAdjust` | Adjust Location |  | `actLocationAdjust` |  |
| `miAreaView` | View Area |  | `actGoto` |  |
| `miEdit` | Edit |  | `actEdit` |  |
| `miBuild` | Build |  | `actBuild` |  |
| — | ——— | | | |
| `miCopyResource` | Create Copy |  | `actCopyResource` |  |
| `miCut` | Cut |  | `actCut` |  |
| `miCopy` | Copy |  | `actCopy` |  |
| `miPaste` |  |  |  |  |
| `miDelete` | Delete |  | `actRemove` |  |
| — | ——— | | | |
| `miRefreshArea` | Refresh Area |  | `actRescanInstances` | hidden |
| `miAddtoPalette` | Add To Palette |  | `actAddToPalette` |  |
| `miAreaExport` | Export Area |  | `actAreaExport` |  |
| — | ——— | | | |
| `miVariables` | Variables |  | `actVariables` |  |
| `miProperties` | Properties |  | `actEdit` |  |

**Notes / behaviors:**
- Context menu is filtered per node type (area node vs object node vs resource node). *Delete* removes areas/resources (not undoable).
- Area nodes lazily populate object children (`tvMainExpanding`), grouped by type; labels = name (creatures/items/placeables) or tag.
- Double-click (`actGotoExecute`) opens the area tab or focuses camera on an object. *Build* compiles/validates the node (e.g. scripts, area).
- In-place label editing (`tvMainEditing/Edited`) renames resources.

#### `TfrmViewer` — frmViewer
*DFM `TFRMVIEWER` · no StrRef table resolved · form events: OnCloseQuery=FormCloseQuery*

**Purpose:** Base class of document windows (open/save dialogs, close query).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `dlgOpen` | file dialog |  |  |  |
|  | `dlgSaveAs` | file dialog |  |  |  |

#### `TfrmProperties` — Properties
*DFM `TFRMPROPERTIES` · 280×376 · border bsDialog · no StrRef table resolved*

**Purpose:** Base class for property dialogs (OK/Cancel).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `m_bCancel` | button | Cancel |  |  |
|  | `m_bOK` | button | OK |  |  |

## 2. Module properties & module wizard

#### `TfrmIFOProp` — Module Properties
*DFM `TFRMIFOPROP` · 330×527 · StrRef table .data+0x6BF40 (71/74 captions matched) · form events: OnCloseQuery=FormCloseQuery, OnShow=FormShow*

**Purpose:** Module Properties (module.ifo).

**Inherits:** `TfrmProperties` (OK/Cancel).

**Tabs** (`pcModuleProperties`): Basic · Events · Advanced · Description · Custom Content

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `m_bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `m_bOK` | button | OK |  | tip: “Accept changes”; `OnClick=m_bOKClick OnKeyDown=m_bOKKeyDown` |
| Basic | `eName` | edit | Name | IFO `Mod_Name` (CExoLocString) | tip: “Edit the Name in the primary language” |
| Basic | `cbStartArea` | combo | Start Area | IFO `Mod_Entry_Area` (read-only; set by painting Start Location) | disabled; editable |
| Basic | `eStartPosX` | edit | Starting X | IFO `Mod_Entry_X` | disabled; read-only; max 3; spin -10000…10000 |
| Basic | `eStartPosY` | edit | Starting Y | IFO `Mod_Entry_Y` | disabled; read-only; max 3; spin -10000…10000 |
| Basic | `eStartPosZ` | edit | Starting Z | IFO `Mod_Entry_Z` | disabled; read-only; max 3; spin -10000…10000 |
| Basic | `bName` | button | ... | `Mod_Name` all languages (TdlgLocString) | tip: “Edit text in multiple languages” |
| Basic | `eTag` | edit | Tag | IFO `Mod_Tag` | `OnChange=eTagChange` |
| Events | `cbOnClientEnter` | combo | OnClientEnter | IFO `Mod_OnClientEntr` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnModuleLoad` | combo | OnModuleLoad | IFO `Mod_OnModLoad` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnHeartbeat` | combo | OnHeartbeat | IFO `Mod_OnHeartbeat` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnUserDefined` | combo | OnUserDefined | IFO `Mod_OnUsrDefined` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnClientLeave` | combo | OnClientLeave | IFO `Mod_OnClientLeav` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnActivateItem` | combo | OnActivateItem | IFO `Mod_OnActvtItem` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnAcquireItem` | combo | OnAcquireItem | IFO `Mod_OnAcquirItem` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnModuleStart` | combo | ⟨Events⟩ | IFO `Mod_OnModStart` | buttons […] [Edit]; **hidden**; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnUnAquireItem` | combo | OnUnAcquireItem | IFO `Mod_OnUnAqreItem` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerDeath` | combo | OnPlayerDeath | IFO `Mod_OnPlrDeath` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerDying` | combo | OnPlayerDying | IFO `Mod_OnPlrDying` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerRespawn` | combo | OnPlayerRespawn | IFO `Mod_OnSpawnBtnDn` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerRest` | combo | OnPlayerRest | IFO `Mod_OnPlrRest` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerLevelUp` | combo | OnPlayerLevelUp | IFO `Mod_OnPlrLvlUp` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnCutsceneAbort` | combo | OnCutsceneAbort | IFO `Mod_OnCutsnAbort` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerEquipItem` | combo | OnPlayerEquipItem | IFO `Mod_OnPlrEqItm` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerUnEquipItem` | combo | OnPlayerUnEquipItem | IFO `Mod_OnPlrUnEqItm` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerChat` | combo | OnPlayerChat | IFO `Mod_OnPlrChat` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnNuiEvent` | combo | OnNuiEvent | IFO `Mod_OnNuiEvent` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerGuiEvent` | combo | OnPlayerGuiEvent | IFO `Mod_OnPlrGuiEvt` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerTarget` | combo | OnPlayerTarget | IFO `Mod_OnPlrTarget` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnPlayerTileAction` | combo | OnPlayerTileAction | IFO `Mod_OnPlrTileAct` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Advanced | `eStartMonth` | edit | Starting Month | IFO `Mod_StartMonth` | max 2; spin 1…12; `OnExit=eStartMonthExit` |
| Advanced | `eStartDay` | edit | Starting Day | IFO `Mod_StartDay` | max 2; spin 1…31; `OnExit=eStartDayExit` |
| Advanced | `eStartHour` | edit | Starting Hour | IFO `Mod_StartHour` | max 2; spin 0…23; `OnExit=eStartHourExit` |
| Advanced | `eTemplateName` | edit | ⟨Advanced⟩ | (dead) | **hidden**; max 16; tip: “Edit the ResRef” |
| Advanced | `eMinPerHour` | edit | Minutes/Hour | IFO `Mod_MinPerHour` | max 3; spin 1…240; `OnExit=eMinutesPerHourExit` |
| Advanced | `eDawnHour` | edit | Dawn Start Hour | IFO `Mod_DawnHour` | max 3; spin -99…99; `OnExit=eDawnHourExit` |
| Advanced | `eDuskHour` | edit | Dusk Start Hour | IFO `Mod_DuskHour` | max 3; spin -99…99; `OnExit=eDuskHourExit` |
| Advanced | `tbXPScale` | slider | XP Scale | IFO `Mod_XPScale` | range 0…200; `OnChange=tbXPScaleChange` |
| Advanced | `eXPScale` | edit | XP Scale | IFO `Mod_XPScale` (numeric mirror) | `OnExit=eXPScaleExit` |
| Advanced | `cbMovieStart` | combo | Starting Movie | IFO `Mod_StartMovie` (movies/*.bik) | buttons […]; max 16; list |
| Advanced | `eStartYear` | edit | Starting Year | IFO `Mod_StartYear` | max 5; spin 0…30000; `OnExit=eStartYearExit` |
| Advanced | `bVariablesEdit` | button | ... | IFO `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesEditClick` |
| Description | `mDescription` | memo | ⟨Description⟩ | IFO `Mod_Description` | tip: “Edit the Name in the primary language” |
| Description | `bDescription` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Custom Content | `lbHakFiles` | listbox | This is a list of Hak Paks used by this module, sorted in order of highest to lowest pr… | IFO `Mod_HakList[].Mod_Hak` (ordered, top = highest priority; legacy `Mod_Hak`) | `OnClick=lbHakFilesClick OnKeyDown=lbHakFilesKeyDown` |
| Custom Content | `bHakAdd` | button | Add | append to hak list | `OnClick=bHakAddClick` |
| Custom Content | `bHakRemove` | button | Remove | remove selected hak | `OnClick=bHakRemoveClick` |
| Custom Content | `bHakMoveUp` | button | Move Up | reorder | `OnClick=bHakMoveUpClick` |
| Custom Content | `bHakMoveDown` | button | Move Down | reorder | `OnClick=bHakMoveDownClick` |
| Custom Content | `cbHakFile` | combo | ⟨Custom Content⟩ | choose .hak from hak dir(s) | max 255; list; tip: “Choose the HAK file to use with this Module”; `OnChange=cbHakFileChange` |
| Custom Content | `bHakConflicts` | button | Check for Conflicts... | opens TdlgHakPak | `OnClick=bHakConflictsClick` |
| Custom Content | `cbCustomTlkFile` | combo | Custom Tlk File | IFO `Mod_CustomTlk` (tlk dir) | list |

**Notes / behaviors:**
- Split name/value panel layout; start location fields are informational (paint Start Location in an area to change).
- Event list includes EE events (OnPlayerChat, OnPlayerTarget, OnPlayerGuiEvent, OnPlayerTileAction, OnNuiEvent); OnModuleStart (`Mod_OnModStart`) row is hidden.
- XP Scale slider and edit mirror each other (0–200). Start-year/month/day/hour and dawn/dusk validated on exit.
- *Custom Content* tab: ordered hak list (top overrides lower), add from dropdown of available .hak files, move up/down, remove, conflict analysis, custom TLK dropdown (tlk folder).
- Not exposed in UI but written by the toolset: `Mod_ID`, `Mod_UUID`, `Mod_Creator_ID`, `Mod_Version`, `Mod_MinGameVer`, `Expansion_Pack`, `Mod_Area_list`, `Mod_CacheNSSList`, `Mod_Expan_List`, `Mod_CutSceneList`, `Mod_GVar_List`, `Mod_PartyControl`, `Mod_DefaultBic`.

#### `TdlgModuleWizard` — Module Wizard
*DFM `TDLGMODULEWIZARD` · 328×None · StrRef table .data+0x55388 (10/15 captions matched) · DFM caption 'dlgModuleWizard'*

**Purpose:** Module creation wizard: name → create areas (launches Area Wizard repeatedly) → finish.

**Inherits:** `TdlgWizard`.

**Tabs** (`pcSteps`): tsStart · tsModuleCreation · tsAreaCreation · tsFinish

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | Help |  |  |
|  | `bFinish` | button | Finish |  | tip: “Finish the Wizard” |
|  | `bNext` | button | Next > |  | tip: “Continue to the next step in the Wizard” |
|  | `bBack` | button | < Back |  | tip: “Return to the previous step in the Wizard” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
| tsModuleCreation | `eModuleName` | edit | Please enter the module title | IFO `Mod_Name` + module filename | `OnChange=eModuleNameChange` |
| tsModuleCreation | `bModuleName` | button | ... | all languages | tip: “Edit text in multiple languages” |
| tsAreaCreation | `lbAreas` | listbox | Current areas | areas created so far |  |
| tsAreaCreation | `bNewArea` | button | Area Wizard (DFM: 'New Area') | runs Area Wizard | `OnClick=bNewAreaClick` |

## 3. Area editor, area properties, area wizard, resize/rotate, tiles, transitions, placement

#### `TfrmViewerArea` — (no caption)
*DFM `TFRMVIEWERAREA` · 619×494 · border bsNone · StrRef table .data+0xC0B4C (28/35 captions matched) · form events: OnCreate=FormCreate, OnDestroy=FormDestroy, OnPaint=FormPaint, OnResize=FormResize, OnShow=FormShow*

**Purpose:** 3D area editor (one per open area tab): OpenGL viewport, optional visual camera-control pad, per-instance context menu, and an area-specific Edit/Scene menu merged into the main menu (GroupIndex 20/30).

**Inherits:** `TfrmViewer` (open/save dialogs, `FormCloseQuery`). Hosted as a tab inside the main frame `pArea`.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `GLPanel` | viewport |  | area 3D viewport | `OnDblClick=GLPanelDblClick OnMouseDown=GLPanelMouseDown` |
|  | `CommandBox` | edit |  | debug console input |  |
|  | `bPanLeft` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Move camera Left”; `OnKeyDown=bPanLeftKeyDown OnMouseDown=bPanLeftMouseDown` |
|  | `bPanRight` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Move camera right”; `OnKeyDown=bPanRightKeyDown OnMouseDown=bPanRightMouseDown` |
|  | `bPanForward` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Move camera forwards”; `OnKeyDown=bPanForwardKeyDown OnMouseDown=bPanForwardMouseDown` |
|  | `bPanBackwards` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Move camera backwards”; `OnKeyDown=bPanBackwardsKeyDown OnMouseDown=bPanBackwardsMouseDown` |
|  | `bRotateClockwise` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Rotate camera clockwise”; `OnKeyDown=bRotateClockwiseKeyDown OnMouseDown=bRotateClockwiseMouseDown` |
|  | `bRotateCounterClockwise` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Rotate camera counterclockwise”; `OnKeyDown=bRotateCounterClockwiseKeyDown OnMouseDown=bRotateCounterClockwiseMouseDown` |
|  | `bPitchUp` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Pitch camera up”; `OnKeyDown=bPitchUpKeyDown OnMouseDown=bPitchUpMouseDown` |
|  | `bPitchDown` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Pitch camera down”; `OnKeyDown=bPitchDownKeyDown OnMouseDown=bPitchDownMouseDown` |
|  | `bZoomIn` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Zoom camera in”; `OnKeyDown=bZoomInKeyDown OnMouseDown=bZoomInMouseDown` |
|  | `bZoomOut` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Zoom camera out”; `OnKeyDown=bZoomOutKeyDown OnMouseDown=bZoomOutMouseDown` |
|  | `bGobRotateCounter` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Rotate object counterclockwise”; `OnKeyDown=bGobRotateCounterKeyDown OnMouseDown=bGobRotateCounterMouseDown` |
|  | `bGobRotateClockwise` | button | (no caption; glyph) |  | **hidden (container)**; tip: “Rotate object clockwise”; `OnKeyDown=bGobRotateClockwiseKeyDown OnMouseDown=bGobRotateClockwiseMouseDown` |
|  | `bGobRotateRandom` | button | (no caption; glyph) | instance Bearing (random) | **hidden (container)**; tip: “Set random facing”; `OnClick=bGobRotateRandomClick` |
|  | `dlgOpen` | file dialog |  |  |  |
|  | `dlgSaveAs` | file dialog |  |  |  |

**Action list `alViewerArea`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actInstanceProperties` | Properties |  | `actInstancePropertiesExecute` | hidden |  |
| `actInstanceDelete` | Delete |  | `actInstanceDeleteExecute` | hidden |  |
| `actEncounterAddSpawnPoint` | Add Spawn Point |  | `actEncounterAddSpawnPointExecute` | hidden |  |
| `actWaypointCreateSet` | Create Set |  | `actWaypointCreateSetExecute` |  |  |
| `actDoorReverse` | Reverse Door |  | `actDoorReverseExecute` |  |  |
| `actAnimateStop` | Stop Animation |  | `actAnimateStopExecute` |  |  |
| `actAnimateOpening1` | Opening Forward |  | `actAnimateOpening1Execute` |  |  |
| `actAnimateOpening2` | Opening Backward |  | `actAnimateOpening2Execute` |  |  |
| `actAnimateClosing1` | Closing from Front |  | `actAnimateClosing1Execute` |  |  |
| `actAnimateClosing2` | Closing from Back |  | `actAnimateClosing2Execute` |  |  |
| `actAnimateClosed` | Closed |  | `actAnimateClosedExecute` |  |  |
| `actAnimateOpened1` | Opened Forward |  | `actAnimateOpened1Execute` |  |  |
| `actAnimateOpened2` | Opened Backward |  | `actAnimateOpened2Execute` |  |  |
| `actPolygonRedraw` | Redraw Polygon |  | `actPolygonRedrawExecute` |  |  |
| `actNewTemplateFromInstance` | Add to Palette |  | `actNewTemplateFromInstanceExecute` |  |  |
| `actAnimatePlaceableOpen` | Opened (DFM 'Open') |  | `actAnimatePlaceableOpenExecute` |  |  |
| `actAnimatePlaceableClosed` | Closed |  | `actAnimatePlaceableClosedExecute` |  |  |
| `actAnimatePlaceableActivate` | Activated |  | `actAnimatePlaceableActivateExecute` |  |  |
| `actAnimatePlaceableDeactivated` | Deactivated |  | `actAnimatePlaceableDeactivatedExecute` |  |  |
| `actAnimatePlaceableDestroyed` | Destroyed |  | `actAnimatePlaceableDestroyedExecute` |  |  |
| `actAnimatePlaceableDefault` | Default |  | `actAnimatePlaceableDefaultExecute` |  |  |
| `actSoundTurnOn` | Turn On |  | `actSoundTurnOnExecute` |  |  |
| `actSoundTurnOff` | Mute |  | `actSoundTurnOffExecute` |  |  |
| `actTileProperties` | Tile Properties |  | `actTilePropertiesExecute` |  |  |
| `actCreatureAddWaypoint` | Create Waypoint (DFM 'Add Waypoint') |  | `actCreatureAddWaypointExecute` |  |  |
| `actAddPopupText` | Add Popup Text |  | `actAddPopupTextExecute` |  |  |
| `actWizardStoreSetup` | Setup Store |  | `actWizardStoreSetupExecute` |  |  |
| `actWizardCreatureLevelup` | Levelup Wizard |  | `actWizardCreatureLevelupExecute` |  |  |
| `actInventoryEdit` | Inventory (DFM 'Edit Inventory') |  | `actInventoryEditExecute` |  | Edit inventory contents |
| `actLocationAdjust` | Adjust Location |  | `actLocationAdjustExecute` |  |  |
| `actVariables` | Variables |  | `actVariablesExecute` |  |  |
| `actConversationEdit` | Conversation (DFM 'Edit Conversation') |  | `actConversationEditExecute` |  |  |

**Main menu `AreaMenu`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `Edit` | Edit |  |  |  |
| &nbsp;&nbsp;`Undo` | Undo | Ctrl+Z |  `UndoClick` | disabled |
| &nbsp;&nbsp;`Redo` | Redo | Ctrl+Shift+Z |  `RedoClick` | disabled |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`Copy1` | Copy | Ctrl+C |  |  |
| &nbsp;&nbsp;`Cut1` | Cut | Ctrl+X |  | disabled |
| &nbsp;&nbsp;`Paste1` | Paste | Ctrl+V |  | disabled |
| &nbsp;&nbsp;— | ——— | | | |
| &nbsp;&nbsp;`AreaProperties` | Area Properties |  |  `AreaPropertiesClick` |  |
| `miScene` | Scene |  |  |  |
| &nbsp;&nbsp;`miRefresh` | Refresh |  |  `miRefreshClick` |  |
| &nbsp;&nbsp;`miDisplayGrid` | Display Grid |  |  `miDisplayGridClick` | checked |
| &nbsp;&nbsp;`miDisplayShadows` | Display Shadows |  |  `miDisplayShadowsClick` | checked |
| &nbsp;&nbsp;`miFadeGeometry` | Fade Geometry |  |  |  |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFadeGeometryNever` | Never |  |  `miFadeGeometryNeverClick` | checked, radio |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFadeGeometryObjectModeOnly` | Object Mode Only |  |  `miFadeGeometryObjectModeOnlyClick` | radio |
| &nbsp;&nbsp;&nbsp;&nbsp;`miFadeGeometryAlways` | Always |  |  `miFadeGeometryAlwaysClick` | radio |
| &nbsp;&nbsp;`miFog` | Fog |  |  `miFogClick` |  |
| &nbsp;&nbsp;`miUseAreaLighting` | Use Area Lighting |  |  `miUseAreaLightingClick` | checked |

**Popup menu `pmViewerArea`** (OnPopup=pmViewerAreaPopup)

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `pmiAddToPalette` | Add To Palette |  | `actNewTemplateFromInstance` |  |
| `pmiLocationAdjust` | Adjust Location |  | `actLocationAdjust` |  |
| `pmiWaypointCreateSet` | Create Set |  | `actWaypointCreateSet` |  |
| `pmiPolygonRedraw` | Redraw Polygon |  | `actPolygonRedraw` |  |
| `pmiAddPopupText` | Add Popup Text |  | `actAddPopupText` |  |
| `pmiCreatureAddWaypoint` | Create Waypoint |  | `actCreatureAddWaypoint` |  |
| `pmiConversationEdit` | Conversation |  | `actConversationEdit` |  |
| `pmiInventoryEdit` | Inventory |  | `actInventoryEdit` |  |
| `pmiWizardCreatureLevelup` | Levelup Wizard |  | `actWizardCreatureLevelup` |  |
| `pmiEncounterAddSpawnPoint` | Add Spawn Point |  | `actEncounterAddSpawnPoint` |  |
| `pmiSetupStore` | Setup Store |  | `actWizardStoreSetup` |  |
| `pmiReverseDoor` | Reverse Door |  | `actDoorReverse` |  |
| `pmiAnimate` | Initial State |  |  |  |
| &nbsp;&nbsp;`miAnimateStop` | Stop Animation |  | `actAnimateStop` | hidden |
| &nbsp;&nbsp;`OpenedOutward1` | Opened Forward |  | `actAnimateOpened1` |  |
| &nbsp;&nbsp;`OpenedInward1` | Opened Backward |  | `actAnimateOpened2` |  |
| &nbsp;&nbsp;`miAnimateClosed` | Closed |  | `actAnimateClosed` |  |
| &nbsp;&nbsp;`miAnimateOpen1` | Opening Forward |  | `actAnimateOpening1` | hidden |
| &nbsp;&nbsp;`miAnimateOpen2` | Opening Backward |  | `actAnimateOpening2` | hidden |
| &nbsp;&nbsp;`miAnimateClose1` | Closing from Front |  | `actAnimateClosing1` | hidden |
| &nbsp;&nbsp;`miAnimateClose2` | Closing from Back |  | `actAnimateClosing2` | hidden |
| `pmiAnimatePlaceable` | Initial State |  |  |  |
| &nbsp;&nbsp;`miPlaceableDefault` | Default |  | `actAnimatePlaceableDefault` |  |
| &nbsp;&nbsp;`miPlaceableOpen` | Opened |  | `actAnimatePlaceableOpen` |  |
| &nbsp;&nbsp;`miPlaceableClosed` | Closed |  | `actAnimatePlaceableClosed` |  |
| &nbsp;&nbsp;`miPlaceableDestroyed` | Destroyed |  | `actAnimatePlaceableDestroyed` |  |
| &nbsp;&nbsp;`miPlaceableActivated` | Activated |  | `actAnimatePlaceableActivate` |  |
| &nbsp;&nbsp;`miPlaceableDeactivated` | Deactivated |  | `actAnimatePlaceableDeactivated` |  |
| `pmiMute` | Mute |  | `actSoundTurnOff` |  |
| `pmiUnMute` | Turn On |  | `actSoundTurnOn` |  |
| `pmiDelete` | Delete |  | `actInstanceDelete` |  |
| `pmiTileProperties` | Tile Properties |  | `actTileProperties` |  |
| `pmiVariables` | Variables |  | `actVariables` |  |
| `pmiProperties` | Properties |  | `actInstanceProperties` |  |

**Notes / behaviors:**
- Context menu items are shown/hidden per clicked object type in `pmViewerAreaPopup`: tiles → Tile Properties; creatures → Create Waypoint, Conversation, Inventory, Levelup Wizard, Setup Store (for merchants), Add Popup Text; doors → Reverse Door, Initial State (door states); placeables → Initial State (placeable states), Inventory; triggers/encounters → Redraw Polygon, Add Spawn Point; waypoints → Create Set; sounds → Mute / Turn On; all → Add To Palette, Adjust Location, Delete, Variables, Properties.
- `pCameraControls` buttons act while held (`MouseDown`/`MouseUp`, also keyboard repeat).
- `AreaMenu.Edit.Copy1` has no handler here: copy is routed through the frame action `actCopy`.

#### `TdlgAreaProperties` — Area Properties
*DFM `TDLGAREAPROPERTIES` · 543×515 · StrRef table .data+0x720A4 (42/49 captions matched) · form events: OnShow=FormShow*

**Purpose:** Area Properties (ARE + GIT `AreaProperties`).

**Inherits:** `TfrmProperties` (OK/Cancel).

**Tabs** (`pcAreaProp`): Basic · Visual · Audio · Events · Advanced · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `m_bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `m_bOK` | button | OK |  | tip: “Accept changes”; `OnClick=m_bOKClick` |
|  | `m_iImage` | image |  | tileset/area thumbnail |  |
| Basic | `eName` | edit | Name | ARE `Name` | max 1024; tip: “Edit the Name in the primary language” |
| Basic | `cbTileSet` | combo | Tileset | ARE `Tileset` (*.set; editable only on creation) | list; tip: “Select a Tileset for the Area”; `OnChange=cbTileSetChange` |
| Basic | `eLength` | edit | Length | ARE `Height` (tiles; shown only when creating) | max 2; spin 2…32; `OnExit=OnLengthWidthExit` |
| Basic | `eWidth` | edit | Width | ARE `Width` (tiles) | max 2; spin 2…32; `OnExit=OnLengthWidthExit` |
| Basic | `bAreaName` | button | ... (Name) | ARE `Name` all languages | tip: “Edit text in multiple languages” |
| Visual | `bCustomize` | button | Customize Environment | opens TdlgEnvironment | tip: “Edit specific values of the environment”; `OnClick=bCustomizeClick` |
| Visual | `mmWarning` | memo | ⟨Visual⟩ | info text | read-only; text: “Note: Selecting an Environment Scheme from the above list will cause any custom area lighting or weather settings, as well as custom tile lighting, to be lost i” |
| Visual | `lvLightingSchemes` | listview | ⟨Visual⟩ | ARE `LightingScheme` (environment.2da presets; applies sun/moon/fog colours) | `OnMouseDown=lvLightingSchemesMouseDown` |
| Audio | `cbAmbientSoundDay` | combo | Ambient Sound, Day | GIT `AreaProperties.AmbientSndDay` (ambientsound.2da) | max 16; list |
| Audio | `cbAmbientSoundNight` | combo | Ambient Sound, Night | GIT `AreaProperties.AmbientSndNight` (ambientsound.2da) | max 16; list |
| Audio | `cbMusicBattle` | combo | Music, Battle | GIT `AreaProperties.MusicBattle` (ambientmusic.2da) | max 16; list |
| Audio | `cbMusicDay` | combo | Music, Day | GIT `AreaProperties.MusicDay` (ambientmusic.2da) | max 16; list |
| Audio | `cbMusicNight` | combo | Music, Night | GIT `AreaProperties.MusicNight` (ambientmusic.2da) | max 16; list |
| Audio | `eMusicDelay` | edit | Music, Playing Delay | GIT `AreaProperties.MusicDelay` | max 3; spin -99…99; `OnExit=OnMusicDelayExit` |
| Audio | `cbEnvironmentalAudio` | combo | Environmental Audio Effects | GIT `AreaProperties.EnvAudio` (soundeax.2da) | max 16; list |
| Audio | `tbAmbientSoundDayVolume` | slider | Ambient Sound, Day Volume | GIT `AreaProperties.AmbientSndDayVol` | range 0…127 |
| Audio | `tbAmbientSoundNightVolume` | slider | Ambient Sound, Night Volume | GIT `AreaProperties.AmbientSndNitVol` | range 0…127 |
| Events | `cbOnEnter` | combo | OnEnter | ARE `OnEnter` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnExit` | combo | OnExit | ARE `OnExit` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnHeartbeat` | combo | OnHeartbeat | ARE `OnHeartbeat` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `cbOnUserDefined` | combo | OnUserDefined | ARE `OnUserDefined` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Events | `bLoadScriptSet` | button | Load Script Set | read script set (.ini) | `OnClick=bLoadScriptSetClick` |
| Events | `bSaveScriptSet` | button | Save Script Set | write script set (.ini) | `OnClick=bSaveScriptSetClick` |
| Advanced | `eWorldMapIcon` | edit | ⟨Advanced⟩ | (dead) | **hidden**; max 16 |
| Advanced | `bWorldMapIcon` | button | ... | (dead) | **hidden**; `OnClick=bWorldMapIconClick` |
| Advanced | `eWorldX` | edit | ⟨Advanced⟩ | (dead) | **hidden**; max 3; spin 0…100; `OnExit=OnWorldMapXYExit` |
| Advanced | `eWorldY` | edit | ⟨Advanced⟩ | (dead) | **hidden**; max 3; spin 0…100; `OnExit=OnWorldMapXYExit` |
| Advanced | `eTag` | edit | Tag | ARE `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
| Advanced | `eModifierListenCheck` | edit | Check Modifier - Listen | ARE `ModListenCheck` | max 3; spin -99…99; `OnExit=OnCheckModifierExit` |
| Advanced | `eModifierSpotCheck` | edit | Check Modifier - Spot | ARE `ModSpotCheck` | max 3; spin -99…99; `OnExit=OnCheckModifierExit` |
| Advanced | `eResRef` | edit | ResRef | ARE `ResRef` (renames area files) | max 16; tip: “Edit the ResRef”; `OnChange=eResRefChange` |
| Advanced | `eTime` | edit | ⟨Advanced⟩ | (dead) | **hidden**; spin 0…24; `OnExit=OnTimeExit` |
| Advanced | `xbNoRest` | checkbox | No Rest | ARE `NoRest` |  |
| Advanced | `cbPlayerVsPlayer` | combo | Player Vs. Player | ARE `PlayerVsPlayer` (pvpsettings.2da) | max 16; list; tip: “Choose the Player vs. Player settings” |
| Advanced | `rbNotInterior` | radio | Exterior | ARE `Flags` bit 0x1 = 0 | `OnClick=rbInteriorClick` |
| Advanced | `rbInterior` | radio | Interior (no weather effects) (DFM: 'Interior') | ARE `Flags` bit 0x1 (interior) | `OnClick=rbInteriorClick` |
| Advanced | `rbNatural` | radio | Natural | ARE `Flags` bit 0x4 (natural) |  |
| Advanced | `rbNotNatural` | radio | Artificial | ARE `Flags` bit 0x4 = 0 |  |
| Advanced | `rbSubterranean` | radio | Underground | ARE `Flags` bit 0x2 (underground) |  |
| Advanced | `rbNotSubterranean` | radio | Above ground | ARE `Flags` bit 0x2 = 0 |  |
| Advanced | `eLoadScreen` | edit | Loading Screen | ARE `LoadScreenID` (loadscreens.2da) | read-only |
| Advanced | `bBrowseAreaTransitionBitmap` | button | ... | opens TdlgLoadScreen | `OnClick=bBrowseAreaTransitionBitmapClick` |
| Advanced | `bVariablesEdit` | button | ... | ARE `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesEditClick` |
| Comments | `mComments` | memo | ⟨Comments⟩ | ARE `Comments` | max 1024 |
|  | `m_bSaveDefault` | button | Save As Default | save as default area template | **hidden**; `OnClick=m_bSaveDefaultClick` |
|  | `m_bLoadDefault` | button | Load Default | load default area template | `OnClick=m_bLoadDefaultClick` |
|  | `bApply` | button | Apply | apply without closing | `OnClick=bApplyClick` |
|  | `dlgOpen` | file dialog |  | world-map icon (dead) | filter `Icon (*.ico)/*.ico/Bitmap (*.bmp)/*.bmp` |

**Action list `alAreaProperties`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actBrowseResource` | ... |  | `actBrowseResourceExecute` |  |  |
| `actEditScript` | E |  | `actEditScriptExecute` |  |  |
| `actBrowseColor` | ... |  |  |  |  |
| `actBrowseCustomColor` | actBrowseCustomColor |  | `actBrowseCustomColorExecute` |  |  |

**Notes / behaviors:**
- Tileset and dimensions are likely only editable for a new area (`pHideAreaDimensions` panel covers them otherwise); resizing an existing area uses Resize Area.
- Visual tab: lighting-scheme presets (environment.2da) + *Customize Environment* sub-dialog; warning memo (TLK): selecting a scheme discards custom area lighting/weather **and custom tile lighting**.
- `m_bLoadDefault` / hidden `m_bSaveDefault`: area-property defaults template. `bApply` applies to the open viewer without closing.
- Hidden/dead: World Map icon/X/Y, Time.

#### `TdlgEnvironment` — Environment Options
*DFM `TDLGENVIRONMENT` · 395×460 · border bsDialog · StrRef table .data+0x6A238 (26/30 captions matched) · DFM caption 'dlgEnvironment'*

**Purpose:** Customize Environment (area lighting/fog/weather/skybox), opened from Area Properties › Visual.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `pMoonAmbientColor` | color swatch/panel | Moon - Ambient Color | ARE `MoonAmbientColor` (BGR dword) | `OnClick=actBrowseColorExecute` |
|  | `pMoonDiffuseColor` | color swatch/panel | Moon - Diffuse Color | ARE `MoonDiffuseColor` | `OnClick=actBrowseColorExecute` |
|  | `pMoonFogColor` | color swatch/panel | Moon - Fog Color | ARE `MoonFogColor` | `OnClick=actBrowseColorExecute` |
|  | `tbMoonFogAmount` | slider | Moon - Fog Amount | ARE `MoonFogAmount` (old slider) | **hidden**; range 0…200 |
|  | `pSunAmbientColor` | color swatch/panel | Sun - Ambient Color | ARE `SunAmbientColor` | `OnClick=actBrowseColorExecute` |
|  | `pSunDiffuseColor` | color swatch/panel | Sun - Diffuse Color | ARE `SunDiffuseColor` | `OnClick=actBrowseColorExecute` |
|  | `pSunFogColor` | color swatch/panel | Sun - Fog Color | ARE `SunFogColor` | `OnClick=actBrowseColorExecute` |
|  | `tbSunFogAmount` | slider | Sun - Fog Amount | ARE `SunFogAmount` (old slider) | **hidden**; range 0…200 |
|  | `eLightning` | edit | Weather - % Lightning | ARE `ChanceLightning` (%) | spin 0…100; `OnExit=OnWeatherChanceExit` |
|  | `eRain` | edit | Weather - % Rain | ARE `ChanceRain` (%) | max 3; spin 0…100; `OnExit=OnWeatherChanceExit` |
|  | `eSnow` | edit | Weather - % Snow | ARE `ChanceSnow` (%) | max 3; spin 0…100; `OnExit=OnWeatherChanceExit` |
|  | `tbWindPower` | slider | Weather - Wind Power | ARE `WindPower` (0 none,1 weak,2 strong) | range 0…2 |
|  | `xbMoonShadows` | checkbox | Moon - Shadows Enabled | ARE `MoonShadows` |  |
|  | `xbSunShadows` | checkbox | Sun - Shadows | ARE `SunShadows` |  |
|  | `rbDayNightCycle` | radio | Cycle Day and Night | ARE `DayNightCycle`=1 | default on; `OnClick=rbDayNightCycleClick` |
|  | `rbAlwaysDay` | radio | Always Bright (DFM: 'Always Day') | ARE `DayNightCycle`=0, `IsNight`=0 | `OnClick=rbAlwaysDayClick` |
|  | `rbAlwaysNight` | radio | Always Dark (DFM: 'Always Night') | ARE `DayNightCycle`=0, `IsNight`=1 | `OnClick=rbAlwaysNightClick` |
|  | `eShadowOpacity` | edit | Shadow Opacity | ARE `ShadowOpacity` (0–100) | spin 0…100; `OnExit=eShadowOpacityExit` |
|  | `eFogClipDistance` | edit | Fog Clip Distance (m) | ARE `FogClipDist` (EE) | max 4; spin 25…999; `OnExit=eFogClipDistanceExit` |
|  | `cbSkyBox` | combo | Sky Box | ARE `SkyBox` (skyboxes.2da) | list |
|  | `eSunFogAmount` | edit | Sun - Fog Amount | ARE `SunFogAmount` | max 4; spin 0…200; `OnExit=eFogAmountExit` |
|  | `eMoonFogAmount` | edit | Moon - Fog Amount | ARE `MoonFogAmount` | max 4; spin 0…200; `OnExit=eFogAmountExit` |
|  | `bApply` | button | Apply | preview in viewer | `OnClick=bApplyClick` |
|  | `bOK` | button | OK |  | tip: “Accept changes” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `dlgColor` | ColorDialog |  | colour chooser |  |

**Action list `alEnvironment`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actBrowseColor` | actBrowseColor |  | `actBrowseColorExecute` |  |  |

**Notes / behaviors:**
- Colour swatches open the Windows colour dialog (`actBrowseColorExecute`); fog amount sliders are hidden (EE uses numeric edits 0–200 and Fog Clip Distance 25–999 m). `Apply` previews in the viewer.

#### `TdlgAreaWizard` — Area Wizard
*DFM `TDLGAREAWIZARD` · 366×387 · StrRef table .data+0x42F8 (13/13 captions matched)*

**Purpose:** Area Wizard: name + tileset → size → finish.

**Inherits:** `TdlgWizard` (Back/Next/Finish/Cancel/Help buttons + hidden-tab page control `pcSteps`).

**Tabs** (`pcSteps`): tsNameAndTileSet · tsSize · tsFinish

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| tsNameAndTileSet | `eName` | edit | Name: | ARE `Name` + resref/tag derived | `OnChange=eNameChange` |
| tsNameAndTileSet | `lbTileSets` | listbox | Tileset: | ARE `Tileset` (installed *.set) | tip: “Select a Tileset for the Area”; `OnClick=lbTileSetsClick OnDblClick=lbTileSetsDblClick` |
| tsSize | `lbSize` | listbox |  | size presets (Tiny…Huge) | `OnClick=lbSizeClick OnDblClick=lbSizeDblClick` |
| tsSize | `eWidth` | edit | Width | ARE `Width` | spin 2…32; `OnExit=eLengthWidthExit` |
| tsSize | `eHeight` | edit | Height | ARE `Height` | spin 2…32; `OnExit=eLengthWidthExit` |
| tsFinish | `xbLaunchAreaDialog` | checkbox | Launch Area Properties Dialog | open Area Properties after |  |
| tsFinish | `xbOpenNewArea` | checkbox | Open Area in the Area Viewer | open viewer tab after | default on |

#### `TfrmAreaResize` — Resize Area
*DFM `TFRMAREARESIZE` · 338×250 · border bsDialog · StrRef table .data+0x57948 (13/13 captions matched)*

**Purpose:** Resize Area (rows/columns, preset list) and Rotate Area (90/180/270 CW/CCW) — same form, panel switched.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Rotations | `rbCounterClockwise90` | radio | CounterClockwise 90 | rotate area | default on |
| Rotations | `rbCounterClockwise180` | radio | CounterClockwise 180 | rotate area |  |
| Rotations | `rbCounterClockwise270` | radio | CounterClockwise 270 | rotate area |  |
| Rotations | `rbClockwise90` | radio | Clockwise 90 | rotate area |  |
| Rotations | `rbClockwise180` | radio | Clockwise 180 | rotate area |  |
| Rotations | `rbClockwise270` | radio | Clockwise 270 | rotate area |  |
|  | `bOk` | button | Ok |  | tip: “Accept changes” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `eRows` | edit | Rows | ARE `Height` | spin 2…32; tip: “Number of rows in the area”; `OnExit=eLengthWidthExit` |
|  | `eColumns` | edit | Columns | ARE `Width` | spin 2…32; tip: “Number of columns in the area”; `OnExit=eLengthWidthExit` |
|  | `lbSize` | listbox | Defaults | size presets | `OnClick=lbSizeClick` |

**Notes / behaviors:**
- Resize anchors at the south-west tile; shrinking deletes objects on removed tiles; growing repeats edge tiles. Rotation is loss-less.

#### `TdlgTileProperties` — Tile Properties
*DFM `TDLGTILEPROPERTIES` · 306×193 · border bsDialog · StrRef table .data+0xFE15C (13/13 captions matched) · form events: OnShow=FormShow*

**Purpose:** Per-tile lighting/animation properties (ARE `Tile_List` entry).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `bDefaults` | button | Defaults | reset to tileset defaults | `OnClick=bDefaultsClick` |
|  | `xbAnimLoop1` | checkbox | Animation Loop 1 | ARE `Tile_AnimLoop1` | default on; tip: “Check to play this tile animation” |
|  | `xbAnimLoop2` | checkbox | Animation Loop 2 | ARE `Tile_AnimLoop2` | default on; tip: “Check to play this tile animation” |
|  | `xbAnimLoop3` | checkbox | Animation Loop 3 | ARE `Tile_AnimLoop3` | default on; tip: “Check to play this tile animation” |
|  | `pMainLight1` | color swatch/panel | Main Light 1 | ARE `Tile_MainLight1` (lightcolor.2da index) | tip: “Click here to select a color”; `OnClick=pMainLight1Click` |
|  | `pMainLight2` | color swatch/panel | Main Light 2 | ARE `Tile_MainLight2` | tip: “Click here to select a color”; `OnClick=pMainLight2Click` |
|  | `pCustomColor1` | color swatch/panel | Source Light 1 | (dead) | **hidden**; tip: “Click here to select a color”; `OnClick=pCustomColor1Click` |
|  | `pCustomColor2` | color swatch/panel | Source Light 2 | (dead) | **hidden**; tip: “Click here to select a color”; `OnClick=pCustomColor2Click` |
|  | `pSourceLight1` | color swatch/panel | Source Light 1 | ARE `Tile_SrcLight1` | tip: “Click here to select a color”; `OnClick=pSourceLight1Click` |
|  | `pSourceLight2` | color swatch/panel | Source Light 2 | ARE `Tile_SrcLight2` | tip: “Click here to select a color”; `OnClick=pSourceLight2Click` |

**Notes / behaviors:**
- Colour swatches open `TdlgColorSelection` (fixed tile light palette). Applies to all selected tiles.

#### `TdlgColorSelection` — Select A Color
*DFM `TDLGCOLORSELECTION` · 350×174 · border bsDialog · no StrRef table resolved*

**Purpose:** "Select A Color": fixed palette of 32 colour swatches (tile light colours, lightcolor.2da).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `Panel1` | color swatch/panel |  | colour index 1 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel2` | color swatch/panel |  | colour index 2 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel3` | color swatch/panel |  | colour index 3 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel8` | color swatch/panel |  | colour index 8 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel9` | color swatch/panel |  | colour index 9 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel10` | color swatch/panel |  | colour index 10 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel11` | color swatch/panel |  | colour index 11 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel16` | color swatch/panel |  | colour index 16 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel0` | color swatch/panel |  | colour index 0 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel17` | color swatch/panel |  | colour index 17 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel18` | color swatch/panel |  | colour index 18 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel19` | color swatch/panel |  | colour index 19 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel24` | color swatch/panel |  | colour index 24 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel25` | color swatch/panel |  | colour index 25 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel26` | color swatch/panel |  | colour index 26 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel27` | color swatch/panel |  | colour index 27 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `OKButton` | button | OK |  |  |
|  | `CancelButton` | button | Cancel |  |  |
|  | `Panel7` | color swatch/panel |  | colour index 7 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel15` | color swatch/panel |  | colour index 15 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel23` | color swatch/panel |  | colour index 23 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel31` | color swatch/panel |  | colour index 31 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel6` | color swatch/panel |  | colour index 6 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel14` | color swatch/panel |  | colour index 14 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel22` | color swatch/panel |  | colour index 22 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel30` | color swatch/panel |  | colour index 30 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel5` | color swatch/panel |  | colour index 5 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel13` | color swatch/panel |  | colour index 13 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel21` | color swatch/panel |  | colour index 21 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel29` | color swatch/panel |  | colour index 29 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel4` | color swatch/panel |  | colour index 4 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel12` | color swatch/panel |  | colour index 12 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel20` | color swatch/panel |  | colour index 20 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |
|  | `Panel28` | color swatch/panel |  | colour index 28 (tile light / lightcolor.2da) | `OnClick=actHitPanelExecute OnDblClick=actDblHitPanelExecute` |

**Action list `ActionList1`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actHitPanel` | actHitPanel |  | `actHitPanelExecute` |  |  |
| `actDblHitPanel` | actDblHitPanel |  | `actDblHitPanelExecute` |  |  |

#### `TdlgAreaTransition` — Area Transition
*DFM `TDLGAREATRANSITION` · StrRef table .data+0xE04 (13/16 captions matched) · form events: OnPaint=FormPaint, OnResize=FormResize, OnShow=FormShow*

**Purpose:** Setup Area Transition: pick target area and target door/trigger/waypoint on a minimap; links LinkedTo/LinkedToFlags on one or both ends.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `cbAreas` | combo | Target Area | target area | list; items: Area001 / Area002 / Area003 / Area004 / Area005 / Area006 / Area007; `OnChange=cbAreasChange` |
| Connection Type | `rbBothWays` | radio | Both Ways | link both ends | default on |
| Connection Type | `rbCurrentToTarget` | radio | Current To Target | link source only |  |
| Connection Type | `rbTargetToCurrent` | radio | Target To Current | link target only |  |
| Target Type | `rbTargetDoor` | radio | Door | target kind → `LinkedToFlags`=1 | `OnClick=rbTargetTypeClick` |
| Target Type | `rbTargetWaypoint` | radio | Waypoint | target kind → `LinkedToFlags`=2 | `OnClick=rbTargetTypeClick` |
| Target Type | `rbTargetTrigger` | radio | Trigger | target kind (trigger↔trigger) | default on; `OnClick=rbTargetTypeClick` |
|  | `bOK` | button | OK |  | `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |
|  | `lbAreaDoors` | listbox | Available Doors: | target object → `LinkedTo` = its tag | `OnClick=lbAreaDoorsClick` |
|  | `lbAreaTriggers` | listbox | Available Triggers: | target object | `OnClick=lbAreaTriggersClick` |
|  | `lbAreaWaypoints` | listbox | Available Waypoints: | target object | `OnClick=lbAreaWaypointsClick` |
|  | `apArea` | 3D view |  | target-area minimap (click to pick) | `OnMouseDown=apAreaMouseDown` |

**Notes / behaviors:**
- `apArea` renders the target area top-down with doors/triggers/waypoints; clicking an object selects it in the lists. Status label under the map. Linking writes `LinkedTo` = target tag and `LinkedToFlags` on source (and target when *Both Ways*).

#### `TdlgLocation` — Adjust Location
*DFM `TDLGLOCATION` · 434×303 · border bsDialog · StrRef table .data+0x4B4F0 (9/9 captions matched) · DFM caption 'Adjust Position/Orientation'*

**Purpose:** Adjust Location: exact position, bearing (with drag dial) and EE visual transform of the selected instance(s).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |
| Position | `ePositionX` | edit | X | instance `XPosition`/`X` | spin 0…100; `OnExit=ePositionXExit OnKeyDown=OnEditUpDownKeyDown` |
| Position | `ePositionZ` | edit | Z | instance `ZPosition`/`Z` | spin 0…100; `OnExit=ePositionZExit OnKeyDown=OnEditUpDownKeyDown` |
| Position | `ePositionY` | edit | Y | instance `YPosition`/`Y` | spin 0…100; `OnExit=ePositionYExit OnKeyDown=OnEditUpDownKeyDown` |
| Orientation | `pOrientationAngle` | color swatch/panel | Bearing | bearing dial (drag) | `OnMouseDown=pOrientationAngleMouseDown` |
| Orientation | `eBearing` | edit | Bearing | instance `Bearing` or `XOrientation`/`YOrientation` | spin 0…100; `OnChange=eBearingChange OnExit=eBearingExit OnKeyDown=OnEditUpDownKeyDown` |
| Visual Transforms | `eScale` | edit | Scale | EE VisualTransform scale | spin 0…100; `OnExit=eScaleExit OnKeyDown=OnEditUpDownKeyDown` |
| Visual Transforms | `eRotationY` | edit | Y Rotation | EE VisualTransform rotate Y | spin 0…100; `OnExit=eRotationYExit OnKeyDown=OnEditUpDownKeyDown` |
| Visual Transforms | `eRotationX` | edit | X Rotation | EE VisualTransform rotate X | spin 0…100; `OnExit=eRotationXExit OnKeyDown=OnEditUpDownKeyDown` |
| Visual Transforms | `eRotationZ` | edit | Z Rotation | EE VisualTransform rotate Z | spin 0…100; `OnExit=eRotationZExit OnKeyDown=OnEditUpDownKeyDown` |
| Visual Transforms | `eTranslationX` | edit | X Translation | EE VisualTransform translate X | spin 0…100; `OnExit=eTranslationXExit OnKeyDown=OnEditUpDownKeyDown` |
| Visual Transforms | `eTranslationY` | edit | Y Translation | EE VisualTransform translate Y | spin 0…100; `OnExit=eTranslationYExit OnKeyDown=OnEditUpDownKeyDown` |
| Visual Transforms | `eTranslationZ` | edit | Z Translation | EE VisualTransform translate Z | spin 0…100; `OnExit=eTranslationZExit OnKeyDown=OnEditUpDownKeyDown` |
|  | `bApply` | button | Apply | apply without closing | `OnClick=bApplyClick` |

**Notes / behaviors:**
- Keyboard up/down on edits nudges values (`OnEditUpDownKeyDown`). Applies to all selected instances. Visual Transform group has no Tags (EE, English only).

#### `TdlgAddPopupText` — Add Popup Text
*DFM `TDLGADDPOPUPTEXT` · 306×239 · border bsDialog · StrRef table .data+0x4C4 (3/5 captions matched) · DFM caption 'dlgAddPopupText'*

**Purpose:** Add Popup Text: creates a one-line conversation (resref given) and assigns it to the selected object (e.g. placeable examine-speak).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mText` | memo | Popup Text | DLG single entry `Text` | tip: “Edit the Name in the primary language” |
|  | `eFileName` | edit | Conversation File | new DLG resref | max 16; `OnChange=eFileNameChange` |
|  | `bText` | button | ... | all languages | tip: “Edit text in multiple languages” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bOK` | button | OK |  | disabled; tip: “Accept changes”; `OnClick=bOKClick` |

#### `TdlgFindInstance` — Find Instance
*DFM `TDLGFINDINSTANCE` · 412×371 · border bsDialog · StrRef table .data+0x36B14 (7/9 captions matched) · form events: OnActivate=FormActivate, OnClose=FormClose, OnCreate=FormCreate · DFM caption 'dlgFindInstance'*

**Purpose:** Find Instance: search placed objects across the module.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bDone` | button | Done |  | `OnClick=bDoneClick` |
|  | `bSearch` | button | Search |  | `Action=actSearch` |
|  | `xlbTypes` | check-listbox | Search For | object types to search | `OnClickCheck=xlbTypesClickCheck` |
|  | `cbArea` | combo | In Area | area filter (or all) | max 16; list |
|  | `eTemplate` | edit | From Blueprint | `TemplateResRef` filter | max 16 |
|  | `eTag` | edit | With Tag | `Tag` filter |  |
|  | `bClear` | button | Clear | reset criteria | tip: “Clear History”; `Action=actClear` |
|  | `lvResults` | listview |  | results (dbl-click = go to) | cols: Type / Tag / Area / Template; `OnColumnClick=lvResultsColumnClick OnDblClick=lvResultsDblClick` |

**Action list `actlMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actSearch` | Search |  | `actSearchExecute` |  |  |
| `actClear` | Clear |  | `actClearExecute` |  | Clear History |

#### `TdlgSystemUsage` — Resources Used
*DFM `TDLGSYSTEMUSAGE` · 133×93 · border bsDialog · StrRef table .data+0xFB474 (3/4 captions matched) · DFM caption 'dlgSystemUsage'*

**Purpose:** Area Statistics: model/texture memory usage of the current area.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bDone` | button | Done |  |  |

#### `TfrmPreview` — Preview
*DFM `TFRMPREVIEW` · StrRef table .data+0xC88F4 (23/25 captions matched) · form events: OnClose=FormClose, OnPaint=FormPaint*

**Purpose:** Non-modal Object Preview window: 3D (or 2D icon) render of the selected blueprint/instance plus read-only summary fields per object type.

**Tabs** (`pcProperties`): Creature · Door · Encounter · Item · Placeable · Sound · Store · Trigger · Waypoint · Terrain (hidden tab)

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `apPreview` | 3D view |  | model preview | `OnMouseDown=apPreviewMouseDown` |
| Creature | `eChallengeRating` | edit | Challenge Rating | UTC `ChallengeRating` | read-only; max 1023 |
| Creature | `eCreatureFaction` | edit | Faction | `FactionID` name | read-only; max 1023 |
| Door | `eDoorTrapType` | edit | Trap Type | `TrapType` (traps.2da) | read-only; max 1023 |
| Door | `eDoorFaction` | edit | Faction | `Faction` | read-only; max 1023 |
| Door | `eDoorDestinationTag` | edit | Destination Tag | `LinkedTo` | read-only; max 1023 |
| Door | `xbDoorLocked` | checkbox | (no caption; runtime) | `Locked` | disabled |
| Encounter | `eDifficulty` | edit | Difficulty | UTE `DifficultyIndex` | read-only; max 1023 |
| Encounter | `eSpawnOption` | edit | Spawn Option | UTE `SpawnOption` | read-only; max 1023 |
| Encounter | `eEncounterFaction` | edit | Faction | `Faction` | read-only; max 1023 |
| Item | `eItemCost` | edit | Total Cost | UTI cost | read-only; max 1023 |
| Item | `rb2D` | radio | 2D | preview mode | `OnClick=rbDClick` |
| Item | `rb3D` | radio | 3D | preview mode | default on; `OnClick=rbDClick` |
| Placeable | `ePlaceableFaction` | edit | Faction | `Faction` | read-only; max 1023 |
| Placeable | `ePlaceableTrapType` | edit | Trap Type | `TrapType` | read-only; max 1023 |
| Placeable | `xbPlaceableLocked` | checkbox | (no caption; runtime) | `Locked` | disabled |
| Sound | `eVolume` | edit | Volume | UTS `Volume` | read-only; max 1023 |
| Sound | `xbActive` | checkbox | (no caption; runtime) | UTS `Active` | disabled |
| Trigger | `eTriggerType` | edit | Trigger Type | UTT `Type` | read-only; max 1023 |
| Trigger | `eTriggerDestinationTag` | edit | Destination Tag | `LinkedTo` | read-only; max 1023 |
| Trigger | `eTriggerFaction` | edit | Faction | `Faction` | read-only; max 1023 |
| Trigger | `eTriggerTrapType` | edit | Trap Type | `TrapType` | read-only; max 1023 |
|  | `eTag` | edit | Tag | `Tag` | read-only; max 1023 |
|  | `eName` | edit | Name | name | read-only; max 1023 |
|  | `mComments` | memo | Comments | `Comment` | read-only |
|  | `eResRef` | edit | Blueprint ResRef | `TemplateResRef` | read-only; max 1023 |

**Notes / behaviors:**
- One property tab per object type is shown (tabs hidden); `pCommon` shows Name/Tag/ResRef/Comments for all. Mouse on `apPreview` rotates/zooms the model; items can switch 2D icon / 3D model.

#### `TfrmObjectList` — Inaccessable Objects
*DFM `TFRMOBJECTLIST` · no StrRef table resolved*

**Purpose:** "Inaccessable Objects" list: objects that cannot be picked in the viewer; double-click focuses.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `tvObjectList` | tree |  | unreachable instances | `OnDblClick=tvObjectListDblClick` |

## 4. Shared "situated" frames and dialogs (doors, placeables, multi-edit, traps)

#### `TdlgSituatedEdit` — Situated Object Properties
*DFM `TDLGSITUATEDEDIT` · 558×367 · border bsDialog · no StrRef table resolved · form events: OnClose=FormClose, OnPaint=FormPaint, OnShow=FormShow*

**Purpose:** Base properties dialog for situated objects (doors, placeables) composed of shared frames; also used directly.

**Tabs** (`pcProperties`): Basic · Lock · Trap · Events · Advanced · Description · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `apAppearance` | 3D view |  | model preview (drag rotate, wheel zoom) | `OnMouseDown=apAppearanceMouseDown` |
|  | `bOK` | button | OK |  |  |
|  | `bCancel` | button | Cancel |  |  |
|  | `bDefaults` | button | Defaults | (hidden) reset | **hidden** |
| Basic | `fraSituatedBasic1` | embedded frame | `TfraSituatedBasic` |  | see frame section; overrides: eFort, eHP, eHardness, eRef, eWill, lHardness, udHardness |
| Lock | `fraSituatedLock1` | embedded frame | `TfraSituatedLock` |  | see frame section |
| Trap | `xbTrapFlag` | checkbox | Is Trapped | `TrapFlag` | `OnClick=xbTrapFlagClick` |
| Trap › Trap Settings | `pTrap` | color swatch/panel | ⟨Trap Settings⟩ | hosts `TfrmTrap` (runtime) |  |
| Events | `fraSituatedScripts1` | embedded frame | `TfraSituatedScripts` |  | see frame section |
| Advanced | `fraSituatedAdvanced1` | embedded frame | `TfraSituatedAdvanced` |  | see frame section; overrides: bEditConversation, cbConversation |
| Description | `fraSituatedDesc1` | embedded frame | `TfraSituatedDesc` |  | see frame section |
| Comments | `fraSituatedComments1` | embedded frame | `TfraSituatedComments` |  | see frame section |

**Notes / behaviors:**
- Tab set shared by doors/placeables; frames: Basic=`TfraSituatedBasic`, Lock=`TfraSituatedLock`, Trap=`xbTrapFlag` + `TfrmTrap` created into `pTrap` at runtime, Events=`TfraSituatedScripts`, Advanced=`TfraSituatedAdvanced`, Description=`TfraSituatedDesc`, Comments=`TfraSituatedComments`. 3D preview (`apAppearance`) supports mouse rotate/zoom.

#### `TfraSituatedBasic` — (no caption)
*DFM `TFRASITUATEDBASIC` · StrRef table .data+0xEFB38 (9/9 captions matched)*

**Purpose:** Shared frame: name, tag, appearance, HP, hardness, saves, plot.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bName` | button | ... | `LocName` all languages |  |
|  | `eName` | edit | Name | `LocName` |  |
|  | `eTag` | edit | Tag | `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
|  | `cbAppearanceType` | combo | Appearance Type | `Appearance` (placeables.2da / doortypes.2da) | list |
|  | `eFort` | edit | Fortitude Save | `Fort` | spin 0…100; `OnChange=eEditChange OnExit=eSaveExit` |
|  | `eHP` | edit | Hit Points | `HP` (+`CurrentHP`) | spin 0…100; `OnChange=eEditChange OnExit=eHPExit` |
|  | `eWill` | edit | Will Save | `Will` | spin 0…100; `OnChange=eEditChange OnExit=eSaveExit` |
|  | `eRef` | edit | Reflex Save | `Ref` | spin 0…100; `OnChange=eEditChange OnExit=eSaveExit` |
|  | `eHardness` | edit |  | `Hardness` | **hidden**; spin 0…100; `OnChange=eEditChange` |
|  | `xbPlot` | checkbox | Plot | `Plot` | `OnClick=xbPlotClick` |

**Notes / behaviors:**
- Hardness hidden in the frame; shown by TdlgSituatedEdit. Saves/HP validated on exit.

#### `TfraSituatedLock` — (no caption)
*DFM `TFRASITUATEDLOCK` · StrRef table .data+0xF0AE8 (5/6 captions matched)*

**Purpose:** Shared frame: lock settings.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `xbAutoRemoveKey` | checkbox | Automatically remove key after use | `AutoRemoveKey` | `OnClick=xbClick` |
|  | `eKeyName` | edit | Key Tag | `KeyName` | max 32; `OnChange=eChange` |
|  | `eOpenLockDC` | edit | Open Lock DC | `OpenLockDC` | spin 0…100; `OnChange=eChange OnExit=eExit` |
|  | `xbKeyRequired` | checkbox | Key required to unlock or lock (DFM: 'Key required to unlock or relock') | `KeyRequired` | `OnClick=xbClick` |
|  | `xbLocked` | checkbox | Locked | `Locked` | `OnClick=xbClick` |
|  | `xbLockable` | checkbox | Can be relocked | `Lockable` | `OnClick=xbClick` |
|  | `eCloseLockDC` | edit | Close Lock DC | `CloseLockDC` | spin 0…100; `OnChange=eChange OnExit=eExit` |

#### `TfrmTrap` — (no caption)
*DFM `TFRMTRAP` · StrRef table .data+0xF2BA0 (10/12 captions matched)*

**Purpose:** Trap settings frame hosted in the Trap tab of doors, placeables and triggers.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `eDisarmDC` | edit | Disarm DC | `DisarmDC` | spin 0…100; `OnExit=eDCExit` |
|  | `eDetectionDC` | edit | Detection DC | `TrapDetectDC` | spin 0…100; `OnExit=eDCExit` |
|  | `cbTrapType` | combo | Trap Type | `TrapType` (traps.2da) | list; tip: “Select the Trap Type.”; `OnChange=cbTrapTypeChange` |
|  | `cbOnDisarm` | combo | OnDisarm | `OnDisarm` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbEventChange` |
|  | `cbOnTrapTriggered` | combo | OnTrapTriggered | `OnTrapTriggered` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbEventChange` |
|  | `eDetectDCMod` | edit | Detect DC Mod. when set by Rogue | traps.2da `DetectDCMod` (read-only) | read-only |
|  | `eDisarmDCMod` | edit | Disarm DC Mod. when set by Rogue | traps.2da `DisarmDCMod` (read-only) | read-only |
|  | `eSetDC` | edit | Set DC | traps.2da `SetDC` (read-only) | read-only |
|  | `xbTrapDisarmable` | checkbox | Disarmable | `TrapDisarmable` | `OnClick=xbTrapDisarmableClick` |
|  | `xbTrapOneShot` | checkbox | One Shot | `TrapOneShot` |  |
|  | `xbTrapDetectable` | checkbox | Detectable | `TrapDetectable` | `OnClick=xbTrapDetectableClick` |

#### `TfraSituatedTrap` — (no caption)
*DFM `TFRASITUATEDTRAP` · StrRef table .data+0xF2BA0 (10/12 captions matched)*

**Purpose:** Shared trap frame (variant; `TfrmTrap` is the one actually hosted in `pTrap`).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `eDisarmDC` | edit | Disarm DC | `DisarmDC` | spin 0…100 |
|  | `eDetectionDC` | edit | Detection DC | `TrapDetectDC` | spin 0…100 |
|  | `cbTrapType` | combo | Trap Type | `TrapType` (traps.2da) | editable; tip: “Select the Trap Type.” |
|  | `cbOnDisarm` | combo | OnDisarm | `OnDisarm` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
|  | `cbOnTrapTriggered` | combo | OnTrapTriggered | `OnTrapTriggered` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
|  | `eDetectDCMod` | edit | Detect DC Mod. when set by Rogue | traps.2da `DetectDCMod` (read-only) | read-only |
|  | `eDisarmDCMod` | edit | Disarm DC Mod. when set by Rogue | traps.2da `DisarmDCMod` (read-only) | read-only |
|  | `eSetDC` | edit | Set DC | traps.2da `SetDC` (read-only) | read-only |
|  | `xbTrapDisarmable` | checkbox | Disarmable | `TrapDisarmable` |  |
|  | `xbTrapOneShot` | checkbox | One Shot | `TrapOneShot` |  |
|  | `xbTrapDetectable` | checkbox | Detectable | `TrapDetectable` |  |

#### `TfraSituatedScripts` — (no caption)
*DFM `TFRASITUATEDSCRIPTS` · StrRef table .data+0xF1078 (25/30 captions matched)*

**Purpose:** Shared frame: event scripts for doors/placeables (panels hidden per object type).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `cbOnClosed` | combo | OnClose | `OnClosed` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnDamaged` | combo | OnDamaged | `OnDamaged` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnDeath` | combo | OnDeath | `OnDeath` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnLock` | combo | OnLock | `OnLock` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnMeleeAttacked` | combo | OnPhysicalAttacked | `OnMeleeAttacked` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnOpen` | combo | OnOpen | `OnOpen` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnSpellCastAt` | combo | OnSpellCastAt | `OnSpellCastAt` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnUnlock` | combo | OnUnLock | `OnUnlock` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnUserDefined` | combo | OnUserDefined | `OnUserDefined` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnHeartbeat` | combo | OnHeartbeat | `OnHeartbeat` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnClick` | combo | OnAreaTransitionClick | `OnClick` (EE) | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnInvDisturbed` | combo | OnDisturbed | UTP `OnInvDisturbed` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnUsed` | combo | OnUsed | UTP `OnUsed` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `cbOnFailToOpen` | combo | OnFailToOpen | UTD `OnFailToOpen` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnEventChange OnClick=cbOnEventChange` |
|  | `bLoadScriptSet` | button | Load Script Set | read script set | `OnClick=bLoadScriptSetClick` |
|  | `bSaveScriptSet` | button | Save Script Set | write script set | `OnClick=bSaveScriptSetClick` |

**Notes / behaviors:**
- Panels are shown per object type: doors hide OnInvDisturbed/OnUsed; placeables hide OnFailToOpen (EE shows OnClick for both).

#### `TfraSituatedAdvanced` — (no caption)
*DFM `TFRASITUATEDADVANCED` · StrRef table .data+0xEEC40 (7/9 captions matched)*

**Purpose:** Shared frame: faction, template resref, conversation, portrait, interruptable, open state, variables.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `cbFaction` | combo | Belongs to Faction | `Faction` (repute.fac) | list; `OnChange=eTextChange` |
|  | `eTemplate` | edit | Blueprint ResRef | `TemplateResRef` | read-only; max 16; tip: “Edit the Blueprint ResRef”; `OnChange=eTextChange` |
|  | `cbConversation` | combo | Conversation | `Conversation` | buttons […] [Edit]; max 16; editable; `OnChange=cbConversationChange` |
|  | `imgPortrait` | image |  | `PortraitId` preview | `OnClick=imgPortraitClick` |
|  | `ePortrait` | edit |  | `PortraitId` (portraits.2da) | read-only; max 16; `OnChange=eTextChange` |
|  | `bPortrait` | button | ... | opens TdlgPortrait | `OnClick=imgPortraitClick` |
|  | `bUpdateInstancesInArea` | button | Update instances in current area | push blueprint to instances | **hidden** |
|  | `xbInterrupt` | checkbox | No Interrupt | `Interruptable` (inverted) | `OnClick=xbInterruptClick` |
|  | `cbOpenState` | combo | Initial State | `AnimationState` (initial open/closed/destroyed…) | list |
|  | `bVariables` | button | ... (Variables) | `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesClick` |

**Notes / behaviors:**
- Template ResRef read-only for instances; `cbOpenState` items depend on object type (door: open/closed; placeable: default/open/closed/destroyed/activated/deactivated).

#### `TfraSituatedDesc` — (no caption)
*DFM `TFRASITUATEDDESC` · no StrRef table resolved*

**Purpose:** Shared frame: localised description.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mDescription` | memo | Description | `Description` | `OnChange=mDescriptionChange` |
|  | `bDescription` | button | ... | all languages |  |

#### `TfraSituatedComments` — (no caption)
*DFM `TFRASITUATEDCOMMENTS` · no StrRef table resolved*

**Purpose:** Shared frame: designer comment.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mComments` | memo |  | `Comment` |  |

#### `TdlgSituatedMultiEditor` — Door Properties
*DFM `TDLGSITUATEDMULTIEDITOR` · 373×359 · border bsDialog · StrRef table .data+0xF2D10 (7/8 captions matched) · DFM caption 'dlgSituatedMultiEditor'*

**Purpose:** Multi-object editor for several selected situated objects (common subset of fields).

**Tabs** (`pcMain`): Basic · Advanced · Events · Description · Lock · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Basic | `fraSituatedBasic1` | embedded frame | `TfraSituatedBasic` |  | see frame section |
| Advanced | `fraSituatedAdvanced1` | embedded frame | `TfraSituatedAdvanced` |  | see frame section |
| Events | `fraSituatedScripts1` | embedded frame | `TfraSituatedScripts` |  | see frame section |
| Description | `fraSituatedDesc1` | embedded frame | `TfraSituatedDesc` |  | see frame section |
| Lock | `fraSituatedLock1` | embedded frame | `TfraSituatedLock` |  | see frame section |
| Comments | `fraSituatedComments1` | embedded frame | `TfraSituatedComments` |  | see frame section |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |

**Notes / behaviors:**
- Shows a subset of each frame (Basic: name/tag/appearance; Advanced: faction/conversation/portrait; all scripts; description; lock; comments). Fields left blank/indeterminate are not written.

## 5. Creature editor (+ wizard, level-up, colours, sound set, inventory)

#### `TdlgCreatureEdit` — Creature Properties
*DFM `TDLGCREATUREEDIT` · 788×640 · border bsDialog · StrRef table .data+0x13DB8 (150/159 captions matched) · form events: OnClose=FormClose, OnPaint=FormPaint, OnShow=FormShow · DFM caption 'Creature Editor'*

**Purpose:** Creature Properties (UTC blueprint or GIT creature instance), with live 3D preview.

**Tabs** (`pcProperties`): Basic · Statistics · Appearance · Classes · Skills · Scripts · Advanced · Feats · Spells · Special Abilities · Template (hidden tab) · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Basic › Profile | `bRandomName` | speed button | (no caption; glyph) | random `FirstName` (race .ltr) | tip: “Generate a random name”; `OnClick=bRandomNameClick` |
| Basic › Profile | `bRandomLastName` | speed button | (no caption; glyph) | random `LastName` | tip: “Generate a random name”; `OnClick=bRandomLastNameClick` |
| Basic › Profile | `bUniqueTag` | speed button | (no caption; glyph) | generate unique `Tag` | tip: “Press this button to assign a unique Tag”; `OnClick=bUniqueTagClick` |
| Basic › Profile | `eFirstName` | edit | First Name | UTC `FirstName` | tip: “Edit the first name in the primary language” |
| Basic › Profile | `bLocFirstName` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Basic › Profile | `eLastName` | edit | Last Name | UTC `LastName` | tip: “Edit the last name in the primary language” |
| Basic › Profile | `cbRace` | combo | Race | UTC `Race` (racialtypes.2da) | list; tip: “Select a race from the dropdown list”; `OnChange=cbRaceChange` |
| Basic › Profile | `cbPhenotype` | combo | Phenotype | UTC `Phenotype` (phenotype.2da) | list; tip: “Select a phenotype from the dropdown list”; `OnChange=cbPhenotypeChange` |
| Basic › Profile | `cbGenders` | combo | Gender | UTC `Gender` (gender.2da) | list; tip: “Select a gender from the dropdown list”; `OnChange=cbGendersChange` |
| Basic › Profile | `eDescription` | edit | Description | UTC `Description` | tip: “Edit the description in the primary language” |
| Basic › Profile | `bLocDescription` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Basic › Profile | `bLocLastName` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Basic › Profile | `cbAppearance` | combo | Appearance | UTC `Appearance_Type` (appearance.2da) | list; tip: “Select an appearance from the dropdown list”; `OnChange=cbAppearanceChange` |
| Basic › Profile | `eTag` | edit | Tag | UTC `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
| Basic › Profile | `eCRBasic` | edit | Challenge Rating | UTC `ChallengeRating` (display) | read-only |
| Basic › Profile | `ePaletteCategory` | edit | Category | UTC `PaletteID` (creaturepal.itp) | read-only |
| Basic › Profile | `bPaletteCategory` | button | ... | opens TdlgPaletteChooser | tip: “Press this button to select a new category for this Blueprint”; `OnClick=bPaletteCategoryClick` |
| Basic › Portrait | `imgPortrait` | image |  | portrait preview | `OnClick=bPortraitClick` |
| Basic › Portrait | `bPortrait` | button | ... | opens TdlgPortrait | tip: “Select a portrait”; `OnClick=bPortraitClick` |
| Basic › Portrait | `ePortrait` | edit | ⟨Portrait⟩ | UTC `PortraitId` (portraits.2da) / `Portrait` | read-only; max 16 |
| Basic › Conversation | `cbConversation` | combo | ⟨Conversation⟩ | UTC `Conversation` | buttons […] [Edit]; max 16; editable; tip: “Select a Conversation file”; `OnChange=cbConversationChange` |
| Basic › Conversation | `xbNoInterrupt` | checkbox | No Interrupt | UTC `Interruptable` (inverted) | tip: “Specify if this conversation can be interrupted” |
| Statistics › Ability Scores | `eStrength` | edit | Strength | UTC `Str` | spin 3…100; tip: “Edit ability score”; `OnExit=eStrengthExit OnKeyDown=eAbilityKeyDown` |
| Statistics › Ability Scores | `eDexterity` | edit | Dexterity | UTC `Dex` | spin 3…100; tip: “Edit ability score”; `OnExit=eDexterityExit OnKeyDown=eAbilityKeyDown` |
| Statistics › Ability Scores | `eConstitution` | edit | Constitution | UTC `Con` | spin 3…100; tip: “Edit ability score”; `OnExit=eConstitutionExit OnKeyDown=eAbilityKeyDown` |
| Statistics › Ability Scores | `eIntelligence` | edit | Intelligence | UTC `Int` | spin 3…100; tip: “Edit ability score”; `OnExit=eIntelligenceExit OnKeyDown=eAbilityKeyDown` |
| Statistics › Ability Scores | `eWisdom` | edit | Wisdom | UTC `Wis` | spin 3…100; tip: “Edit ability score”; `OnExit=eWisdomExit OnKeyDown=eAbilityKeyDown` |
| Statistics › Ability Scores | `eCharisma` | edit | Charisma | UTC `Cha` | spin 3…100; tip: “Edit ability score”; `OnExit=eCharismaExit OnKeyDown=eAbilityKeyDown` |
| Statistics › Ability Scores | `eStrengthBonus` | edit | Strength — Bonus | derived | read-only |
| Statistics › Ability Scores | `eDexterityBonus` | edit | Dexterity — Bonus | derived | read-only |
| Statistics › Ability Scores | `eConstitutionBonus` | edit | Constitution — Bonus | derived | read-only |
| Statistics › Ability Scores | `eIntelligenceBonus` | edit | Intelligence — Bonus | derived | read-only |
| Statistics › Ability Scores | `eWisdomBonus` | edit | Wisdom — Bonus | derived | read-only |
| Statistics › Ability Scores | `eCharismaBonus` | edit | Charisma — Bonus | derived | read-only |
| Statistics › Ability Scores | `eRacialStrMod` | edit | Strength — Racial Modifier | racialtypes.2da `StrAdjust` | read-only |
| Statistics › Ability Scores | `eRacialDexMod` | edit | Dexterity — Racial Modifier | racialtypes.2da `DexAdjust` | read-only |
| Statistics › Ability Scores | `eRacialConMod` | edit | Constitution — Racial Modifier | racialtypes.2da `ConAdjust` | read-only |
| Statistics › Ability Scores | `eRacialIntMod` | edit | Intelligence — Racial Modifier | racialtypes.2da `IntAdjust` | read-only |
| Statistics › Ability Scores | `eRacialWisMod` | edit | Wisdom — Racial Modifier | racialtypes.2da `WisAdjust` | read-only |
| Statistics › Ability Scores | `eRacialChaMod` | edit | Charisma — Racial Modifier | racialtypes.2da `ChaAdjust` | read-only |
| Statistics › Ability Scores | `eStrTotal` | edit | Strength — Total | derived | read-only |
| Statistics › Ability Scores | `eDexTotal` | edit | Dexterity — Total | derived | read-only |
| Statistics › Ability Scores | `eConTotal` | edit | Constitution — Total | derived | read-only |
| Statistics › Ability Scores | `eIntTotal` | edit | Intelligence — Total | derived | read-only |
| Statistics › Ability Scores | `eWisTotal` | edit | Wisdom — Total | derived | read-only |
| Statistics › Ability Scores | `eChaTotal` | edit | Charisma — Total | derived | read-only |
| Statistics › Armor Class | `eNaturalAC` | edit | Natural AC | UTC `NaturalAC` | spin 0…100; tip: “Edit the natural armor class value”; `OnExit=eNaturalACExit OnKeyDown=eNaturalACKeyDown` |
| Statistics › Armor Class | `eACBase` | edit | Base | derived (10) | read-only |
| Statistics › Armor Class | `eACBonus` | edit | Dexterity Bonus | derived | read-only |
| Statistics › Armor Class | `eACSize` | edit | Size Modifier | appearance.2da `SIZECATEGORY` → creaturesize.2da | read-only |
| Statistics › Armor Class | `eACTotal` | edit | Total Armor Class | derived | read-only |
| Statistics › Saves | `eSaveReflex` | edit | Reflex — Base | classes.2da save tables | read-only; tip: “Edit saving throw values” |
| Statistics › Saves | `eSaveWill` | edit | Will — Base | classes.2da save tables | read-only; tip: “Edit saving throw values” |
| Statistics › Saves | `eSaveFortitude` | edit | Fortitude — Base | classes.2da save tables | read-only; tip: “Edit saving throw values” |
| Statistics › Saves | `eSaveFortitudeBonus` | edit | Fortitude — Bonus | UTC `fortbonus` | spin -100…100; `OnExit=eSaveExit OnKeyDown=eSaveKeyDown` |
| Statistics › Saves | `eSaveReflexBonus` | edit | Reflex — Bonus | UTC `refbonus` | spin -100…100; `OnExit=eSaveExit OnKeyDown=eSaveKeyDown` |
| Statistics › Saves | `eSaveWillBonus` | edit | Will — Bonus | UTC `willbonus` | spin -100…100; `OnExit=eSaveExit OnKeyDown=eSaveKeyDown` |
| Statistics › Saves | `eSaveFortitudeTotal` | edit | Fortitude — Total | derived | read-only |
| Statistics › Saves | `eSaveReflexTotal` | edit | Reflex — Total | derived | read-only |
| Statistics › Saves | `eSaveWillTotal` | edit | Will — Total | derived | read-only |
| Statistics › Saves | `eFortModifier` | edit | Fortitude — Racial/Ability Modifier | racial/feat mods | read-only |
| Statistics › Saves | `eReflexModifier` | edit | Reflex — Racial/Ability Modifier | racial/feat mods | read-only |
| Statistics › Saves | `eWillModifier` | edit | Will — Racial/Ability Modifier | racial/feat mods | read-only |
| Statistics › Hit Points | `eHitPoints` | edit | Base Hit Points | UTC `HitPoints` (+`CurrentHitPoints`,`MaxHitPoints`) | spin 0…100; tip: “Edit base hit points”; `OnExit=eHitPointsExit OnKeyDown=eHitPointsKeyDown` |
| Statistics › Hit Points | `eHPBonus` | edit | Hit Point Bonuses | derived | read-only |
| Statistics › Hit Points | `eHPTotal` | edit | Total Hit Points | derived | read-only |
| Statistics › Speed | `cbMovementRate` | combo | Movement Rate | UTC `WalkRate` (creaturespeed.2da) | list |
| Appearance | `bColor` | button | Color ... | opens TColorPicker → `Color_Skin`,`Color_Hair`,`Color_Tattoo1`,`Color_Tattoo2` | tip: “Select color”; `OnClick=bColorClick` |
| Appearance | `cbThighRight` | combo | Right Thigh | UTC `BodyPart_RThigh` (parts_legs) | list; tip: “Select model”; `OnChange=cbThighRightChange` |
| Appearance | `cbShinRight` | combo | Right Shin | UTC `BodyPart_RShin` | list; tip: “Select model”; `OnChange=cbShinRightChange` |
| Appearance | `cbFootRight` | combo | Right Foot | UTC `ArmorPart_RFoot` (sic) | list; tip: “Select model”; `OnChange=cbFootRightChange` |
| Appearance | `cbThighLeft` | combo | Left Thigh | UTC `BodyPart_LThigh` | list; tip: “Select model”; `OnChange=cbThighLeftChange` |
| Appearance | `cbShinLeft` | combo | Left Shin | UTC `BodyPart_LShin` | list; tip: “Select model”; `OnChange=cbShinLeftChange` |
| Appearance | `cbFootLeft` | combo | Left Foot | UTC `BodyPart_LFoot` | list; tip: “Select model”; `OnChange=cbFootLeftChange` |
| Appearance | `cbBicepRight` | combo | Right Bicep | UTC `BodyPart_RBicep` | list; tip: “Select model”; `OnChange=cbBicepRightChange` |
| Appearance | `cbForearmRight` | combo | Right Forearm | UTC `BodyPart_RFArm` | list; tip: “Select model”; `OnChange=cbForearmRightChange` |
| Appearance | `cbHandRight` | combo | Right Hand | UTC `BodyPart_RHand` | list; tip: “Select model”; `OnChange=cbHandRightChange` |
| Appearance | `cbBicepLeft` | combo | Left Bicep | UTC `BodyPart_LBicep` | list; tip: “Select model”; `OnChange=cbBicepLeftChange` |
| Appearance | `cbForearmLeft` | combo | Left Forearm | UTC `BodyPart_LFArm` | list; tip: “Select model”; `OnChange=cbForearmLeftChange` |
| Appearance | `cbHandLeft` | combo | Left Hand | UTC `BodyPart_LHand` | list; tip: “Select model”; `OnChange=cbHandLeftChange` |
| Appearance | `cbHead` | combo | Head | UTC `Appearance_Head` | list; tip: “Select model”; `OnChange=cbHeadChange` |
| Appearance | `cbNeck` | combo | Neck | UTC `BodyPart_Neck` | list; tip: “Select model”; `OnChange=cbNeckChange` |
| Appearance | `cbTorso` | combo | Torso | UTC `BodyPart_Torso` | list; tip: “Select model”; `OnChange=cbTorsoChange` |
| Appearance | `cbPelvis` | combo | Pelvis | UTC `BodyPart_Pelvis` | list; tip: “Select model”; `OnChange=cbPelvisChange` |
| Appearance | `cbWings` | combo | Wings | UTC `Wings_New` (wingmodel.2da) | list; tip: “Select model”; `OnChange=cbWingsChange` |
| Appearance | `cbTail` | combo | Tail | UTC `Tail_New` (tailmodel.2da) | list; tip: “Select model”; `OnChange=cbTailChange` |
| Classes › Alignment | `cbAlignment` | combo | Alignment | UTC `GoodEvil` + `LawfulChaotic` (9 presets) | list; tip: “Select an alignment from the dropdown list” |
| Classes › Classes | `cbClass1` | combo | Class 1 | UTC `ClassList[0].Class` (classes.2da) | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass1Change` |
| Classes › Classes | `cbClass2` | combo | Class 2 | `ClassList[1].Class` | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass2Change` |
| Classes › Classes | `cbClass3` | combo | Class 3 | `ClassList[2].Class` | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass3Change` |
| Classes › Classes | `cbClass4` | combo | Class 4 | `ClassList[3].Class` | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass4Change` |
| Classes › Classes | `cbClass5` | combo | Class 5 | `ClassList[4].Class` | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass5Change` |
| Classes › Classes | `cbClass6` | combo | Class 6 | `ClassList[5].Class` | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass6Change` |
| Classes › Classes | `cbClass7` | combo | Class 7 | `ClassList[6].Class` | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass7Change` |
| Classes › Classes | `cbClass8` | combo | Class 8 | `ClassList[7].Class` | list; tip: “Select a class from the dropdown list”; `OnChange=cbClass8Change` |
| Classes › Classes | `eClass1Level` | edit | Class 1 level | `ClassList[0].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `eClass2Level` | edit | Class 2 level | `ClassList[1].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `eClass3Level` | edit | Class 3 level | `ClassList[2].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `eClass4Level` | edit | Class 4 level | `ClassList[3].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `eClass5Level` | edit | Class 5 level | `ClassList[4].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `eClass6Level` | edit | Class 6 level | `ClassList[5].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `eClass7Level` | edit | Class 7 level | `ClassList[6].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `eClass8Level` | edit | Class 8 level | `ClassList[7].ClassLevel` | spin 1…60; tip: “Edit the level for this class”; `OnExit=eLevelExit` |
| Classes › Classes | `cbStartingPackage` | combo | Default Package for Autolevelup | UTC `StartingPackage` (packages.2da) | list; tip: “Specify the package to use for autolevelup via scripting” |
| Classes › Classes | `bCreatureLevelupWizard` | button | Levelup Wizard | opens TfrmCreatureLevelupWizard | `OnClick=bCreatureLevelupWizardClick` |
| Skills | `sgSkills` | grid | ⟨Skills⟩ | UTC `SkillList[].Rank` (skills.2da order) | editable cells; `OnExit=sgSkillsExit OnSetEditText=sgSkillsSetEditText` |
| Scripts › Scripts | `cbOnHeartbeat` | combo | OnHeartBeat | UTC `ScriptHeartbeat` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnPerception` | combo | OnPerception | UTC `ScriptOnNotice` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnSpellCast` | combo | OnSpellCastAt | UTC `ScriptSpellAt` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnMeleeAttacked` | combo | OnPhysicalAttacked | UTC `ScriptAttacked` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnDamaged` | combo | OnDamaged | UTC `ScriptDamaged` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnDisturbed` | combo | OnDisturbed | UTC `ScriptDisturbed` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnEndCombatRound` | combo | OnCombatRoundEnd | UTC `ScriptEndRound` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnDialogue` | combo | OnConversation | UTC `ScriptDialogue` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnSpawnIn` | combo | OnSpawn | UTC `ScriptSpawn` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnRested` | combo | OnRested | UTC `ScriptRested` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnDeath` | combo | OnDeath | UTC `ScriptDeath` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbUserDefine` | combo | OnUserDefined | UTC `ScriptUserDefine` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `cbOnBlocked` | combo | OnBlocked | UTC `ScriptOnBlocked` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Scripts › Scripts | `bLoadScriptSet` | button | Load Script Set | read script set | `OnClick=bLoadScriptSetClick` |
| Scripts › Scripts | `bSaveScriptSet` | button | Save Script Set | write script set | `OnClick=bSaveScriptSetClick` |
| Advanced › Interface | `cbTreasurePile` | combo | Treasure Model | UTC `BodyBag` (bodybag.2da) | list; tip: “Specify the treasure model left behind when this creature dies.” |
| Advanced › Interface | `eDecayTime` | edit | Corpse Decay Time (s) | UTC `DecayTime` (ms in GFF, seconds in UI) | spin 0…32767; tip: “Specify how many seconds the creature's corpse remains after being loo”; `OnExit=eDecayTimeExit` |
| Advanced › Interface | `xbLootable` | checkbox | Leaves Lootable Corpse | UTC `Lootable` | tip: “Specify if this creature leaves a lootable corpse when it dies instead”; `OnClick=xbLootableClick` |
| Advanced › Interface | `cbDisarmable` | checkbox | Disarmable | UTC `Disarmable` | tip: “Specify if this creature can be disarmed in combat” |
| Advanced › Interface | `xbPlot` | checkbox | Plot | UTC `Plot` | tip: “Specify if this creature is plot related” |
| Advanced › Interface | `xbNoPermanentDeath` | checkbox | No Permanent Death | UTC `NoPermDeath` | tip: “Specify if this creature should never explode on death” |
| Advanced › Interface | `xbImmortal` | checkbox | Immortal | UTC `IsImmortal` | tip: “Specify if this creature can be damaged but cannot die” |
| Advanced › Position | `ePositionX` | edit | Position X | (dead) | **hidden (container)**; read-only |
| Advanced › Position | `ePositionZ` | edit | Position Z | (dead) | **hidden (container)**; read-only |
| Advanced › Position | `ePositionY` | edit | Position Y | (dead) | **hidden (container)**; read-only |
| Advanced › Orientation | `eOrientationX` | edit | Orientation X | (dead) | **hidden (container)**; read-only |
| Advanced › Orientation | `eOrientationY` | edit | Orientation Y | (dead) | **hidden (container)**; read-only |
| Advanced › Faction | `cbFaction` | combo | ⟨Faction⟩ | UTC `FactionID` (repute.fac) | list; tip: “Select Faction”; `OnChange=cbFactionChange` |
| Advanced › Faction | `sbFactionEditor` | button | Edit Factions | opens Faction Editor | tip: “Edit faction relationships”; `OnClick=sbFactionEditorClick` |
| Advanced › Challenge Rating | `eCRAdjustment` | edit | Adjustment | UTC `CRAdjust` | spin 0…100; tip: “Edit value to artificially increase the CR”; `OnExit=eCRAdjustmentExit OnKeyDown=eCRAdjustmentKeyDown` |
| Advanced › Challenge Rating | `eCRCalculated` | edit | Calculated | derived CR | read-only; tip: “Displays the calculated CR” |
| Advanced › Challenge Rating | `eChallengeRating` | edit | Challenge Rating | UTC `ChallengeRating` (= calc + adjust) | read-only; tip: “Overall CR value” |
| Advanced › Special | `bChooseTemplates` | button | Apply Template | shows Template tab | tip: “Apply a DD class template”; `OnClick=bChooseTemplatesClick` |
| Advanced › Special | `eSubRace` | edit | Subrace | UTC `Subrace` | tip: “Edit the subrace.” |
| Advanced › Special | `eDeity` | edit | Deity | UTC `Deity` | tip: “Edit the deity.” |
| Advanced › Sound Set | `eSoundSet` | edit | ⟨Sound Set⟩ | UTC `SoundSetFile` (soundset.2da) |  |
| Advanced › Sound Set | `bEditSoundSet` | button | ... | opens TdlgSoundSetSelect | `OnClick=bEditSoundSetClick` |
| Advanced | `bUpdateInstancesInArea` | button | Update Instances (DFM: 'Update instances in current area') | push blueprint to instances | **hidden**; tip: “Update all instances created from this Blueprint”; `OnClick=bUpdateInstancesInAreaClick` |
| Advanced › Perception Range | `cbPerceptionRange` | combo | ⟨Perception Range⟩ | UTC `PerceptionRange` (ranges.2da) | list; tip: “Select perception range” |
| Advanced | `bVariables` | button | Variables... | UTC `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesClick` |
| Advanced | `eResRef` | edit | Blueprint ResRef | UTC `TemplateResRef` | max 16; tip: “Edit the Blueprint ResRef”; `OnChange=eResRefChange OnExit=eResRefExit` |
| Feats | `sgFeatsTable` | grid | ⟨Feats⟩ | UTC `FeatList[].Feat` (feat.2da; granted/available/selected) | editable cells; tip: “Assign innate special abilities” |
| Feats | `cbFeatsFilter` | combo | Filter | feat list filter | list; `OnChange=cbFeatsFilterChange` |
| Feats › Feats Selection Summary | `meFeatsSummary` | memo | ⟨Feats Selection Summary⟩ | derived summary | read-only; tip: “Displays summary of assigned feats” |
| Feats | `bFeatsHelp` | button | ? | help | `OnClick=bFeatsHelpClick` |
| Spells | `sgSpellTable` | grid | ⟨Spells⟩ | (legacy grid) | **hidden**; editable cells; tip: “Edit number of spells prepared”; `OnExit=sgSpellTableExit OnSetEditText=sgSpellTableSetEditText` |
| Spells | `lvSpellTable` | listview | ⟨Spells⟩ | UTC `ClassList[c].KnownList{L}` / `MemorizedList{L}` (`Spell`,`SpellFlags`,`SpellMetaMagic`) | cols: Prepared / Name / Spell Level / Casting Level; tip: “Edit number of spells prepared”; `OnClick=lvSpellTableClick OnColumnClick=lvSpellTableColumnClick OnEdited=lvSpellTableEdited OnExit=lvSpellTableExit OnKeyDown=lvSpellTableKeyDown OnSelectItem=lvSpellTableSelectItem` |
| Spells | `cbSpellLevels` | combo | Spell Level | spell level 0–9 filter | list; tip: “Select level to filter available spells by”; `OnChange=cbSpellLevelsChange` |
| Spells | `cbMetamagic` | combo | Meta-Magic | `SpellMetaMagic` of selected entry | list; tip: “Select meta-magic to filter spells by”; `OnChange=cbMetamagicChange` |
| Spells › Class | `rbClass1` | radio | Class 1 | class selector for spell list | default on; tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells › Class | `rbClass2` | radio | Class 2 | class selector | tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells › Class | `rbClass3` | radio | Class 3 | class selector | tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells › Class | `rbClass4` | radio | Class 4 | class selector | tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells › Class | `rbClass5` | radio | Class 5 | class selector | tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells › Class | `rbClass6` | radio | Class 6 | class selector | tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells › Class | `rbClass7` | radio | Class 7 | class selector | tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells › Class | `rbClass8` | radio | Class 8 | class selector | tip: “Select class to filter spells for”; `OnClick=rbClassClick` |
| Spells | `bSpellsHelp` | button | ? | help | `OnClick=bSpellsHelpClick` |
| Spells › Spell Selection Summary | `meSummary` | memo | ⟨Spell Selection Summary⟩ | derived summary | read-only; tip: “Displays summary of prepared spells” |
| Spells | `bClearSpellList` | button | Clear Class Spell List | clear class spells | `OnClick=bClearSpellListClick` |
| Spells | `bSaveSpellSet` | button | Save Class Spell List | write spell list file | `OnClick=bSaveSpellSetClick` |
| Spells | `bLoadSpellSet` | button | Load Class Spell List | read spell list file | `OnClick=bLoadSpellSetClick` |
| Special Abilities | `sgSpecialAbilitiesTable` | grid | ⟨Special Abilities⟩ | UTC `SpecAbilityList[]` (`Spell`,`SpellCasterLevel`,`SpellFlags`) | editable cells; tip: “Assign innate special abilities”; `OnExit=sgSpecialAbilitiesTableExit OnSetEditText=sgSpecialAbilitiesTableSetEditText` |
| Special Abilities | `cbSpecialAbilitiesFilter` | combo | Filter | filter | editable; `OnChange=cbSpecialAbilitiesFilterChange` |
| Special Abilities › Special Abilities Selection Summary | `meSpecialAbilitiesSummary` | memo | ⟨Special Abilities Selection Summary⟩ | summary | read-only; tip: “Displays summary of assigned feats” |
| Special Abilities | `bSpecialAbilitiesHelp` | button | ? | help | `OnClick=bSpecialAbilitiesHelpClick` |
| Template | `lbTemplates` | listbox | Choose the template to apply to this creature | crtemplates.2da rows | `OnClick=lbTemplatesClick OnDblClick=lbTemplatesDblClick` |
| Template | `bApplySelectedTemplates` | button | Apply Template | apply creature template(s) | `OnClick=bApplySelectedTemplatesClick` |
| Template | `bCancelTemplates` | button | Cancel | leave Template tab | `OnClick=bCancelTemplatesClick` |
| Comments | `mComments` | memo | ⟨Comments⟩ | UTC `Comment` | tip: “Add or edit a comment” |
|  | `apCreature` | 3D view |  | model preview | `OnMouseDown=apCreatureMouseDown` |
|  | `bInventory` | button | Inventory | opens TdlgInventory (`Equip_ItemList`, `ItemList`) | tip: “Edit inventory contents”; `OnClick=bInventoryClick` |
|  | `bRestoreDefaults` | button | Restore |  | dead; **hidden** |
|  | `bOK` | button | OK | save | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `Button1` | button | Button1 |  | debug button; **hidden**; `OnClick=Button1Click` |
|  | `dlgOpenFile` | file dialog |  | (unused .dlg picker) | filter `Conversation Files (*.DLG)/*.dlg/All Files (*.*)/*.*` ext .dlg |

**Notes / behaviors:**
- Right side: live 3D preview (`apCreature`: drag rotate, wheel zoom); bottom buttons: Inventory, OK, Cancel.
- Statistics recompute on every change (`OnAbilityDependentKeyUp`): ability totals include racial adjustments; AC = 10 + dex + size + natural; saves = class tables + ability + bonus; HP total = base + Con bonus × levels.
- Challenge Rating = calculated (classes, HD, abilities, feats/specials, `fractionalcr.2da`) + CR Adjustment; shown on Basic and Advanced tabs.
- Classes: up to 8 (EE) with levels 1–60; `Package` drives Levelup Wizard auto-picks.
- Skills grid: editable ranks, edit mask numeric; Feats grid: owner-drawn check column + filter (all / assigned / available); Spells: per class radio, per level list with Prepared/Name/Level/Caster-level columns, metamagic combo, save/load/clear class spell list; Special Abilities grid (spell, caster level, uses) with filter.
- Template tab (`crtemplates.2da`: Half-Dragon, Lich, Vampire, Half-Fiend…) — applying modifies stats/feats/abilities.
- Hidden/dead: Position/Orientation groups, `Restore`, `Button1`, `dlgOpenFile`, `bUpdateInstancesInArea` (shown in blueprint mode).

#### `TdlgCreatureWizard` — Creature Wizard
*DFM `TDLGCREATUREWIZARD` · 590×431 · border bsDialog · StrRef table .data+0x22DAC (37/43 captions matched) · form events: OnPaint=FormPaint, OnShow=FormShow*

**Purpose:** Creature Wizard: start → racial type → appearance/portrait/gender → class & level → faction → name → palette category → review → finish.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bFinish` | button | Finish |  | **hidden**; disabled; tip: “Finish the Wizard”; `OnClick=bFinishClick` |
|  | `bNext` | button | Next |  | tip: “Continue to the next step in the Wizard”; `OnClick=bNextClick` |
|  | `bBack` | button | Back |  | disabled; tip: “Return to the previous step in the Wizard”; `OnClick=bBackClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `bHelp` | button | Help |  | **hidden**; `OnClick=bHelpClick` |
|  | `tvPaletteSelector` | tree |  | UTC `PaletteID` | `OnChange=tvPaletteSelectorChange OnClick=tvPaletteSelectorClick OnDblClick=tvPaletteSelectorDblClick OnMouseDown=tvPaletteSelectorMouseDown` |
|  | `cbAppearance` | combo | Appearance | UTC `Appearance_Type` | list; `OnChange=cbAppearanceChange` |
|  | `imgPortrait` | image |  | UTC `PortraitId` | `OnClick=imgPortraitClick` |
|  | `bPortraitSelect` | button | Select Portrait | opens TdlgPortrait | `OnClick=imgPortraitClick` |
|  | `cbGender` | combo | Gender | UTC `Gender` | list; `OnChange=cbGenderChange` |
|  | `mSummary` | memo | Check that the following statistics are correct. Click Back to make any changes. | review text |  |
|  | `xbLaunchCreatureEditor` | checkbox | Launch Creature Properties (DFM: 'Lauch Creature Editor') | open Creature Properties after |  |
|  | `eCreatureName` | edit | Name | UTC `FirstName` | `OnChange=eCreatureNameChange` |
|  | `bRandom` | button | Random | random name | `OnClick=bRandomClick` |
|  | `eLastName` | edit | Last Name | UTC `LastName` |  |
|  | `bRandomLastName` | button | Random | random last name | `OnClick=bRandomLastNameClick` |
|  | `lbFactions` | listbox | The Faction determines how this Creature reacts to players and other Creatures | UTC `FactionID` | `OnClick=lbFactionsClick OnDblClick=lbFactionsDblClick` |
|  | `lbRacialType` | listbox | The racial type determines the default abilities, class, and appearance of the creature. | UTC `Race` (+default appearance/abilities) | `OnClick=lbRacialTypeClick OnDblClick=lbRacialTypeDblClick` |
|  | `xbDoNotShowAgain` | checkbox | Do not show me this again | (hidden) ini | **hidden** |
|  | `eClass1Level` | edit | Class 1 level | `ClassList[0].ClassLevel` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass2Level` | edit | Class 2 level | new UTC `ClassList[n]` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass3Level` | edit | Class 3 level | new UTC `ClassList[n]` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass4Level` | edit | Class 4 level | new UTC `ClassList[n]` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass5Level` | edit | Class 5 level | new UTC `ClassList[n]` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass6Level` | edit | Class 6 level | new UTC `ClassList[n]` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass7Level` | edit | Class 7 level | new UTC `ClassList[n]` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass8Level` | edit | Class 8 level | new UTC `ClassList[n]` | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `lbClasses` | listbox | The higher the level in a class, the more skills and feats the creature will have and t… | classes.2da (player + NPC classes) | tip: “Select a Class to add”; `OnClick=lbClassesClick OnDblClick=lbClassesDblClick` |
|  | `eClass1` | edit | Class 1 | `ClassList[0].Class` | read-only |
|  | `eClass2` | edit | Class 2 | new UTC `ClassList[n]` | read-only |
|  | `eClass3` | edit | Class 3 | new UTC `ClassList[n]` | read-only |
|  | `eClass4` | edit | Class 4 | new UTC `ClassList[n]` | read-only |
|  | `eClass5` | edit | Class 5 | new UTC `ClassList[n]` | read-only |
|  | `eClass6` | edit | Class 6 | new UTC `ClassList[n]` | read-only |
|  | `eClass7` | edit | Class 7 | new UTC `ClassList[n]` | read-only |
|  | `eClass8` | edit | Class 8 | new UTC `ClassList[n]` | read-only |
|  | `bClassAdd` | button | Add Class | add selected class (max 8) | disabled; tip: “Add the currently selected class”; `OnClick=bClassAddClick` |
|  | `bClass2Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass2DeleteClick` |
|  | `bClass3Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass3DeleteClick` |
|  | `bClass4Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass4DeleteClick` |
|  | `bClass5Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass5DeleteClick` |
|  | `bClass6Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass6DeleteClick` |
|  | `bClass7Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass7DeleteClick` |
|  | `bClass8Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass8DeleteClick` |
|  | `bClass1Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass1DeleteClick` |
|  | `apAppearance` | 3D view |  | model preview |  |

**Notes / behaviors:**
- Implemented with panels (not tabs) switched by Next/Back; wizard auto-derives tag/resref, default portrait by race/gender, abilities by race+class, and calls the level-up logic to fill feats/skills/spells.

#### `TfrmCreatureLevelupWizard` — Creature Levelup Wizard
*DFM `TFRMCREATURELEVELUPWIZARD` · 427×448 · border bsDialog · StrRef table .data+0x2198C (8/9 captions matched)*

**Purpose:** Levelup Wizard: add classes/levels to an existing creature (auto-picks feats/skills/spells via packages).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `eClass1Level` | edit | Class 1 level | levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass2Level` | edit | Class 2 level | classes/levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass3Level` | edit | Class 3 level | classes/levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass4Level` | edit | Class 4 level | classes/levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass5Level` | edit | Class 5 level | classes/levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass6Level` | edit | Class 6 level | classes/levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass7Level` | edit | Class 7 level | classes/levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `eClass8Level` | edit | Class 8 level | classes/levels to add | disabled; max 2; spin 1…60; tip: “Adjust the Level of this Class”; `OnChange=eClassLevelChange` |
|  | `lbClasses` | listbox | The higher the level in a class, the more skills and feats the creature will have and t… | classes.2da | tip: “Select a Class to add”; `OnClick=lbClassesClick OnDblClick=lbClassesDblClick` |
|  | `eClass1` | edit | Class 1 | class 1 | read-only |
|  | `eClass2` | edit | Class 2 | classes/levels to add | read-only |
|  | `eClass3` | edit | Class 3 | classes/levels to add | read-only |
|  | `eClass4` | edit | Class 4 | classes/levels to add | read-only |
|  | `eClass5` | edit | Class 5 | classes/levels to add | read-only |
|  | `eClass6` | edit | Class 6 | classes/levels to add | read-only |
|  | `eClass7` | edit | Class 7 | classes/levels to add | read-only |
|  | `eClass8` | edit | Class 8 | classes/levels to add | read-only |
|  | `bClassAdd` | button | Add Class | add class | disabled; tip: “Add the currently selected class”; `OnClick=bClassAddClick` |
|  | `bClass2Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass2DeleteClick` |
|  | `bClass3Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass3DeleteClick` |
|  | `bClass4Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass4DeleteClick` |
|  | `bClass5Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass5DeleteClick` |
|  | `bClass6Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass6DeleteClick` |
|  | `bClass7Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass7DeleteClick` |
|  | `bClass8Delete` | button | (no caption; glyph) |  | disabled; tip: “Remove this Class”; `OnClick=bClass8DeleteClick` |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |

**Notes / behaviors:**
- Label says "1 to 3 classes" (legacy) but 8 class slots exist.

#### `TColorPicker` — Character Colors
*DFM `TCOLORPICKER` · 254×323 · border bsDialog · no StrRef table resolved*

**Purpose:** Character/armour colour picker: palette image (PLT colour ramps) with a layer selector.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `Image` | image |  | PLT colour ramp image (click = select index) | `OnMouseDown=ImageMouseDown` |
|  | `TexLayerList` | combo |  | layer: skin/hair/tattoo1/tattoo2 or cloth1/2, leather1/2, metal1/2 | list; `OnChange=TexLayerListChange` |
|  | `m_bOk` | button | Ok |  |  |
|  | `m_bCancel` | button | Cancel |  |  |

#### `TdlgSoundSetSelect` — Sound Set
*DFM `TDLGSOUNDSETSELECT` · 254×338 · border bsDialog · StrRef table .data+0xF5F8C (6/6 captions matched) · form events: OnCreate=FormCreate, OnDestroy=FormDestroy · DFM caption 'Sound Set Select'*

**Purpose:** Sound Set Select (soundset.2da filtered by gender/type; click previews).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Options | `rbMale` | radio | Male | filter soundset.2da `GENDER` | `OnClick=actRefreshExecute` |
| Options | `rbFemale` | radio | Female | filter | `OnClick=actRefreshExecute` |
| Options | `rbMaleFemale` | radio | Both | filter | default on; `OnClick=actRefreshExecute` |
| Options | `cbTypes` | combo | ⟨Options⟩ | filter soundset.2da `TYPE` (soundsettype.2da) | list; items: All / Player / Henchmen / NPC - Primary / NPC - Secondary / Monsters; `OnChange=actRefreshExecute` |
|  | `lbStrRefs` | listbox |  | soundset.2da rows (click = play sample) | `OnMouseDown=lbStrRefsMouseDown` |
|  | `bCancel` | button | Cancel |  |  |
|  | `bOk` | button | Ok |  |  |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actRefresh` | actRefresh |  | `actRefreshExecute` |  |  |

#### `TdlgInventory` — Inventory Contents
*DFM `TDLGINVENTORY` · 634×545 · border bsDialog · StrRef table .data+0xA7324 (26/28 captions matched) · form events: OnShow=FormShow*

**Purpose:** Inventory editor for creatures (equipment slots + backpack), placeables (contents) and stores (categorised stock).

**Tabs** (`pcPalettes`): Standard Items · Custom Items
**Tabs** (`pgInventory`): Standard Equipment · Natural Equipment
**Tabs** (`pcContents`): Contents · Armor · Weapons · Potions/Scrolls · Rings/Amulets · Miscellaneous

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mbNew` | button | New | Item Wizard | tip: “Create a new item Blueprint”; `OnClick=mbNewClick` |
| Standard Items | `tvItems` | tree | ⟨Standard Items⟩ | standard item palette (drag source) | drag source; `OnDblClick=tvItemsDblClick OnMouseDown=tvItemsMouseDown` |
| Custom Items | `tvCustomItems` | tree | ⟨Custom Items⟩ | custom item palette (drag source) | drag source; `OnDblClick=tvItemsDblClick OnMouseDown=tvItemsMouseDown` |
| Options | `cbInfinite` | checkbox | Infinite | UTM `StoreList[].ItemList[].Infinite` | tip: “Specify if store has infinite amounts of selected item”; `OnClick=cbInfiniteClick` |
| Options | `cbPickpocketable` | checkbox | Pickpocketable | UTC `ItemList[].Pickpocketable` | tip: “Specify if selected item can be pickpocketed”; `OnClick=cbPickpocketableClick` |
| Options | `cbDropable` | checkbox | Dropable | UTC `ItemList[].Dropable`/`Equip_ItemList` droppable | tip: “Specify if selected item can be dropped”; `OnClick=cbDropableClick` |
| Options | `meSelectedName` | memo | Selected Item: | selected item name | read-only |
| Standard Equipment | `imPrimary` | image |  | equip slot RIGHTHAND (0x10) | tip: “Primary weapon slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imArmor` | image |  | equip slot CHEST (0x2) | tip: “Armor slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imSecondary` | image |  | equip slot LEFTHAND (0x20) | tip: “Secondary weapon or shield slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imCloak` | image |  | equip slot CLOAK (0x40) | tip: “Cloak slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imHelmet` | image |  | equip slot HEAD (0x1) | tip: “Helmet slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imBoots` | image |  | equip slot BOOTS (0x4) | tip: “Boots slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imGauntlets` | image |  | equip slot ARMS (0x8) | tip: “Gloves or bracers slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imAmulet` | image |  | equip slot NECK (0x200) | tip: “Amulet slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imBelt` | image |  | equip slot BELT (0x400) | tip: “Belt slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imArrows` | image |  | equip slot ARROWS (0x800) | tip: “Arrows slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imBolts` | image |  | equip slot BOLTS (0x2000) | tip: “Bolts slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imBullets` | image |  | equip slot BULLETS (0x1000) | tip: “Bullets slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imRing1` | image |  | equip slot LEFTRING (0x80) | tip: “Ring slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Standard Equipment | `imRing2` | image |  | equip slot RIGHTRING (0x100) | tip: “Ring slot”; `OnDblClick=imEquippedDblClick OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Natural Equipment | `imCWeapon1` | image |  | creature weapon L (0x4000) | tip: “Creature item or regular weapon slot”; `OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Natural Equipment | `imCWeapon2` | image |  | creature weapon R (0x8000) | tip: “Creature item or regular weapon slot”; `OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Natural Equipment | `imCWeapon3` | image |  | creature weapon B / special (0x10000) | tip: “Creature item or regular weapon slot”; `OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Natural Equipment | `imCArmor` | image |  | creature hide (0x20000) | tip: “Creature skin/hide item slot”; `OnDragDrop=imEquippedDragDrop OnMouseDown=imEquippedMouseDown` |
| Contents | `dgBackpack` | draw-grid | ⟨Contents⟩ | `ItemList[]` (`Repos_PosX/Y`) — all | `OnDblClick=dgBackpackDblClick OnDragDrop=dgBackpackDragDrop OnKeyDown=dgBackpackKeyDown OnMouseDown=dgBackpackMouseDown` |
| Armor | `dgArmor` | draw-grid | ⟨Armor⟩ | store cat. armor (StoreList 0) | `OnDragDrop=dgBackpackDragDrop OnKeyDown=dgBackpackKeyDown OnMouseDown=dgBackpackMouseDown` |
| Weapons | `dgWeapons` | draw-grid | ⟨Weapons⟩ | store cat. weapons (StoreList 4) | `OnDragDrop=dgBackpackDragDrop OnKeyDown=dgBackpackKeyDown OnMouseDown=dgBackpackMouseDown` |
| Potions/Scrolls | `dgPotions` | draw-grid | ⟨Potions/Scrolls⟩ | store cat. potions/scrolls (StoreList 2) | `OnDragDrop=dgBackpackDragDrop OnKeyDown=dgBackpackKeyDown OnMouseDown=dgBackpackMouseDown` |
| Rings/Amulets | `dgRings` | draw-grid | ⟨Rings/Amulets⟩ | store cat. rings/amulets (StoreList 3) | `OnDragDrop=dgBackpackDragDrop OnKeyDown=dgBackpackKeyDown OnMouseDown=dgBackpackMouseDown` |
| Miscellaneous | `dgMisc` | draw-grid | ⟨Miscellaneous⟩ | store cat. misc (StoreList 1) | `OnDragDrop=dgBackpackDragDrop OnKeyDown=dgBackpackKeyDown OnMouseDown=dgBackpackMouseDown` |
|  | `imDrop` | image |  | drop target | **hidden**; `OnDragDrop=imDropDragDrop` |
|  | `mbOk` | button | OK |  | tip: “Accept changes”; `OnClick=mbOkClick` |
|  | `mbCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `mbRandom` | button | Random Item Templates | (hidden) random item generators | **hidden**; `OnClick=mbRandomClick` |
|  | `imTrash` | image |  | drag here to delete | tip: “Drag an item here to destroy it”; `OnDragDrop=imTrashDragDrop` |

**Popup menu `pmInventory`** (OnPopup=pmInventoryPopup)

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miEdit` | Edit |  |  `miEditClick` |  |
| `miCopy` | Edit Copy |  |  `miEditClick` |  |

**Notes / behaviors:**
- Mode-dependent: creature (Standard Equipment + Natural Equipment tabs, Dropable/Pickpocketable), placeable (contents only), store (category tabs + Infinite + price preview "Store buys/sells/BlackMarket buys for").
- Drag & drop from palette trees to slots/grid; drag to trash image deletes; double-click equips/edits; popup: Edit / Edit Copy on custom items.
- Equip slot validation against baseitems.2da `EquipableSlots`; warnings for invalid inventory (Options: creature inventory warning).

## 6. Item editor (+ property editor, wizard, generators)

#### `TdlgItemEdit` — Item Properties
*DFM `TDLGITEMEDIT` · 787×455 · border bsDialog · StrRef table .data+0x3C714 (69/73 captions matched) · form events: OnClose=FormClose, OnCreate=FormCreate, OnMouseWheelDown=FormMouseWheelDown, OnMouseWheelUp=FormMouseWheelUp, OnPaint=FormPaint, OnShow=FormShow · DFM caption 'Item Editor'*

**Purpose:** Item Properties (UTI blueprint or instance), with 3D/2D preview.

**Tabs** (`pgAdvanced`): General · Appearance · Properties · Description · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `bDefault` | button | Restore Default | (hidden) | **hidden**; disabled; `OnClick=bDefaultClick` |
|  | `apItem` | 3D view |  | model preview | `OnMouseDown=apItemMouseDown` |
| General | `meCost` | masked edit | Total Cost | UTI `Cost` (derived) | read-only |
| General | `meWeight` | masked edit | Base Weight | baseitems.2da `TenthLBS` + props | read-only |
| General | `meDamage` | masked edit | Damage | baseitems.2da dice | read-only |
| General | `meCritical` | masked edit | Critical | baseitems.2da `CritThreat`/`CritHitMult` | read-only |
| General | `eDamageType` | edit | Damage Type | baseitems.2da `WeaponType` | read-only |
| General | `meArmor` | masked edit | Armor Class | armor AC (armor.2da via `ArmorPart_Torso` parts_chest.2da `ACBONUS`) | read-only |
| General | `eCharges` | edit | Charges | UTI `Charges` (0–250) | spin 0…250; tip: “Edit property values”; `OnExit=eChargesExit OnKeyDown=eMaskEditKeyDown` |
| General | `cbStolen` | checkbox | Stolen | UTI `Stolen` | tip: “Mark as stolen item” |
| General | `ePaletteCategory` | edit | Category | UTI `PaletteID` (itempal.itp) | read-only |
| General | `bPaletteCategory` | button | ... (Category) | opens TdlgPaletteChooser | tip: “Select Blueprint category”; `OnClick=bPaletteCategoryClick` |
| General | `cbPlotItem` | checkbox | Plot Item | UTI `Plot` | tip: “Mark as plot item” |
| General | `eAddCost` | edit | Additional Cost | UTI `AddCost` | spin 0…32767; tip: “Edit property values”; `OnExit=eAddCostExit OnKeyDown=eMaskEditKeyDown` |
| General | `eStackSize` | edit | Stack Size | UTI `StackSize` (1…baseitems `Stacking`) | spin -32767…32767; tip: “Edit property values”; `OnExit=eStackSizeExit OnKeyDown=eMaskEditKeyDown` |
| General | `meLevel` | masked edit | Required Level | itemvalue.2da required level | read-only |
| General | `meLore` | masked edit | Required Lore | skillvsitemcost.2da required lore | read-only |
| General | `eACType` | edit | Armor Type | armor.2da type | read-only |
| General | `eACCheck` | edit | Armor Check Penalty: | armor.2da `ACCHECK` | read-only |
| General | `eArcaneFailure` | edit | Arcane Spell Failure: | armor.2da `ARCANEFAILURE%` | read-only |
| General | `eMaxDexBonus` | edit | Max Dex Bonus: | armor.2da `DEXBONUS` | read-only |
| General | `eName` | edit | Item Name | UTI `LocalizedName` | tip: “Edit the Name in the primary language” |
| General | `bLocName` | button | ... | all languages | tip: “Edit text in multiple languages” |
| General | `eResRef` | edit | Blueprint ResRef | UTI `TemplateResRef` | max 16; tip: “Edit the Blueprint ResRef”; `OnChange=eResRefChange OnExit=eResRefExit` |
| General | `eTag` | edit | Tag | UTI `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
| General | `eBaseName` | edit | Base Type Name | UTI `BaseItem` name (baseitems.2da; fixed after creation) | read-only; max 32 |
| General | `bUniqueTag` | button | ... | generate unique tag | tip: “Assign a unique Tag”; `OnClick=bUniqueTagClick` |
| Appearance | `icItem` | image |  | inventory icon preview |  |
| Appearance | `cbSimple` | combo | Appearance | UTI `ModelPart1` (simple items) | **hidden (container)**; list; tip: “Select model”; `OnChange=cbSimpleChange` |
| Appearance | `cbWeaponMiddle` | combo | Middle | UTI `ModelPart2` model | list; tip: “Select model”; `OnChange=cbWeaponMiddleChange` |
| Appearance | `cbWeaponBottom` | combo | Bottom | UTI `ModelPart1` model | list; tip: “Select model”; `OnChange=cbWeaponBottomChange` |
| Appearance | `cbWeaponTop` | combo | Top | UTI `ModelPart3` model | list; tip: “Select model”; `OnChange=cbWeaponTopChange` |
| Appearance | `cbWeaponTopColor` | combo | Color | UTI `ModelPart3` colour digit | list; tip: “Select color”; `OnChange=cbWeaponTopColorChange` |
| Appearance | `cbWeaponMiddleColor` | combo | Middle | UTI `ModelPart2` colour digit | list; tip: “Select color”; `OnChange=cbWeaponMiddleColorChange` |
| Appearance | `cbWeaponBottomColor` | combo | Bottom | UTI `ModelPart1` colour digit | list; tip: “Select color”; `OnChange=cbWeaponBottomColorChange` |
| Appearance | `lvSimple` | listview | Appearance | UTI `ModelPart1` (icon grid) | tip: “Select appearance”; `OnChange=lvSimpleChange OnClick=lvSimpleClick` |
| Appearance | `cbForearmRight` | combo | Right Forearm | UTI `ArmorPart_RFArm` | list; tip: “Select model”; `OnChange=cbForearmRightChange` |
| Appearance | `cbHandRight` | combo | Right Hand | UTI `ArmorPart_RHand` | list; tip: “Select model”; `OnChange=cbHandRightChange` |
| Appearance | `cbBicepRight` | combo | Right Bicep | UTI `ArmorPart_RBicep` | list; tip: “Select model”; `OnChange=cbBicepRightChange` |
| Appearance | `cbBicepLeft` | combo | Left Bicep | UTI `ArmorPart_LBicep` | list; tip: “Select model”; `OnChange=cbBicepLeftChange` |
| Appearance | `cbForearmLeft` | combo | Left Forearm | UTI `ArmorPart_LFArm` | list; tip: “Select model”; `OnChange=cbForearmLeftChange` |
| Appearance | `cbHandLeft` | combo | Left Hand | UTI `ArmorPart_LHand` | list; tip: “Select model”; `OnChange=cbHandLeftChange` |
| Appearance | `cbNeck` | combo | Neck | UTI `ArmorPart_Neck` | list; tip: “Select model”; `OnChange=cbNeckChange` |
| Appearance | `cbThighs` | combo | Thighs | UTI `ArmorPart_LThigh`+`RThigh` | list; tip: “Select model”; `OnChange=cbThighsChange` |
| Appearance | `cbShins` | combo | Shins | UTI `ArmorPart_LShin`+`RShin` | list; tip: “Select model”; `OnChange=cbShinsChange` |
| Appearance | `cbFeet` | combo | Feet | UTI `ArmorPart_LFoot`+`RFoot` | list; tip: “Select model”; `OnChange=cbFeetChange` |
| Appearance | `cbShoulderLeft` | combo | Left Shoulder | UTI `ArmorPart_LShoul` | list; tip: “Select model”; `OnChange=cbShoulderLeftChange` |
| Appearance | `cbShoulderRight` | combo | Right Shoulder | UTI `ArmorPart_RShoul` | list; tip: “Select model”; `OnChange=cbShoulderRightChange` |
| Appearance | `cbTorso` | combo | Torso | UTI `ArmorPart_Torso` (parts_chest.2da → AC) | list; tip: “Select model”; `OnChange=cbTorsoChange` |
| Appearance | `cbBelt` | combo | Belt | UTI `ArmorPart_Belt` | list; tip: “Select model”; `OnChange=cbBeltChange` |
| Appearance | `cbPelvis` | combo | Pelvis | UTI `ArmorPart_Pelvis` | list; tip: “Select model”; `OnChange=cbPelvisChange` |
| Appearance | `meArmor2` | masked edit | Armor Class | armour AC (derived) | read-only |
| Appearance | `cbRobe` | combo | Robe | UTI `ArmorPart_Robe` (parts_robe.2da) | list; tip: “Select model”; `OnChange=cbRobeChange` |
| Appearance › View | `rb2D` | radio | 2D Icon | preview mode | **hidden (container)**; default on; `OnClick=rbViewClick` |
| Appearance › View | `rb3D` | radio | 3D Model | preview mode | **hidden (container)**; `OnClick=rbViewClick` |
| Appearance › Appearance | `rbColor` | radio | Color | (hidden) | **hidden (container)**; `OnClick=rbModeClick` |
| Appearance › Appearance | `rbModel` | radio | Model | (hidden) | **hidden (container)**; default on; `OnClick=rbModeClick` |
| Appearance › Appearance | `rbFemale` | radio | Female | preview body gender | **hidden (container)**; tip: “Select gender”; `OnClick=rbGenderClick` |
| Appearance › Appearance | `rbMale` | radio | Male | preview body gender | **hidden (container)**; tip: “Select gender”; `OnClick=rbGenderClick` |
| Appearance | `bColorChooser` | button | Color ... | TColorPicker → `Cloth1Color`,`Cloth2Color`,`Leather1Color`,`Leather2Color`,`Metal1Color`,`Metal2Color` (also helmets/cloaks) | tip: “Select color”; `OnClick=bColorChooserClick` |
| Properties | `tvMaster` | tree | Available Properties | available properties (itemprops.2da column for base item; itempropdef.2da names) | `OnClick=tvMasterClick OnDblClick=bAddClick OnMouseDown=tvPropertiesMouseDown` |
| Properties | `bAdd` | button | -> | add property (→ TdlgPropEdit if params) | tip: “Assign selected property”; `OnClick=bAddClick` |
| Properties | `bRemove` | button | <- | remove property | tip: “Remove assigned property”; `OnClick=bRemoveClick` |
| Properties | `tvAssigned` | tree | Assigned Properties | UTI `PropertiesList[]` (`PropertyName`,`Subtype`,`CostTable`,`CostValue`,`Param1`,`Param1Value`,`ChanceAppear`) | `OnClick=tvAssignedClick OnDblClick=tvAssignedDblClick OnMouseDown=tvPropertiesMouseDown` |
| Properties | `meCost2` | masked edit | Total Cost | UTI `Cost` | read-only |
| Properties | `bEdit` | button | Edit Property | opens TdlgPropEdit | tip: “Edit assigned property”; `OnClick=bEditClick` |
| Properties | `eMin` | edit | ⟨Properties⟩ | property-count info | read-only |
| Properties | `eMax` | edit | Max # of Castspell | cast-spell use limit info | read-only |
| Properties | `cbIdentified` | checkbox | Identified | UTI `Identified` |  |
| Properties | `xbCursed` | checkbox | Undroppable | UTI `Cursed` (EE label "Undroppable") | tip: “Specify if this item cannot be removed from the owner's inventory.” |
| Description | `meDescUnidentified` | memo | Unidentified Description | UTI `Description` |  |
| Description | `meDescIdentified` | memo | Identified Description | UTI `DescIdentified` |  |
| Description | `bDescUnidentified` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Description | `bDescIdentified` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Description | `meDescType` | memo | Item Type Description | baseitems.2da `Description` (read-only) | read-only |
| Description | `meDescStats` | memo | Item Statistics Description | generated property text | read-only |
| Description | `bVariables` | button | Variables... | UTI `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesClick` |
| Comments | `mComments` | memo | ⟨Comments⟩ | UTI `Comment` |  |
|  | `bInventory` | button | Inventory ... | (hidden) container contents | **hidden**; disabled; tip: “Edit inventory contents”; `OnClick=bInventoryClick` |
|  | `bUpdateInstancesInArea` | button | Update Instances (DFM: 'Update instances in current area') | push blueprint to instances | **hidden**; tip: “Update all instances created from this Blueprint”; `OnClick=bUpdateInstancesInAreaClick` |

**Notes / behaviors:**
- Appearance tab switches panel by base item `ModelType` (0 simple → `lvSimple` icon grid; 1 layered/2 composite weapon → top/middle/bottom model+colour; 3 armour → 18 part combos + robe). Colour button for PLT-coloured items.
- Properties tab: available tree filtered by base item (itemprops.2da column), assigned tree; double-click adds/edits; cost recalculated (`meCost2`).
- Description tab: unidentified/identified descriptions + read-only base-type description and generated statistics text.
- Stack size spin validated against baseitems.2da `Stacking`; charges 0–250.

#### `TdlgPropEdit` — Select Property Parameters
*DFM `TDLGPROPEDIT` · 517×319 · border bsDialog · StrRef table .data+0x444F4 (12/12 captions matched)*

**Purpose:** Item property parameter editor ("Select Property Parameters").

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `eProperty` | edit | Item Property | `PropertyName` (itempropdef.2da) | read-only; tip: “Name of the item property” |
|  | `eSubProperty` | edit | Item Sub-Property | `Subtype` (itempropdef `SubTypeResRef` table) | read-only; tip: “Name of item sub-property” |
|  | `eCostParam` | edit | Item Cost Parameter | `CostTable`/`CostValue` | read-only; tip: “Selected cost parameter value” |
|  | `eParam1` | edit | Parameter 1 | `Param1`/`Param1Value` | read-only; tip: “Selected parameter 1 value” |
|  | `bCostSelect` | button | Select | pick from tvCostParam | tip: “Select this value”; `OnClick=bCostSelectClick` |
|  | `bParam1Select` | button | Select | pick from tvParam1 | tip: “Select this value”; `OnClick=bParam1SelectClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `tvCostParam` | tree | Cost Parameter Values | iprp_costtable.2da → cost table rows | tip: “Select cost parameter value”; `OnDblClick=tvCostParamDblClick` |
|  | `tvParam1` | tree | Parameter 1 Values | iprp_paramtable.2da → param table rows | tip: “Select parameter 1 value”; `OnDblClick=tvParam1DblClick` |
|  | `eChance` | edit | Chance of Appearing (%) | `ChanceAppear` (0–100) | spin 0…100; tip: “Edit % chance this property is used” |

#### `TdlgItemWizard` — Item Wizard
*DFM `TDLGITEMWIZARD` · StrRef table .data+0x45B9C (11/15 captions matched)*

**Purpose:** Item Wizard: base item type → name/quality → palette category → finish.

**Inherits:** `TdlgWizard`.

**Tabs** (`pcSteps`): Item Wizard (hidden tab) · Item Wizard (hidden tab) · Item Wizard (hidden tab) · Item Wizard (hidden tab)

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  | `OnClick=None` |
| Item Wizard | `pSubChooser` | color swatch/panel | Please choose the type of item you wish to create | base item type chooser (TdlgChooser tree; itmwiz*.2da) |  |
| Item Wizard | `eName` | edit | Name: | UTI `LocalizedName` + resref | tip: “Edit item name”; `OnChange=eNameChange` |
| Item Wizard | `rgItemLevel` | radio group | Item Level | item level band (disabled) | disabled; items: 1-5 / 6-10 / 11-15 / 16-20; tip: “Select the player level you wish to create this item for”; `OnClick=rgItemLevelClick` |
| Item Wizard | `rgItemQuality` | radio group | Item Quality | quality (disabled) | disabled; items: Low / Medium / HIgh / Godly; tip: “Select how powerful the item will be”; `OnClick=rgItemQualityClick` |
| Item Wizard | `xbMagical` | checkbox | Magical | magical flag (adds props) | tip: “Make this a magical item”; `OnClick=xbMagicalClick` |
| Item Wizard | `cbLaunch` | checkbox | Launch Item Properties (DFM: 'Launch Item Editor') | open Item Properties after | tip: “Launch the Item Properties for additional modifications” |
| Item Wizard | `tvPaletteSelector` | tree | ⟨Item Wizard⟩ | UTI `PaletteID` | tip: “Select a category”; `OnChange=tvPaletteSelectorChange OnClick=tvPaletteSelectorClick OnDblClick=tvPaletteSelectorDblClick OnMouseDown=tvPaletteSelectorMouseDown` |

#### `TdlgItemGeneratorEdit` — Item Generator Editor
*DFM `TDLGITEMGENERATOREDIT` · 634×388 · border bsDialog · no Tags — never localised (dead feature) · form events: OnShow=FormShow*

**Purpose:** Random item generator editor (unlocalised, unreachable in retail UI).

**Tabs** (`pcProperties`): Basic · Advanced · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `imTrash` | image |  | drag to delete | `OnDragDrop=imTrashDragDrop` |
|  | `Button2` | button | Cancel |  |  |
|  | `bOk` | button | Ok |  | `OnClick=bOkClick` |
| Basic | `eName` | edit | Name | generator name |  |
| Basic | `bName` | button | ... | all languages |  |
| Basic | `eTag` | edit | Tag | tag |  |
| Basic | `dgContents` | draw-grid | ⟨Basic⟩ | item templates in generator (drag from palette) | `OnDragDrop=dgContentsDragDrop OnMouseDown=dgContentsMouseDown` |
| Advanced | `cbScaleByLevel` | checkbox | Scale by Level | scale by level |  |
| Advanced | `cbTreasureValue` | combo | Treasure Value | treasure value (treasurescale.2da?) | editable |
| Advanced | `eResRef` | edit | Template ResRef | template resref |  |
| Comments | `mComments` | memo | ⟨Comments⟩ | comment |  |
|  | `tvItems` | tree |  | item palette (drag source) | drag source; `OnMouseDown=tvItemsMouseDown` |
|  | `mbNew` | button | New | new item | `OnClick=mbNewClick` |

**Popup menu `pmPalette`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miEdit` | Edit |  |  `miEditClick` |  |
| `miCopy` | Edit Copy |  |  |  |
| `miDelete` | Delete |  |  |  |
| — | ——— | | | |
| `miExport` | Export |  |  |  |
| `miImport` | Import |  |  |  |

#### `TdlgGeneratorChooser` — Random Item Generator Chooser
*DFM `TDLGGENERATORCHOOSER` · 446×301 · border bsDialog · no Tags — never localised (dead feature)*

**Purpose:** Random item generator chooser (unlocalised, unreachable; `mbRandom` button hidden in inventory).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bAssign` | button | -> | assign | `OnClick=bAssignClick` |
|  | `bRemove` | button | <- | remove | `OnClick=bRemoveClick` |
|  | `bRemoveAll` | button | << | remove all | `OnClick=bRemoveAllClick` |
| Assigned Templates | `sgAssigned` | grid | ⟨Assigned Templates⟩ | assigned generator templates |  |
| Item Generator Palette | `tvGeneratorPalette` | tree | ⟨Item Generator Palette⟩ | generator palette |  |
|  | `bOk` | button | Ok |  | `OnClick=bOkClick` |
|  | `bCancel` | button | Cancel |  |  |

## 7. Placeable editor

#### `TdlgPlaceableEdit` — Placeable Object Properties
*DFM `TDLGPLACEABLEEDIT` · StrRef table .data+0xCADF8 (4/7 captions matched) · DFM caption 'Edit Placeables'*

**Purpose:** Placeable Object Properties (UTP / GIT placeable).

**Inherits:** `TdlgSituatedEdit` — all tabs/frames of the base dialog (Basic/Lock/Trap/Events/Advanced/Description/Comments) plus the extras below.

**Tabs** (`pcProperties`): Basic · Lock · Trap · Scripts · Advanced · Description · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `apAppearance` | 3D view |  | model preview |  |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bDefaults` | button | Defaults |  | `OnClick=bDefaultsClick` |
| Basic | `fraSituatedBasic1` | embedded frame | `TfraSituatedBasic` | UTP basic fields | see frame section; overrides: cbAppearanceType |
| Basic | `xbUseable` | checkbox | Useable | UTP `Useable` | `OnClick=xbUseableClick` |
| Basic | `xbInventory` | checkbox | Has Inventory (DFM: 'Inventory') | UTP `HasInventory` | `OnClick=xbInventoryClick` |
| Basic | `ePaletteCategory` | edit | Category | UTP `PaletteID` | read-only |
| Basic | `bPaletteCategory` | button | ... | opens TdlgPaletteChooser | `OnClick=bPaletteCategoryClick` |
| Basic | `xbStatic` | checkbox | Static | UTP `Static` | `OnClick=xbStaticClick` |
| Lock | `fraSituatedLock1` | embedded frame | `TfraSituatedLock` |  | see frame section |
| Trap | `xbTrapFlag` | checkbox | Is Trapped | UTP `TrapFlag` |  |
| Trap › Trap Settings | `pTrap` | color swatch/panel | ⟨Trap Settings⟩ | hosts `TfrmTrap` (runtime) |  |
| Scripts | `fraSituatedScripts1` | embedded frame | `TfraSituatedScripts` |  | see frame section |
| Advanced | `fraSituatedAdvanced1` | embedded frame | `TfraSituatedAdvanced` | UTP advanced fields | see frame section; overrides: bUpdateInstancesInArea, eTemplate |
| Advanced | `cbTreasurePile` | combo | Treasure Model | UTP `BodyBag` (bodybag.2da) | editable |
| Description | `fraSituatedDesc1` | embedded frame | `TfraSituatedDesc` |  | see frame section |
| Comments | `fraSituatedComments1` | embedded frame | `TfraSituatedComments` |  | see frame section |
|  | `bInventory` | button | Inventory | opens TdlgInventory (`ItemList`) | tip: “Edit inventory contents”; `OnClick=bInventoryClick` |

**Notes / behaviors:**
- Category is a captioned panel + read-only edit; `xbInventory` enables the Inventory button; `xbStatic` disables most scripts/interaction; Treasure Model only when HasInventory.

#### `TdlgPlaceableWizard` — Placeable Wizard
*DFM `TDLGPLACEABLEWIZARD` · 326×331 · no StrRef table resolved*

**Purpose:** Placeable Wizard.

**Inherits:** `TdlgBlueprintWizard`.

**Tabs** (`pcSteps`): tsPalette · tsName

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| tsPalette | `tvPaletteSelector` | tree |  | `PaletteID` |  |
| tsName | `xbLaunchPropertiesDialog` | checkbox | Launch Properties Dialog | open properties after |  |

## 8. Door editor

#### `TdlgDoorEdit` — Door Properties
*DFM `TDLGDOOREDIT` · 608×405 · StrRef table .data+0x2A5C0 (10/12 captions matched) · DFM caption 'dlgDoorEdit'*

**Purpose:** Door Properties (UTD / GIT door).

**Inherits:** `TdlgSituatedEdit` — base tabs/frames plus Area Transition tab and generic-door appearance.

**Tabs** (`pcProperties`): Basic · Lock · Trap · Area Transition · Scripts · Advanced · Description · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `apAppearance` | 3D view |  | model preview |  |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bDefaults` | button | Defaults |  | `OnClick=bDefaultsClick` |
| Basic | `fraSituatedBasic1` | embedded frame | `TfraSituatedBasic` | UTD basic fields (`cbAppearanceType` = doortypes.2da tileset door) | see frame section; overrides: cbAppearanceType |
| Basic | `cbGenericType` | combo | Generic Appearance | UTD `GenericType_New` (genericdoors.2da) | list; `OnChange=cbGenericTypeChange` |
| Basic | `ePaletteCategory` | edit | Category | UTD `PaletteID` | read-only |
| Basic | `bPaletteCategory` | button | ... | opens TdlgPaletteChooser | `OnClick=bPaletteCategoryClick` |
| Lock | `fraSituatedLock1` | embedded frame | `TfraSituatedLock` |  | see frame section |
| Trap | `xbTrapFlag` | checkbox | Is Trapped | UTD `TrapFlag` |  |
| Trap › Trap Settings | `pTrap` | color swatch/panel | ⟨Trap Settings⟩ | hosts `TfrmTrap` (runtime) |  |
| Area Transition | `eLinkedTo` | edit | Destination Tag | UTD `LinkedTo` | max 32; `OnChange=eLinkedToChange` |
| Area Transition | `xbPartyRequired` | checkbox | (no caption; runtime) | (dead) | **hidden**; disabled |
| Area Transition | `rbLinkedToWaypoint` | radio | Waypoint | UTD `LinkedToFlags`=2 | `OnClick=rbLinkedToSomethingClick` |
| Area Transition | `rbLinkedToDoor` | radio | Door | UTD `LinkedToFlags`=1 | `OnClick=rbLinkedToSomethingClick` |
| Area Transition | `rbLinkedToNothing` | radio | None | UTD `LinkedToFlags`=0 | `OnClick=rbLinkedToNothingClick` |
| Area Transition | `bSetupAreaTransition` | button | Setup Area Transition | opens TdlgAreaTransition | `OnClick=bSetupAreaTransitionClick` |
| Area Transition | `eLoadScreen` | edit | Loading Screen | UTD `LoadScreenID` | read-only |
| Area Transition | `bBrowseAreaTransitionBitmap` | button | ... | opens TdlgLoadScreen | `OnClick=bBrowseAreaTransitionBitmapClick` |
| Scripts | `fraSituatedScripts1` | embedded frame | `TfraSituatedScripts` |  | see frame section |
| Advanced | `fraSituatedAdvanced1` | embedded frame | `TfraSituatedAdvanced` |  | see frame section; overrides: bUpdateInstancesInArea, eTemplate |
| Advanced | `eSecretDoorDC` | edit | Secret Door Detection DC | (dead) | **hidden**; spin 0…100; `OnExit=eDCExit` |
| Description | `fraSituatedDesc1` | embedded frame | `TfraSituatedDesc` |  | see frame section |
| Comments | `fraSituatedComments1` | embedded frame | `TfraSituatedComments` |  | see frame section |

**Notes / behaviors:**
- Appearance = tileset door type (doortypes.2da) *or* Generic Appearance (genericdoors.2da, `GenericType_New`); Area Transition tab like triggers.

#### `TdlgDoorWizard` — Door Wizard
*DFM `TDLGDOORWIZARD` · 326×None · StrRef table .data+0x2CE04 (1/1 captions matched)*

**Purpose:** Door Wizard: palette category → name → (appearance list, strength — pages defined but not localised).

**Inherits:** `TdlgBlueprintWizard` (palette category → name).

**Tabs** (`pcSteps`): tsPalette · tsName · tsAppearance · tsStrength

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| tsName | `xbLaunchPropertiesDialog` | checkbox | Launch Properties Dialog | open properties after |  |
| tsAppearance | `lbGenericAppearances` | listbox |  | UTD `GenericType_New` |  |

## 9. Trigger editor

#### `TdlgTriggerEdit` — Trigger Properties
*DFM `TDLGTRIGGEREDIT` · 357×394 · border bsDialog · StrRef table .data+0x101370 (32/43 captions matched) · form events: OnShow=FormShow*

**Purpose:** Trigger Properties (UTT / GIT trigger). Uses a split name/value property-sheet layout.

**Tabs** (`pcTrigger`): Basic · Area Transition · Trap · Scripts · Advanced · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOk` | button | OK |  | tip: “Accept changes”; `OnClick=bOkClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `bDefault` | button | Defaults (DFM: 'Restore Defaults') | (hidden) | **hidden**; `OnClick=bDefaultClick` |
| Basic | `eDisplayName` | edit | Name | UTT `LocalizedName` | tip: “Edit the Name in the primary language” |
| Basic | `eTag` | edit | Tag | UTT `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
| Basic | `cbTriggerType` | combo | Trigger Type | UTT `Type` (0 generic, 1 area transition, 2 trap) | list; items: Area Transition / Generic / Sound / Waypoint; `OnChange=cbTriggerTypeChange` |
| Basic | `bDisplayName` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Basic | `ePaletteCategory` | edit | Category | UTT `PaletteID` | read-only |
| Basic | `bPaletteCategory` | button | ... | opens TdlgPaletteChooser | `OnClick=bPaletteCategoryClick` |
| Area Transition | `eLinkedTo` | edit | Destination Tag | UTT `LinkedTo` | max 32; `OnChange=eTagChange` |
| Area Transition | `rbLinkedToWaypoint` | radio | Waypoint | `LinkedToFlags`=2 | `OnClick=rbLinkedToSomethingClick` |
| Area Transition | `rbLinkedToDoor` | radio | Door | `LinkedToFlags`=1 | `OnClick=rbLinkedToSomethingClick` |
| Area Transition | `rbLinkedToNothing` | radio | None | `LinkedToFlags`=0 | `OnClick=rbLinkedToNothingClick` |
| Area Transition | `xbPartyRequired` | checkbox | (no caption; runtime) | (dead) | **hidden** |
| Area Transition | `bSetupAreaTransition` | button | Setup Area Transition | opens TdlgAreaTransition | `OnClick=bSetupAreaTransitionClick` |
| Area Transition | `bBrowseAreaTransitionBitmap` | button | ... | opens TdlgLoadScreen | `OnClick=bBrowseAreaTransitionBitmapClick` |
| Area Transition | `eLoadScreen` | edit | Loading Screen | UTT `LoadScreenID` | read-only |
| Scripts | `cbOnEnter` | combo | OnEnter | UTT `ScriptOnEnter` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbAutoComplete` |
| Scripts | `cbOnExit` | combo | OnExit | UTT `ScriptOnExit` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbAutoComplete` |
| Scripts | `cbOnHeartbeat` | combo | OnHeartbeat | UTT `ScriptHeartbeat` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbAutoComplete` |
| Scripts | `cbOnUserDefine` | combo | OnUserDefined | UTT `ScriptUserDefine` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbAutoComplete` |
| Scripts | `cbOnClick` | combo | OnClick | UTT `OnClick` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnClickChange OnClick=cbAutoComplete OnKeyDown=cbOnClickKeyDown` |
| Scripts | `bLoadScriptSet` | button | Load Script Set | read script set | `OnClick=bLoadScriptSetClick` |
| Scripts | `bSaveScriptSet` | button | Save Script Set | write script set | `OnClick=bSaveScriptSetClick` |
| Advanced | `eTemplateName` | edit | Blueprint ResRef | UTT `TemplateResRef` | read-only; max 16; tip: “Edit the ResRef”; `OnChange=eTemplateNameChange` |
| Advanced | `cbFaction` | combo | Faction | UTT `Faction` | list; items: false / true |
| Advanced | `eKeyName` | edit | Key Tag | UTT `KeyName` (transition key) | max 32; `OnChange=eTagChange` |
| Advanced | `xbAutoRemoveKey` | checkbox | Auto Remove Key | UTT `AutoRemoveKey` |  |
| Advanced | `cbCursor` | combo | Cursor | UTT `Cursor` (cursors.2da, icons drawn) | list; items: false / true; `OnDrawItem=cbCursorDrawItem` |
| Advanced | `imgPortrait` | image |  | UTT `PortraitId` preview | `OnClick=bPortraitSelectClick` |
| Advanced | `bPortraitSelect` | button | Select Portrait | opens TdlgPortrait | `OnClick=bPortraitSelectClick` |
| Advanced | `ePortrait` | edit | Portrait | UTT `PortraitId` | read-only |
| Advanced | `eHighlightHeight` | edit | Highlight Height | UTT `HighlightHeight` | spin 0…100; `OnExit=eHighlightHeightExit OnKeyDown=eHighlightHeightKeyDown` |
| Advanced | `bUpdateInstancesInArea` | button | Update Instances (DFM: 'Update instances in current area') | push blueprint to instances | **hidden**; tip: “Update all instances created from this Blueprint”; `OnClick=bUpdateInstancesInAreaClick` |
| Advanced | `bVariablesEdit` | button | ... | UTT `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesEditClick` |
| Comments | `mComments` | memo | ⟨Comments⟩ | UTT `Comment` |  |
|  | `sbMain` | status bar |  | status bar |  |

**Notes / behaviors:**
- Property-sheet layout with draggable splitters (`splitProperties`, `splitEvent`); Area Transition tab visible for type Area Transition; Trap tab hosts `TfrmTrap` for type Trap. `cbTriggerType` items in DFM are placeholders; runtime list = Generic / Area Transition / Trap. `cbFaction`/`cbCursor` placeholder items false/true replaced at runtime.

#### `TdlgTriggerWizard` — Trigger Wizard
*DFM `TDLGTRIGGERWIZARD` · 415×342 · StrRef table .data+0x1056D4 (3/3 captions matched)*

**Purpose:** Trigger Wizard: palette category → trigger type → name.

**Inherits:** `TdlgBlueprintWizard`.

**Tabs** (`pcSteps`): tsPalette · tsType · tsName

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| tsPalette | `tvPaletteSelector` | tree |  | UTT `PaletteID` |  |
| tsType | `lbTriggerType` | listbox | Choose the trigger type | UTT `Type` | `OnClick=lbTriggerTypeClick OnDblClick=lbTriggerTypeDblClick` |
| tsName | `eName` | edit |  | UTT `LocalizedName` |  |
| tsName | `xbLaunchPropertiesDialog` | checkbox | Launch Properties Dialog | open properties after |  |

## 10. Encounter editor

#### `TdlgEncounterEdit` — Encounter Properties
*DFM `TDLGENCOUNTEREDIT` · 609×371 · border bsDialog · StrRef table .data+0x2E0FC (32/38 captions matched) · form events: OnShow=FormShow · DFM caption 'dlgEncounterEdit'*

**Purpose:** Encounter Properties (UTE / GIT encounter).

**Tabs** (`pcEncounter`): Basic · Creature List · Scripts · Advanced · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
| Basic | `eDifficulty` | edit | Difficulty | (hidden) UTE `Difficulty` | **hidden**; max 8; spin 0…100; `OnExit=eDifficultyExit` |
| Basic | `eMaxCreatures` | edit | Maximum Creatures | UTE `MaxCreatures` | max 8; spin 0…100; `OnExit=eMaxCreaturesExit` |
| Basic | `eRecCreatures` | edit | Minimum Creatures | UTE `RecCreatures` | max 8; spin 0…100; `OnExit=eRecCreaturesExit` |
| Basic | `eTag` | edit | Tag | UTE `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
| Basic | `cbSpawnOption` | combo | Spawn Option | UTE `SpawnOption` (0 continuous, 1 single shot) | list; `OnChange=cbSpawnOptionChange` |
| Basic | `eLocName` | edit | Name | UTE `LocalizedName` |  |
| Basic | `bLocName` | button | ... (Name) | all languages | tip: “Edit text in multiple languages” |
| Basic | `cbDifficulty` | combo | Difficulty | UTE `DifficultyIndex` (encdifficulty.2da; also sets `Difficulty`) | list; `OnChange=cbDifficultyChange` |
| Basic | `ePaletteCategory` | edit | Category | UTE `PaletteID` | read-only |
| Basic | `bPaletteCategory` | button | ... (Category) | opens TdlgPaletteChooser | `OnClick=bPaletteCategoryClick` |
| Creature List | `bRecalculateCRs` | button | Recalculate Challenge Ratings | (hidden) refresh `CreatureList[].CR` | **hidden**; `OnClick=bRecalculateCRsClick` |
| Scripts | `cbOnEnter` | combo | OnEnter | UTE `OnEntered` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbEventChange` |
| Scripts | `cbOnExhausted` | combo | OnExhausted | UTE `OnExhausted` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbEventChange` |
| Scripts | `cbOnUserDefined` | combo | OnUserDefined | UTE `OnUserDefined` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbEventChange` |
| Scripts | `cbOnExit` | combo | OnExit | UTE `OnExit` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbEventChange` |
| Scripts | `cbOnHeartbeat` | combo | OnHeartbeat | UTE `OnHeartbeat` | buttons […] [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbEventChange` |
| Scripts | `bLoadScriptSet` | button | Load Script Set | read script set | `OnClick=bLoadScriptSetClick` |
| Scripts | `bSaveScriptSet` | button | Save Script Set | write script set | `OnClick=bSaveScriptSetClick` |
| Advanced | `eResetTime` | edit | Respawn Time (seconds) | UTE `ResetTime` (s) | max 8; spin 0…100; `OnExit=eResetTimeExit` |
| Advanced | `xbReset` | checkbox | Encounter Respawns | UTE `Reset` | `OnClick=xbResetClick` |
| Advanced | `cbFaction` | combo | Faction | UTE `Faction` | list |
| Advanced | `eTemplate` | edit | Blueprint ResRef | UTE `TemplateResRef` | read-only; max 16; tip: “Edit the Blueprint ResRef”; `OnChange=eTemplateChange` |
| Advanced | `xbActive` | checkbox | Active | UTE `Active` |  |
| Advanced | `eRespawns` | edit | Number of times to respawn | UTE `Respawns` | max 8; spin 1…100; `OnExit=eRespawnsExit` |
| Advanced | `xbPlayerTriggeredOnly` | checkbox | Player Triggered Only | UTE `PlayerOnly` |  |
| Advanced | `bUpdateInstancesInArea` | button | Update Instances (DFM: 'Update instances in current area') | push blueprint to instances | **hidden**; tip: “Update Instances”; `OnClick=bUpdateInstancesInAreaClick` |
| Advanced | `xbRespawnInfinite` | checkbox | Infinite Respawn | UTE `Respawns`=-1 | `OnClick=xbRespawnInfiniteClick` |
| Advanced | `bVariables` | button | Variables... | UTE `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesClick` |
| Comments | `mComments` | memo | ⟨Comments⟩ | UTE `Comment` |  |
|  | `bDefaults` | button | Defaults | (hidden) | **hidden**; `OnClick=bDefaultsClick` |

**Notes / behaviors:**
- Creature List tab hosts `TfraEncounterCreatureList` in `pCreatureListFrame`. Encounter geometry & spawn points are edited in the area viewer (Redraw Polygon / Add Spawn Point).

#### `TfraEncounterCreatureList` — (no caption)
*DFM `TFRAENCOUNTERCREATURELIST` · StrRef table .data+0x2D3CC (3/6 captions matched)*

**Purpose:** Encounter creature list frame (palette trees ↔ CreatureList grid).

**Tabs** (`pcCreaturePalettes`): Standard Palette · Custom Palette

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bAddCreature` | button | -> | append to CreatureList | `OnClick=actAddCreatureExecute` |
|  | `bRemoveCreature` | button | <- | remove row | `OnClick=bRemoveCreatureClick` |
| Standard Palette | `tvStandardCreatures` | tree | ⟨Standard Palette⟩ | standard creature palette | `OnDblClick=actAddCreatureExecute OnMouseDown=tvAvailableCreaturesMouseDown` |
| Custom Palette | `tvCustomCreatures` | tree | ⟨Custom Palette⟩ | custom creature palette | `OnDblClick=actAddCreatureExecute OnMouseDown=tvAvailableCreaturesMouseDown` |
|  | `sgCreatureList` | grid |  | UTE `CreatureList[]` (`ResRef`,`CR`,`Appearance`,`SingleSpawn`="Unique") | `OnDblClick=bRemoveCreatureClick OnKeyDown=sgCreatureListKeyDown` |

**Action list `alEncounters`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actAddCreature` | <- (DFM 'Add Creature') |  | `actAddCreatureExecute` |  |  |
| `actEditCreature` | Add Creature (DFM 'Edit Creature') |  | `actEditCreatureExecute` |  |  |
| `actEditCopy` | Edit Copy |  | `actEditCreatureExecute` |  |  |

**Popup menu `pmAvailableCreatures`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miAddCreature` | Add Creature |  | `actAddCreature` |  |
| `miEditCreature` | Edit Creature |  | `actEditCreature` |  |
| `miEditCopy` | Edit Copy |  | `actEditCopy` |  |

**Notes / behaviors:**
- Grid columns set at runtime (creature name, CR, Unique). Double-click palette = add; double-click grid = remove; popup on palette: Add / Edit Creature / Edit Copy.

#### `TdlgEncounterWizard` — Encounter Wizard
*DFM `TDLGENCOUNTERWIZARD` · 605×376 · StrRef table .data+0x31188 (3/3 captions matched)*

**Purpose:** Encounter Wizard: palette category → creature list → name.

**Inherits:** `TdlgBlueprintWizard`.

**Tabs** (`pcSteps`): tsPalette · tsCreatureList · tsName

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| tsPalette | `tvPaletteSelector` | tree |  | `PaletteID` / wizard option |  |
| tsCreatureList | `pCreatureListFrame` | color swatch/panel | Select the creatures that this Encounter can spawn. | hosts TfraEncounterCreatureList |  |
| tsName | `xbLaunchPropertiesDialog` | checkbox | Launch Properties Dialog | `PaletteID` / wizard option |  |

## 11. Sound editor

#### `TdlgSoundEdit` — Sound Properties
*DFM `TDLGSOUNDEDIT` · 557×543 · border bsDialog · StrRef table .data+0xF31F4 (47/63 captions matched) · form events: OnResize=FormResize, OnShow=FormShow*

**Purpose:** Sound Properties (UTS / GIT sound).

**Tabs** (`pcSound`): Basic · Positioning · Advanced · Comments (hidden tab)

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
| Basic | `bPlay` | button | Play | preview selected | `OnClick=bPlayClick` |
| Basic | `lbSounds` | listbox | List of Sounds to Play | UTS `Sounds[].Sound` | Del key removes; multi-select; `OnClick=lbSoundsClick OnKeyDown=lbSoundsKeyDown` |
| Basic | `bRemoveSound` | button | Remove | remove | `OnClick=bRemoveSoundClick` |
| Basic | `bAdd` | button | Add Sounds... | opens TdlgResOpenSound (multi-add) | `OnClick=bAddClick` |
| Basic | `bMoveDown` | button | Move Down | reorder | `OnClick=bMoveDownClick` |
| Basic | `bMoveUp` | button | Move Up | reorder | `OnClick=bMoveUpClick` |
| Basic | `bStop` | button | Stop | stop preview | `OnClick=bStopClick` |
| Basic | `bPaletteCategory` | button | ... (Category) | opens TdlgPaletteChooser | `OnClick=bPaletteCategoryClick` |
| Basic | `ePaletteCategory` | edit | Category | UTS `PaletteID` | read-only |
| Basic | `bLocName` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Basic | `eLocName` | edit | Name | UTS `LocName` | tip: “Edit the Name in the primary language” |
| Basic | `eTag` | edit | Tag | UTS `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
| Basic | `tbVolume` | slider | Volume | UTS `Volume` (0–127) | range 0…127; `OnChange=tbVolumeChange` |
| Basic | `eComments` | edit | Comments | (hidden) | **hidden** |
| Basic | `bComments` | button | ... (Comments) | (hidden) | **hidden**; `OnClick=bCommentsClick` |
| Basic | `mComments` | memo | Comments | UTS `Comment` | max 1024 |
| Positioning | `rbOmnipresent` | radio | Plays everywhere in area | UTS `Positional`=0 (area-wide) | `OnClick=rbPositionalClick` |
| Positioning | `rbPositionalRandom` | radio | Plays from a random position each time it is played | UTS `Positional`=1,`RandomPosition`=1 | `OnClick=rbPositionalClick` |
| Positioning | `rbPositional` | radio | Plays from a specific position (DFM: 'Plays from specific position') | UTS `Positional`=1,`RandomPosition`=0 | `OnClick=rbPositionalClick` |
| Positioning › Volume Distances | `eDistanceMax` | edit | Cutoff distance (m) | UTS `MaxDistance` | max 6; spin 0…32000 step 10; `OnExit=eDistanceMaxExit OnKeyDown=eEditUpDownKeyDown` |
| Positioning › Volume Distances | `eDistanceMin` | edit | Max Volume Distance (m) | UTS `MinDistance` | read-only; max 6; `OnExit=eDistanceMinExit OnKeyDown=eEditUpDownKeyDown` |
| Positioning › Volume Distances | `tbDistanceMin` | slider | ⟨Volume Distances⟩ | (hidden) | **hidden**; disabled; range 2…255; `OnChange=tbDistanceMinChange` |
| Positioning › Random Range | `eRandomRangeX` | edit | West-East Random Range (m) | UTS `RandomRangeX` | max 6; spin 0…32000; `OnExit=eRandomRangeXExit OnKeyDown=eEditUpDownKeyDown` |
| Positioning › Random Range | `eRandomRangeY` | edit | North-South Random Range (m) | UTS `RandomRangeY` | max 6; spin 0…32000; `OnExit=eRandomRangeYExit OnKeyDown=eEditUpDownKeyDown` |
| Positioning › Height (m) | `eElevation` | edit | Height (m) | UTS `Elevation` | max 6; `OnExit=eElevationExit` |
| Positioning › Height (m) | `tbElevation` | slider | ⟨Height (m)⟩ | (hidden) | **hidden**; range -10…10; `OnChange=tbElevationChange` |
| Advanced | `eTemplateResRef` | edit | Blueprint ResRef | UTS `TemplateResRef` | read-only; max 16; tip: “Edit the Blueprint ResRef”; `OnChange=eTemplateResRefChange` |
| Advanced | `ePitchVariation` | edit | Pitch Variation (octaves) | UTS `PitchVariation` | max 5; `OnExit=ePitchVariationExit` |
| Advanced | `tbPitchVariation` | slider | Pitch Variation (octaves) | UTS `PitchVariation` | range 0…100; `OnChange=tbPitchVariationChange` |
| Advanced | `bUpdateInstancesInArea` | button | Update Instances (DFM: 'Update instances in area') | push blueprint to instances | tip: “Update all instances created from this Blueprint”; `OnClick=bUpdateInstancesInAreaClick` |
| Advanced | `xbActive` | checkbox | Active | UTS `Active` | `OnClick=xbActiveClick` |
| Advanced › When to play | `rbTimeAlways` | radio | Play at all times (DFM: 'Always') | UTS `Times`=3 | `OnClick=rbTimeClick` |
| Advanced › When to play | `rbTimeNight` | radio | Play at night (DFM: 'Night') | UTS `Times`=2 | `OnClick=rbTimeClick` |
| Advanced › When to play | `rbTimeDay` | radio | Play during the day (DFM: 'Day') | UTS `Times`=1 | `OnClick=rbTimeClick` |
| Advanced › When to play | `rbTimeSpecific` | radio | Specific Hours | UTS `Times`=0 + `Hours` | `OnClick=rbTimeClick` |
| Advanced › When to play | `xb12AM` | checkbox | 12 AM | UTS `Hours` bit 0 |  |
| Advanced › When to play | `xb1AM` | checkbox | 1 AM | UTS `Hours` bit 1 |  |
| Advanced › When to play | `xb2AM` | checkbox | 2 AM | UTS `Hours` bit 2 |  |
| Advanced › When to play | `xb3AM` | checkbox | 3 AM | UTS `Hours` bit 3 |  |
| Advanced › When to play | `xb4AM` | checkbox | 4 AM | UTS `Hours` bit 4 |  |
| Advanced › When to play | `xb5AM` | checkbox | 5 AM | UTS `Hours` bit 5 |  |
| Advanced › When to play | `xb6AM` | checkbox | 6 AM | UTS `Hours` bit 6 |  |
| Advanced › When to play | `xb7AM` | checkbox | 7 AM | UTS `Hours` bit 7 |  |
| Advanced › When to play | `xb8AM` | checkbox | 8 AM | UTS `Hours` bit 8 |  |
| Advanced › When to play | `xb9AM` | checkbox | 9 AM | UTS `Hours` bit 9 |  |
| Advanced › When to play | `xb10AM` | checkbox | 10 AM | UTS `Hours` bit 10 |  |
| Advanced › When to play | `xb11AM` | checkbox | 11 AM | UTS `Hours` bit 11 |  |
| Advanced › When to play | `xb12PM` | checkbox | 12 PM | UTS `Hours` bit 12 |  |
| Advanced › When to play | `xb1PM` | checkbox | 1 PM | UTS `Hours` bit 13 |  |
| Advanced › When to play | `xb2PM` | checkbox | 2 PM | UTS `Hours` bit 14 |  |
| Advanced › When to play | `xb3PM` | checkbox | 3 PM | UTS `Hours` bit 15 |  |
| Advanced › When to play | `xb4PM` | checkbox | 4 PM | UTS `Hours` bit 16 |  |
| Advanced › When to play | `xb5PM` | checkbox | 5 PM | UTS `Hours` bit 17 |  |
| Advanced › When to play | `xb6PM` | checkbox | 6 PM | UTS `Hours` bit 18 |  |
| Advanced › When to play | `xb7PM` | checkbox | 7 PM | UTS `Hours` bit 19 |  |
| Advanced › When to play | `xb8PM` | checkbox | 8 PM | UTS `Hours` bit 20 |  |
| Advanced › When to play | `xb9PM` | checkbox | 9 PM | UTS `Hours` bit 21 |  |
| Advanced › When to play | `xb10PM` | checkbox | 10 PM | UTS `Hours` bit 22 |  |
| Advanced › When to play | `xb11PM` | checkbox | 11 PM | UTS `Hours` bit 23 |  |
| Advanced › Play Style | `rbLooping` | radio | Seamlessly looping | UTS `Looping`=1 | `OnClick=rbTypeClick` |
| Advanced › Play Style | `rbContinuous` | radio | Repeating (DFM: 'Repeat') | UTS `Continuous`=1,`Looping`=0 | `OnClick=rbTypeClick` |
| Advanced › Play Style | `rbOnce` | radio | Once (DFM: 'Play Once') | UTS `Continuous`=0 | `OnClick=rbTypeClick` |
| Advanced › Play Order | `rbSequential` | radio | Sequential | UTS `Random`=0 |  |
| Advanced › Play Order | `rbRandom` | radio | Random | UTS `Random`=1 |  |
| Advanced › Interval | `eInterval` | edit | Interval between playing sounds (seconds) | UTS `Interval` (ms; UI in s) | max 5; spin 0…100 step 10; `OnExit=eIntervalExit` |
| Advanced › Interval | `eIntervalVariation` | edit | Interval Variation (seconds) | UTS `IntervalVrtn` | max 5; spin 0…100 step 10; `OnExit=eIntervalVariationExit` |
| Advanced › Options for playing multiple sounds | `rbContinuousRandom` | radio | Continously choose a new random sound to play | (hidden legacy) | **hidden (container)**; `OnClick=rbPlayOptionsClick` |
| Advanced › Options for playing multiple sounds | `rbOnceRandom` | radio | Play a randomly selected sound once | (hidden legacy) | **hidden (container)**; `OnClick=rbPlayOptionsClick` |
| Advanced › Options for playing multiple sounds | `rbContinuousSequential` | radio | Continuously play sounds in order | (hidden legacy) | **hidden (container)**; `OnClick=rbPlayOptionsClick` |
| Advanced › Options for playing multiple sounds | `rbOnceSequential` | radio | Play list in order once | (hidden legacy) | **hidden (container)**; `OnClick=rbPlayOptionsClick` |
| Advanced › Options for playing a single sound | `rbContinuousSequentialSingle` | radio | Continuously play sound | (hidden legacy) | **hidden (container)**; `OnClick=rbPlayOptionsSingleClick` |
| Advanced › Options for playing a single sound | `rbOnceSequentialSingle` | radio | Options for playing multiple sounds (DFM: 'Play sound once') | (hidden legacy) | **hidden (container)**; `OnClick=rbPlayOptionsSingleClick` |
| Advanced › Sound Type | `rbOneShot` | radio | One or more sounds played individually (DFM: 'One or more distinct sounds played indivi… | (hidden legacy) | **hidden (container)**; `OnClick=rbTypeClick` |
| Advanced | `tbVolumeVariation` | slider | Volume Variation | UTS `VolumeVrtn` | range 0…127; `OnChange=tbVolumeVariationChange` |
| Advanced | `bVariables` | button | Variables... | UTS `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesClick` |
|  | `bDefaults` | button | Defaults | (hidden) | **hidden**; `OnClick=bDefaultsClick` |

**Notes / behaviors:**
- Volume/pitch/volume-variation sliders with min/mid/max tick labels; radii diagram images; hour checkboxes enabled only for *Specific Hours*; interval edits disabled for Looping. Priority (`Priority`) is derived from prioritygroups.2da (not shown). Legacy hidden "play options" radio groups remain.

#### `TdlgSoundWizard` — Sound Wizard
*DFM `TDLGSOUNDWIZARD` · StrRef table .data+0xF675C (16/19 captions matched)*

**Purpose:** Sound Wizard: palette category → timing → positioning → wave list → name (defaults from sounddefaultspos/stim.2da).

**Inherits:** `TdlgBlueprintWizard`.

**Tabs** (`pcSteps`): tsPalette · tsTiming · tsPositioning · tsWaveList · tsName

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| tsPalette | `tvPaletteSelector` | tree |  | `PaletteID` / wizard option |  |
| tsTiming | `rbLooping` | radio | Seamlessly looping (DFM: 'Looping') | timing preset (sounddefaultstim.2da Looping) | `OnClick=rbTimingClick OnDblClick=rbTimingDblClick` |
| tsTiming | `rbSingleShot` | radio | Single-shot(s) | timing preset (sounddefaultstim.2da) | `OnClick=rbTimingClick OnDblClick=rbTimingDblClick` |
| tsPositioning | `rbAreaWide` | radio | Area-wide | sounddefaultspos.2da AreaWide | `OnClick=rbPositioningClick OnDblClick=rbPositioningDblClick` |
| tsPositioning | `rbPositionalRandom` | radio | Random Positional | sounddefaultspos.2da PositionalRandom | `OnClick=rbPositioningClick OnDblClick=rbPositioningDblClick` |
| tsPositioning | `rbPositional` | radio | Positional | sounddefaultspos.2da Positional | `OnClick=rbPositioningClick OnDblClick=rbPositioningDblClick` |
| tsWaveList | `lbWaves` | listbox | Select the Wave files that this Sound Object will play | UTS `Sounds[]` | `OnClick=lbWavesClick OnKeyDown=lbWavesKeyDown` |
| tsWaveList | `bAddWaves` | button | Add Sounds... (DFM: 'Add Waves') | opens TdlgResOpenSound | `OnClick=bAddWavesClick` |
| tsWaveList | `bRemoveWave` | button | Remove (DFM: 'Remove Wave') | remove | disabled; `OnClick=bRemoveWaveClick` |
| tsName | `xbLaunchPropertiesDialog` | checkbox | Launch Properties Dialog | `PaletteID` / wizard option | default on |

## 12. Store (merchant) editor

#### `TdlgStoreEdit` — Merchant Properties
*DFM `TDLGSTOREEDIT` · 363×376 · border bsDialog · StrRef table .data+0xF7444 (32/36 captions matched) · form events: OnShow=FormShow · DFM caption 'Store'*

**Purpose:** Store (merchant) Properties (UTM / GIT store).

**Tabs** (`pcProperties`): Basic · Advanced · Restrictions · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bOk` | button | OK |  | tip: “Accept changes”; `OnClick=bOkClick` |
| Basic | `eName` | edit | Name | UTM `LocName` | tip: “Edit the Name in the primary language” |
| Basic | `bName` | button | ... | all languages | tip: “Edit text in multiple languages” |
| Basic | `eTag` | edit | Tag | UTM `Tag` | max 32; tip: “Edit the Tag”; `OnChange=eTagChange` |
| Basic | `bInventory` | button | Inventory ... | opens TdlgInventory (store mode: `StoreList[0..4].ItemList`) | tip: “Edit inventory contents”; `OnClick=bInventoryClick` |
| Basic › Stolen Goods | `cbBlackMarket` | checkbox | Buy Stolen Goods | UTM `BlackMarket` | tip: “Specify if this store will buy stolen goods”; `OnClick=cbBlackMarketClick` |
| Basic › Stolen Goods | `eBMMarkDown` | edit | Buy Mark Down | UTM `BM_MarkDown` (%) | disabled; read-only; spin 0…100; tip: “Edit store markdown for stolen goods”; `OnExit=eBMMarkDownExit OnKeyDown=eBMMarkDownKeyDown` |
| Basic › Pricing | `eMarkUp` | edit | Sell Mark Up | UTM `MarkUp` (%) | spin 1…1000; tip: “Edit store mark up value”; `OnExit=eMarkUpExit OnKeyDown=eMarkUpKeyDown` |
| Basic › Pricing | `eMarkDown` | edit | Buy Mark Down | UTM `MarkDown` (%) | spin 0…100; tip: “Edit store mark down value”; `OnExit=eMarkDownExit OnKeyDown=eMarkDownKeyDown` |
| Basic › Pricing | `eIdentifyPrice` | edit | Identify Price | UTM `IdentifyPrice` | max 9; tip: “Enter the price to identify an item”; `OnExit=eIdentifyPriceExit OnKeyDown=OnPriceKeyDown` |
| Basic › Pricing | `xbWillIdentify` | checkbox | Will Identify Items | UTM `IdentifyPrice` ≠ -1 | `OnClick=xbWillIdentifyClick` |
| Basic | `ePaletteCategory` | edit | Category | UTM `ID` (palette id) | read-only |
| Basic | `bPaletteCategory` | button | ... | opens TdlgPaletteChooser | tip: “Select Blueprint category”; `OnClick=bPaletteCategoryClick` |
| Basic | `bUniqueTag` | button | ... | unique tag | tip: “Edit the Tag”; `OnClick=bUniqueTagClick` |
| Basic › Restrictions | `eMaxBuyPrice` | edit | Max Buy Price | UTM `MaxBuyPrice` | max 9; tip: “Enter the maximum price that the store will pay for an item”; `OnExit=eMaxBuyPriceExit OnKeyDown=OnPriceKeyDown` |
| Basic › Restrictions | `xbHasMaxBuyPrice` | checkbox | Has Maximum Buy Price | UTM `MaxBuyPrice` ≠ -1 | `OnClick=xbHasMaxBuyPriceClick` |
| Basic › Restrictions | `xbHasLimitedGold` | checkbox | Has Limited Gold | UTM `StoreGold` ≠ -1 | `OnClick=xbHasLimitedGoldClick` |
| Basic › Restrictions | `eStoreGold` | edit | Gold Amount | UTM `StoreGold` | max 9; tip: “Enter the amount of gold that this store has”; `OnExit=eStoreGoldExit OnKeyDown=OnPriceKeyDown` |
| Advanced | `eResRef` | edit | Blueprint ResRef | UTM `ResRef` (template resref) | max 16; tip: “Edit the Blueprint ResRef”; `OnChange=eResRefChange` |
| Advanced › Scripts | `bOnOpenStore` | button | ... (OnOpenStore) | browse | tip: “Select a script”; `OnClick=bOnOpenStoreClick` |
| Advanced › Scripts | `bOnOpenStoreEdit` | button | Edit | edit | tip: “Edit a script”; `OnClick=bOnOpenStoreEditClick` |
| Advanced › Scripts | `cbOnOpenStore` | combo | OnOpenStore | UTM `OnOpenStore` | max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Advanced › Scripts | `bOnStoreClosed` | button | ... (OnStoreClosed) | browse | tip: “Select a script”; `OnClick=bOnStoreClosedClick` |
| Advanced › Scripts | `cbOnStoreClosed` | combo | OnStoreClosed | UTM `OnStoreClosed` | buttons [Edit]; max 16; editable; tip: “Select a script”; `OnChange=cbOnScriptChange` |
| Advanced | `bUpdateInstancesInArea` | button | Update Instances (DFM: 'Update instances in area') | push blueprint to instances | tip: “Update all instances created from this Blueprint”; `OnClick=bUpdateInstancesInAreaClick` |
| Advanced | `bVariables` | button | Variables... | UTM `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesClick` |
| Restrictions | `rbWillNotBuy` | radio | Store will NOT buy the following items | UTM `WillNotBuy[]` mode | `OnClick=rbWillNotBuyClick` |
| Restrictions | `rbWillOnlyBuy` | radio | Store will ONLY buy the following items | UTM `WillOnlyBuy[]` mode | `OnClick=rbWillOnlyBuyClick` |
| Restrictions | `lbRestrictedItems` | listbox | Restricted Items | UTM `WillNotBuy`/`WillOnlyBuy` `BaseItem` list (baseitems.2da) | `OnClick=lbRestrictedItemsClick OnDblClick=bRestrictedItemsRemoveClick` |
| Restrictions | `bRestrictedItemsAdd` | button | -> | add base item | `OnClick=bRestrictedItemsAddClick` |
| Restrictions | `bRestrictedItemsRemove` | button | <- | remove | `OnClick=bRestrictedItemsRemoveClick` |
| Restrictions | `bRestrictedItemsRemoveAll` | button | << | clear | `OnClick=bRestrictedItemsRemoveAllClick` |
| Restrictions | `bRestrictedItemsAddAll` | button | >> | (hidden) | **hidden**; disabled; `OnClick=bRestrictedItemsAddAllClick` |
| Comments | `mComments` | memo | ⟨Comments⟩ | UTM `Comment` |  |

**Notes / behaviors:**
- Mark-up/down spinners use `OnChangingEx` validation; black-market mark-down enabled only with *Buy Stolen Goods*; Restrictions tab moves base items between a base-item chooser (`pBaseItemChooser`, runtime tree) and the restricted list.

#### `TdlgStoreWizard` — Store Wizard
*DFM `TDLGSTOREWIZARD` · no StrRef table resolved*

**Purpose:** Merchant Wizard.

**Inherits:** `TdlgBlueprintWizard`.

**Tabs** (`pcSteps`): tsPalette · tsName

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| tsPalette | `tvPaletteSelector` | tree |  | UTM `ID` |  |
| tsName | `eName` | edit |  | UTM `LocName` |  |
| tsName | `xbLaunchPropertiesDialog` | checkbox | Launch Properties Dialog | open properties after |  |

#### `TdlgStoreSetupWizard` — Store Setup Wizard
*DFM `TDLGSTORESETUPWIZARD` · StrRef table .data+0xF92A8 (17/19 captions matched)*

**Purpose:** Store Setup Wizard (context menu on a creature): writes shopkeeper conversation + open-store script and links a store.

**Inherits:** `TdlgWizard`.

**Tabs** (`pcSteps`): tsConversation · tsChooseStore · tsChooseShopkeeper · tsFinish

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| tsConversation | `mGreeting` | memo | What does the shopkeeper say when the conversation begins? | DLG NPC entry text | `OnChange=eConversationChange` |
| tsConversation | `eReplyYes` | edit | What does the player say when the player wishes to see the store? | DLG PC reply (opens store) | `OnChange=eConversationChange` |
| tsConversation | `eReplyNo` | edit | What does the player say when the player is not interested? | DLG PC reply (exit) | `OnChange=eConversationChange` |
| tsConversation | `eConversationResRef` | edit | (optional) Enter the filename for the shopkeeper's conversation | new DLG resref → creature `Conversation` | max 16; `OnChange=eResRefChange` |
| tsConversation | `eOpenStoreScriptResRef` | edit | (optional) Enter the filename for the script that will open the store | generated NSS resref (nw_ / gplotAppraiseOpenStore) | max 16; `OnChange=eResRefChange` |
| tsConversation | `xbUseAppraiseCheck` | checkbox | Use appraise checks (DFM: 'Use Appraise Check') | script uses appraise-adjusted OpenStore | default on |
| tsChooseStore | `pPalettesStore` | color swatch/panel | Choose the store to use. | hosts TfraBlueprintSelect (stores) |  |
| tsChooseShopkeeper | `pPalettesShopkeeper` | color swatch/panel | Choose the shopkeeper to use. | hosts TfraBlueprintSelect (creatures) |  |
| tsFinish | `cbFaction` | combo |  | shopkeeper `FactionID` override | **hidden (container)**; list |
| tsFinish | `xbUseSuggestedFaction` | checkbox | Use selected faction instead: | apply faction change | **hidden (container)**; default on |
| tsFinish | `eOldFaction` | edit | Current faction: | current faction (info) | **hidden (container)** |

## 13. Waypoint editor

#### `TdlgWaypointEdit` — Waypoint Properties
*DFM `TDLGWAYPOINTEDIT` · 309×235 · border bsDialog · StrRef table .data+0x10D980 (16/18 captions matched) · form events: OnShow=FormShow*

**Purpose:** Waypoint Properties (UTW / GIT waypoint).

**Tabs** (`pcWaypoint`): Basic · Advanced · Description · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
| Basic | `eTag` | edit | Tag | UTW `Tag` | max 32; `OnChange=eTagChange` |
| Basic | `eLocName` | edit | Name | UTW `LocalizedName` |  |
| Basic | `eLinkedTo` | edit | ⟨Basic⟩ | (hidden) UTW `LinkedTo` | **hidden** |
| Basic | `bLocName` | button | ... (Name) | all languages |  |
| Basic | `ePaletteCategory` | edit | Category | UTW `PaletteID` | read-only |
| Basic | `bPaletteCategory` | button | ... (Category) | opens TdlgPaletteChooser | `OnClick=bPaletteCategoryClick` |
| Basic | `cbAppearance` | combo | Appearance Type | UTW `Appearance` (waypoint.2da) | list |
| Advanced | `eMapNote` | edit | Map Note Text | UTW `MapNote` | disabled |
| Advanced | `bMapNote` | button | ... (Map Note Text) | all languages | disabled |
| Advanced | `cbHasMapNote` | checkbox | Waypoint Contains a Map Note | UTW `HasMapNote` | `OnClick=cbHasMapNoteClick` |
| Advanced | `cbMapNoteEnabled` | checkbox | Map Note Enabled | UTW `MapNoteEnabled` | disabled |
| Advanced | `eResRef` | edit | Blueprint ResRef | UTW `TemplateResRef` | max 16; tip: “Edit the Blueprint ResRef”; `OnChange=eResRefChange` |
| Advanced | `bUpdateInstancesInArea` | button | Update instances in current area | push blueprint to instances | **hidden**; `OnClick=bUpdateInstancesInAreaClick` |
| Advanced | `bVariables` | button | Variables... | UTW `VarTable` | tip: “Edit scripting variables”; `OnClick=bVariablesClick` |
| Description | `mDescription` | memo | ⟨Description⟩ | UTW `Description` |  |
| Description | `bDescription` | button | ... | all languages |  |
| Comments | `mComments` | memo | ⟨Comments⟩ | UTW `Comment` |  |

**Notes / behaviors:**
- Map-note edits enabled only when *Waypoint Contains a Map Note* is checked.

#### `TdlgWaypointWizard` — Waypoint Wizard
*DFM `TDLGWAYPOINTWIZARD` · 372×271 · StrRef table .data+0x10EDF8 (2/3 captions matched)*

**Purpose:** Waypoint Wizard.

**Inherits:** `TdlgBlueprintWizard`.

**Tabs** (`pcSteps`): tsPalette · tsName · tsBasic

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| tsPalette | `tvPaletteSelector` | tree |  | UTW `PaletteID` |  |
| tsPalette | `xbLaunchWaypointDialog` | checkbox | Launch Properties Dialog (DFM: 'Launch Waypoint Properties Dialog') | open properties after |  |
| tsName | `eName` | edit |  | UTW `LocalizedName` |  |
| tsBasic | `eTag` | edit | Tag | UTW `Tag` | max 32; `OnChange=eTagChange` |
| tsBasic | `cbAppearance` | combo | Appearance | UTW `Appearance` | list; `OnChange=cbAppearanceChange` |
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |

## 14. Wizard base classes

#### `TdlgWizard` — Wizard
*DFM `TDLGWIZARD` · 499×370 · border bsDialog · StrRef table .data+0x42F8 (5/5 captions matched) · form events: OnShow=FormShow*

**Purpose:** Generic wizard base.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | Help |  | **hidden** |
|  | `bFinish` | button | Finish |  | disabled; tip: “Finish the Wizard”; `OnClick=bFinishClick` |
|  | `bNext` | button | Next |  | tip: “Continue to the next step in the Wizard”; `OnClick=bNextClick` |
|  | `bBack` | button | Back |  | disabled; tip: “Return to the previous step in the Wizard”; `OnClick=bBackClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |

#### `TdlgBlueprintWizard` — Blueprint Wizard
*DFM `TDLGBLUEPRINTWIZARD` · 405×319 · no StrRef table resolved*

**Inherits:** `TdlgWizard`. Base for Door/Encounter/Placeable/Sound/Store/Trigger/Waypoint wizards: palette-category step + name step.

**Tabs** (`pcSteps`): tsPalette · tsName

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| tsPalette | `tvPaletteSelector` | tree | Choose the palette category that this new blueprint should appear under | `PaletteID` | `OnChange=tvPaletteSelectorChange OnClick=tvPaletteSelectorClick OnDblClick=tvPaletteSelectorDblClick OnMouseDown=tvPaletteSelectorMouseDown` |
| tsName | `eName` | edit |  | name → resref/tag derived | max 1023; `OnChange=eNameChange` |

## 15. Conversation editor

#### `TdlgConversationEditor` — Conversation Editor
*DFM `TDLGCONVERSATIONEDITOR` · 939×660 · StrRef table .data+0x7F60 (29/60 captions matched) · form events: OnCloseQuery=FormCloseQuery, OnDestroy=FormDestroy, OnResize=FormResize, OnShow=FormShow*

**Purpose:** Conversation Editor (DLG), multi-file (one tab per open conversation + Scrap tab).

**Tabs** (`pcMainText`): Scrap
**Tabs** (`pcBottom`): Data · Bookmarks · Search
**Tabs** (`pcData`): Text Appears When ... · Actions Taken · Other Actions · Comments · Current File

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `RightViewStatusBar` | status bar |  | status (4 panels; dbl-click) | `OnDblClick=RightViewStatusBarDblClick` |
| Scrap | `fraConversationTree1` | embedded frame | `TfraConversationTree` | Scrap tree (cut nodes) | see frame section |
|  | `tbNew` | tool button | New |  | tip: “Create a new file”; `Action=actNew` |
|  | `tbOpen` | tool button | Open |  | tip: “Open an existing file”; `Action=actOpen` |
|  | `tbClose` | tool button | Close File |  | tip: “Close the current file”; `Action=actClose` |
|  | `tbSave` | tool button | Save File |  | tip: “Save the current file”; `Action=actSave` |
|  | `tbSaveAs` | tool button | Save As |  | tip: “Save As”; `Action=actSaveAs` |
|  | `tbSaveAll` | tool button | Save All |  | tip: “Save All”; `Action=actSaveAll` |
|  | `tbOptions` | tool button | Options |  | tip: “Options”; `Action=actShowOptionsDlg` |
|  | `tbFilterComment` | tool button |  | highlight nodes with `Comment` | toggle; tip: “Highlight Comments”; `OnClick=tbFilterClick` |
|  | `tbFilterAction` | tool button |  | highlight nodes with `Script` | toggle; tip: “Highlight Actions”; `OnClick=tbFilterClick` |
|  | `tbFilterQuest` | tool button |  | highlight nodes with `Quest` | toggle; tip: “Highlight Quests”; `OnClick=tbFilterClick` |
|  | `tbFilterAnimation` | tool button | Animation | highlight nodes with `Animation` | toggle; tip: “Highlight Animations”; `OnClick=tbFilterClick` |
|  | `tbFilterSound` | tool button | Sound | highlight nodes with `Sound` | toggle; tip: “Highlight Sounds”; `OnClick=tbFilterClick` |
|  | `tbAdd` | tool button | Add |  | tip: “Add”; `Action=actConversationAdd` |
|  | `tbCopy` | tool button | Copy |  | tip: “Copy”; `Action=actConversationCopy` |
|  | `tbCut` | tool button | Cut |  | tip: “Cut”; `Action=actConversationCut` |
|  | `tbPaste` | tool button | Paste |  | tip: “Paste”; `Action=actConversationPaste` |
|  | `tbFind` | tool button | actFindText |  | tip: “Find Text”; `Action=actFindText` |
|  | `tbFindNext` | tool button | actFindNextText |  | tip: “Find Next”; `Action=actFindNextText` |
|  | `tbFindInFiles` | tool button | Find In Files |  | tip: “Find In Files”; `Action=actFindInFiles` |
|  | `tbReplace` | tool button | actReplace |  | tip: “Replace”; `Action=actReplace` |
|  | `tbExpandAll` | tool button | Expand All | expand tree | tip: “Expand All”; `OnClick=tbExpandAllClick` |
|  | `tbCompressAll` | tool button | Compress All | collapse tree | tip: “Collapse All”; `OnClick=tbCompressAllClick` |
|  | `bOK` | button | Done | close editor | `OnClick=actQuitExecute` |
| Data › Text Appears When ... | `bConditionWizard` | speed button | (no caption; glyph) | Script Wizard (condition) | tip: “Script Wizard”; `OnClick=bWizardClick` |
| Data › Text Appears When ... | `bConditionEdit` | button | Edit | edit condition script | tip: “Select a script”; `OnClick=actLaunchScriptEditorExecute` |
| Data › Text Appears When ... | `bConditionBrowse` | button | ... | browse | `OnClick=bBrowseScriptClick` |
| Data › Text Appears When ... | `xbOnceOnly` | checkbox | Show Once Only | (hidden) | **hidden** |
| Data › Text Appears When ... | `cbCondition` | combo | Script | DLG link `Active` (StartingList/RepliesList/EntriesList struct) | max 16; editable; `OnChange=actUpdateCondtionScriptExecute` |
| Data › Text Appears When ... | `vlConditionParameters` | key/value grid | ⟨Text Appears When ...⟩ | DLG link `ConditionParams[]` (Key/Value, EE) | `OnSetEditText=vlParametersSetEditText` |
| Data › Text Appears When ... | `bAddConditionParameter` | button | + | add param | `OnClick=bAddScriptParameterClick` |
| Data › Text Appears When ... | `bRemoveConditionParameter` | button | - | remove param | `OnClick=bRemoveScriptParameterClick` |
| Data › Text Appears When ... › Script Preview | `mConditionScript` | memo | ⟨Script Preview⟩ | NSS preview | read-only |
| Data › Actions Taken | `bActionWizard` | speed button | (no caption; glyph) | Script Wizard (action) | tip: “Script Wizard”; `OnClick=bWizardClick` |
| Data › Actions Taken | `bEditScript` | button | Edit | edit action script | tip: “Select a script”; `OnClick=actLaunchScriptEditorExecute` |
| Data › Actions Taken | `cbActionScript` | combo | Script | DLG node `Script` | buttons […]; max 16; editable; `OnChange=actUpdateScriptExecute` |
| Data › Actions Taken | `vlActionParameters` | key/value grid | ⟨Actions Taken⟩ | DLG node `ActionParams[]` (EE) | `OnSetEditText=vlParametersSetEditText` |
| Data › Actions Taken | `bRemoveActionParameter` | button | - | remove param | `OnClick=bRemoveScriptParameterClick` |
| Data › Actions Taken | `bAddActionParameter` | button | + | add param | `OnClick=bAddScriptParameterClick` |
| Data › Actions Taken › Script Preview | `mActionScriptPreview` | memo | ⟨Script Preview⟩ | NSS preview | read-only |
| Data › Other Actions | `cbAnimation` | combo | Play animation | DLG node `Animation` | disabled; list; `OnClick=cbAnimationChange` |
| Data › Other Actions | `cbSound` | combo | Play Sound | DLG node `Sound` | max 16; editable; `OnChange=cbSoundChange OnClick=cbSoundChange` |
| Data › Other Actions | `bSoundPlay` | button | Play | preview | `OnClick=bSoundPlayClick` |
| Data › Other Actions | `cbQuest` | combo | Journal | DLG node `Quest` (journal tag) | list; `OnClick=actQuestChangeExecute` |
| Data › Other Actions | `cbQuestEntry` | combo | Journal entry | DLG node `QuestEntry` | list; `OnClick=cbQuestEntryChange` |
| Data › Other Actions | `bJournalEditor` | button | Edit | opens Journal Editor | `OnClick=bJournalEditorClick` |
| Data › Other Actions | `xbAnimLoop` | checkbox | Loop | (hidden) `AnimLoop` | **hidden**; `OnClick=xbAnimLoopClick` |
| Data › Comments | `mComment` | memo | ⟨Comments⟩ | DLG node `Comment` | `OnChange=mCommentChange OnKeyDown=mCommentKeyDown` |
| Data › Current File | `bScriptBrowse1` | button | ... | browse | `OnClick=bBrowseScriptClick` |
| Data › Current File | `bScriptEditor2` | button | Edit | edit | tip: “Select a script”; `OnClick=bScriptEditorClick` |
| Data › Current File | `bScriptEditor1` | button | Edit | edit | tip: “Select a script”; `OnClick=bScriptEditorClick` |
| Data › Current File | `bScriptBrowse2` | button | ... | browse | `OnClick=bBrowseScriptClick` |
| Data › Current File | `cbEndConvScript` | combo | Normal | DLG `EndConversation` | max 16; editable; `OnChange=actUpdateScriptExecute OnClick=cbEndConvScriptClick` |
| Data › Current File | `cbEndConvAbortScript` | combo | Aborted | DLG `EndConverAbort` | max 16; editable; `OnChange=actUpdateScriptExecute OnClick=cbEndConvScriptClick` |
| Data › Current File › Script Preview | `mFileScriptPreview` | memo | ⟨Script Preview⟩ | NSS preview | read-only |
| Data › Current File | `xbFileZoomIn` | checkbox | Stop Camera Zoom In | DLG `PreventZoomIn` (EE) | `OnClick=xbFileZoomInClick` |
| Data | `bEditText` | button | ... | node `Text` all languages | tip: “Edit text in multiple languages”; `OnClick=bEditTextClick` |
| Data | `RightTextDelayEdit` | edit | ⟨Data⟩ | (hidden) node `Delay` | **hidden**; `OnChange=RightTextDelayEditChange` |
| Data | `bInsertToken` | button | (no caption; glyph) | opens TdlgTokenSelector | tip: “Insert Token”; `OnClick=actGetTokenExecute` |
| Data | `imSpeaker` | image |  | speaker portrait |  |
| Data | `mText` | memo | Text | DLG node `Text` (CExoLocString) | max 1024; `OnChange=mTextChange OnExit=mTextExit OnKeyDown=mTextKeyDown` |
| Data | `cbSpeakers` | combo | Speaker Tag | DLG entry `Speaker` (tags of creatures in module) | list; `OnClick=cbSpeakersChange` |
| Data | `bAddNPCTag` | button | Add | add tag to speaker list | `Action=actAddNPCTagToList` |
| Bookmarks | `lbBookmarks` | listbox | ⟨Bookmarks⟩ | bookmarked nodes | `OnDblClick=lbBookmarksDblClick` |
| Search | `lbSearchResults` | listbox | ⟨Search⟩ | search hits | `OnDblClick=lbSearchResultsDblClick` |
|  | `ExportSaveDialog` | file dialog | File To Export To | text export | filter `Text Files (*.TXT)/*.txt/All Files (*.*)/*.*` ext .txt |

**Action list `MainActionList`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actNew` | New |  | `actNewExecute` |  | Create a new file |
| `actOpen` | Open |  | `actOpenExecute` |  | Open an existing file |
| `actClose` | Close File |  | `actCloseExecute` |  | Close the current file |
| `actSave` | Save File |  | `actSaveExecute` |  | Save the current file |
| `actSaveAs` | Save As |  | `actSaveAsExecute` |  | Save As |
| `actSaveAll` | Save All |  | `actSaveAllExecute` |  | Save All |
| `actQuit` | Exit |  | `actQuitExecute` |  |  |
| `actShowOptionsDlg` | Options |  | `actShowOptionsDlgExecute` |  | Options |
| `actUpdateActionScript` | Update Action |  | `actUpdateScriptExecute` |  |  |
| `actAddNPCTagToList` | Add |  | `actAddNPCTagToListExecute` |  |  |
| `actRemoveNPCTagFromList` | Remove |  | `actRemoveNPCTagFromListExecute` |  |  |
| `actUpdateEndConvScript` | actUpdateEndConvScript |  | `actUpdateScriptExecute` |  |  |
| `actUpdateEndConvAbortScript` |  |  |  |  |  |
| `actTest` | Test |  |  |  |  |
| `actExportDialog` | Export |  | `actExportDialogExecute` |  |  |
| `actGetToken` | actGetToken |  | `actGetTokenExecute` |  |  |
| `actLaunchScriptEditor` | Edit |  | `actLaunchScriptEditorExecute` |  |  |
| `actQuestChange` | actQuestChange |  | `actQuestChangeExecute` |  |  |
| `actSetModified` | actSetModified |  | `actSetModifiedExecute` |  |  |
| `actImportDialog` | Import |  | `actImportDialogExecute` |  |  |
| `actCheckSpelling` | Spell Check (DFM 'Spelling') |  | `actCheckSpellingExecute` |  |  |
| `actConversationAdd` | Add |  | `actConversationActionExecute` |  | Add |
| `actConversationCopy` | Copy |  | `actConversationActionExecute` |  | Copy |
| `actConversationCut` | Cut |  | `actConversationActionExecute` |  | Cut |
| `actConversationPaste` | Paste |  | `actConversationActionExecute` |  | Paste |
| `actFindText` | actFindText | Ctrl+F | `actFindTextExecute` |  | Find Text |
| `actFindNextText` | actFindNextText | F3 | `actFindNextTextExecute` |  | Find Next |
| `actFindInFiles` | Find In Files |  | `actFindInFilesExecute` |  | Find In Files |
| `actReplace` | actReplace | Ctrl+R | `actReplaceExecute` |  | Replace |
| `actUpdateCondtionScript` | actUpdateCondtionScript |  | `actUpdateCondtionScriptExecute` |  |  |
| `actExpandConvTree` | actExpandConvTree |  | `actExpandConvTreeExecute` |  | Expand All |
| `actCollapseConvTree` | actCollapseConvTree |  | `actCollapseConvTreeExecute` |  | Collapse All |

**Popup menu `pmLowerText`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miInsertToken` | Insert Token |  |  `actGetTokenExecute` |  |
| `miSpelling` | Spell Check |  | `actCheckSpelling` |  |

**Popup menu `pmConditional`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miUniqueScript` | Unique Script |  |  `miUniqueScriptClick` |  |

**Notes / behaviors:**
- Top: toolbar (file + filters), tabbed trees (`pcMainText`: one tab per open DLG + Scrap); bottom: node text (speaker combo, `mText`, `...` languages, token button, portrait) and Data tabs (Text Appears When… / Actions Taken / Other Actions / Comments / Current File), plus Bookmarks and Search result tabs.
- Condition/action script panes show a read-only preview of the NSS source. EE parameter grids (`TValueListEditor`) with +/− add key/value pairs passed to scripts.
- Speaker combo lists creature tags in the module; `Add` stores extra tags. Quest combo lists journal categories, entry combo lists entries.
- Hidden: `xbOnceOnly`, `RightTextDelayEdit`, `xbAnimLoop`, `lUnique`; actions without UI: `actExportDialog`, `actImportDialog`, `actTest`, `actRemoveNPCTagFromList`.
- Conditional popup `pmConditional` → *Unique Script* (generate a node-specific script name).

#### `TfraConversationTree` — (no caption)
*DFM `TFRACONVERSATIONTREE` · StrRef table .data+0x12080 (9/11 captions matched)*

**Purpose:** Conversation tree frame (one per open DLG tab).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `tvMain` | tree |  | DLG `StartingList` → `EntryList`/`ReplyList` graph (links = `IsChild`=1) | drag source; `OnChange=tvMainChange OnClick=tvMainClick OnDblClick=tvMainDblClick OnDragDrop=tvMainDragDrop OnMouseDown=tvMainMouseDown` |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actCopy` | Copy |  | `actCopyExecute` |  |  |
| `actDelete` | Delete (DFM 'Cut') |  | `actDeleteExecute` |  |  |
| `actDeleteWithCopy` | Cut |  | `actDeleteWithCopyExecute` |  |  |
| `actSave` | Save |  | `actSaveExecute` |  |  |
| `actPaste` | Paste |  | `actPasteExecute` |  |  |
| `actGetDelete` | Delete |  | `actGetDeleteExecute` |  |  |
| `actAdd` | Add |  | `actAddExecute` |  |  |
| `actPasteAsLink` | Paste As Link |  | `actPasteAsLinkExecute` |  |  |
| `actClose` | Close |  | `actCloseExecute` |  |  |
| `actTest` | Test |  | `actTestExecute` |  |  |
| `actExpandAll` | Expand All |  | `actExpandAllExecute` |  |  |
| `actCollapseAll` | Collapse All |  | `actCollapseAllExecute` |  |  |
| `actSetBookmark` | Bookmark |  | `actSetBookmarkExecute` |  |  |
| `actSetModified` |  |  | `actSetModifiedExecute` |  |  |
| `actSpellCheck` | Spell Check (DFM 'Check Spelling') |  | `actSpellCheckExecute` |  |  |

**Popup menu `pmMain`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miAdd` | Add |  | `actAdd` |  |
| `miCopy` | Copy |  | `actCopy` |  |
| `miDelete` | Cut |  | `actDeleteWithCopy` |  |
| `miPaste` | Paste |  | `actPaste` |  |
| `miPasteAsLink` | Paste As Link |  | `actPasteAsLink` |  |
| `miBookmark` | Bookmark |  | `actSetBookmark` |  |
| — | ——— | | | |
| `miSave` | Save |  | `actSave` |  |
| `miClose` | Close |  | `actClose` |  |
| `miTest` | Test |  | `actTest` |  |
| `miSpellCheck` | Spell Check |  | `actSpellCheck` |  |
| `miTEMP` | -- REMOVE STRREF -- |  |  `miTEMPClick` |  |
| `miTEMPTREE` | -- REMOVE STRREF FROM TREE-- |  |  `miTEMPTREEClick` |  |
| `miTEXTDUMP` | -- TEXT DUMP -- |  |  `miTEXTDUMPClick` |  |

**Notes / behaviors:**
- Custom-drawn nodes (colours from Options; link nodes grey; highlight filters); drag & drop move/link (`tvMainDragDrop`); timer `tMain` for auto-backup/scroll. Keyboard: Ctrl+A add, Ctrl+C/X/V, Delete, arrows.

#### `TdlgConversationInput` — Input Text
*DFM `TDLGCONVERSATIONINPUT` · 361×220 · border bsDialog · StrRef table .data+0xF3D0 (3/5 captions matched) · form events: OnActivate=FormActivate*

**Purpose:** Popup to type new node text (Options: "Show popup when creating a new text entry").

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `TextMemo` | memo |  | new node `Text` |  |
|  | `OKButton` | button | OK |  | tip: “Accept changes” |
|  | `CancelButton` | button | Cancel |  | tip: “Discard changes” |

**Action list `ActionList1`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actInsertToken` | Insert Token |  | `actInsertTokenExecute` |  |  |

**Popup menu `mPopupMenu`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miInsertToken` | Insert Token |  |  `actInsertTokenExecute` |  |

#### `TdlgConversationSearch` — Search
*DFM `TDLGCONVERSATIONSEARCH` · 394×249 · border bsDialog · StrRef table .data+0x11154 (13/18 captions matched) · form events: OnActivate=FormActivate*

**Purpose:** Find / Replace in conversations.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `cbSearch` | combo | Find What | find text (history) | editable |
|  | `cbReplace` | combo | Replace With | replace text (history) | editable |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
| Language | `rbCurrentLanguage` | radio | Current Language | scope | default on |
| Language | `rbAllLanguages` | radio | All Languages | scope |  |
| Gender | `xbMale` | checkbox | Male | gender scope | default on; `OnClick=xbGenderClick` |
| Gender | `xbFemale` | checkbox | Female | gender scope | default on; `OnClick=xbGenderClick` |
| Other | `xbCaseSensitive` | checkbox | Match Case (DFM: 'Case Sensitive') | option |  |
| Other | `xbWholeWord` | checkbox | Match Whole Word Only (DFM: 'Whole Word') | option |  |
| Scope | `rbCurrentFile` | radio | Current File | scope: current file | default on |
| Scope | `rbCurrentlyOpen` | radio | Currently Open Files | scope: open files |  |
| Scope | `rbAllFiles` | radio | All Files in Module | scope: all module DLGs |  |

#### `TdlgConversationTest` — Conversation Test
*DFM `TDLGCONVERSATIONTEST` · 258×319 · border bsDialog · no StrRef table resolved · form events: OnShow=FormShow*

**Purpose:** Conversation test-run window (click-through of the tree).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bDone` | button | Done |  |  |
|  | `bBack` | button | <-- | step back | `OnClick=bBackClick` |

#### `TdlgConversationExportPicker` — Select Mode
*DFM `TDLGCONVERSATIONEXPORTPICKER` · 179×120 · border bsDialog · no Tags — English only*

**Purpose:** Export mode chooser for conversation text export.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `rbStringBased` | radio | String based | export mode | default on |
|  | `rbCharBased` | radio | Character based | export mode |  |
|  | `OKButton` | button | OK |  |  |
|  | `CancelButton` | button | Cancel |  |  |

#### `TdlgTokenSelector` — Select Token
*DFM `TDLGTOKENSELECTOR` · 370×290 · border bsDialog · StrRef table .data+0xFF7D0 (9/9 captions matched) · form events: OnCreate=FormCreate, OnDestroy=FormDestroy*

**Purpose:** Insert Token (custom tokens from stringtokens.2da, highlight colour tokens).

**Tabs** (`pcMain`): Standard · Highlight

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bOK` | button | OK |  | tip: “Accept changes” |
| Standard | `lbTokens` | listbox | ⟨Standard⟩ | stringtokens.2da tokens | `OnClick=actLoadExamplesExecute` |
| Standard | `lbExamples` | listbox | ⟨Standard⟩ | token example expansions |  |
| Highlight | `rbHighlightAction` | radio | Action | <StartAction>…</Start> | default on; `Action=actRadioButtonPress` |
| Highlight | `rbHighlightSkillCheck` | radio | Skill Check | <StartCheck>…</Start> | `Action=actRadioButtonPress` |
| Highlight | `rbHighlight` | radio | Highlight | <StartHighlight>…</Start> | `Action=actRadioButtonPress` |
| Highlight | `eHighlightAction` | edit | ⟨Highlight⟩ | text to wrap |  |
| Highlight | `eHighlight` | edit | ⟨Highlight⟩ | text to wrap |  |
| Highlight | `cbHighlightSkill` | combo | ⟨Highlight⟩ | skill for [Skill] check prefix | disabled; list |

**Action list `ActionList1`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actLoadTokens` |  |  | `actLoadTokensExecute` |  |  |
| `actLoadExamples` |  |  | `actLoadExamplesExecute` |  |  |
| `actRadioButtonPress` |  |  | `actRadioButtonPressExecute` |  |  |

## 16. Script editor & script wizard

#### `TdlgScriptEditor` — Script Editor
*DFM `TDLGSCRIPTEDITOR` · StrRef table .data+0xE09E4 (23/37 captions matched) · form events: OnClose=FormClose, OnCloseQuery=FormCloseQuery, OnShow=FormShow*

**Purpose:** Script Editor (NSS), multi-tab, with compiler output/help/bookmarks/search panes.

**Tabs** (`pcShortcuts`): Functions · Variables · Constants · Templates
**Tabs** (`pcInfo`): Compiler · Help · Bookmarks · Search Results

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `eFilter` | edit | Filter: | filter side lists | `OnChange=actFilterExecute OnKeyDown=eFilterKeyDown` |
| Functions | `lbFunctions` | listbox | ⟨Functions⟩ | nwscript.nss + include functions | `OnClick=lbFunctionsClick OnDblClick=ShortcutListDblClick OnDrawItem=lbShortcutsDrawItem OnKeyDown=ShortcutsKeyDown` |
| Variables | `lbVariables` | listbox | ⟨Variables⟩ | globals in current script | `OnClick=lbVariablesClick OnDblClick=ShortcutListDblClick OnKeyDown=ShortcutsKeyDown` |
| Constants | `lbConstants` | listbox | ⟨Constants⟩ | constants | `OnClick=lbConstantsClick OnDblClick=ShortcutListDblClick OnDrawItem=lbShortcutsDrawItem OnKeyDown=ShortcutsKeyDown` |
| Templates | `lbTemplates` | listbox | ⟨Templates⟩ | script templates dir | `OnClick=lbTemplatesClick OnDblClick=lbTemplatesDblClick OnKeyDown=ShortcutsKeyDown` |
|  | `tbCompile` | tool button | Compile |  | tip: “Save and Compile”; `Action=actCompile` |
|  | `tbNew` | tool button | New |  | tip: “Create a new file”; `Action=actNew` |
|  | `tbOpen` | tool button | Open |  | tip: “Open an existing file”; `Action=actOpen` |
|  | `tbClose` | tool button | Close |  | tip: “Close the current file”; `Action=actClose` |
|  | `tbSave` | tool button | Save |  | tip: “Save the current file”; `Action=actSave` |
|  | `tbSaveAs` | tool button | Save As |  | tip: “Save As”; `Action=actSaveAs` |
|  | `tbSaveAll` | tool button | Save All |  | tip: “Save All”; `Action=actSaveAll` |
|  | `tbToggleBookmark` | tool button | Toggle Bookmark |  | tip: “Toggle bookmark”; `Action=actToggleBookmark` |
|  | `tbFind` | tool button | Find |  | tip: “Find Text”; `Action=actFind` |
|  | `tbFindInFiles` | tool button | Find In Files |  | tip: “Find In Files”; `Action=actFindInFiles` |
|  | `tbFindAgain` | tool button | Search Again |  | tip: “Find Next”; `Action=actSearchAgain` |
|  | `tbReplace` | tool button | Replace |  | tip: “Replace Text”; `Action=actReplace` |
|  | `tbOptions` | tool button | Options |  | tip: “Options”; `Action=actOptions` |
|  | `bOK` | button | Exit (DFM: 'Done') | close | tip: “Exit and close all open files”; `Action=actExit` |
|  | `dlgPrint` | PrintDialog |  | print |  |

**Action list `alActions`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actCompile` | Compile | F7 | `actCompileExecute` |  | Save and Compile |
| `actUndo` | Undo |  | `actUndoExecute` |  |  |
| `actRedo` | Redo |  | `actRedoExecute` |  |  |
| `actFind` | Find | Ctrl+F | `actFindExecute` |  | Find Text |
| `actSearchAgain` | Search Again | F3 | `actSearchAgainExecute` |  | Find Next |
| `actFindInFiles` | Find In Files |  | `actFindInFilesExecute` |  | Find In Files |
| `actReplace` | Replace | Ctrl+R | `actReplaceExecute` |  | Replace Text |
| `actToggleBookmark` | Toggle Bookmark | F5 | `actToggleBookmarkExecute` |  | Toggle bookmark |
| `actDeleteBookmark` | Delete Bookmark |  | `actDeleteBookmarkExecute` |  |  |
| `actShortcutsReparse` | Update Shortcut List | F9 | `actShortcutsReparseExecute` |  |  |
| `actSave` | Save | Ctrl+S | `actSaveExecute` |  | Save the current file |
| `actSaveAs` | Save As | Ctrl+Alt+S | `actSaveAsExecute` |  | Save As |
| `actSaveAll` | Save All |  | `actSaveAllExecute` | disabled | Save All |
| `actCut` | Cut |  | `actCutExecute` |  |  |
| `actCopy` | Copy |  | `actCopyExecute` |  |  |
| `actPaste` | Paste |  | `actPasteExecute` |  |  |
| `actEditDelete` | Delete |  | `actEditDeleteExecute` |  |  |
| `actEditSelectAll` | Select All |  | `actEditSelectAllExecute` |  |  |
| `actOpen` | Open | Ctrl+O | `actOpenExecute` |  | Open an existing file |
| `actClose` | Close | Ctrl+F4 | `actCloseExecute` |  | Close the script |
| `actNew` | New | Ctrl+N | `actNewExecute` |  | Create a new script |
| `actPrint` | Print |  | `actPrintExecute` |  | Print |
| `actFilter` | Filter |  | `actFilterExecute` |  |  |
| `actUpdateResources` | Rescan Resources |  | `actUpdateResourcesExecute` |  | Rescan the resource directories |
| `actCancel` | Cancel |  | `actCancelExecute` |  | Quit without saving |
| `actExit` | Exit |  | `actExitExecute` |  | Exit and close all open files |
| `actOptions` | Options |  | `actOptionsExecute` |  | Options |

**Popup menu `pmBookmarks`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miDeleteBookmark` | Delete Bookmark |  | `actDeleteBookmark` |  |

**Popup menu `pmEdit`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `Undo1` | Undo |  | `actUndo` |  |
| — | ——— | | | |
| `Cut1` | Cut |  | `actCut` |  |
| `Copy1` | Copy |  | `actCopy` |  |
| `Paste1` | Paste |  | `actPaste` |  |
| `Delete1` | Delete |  | `actEditDelete` |  |
| — | ——— | | | |
| `SelectAll1` | Select All |  | `actEditSelectAll` |  |

**Notes / behaviors:**
- Editor component is a custom syntax-highlighting memo (`TSEdit…`, completion list `TSEditCodeCompletionList`); tabs in `pcEditor`; `sHorizontalSplitter` between editor and info pane.
- Bottom info tabs are panels filled at runtime: Compiler (errors, double-click jumps to line), Help (doc comment of selected function), Bookmarks, Search Results.
- Side lists: double-click inserts identifier/template; hover shows prototype (`ShortcutListMouseMove`).
- Keyboard (from nwn.wiki): F2 completion, Ctrl+Shift+1…9 set / Ctrl+1…9 jump bookmark, Tab/Shift+Tab indent, Ctrl+Y redo, Ctrl+A select all.

#### `TdlgScriptSearch` — Find Text
*DFM `TDLGSCRIPTSEARCH` · 392×220 · border bsDialog · StrRef table .data+0xE6A40 (11/17 captions matched) · form events: OnShow=FormShow · DFM caption 'Search'*

**Purpose:** Script Find / Replace / Find in Files.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Scope | `xbBackwards` | checkbox | Search Backward (DFM: 'Backward') | direction |  |
| Scope | `xbSelectedOnly` | checkbox | Selected Only | (hidden) | **hidden** |
| Replace | `xbPrompt` | checkbox | Prompt On Replace | confirm each replace | default on |
| Replace | `xbReplaceAll` | checkbox | Replace All | replace all |  |
| Find In Files | `rbFindInOpenFiles` | radio | Find in Currently Open Files (DFM: 'Currently Open Scripts') | find-in-files scope | default on |
| Find In Files | `rbFindInAllFiles` | radio | Find in All Files in Module (DFM: 'All Scripts in Module') | find-in-files scope (module) |  |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |
|  | `bGo` | button | OK |  | `OnClick=bGoClick` |
|  | `cbFind` | combo | Find What | find text (history) | editable |
|  | `cbReplace` | combo | Replace With | replace text (history) | editable |
| Search Conditions | `xbMatchCase` | checkbox | Match Case | option |  |
| Search Conditions | `xbWholeWord` | checkbox | Match Whole Word Only | option |  |
| Search Conditions | `xbFindInFiles` | checkbox | Find In Files | switches to find-in-files panel | `OnClick=xbFindInFilesClick` |

#### `TfraScriptEditorColor` — (no caption)
*DFM `TFRASCRIPTEDITORCOLOR` · StrRef table .data+0xE6564 (2/3 captions matched)*

**Purpose:** Syntax colour options (embedded in Options › Script Editor).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `pScriptColorPreview` | color swatch/panel | Preview: | preview |  |
|  | `lbColors` | listbox | Text Type | element: text/selected/margin/comment/directive/identifier/keyword/number/string/error (ini Color*) | `OnClick=lbColorsClick` |
|  | `pColor` | color swatch/panel | Color: | current colour | tip: “Click to change the color of the selected text type.” |
|  | `bChangeColor` | button | Select Color (DFM: 'Change Color...') | colour dialog | disabled; tip: “Click to change the color of the selected text type.”; `OnClick=bChangeColorClick` |
|  | `dlgColor` | ColorDialog |  |  |  |

#### `TSEditCodeCompletionList` — (no caption)
*DFM `TSEDITCODECOMPLETIONLIST` · 390×162 · border bsNone · no StrRef table resolved · form events: OnDeactivate=FormDeactivate*

**Purpose:** Code-completion popup of the script editor (virtual list).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `lbCompletionCandidates` | listbox |  | matching identifiers | `OnClick=lbCompletionCandidatesClick OnKeyDown=lbCompletionCandidatesKeyDown OnMouseDown=lbCompletionCandidatesMouseDown` |

#### `TdlgScriptWizard` — Script Wizard
*DFM `TDLGSCRIPTWIZARD` · 509×389 · border bsDialog · StrRef table .data+0xE713C (106/117 captions matched) · DFM caption 'dlgScriptWizard'*

**Purpose:** Script Wizard: generates conversation condition or action scripts from checklists.

**Tabs** (`pcSteps`): tsType · tsCondOptions · tsCondAbilities · tsCondClass · tsCondFeats · tsCondGender · tsCondItem · tsCondLocal · tsCondRace · tsCondSkill · tsCondSkillCheck · tsActionReward · tsActionLocal · tsDone · tsActionOptions · tsActionActions · tsCondRandom · tsActionTake · tsCondAlignment

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | Help |  | **hidden** |
|  | `bFinish` | button | Finish |  | tip: “Finish the Wizard”; `Action=actFinish` |
|  | `bNext` | button | Next > |  | tip: “Continue to the next step in the Wizard”; `Action=actNextStep` |
|  | `bBack` | button | < Back |  | tip: “Return to the previous step in the Wizard”; `Action=actPrevStep` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
| tsType | `rbConditionScript` | radio | Conditional script (DFM: 'Condition Script') | generate `int StartingConditional()` | `OnClick=actEnableNextButtonExecute` |
| tsType | `rbActionScript` | radio | Action Script | generate `void main()` | `OnClick=actEnableNextButtonExecute` |
| tsCondOptions | `xbCondOptions_Ability` | checkbox | Abilities | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Feats` | checkbox | Feats | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Skills` | checkbox | Skills | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Gender` | checkbox | Gender | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Class` | checkbox | Class | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Race` | checkbox | Race | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_LocalVariable` | checkbox | Local Variable | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Item` | checkbox | Item In Inventory | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_SkillCheck` | checkbox | Skill Check | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Random` | checkbox | Random | script-generator input (emits NSS) |  |
| tsCondOptions | `xbCondOptions_Alignment` | checkbox | Alignment | script-generator input (emits NSS) |  |
| tsCondAbilities | `xbCondAbility_Str` | checkbox | Strength | script-generator input (emits NSS) | `OnClick=xbCondAbility_Click` |
| tsCondAbilities | `xbCondAbility_Dex` | checkbox | Dexterity | script-generator input (emits NSS) | `OnClick=xbCondAbility_Click` |
| tsCondAbilities | `xbCondAbility_Con` | checkbox | Constitution | script-generator input (emits NSS) | `OnClick=xbCondAbility_Click` |
| tsCondAbilities | `xbCondAbility_Int` | checkbox | Intelligence | script-generator input (emits NSS) | `OnClick=xbCondAbility_Click` |
| tsCondAbilities | `xbCondAbility_Wis` | checkbox | Wisdom | script-generator input (emits NSS) | `OnClick=xbCondAbility_Click` |
| tsCondAbilities | `xbCondAbility_Cha` | checkbox | Charisma | script-generator input (emits NSS) | `OnClick=xbCondAbility_Click` |
| tsCondAbilities | `cbCondAbility_Str_Operator` | combo | Strength operator (> < =) | script-generator input (emits NSS) | disabled; list; items: > / < / = |
| tsCondAbilities | `eCondAbility_Str_Value` | edit | Strength value | script-generator input (emits NSS) | disabled; max 2; spin 0…99; `OnExit=eCondAbilityIntValueExit` |
| tsCondAbilities | `cbCondAbility_Dex_Operator` | combo | Dexterity operator (> < =) | script-generator input (emits NSS) | disabled; list; items: > / < / = |
| tsCondAbilities | `eCondAbility_Dex_Value` | edit | Dexterity value | script-generator input (emits NSS) | disabled; max 2; spin 0…99; `OnExit=eCondAbilityIntValueExit` |
| tsCondAbilities | `cbCondAbility_Con_Operator` | combo | Constitution operator (> < =) | script-generator input (emits NSS) | disabled; list; items: > / < / = |
| tsCondAbilities | `eCondAbility_Con_Value` | edit | Constitution value | script-generator input (emits NSS) | disabled; max 2; spin 0…99; `OnExit=eCondAbilityIntValueExit` |
| tsCondAbilities | `cbCondAbility_Int_Operator` | combo | Intelligence operator (> < =) | script-generator input (emits NSS) | disabled; list; items: > / < / = |
| tsCondAbilities | `eCondAbility_Int_Value` | edit | Intelligence value | script-generator input (emits NSS) | disabled; max 2; spin 0…99; `OnExit=eCondAbilityIntValueExit` |
| tsCondAbilities | `cbCondAbility_Wis_Operator` | combo | Wisdom operator (> < =) | script-generator input (emits NSS) | disabled; list; items: > / < / = |
| tsCondAbilities | `eCondAbility_Wis_Value` | edit | Wisdom value | script-generator input (emits NSS) | disabled; max 2; spin 0…99; `OnExit=eCondAbilityIntValueExit` |
| tsCondAbilities | `cbCondAbility_Cha_Operator` | combo | Charisma operator (> < =) | script-generator input (emits NSS) | disabled; list; items: > / < / = |
| tsCondAbilities | `eCondAbility_Cha_Value` | edit | Charisma value | script-generator input (emits NSS) | disabled; max 2; spin 0…99; `OnExit=eCondAbilityIntValueExit` |
| tsCondClass | `cbCondClass_Classes` | combo | Class | script-generator input (emits NSS) | list |
| tsCondClass | `bCondClass_Add` | button | Add |  | `OnClick=bCondClass_AddClick` |
| tsCondClass | `sgCondClass_All` | grid | Class restrictions (class, level) | script-generator input (emits NSS) |  |
| tsCondClass | `eCondClass_Level` | edit | Specific level | script-generator input (emits NSS) | disabled; spin 1…20; `OnChange=eNumberOnlyChange` |
| tsCondClass | `rbCondClass_Any` | radio | Any Level | script-generator input (emits NSS) | default on; `OnClick=rbCondClass_LevelClick` |
| tsCondClass | `rbCondClass_Specific` | radio | Specific Level | script-generator input (emits NSS) | `OnClick=rbCondClass_LevelClick` |
| tsCondClass | `bCondClass_Remove` | button | Remove |  | `OnClick=bCondClass_RemoveClick` |
| tsCondClass | `rbCondClass_AllowAny` | radio | The player needs to meet only one of the restrictions (DFM: 'Allow Any Of These Restict… | script-generator input (emits NSS) | disabled; default on |
| tsCondClass | `rbCondClass_AllowOnly` | radio | The player needs to meet all of the restrictions (DFM: 'Allow Only These Restictions') | script-generator input (emits NSS) | disabled; `OnClick=rbCondClass_AllowOnlyClick` |
| tsCondFeats | `lbCondFeats_All` | listbox | All Feats | feat.2da | multi-select; `OnDblClick=bCond_ToRequiredClick` |
| tsCondFeats | `lbCondFeats_Required` | listbox | Required Feats | required feats → GetHasFeat | multi-select; `OnDblClick=bCond_ToAllClick` |
| tsCondFeats | `bCondFeats_ToAll` | button | <-- |  | `OnClick=bCond_ToAllClick` |
| tsCondFeats | `bCondFeats_ToRequired` | button | --> |  | `OnClick=bCond_ToRequiredClick` |
| tsCondGender | `lbCondGender` | listbox | Select required genders | gender.2da | multi-select |
| tsCondItem | `lbCondItem` | listbox | Currently required item tags | item tags → GetItemPossessedBy | multi-select; `OnClick=lbClick` |
| tsCondItem | `eCondItem_Tag` | edit | Enter a new tag | item tag | max 32; `OnChange=eCondItem_TagChange` |
| tsCondItem | `bCondItem_Add` | button | Add |  | disabled; `OnClick=bListBox_AddClick` |
| tsCondItem | `bCondItem_Remove` | button | Remove |  | disabled; `OnClick=bRemoveFromListBoxClick` |
| tsCondLocal | `lbCondLocal` | listbox | Local Expressions | script-generator input (emits NSS) | multi-select; `OnClick=lbClick` |
| tsCondLocal | `eCondLocal_LHand` | edit | Variable name | script-generator input (emits NSS) | max 32; `OnChange=eCondLocal_Change` |
| tsCondLocal | `bCondLocal_Add` | button | Add |  | disabled; `OnClick=bCondLocal_AddClick` |
| tsCondLocal | `bCondLocal_Remove` | button | Remove |  | disabled; `OnClick=bRemoveFromListBoxClick` |
| tsCondLocal | `eCondLocal_RHand` | edit | Value | script-generator input (emits NSS) | `OnChange=eCondLocal_Change` |
| tsCondLocal | `cbCondLocal_Operator` | combo | Operator | script-generator input (emits NSS) | list |
| tsCondLocal | `cbCondLocal_LHandType` | combo | Variable type (int/float/string) | script-generator input (emits NSS) | list; `OnChange=cbCondLocal_LHandTypeChange` |
| tsCondLocal | `cbCondLocal_RHandType` | combo | Value type | script-generator input (emits NSS) | list |
| tsCondRace | `bCondRace_Player_ToAccept` | button | --> |  | `OnClick=bCond_ToAcceptClick` |
| tsCondRace | `bCondRace_Player_ToReject` | button | <-- |  | `OnClick=bCond_ToRejectClick` |
| tsCondRace › Accepted | `lbCondRace_Player_Accept` | listbox | Player | racialtypes.2da (player races) | multi-select; `OnDblClick=bCond_ToRejectClick` |
| tsCondRace › Accepted | `lbCondRace_Other_Accept` | listbox | Other | racialtypes.2da (other) | multi-select; `OnDblClick=bCond_ToRejectClick` |
| tsCondRace › Rejected | `lbCondRace_Player_Reject` | listbox | Player | rejected | multi-select; `OnDblClick=bCond_ToAcceptClick` |
| tsCondRace › Rejected | `lbCondRace_Other_Reject` | listbox | Other | rejected | multi-select; `OnDblClick=bCond_ToAcceptClick` |
| tsCondRace | `bCondRace_Other_ToAccept` | button | --> |  | `OnClick=bCond_ToAcceptClick` |
| tsCondRace | `bCondRace_Other_ToReject` | button | <-- |  | `OnClick=bCond_ToRejectClick` |
| tsCondSkill | `lbCondSkills_All` | listbox | All Skills | skills.2da | multi-select; `OnDblClick=bCond_ToRequiredClick` |
| tsCondSkill | `lbCondSkills_Required` | listbox | Required Skills | required skills → GetHasSkill | multi-select; `OnDblClick=bCond_ToAllClick` |
| tsCondSkill | `bCondSkills_ToAll` | button | <-- |  | `OnClick=bCond_ToAllClick` |
| tsCondSkill | `bCondSkills_ToRequired` | button | --> |  | `OnClick=bCond_ToRequiredClick` |
| tsCondSkillCheck | `lbCondSkillcheck_All` | listbox | Available skills | skills.2da | `OnClick=lbClick` |
| tsCondSkillCheck › Difficulty | `rbCondSkillCheck_Easy` | radio | Easy | script-generator input (emits NSS) | default on |
| tsCondSkillCheck › Difficulty | `rbCondSkillCheck_Medium` | radio | Normal (DFM: 'Medium') | script-generator input (emits NSS) |  |
| tsCondSkillCheck › Difficulty | `rbCondSkillCheck_Hard` | radio | Hard | script-generator input (emits NSS) |  |
| tsCondSkillCheck | `bCondSkillCheck_Add` | button | Add |  | `OnClick=bCondSkillCheck_AddClick` |
| tsCondSkillCheck | `lbCondSkillCheck_Checks` | listbox | Checks | skill checks (skill, DC band) | multi-select; `OnClick=lbClick` |
| tsCondSkillCheck | `bCondSkillCheck_Remove` | button | Remove |  | `OnClick=bRemoveFromListBoxClick` |
| tsActionReward | `eActionRewards_Gold` | edit | Give gold | GiveGoldToCreature | max 7; `OnChange=eNumberOnlyChange` |
| tsActionReward | `eActionRewards_XP` | edit | Give XP | GiveXPToCreature | max 7; `OnChange=eNumberOnlyChange` |
| tsActionReward | `eActionRewards_Item` | edit | Give item (by ResRef) | CreateItemOnObject resref | max 16; `OnChange=eActionRewards_ItemChange` |
| tsActionReward | `lbActionRewards` | listbox | Rewards list | script-generator input (emits NSS) | multi-select; `OnClick=lbClick` |
| tsActionReward | `bActionRewards_Add` | button | Add |  | disabled; `OnClick=bActionRewards_AddClick` |
| tsActionReward | `bActionRewards_Remove` | button | Remove |  | disabled; `OnClick=bRemoveFromListBoxClick` |
| tsActionReward | `xbActionRewards_GoldParty` | checkbox | To Party | script-generator input (emits NSS) |  |
| tsActionReward | `xbActionRewards_XPParty` | checkbox | To Party | script-generator input (emits NSS) |  |
| tsActionReward | `bActionRewards_Items` | button | ... |  | `OnClick=bAction_ItemsClick` |
| tsActionLocal | `cbActionLocal_LHandType` | combo | Set local variable (type) | script-generator input (emits NSS) | list; `OnChange=cbActionLocal_LHandTypeChange` |
| tsActionLocal | `eActionLocal_Name` | edit | Variable name | script-generator input (emits NSS) | `OnChange=eActionLocalChange` |
| tsActionLocal | `eActionLocal_Value` | edit | Value | script-generator input (emits NSS) | `OnChange=eActionLocalChange` |
| tsActionLocal | `lbActionLocal` | listbox | Local Expressions | script-generator input (emits NSS) | multi-select; `OnClick=lbClick` |
| tsActionLocal | `bActionLocal_Remove` | button | Remove |  | disabled; `OnClick=bRemoveFromListBoxClick` |
| tsActionLocal | `bActionLocal_Add` | button | Add |  | disabled; `OnClick=bActionLocal_AddClick` |
| tsActionLocal | `cbActionLocal_RHandType` | combo | To the value | script-generator input (emits NSS) | list |
| tsDone | `eDone_ScriptName` | edit | Enter the script name | output .nss resref | max 16; `OnChange=eDone_ScriptNameChange` |
| tsDone | `xbDone_StartEditor` | checkbox | Start the script editor | open in Script Editor |  |
| tsActionOptions | `xbActionOptions_Rewards` | checkbox | Give rewards | script-generator input (emits NSS) |  |
| tsActionOptions | `xbActionOptions_Variables` | checkbox | Set local variables | script-generator input (emits NSS) |  |
| tsActionOptions | `xbActionOptions_Actions` | checkbox | Perform an action | script-generator input (emits NSS) |  |
| tsActionOptions | `xbActionOptions_Take` | checkbox | Take from the player | script-generator input (emits NSS) |  |
| tsActionActions | `eActionActions_ModFaction` | edit | Modify Faction | script-generator input (emits NSS) | read-only; max 4 |
| tsActionActions | `eActionActions_StoreTag` | edit | Script Tag | store tag → OpenStore | disabled |
| tsActionActions | `rbActionActions_OpenStore` | radio | Start a Merchant (DFM: 'Open A Store') | script-generator input (emits NSS) | `OnClick=actActionActionButtonPressExecute` |
| tsActionActions | `rbActionActions_Attack` | radio | Attack | script-generator input (emits NSS) | `OnClick=actActionActionButtonPressExecute` |
| tsActionActions | `rbActionActions_NoAction` | radio | No Action | script-generator input (emits NSS) | default on; `OnClick=actActionActionButtonPressExecute` |
| tsActionActions | `xbActionActions_UseAppraise` | checkbox | Use appraise checks (DFM: 'Use appraise check') | script-generator input (emits NSS) | disabled; default on |
| tsActionActions | `tbActionActions_ModFaction` | slider | Modify Faction | AdjustReputation amount | range -10…10; `OnChange=tbActionActions_ModFactionChange` |
| tsCondRandom | `eCondRandom_1` | edit | Has the chance of appearing of | Random numerator | spin 1…1000; `OnChange=eCondRandom_1Change` |
| tsCondRandom | `eCondRandom_2` | edit | in | Random denominator | spin 1…1000; `OnChange=eCondRandom_2Change` |
| tsActionTake | `eActionTake_Gold` | edit | Take gold | TakeGoldFromCreature | max 7; `OnChange=eNumberOnlyChange` |
| tsActionTake | `eActionTake_XP` | edit | Take XP | SetXP (reduce) | max 7; `OnChange=eNumberOnlyChange` |
| tsActionTake | `eActionTake_Item` | edit | Take item (by Tag) | item tag to take | max 32; `OnChange=eActionTake_ItemChange` |
| tsActionTake | `lbActionTake_Item` | listbox | Items to take | script-generator input (emits NSS) | multi-select; `OnClick=lbClick` |
| tsActionTake | `bActionTake_Add` | button | Add |  | disabled; `OnClick=bListBox_AddClick` |
| tsActionTake | `bActionTake_Remove` | button | Remove |  | disabled; `OnClick=bRemoveFromListBoxClick` |
| tsActionTake | `rbActionTake_ItemDestroy` | radio | Destroy | script-generator input (emits NSS) | default on |
| tsActionTake | `rbActionTake_ItemKeep` | radio | Keep | script-generator input (emits NSS) |  |
| tsActionTake | `rbActionTake_GoldKeep` | radio | Keep | script-generator input (emits NSS) |  |
| tsActionTake | `rbActionTake_GoldDestroy` | radio | Destroy | script-generator input (emits NSS) | default on |
| tsCondAlignment | `xbCondAlignment_Good` | checkbox | Good | script-generator input (emits NSS) |  |
| tsCondAlignment | `xbCondAlignment_Neutral` | checkbox | Neutral | script-generator input (emits NSS) |  |
| tsCondAlignment | `xbCondAlignment_Evil` | checkbox | Evil | script-generator input (emits NSS) |  |
| tsCondAlignment | `xbCondAlignment_Lawful` | checkbox | Lawful | script-generator input (emits NSS) |  |
| tsCondAlignment | `xbCondAlignment_Neutral2` | checkbox | Neutral | script-generator input (emits NSS) |  |
| tsCondAlignment | `xbCondAlignment_Chaos` | checkbox | Chaotic | script-generator input (emits NSS) |  |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actEnableNextButton` | actEnableNextButton |  | `actEnableNextButtonExecute` |  |  |
| `actNextStep` | Next |  | `actNextStepExecute` |  | Continue to the next step in the Wizard |
| `actPrevStep` | Back |  | `actPrevStepExecute` | disabled | Return to the previous step in the Wizard |
| `actFinish` | Finish |  | `actFinishExecute` | disabled | Finish the Wizard |
| `actActionActionButtonPress` | actActionActionButtonPress |  | `actActionActionButtonPressExecute` |  |  |

**Notes / behaviors:**
- Condition steps: abilities (op + value), class/level (any/specific, allow any/only), feats, gender, items in inventory (tags), local variables (lhs type/name op rhs type/value), race (player/other accept/reject), skills, skill checks (easy/medium/hard), random (x in y), alignment. Action steps: rewards (gold/XP/items, to party), set locals, actions (open store by tag with appraise, attack, adjust faction −100…100), take (gold/XP/items; destroy/keep). Done: script name + open editor.

## 17. Journal editor

#### `TdlgJournalEditor` — Journal Editor
*DFM `TDLGJOURNALEDITOR` · StrRef table .data+0x472A8 (16/20 captions matched) · DFM caption 'dlgJournalEditor'*

**Purpose:** Journal Editor (module.jrl).

**Tabs** (`pcMain`): Category (node selected) · Entry (node selected)

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `tvMain` | tree |  | JRL `Categories[]` / `EntryList[]` | `OnChange=tvMainChange OnKeyDown=tvMainKeyDown OnMouseDown=tvMainMouseDown` |
|  | `bAdd` | tool button | Add |  | tip: “Add”; `Action=actAdd` |
|  | `tbCopy` | tool button | Copy |  | tip: “Copy”; `Action=actCopy` |
|  | `tbCut` | tool button | Cut |  | tip: “Cut”; `Action=actCut` |
|  | `tbPaste` | tool button | Paste |  | tip: “Paste”; `Action=actPaste` |
|  | `bApply` | button | Apply | save without closing | `Action=actApply` |
|  | `bOk` | button | OK |  | tip: “Accept changes”; `OnClick=actApplyExecute` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
| Category (node selected) | `mCatComments` | memo | Comments | JRL category `Comment` | `OnChange=actSaveFromControlsExecute` |
| Category (node selected) › Category | `cbPriority` | combo | Priority | JRL `Priority` (0 highest…4 lowest) | list; `OnChange=actSaveFromControlsExecute OnClick=actSetModifiedExecute` |
| Category (node selected) › Category | `eTag` | edit | Tag | JRL `Tag` | max 32; tip: “Edit the Tag”; `OnChange=actSaveFromControlsExecute OnExit=eExit` |
| Category (node selected) › Category | `bCatEdit` | button | ... (Name :) | `Name` all languages | tip: “Edit text in multiple languages” |
| Category (node selected) › Category | `eCatName` | edit | Name : | JRL `Name` |  |
| Category (node selected) › Category | `eXP` | edit | XP | JRL `XP` | `OnChange=actSaveFromControlsExecute OnExit=eExit` |
| Entry (node selected) › Entry | `eEntryID` | edit | ID | JRL entry `ID` | `OnChange=actSaveFromControlsExecute OnExit=eExit` |
| Entry (node selected) › Entry | `xbEntryFinish` | checkbox | Finish Category | JRL entry `End` | `OnClick=xbEntryFinishClick` |
| Entry (node selected) › Entry | `mEntryText` | memo | Text | JRL entry `Text` |  |
| Entry (node selected) › Entry | `bEntryTextEdit` | button | ... | `Text` all languages | tip: “Edit text in multiple languages” |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actApply` | Apply |  | `actApplyExecute` |  |  |
| `actAdd` | Add |  | `actAddExecute` |  | Add |
| `actCopy` | Copy |  | `actCopyExecute` |  | Copy |
| `actCut` | Cut |  | `actCutExecute` |  | Cut |
| `actPaste` | Paste |  | `actPasteExecute` |  | Paste |
| `actSaveFromControls` |  |  | `actSaveFromControlsExecute` |  |  |
| `actDelete` | Delete |  | `actDeleteExecute` |  |  |
| `actSetModified` |  |  | `actSetModifiedExecute` |  |  |

**Popup menu `pmMain`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miAdd` | Add |  | `actAdd` |  |
| `miCopy` | Copy |  | `actCopy` |  |
| `miCut` | Cut |  | `actCut` |  |
| `miPaste` | Paste |  | `actPaste` |  |

**Notes / behaviors:**
- Tree of categories → entries; tab sheet shown per selected node type (category vs entry). Edits save back to the tree on change (`actSaveFromControls`). Priority list: Highest/High/Medium/Low/Lowest.

## 18. Faction editor

#### `TdlgFactionEditor` — Faction Editor
*DFM `TDLGFACTIONEDITOR` · StrRef table .data+0x32374 (14/15 captions matched) · form events: OnShow=FormShow*

**Purpose:** Faction Editor (repute.fac).

**Tabs** (`pcMain`): Basic · Advanced

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Basic | `xbShowInverse` | checkbox | Full Detail | chart detail mode | `OnClick=xbShowInverseClick` |
| Basic | `bBasicAdd` | button | Add Faction |  | `Action=actAddFaction` |
| Basic | `bBasicRemove` | button | Remove Faction |  | `Action=actRemoveFaction` |
| Basic | `lbBasic` | check-listbox | Factions | FAC `FactionList` (check = show in chart) | `OnClickCheck=lbBasicClickCheck OnClick=lbBasicClick` |
| Basic › Properties | `xbBasicGlobal` | checkbox | Global Effect | FAC `FactionGlobal` | disabled; `OnClick=xbBasicGlobalClick` |
| Basic | `OpenGLPanel1` | GL view | ⟨Basic⟩ | reputation chart (drag to edit) | `OnMouseDown=OpenGLPanel1MouseDown` |
| Advanced | `sgAdvanced` | grid | ⟨Advanced⟩ | FAC `RepList` (`FactionID1`,`FactionID2`,`FactionRep` 0–100) | editable cells; `OnSetEditText=sgAdvancedSetEditText` |
| Advanced | `lbAdvanced` | listbox | Factions | factions shown in grid | multi-select; `OnClick=actRefreshAdvChartExecute` |
| Advanced | `bAdvAdd` | button | Add Faction |  | `Action=actAddFaction` |
| Advanced | `bAdvRemove` | button | Remove Faction |  | `Action=actRemoveFaction` |
| Advanced › Properties | `xbAdvGlobal` | checkbox | Global Effect | FAC `FactionGlobal` | `OnClick=xbAdvGlobalClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `bOk` | button | OK |  | tip: “Accept changes”; `OnClick=SaveButtonClick` |
|  | `MainStatusBar` | status bar |  | status |  |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actRefreshAdvChart` | Refresh |  | `actRefreshAdvChartExecute` |  |  |
| `actRefreshBasicChart` | Refresh |  |  |  |  |
| `actAddFaction` | Add Faction |  | `actAddFactionExecute` |  |  |
| `actRemoveFaction` | Remove Faction |  | `actRemoveFactionExecute` |  |  |
| `actDoNothing` | actDoNothing |  | `actDoNothingExecute` |  |  |
| `actUpdateStatusBar` | actUpdateStatusBar |  | `actUpdateStatusBarExecute` |  |  |
| `actChangeFactionName` | Change Faction Name (DFM 'Change Name') |  | `actChangeFactionNameExecute` |  |  |

**Popup menu `pmMain`** (OnPopup=pmMainPopup)

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miAddFaction` | Add Faction |  | `actAddFaction` |  |
| `RemoveFaction1` | Remove Faction |  | `actRemoveFaction` |  |
| `ChangeName1` | Change Faction Name |  | `actChangeFactionName` |  |

**Notes / behaviors:**
- Basic tab: check factions to plot their mutual reputations in the OpenGL chart (mouse handlers present). Advanced tab: editable N×N grid (row = faction feeling about column). The standard factions (PC, Hostile, Commoner, Merchant, Defender) cannot be removed.

#### `TdlgFactionSelect` — Select Faction
*DFM `TDLGFACTIONSELECT` · 238×256 · border bsDialog · StrRef table .data+0x34EC8 (7/7 captions matched)*

**Purpose:** Add Faction (name, global, parent).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | disabled; tip: “Accept changes” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
| Name | `eName` | edit | ⟨Name⟩ | FAC `FactionName` | `OnChange=actActivateOKExecute` |
| Properties | `xbGlobal` | checkbox | Global Effect | FAC `FactionGlobal` | default on |
| Parent | `lbParent` | listbox | ⟨Parent⟩ | FAC `FactionParentID` (copy reps from parent) | `OnClick=actActivateOKExecute` |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actActivateOK` | actActivateOK |  | `actActivateOKExecute` |  |  |

## 19. Plot wizard & plot manager

#### `TdlgPlotWizard` — Plot Wizard
*DFM `TDLGPLOTWIZARD` · 602×452 · border bsDialog · StrRef table .data+0xD5FFC (36/42 captions matched)*

**Purpose:** Plot Wizard: plot name/template → cast (giver, villains, extras) → props → plot nodes → summary; saves a plot manager file.

**Tabs** (`pcMain`): tsName · tsCast · tsVillain · tsExtras · tsProps · tsPlot · tsSummary

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bClose` | button | Close |  | `OnClick=bCloseClick` |
|  | `bSave` | button | Save | write .ptm + generated resources | tip: “Save the current file”; `OnClick=bSaveClick` |
|  | `bNext` | button | Next --> |  | tip: “Continue to the next step in the Wizard”; `OnClick=bNextClick` |
|  | `bBack` | button | <-- Back |  | disabled; tip: “Return to the previous step in the Wizard”; `OnClick=bBackClick` |
|  | `fraProgress` | embedded frame | `TfraProgress` |  | see frame section |
| tsName | `ePlotName` | edit | Name | plot name | `OnChange=ePlotNameChange` |
| tsName | `bPlotName` | button | ... |  | tip: “Edit text in multiple languages” |
| tsName | `bBasic_ApplyTemplate` | button | Apply |  | `Action=actApplyTemplate` |
| tsName | `lbBasic_Templates` | listbox | Plot Blueprints | plot templates (.ptt) | `OnClick=lbBasic_TemplatesClick OnDblClick=actApplyTemplateExecute` |
| tsName | `bBasic_ImportTemplate` | button | (no caption; runtime) |  | **hidden**; `Action=actImportTemplate` |
| tsName | `ePlot_JournalTag` | edit |  | (hidden) JRL `Tag` | **hidden** |
| tsName | `ePlot_JournalName` | edit | Journal Name | JRL category `Name` | `OnChange=ePlot_JournalNameChange` |
| tsName | `bPlot_JournalName` | button | ... |  | tip: “Edit text in multiple languages” |
| tsCast | `sgCast_PlotGiver` | grid |  | plot giver creature (Name/Tag/Blueprint) |  |
| tsCast | `bCast_PlotGiver_Edit` | button | Edit |  | `OnClick=bCast_EditClick` |
| tsCast | `bCast_PlotGiver_New` | button | New |  | `OnClick=bCast_NewClick` |
| tsCast | `bCast_PlotGiver_Browse` | button | Browse |  | `OnClick=bCast_BrowseClick` |
| tsCast | `bCast_PlotGiver_EditPlotlessConversation` | button | Edit non-plot-related Conversation |  | `OnClick=bEditPlotlessConversationClick` |
| tsVillain | `sgCast_Villain` | grid |  | villains | `OnKeyDown=sgCast_VillainKeyDown` |
| tsVillain | `bCast_Villain_Edit` | button | Edit |  | `OnClick=bCast_EditClick` |
| tsVillain | `bCast_Villain_Remove` | button | Delete |  | `OnClick=bCast_RemoveClick` |
| tsVillain | `bCast_Villain_New` | button | New |  | `OnClick=bCast_NewClick` |
| tsVillain | `bCast_Villain_Browse` | button | Browse |  | `OnClick=bCast_BrowseClick` |
| tsVillain | `bCast_Villain_EditPlotlessConversation` | button | Edit non-plot-related Conversation |  | `OnClick=bEditPlotlessConversationClick` |
| tsExtras | `sgCast_Extras` | grid |  | extras | `OnClick=sgCast_ExtrasClick OnDblClick=sgCast_ExtrasDblClick OnKeyDown=sgCast_ExtrasKeyDown` |
| tsExtras | `bCast_Extras_Edit` | button | Edit |  | `OnClick=bCast_EditClick` |
| tsExtras | `bCast_Extras_New` | button | New |  | `OnClick=bCast_NewClick` |
| tsExtras | `bCast_Extras_Remove` | button | Delete |  | `OnClick=bCast_RemoveClick` |
| tsExtras | `bCast_Extras_Browse` | button | Browse |  | `OnClick=bCast_BrowseClick` |
| tsExtras | `bCast_Extras_EditPlotlessConversation` | button | Edit non-plot-related Conversation |  | `OnClick=bEditPlotlessConversationClick` |
| tsProps | `sgProps` | grid | Props | items/placeables used | `OnClick=sgPropsClick OnDblClick=sgPropsDblClick OnKeyDown=sgPropsKeyDown` |
| tsProps | `bProps_Edit` | button | Edit |  | `OnClick=bCast_EditClick` |
| tsProps | `bProps_New` | button | New |  | `OnClick=bProps_NewClick` |
| tsProps | `bProps_Remove` | button | Delete (DFM: 'Del') |  | `OnClick=bCast_RemoveClick` |
| tsProps | `bProps_Browse` | button | Browse |  | `OnClick=bCast_BrowseClick` |
| tsPlot | `lbPlotNodes` | listbox | Plot Nodes | sub-plot nodes (ordered) | `OnClick=lbPlotNodesClick OnDblClick=lbPlotNodesDblClick OnKeyDown=lbPlotNodesKeyDown` |
| tsPlot | `bPlot_NodeNew` | button | New |  | `OnClick=bPlot_NodeNewClick` |
| tsPlot | `bPlot_NodeEdit` | button | Edit |  | `OnClick=bPlot_NodeEditClick` |
| tsPlot | `bPlot_NodeDelete` | button | Delete |  | `OnClick=bPlot_NodeDeleteClick` |
| tsPlot | `bPlotNodeMoveUp` | button | Move Up |  | `OnClick=bPlotNodeMoveUpClick` |
| tsPlot | `bPlotNodeMoveDown` | button | Move Down |  | `OnClick=bPlotNodeMoveDownClick` |
| tsSummary | `bSummary_SaveTemplate` | button | Save As Template |  | **hidden**; `OnClick=bSummary_SaveTemplateClick` |
| tsSummary | `mDescFinish` | memo | Finish | plot-generator input |  |
|  | `mHelp` | memo |  | context help | read-only |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actApplyTemplate` | Apply |  | `actApplyTemplateExecute` |  |  |
| `actImportTemplate` | Import |  | `actImportTemplateExecute` |  |  |

**Notes / behaviors:**
- Uses `TfraProgress` step strip; Next/Back navigate; `mHelp` shows context help per step; cast grids with New/Edit/Browse/Delete; "Edit non-plot-related Conversation".

#### `TdlgPlotNodeWizard` — Plot Node Wizard
*DFM `TDLGPLOTNODEWIZARD` · 692×509 · StrRef table .data+0xD2E64 (70/75 captions matched)*

**Purpose:** Plot Node Wizard (one sub-plot step).

**Inherits:** `TdlgWizard`.

**Tabs** (`pcSteps`): Basic · Plot Type · Conversation Type · Lock · Items · Conversation · Conversation Condition · tsConvOther · tsMisc · Journal

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bHelp` | button | (no caption; runtime) |  |  |
|  | `bFinish` | button | (no caption; runtime) |  |  |
|  | `bNext` | button | (no caption; runtime) |  |  |
|  | `bBack` | button | (no caption; runtime) |  |  |
|  | `bCancel` | button | (no caption; runtime) |  |  |
| Basic | `mComments` | memo | Comments | node comment | `OnChange=mCommentsChange` |
| Basic | `cbCastType` | combo | Type | cast member kind | list; `OnChange=cbCastTypeChange` |
| Basic | `cbDoors` | combo | Name | cast door | **hidden**; list; `OnChange=cbCastMemberChange` |
| Basic | `cbPlaceables` | combo | Name | cast placeable | **hidden**; list; `OnChange=cbCastMemberChange` |
| Basic | `cbCreatures` | combo | Name | cast creature | **hidden**; list; `OnChange=cbCastMemberChange` |
| Basic | `ePlotNodeName` | edit | Plot Node Name | node name | `OnChange=ePlotNodeNameChange` |
| Basic | `ePlotNodeTag` | edit | Plot Node Tag | plot-generator input (DLG/NSS/JRL) | **hidden**; read-only; max 32 |
| Basic | `bPlotNodeName` | button | ... |  | tip: “Edit text in multiple languages” |
| Plot Type | `rbPlotTypeConversation` | radio | Conversation | plot-generator input (DLG/NSS/JRL) | `OnClick=rbPlotTypeClick OnDblClick=rbPlotTypeDoubleClick` |
| Plot Type | `rbPlotTypeVillain` | radio | Conflict with Villain | plot-generator input (DLG/NSS/JRL) | `OnClick=rbPlotTypeClick OnDblClick=rbPlotTypeDoubleClick` |
| Plot Type | `rbPlotTypeOpenObject` | radio | Open a Door or Container | plot-generator input (DLG/NSS/JRL) | `OnClick=rbPlotTypeClick OnDblClick=rbPlotTypeDoubleClick` |
| Plot Type | `xbVillainTalks` | checkbox | Villain talks to player before attacking. | plot-generator input (DLG/NSS/JRL) | `OnClick=xbVillainTalksClick` |
| Plot Type | `xbVillainSurrenders` | checkbox | Villain surrenders and talks to player when near death. | plot-generator input (DLG/NSS/JRL) | **hidden** |
| Plot Type | `xbContainer` | checkbox | The object contains an item that the player must obtain. | plot-generator input (DLG/NSS/JRL) | `OnClick=xbContainerClick` |
| Plot Type | `xbVillainHasInventory` | checkbox | Villain carries items that drop on death. | plot-generator input (DLG/NSS/JRL) | `OnClick=xbVillainHasInventoryClick` |
| Plot Type | `rbActivateObject` | radio | Activate an object | plot-generator input (DLG/NSS/JRL) | **hidden**; `OnClick=rbPlotTypeClick OnDblClick=rbPlotTypeDoubleClick` |
| Conversation Type | `rbConvTypeQuest` | radio | Quest/Informational | plot-generator input (DLG/NSS/JRL) | `OnClick=rbConvTypeClick OnDblClick=rbConvTypeDblClick` |
| Conversation Type | `rbConvTypeExchangeItems` | radio | Exchange Items | plot-generator input (DLG/NSS/JRL) | `OnClick=rbConvTypeClick OnDblClick=rbConvTypeDblClick` |
| Conversation Type | `rbConvTypeFreeForm` | radio | Free-form | plot-generator input (DLG/NSS/JRL) | **hidden**; `OnClick=rbConvTypeClick OnDblClick=rbConvTypeDblClick` |
| Conversation Type | `rbConvTypeSingle` | radio | Single Statement | plot-generator input (DLG/NSS/JRL) | `OnClick=rbConvTypeClick OnDblClick=rbConvTypeDblClick` |
| Lock | `fraLock` | embedded frame | `TfraSituatedLock` |  | see frame section |
| Lock | `cbItemKeyTags` | combo | ⟨Lock⟩ | key item tag | list |
| Lock | `cbPlaceableUnlocker` | combo | ⟨Lock⟩ | unlocking placeable | list; `OnChange=cbPlaceableUnlockerChange` |
| Lock | `xbUnlockerConversation` | checkbox | On using object, display descriptive text | plot-generator input (DLG/NSS/JRL) |  |
| Items | `cbTakeItemFromPlayer` | combo | This is the item taken from the player in conversation. | item taken | list; `OnChange=cbItemChange` |
| Items | `xbTakeItemFromPlayer` | checkbox | Take Item from Player | plot-generator input (DLG/NSS/JRL) | `OnClick=xbTakeItemFromPlayerClick` |
| Items | `eGoldToTakeFromPlayer` | edit | Take gold from player | gold taken | `OnChange=eGoldAmountEdit OnExit=eGoldAmountExit` |
| Items | `cbGiveItemToPlayer` | combo | This is the item given to the player in conversation. | item given | list; `OnChange=cbItemChange` |
| Items | `xbGiveItemToPlayer` | checkbox | Give Item to Player | plot-generator input (DLG/NSS/JRL) | `OnClick=xbGiveItemToPlayerClick` |
| Items | `eGoldToGiveToPlayer` | edit | Give gold to player. | gold given | `OnChange=eGoldAmountEdit OnExit=eGoldAmountExit` |
| Items | `xbItemToGivePickpocketable` | checkbox | Player can steal item. | plot-generator input (DLG/NSS/JRL) |  |
| Items | `cbLootItem` | combo | This is the item that the player must take from the villain or container. | loot item | list; `OnChange=cbItemChange` |
| Items | `xbItemToLootPickpocketable` | checkbox | Player can steal item. | plot-generator input (DLG/NSS/JRL) |  |
| Conversation | `mGreeting` | memo | Plot Node Name | generated DLG text | `OnChange=eConversationNodeChange` |
| Conversation | `bGreeting` | button | ... |  | tip: “Edit text in multiple languages” |
| Conversation | `eAccept` | edit | ... | generated DLG text | `OnChange=eConversationNodeChange` |
| Conversation | `eReject` | edit | Cast member to interact with | generated DLG text | `OnChange=eConversationNodeChange` |
| Conversation | `mAction` | memo | Plot Node Name | generated DLG text | `OnChange=eConversationNodeChange` |
| Conversation | `bAccept` | button | ... |  | tip: “Edit text in multiple languages” |
| Conversation | `bAction` | button | ... |  | tip: “Edit text in multiple languages” |
| Conversation | `bReject` | button | ... |  | tip: “Edit text in multiple languages” |
| Conversation | `bConvActionScript` | button | (no caption; glyph) |  | tip: “Edit Custom Script”; `OnClick=bConvActionScriptClick` |
| Conversation | `bConvGreetingScript` | button | (no caption; glyph) |  | tip: “Edit Custom Script”; `OnClick=bConvGreetingScriptClick` |
| Conversation Condition | `cbCompletionPlotNode` | combo | Prerequisite Plot Node: | prerequisite node | list; `OnChange=cbCompletionPlotNodeChange` |
| Conversation Condition | `rbPrerequisiteNone` | radio | No Prerequisite | plot-generator input (DLG/NSS/JRL) | `OnClick=rbPrerequisiteClick OnDblClick=rbPrerequisiteDblClick` |
| Conversation Condition | `rbPrerequisiteIsPlotState` | radio | Simple Prerequisite | plot-generator input (DLG/NSS/JRL) | `OnClick=rbPrerequisiteClick OnDblClick=rbPrerequisiteDblClick` |
| Conversation Condition | `rbPrerequisiteIsConversation` | radio | Cast-Specific Prerequisite | plot-generator input (DLG/NSS/JRL) | `OnClick=rbPrerequisiteClick OnDblClick=rbPrerequisiteDblClick` |
| tsConvOther | `mUndefinedGreeting` | memo | The speaker greets a player who is involved in the plot, but whose current status in th… | plot-generator input (DLG/NSS/JRL) | `OnChange=mConvOtherChange` |
| tsConvOther | `mPlotlessGreeting` | memo | The speaker greets a player who is not involved at all in this plot. | plot-generator input (DLG/NSS/JRL) | `OnChange=mConvOtherChange` |
| tsConvOther | `mDescConvOther` | memo | Other Conversation | plot-generator input (DLG/NSS/JRL) |  |
| tsConvOther | `bPlotlessGreeting` | button | ... |  | tip: “Edit text in multiple languages” |
| tsConvOther | `bUndefinedGreeting` | button | ... |  | tip: “Edit text in multiple languages” |
| tsMisc | `xbHostile` | checkbox | Hostile | plot-generator input (DLG/NSS/JRL) |  |
| tsMisc | `xbInvulnerable` | checkbox | Invulnerable | plot-generator input (DLG/NSS/JRL) |  |
| Journal | `mJournalEntry` | memo | ⟨Journal⟩ | JRL entry `Text` |  |
| Journal | `xbJournal` | checkbox | Create Journal Entry on completion | create JRL entry | `OnClick=xbJournalClick` |
| Journal | `bEditJournalEntry` | button | ... |  | tip: “Edit text in multiple languages” |
| Journal | `eExperience` | edit | Enter the amount of experience that the player should get for completing this subplot. | XP reward | `OnChange=eGoldAmountEdit OnExit=eExperienceExit` |
| Journal | `xbEnd` | checkbox | This journal entry appears in the Completed Quests list. | JRL entry `End` | `OnClick=xbJournalClick` |
|  | `fraProgress` | embedded frame | `TfraProgress` |  | see frame section |
|  | `bConvOtherOK` | button | OK |  | **hidden**; `OnClick=bConvOtherOKClick` |

**Popup menu `pmConversation`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miInsertToken` | Insert Token... |  |  `miInsertTokenClick` |  |

**Notes / behaviors:**
- Pages shown depend on plot type (conversation / villain / open object) and conversation type (quest / exchange items / single statement); first use of a cast member asks for Plotless/Undefined greetings (`tsConvOther`). Generates DLG nodes, scripts and journal entries.

#### `TfraPlotManager` — (no caption)
*DFM `TFRAPLOTMANAGER` · StrRef table .data+0xD196C (6/6 captions matched)*

**Purpose:** Plot Manager tree (bottom of palette pane).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `tvMain` | tree |  | plots (.ptm) and nodes | `OnClick=tvMainClick OnDblClick=tvMainDblClick` |

**Action list `alMain`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actEdit` | Edit |  | `actEditExecute` |  |  |
| `actSelect` | Select |  | `actSelectExecute` |  |  |
| `actDelete` | Delete |  | `actDeleteExecute` |  |  |
| `actNew` | New |  | `actNewExecute` |  |  |
| `actNewNonPlot` | New |  | `actNewNonPlotExecute` |  |  |
| `actAddToNonPlot` | Add |  | `actAddToNonPlotExecute` |  |  |
| `actSetModified` | actSetModified |  | `actSetModifiedExecute` |  |  |

**Popup menu `pmMain`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `New1` | New |  | `actNew` |  |
| `New2` | New |  | `actNewNonPlot` |  |
| `miEdit` | Edit |  | `actEdit` |  |
| `miSelect` | Select |  | `actSelect` |  |
| `Delete1` | Delete |  | `actDelete` |  |
| `Add1` | Add |  | `actAddToNonPlot` |  |

#### `TfraProgress` — (no caption)
*DFM `TFRAPROGRESS` · no StrRef table resolved · form events: OnResize=FrameResize*

**Purpose:** Wizard progress strip (step list) used by plot wizards.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `dgMain` | draw-grid |  | step list (current step highlighted) |  |

## 20. Palettes & blueprint selection

#### `TfraMainPalette` — (no caption)
*DFM `TFRAMAINPALETTE` · StrRef table .data+0x50838 (13/25 captions matched)*

**Purpose:** Right pane: blueprint palettes (Standard/Custom × 9 object types, Terrain), paint-mode toolbar, find bar, plot manager.

**Tabs** (`pcMain`): Standard · Custom
**Tabs** (`pcStandard`): Creatures · Doors · Encounters · Items · Placeables · Sounds · Stores · Triggers · Waypoints · Terrain
**Tabs** (`pcCustom`): Creatures · Doors · Encounters · Items · Placeables · Sounds · Stores · Triggers · Waypoints

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Standard › Creatures | `tvStandardCreatures` | tree | ⟨Creatures⟩ | standard creatures palette (`creaturepal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Doors | `tvStandardDoors` | tree | ⟨Doors⟩ | standard doors palette (`doorpal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Encounters | `tvStandardEncounters` | tree | ⟨Encounters⟩ | standard encounters palette (`encounterpal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Items | `tvStandardItems` | tree | ⟨Items⟩ | standard items palette (`itempal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Placeables | `tvStandardPlaceables` | tree | ⟨Placeables⟩ | standard placeables palette (`placeablepal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Sounds | `tvStandardSounds` | tree | ⟨Sounds⟩ | standard sounds palette (`soundpal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Stores | `tvStandardStores` | tree | ⟨Stores⟩ | standard stores palette (`storepal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Triggers | `tvStandardTriggers` | tree | ⟨Triggers⟩ | standard triggers palette (`triggerpal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Waypoints | `tvStandardWaypoints` | tree | ⟨Waypoints⟩ | standard waypoints palette (`waypointpal.itp`, read-only) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvStandardKeyDown OnMouseDown=tvStandardMouseDown` |
| Standard › Terrain | `tvTerrain` | tree | ⟨Terrain⟩ | tileset Features / Groups / terrain types, Eraser, Raise/Lower | `OnChange=tvTerrainChange OnClick=tvTerrainClick OnKeyDown=tvTerrainKeyDown OnMouseDown=tvTerrainMouseDown` |
| Custom › Creatures | `tvCustomCreatures` | tree | ⟨Creatures⟩ | custom creatures palette (`creaturepalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Doors | `tvCustomDoors` | tree | ⟨Doors⟩ | custom doors palette (`doorpalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Encounters | `tvCustomEncounters` | tree | ⟨Encounters⟩ | custom encounters palette (`encounterpalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Items | `tvCustomItems` | tree | ⟨Items⟩ | custom items palette (`itempalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Placeables | `tvCustomPlaceables` | tree | ⟨Placeables⟩ | custom placeables palette (`placeablepalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Sounds | `tvCustomSounds` | tree | ⟨Sounds⟩ | custom sounds palette (`soundpalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Stores | `tvCustomStores` | tree | ⟨Stores⟩ | custom stores palette (`storepalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Triggers | `tvCustomTriggers` | tree | ⟨Triggers⟩ | custom triggers palette (`triggerpalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
| Custom › Waypoints | `tvCustomWaypoints` | tree | ⟨Waypoints⟩ | custom waypoints palette (`waypointpalcus.itp`, editable, in-place rename) | `OnChange=tvChange OnClick=tvClick OnKeyDown=tvCustomKeyDown OnMouseDown=tvCustomMouseDown` |
|  | `sbStartLocation` | speed button | (no caption; glyph) | paint start location → IFO `Mod_Entry_*` | tip: “Paint Start Location”; `OnClick=sbStartLocationClick` |
|  | `sbTerrain` | speed button | (no caption; glyph) | terrain paint mode | tip: “Paint Terrain”; `OnClick=sbTerrainClick` |
|  | `cbPalettes` | combo |  | (hidden) old palette selector | **hidden**; list; `OnChange=cbPalettesChange` |
|  | `tbCreatures` | tool button |  |  | toggle; tip: “Paint Creatures”; `OnClick=tbPaletteClick` |
|  | `tbDoors` | tool button |  |  | toggle; tip: “Paint Doors”; `OnClick=tbPaletteClick` |
|  | `tbEncounters` | tool button |  |  | toggle; tip: “Paint Encounters”; `OnClick=tbPaletteClick` |
|  | `tbItems` | tool button |  |  | toggle; tip: “Paint Items”; `OnClick=tbPaletteClick` |
|  | `tbStores` | tool button |  |  | toggle; tip: “Paint Merchants”; `OnClick=tbPaletteClick` |
|  | `tbPlaceables` | tool button |  |  | toggle; tip: “Paint Placeable Objects”; `OnClick=tbPaletteClick` |
|  | `tbSounds` | tool button |  |  | toggle; tip: “Paint Sounds”; `OnClick=tbPaletteClick` |
|  | `tbTriggers` | tool button |  |  | toggle; tip: “Paint Triggers”; `OnClick=tbPaletteClick` |
|  | `tbWaypoints` | tool button |  |  | toggle; tip: “Paint Waypoints”; `OnClick=tbPaletteClick` |
|  | `xbShowPlot` | checkbox | Show Plot (DFM: 'Show Plots') | show Plot Manager | `OnClick=xbShowPlotClick` |
|  | `fraPlotManager1` | embedded frame | `TfraPlotManager` | plot manager | see frame section |
|  | `eFindText` | edit | Find What | palette find | **hidden (container)**; `OnKeyDown=eFindTextKeyDown` |

**Popup menu `pmFrame`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miEdit` | Edit |  |  `miEditClick` |  |
| `miCopy` | Edit Copy |  |  `miEditClick` |  |
| `miNew` | New |  |  `miNewClick` |  |
| `miDelete` | Delete |  |  `miDeleteClick` |  |
| — | ——— | | | |
| `miUpdateInstances` | Update Instances |  |  `miUpdateInstancesClick` |  |
| `miExport` | Export |  |  `miExportClick` | hidden |
| `miImport` | Import |  |  `miImportClick` | hidden |
| — | ——— | | | |
| `miDefault` | Restore Default |  |  | hidden |
| `miFind` | Find Text | Ctrl+F |  `miFindClick` |  |
| `miFindNext` | Find Next | F3 |  `miFindNextClick` |  |
| `miRefreshPalette` | Refresh Palette |  |  `miRefreshPaletteClick` |  |

**Notes / behaviors:**
- Top: paint-mode toggle buttons (one per blueprint type + Terrain + Start Location) switch the visible palette tree; Standard/Custom tabs; bottom: Plot Manager (toggle *Show Plot*).
- Standard trees read-only; custom trees support in-place rename, Del key, popup (Edit, Edit Copy, New, Delete, Update Instances, hidden Export/Import/Restore Default, Find Text Ctrl+F, Find Next F3, Refresh Palette).
- Selecting an item arms placement in the viewer (`tvChange`); `pFind` find bar appears on Ctrl+F.

#### `TfraBlueprintSelect` — (no caption)
*DFM `TFRABLUEPRINTSELECT` · StrRef table .data+0x5E94 (4/4 captions matched)*

**Purpose:** Embeddable blueprint picker (Standard/Custom trees + New/Edit Copy) used by wizards (Store Setup, Plot).

**Tabs** (`pcPalettes`): Standard · Custom

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Standard | `tvStandard` | tree | ⟨Standard⟩ | standard palette | `OnChange=tvChange OnDblClick=tvDblClick OnKeyDown=tvKeyDown OnMouseDown=tvMouseDown` |
| Custom | `tvCustom` | tree | ⟨Custom⟩ | custom palette | `OnChange=tvChange OnDblClick=tvDblClick OnKeyDown=tvKeyDown OnMouseDown=tvMouseDown` |
|  | `bNew` | button | New | new blueprint (wizard) | `Action=actNew` |
|  | `bEditCopy` | button | Edit Copy | clone into custom palette | `Action=actEditCopy` |

**Action list `alBlueprints`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actNew` | New |  | `actNewExecute` |  |  |
| `actEditCopy` | Edit Copy |  | `actEditCopyExecute` |  |  |

**Popup menu `pmBlueprints`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miSelect` | Select |  |  `miSelectClick` |  |
| `miNew` | New |  | `actNew` |  |
| `miEditCopy` | Edit Copy |  | `actEditCopy` |  |

#### `TdlgPaletteChooser` — Select Category
*DFM `TDLGPALETTECHOOSER` · 258×332 · border bsDialog · StrRef table .data+0xB47E4 (3/3 captions matched)*

**Purpose:** "Select Category": choose palette category (ITP node) for a blueprint.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `tvPalette` | tree |  | ITP category (`PaletteID`) | tip: “Select a category”; `OnClick=tvPaletteClick OnDblClick=tvPaletteDblClick` |
|  | `bOk` | button | Ok |  | tip: “Accept changes”; `OnClick=bOkClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |

#### `TdlgResourceSelection` — Select Resource
*DFM `TDLGRESOURCESELECTION` · 224×426 · border bsDialog · StrRef table .data+0xDF9EC (4/14 captions matched) · DFM caption 'Select Resources'*

**Purpose:** "Select Resources": pick blueprints of any type (used by Export and Plot wizard).

**Tabs** (`pcMain`): Standard · Custom

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOk` | button | Ok |  | disabled |
|  | `bCancel` | button | Cancel |  |  |
| Standard | `tvStandard` | tree | ⟨Standard⟩ | standard blueprints | `OnChange=tvChange OnDblClick=tvDblClick` |
| Custom | `tvCustom` | tree | ⟨Custom⟩ | custom blueprints | `OnChange=tvChange OnDblClick=tvDblClick` |
|  | `tbCreatures` | tool button |  |  | toggle; tip: “Paint Creatures”; `OnClick=tbObjectClick` |
|  | `tbDoors` | tool button |  |  | toggle; tip: “Paint Doors”; `OnClick=tbObjectClick` |
|  | `tbEncounters` | tool button |  |  | toggle; tip: “Paint Encounters”; `OnClick=tbObjectClick` |
|  | `tbItems` | tool button |  |  | toggle; tip: “Paint Items”; `OnClick=tbObjectClick` |
|  | `tbStores` | tool button |  |  | toggle; tip: “Paint Merchants”; `OnClick=tbObjectClick` |
|  | `tbPlaceables` | tool button |  |  | toggle; tip: “Paint Placeable Objects”; `OnClick=tbObjectClick` |
|  | `tbSounds` | tool button |  |  | toggle; tip: “Paint Sounds”; `OnClick=tbObjectClick` |
|  | `tbTriggers` | tool button |  |  | toggle; tip: “Paint Triggers”; `OnClick=tbObjectClick` |
|  | `tbWaypoints` | tool button |  |  | toggle; tip: “Paint Waypoints”; `OnClick=tbObjectClick` |

#### `TdlgChooser` — Select Type
*DFM `TDLGCHOOSER` · 253×201 · border bsDialog · StrRef table .data+0x59724 (3/3 captions matched)*

**Purpose:** Generic "Select Type" tree chooser (e.g. item base type in Item Wizard / script-editor templates).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `m_bOk` | button | OK |  | tip: “Accept changes”; `OnClick=m_bOkClick` |
|  | `m_bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `tvDisplay` | tree |  | choice tree | `OnClick=tvDisplayClick OnDblClick=m_bOkClick` |

#### `TdlgDefaultSelector` — Select Patttern
*DFM `TDLGDEFAULTSELECTOR` · 187×242 · border bsDialog · no StrRef table resolved*

**Purpose:** Generic "Select Pattern" list chooser.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  |  |
|  | `bCancel` | button | Cancel |  |  |
|  | `lbDefaults` | listbox |  | choice list | `OnDblClick=lbDefaultsDblClick` |

## 21. Resource pickers (portrait, sound, sound set, load screen, resource open)

Sound-set picker (`TdlgSoundSetSelect`) is in §5, colour pickers in §3/§5.

#### `TdlgPortrait` — Select Portrait
*DFM `TDLGPORTRAIT` · 632×453 · border bsDialog · StrRef table .data+0xD9CD4 (9/9 captions matched) · form events: OnShow=FormShow*

**Purpose:** Select Portrait (portraits.2da thumbnails with filters).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `imgPortrait` | image |  | large preview (po_*.tga/dds) |  |
|  | `lvPortraits` | listview |  | portraits.2da thumbnails | tip: “Click on the portrait that you wish to use.”; `OnDblClick=lvPortraitsDblClick OnSelectItem=lvPortraitsSelectItem` |
| Filters | `cbRace` | combo | ⟨Filters⟩ | portraits.2da `Race` filter | list; `OnChange=cbRaceChange` |
| Filters | `xbGender` | checkbox | Gender: | enable gender filter | `OnClick=xbGenderClick` |
| Filters | `cbGender` | combo | ⟨Filters⟩ | portraits.2da `Sex` filter | list; `OnChange=cbGenderChange` |
| Filters | `xbRace` | checkbox | Race: | enable race filter | `OnClick=xbRaceClick` |
| Filters | `rbInanimate` | radio | Placeable Objects and Doors | portraits.2da `InanimateType` set | `OnClick=rbInanimateClick` |
| Filters | `rbCreature` | radio | Characters and Creatures | creature portraits | `OnClick=rbCreatureClick` |
| Filters | `xbInanimateCategory` | checkbox | Category | enable category filter | `OnClick=xbInanimateCategoryClick` |
| Filters | `cbInanimateCategory` | combo | ⟨Filters⟩ | `InanimateType` category | list; `OnChange=cbInanimateCategoryChange` |
| Filters | `rbPlotCharacters` | radio | Plot Characters | `Plot` portraits | `OnClick=rbPlotCharactersClick` |

**Notes / behaviors:**
- Thumbnails generated from portraits.2da `BaseResRef` + size suffix; filters: creature vs inanimate (placeables/doors) vs plot characters, race, gender, inanimate category.

#### `TdlgResOpen` — Select Resource
*DFM `TDLGRESOPEN` · 632×453 · border bsDialog · StrRef table .data+0x767E8 (12/14 captions matched) · form events: OnShow=FormShow · DFM caption 'Open Resource'*

**Purpose:** Open Resource: generic resource browser (type, name filter, location filter); used for scripts, conversations, sounds, etc.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `lvResources` | listview |  | resources (in-place rename, context menu) | `OnDblClick=lvResourcesDblClick OnEdited=lvResourcesEdited OnKeyDown=lvResourcesKeyDown OnSelectItem=lvResourcesSelectItem` |
|  | `cbResType` | combo | Resources of Type: | resource type | list; `OnChange=cbResTypeChange` |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `cbResRef` | combo | Resource Name: | resource name (type-ahead) | max 16; editable; `OnChange=cbResRefChange` |
| Resources to Show | `rbAllResources` | radio | All Resources | location filter | `OnClick=rbResourceLocationFilterClick` |
| Resources to Show | `rbGlobalResourcesOnly` | radio | Global Resources Only | (hidden) | **hidden**; `OnClick=rbResourceLocationFilterClick` |
| Resources to Show | `rbHakPakResourcesOnly` | radio | Hak Pak Resources Only | location filter | `OnClick=rbResourceLocationFilterClick` |
| Resources to Show | `rbModuleResourcesOnly` | radio | Module Resources Only | location filter | default on; `OnClick=rbResourceLocationFilterClick` |
|  | `cbResRefFilter` | combo | Name Filter: | name prefix filter | max 16; list; `OnChange=cbResRefFilterChange` |

**Action list `alOpenResource`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actDeleteResource` | Delete |  | `actDeleteResourceExecute` |  |  |
| `actExportResource` | Export... |  | `actExportResourceExecute` |  |  |

**Popup menu `pmResource`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miExport` | Export... |  | `actExportResource` | hidden |
| `miDelete` | Delete |  | `actDeleteResource` |  |

**Notes / behaviors:**
- Location filter: module / hak paks / all (global hidden). List view in-place rename (module resources only); popup Delete (and hidden Export…). Name filter combo with prefixes. Count label.

#### `TdlgResOpenSound` — dlgResOpenSound
*DFM `TDLGRESOPENSOUND` · no StrRef table resolved*

**Purpose:** Open Resource for WAV sounds with preview playback.

**Inherits:** `TdlgResOpen` — adds Play/Stop for WAV preview.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `lvResources` | listview |  | WAV resources |  |
|  | `cbResType` | combo |  | resource type (wav) | editable |
|  | `bCancel` | button | (no caption; runtime) |  |  |
|  | `cbResRef` | combo |  | sound resref | editable |
| gbFilters | `rbGlobalResourcesOnly` | radio |  | location filter |  |
| gbFilters | `rbModuleResourcesOnly` | radio |  | location filter |  |
|  | `bPlay` | button | Play | play WAV | `OnClick=bPlayClick` |
|  | `bStop` | button | Stop | stop | `OnClick=bStopClick` |

**Action list `alOpenResource`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actPlay` | Play |  | `actPlayExecute` |  |  |

**Popup menu `pmResource`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `pmiPlay` | Play |  | `actPlay` |  |

#### `TdlgLoadScreen` — Loading Screen
*DFM `TDLGLOADSCREEN` · 769×525 · border bsDialog · StrRef table .data+0x499F0 (4/5 captions matched) · form events: OnShow=FormShow · DFM caption 'Select Load Screen'*

**Purpose:** Select Load Screen (loadscreens.2da thumbnails, tileset filter, random).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `imgLoadScreen` | image |  | large preview |  |
|  | `lvLoadScreens` | listview |  | loadscreens.2da thumbnails → `LoadScreenID` | `OnDblClick=lvLoadScreensDblClick OnSelectItem=lvLoadScreensSelectItem` |
| Filters | `cbTileSet` | combo | ⟨Filters⟩ | loadscreens.2da tileset filter | list; `OnChange=cbTileSetChange` |
| Filters | `xbFilterTileSet` | checkbox | Tileset (DFM: 'Filter by Tileset:') | enable filter | `OnClick=xbFilterTileSetClick` |
|  | `xbRandom` | checkbox | Use Random Loading Screen | `LoadScreenID`=0 (random) | `OnClick=xbRandomClick` |

**Notes / behaviors:**
- Thumbnails of loadscreens.2da entries; *Use Random Loading Screen* stores 0.

#### `TdlgResTypeSelector` — Resource Type Selection
*DFM `TDLGRESTYPESELECTOR` · 275×96 · border bsDialog · no StrRef table resolved*

**Purpose:** Resource Type Selection (radio buttons built at runtime).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `OK` | button | OK |  |  |
|  | `bCancel` | button | Cancel |  |  |

## 22. Localised strings, variables, comments

#### `TdlgLocString` — String Edit
*DFM `TDLGLOCSTRING` · 368×343 · border bsDialog · StrRef table .data+0x77004 (3/3 captions matched) · form events: OnShow=FormShow*

**Purpose:** String Edit: CExoLocString editor (languages × genders grid).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | tip: “Accept changes” |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `sgStrings` | grid |  | CExoLocString entries (Language × Gender) / StrRef | `OnDblClick=actEditExecute` |
|  | `mString` | memo |  | selected entry text | read-only |
|  | `bEdit` | button | Edit | opens TdlgNewEditStringExternal | `Action=actEdit` |
|  | `bReset` | button | Reset | clear entry | disabled; `Action=actReset` |
|  | `bApply` | button | Done | commit | disabled; `Action=actApply` |

**Action list `alLocString`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actEdit` | Edit |  | `actEditExecute` |  |  |
| `actApply` | Apply |  | `actApplyExecute` |  |  |
| `actReset` | Reset |  | `actResetExecute` |  |  |

#### `TdlgNewEditStringExternal` — Edit String
*DFM `TDLGNEWEDITSTRINGEXTERNAL` · 346×247 · border bsDialog · StrRef table .data+0xFABCC (6/6 captions matched)*

**Purpose:** Edit String (one language/gender entry or a TLK StrRef).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `OKButton` | button | OK |  | `OnClick=OKButtonClick` |
|  | `CancelButton` | button | Cancel |  |  |
|  | `rbMale` | radio | Male | gender | default on; `OnClick=rbClick` |
|  | `rbFemale` | radio | Female | gender | `OnClick=rbClick` |
| Language | `cbLanguage` | combo | ⟨Language⟩ | language id | list; `OnChange=cbLanguageChange` |
| String Ref | `eStrRef` | edit | ⟨String Ref⟩ | CExoLocString `StringRef` (TLK) | `OnChange=eStrRefChange` |
|  | `StringMemo` | memo |  | substring text |  |

#### `TdlgVarTable` — Variables
*DFM `TDLGVARTABLE` · 555×375 · border bsDialog · StrRef table .data+0x105E2C (8/8 captions matched) · DFM caption 'Scripting Variables'*

**Purpose:** Scripting Variables (VarTable) editor.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |
|  | `lvVariables` | listview |  | `VarTable[]` (`Name`,`Type`,`Value`) | cols: Name / Type / Value; `OnColumnClick=lvVariablesColumnClick OnKeyDown=lvVariablesKeyDown OnSelectItem=lvVariablesSelectItem` |
|  | `eName` | edit | Name | var `Name` | max 1024; `OnChange=eNameChange` |
|  | `eValue` | edit | Value | var `Value` | max 1024; `OnChange=eValueChange OnExit=eValueExit` |
|  | `cbType` | combo | Type | var `Type` (1 int, 2 float, 3 string) | list; `OnChange=cbTypeChange` |
|  | `bReplace` | button | Replace | update selected | `OnClick=bReplaceClick` |
|  | `bAdd` | button | Add | add | `OnClick=bAddClick` |
|  | `bDelete` | button | Delete | delete | `OnClick=bDeleteClick` |

#### `TdlgComments` — Edit Comments
*DFM `TDLGCOMMENTS` · no Tags — English only*

**Purpose:** Edit Comments (plain memo).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mComments` | memo |  | `Comment` |  |
|  | `bOK` | button | OK |  | `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |

## 23. Import / export

#### `TdlgImportExport` — Import Resources
*DFM `TDLGIMPORTEXPORT` · 255×495 · border bsDialog · StrRef table .data+0x3B76C (21/24 captions matched)*

**Purpose:** Import / Export resources (ERF): export list, missing-dependency confirmation, overwrite resolution, comments.

**Tabs** (`pcPages`): Export list · Missing resources · Overwrite · Comments

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Export list | `bExportContinue` | button | OK |  | tip: “Accept changes”; `OnClick=bExportContinueClick` |
| Export list | `lbExportList` | listbox | Click the "Add Resources..." button to select the resources that you wish to export. | resources to export | multi-select; `OnKeyDown=lbExportListKeyDown` |
| Export list | `bExportCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bOverwriteCancelClick` |
| Export list | `bSelectForExport` | button | Add Resources... | opens resource selection | `OnClick=bSelectForExportClick` |
| Missing resources | `bMissingContinueYes` | button | Yes |  | `OnClick=bMissingContinueYesClick` |
| Missing resources | `bMissingContinueNo` | button | No |  | `OnClick=bMissingContinueNoClick` |
| Missing resources | `mMissingResources` | memo | The following resources were required by the package selected for import, but they coul… | missing dependencies | read-only |
| Missing resources | `bMissingBack` | button | Back |  | **hidden**; tip: “Return to the previous step in the Wizard”; `OnClick=bMissingBackClick` |
| Overwrite | `lbOverwriteCandidates` | listbox | Select the resources that you wish to overwrite. Deselect the resources that you do not… | resources to overwrite | multi-select; `OnClick=lbOverwriteCandidatesClick OnKeyDown=lbOverwriteCandidatesKeyDown` |
| Overwrite | `bOverwriteOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOverwriteOKClick` |
| Overwrite | `bOverwriteCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bOverwriteCancelClick` |
| Overwrite | `bOverwriteBack` | button | Back |  | tip: “Return to the previous step in the Wizard”; `OnClick=bOverwriteBackClick` |
| Overwrite | `bOverwriteSelectAll` | button | Select All |  | `OnClick=bOverwriteSelectAllClick` |
| Overwrite | `bOverwriteSelectNone` | button | Select None |  | `OnClick=bOverwriteSelectNoneClick` |
| Comments | `bCommentsOK` | button | OK |  | tip: “Accept changes”; `OnClick=bCommentsOKClick` |
| Comments | `bCommentsCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCommentsCancelClick` |
| Comments | `bCommentsBack` | button | Back |  | tip: “Return to the previous step in the Wizard”; `OnClick=bCommentsBackClick` |
| Comments | `mComments` | memo | Type in any comments that you have about this export package. | ERF description (`DescriptionStrRef`/localized string list) | max 1023 |
| Comments | `xbFactionReset` | checkbox | Reset Factions to Parent Factions | export: reset instance factions to parent faction | default on |

**Notes / behaviors:**
- Wizard-like pages: Export list → (Missing resources prompt) → Overwrite candidates (select all/none) → Comments (+ reset factions). Import = choose .erf, resolve overwrites.

#### `TdlgMultiSelect` — dlgMultiSelect
*DFM `TDLGMULTISELECT` · 323×490 · border bsDialog · StrRef table .data+0x6FDF4 (4/4 captions matched) · form events: OnResize=FormResize*

**Purpose:** Generic multi-select list/grid dialog (instructions set by caller).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `sgItems` | grid | Calling function should place usage instructions here. | multi-column choice |  |
|  | `lbItems` | listbox | Calling function should place usage instructions here. | single-column choice | multi-select |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes”; `OnClick=bCancelClick` |
|  | `bSelectAll` | button | Select All |  | `OnClick=bSelectAllClick` |
|  | `bSelectNone` | button | Select None |  | `OnClick=bSelectNoneClick` |

## 24. Hak paks & custom TLK

Hak list and custom TLK are edited in Module Properties › Custom Content (§2); this dialog is the conflict analyser.

#### `TdlgHakPak` — Hak Pak Conflict Analysis
*DFM `TDLGHAKPAK` · 781×548 · border bsDialog · StrRef table .data+0x3936C (8/10 captions matched) · form events: OnClose=FormClose*

**Purpose:** Hak Pak Conflict Analysis (Module Properties › Custom Content › Check for Conflicts).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `lbHakFiles` | listbox | List of Hak Paks attached to module | IFO `Mod_HakList` (same as Module Properties) | tip: “List of Hak Paks attached to module”; `OnClick=lbHakFilesClick OnKeyDown=lbHakFilesKeyDown` |
|  | `cbHakFile` | combo |  | hak to add | list; tip: “List of available Hak Paks”; `OnChange=cbHakFileChange` |
|  | `bHakAdd` | button | Add (DFM: 'Add Hak') |  | `OnClick=bHakAddClick` |
|  | `bHakMoveUp` | button | Move Up |  | `OnClick=bHakMoveUpClick` |
|  | `bHakMoveDown` | button | Move Down |  | `OnClick=bHakMoveDownClick` |
|  | `bHakRemove` | button | Remove (DFM: 'Delete') |  | `OnClick=bHakRemoveClick` |
|  | `bReport` | button | Report Resources and Conflicts... | save text report | tip: “Click to generate conflict and resource usage information.”; `OnClick=bReportClick` |
|  | `lvConflicts` | listview | Conflicting Resources | resources in >1 hak | cols: Resource / Type / Hak Paks Containing Resource; tip: “Resources found in multiple Hak Paks”; `OnColumnClick=lvResourcesColumnClick` |
|  | `lvResources` | listview | Complete Resource List | every resource across haks | cols: Resource / Type / Hak Paks Containing Resource; tip: “List of all resources in all Hak Paks”; `OnColumnClick=lvResourcesColumnClick` |
|  | `lvOverrides` | listview | Overriden standard resources | haks overriding base-game resources | cols: Resource / Type / Hak Paks Containing Resource; tip: “Standard resources overriden by Hak Pak resources”; `OnColumnClick=lvResourcesColumnClick` |
|  | `bClose` | button | Close |  | `OnClick=bCloseClick` |

**Notes / behaviors:**
- Three report list views (Resource / Type / Hak Paks containing resource), sortable by column; *Report Resources and Conflicts…* saves a text report.

## 25. Options

#### `TdlgOptions` — Options
*DFM `TDLGOPTIONS` · 632×453 · border bsDialog · StrRef table .data+0x70374 (66/79 captions matched) · form events: OnShow=FormShow*

**Purpose:** Toolset Options (tree of pages; values persisted in nwtoolset.ini).

**Tabs** (`pcOptions`): Area · General · Script Editor · Conversation Editor · Spell Checking · Sounds · Language

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `tvOptions` | tree |  | page selector | `OnChange=tvOptionsChange` |
|  | `mInfo` | memo |  | (hidden) option info | **hidden**; read-only |
| Area | `pBackgroundColor` | color swatch/panel | Background Color (click panel to change) | ini `Background Color` | `OnClick=pBackgroundColorClick` |
| Area | `xbAreaShowSpawnpointMarkers` | checkbox | Show Encounter Spawnpoint Markers (DFM: 'Show Markers Above Encounter Spawnpoints') | ini `Show Spawnpoint Markers` | `OnClick=xbAreaShowSpawnpointMarkersClick` |
| Area | `eAreaSpawnpointMarkerHeight` | edit | Height | ini `Spawnpoint Marker Height` | spin 0…100; `OnExit=eAreaSpawnpointMarkerLengthExit` |
| Area | `eAreaSpawnpointMarkerWidth` | edit | Width | ini `Spawnpoint Marker Width` | spin 0…100; `OnExit=eAreaSpawnpointMarkerLengthExit` |
| Area | `xbAreaShowDoorOrientation` | checkbox | Show Door Orientation Arrows | ini `Show Door Orientation` | `OnClick=xbAreaShowSpawnpointMarkersClick` |
| Area | `eUndo` | edit | Undo Levels | ini `UndoStackSize` | spin 1…32 |
| General | `xbShowWelcome` | checkbox | Show welcome dialog at startup | ini `Show Welcome` |  |
| General | `xbVerifyOnSave` | checkbox | Build module on save (DFM: 'Verify module on save') | ini `Verify On Save` |  |
| General | `xbWarnReservedNameSpace` | checkbox | Show reserved Blueprint ResRef namespace warning (DFM: 'Show Blueprint ResRef name spac… | ini `NameSpace Warning` |  |
| General | `xbWarnCreatureSpells` | checkbox | Show invalid creature spell assignment warning | ini `Creature Spells` |  |
| General | `xbWarnInventory` | checkbox | Show creature inventory warning | ini `Inventory` |  |
| General | `xbWarnColorDepth` | checkbox | Show 32-bit color depth warning | ini `ColorDepth` |  |
| General | `xbWarnCharacterSet` | checkbox | Show character set warning | ini `Character Set` |  |
| General | `xbUseEnvironmentMapping` | checkbox | Use environment mapping on Creatures and Items (DFM: 'Use environment mapping in second… | ini `UseEnvironmentMapping` |  |
| General | `xbWarnResourceInHak` | checkbox | Show resource in Hak Pak warning | ini `ResourceInHak` |  |
| General | `xbWarnStandardResource` | checkbox | Show standard resource overwrite warning | ini `Standard Resource Warning` |  |
| General | `xbCreateBackupModules` | checkbox | Create backups of modules (DFM: 'Create Backup Modules') | ini `Create Backup Modules` |  |
| General | `xbMinimizeOnTest` | checkbox | Minimize Toolset on test module | ini `MinimizeOnTest` |  |
| General | `xbOpenModuleDir` | checkbox | Always open module directories | ini `Always Open Module Directory` |  |
| Script Editor | `eScriptTemplateDir` | edit | Code Templates Directory | ini `Templates` |  |
| Script Editor | `bScriptTemplateDir` | button | ... | browse dir | `OnClick=bScriptTemplateDirClick` |
| Script Editor | `eScriptFontName` | edit | Font (name) | ini `FontName` | read-only |
| Script Editor | `eScriptFontSize` | edit | Font (size) | ini `FontSize` | read-only |
| Script Editor | `bScriptFont` | button | ... (Font) | font dialog | `OnClick=bScriptFontClick` |
| Script Editor | `pScriptEditorColors` | color swatch/panel | Colours | hosts TfraScriptEditorColor |  |
| Script Editor | `xbAutoCompile` | checkbox | Automatically Compile Scripts on Save | ini `AutoCompile` |  |
| Script Editor | `xbScriptGenerateDebugInfo` | checkbox | Generate Debug Information When Compiling Scripts | ini `GenerateDebugInfo` (.ndb) |  |
| Script Editor | `bScriptExternalEditor` | button | ... | browse exe | `OnClick=bScriptExternalEditorClick` |
| Script Editor | `eScriptExternalEditor` | edit | External Script Editor | ini `ExternalEditor` |  |
| Conversation Editor | `ePlrDefaultDelay` | edit | Default delay for player speech | (hidden) ini `PlrDefaulDelay` | **hidden**; spin 0…60 |
| Conversation Editor | `eNPCDefaultDelay` | edit | Default delay for NPC speech | (hidden) ini `NPCDefaulDelay` | **hidden**; spin 0…60 |
| Conversation Editor | `chbTextPopup` | checkbox | Show popup when creating a new text entry | ini `TextPopup` | default on |
| Conversation Editor | `chbShowName` | checkbox | Show speaker name before text | ini `ShowNames` | default on |
| Conversation Editor | `pPlrTextColor` | color swatch/panel | Player Text Color | ini `PlayerTextColor` | `OnClick=TextColorClick` |
| Conversation Editor | `pNPCTextColor` | color swatch/panel | NPC Text Color | ini `NPCTextColor` | `OnClick=TextColorClick` |
| Conversation Editor › Paste Link Options | `rbSrcToDest` | radio | Link Source To Destination | ini `ConvLinkPasteMode` |  |
| Conversation Editor › Paste Link Options | `rbDestToSrc` | radio | Link Destination To Source | ini `ConvLinkPasteMode` |  |
| Conversation Editor › Drag Link Options | `rbSrcToDest2` | radio | Link Source To Destination | ini `ConvLinkLinkMode` |  |
| Conversation Editor › Drag Link Options | `rbDestToSrc2` | radio | Link Destination To Source | ini `ConvLinkLinkMode` |  |
| Conversation Editor | `xbAutoBackup` | checkbox | Automatically backup the conversation files (DFM: 'Automatically backup the conversatio… | ini `AutoBackup` | default on; `OnClick=xbAutoBackupClick` |
| Conversation Editor | `eAutoBackup` | edit | Backup interval (minutes) | ini `AutoBackupInterval` (min) | read-only; spin 1…180 |
| Conversation Editor | `pConvBackColor` | color swatch/panel | Background colour | (hidden) ini `BackgroundColor` | **hidden**; `OnClick=TextColorClick` |
| Spell Checking | `xbEnableSpellCheck` | checkbox | Enable Spell Check (master toggle for all spell checking in toolset) | ini `EnableSpellCheck` | `OnClick=xbEnableSpellCheckClick` |
| Spell Checking › Non-conversation automatic spellcheck | `xbCheckSpellingLocEdit` | checkbox | Spelling | (hidden) ini `CheckSpellingLoc` | **hidden** |
| Spell Checking › Non-conversation automatic spellcheck | `xbCheckGrammarLocEdit` | checkbox | Grammar | (hidden) ini `CheckGrammarLoc` | **hidden** |
| Spell Checking › Non-conversation automatic spellcheck | `xbAutoSpellCheckLocEdit` | checkbox | Automatically check spelling when editing localized strings | ini `AutoSpellCheckLoc` | `OnClick=xbAutoSpellCheckLocEditClick` |
| Spell Checking › Non-conversation automatic spellcheck | `xbInteractiveCheckOnEnterLocEdit` | checkbox | Use interactive spellcheck on enter | (hidden) ini `InteractiveCheckOnEnterLoc` | **hidden** |
| Spell Checking › Non-conversation automatic spellcheck | `xbInteractiveCheckOnExitLocEdit` | checkbox | Use interactive spellcheck on exit | (hidden) ini `InteractiveCheckOnExitLoc` | **hidden** |
| Spell Checking › Non-conversation automatic spellcheck | `pTextColorGrammaticalError` | color swatch/panel | Color of Text with Grammatical Errors | (hidden) ini `TextColorGrammaticalError` | **hidden**; `OnClick=pColorClick` |
| Spell Checking › Non-conversation automatic spellcheck | `pTextColorSpellingAndGrammaticalError` | color swatch/panel | Color of Text with both Spelling and Grammatical Errors | (hidden) ini `TextColorSpellingAndGrammaticalError` | **hidden**; `OnClick=pColorClick` |
| Spell Checking › Non-conversation automatic spellcheck | `mLocEditSpellCheckOptions` | memo | info text | info text | read-only |
| Spell Checking | `pTextColorSpellingError` | color swatch/panel | Color of Text with Spelling Errors | ini `TextColorSpellingError` | `OnClick=pColorClick` |
| Spell Checking › Conversation Editor Spellcheck Options | `xbCheckSpellingConvEdit` | checkbox | Spelling | (hidden) ini `CheckSpellingConv` | **hidden** |
| Spell Checking › Conversation Editor Spellcheck Options | `xbCheckGrammarConvEdit` | checkbox | Grammar | (hidden) ini `CheckGrammarConv` | **hidden** |
| Spell Checking › Conversation Editor Spellcheck Options | `xbAutoSpellCheckConvEdit` | checkbox | Automatically check spelling when editing conversation nodes | ini `AutoSpellCheckConv` | `OnClick=xbAutoSpellCheckConvEditClick` |
| Spell Checking › Conversation Editor Spellcheck Options | `xbInteractiveCheckOnEnterConvEdit` | checkbox | Use interactive spellcheck on enter | ini `InteractiveCheckOnEnterConv` |  |
| Spell Checking › Conversation Editor Spellcheck Options | `xbInteractiveCheckOnExitConvEdit` | checkbox | Use interactive spellcheck on exit | ini `InteractiveCheckOnExitConv` |  |
| Spell Checking | `rbSpellCheckClearTempLex` | radio | Clear Ignore All and Change All word lists after every spellcheck | ini `ClearIgnoreChangeAllLists` |  |
| Spell Checking | `rbSpellCheckKeepTempLex` | radio | Remember Ignore All and Change All word lists between spellchecks | ini `ClearIgnoreChangeAllLists` |  |
| Sounds | `lbSound3DProviders` | listbox |  | (hidden) ini `3D Provider` | **hidden** |
| Sounds | `xbSoundsInArea` | checkbox | Play placed sound objects in area | ini `Enable Area Sounds` |  |
| Sounds | `xbAreaAmbientSound` | checkbox | Play ambient sound in area | ini `Enable Area Ambient Sound` |  |
| Sounds | `xbAreaMusic` | checkbox | Play ambient music in area | ini `Enable Area Music` |  |
| Sounds | `tbVolumeAmbientSound` | slider | Ambient sound volume | (hidden) ambient sound volume | **hidden**; range 0…127 |
| Sounds | `tbVolumeMusic` | slider | Ambient music volume | ini `Ambient Music Volume` | range 0…127 |
| Sounds | `tb2D3DBias` | slider | 2D/3D Bias (2D … 3D) | ini `2D3D Bias` | range 0…200 |
| Sounds | `xbShowListener` | checkbox | Show listener position for 3D sounds | ini `Show Listener Position` |  |
| Language | `rbLanguageUseSpecified` | radio | Specified Language: (DFM: 'Use Specified Language:') | ini `UseDefault`=0 | `OnClick=rbLanguageUseSpecifiedClick` |
| Language | `rbLanguageDefault` | radio | Use Default Language: | ini `UseDefault`=1 | `OnClick=rbLanguageDefaultClick` |
| Language | `lbLanguages` | listbox |  | ini `LanguageID` (editing language) |  |
| Language | `mLanguageHelp` | memo | help text | help text (read-only) | read-only |
|  | `bOK` | button | OK |  | tip: “Accept changes”; `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | tip: “Discard changes” |
|  | `dlgColor` | ColorDialog |  | colour picker |  |
|  | `dlgFileOpen` | file dialog |  | file/dir picker |  |
|  | `dlgFont` | FontDialog |  | font picker |  |

**Notes / behaviors:**
- Tree (`tvOptions`) selects hidden tab sheets: Area, General, Script Editor, Conversation Editor, Spell Check, Sound, Language. Undo levels change requires restart.

## 26. Build / verify / test module

Test Module (F9) has no dialog — see §1 notes and the implicit-behaviors section.

#### `TdlgVerifyModule` — Build Module
*DFM `TDLGVERIFYMODULE` · 542×559 · border bsDialog · StrRef table .data+0x106FA8 (40/42 captions matched) · form events: OnDestroy=FormDestroy, OnShow=FormShow · DFM caption 'Verify Module'*

**Purpose:** Build Module / Verify (Build menu): compile, CR/encounter/palette rebuild, missing/unused resource reports, spell check.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
| Filter | `xbCompileScripts` | checkbox | Scripts | build/verify pass option | default on |
| Filter | `xbMissingEncounters` | checkbox | Encounters | build/verify pass option | default on |
| Filter | `xbUnusedScripts` | checkbox | Scripts | build/verify pass option | default on |
| Filter | `xbUnused` | checkbox | Unused | build/verify pass option | tip: “Check for use”; `OnClick=actEnableFiltersExecute` |
| Filter | `xbUnusedConversations` | checkbox | Conversations | build/verify pass option | default on |
| Filter | `xbUnusedBlueprints` | checkbox | Blueprints | build/verify pass option | default on |
| Filter | `xbMissing` | checkbox | Missing Resources | build/verify pass option | default on; tip: “Check that resources are available”; `OnClick=actEnableFiltersExecute` |
| Filter | `xbMissingCreatures` | checkbox | Creatures | build/verify pass option | default on |
| Filter | `xbMissingDoors` | checkbox | Doors | build/verify pass option | default on |
| Filter | `xbMissingPlaceables` | checkbox | Placeables | build/verify pass option | default on |
| Filter | `xbMissingItems` | checkbox | Items | build/verify pass option | default on |
| Filter | `xbMissingSounds` | checkbox | Sounds | build/verify pass option | default on |
| Filter | `xbMissingTriggers` | checkbox | Triggers | build/verify pass option | default on |
| Filter | `xbMissingWaypoints` | checkbox | Waypoints | build/verify pass option | default on |
| Filter | `xbMissingStores` | checkbox | Stores | build/verify pass option | default on |
| Filter | `xbMissingConversations` | checkbox | Conversations | build/verify pass option | default on |
| Filter | `xbCompile` | checkbox | Compile | build/verify pass option | default on; `OnClick=actEnableFiltersExecute` |
| Filter | `xbCompileCreatureCR` | checkbox | Creature CR | build/verify pass option | default on |
| Filter | `xbCompileEncounter` | checkbox | Encounters | build/verify pass option | default on |
| Filter | `xbCompilePalettes` | checkbox | Palettes | build/verify pass option | default on |
| Filter | `xbMissingAreas` | checkbox | Areas | build/verify pass option | default on |
| Filter | `xbSpellCheckEncounters` | checkbox | Encounters | build/verify pass option | default on |
| Filter | `xbSpellCheck` | checkbox | Spell Check | build/verify pass option | `OnClick=actEnableFiltersExecute` |
| Filter | `xbSpellCheckCreatures` | checkbox | Creatures | build/verify pass option | default on |
| Filter | `xbSpellCheckDoors` | checkbox | Doors | build/verify pass option | default on |
| Filter | `xbSpellCheckPlaceables` | checkbox | Placeables | build/verify pass option | default on |
| Filter | `xbSpellCheckItems` | checkbox | Items | build/verify pass option | default on |
| Filter | `xbSpellCheckSounds` | checkbox | Sounds | build/verify pass option | default on |
| Filter | `xbSpellCheckTriggers` | checkbox | Triggers | build/verify pass option | default on |
| Filter | `xbSpellCheckWaypoints` | checkbox | Waypoints | build/verify pass option | default on |
| Filter | `xbSpellCheckStores` | checkbox | Stores | build/verify pass option | default on |
| Filter | `xbSpellCheckConversations` | checkbox | Conversations | build/verify pass option | default on |
| Filter | `xbSpellCheckAreas` | checkbox | Areas | build/verify pass option | default on |
| Filter | `xbSpellCheckBlueprints` | checkbox | Blueprints | build/verify pass option | default on |
|  | `bVerify` | button | Build |  | `Action=actVerify` |
|  | `bDone` | button | Done |  |  |
| Results | `lbResults` | listbox | ⟨Results⟩ | result lines (dbl-click = open offending object) | `OnDblClick=lbResultsDblClick` |
|  | `xbAdvanced` | checkbox | Advanced Controls | show filter panel | `OnClick=actSetAdvancedPanelExecute` |
|  | `sdResults` | file dialog | Export File | export results | filter `Text Files (*.txt)/*.txt/All Files/*.*` ext .txt |

**Action list `ActionList1`**

| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |
|---|---|---|---|---|---|
| `actVerify` | Build |  | `actVerifyExecute` |  |  |
| `actExport` | Export To File |  | `actExportExecute` |  |  |
| `actSetAdvancedPanel` | Advanced |  | `actSetAdvancedPanelExecute` |  |  |
| `actEnableFilters` |  |  | `actEnableFiltersExecute` |  |  |

**Popup menu `pmResults`**

| Item | Caption (runtime) | Shortcut | Action / handler | State |
|---|---|---|---|---|
| `miExport` | Export to File |  | `actExport` |  |

**Notes / behaviors:**
- Build = compile scripts (+ unused), recompute creature CR, encounter CRs, rebuild palettes, report missing resources by type, unused resources, spell check. Results list exportable to .txt; double-click opens the offending object. Filter checkboxes enabled in groups (`actEnableFiltersExecute`).

## 27. Misc dialogs (startup, about, help, progress, warnings)

#### `TdlgWelcome` — BioWare Aurora Neverwinter Nights Toolset
*DFM `TDLGWELCOME` · 370×396 · border bsDialog · StrRef table .data+0x10F4F4 (8/9 captions matched) · DFM caption 'Aurora Neverwinter Nights Toolset'*

**Purpose:** Startup welcome dialog: create new module / open existing (list) / start normally.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | OK |  | `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |
|  | `rbCreateNew` | radio | Create a new Module | Module Wizard / new module | default on; `OnDblClick=rbCreateNewDblClick` |
|  | `rbOpenExisting` | radio | Open an existing Module: | open selected |  |
|  | `lbModuleFiles` | listbox |  | modules/*.mod | `OnClick=lbModuleFilesClick OnDblClick=lbModuleFilesDblClick` |
|  | `xbShowAtStartup` | checkbox | Show this screen at startup | ini `Show Welcome` |  |
|  | `rbDoNothing` | radio | Start normally | start with no module |  |

**Notes / behaviors:**
- Shown at startup unless disabled; module list from modules folder.

#### `TdlgModuleSelect` — Open
*DFM `TDLGMODULESELECT` · 345×432 · border bsDialog · StrRef table .data+0x54000 (3/5 captions matched) · form events: OnShow=FormShow · DFM caption 'Open/Save Module (As)'*

**Purpose:** Open / Save module picker (modules folder or official campaign).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bOK` | button | Open (DFM: 'OK') |  | `OnClick=bOKClick` |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |
|  | `lbModules` | listbox |  | modules list | `OnClick=lbModulesClick OnDblClick=lbModulesDblClick` |
|  | `eModuleName` | edit | Module Name: | module filename |  |
|  | `rbNormal` | radio | Normal Modules | modules/ folder | `OnClick=rbModuleTypeClick` |
|  | `rbOfficial` | radio | Campaign Modules (DFM: 'Official Campaign') | data/nwm official modules | `OnClick=rbModuleTypeClick` |

#### `TdlgAbout` — About
*DFM `TDLGABOUT` · 464×419 · border bsDialog · StrRef table .data+0x63514 (3/4 captions matched)*

**Purpose:** About box (versions, key file string, licence, expansions installed).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mLicense` | memo |  | EULA text | read-only |
|  | `mExpansionPacks` | memo | Expansion Packs Installed: | installed expansions | read-only |
|  | `bOK` | button | OK |  | tip: “Accept changes” |

#### `TfrmHelp` — NWToolset Help
*DFM `TFRMHELP` · 312×213 · border bsDialog · StrRef table .data+0x8AF80 (2/2 captions matched)*

**Purpose:** Help popup (TLK help text).

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `mHelp` | memo |  | TLK help text of focused control | read-only |
|  | `bClose` | button | Close |  | `OnClick=bCloseClick` |

#### `TdlgProgress` — BioWare Aurora Neverwinter Nights Toolset
*DFM `TDLGPROGRESS` · 409×137 · border bsDialog · StrRef table .data+0xDAC24 (2/2 captions matched) · form events: OnDeactivate=FormDeactivate, OnHide=FormHide, OnShow=FormShow · DFM caption 'Progress'*

**Purpose:** Two-bar progress dialog (task + total), optional Cancel.

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `pbTask` | progress | BioWare Aurora Neverwinter Nights Toolset | current task |  |
|  | `pbTotalProgress` | progress | Total Progress | overall |  |
|  | `bCancel` | button | Cancel |  | **hidden (container)**; `OnClick=bCancelClick` |

#### `TdlgWarning` — dlgWarning
*DFM `TDLGWARNING` · 414×86 · border bsDialog · StrRef table .data+0x7710C (4/4 captions matched) · form events: OnShow=FormShow*

**Purpose:** Suppressible warning box (Yes/No or OK, "Never warn again").

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `xbNoDisplay` | checkbox | Never warn again | suppress this warning (ini) |  |
|  | `bNo` | button | No |  |  |
|  | `bYes` | button | Yes |  |  |
|  | `bOk` | button | OK |  | tip: “Accept changes” |

#### `TdlgConfirmation` — Confirm Action
*DFM `TDLGCONFIRMATION` · 427×97 · border bsDialog · no Tags — English only; button captions set at runtime · form events: OnCloseQuery=FormCloseQuery*

**Purpose:** Yes / Yes to All / No / No to All / Cancel confirmation with "Remember this Response".

| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |
|---|---|---|---|---|---|
|  | `bYes` | button | Yes |  | `OnClick=bYesClick` |
|  | `bYesAll` | button | Yes to All |  | `OnClick=bYesAllClick` |
|  | `bNo` | button | No |  | `OnClick=bNoClick` |
|  | `bCancel` | button | Cancel |  | `OnClick=bCancelClick` |
|  | `xbRememberResponse` | checkbox | Remember this Response | remember answer for this session |  |
|  | `bNoAll` | button | No to All |  | `OnClick=bNoAllClick` |

## Appendix A — All keyboard shortcuts declared in the forms

VCL `ShortCut` decoding: low byte = virtual key, 0x2000 = Shift, 0x4000 = Ctrl, 0x8000 = Alt.
Hard-coded shortcuts handled in `OnKeyDown` code (not in DFMs) are listed in the "Hidden/implicit behaviors" section.

| Form | Command | Shortcut |
|---|---|---|
| TdlgConversationEditor | Find Text (`actFindText`) | Ctrl+F |
| TdlgConversationEditor | Find Next (`actFindNextText`) | F3 |
| TdlgConversationEditor | Replace (`actReplace`) | Ctrl+R |
| TdlgScriptEditor | Compile (`actCompile`) | F7 |
| TdlgScriptEditor | Find (`actFind`) | Ctrl+F |
| TdlgScriptEditor | Search Again (`actSearchAgain`) | F3 |
| TdlgScriptEditor | Replace (`actReplace`) | Ctrl+R |
| TdlgScriptEditor | Toggle Bookmark (`actToggleBookmark`) | F5 |
| TdlgScriptEditor | Update Shortcut List (`actShortcutsReparse`) | F9 |
| TdlgScriptEditor | Save (`actSave`) | Ctrl+S |
| TdlgScriptEditor | Save As (`actSaveAs`) | Ctrl+Alt+S |
| TdlgScriptEditor | Open (`actOpen`) | Ctrl+O |
| TdlgScriptEditor | Close (`actClose`) | Ctrl+F4 |
| TdlgScriptEditor | New (`actNew`) | Ctrl+N |
| TfraMainPalette | Find Text (`miFind`) | Ctrl+F |
| TfraMainPalette | Find Next (`miFindNext`) | F3 |
| TfrmFrame | Exit (`miFileExit`) | Alt+X |
| TfrmFrame | Undo (`miUndo`) | Ctrl+Z |
| TfrmFrame | Redo (`miRedo`) | Ctrl+Shift+Z |
| TfrmFrame | Refresh (`miRefresh`) | F5 |
| TfrmFrame | Test Module (`miTestModule`) | F9 |
| TfrmFrame | Plot Wizard (`miWizardPlot`) | Ctrl+Alt+P |
| TfrmFrame | New (`actNew`) | Ctrl+N |
| TfrmFrame | Open (`actOpen`) | Ctrl+O |
| TfrmFrame | Save (`actSave`) | Ctrl+S |
| TfrmFrame | Conversation Editor (`actEditorConversation`) | Ctrl+Alt+V |
| TfrmFrame | Faction Editor (`actEditorFaction`) | Ctrl+Alt+F |
| TfrmFrame | Script Editor (`actEditorScript`) | Ctrl+Alt+S |
| TfrmFrame | Journal Editor (`actJournalEdit`) | Ctrl+Alt+J |
| TfrmFrame | Copy (`actCopy`) | Ctrl+C |
| TfrmFrame | Cut (`actCut`) | Ctrl+X |
| TfrmFrame | Paste (`actPaste`) | Ctrl+V |
| TfrmFrame | Area Wizard (`actWizardArea`) | Ctrl+Alt+A |
| TfrmFrame | Module Wizard (`actWizardModule`) | Ctrl+Alt+M |
| TfrmFrame | Creature Wizard (`actWizardCreature`) | Ctrl+Alt+C |
| TfrmFrame | Item Wizard (`actWizardItem`) | Ctrl+Alt+I |
| TfrmFrame | Full Screen (`actViewFullScreen`) | F11 |
| TfrmFrame | Toggle Select Terrain / Select Objects (`actSelectToggle`) | F10 |
| TfrmViewerArea | Undo (`Undo`) | Ctrl+Z |
| TfrmViewerArea | Redo (`Redo`) | Ctrl+Shift+Z |
| TfrmViewerArea | Copy (`Copy1`) | Ctrl+C |
| TfrmViewerArea | Cut (`Cut1`) | Ctrl+X |
| TfrmViewerArea | Paste (`Paste1`) | Ctrl+V |

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

## Provenance / regeneration

Working files (scratchpad `uiinv/`): `forms.json` (parsed TPF0 trees), `rep/*.txt` (condensed per-form dumps),
`locbases.json` (StrRef table offsets), `tools/dfmparse.py` (binary DFM → tree), `tools/matchloc.py` (table voting),
`tools/loccheck.py` (per-form DFM-vs-TLK comparison), `tools/gen.py` + `tools/mapping.py` (this document; GFF mappings
and notes are hand-authored in `mapping.py`). Regenerate: `python3 uiinv/tools/gen.py research/aurora_ui_inventory.md`.
GFF field names were checked against base-game blueprints (`nw_*.ut?`), the Prelude module (`module.ifo`, `.are`,
`.git`, `.jrl`, `.dlg`) and field-name strings in the exe (e.g. `Mod_OnPlrTileAct`, `Mod_UUID`, `VisTransformList`).
