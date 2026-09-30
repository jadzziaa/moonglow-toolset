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
| Bookmarks (F5 toggle; Ctrl+Shift+1…9 / Ctrl+1…9) | F5 toggle, Bookmarks tab | ◐ (numbered bookmarks not yet) |
| New (Ctrl+N), Save As (Ctrl+Alt+S), Save All, Close | Tools > New Script…, Save As…, module Save stores all edited scripts, tab close | ✅ |
| Open (all resources / module / hak) | resource browser and the module tree | ✅ |
| Print, colour options, indent with Tab | not yet | ✗ |
| Large scripts | nwscript.nss (13,869 lines): 0.4 ms per idle frame, ~10 ms per keystroke | ✅ (`editor_perf.rs`) |
