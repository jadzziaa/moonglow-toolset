# Parity checklists

Aurora's dialogs (from `aurora-ui-inventory.md`) against Moonglow, control by
control. ✅ done and tested, ◐ partly, ✗ not yet, — deliberately different
(with the reason). Tests named in brackets are in `crates/mg-ui/tests/app.rs`
unless given otherwise.

## Module Properties (`TfrmIFOProp`)

| Aurora | Moonglow | |
|---|---|---|
| Basic: Name (primary language) + `…` all languages | Name field + `…` String Edit | ✅ [module_name_in_every_language_and_variables] |
| Basic: Tag | Tag field | ✅ [edit_undo_save_reopen] |
| Basic: Start Area, X/Y/Z (read-only; set by painting the start location) | read-only | ✅ (start location placement: Phase 9) |
| Events: 21 module events with script picker `…` and Edit; OnModuleStart hidden | same order and labels, picker, Edit opens the script (module) or its read-only view (game); OnModuleStart shown only when set | ✅ |
| Advanced: start month/day/hour/year, minutes per hour, dawn/dusk hour | same, ranges 1–12, 1–31, 0–23, 0–30000, 1–240, 0–23 | ✅ (Aurora's dawn/dusk spin allows −99…99 in a byte field; Moonglow keeps 0–23) |
| Advanced: XP scale slider + number (0–200) | slider with number | ✅ |
| Advanced: Starting Movie (movies/*.bik) | list of `.bik` in the user's and the install's `movies/` | ✅ |
| Advanced: Variables `…` | Variables window | ✅ [module_name_in_every_language_and_variables] |
| Description: memo + `…` all languages | multi-line field (line ends kept) + `…` | ✅ [multi_line_text_keeps_its_line_ends] |
| Custom Content: hak list, Add (from hak folders), Remove, Move Up, Move Down | same; changes apply when the module is reopened | ✅ |
| Custom Content: Check for Conflicts… (TdlgHakPak) | report window (the text of `mg haks`) | ◐ report text only; Aurora's dialog lists conflicts in a grid |
| Custom Content: Custom Tlk File (tlk folder) | list of `.tlk` in the user's `tlk/`; the chosen table is loaded for StrRef lookups | ✅ |
| OK / Cancel | every change is applied at once and undoable | — Moonglow edits live with undo instead of a modal OK |

## String Edit (`TdlgLocString`, `TdlgNewEditStringExternal`)

| Aurora | Moonglow | |
|---|---|---|
| Grid of entries (language × gender) | one row per entry: language, gender, text | ✅ |
| String Ref (TLK) | String Ref field, the talk table's text shown (custom TLK included) | ✅ |
| Edit / Reset / Apply per entry | edit in place, Remove, Add Text | ✅ |
| Line ends | CRLF kept where the text had it | ✅ [widgets::tests] |

## Variables (`TdlgVarTable`)

| Aurora | Moonglow | |
|---|---|---|
| List Name / Type / Value; Add, Replace, Delete | editable rows, Add, Delete | ✅ |
| Types int, float, string | same, values checked before OK | ✅ [widgets::tests] |
| Unknown fields (a `Comment` some shipped variables have) | kept | ✅ [widgets::tests] |

## Select Resource (`TdlgResOpen`)

| Aurora | Moonglow | |
|---|---|---|
| List of resources of a type from the module and the game, filter | same, with None to clear | ✅ |

## Module and Area wizards (`TdlgModuleWizard`, `TdlgAreaWizard`)

| Aurora | Moonglow | |
|---|---|---|
| Module name, then areas, then finish | New Module (name), then the Area Wizard | ✅ [new_module_and_area_through_the_wizards] |
| Area name, tileset list sorted by name, size presets Tiny/Small/Medium/Large and 2–32 per side | same | ✅ |
| Launch Area Properties / Open in Area Viewer | not yet (area properties and viewer: Phases 7 and 9) | ✗ |
| Files written (module.ifo, repute.fac, palettes, ARE/GIT/GIC) | identical apart from Aurora's random choices | ✅ (`mg-corpus-tests/tests/aurora_new.rs`, engine: `engine_new_module.rs`) |

## Import / Export (`TdlgImportExport`)

| Aurora | Moonglow | |
|---|---|---|
| Export chosen resources (+ dependencies) to `.erf` | Export Resources window | ✅ [export_then_import_into_another_module] |
| Import `.erf`, ask about overwriting | Import Resources window with overwrite choices, one undoable step | ✅ |

## Options (`TdlgOptions`)

| Aurora | Moonglow | |
|---|---|---|
| Game and user folders | Tools > Options | ✅ [options_choose_the_game_folder] |
| Script editor, area viewer, graphics options | not yet | ✗ |

## Faction Editor (`TdlgFactionEditor`, `TdlgFactionSelect`)

| Aurora | Moonglow | |
|---|---|---|
| Faction list | list, selection | ✅ |
| Add Faction: name, Global Effect (default on), parent among the standard factions after PC | same; reputations copied from the parent both ways, missing entries written as 100, exactly as Aurora writes the file | ✅ (`aurora_factions.rs`: identical file) [faction_editor_adds_and_removes_factions] |
| Remove Faction (not the standard five), with a confirmation | same, without the confirmation (undoable instead); later factions renumbered as Aurora does; refused while objects use the faction (Aurora's handling of such objects is not captured yet) | ✅ / ◐ |
| Change Faction Name | Change Name… | ✅ |
| Global Effect per faction | checkbox (custom factions) | ✅ |
| Basic: OpenGL chart of how the selected faction regards the others (drag bars to edit), Full Detail | sliders per faction with Aurora's colours, Full Detail adds how the others regard it | ✅ (sliders instead of a drawn chart) |
| Advanced: grid, row regards column, editable 0–100 | same grid, coloured, hover says hostile/neutral/friendly like Aurora's status bar | ✅ |
| Reputations in the engine | 36 creature pairs report `GetReputation` as written, after adding, editing and removing | ✅ (`engine_factions.rs`) |
| OK / Cancel | live edits, one undoable command each | — |

## Journal Editor (`TdlgJournalEditor`)

| Aurora | Moonglow | |
|---|---|---|
| Tree Root → categories → entries (`[0001] - text`) | same (`[0001] text`, "(finishes)" marked) | ✅ |
| Add: with Root a category, with a category an entry, disabled on an entry | same | ✅ [journal_editor_builds_what_aurora_builds] |
| New category: CategoryNNN name and tag, priority Lowest, XP 0, no picture; new entry: next ID, text EntryNNN | same; the file equals Aurora's | ✅ (`aurora_journal.rs`, and through the UI) |
| Copy, Cut, Paste, Delete | same (paste appends; IDs are kept) | ✅ |
| Category: Name + `…`, Tag (32), Priority (Highest…Lowest), XP, Comments | same | ✅ |
| Entry: ID, Finish Category, Text + `…` | same | ✅ |
| Apply / OK / Cancel | live edits, one undoable command each | — |
| In the engine | `GetJournalQuestExperience` returns each category's XP by tag | ✅ (`engine_journal.rs`) |

## Script Editor (`TdlgScriptEditor`, `TdlgScriptSearch`, `TSEditCodeCompletionList`)

| Aurora | Moonglow | |
|---|---|---|
| Syntax highlighting, line numbers | lexer highlighting (EE literals), line numbers, bookmarks marked; no wrapping | ✅ |
| Compile (F7: save and compile) | Compile button saves and compiles this script; F7 compiles all | ✅ / — (F7 keeps compiling all) |
| Compiler pane, double-click jumps to the line | Compiler tab; clicking a message goes to its line, in an include too (opened, or read-only for game scripts) | ✅ [compile_errors_go_to_their_line] |
| Functions / Variables / Constants lists with Filter; custom ones bold; hover shows the prototype; double-click inserts | same, from `nwscript.nss` and the script with its includes | ✅ |
| Templates list (script templates folder) | the `.txt` templates of the install's `data/scr` and the user's `scripttemplates` | ✅ |
| Help pane (doc comment) | Help tab: the selected symbol, or the one under the cursor | ✅ |
| Completion (F2), Enter to pick, `(` added for functions | F2 or Ctrl+Space, arrows, Enter/Tab, Esc | ✅ [script_editor_completion_find_bookmarks] |
| Find (Ctrl+F), Find Next (F3), Replace (Ctrl+R): match case, whole word, backward, prompt, replace all | same (no per-replacement prompt: Replace does one, Replace All all) | ✅ |
| Find In Files (open scripts / all module scripts) → Search Results | all module scripts (with unsaved editor text) → Search Results tab, click opens | ✅ |
| Bookmarks (F5 toggle; Ctrl+Shift+1…9 / Ctrl+1…9) | F5 toggle; Ctrl+Shift+1…9 sets, Ctrl+1…9 goes; the gutter shows ◆ or the number; Bookmarks tab | ✅ [script_editor_completion_find_bookmarks] |
| New (Ctrl+N), Save As (Ctrl+Alt+S), Save All, Close | Tools > New Script…, Save As…, module Save stores all edited scripts, tab close | ✅ |
| Open (all resources / module / hak) | resource browser and the module tree | ✅ |
| Options › Script Editor: font, colours of text, comment, directive, identifier, keyword, number, string, error, with a preview | font size and a colour per element (else the light or dark theme's), preview, reset; applied without reloading the game | ✅ (`options.rs` tests) |
| Options › Script Editor: templates folder, auto-compile, debug info (.ndb), external editor | not yet | ✗ |
| Print, indent with Tab | not yet | ✗ |
| Large scripts | nwscript.nss (13,869 lines): 0.4 ms per idle frame, ~10 ms per keystroke | ✅ (`editor_perf.rs`) |

## Conversation Editor (`TdlgConversationEditor`, `TfraConversationTree`)

| Aurora | Moonglow | |
|---|---|---|
| Tree: Root, NPC lines `[OWNER] - text` / `[tag] - text` (red), PC lines (blue), `[END DIALOGUE]`, `[CONTINUE]`, links grey | same; conditions marked `?` with the script on hover | ✅ |
| Add (with Root: a greeting; with a line: a line of the other kind), new conversation with `nw_walk_wp` end scripts | same; the file's fields, order and defaults as Aurora's | ✅ (`aurora_dialog.rs`) [conversation_editor_builds_and_links] |
| Input Text popup for a new line | the new line is selected and its Text field edited in place | — |
| Copy, Cut, Paste, Paste As Link, Delete | same; Delete removes the branch and every link to it, then renumbers | ✅ (`dialog.rs` tests) |
| Expand All, Collapse All; highlight filters (comments, actions, quests, animations, sounds) | same | ✅ |
| Speaker Tag (combo of module creature tags, Add) | text field | ◐ (creature tag list: with blueprints, Phase 8) |
| Text + `…` all languages; Insert Token (stringtokens.2da, highlight tokens) | same | ✅ |
| Text Appears When…: script, `…`, Edit, EE parameters (+/−), script preview | same | ✅ (engine: conditions in order with parameters, `engine_dialog.rs`) |
| Actions Taken: script, parameters, preview | same | ✅ (engine: the action runs with its parameter) |
| Other Actions: animation (Aurora's list), sound (`…`, Play), journal category and entry, Edit | animation list with Aurora's values, sound picker, journal and entry from the module's journal | ◐ (sound preview needs audio) |
| Comments (node comment; link comment on links) | same | ✅ |
| Current File: end scripts normal/aborted, Stop Camera Zoom In | same | ✅ |
| Status bar: line letters/words, file words | line letters and words, file words | ✅ |
| Bookmarks (list, double-click to go) | Bookmark toggles the selected line; the Bookmarks tab lists them, click selects | ✅ [conversation_search_bookmarks_and_test] |
| Search: find, replace, match case, whole word, current file / all files in module | Search tab: find (this conversation or all), results select the line or open the conversation, Replace All in this one | ✅ (language and gender scopes: English only) |
| Test (click through the conversation) | Test window: greetings, then each line's next lines, Back, Done; conditions not evaluated | ✅ |
| Script Wizard (Text Appears When…: abilities, class and level, gender, race, alignment, feats, skills, skill checks, items, local variables, random; Actions Taken: rewards, take gold/XP/items, set locals, attack or open a store, faction change; name sc_NNN/at_NNN, start the editor) | same pages and lists; Finish writes, compiles and sets the script in one undoable command | ✅ (`aurora_script_wizard.rs`: Aurora's five captured scripts byte for byte, all compile) [script_wizard_writes_compiles_and_sets_scripts] |
| Script Wizard output that does not work in EE: store with appraise checks plus a party reward (`nw_i0_tool` and `nw_i0_plot` both define `HasItem`: no compile), Take XP (`GiveXPToCreature` ignores negative amounts) | `nw_i0_plot` only, party XP as a loop; Take XP with `SetXP` | ✅ (improvement) |
| String local variables: "is equal to" only | also "is not equal to" | ✅ (improvement) |
| Spell Check (text popup menu) | not yet | — (in EE it finds nothing: no lexicon ships; "Helo thre wrold." passes. Moonglow could use Hunspell dictionaries: an improvement for later) |
| Text export/import (`actExportDialog`, `actImportDialog`, String/Char based) | not offered | — (hidden actions: no menu, button or shortcut reaches them) |
| Scrap tab of cut lines | the clipboard holds one copied or cut branch | ◐ |

## Model viewer (Aurora: blueprint previews; Moonglow: also any model from the resources)

| Aurora | Moonglow | |
|---|---|---|
| Preview window: orbit, zoom | Resources > open a model: orbit (left drag), pan (right/middle drag), zoom (wheel), Frame | ✅ [model_viewer_plays_animations] |
| Aurora's own lighting (no EE lighting) | the game's enhanced-lighting equations (area light, point lights, GGX specular, environment maps, fog) | ✅ improvement (`render.rs`: lighting matches a CPU evaluation; `client_render.rs`: reference scenes within 10/255 of the game client, light uniforms equal to the client's) |
| Animations (placeables, doors, creatures idle) | any animation of the model or its supermodels, play/pause; skins deform on the GPU with the stored bind poses | ✅ (`render.rs`: animated models; bind poses match the stored ones for 98% of bones) |
| Light nodes light the model (Aurora: no) | light nodes as point lights (colour, radius, multiplier, animated), converted as the game does (colours above 1 reach further); tile main lights take the area's colours and the engine's radii | ✅ [lights_convert_like_the_game] |
| Emitters | fountain, single and explosion emitters with flip-book textures, colour, alpha and size over life, camera-facing, upright, ground-aligned, motion-blur and linked quads, alpha, additive and punch-through blending; trail spawning; point-to-point (Bezier and gravity) to the reference child; lightning bolts; chunk models | ✅ (`render.rs`: a brazier's fire and smoke, a tile's lightning; the p2p, drag and lightning details follow the wiki's descriptions, not measurements) |
| Animated meshes (water, waterfalls, door skins), animated alpha and self-illumination | vertex and UV sets sampled `sampleperiod` apart and interpolated, normals recomputed; alpha and self-illumination keys | ✅ (`render.rs`: a water tile and a lamp's "on" animation) [mesh_animations_render] |
| Dangly meshes (hair, cloaks, pennants) | a damped spring per vertex within displacement × constraint; an approximation, the game's dynamics are undocumented | ◐ (`render.rs`: a creature's hair and coat lag and settle) [dangly_meshes_sway] |
| Normal, specular, roughness, height (parallax, occlusion) and self-illumination maps, from MDL `texture1`–`3` or an MTR (named by `materialname` or like the bitmap) with its parameters | as the game's normal-mapped shaders; tangent frames from screen-space derivatives | ✅ (`render.rs`: synthetic maps shade as expected; the base game ships almost none) [material_maps_shade] |
| Environment maps | TXI `envmaptexture`/`bumpyshinytexture` (`default`: the object's, then the area's, then `chrome1`), the object's own (appearance `ENVMAP`: its textures' alpha becomes reflectivity), 2D sphere maps and cube maps (TXI `cube 1`, faces `name0`…`name5`) | ✅ [environment_maps_reflect] |
| Blueprint previews (Aurora: creature, item, placeable and door previews in their editors and palettes) | Resources › Preview (and the blueprint editors in Phase 8): part-based creatures (skeleton, body or armour parts, robes hiding parts, head or helmet, cloak), single-model creatures, wings, tails, weapons and shields in hand, PLT colours, appearance environment maps, the idle animation; items (simple, layered, composite, armour on a body); placeables (their state's animation, light, reflection); doors | ✅ (`previews.rs`: base-game blueprints assemble and draw) [blueprint_previews_open] |

## Palettes (`TfraMainPalette`)

| Aurora | Moonglow | |
|---|---|---|
| Standard and Custom trees for creatures, doors, encounters, items, placeables, sounds, stores, triggers, waypoints (the game's `*palstd.itp`, the module's `*palcus.itp`) | Tools › Palettes (its own pane on the right): type, Standard or Custom, categories with their blueprints; creatures show their CR | ✅ [palette_edit_copy_and_delete] |
| Custom palettes regenerated from the module's blueprints (category by `PaletteID`, stores `ID`; 255 hidden; creatures with CR and faction) | rebuilt from the blueprints on every save, and live in the pane | ✅ (`palettes.rs`: Chapter 1's nine custom palettes rebuild entry for entry, in order) |
| Edit, Edit Copy, Delete (Del key), New | Edit (custom: opens the blueprint), Edit Copy (a copy as `<resref stem>NNN`, undoable), Delete (undoable), Preview | ◐ (New waits for the blueprint wizards; the copy's resref rule is not yet checked against Aurora) |
| Find Text (Ctrl+F), Find Next (F3) | Find: shows the blueprints whose name or resref matches, categories opened | ✅ |
| Update Instances | not yet (with the area editor, Phase 9) | ✗ |
| Selecting a blueprint arms placement in the area | with the area editor (Phase 9) | ✗ |
| In-place rename of custom blueprints | not yet (names are edited in the blueprint editors) | ✗ |

## Blueprint editors (object Properties dialogs)

Each blueprint opens in its own tab with the dialog's pages; every change
is one undoable command, integer fields keep their stored type (a missing
field takes the type the game's files give it), fields the editor does not
show are kept, and a stored value outside a field's range is shown as it is
(only edits are clamped). Aurora's OK/Cancel are the module's undo and save.
Shared parts: Name (`…` for all languages), Tag, palette Category, Blueprint
ResRef (renames the blueprint, a store's `ResRef` field), Variables…,
scripts (name, picker, Edit), Comments.

| Aurora | Moonglow | |
|---|---|---|
| Waypoint (`TdlgWaypointEdit`): Basic (appearance), Advanced (map note), Description, Comments | the same pages | ✅ [waypoint_editor_edits_and_renames] |
| Sound (`TdlgSoundEdit`): Basic (sound list add/remove/move, volume), Positioning, Advanced (hours, play style and order, intervals, variations), Comments | the same pages; no playback (Moonglow has no audio yet) | ◐ [sound_editor_lists_positions_and_times] (Play, and Priority from prioritygroups.2da, not yet) |
| Trigger (`TdlgTriggerEdit`): Basic (type), Area Transition (destination tag and type, Setup Area Transition, loading screen), Trap (`TfrmTrap`), Scripts, Advanced (faction, key, cursor, portrait, highlight height), Comments | the same pages; Setup Area Transition lists the tagged doors and waypoints in the module's areas | ✅ [trigger_editor_sets_the_type_and_trap] |
| Encounter (`TdlgEncounterEdit`): Basic (difficulty, spawn option, min/max creatures), Creature List (palette, CR, Unique), Scripts, Advanced (active, player only, faction, respawns), Comments | the same pages; the difficulty sets `DifficultyIndex` and its encdifficulty.2da `VALUE` | ✅ [encounter_editor_lists_creatures_and_respawns] |
| Store (`TdlgStoreEdit`): Basic (mark up/down, identify price, stolen goods, max buy price, gold), Advanced (scripts), Restrictions (will not / will only buy base items), Comments | the same pages | ✅ [store_editor_stocks_prices_and_restricts] (the amount a checked Will Identify / Max Buy Price / Limited Gold starts at is not yet checked against Aurora) |
| Store inventory (`TdlgInventory`, store mode): item palette, the five store pages, Infinite | an Inventory page: item palette, the pages (by the base item's `StorePanel`), items placed at the first free place in the 10-wide grid, Infinite, Remove | ◐ (a list, not Aurora's icon grid; drag between places not yet) |
| Door (`TdlgDoorEdit`): Basic (`TfraSituatedBasic`: appearance from doortypes.2da, Generic Appearance from genericdoors.2da, hit points, hardness, saves, plot), Lock (`TfraSituatedLock`), Trap, Area Transition, Scripts (`TfraSituatedScripts`), Advanced (`TfraSituatedAdvanced`: faction, conversation, portrait, initial state, No Interrupt), Description, Comments | the same pages; Generic Appearance writes `GenericType_New` (and the old `GenericType` where the blueprint has it); Hit Points sets `CurrentHP` too | ✅ [door_editor_sets_appearance_lock_and_transition] |
| Placeable (`TdlgPlaceableEdit`): Basic (appearance from placeables.2da, Static, Useable, Has Inventory), Lock, Trap, Scripts, Advanced (plus Treasure Model), Description, Comments; Inventory… | the same pages; Static disables Useable and Has Inventory; the inventory is a page like the store's | ✅ [placeable_editor_fills_its_inventory] |
| The 3D preview in the door and placeable dialogs | a Preview button (the model viewer, Phase 7) | ◐ |
| Portrait (`TdlgPortrait`, an image chooser) | a filtered list of portraits.2da base resrefs; older blueprints' `Portrait` resref is read and kept in step | ◐ (no images yet) |
| Load Script Set / Save Script Set | not yet | ✗ |
| Showing a page never changes the blueprint | every page of each editor, on blueprints with odd values | ✅ [showing_blueprint_editors_changes_nothing] |
| Update Instances | with the area editor (Phase 9) | ✗ |
| Item (`TdlgItemEdit`) General: name, tag, resref, base type, category, stack size (1 to the base item's `Stacking`), charges (0–250), additional cost, plot, stolen; Total Cost, weight, damage, critical, damage type, armor class and armor penalties, required level and Lore | the same; the cost is the engine's (every base-game item agrees with `nwserver`, `engine_item_cost.rs`), and every change stores it in `Cost` within the same undo step | ✅ [item_editor_adds_properties_and_keeps_the_cost] (Required Level and Lore read itemvalue.2da and skillvsitemcost.2da as described, not yet checked against Aurora) |
| Item Appearance: simple (icon grid), layered (+ colours), composite weapons (top/middle/bottom model and colour), armor (18 parts and robe, colour chooser) | model numbers from the models that exist (cloaks: cloakmodel.2da), weapon parts by shape and colour, armor parts from parts_*.2da, colours as numbers; EE part twins (`xModelPart1`, `xArmorPart_*`) read and written | ◐ (no icon grid or colour swatches yet; the Preview button shows the model) |
| Item Properties (`tvMaster`, `tvAssigned`, `TdlgPropEdit`): available by the base item (itemprops.2da), add, remove, edit subtype, cost value, parameter, chance; Identified, Undroppable | the same, the parameters edited in place under the assigned list | ✅ (`item_properties.rs`: every property of every base-game item is offered as the editor lists it) |
| Item Description: unidentified, identified, the base type's description, the property text | the same; Variables | ✅ |
| Creature (`TdlgCreatureEdit`) Basic: names, tag, race, appearance, phenotype, gender, description, CR, category, portrait, conversation, No Interrupt | the same (random names not yet) | ◐ [creature_editor_levels_and_aligns] |
| Creature Statistics: abilities with racial modifiers, totals and bonuses; natural AC and AC; saves (class tables, ability and feat modifiers, bonuses); base, bonus and total hit points; movement rate | the same; every change stores `MaxHitPoints` in the same undo step (`creature_stats.rs`: 1,526 of 1,582 base-game creatures agree with their stored maximum, the rest were stored before a later change) | ✅ |
| Creature Appearance: body parts, head, wings, tail, colour chooser | parts from the models that exist for the body (`p{gender}{race}{phenotype}_{part}`), EE part twins, wings, tail, colours as numbers | ◐ (no colour swatches; Preview shows the model) |
| Creature Classes: alignment presets, 8 classes with levels, package, Levelup Wizard | presets and both axes, up to 8 classes (add, change, remove), package | ◐ (no Levelup Wizard yet) |
| Creature Skills, Scripts, Advanced (treasure, decay, lootable, disarmable, plot, no permanent death, immortal, faction and Edit Factions, CR adjustment, subrace, deity, sound set, perception range, variables) | the same | ✅ |
| Creature Feats, Spells, Special Abilities, Template, Inventory; the CR calculation; Load/Save Script Set | not yet | ✗ |
