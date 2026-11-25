---
type: Checklist
title: Parity checklists
description: Aurora's dialogs against Moonglow, control by control - done and tested, partly, not yet, or deliberately different with the reason - naming the test that covers each.
tags: [aurora, parity, checklist, ui]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-25T08:49:32Z }
sources:
  - id: inventory
    resource: aurora-ui-inventory.md
    title: Aurora Toolset UI feature inventory
---

# Parity checklists

Aurora's dialogs (from `aurora-ui-inventory.md`) against Moonglow, control by
control. ✅ done and tested, ◐ partly, ✗ not yet, — deliberately different
(with the reason). Tests named in brackets are in `crates/mg-ui/tests/app/`
unless their file (under `crates/`) follows the name.

## Module Properties (`TfrmIFOProp`)

| Aurora | Moonglow | |
|---|---|---|
| Basic: Name (primary language) + `…` all languages | Name field + `…` String Edit | ✅ [module_name_in_every_language_and_variables] |
| Basic: Tag | Tag field | ✅ [edit_undo_save_reopen] |
| Basic: Start Area, X/Y/Z (read-only; set by painting the start location) | read-only | ✅ (the start location is placed and dragged in the area's view) |
| Events: 21 module events with script picker `…` and Edit; OnModuleStart hidden | same order and labels, picker, Edit opens the script (module) or its read-only view (game); OnModuleStart shown only when set | ✅ |
| Advanced: start month/day/hour/year, minutes per hour, dawn/dusk hour | same, ranges 1–12, 1–31, 0–23, 0–30000, 1–240, 0–23 | ✅ (Aurora's dawn/dusk spin allows −99…99 in a byte field; Moonglow keeps 0–23) |
| Advanced: XP scale slider + number (0–200) | slider with number | ✅ |
| Advanced: Starting Movie (movies/*.bik) | list of `.bik` in the user's and the install's `movies/` | ✅ |
| Advanced: Variables `…` | Variables window (Advanced's Variables (N)…), and the same list on a Variables page of its own | ✅ [module_name_in_every_language_and_variables] |
| Description: memo + `…` all languages | multi-line field (line ends kept) + `…` | ✅ [multi_line_text_keeps_its_line_ends] |
| Custom Content: hak list, Add (from hak folders), Remove, Move Up, Move Down | same; changes apply when the module is reopened | ✅ |
| Custom Content: Check for Conflicts… (TdlgHakPak) | Hak Pak Conflict Analysis, a Bottom Dock tab (M4 U28): the haks read one by one as a job (its progress, Cancel stops it), then the conflicts, the overrides and the haks each a sortable grid, What the Module Uses, Copy Report (the text of `mg haks`) | ✅ [hak_conflicts_is_a_tab_with_views_sorting_keys_and_close, the_report_says_how_far_it_is_and_stops_when_called_off] |
| Custom Content: Custom Tlk File (tlk folder) | list of `.tlk` in the user's `tlk/`; the chosen table is loaded for StrRef lookups | ✅ |
| OK / Cancel | the same: every change applies as it is made, one undoable step, and Cancel sets back what the window changed since it was opened, as one step; at the right end of the window's header (M4 U9) | ✅ [the_editors_cancel_sets_back_what_they_changed, module_and_area_properties_list_their_pages_under_their_header] |
| The tabs Basic … Custom Content | the pages listed at the left with icons, one after another on one scroll (a page clicked comes to the top; M4 U9) | ✅ [module_and_area_properties_list_their_pages_under_their_header] |

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
| Launch Area Properties Dialog / Open Area in the Area Viewer (on by default) | the same, worded "Open Area Properties" (the owner's ruling, M4's U30 review) and "Open area in the Area Viewer" (the style guide's sentence case for check boxes) | ✅ [new_module_and_area_through_the_wizards, the_core_dialogs_hold_the_pattern] |
| Buttons Create and Cancel; the Enter key and Escape | a footer row at the dialog's foot, under a rule: **Create** in the accent fill (the main action) and **Cancel** at the right; Enter presses Create whenever it is enabled (a name and a tileset given; also from a text field), Escape is Cancel; the body has 12 points of padding; one page, no step list | ✅ [the_core_dialogs_hold_the_pattern] |
| Files written (module.ifo, repute.fac, palettes, ARE/GIT/GIC) | identical apart from Aurora's random choices | ✅ (`mg-corpus-tests/tests/aurora_new.rs`, engine: `engine_new_module.rs`) |

## Import / Export (`TdlgImportExport`)

| Aurora | Moonglow | |
|---|---|---|
| Export chosen resources (+ dependencies) to `.erf` | Export Resources window | ✅ [export_then_import_into_another_module] |
| (none: Aurora's export has each resource ticked by hand) | File › Export Changed Resources…: the Export Resources window with the resources changed since the module was opened checked (saved or not, by any route; `module.ifo` listed unchecked), and its **Select Changed** | — Moonglow's own (Twohand's ask) [export_changed_resources_ticks_what_changed_since_the_module_was_opened], [what_changed_since_the_module_was_opened] |
| Import `.erf`, ask about overwriting | Import Resources window with overwrite choices, one undoable step | ✅ |

## Options (`TdlgOptions`)

| Aurora | Moonglow | |
|---|---|---|
| Game and user folders | Tools > Options | ✅ [options_choose_the_game_folder] |
| Options as pages (Aurora's tree): Area, General, Script Editor, Conversation Editor, Spell Checking, Sounds, Language | pages Folders, Area, General, Script Editor, Conversation Editor, Sounds, Verify, Language, Keyboard, Updates | ✅ (no Spell Checking: see below) |
| OK, Cancel at the dialog's foot; the page tree at its left | the page list at the left, each page with its icon; OK and Cancel at the right end of the header (Enter and Escape as they do); Cancel and Escape set back every page's changes, OK writes the settings once (M4's U23) | ✅ [options_lists_its_pages_with_icons_and_ok_and_cancel_end_its_header], [options_ok_sets_the_settings_once_and_cancel_and_esc_set_every_page_back] |
| General as one column of checkboxes | General in three columns where wide (two or one where narrower, nothing cut), the labels of Open editors, Interface size, Graphics backend and the Questions in a column beside their controls (M4's U23) | ✅ [the_general_page_folds_its_columns_never_cut_never_taller_where_wider] |
| General: Build module on save; Minimize Toolset on test module | the same | ✅ [build_on_save_opens_problems_when_something_is_wrong]; minimizing is the window manager's (not tested) |
| General: Create backups of modules (on by default: the module as it was kept as `<name>.BackupMod`, Aurora's name) | the same | ✅ [saving_keeps_the_module_as_it_was_as_a_backup] |
| General: welcome dialog at startup; color depth and character set warnings | Moonglow's Welcome tab is its start page, not a dialog; the warnings are about Windows display settings | — |
| General: always open module directories (off: a `.mod` with a folder of its name beside it asks "A directory with the same name has been found. Do you want to open this directory as a module?", Yes the folder, No the `.mod`) | General: A .mod with its folder beside it (ask, the folder, or the `.mod`); the question in Aurora's words, Open Directory and Open .mod File; the folder saved into and written as the `.mod` too | ✅ [a_mod_with_its_folder_beside_it_asks_which_to_open] |
| General: reserved resref namespace, resource in hak and standard resource overwrite warnings | in the log when a command adds a blueprint named nw_, x0_ to x3_, or a script, conversation or blueprint a hak also has (the hak's wins) or the game has; each has its checkbox on Options › General | ✅ [a_script_named_as_the_game_s_own_is_warned_about], [warnings_are_on_by_default_and_apply (`mg-ui/src/options.rs`)] |
| General: invalid creature spell assignment warning (on closing a creature's properties: spells too high for the class level, too high for the casting ability, more than the class may have) | the same checks, shown on the creature's Spells page while editing, with Never warn again; the option on Options › General | ✅ (`aurora_spell_warnings.rs`: 31 creatures) [the_spells_page_warns_about_invalid_assignments] |
| General: creature inventory warning (notice on opening a creature's inventory); equipping an item without its feats asks to add the first | the notice on the Inventory page, with Never warn again; Equip asks the same (Yes adds the feat and equips, one undoable step; No leaves the slot) | ✅ (`aurora_inventory.rs`) [equipping_without_the_feat_asks_to_add_it] |
| General: use environment mapping on creatures and items | — (Moonglow's renderer always applies environment maps, as the game does) | — |
| Area: background color, encounter spawn point markers (height, width), door orientation arrows | Options › Area: a background color (default: the area's fog, as the game shows it; Aurora's is silver gray), a post over each spawn point (Height and Width as Aurora's, 12 and 4 tenths of a meter by default), a cyan arrow along each door's facing; also Moonglow's Middle drag moves the view (the area camera's two middle drags swapped) | ✅ [area_options_apply (`mg-ui/src/options.rs`)] |
| Area: undo levels | — (undo is unlimited) | — |
| Conversation Editor: Show speaker name before text, player and NPC text colors | the same | ✅ [conversation_lines_without_speaker_names_when_chosen] |
| Conversation Editor: Show popup when creating a new text entry (on), Paste Link Options (Link Destination To Source), Drag Link Options (Link Source To Destination), automatic backup every 5 minutes | the same options and defaults (`notes_dialog_options.md`); backups go to Moonglow's data folder's `conversation-backups/<module>/<name>.bak` (Aurora: the module's working folder) | ✅ [conversation_options_popup_link_directions_drag_and_backup], [conversation_link_and_backup_options_apply (`mg-ui/src/options.rs`)] |
| Language page | the language text fields show and edit (Use Default Language: English, or a chosen one); labels name it | ✅ |
| Sounds page: play placed sound objects (on), ambient sound (off) and ambient music (off) in the area; ambient music volume (92) | the same, with Aurora's defaults | ✅ [sound_options_apply (`mg-ui/src/options.rs`)] |
| Sounds page: 2D/3D bias, show listener position, 3D provider (hidden) | — (Moonglow's placed sounds fade with distance from the view's focus; no 3D panning) | — |
| Spell Checking page | — (in EE Aurora's spell check finds nothing: no lexicon ships; see the Conversation Editor) | — |

## Faction Editor (`TdlgFactionEditor`, `TdlgFactionSelect`)

| Aurora | Moonglow | |
|---|---|---|
| Faction list | the Outliner's factions (M4 U22): a click chooses, the arrows, Page Up and Down, Home and End move, a letter goes by name; and the matrix's row headers | ✅ [the_outliner_s_factions_choose_and_its_keys_move] |
| Add Faction: name, Global Effect (default on), parent among the standard factions after PC | same; reputations copied from the parent both ways, missing entries written as 100, exactly as Aurora writes the file | ✅ (`aurora_factions.rs`: identical file) [faction_editor_adds_and_removes_factions] |
| Remove Faction (not the standard five), with a confirmation | same, without the confirmation (undoable instead); later factions renumbered as Aurora does; refused while objects use the faction (Aurora's handling of such objects is not captured yet) | ✅ / ◐ |
| Change Faction Name | Change Name… | ✅ |
| Global Effect per faction | "Global effect" in the Right Dock's Faction panel (custom factions; M4 U22), beside a read-only Name and Parent | ✅ [the_faction_and_reaction_panels_change_in_one_step_each] |
| Basic: OpenGL chart of how the selected faction regards the others (drag bars to edit), Full Detail | Basic of the header's switch: sliders per faction with Aurora's colors, Full Detail adds how the others regard it (one Undo step as a slider is let go) | ✅ (sliders instead of a drawn chart) |
| Advanced: grid, row regards column, editable 0–100 | the matrix (M4 U22): row and column headers frozen, cells framed red where hostile and blue where friendly, the hover sentence like Aurora's status bar; a click chooses a cell, the arrows move, a number typed and Enter set it, a drag or a double click edits (one Undo step each); the chosen cell's Reaction panel (sentence, slider, value, the scale) and the Reactions panel (Full detail) in the Right Dock; only the cells in sight laid out | ✅ [the_matrix_is_chosen_by_a_click_and_its_keys_move_and_type], [a_dragged_cell_changes_in_one_step], [a_hundred_factions_are_read_once_and_drawn_in_sight] |
| Reputations in the engine | 36 creature pairs report `GetReputation` as written, after adding, editing and removing | ✅ (`engine_factions.rs`) |
| OK / Cancel | the same, over live edits (one undoable command each): Cancel sets back what the window changed, as one step | ✅ [the_editors_cancel_sets_back_what_they_changed] |

## Journal Editor (`TdlgJournalEditor`)

| Aurora | Moonglow | |
|---|---|---|
| Tree Root → categories → entries (`[0001] - text`) | same (`[0001] text`), an End badge on an entry that finishes, indent guides; names and texts from the talk table where they are StrRefs (M4 U22) | ✅ |
| Add: with Root a category, with a category an entry, disabled on an entry | same, but with an entry chosen Add adds an entry to its category (Moonglow's own, 2026-11-25: Aurora's is disabled there), and Delete chooses what takes the deleted one's place | ✅, — with an entry chosen [journal_editor_builds_what_aurora_builds] |
| New category: CategoryNNN name and tag, priority Lowest, XP 0, no picture; new entry: next ID, text EntryNNN | same; the file equals Aurora's | ✅ (`aurora_journal.rs`, and through the UI) |
| Copy, Cut, Paste, Delete | the header's icons, Expand All and Collapse All after them (M4 U22; paste appends; IDs are kept) | ✅ [expand_all_and_collapse_all_open_and_close_every_category] |
| Category: Name + `…`, Tag (32), Priority (Highest…Lowest), XP, Comments | the Right Dock's Category panel (M4 U22; the parent of the entry chosen); what is typed follows its category | ✅ [typed_text_follows_its_category] |
| Entry: ID, Finish Category, Text + `…` | the Right Dock's Entry panel (M4 U22; "Finish category", its subtitle "[0002] of Category000") | ✅ [the_journal_s_header_and_panels_follow_the_choice] |
| Apply / OK / Cancel | OK and Cancel, over live edits (one undoable command each): Cancel sets back what the window changed, as one step; no Apply, as every edit is applied as it is made | ✅ [the_editors_cancel_sets_back_what_they_changed] |
| In the engine | `GetJournalQuestExperience` returns each category's XP by tag | ✅ (`engine_journal.rs`) |

## Script Editor (`TdlgScriptEditor`, `TdlgScriptSearch`, `TSEditCodeCompletionList`)

| Aurora | Moonglow | |
|---|---|---|
| Syntax highlighting, line numbers | lexer highlighting (EE literals), line numbers, bookmarks marked, the cursor's line marked; no wrapping; lines 18 points apart at the 13-point text and the gutter as the Script board draws them (M5 W3) | ✅ [the_script_editor_has_the_board_s_line_pitch_and_gutter], [a_read_only_script_s_lines_are_the_editor_s_pitch_apart] |
| Compile (F7: save and compile) | Compile button saves and compiles this script; F7 compiles all | ✅ / — (F7 keeps compiling all) |
| Compiler pane, double-click jumps to the line | Compiler panel (the Bottom Dock's, or a floating script's window's bottom pane); clicking a message goes to its line, in an include too (opened, or read-only for game scripts) | ✅ [compile_errors_go_to_their_line] |
| Functions / Variables / Constants lists with Filter; custom ones bold; hover shows the prototype; double-click inserts | same, from `nwscript.nss` and the script with its includes, in the Reference panel (the Right Dock, or the floating script window's right pane; M4's U12): a segmented control over the filter; custom ones in the theme's text, the game's quieter | ✅ [reference_on_the_right_and_help_under_the_text_in_either_host] |
| Templates list (script templates folder) | the `.txt` templates of the install's `data/scr` and the user's `scripttemplates`, Reference's fourth list | ✅ |
| Help pane (doc comment) | the Help panel under the text as Aurora has it (5.2.0: a tab of the Bottom Dock, or of a script window's bottom pane), as wide as the text: the selected symbol, or the one under the cursor; its documentation comment and declaration verbatim, as the source has them, in the editor's font, not wrapped | ✅ [help_shows_the_comment_and_the_declaration_as_written] [help_takes_a_block_comment_as_documentation] [help_comes_forward_in_the_bottom_dock] |
| The toolbar (Compile, Save, Find…) | the script header: Save and Compile with their words, the rest as icons, To Scratch in More, the codepage, then OK and Cancel at its right end; everything in reach at every width (More) | ✅ [the_script_header_holds_every_control_from_420_to_1600] |
| Completion (F2), Enter to pick, `(` added for functions | F2 or Ctrl+Space, arrows, Enter/Tab, Esc | ✅ [script_editor_completion_find_bookmarks] |
| Find (Ctrl+F), Find Next (F3), Replace (Ctrl+R): match case, whole word, backward, prompt, replace all | same (no per-replacement prompt: Replace does one, Replace All all) | ✅ |
| Find In Files (open scripts / all module scripts) → Search Results | the scripts open now (Find In Currently Open Scripts) or all module scripts (with unsaved editor text) → the Bottom Dock's Search Results, click opens the line in its script | ✅ [a_search_s_hits_open_the_right_line_in_the_right_script] |
| Bookmarks (F5 toggle; Ctrl+Shift+1…9 / Ctrl+1…9) | F5 toggle; Ctrl+Shift+1…9 sets, Ctrl+1…9 goes; the gutter shows the bookmark icon or the number before the line's number; the Bookmarks panel (the Bottom Dock's) | ✅ [script_editor_completion_find_bookmarks] |
| New (Ctrl+N), Save As (Ctrl+Alt+S), Save All, Close | Tools > New Script…, Save As…, module Save stores all edited scripts, tab close (with unsaved text it asks Save, Don't Save or Cancel: the owner's ruling of 2026-11-25) | ✅ |
| Open (all resources / module / hak) | the script header's Open… (the Open Script window: the module's, its haks' or all scripts, several opened at once), the resource browser (the Left Dock's Resources panel) and the module tree | ✅ [open_script_opens_the_scripts_selected_together] |
| Options › Script Editor: font, colors of text, comment, directive, identifier, keyword, number, string, error, with a preview | font size and a color per element (else the light or dark theme's), preview, reset; applied without reloading the game | ✅ (`options.rs` tests) |
| Options › Script Editor: templates folder, auto-compile, debug info (.ndb), external editor | the templates folder (listed with `data/scr` and `scripttemplates`), compile on save, `.ndb` with each compiled script; font size and colors | ✅ [saving_a_script_compiles_it_with_debug_information_when_chosen]; the external editor: External Editor opens the script's file in the chosen program, and its saves come back into the editor [the_external_script_editor_s_saves_come_back] |
| Indent with Tab | Tab indents the lines a selection touches, Shift+Tab outdents them (or the cursor's line) | ✅ [tab_indents_selected_lines_in_the_script_editor] |
| Print | Print…: the script, highlighted in the editor's colors on white, as a page that opens in the web browser with its print dialog | ✅ [print_opens_a_highlighted_page_of_the_script] |
| Large scripts | nwscript.nss (13,869 lines) at 3440×1400: 0.9 ms a frame idle, 1.8 ms a key (2.3 at a typist's pace), 1 ms moving the cursor or the selection or scrolling; each line laid out and kept, a change laying out the lines it touched | ✅ (`editor_perf.rs`, held to 144 fps) [a_long_script_s_lines_out_of_sight_are_edited_where_they_are] |

## Conversation Editor (`TdlgConversationEditor`, `TfraConversationTree`)

| Aurora | Moonglow | |
|---|---|---|
| Tree: Root, NPC lines `[OWNER] - text` / `[tag] - text` (red), PC lines (blue), `[END DIALOGUE]`, `[CONTINUE]`, links gray | same; conditions marked `?` with the script on hover; each line cut with an ellipsis at the tree's right edge, its marks and `[END DIALOGUE]` kept after it, the whole line in its tip and its accessible name (M5 W4, the owner's ruling 29) | ✅ [a_long_line_is_cut_at_the_tree_s_right_edge, a_deep_line_keeps_its_least_room] |
| Add (with Root: a greeting; with a line: a line of the other kind), new conversation with `nw_walk_wp` end scripts | same; the file's fields, order and defaults as Aurora's | ✅ (`aurora_dialog.rs`) [conversation_editor_builds_and_links] |
| Input Text popup for a new line ("Enter what the NPC/player says next:", the placeholder selected; OK adds it, the parent staying selected; Cancel adds nothing) | the same; with the option off, the new line is selected and its Text field edited in place | ✅ [conversation_options_popup_link_directions_drag_and_backup] |
| Drag a line onto another: moves it with its branch; Ctrl+drag links them (direction from Options) | the same (Ctrl or Cmd); a line is not dropped into its own branch, nor a link onto Root | ✅ (`dialog.rs` tests) [conversation_options_popup_link_directions_drag_and_backup] |
| Toolbar: Add, Copy, Cut, Paste, Paste As Link, Delete, move up and down, Expand All, Collapse All, the highlight filters, Export, Import, Test | the editor's header (M4 U14, the Conversation board): Add with its word, the rest as icons, the five highlights a group of toggles, Scripts, Export, Import Lines…, Test, then OK and Cancel at its end (Close alone for a conversation only looked at); into More where the editor is narrow | ✅ [conversation_editor_builds_and_links, a_conversation_only_looked_at_is_read_only] |
| Copy, Cut, Paste, Paste As Link (direction from Options), Delete | same; Delete removes the branch and every link to it, then renumbers | ✅ (`dialog.rs` tests) |
| Expand All, Collapse All; highlight filters (comments, actions, quests, animations, sounds) | same | ✅ |
| The line's fields under the tree (`pnlData`) | the Right Dock's Node section, its Data tab (M4 U14), or a floating conversation's window's right pane: the sections Text, Text Appears When…, Actions Taken, Other Actions and Comments on one scroll | ✅ [the_line_s_fields_are_the_right_dock_s_or_the_window_s, the_panels_follow_the_conversation_in_front, every_conversation_panel_fits_the_right_dock] |
| Speaker Tag (combo of module creature tags, Add) | a text field and the list of the tags of the creatures placed in the module's areas | ✅ [speaker_tags_come_from_the_module_s_creatures] |
| Text + `…` all languages; Insert Token (stringtokens.2da, highlight tokens) | same | ✅ |
| Text Appears When…: script, `…`, Edit, EE parameters (+/−), script preview | same | ✅ (engine: conditions in order with parameters, `engine_dialog.rs`) |
| Actions Taken: script, parameters, preview | same | ✅ (engine: the action runs with its parameter) |
| Other Actions: animation (Aurora's list), sound (`…`, Play), journal category and entry, Edit | animation list with Aurora's values, sound picker and Play, journal and entry from the module's journal | ✅ |
| Comments (node comment; link comment on links) | same | ✅ |
| Current File: end scripts normal/aborted, Stop Camera Zoom In | the Conversation panel under the Node section (M4 U14), the file's word count in its header | ✅ [the_line_s_fields_are_the_right_dock_s_or_the_window_s] |
| Status bar: line letters/words, file words | over the line's Data: its letters and words (the talk table's text where that is shown: Aurora counts 0) and the file's words | ✅ improvement [a_talk_table_line_counts_the_text_shown] |
| Bookmarks (list, double-click to go) | Bookmark on the Node section's bar, Bookmark in a line's right-click menu (Aurora's), Toggle Bookmark's key F5 (Options › Keyboard; M5 W4, the owner's ruling 28) and the Command Palette's Toggle Bookmark toggle the selected line; the bookmarked lines are listed in the Outliner under the Left Dock's tabs (M4 U15; a floating conversation's at its window's left), a click or Enter going to the line, the rows following their lines | ✅ [the_outliner_lists_the_conversation_s_bookmarks, bookmarks_follow_a_line_moved_by_another_program, conversation_search_bookmarks_and_test, f5_toggles_the_line_s_bookmark, bookmark_in_the_row_s_menu_and_the_command_palette] |
| Search: find, replace, match case, whole word, current file / all files in module | the Node section's Search tab: find (this conversation or all), results select the line or open the conversation, Replace All in this one; Ctrl+F brings it to the front | ✅ (language and gender scopes: English only) [the_line_s_fields_are_the_right_dock_s_or_the_window_s] |
| Test (click through the conversation) | Test window: greetings, then each line's next lines, < Back, Done; conditions not run: each is taken to pass unless switched off in the window | ✅ |
| Script Wizard (Text Appears When…: abilities, class and level, gender, race, alignment, feats, skills, skill checks, items, local variables, random; Actions Taken: rewards, take gold/XP/items, set locals, attack or open a store, faction change; name sc_NNN/at_NNN, start the editor) | same pages and lists; Finish writes, compiles and sets the script in one undoable command | ✅ (`aurora_script_wizard.rs`: Aurora's five captured scripts byte for byte, all compile) [script_wizard_writes_compiles_and_sets_scripts] |
| Script Wizard output that does not work in EE: store with appraise checks plus a party reward (`nw_i0_tool` and `nw_i0_plot` both define `HasItem`: no compile), Take XP (`GiveXPToCreature` ignores negative amounts) | `nw_i0_plot` only, party XP as a loop; Take XP with `SetXP` | ✅ (improvement) |
| String local variables: "is equal to" only | also "is not equal to" | ✅ (improvement) |
| Spell Check (text popup menu) | not yet | — (in EE it finds nothing: no lexicon ships; "Helo thre wrold." passes. Moonglow could use Hunspell dictionaries: an improvement for later) |
| Text export/import (`actExportDialog`, `actImportDialog`, String/Char based) | not offered | — (hidden actions: no menu, button or shortcut reaches them) |
| Scrap tab of cut lines | the clipboard holds one copied or cut branch | ◐ |

## Model viewer (Aurora: blueprint previews; Moonglow: also any model from the resources)

| Aurora | Moonglow | |
|---|---|---|
| Preview window: orbit, zoom | Resources (the Left Dock's panel) > open a model: orbit (left drag), pan (right/middle drag), zoom (wheel), Frame | ✅ [model_viewer_plays_animations] |
| Aurora's own lighting (no EE lighting) | the game's enhanced-lighting equations (area light, point lights, GGX specular, environment maps, fog) | ✅ improvement (`render.rs`: lighting matches a CPU evaluation; `client_render.rs`: reference scenes within 10/255 of the game client, light uniforms equal to the client's) |
| Animations (placeables, doors, creatures idle) | any animation of the model or its supermodels, play/pause; skins deform on the GPU with the stored bind poses | ✅ (`render.rs`: animated models; bind poses match the stored ones for 98% of bones) |
| Light nodes light the model (Aurora: no) | light nodes as point lights (color, radius, multiplier, animated), converted as the game does (colors above 1 reach further); tile main lights take the area's colors and the engine's radii | ✅ [lights_convert_like_the_game (`mg-render/src/scene.rs`)] |
| Emitters | fountain, single and explosion emitters with flip-book textures, color, alpha and size over life, camera-facing, upright, ground-aligned, motion-blur and linked quads, alpha, additive and punch-through blending; trail spawning; point-to-point (Bezier and gravity) to the reference child; lightning bolts; chunk models | ✅ (`render.rs`: a brazier's fire and smoke, a tile's lightning; the p2p, drag and lightning details follow the wiki's descriptions, not measurements) |
| Animated meshes (water, waterfalls, door skins), animated alpha and self-illumination | vertex and UV sets sampled `sampleperiod` apart and interpolated, normals recomputed; alpha and self-illumination keys | ✅ (`render.rs`: a water tile and a lamp's "on" animation) [mesh_animations_render (`mg-render/tests/render.rs`)] |
| Dangly meshes (hair, cloaks, pennants) | a damped spring per vertex within displacement × constraint; an approximation, the game's dynamics are undocumented | ◐ (`render.rs`: a creature's hair and coat lag and settle) [dangly_meshes_sway (`mg-render/tests/render.rs`)] |
| Normal, specular, roughness, height (parallax, occlusion) and self-illumination maps, from MDL `texture1`–`3` or an MTR (named by `materialname` or like the bitmap) with its parameters | as the game's normal-mapped shaders; tangent frames from screen-space derivatives | ✅ (`render.rs`: synthetic maps shade as expected; the base game ships almost none) [material_maps_shade (`mg-render/tests/render.rs`)] |
| Environment maps | TXI `envmaptexture`/`bumpyshinytexture` (`default`: the object's, then the area's, then `chrome1`), the object's own (appearance `ENVMAP`: its textures' alpha becomes reflectivity), 2D sphere maps and cube maps (TXI `cube 1`, faces `name0`…`name5`) | ✅ [environment_maps_reflect (`mg-render/tests/render.rs`)] |
| Blueprint previews (Aurora: creature, item, placeable and door previews in their editors and palettes) | Resources › Preview (and the blueprint editors' Preview panel): part-based creatures (skeleton, body or armour parts, robes hiding parts, head or helmet, cloak), single-model creatures, wings, tails, weapons and shields in hand, PLT colors, appearance environment maps, the idle animation; items (simple, layered, composite, armour on a body); placeables (their state's animation, light, reflection); doors | ✅ (`previews.rs`: base-game blueprints assemble and draw) [blueprint_previews_open] |

## Palettes (`TfraMainPalette`)

| Aurora | Moonglow | |
|---|---|---|
| Standard and Custom trees for creatures, doors, encounters, items, placeables, sounds, stores, triggers, waypoints (the game's `*palstd.itp`, the module's `*palcus.itp`) | The Right Dock's Palette panel (Tools › Palettes, View › Right Dock; fixed in the dock, no longer a pane to drag; the dock drawn beside an area's tab, not beside another tab unless asked for by Tools › Palettes): type, Standard or Custom, categories with their blueprints; creatures show their CR; in the narrow dock the type a dropdown with Standard and Custom beside it, Expand All and Collapse All at the Group by row's end (M4 U6); with an area, the Properties panel under it (a placed object's Properties, the next section) | ✅ [palette_edit_copy_and_delete, tools_palettes_and_view_right_dock_show_and_fold_it, the_palette_fits_the_right_dock_from_its_least_width_to_600, the_right_dock_follows_the_active_editor, tools_palettes_beside_a_tab_that_brings_none, the_narrow_dock_has_three_tabs_and_the_narrow_row] |
| Custom palettes regenerated from the module's blueprints (category by `PaletteID`, stores `ID`; 255 hidden; creatures with CR and faction) | rebuilt from the blueprints on every save, and live in the panel | ✅ (`palettes.rs`: Chapter 1's nine custom palettes rebuild entry for entry, in order) |
| Edit, Edit Copy, Delete (Del key), New | the blueprint's right-click menu: New… (the type's wizard), Edit (custom: opens the blueprint), View (the game's: its pages to look at, unchanged), Edit Copy… (a window asks the copy's ResRef and Tag, offering a free ResRef named as Aurora names it: `nw_` dropped, numbered past the template's; Cancel takes the copy back), Delete (custom, undoable), Preview | ◐ (no Del key) [palette_edit_copy_and_delete, edit_copy_s_cancel_takes_the_copy_back], [an_edit_copy_is_numbered_from_the_template_s_number (`mg-module/src/palette_add.rs`)] |
| Find Text (Ctrl+F), Find Next (F3) | Find: shows the blueprints whose name, ResRef or tag has every word typed (else close matches, letters in order), categories opened; every match shown at once, so no Find Next | ✅ [palette_finds_by_tag_keeps_favorites_and_moves_between_categories] |
| Update Instances (Advanced page of every blueprint dialog but the waypoint's) | the same: every placed instance in every area made again from the blueprint where it stands (a trigger's or encounter's outline and spawn points kept); also on the palette's right-click menu (a blueprint, or a custom category's) and Edit › Update All Instances… | ✅ [update_instances_remakes_placed_sounds]; what Aurora keeps of an instance: [update_instances_matches_aurora (`mg-corpus-tests/tests/aurora_update_instances.rs`)] |
| Selecting a blueprint arms placement in the area | the same | ✅ [area_viewer_places_draws_boxes_and_turns] |
| In-place rename of custom blueprints | names are edited in the blueprint editors: in Aurora 89.8193.37 a second click on a custom palette entry opens an edit box, but nothing handles it (the trees have no OnEdited) and the name stays | — |

## Blueprint editors (object Properties dialogs)

Each blueprint opens in its own tab with the dialog's pages; every change
is one undoable command, integer fields keep their stored type (a missing
field takes the type the game's files give it), fields the editor does not
show are kept, and a stored value outside a field's range is shown as it is
(only edits are clamped). The window's OK and Cancel are Aurora's: Cancel
sets back what the window changed since it was opened, as one step
([a_properties_window_s_cancel_sets_back_what_it_changed]); they stand
at the right end of the window's header (the blueprint's kind, name and
ResRef), and the dialog's tabs are a list of the pages at the left with
icons, the pages one after another on one scroll, drawn only where in
sight, a page clicked in the list brought to the top (M4 U9;
[a_creature_s_pages_stand_on_one_scroll_beside_their_list,
the_keyboard_goes_down_the_pages_and_along_the_list,
blueprint_editor_pages_draw_nothing_over_anything_else]). A page's fields
stand in sections under headings (a creature's Basic: Identity, Race and
Appearance, Palette, Portrait, Conversation, its Description across the
page), flowing into columns of 440 points or more where the page has the
width; where the room holds four columns of 480, the pages stand side by
side in columns, every page of a creature in view at 3440 (M4 U10;
[a_creature_s_basic_page_stands_in_its_sections,
sections_flow_into_columns_by_the_width,
at_3440_every_page_of_a_creature_is_in_view_in_columns]). A name whose
text is the talk table's shows its StrRef inside its field.
A placed object's (or several of one type's) pages also show in the area
editor's Properties panel, the Right Dock's lower section, without OK or
Cancel: each change one Undo step (M4 U6, the UI redesign's S2;
[a_change_applies_at_once_and_undo_takes_it_back,
several_of_one_type_edit_together]); a double click on the object shows
the panel, and its Open in Tab and the object's menu's Properties open
the window ([a_double_click_shows_the_panel_and_open_in_tab_the_window]).
In the wide layout the panel shows every page on one scroll, each under
its heading, drawn only where in sight, its icon rail the index (M5 W2,
ruling 12; [the_wide_layout_s_pages_stand_on_one_scroll,
only_the_pages_in_sight_are_drawn_on_one_scroll,
tab_goes_down_the_pages_on_one_scroll]); narrower, one page at a time.
Shared parts: Name (`…` for all languages), Tag, palette Category, Blueprint
ResRef (renames the blueprint, a store's `ResRef` field), Variables (N)…,
scripts (name, picker, Edit), Comments. Beside the pages each row names,
every kind has a Variables page (the same list, edited in place), and
creatures, doors, items, placeables and triggers a Visuals page (what
EE's scripts can change about an object's look; Aurora has no fields
for it).

| Aurora | Moonglow | |
|---|---|---|
| Waypoint (`TdlgWaypointEdit`): Basic (appearance), Advanced (map note), Description, Comments | the same pages | ✅ [waypoint_editor_edits_and_renames] |
| Sound (`TdlgSoundEdit`): Basic (sound list add/remove/move, volume, Play, Stop), Positioning, Advanced (hours, play style and order, intervals, variations), Comments; Priority set from prioritygroups.2da | the same pages; Play plays the selected sound (else the first) at the blueprint's volume, Stop stops it (`mg-audio`: every game sound decodes, `audio.rs`); play styles write Aurora's `Looping`/`Continuous` (seamless looping 1/0, only with a single sound, in order), and play style or positioning sets the priority group (looping area-wide 2, positional 3; single shots 19 and 20) | ✅ (`aurora_sounds.rs`: six sounds as Aurora saved them) [sound_editor_lists_positions_and_times], [sound_blueprints_play_their_sounds], [sound_play_styles_set_aurora_s_fields_and_priority] |
| Trigger (`TdlgTriggerEdit`): Basic (type), Area Transition (destination tag and type, Setup Area Transition, loading screen), Trap (`TfrmTrap`), Scripts, Advanced (faction, key, cursor, portrait, highlight height), Comments | the same pages; Setup Area Transition: see [Setup Area Transition](#setup-area-transition-tdlgareatransition) | ✅ [trigger_editor_sets_the_type_and_trap, setup_area_transition_picks_on_the_target_area_s_map] |
| Encounter (`TdlgEncounterEdit`): Basic (difficulty, spawn option, min/max creatures), Creature List (palette, CR, Unique), Scripts, Advanced (active, player only, faction, respawns), Comments | the same pages; the difficulty sets `DifficultyIndex` and its encdifficulty.2da `VALUE` | ✅ [encounter_editor_lists_creatures_and_respawns] |
| Store (`TdlgStoreEdit`): Basic (mark up/down, identify price, stolen goods, max buy price, gold), Advanced (scripts), Restrictions (will not / will only buy base items), Comments | the same pages | ✅ [store_editor_stocks_prices_and_restricts] (the amount a checked Will Identify / Max Buy Price / Limited Gold starts at is not yet checked against Aurora) |
| Store inventory (`TdlgInventory`, store mode): item palette, the five store pages, Infinite | an Inventory page: item palette, the pages (by the base item's `StorePanel`), items placed at the first free place in the 10-wide grid, Infinite, Remove | ◐ (a list, not Aurora's icon grid; drag between places not yet) |
| Door (`TdlgDoorEdit`): Basic (`TfraSituatedBasic`: appearance from doortypes.2da, Generic Appearance from genericdoors.2da, hit points, hardness, saves, plot), Lock (`TfraSituatedLock`), Trap, Area Transition, Scripts (`TfraSituatedScripts`), Advanced (`TfraSituatedAdvanced`: faction, conversation, portrait, initial state, No Interrupt), Description, Comments | the same pages; Generic Appearance writes `GenericType_New` (and the old `GenericType` where the blueprint has it); Hit Points sets `CurrentHP` too; Setup Area Transition: see [Setup Area Transition](#setup-area-transition-tdlgareatransition) | ✅ [door_editor_sets_appearance_lock_and_transition]; Advanced's Conversation: Edit is New with the field empty, as on a creature (Aurora 89.8193.37's Edit there opens a new untitled000 and gives the object the name it is saved under, driven on the oracle 2026-11-24; Moonglow asks the name first) [a_placeable_makes_its_conversation_and_a_missing_one_is_offered] |
| Placeable (`TdlgPlaceableEdit`): Basic (appearance from placeables.2da, Static, Useable, Has Inventory), Lock, Trap, Scripts, Advanced (plus Treasure Model), Description, Comments; Inventory… | the same pages; Static disables Useable and Has inventory; the inventory is a page like the store's | ✅ [placeable_editor_fills_its_inventory]; Advanced's Conversation: Edit is New with the field empty, as on a creature (Aurora 89.8193.37's Edit there opens a new untitled000 and gives the object the name it is saved under, driven on the oracle 2026-11-24; Moonglow asks the name first) [a_placeable_makes_its_conversation_and_a_missing_one_is_offered] |
| The 3D preview in the door and placeable dialogs | the Preview panel beside the editor (M4's U11: the Right Dock, or the window's own pane where it floats), with Pop Out in its header's More; the Basic page's Preview button brings it | ✅ [a_placeable_s_and_a_door_s_model_is_their_preview_panel_s, the_preview_button_and_pop_out] |
| Portrait (`TdlgPortrait`: portraits.2da thumbnails, a large preview; Characters and Creatures, Placeable Objects and Doors or Plot Characters; race, gender and category filters; double-click or OK) | the portrait's picture beside a filtered list of portraits.2da base resrefs, and Portraits… opens the same Select Portrait window (pictures DDS over TGA, as the engine prefers); older blueprints' `Portrait` resref is kept in step | ✅ [portraits_are_chosen_from_their_pictures] |
| Load Script Set / Save Script Set | the same buttons under the scripts of creatures, doors, placeables, triggers, encounters, stores and areas: `.ini` files with a `[ResRefs]` section and Aurora's event names (`mg_module::script_set`, as the game's own `data/scr` sets); loading sets every event of the page (those the file leaves out to none) in one undoable step | ✅ [script_sets_save_and_load] (whether Aurora clears the events a set leaves out is not captured) |
| Showing a page never changes the blueprint | every page of each editor, on blueprints with odd values | ✅ [showing_blueprint_editors_changes_nothing] |
| Update Instances | see above | ✅ |
| Item (`TdlgItemEdit`) General: name, tag, resref, base type, category, stack size (1 to the base item's `Stacking`), charges (0–250), additional cost, plot, stolen; Total Cost, weight, damage, critical, damage type, armor class and armor penalties, required level and Lore | the same, but the stack size may go past the base item's `Stacking` (1 to 65,535, "more than the base item's N" said beside it; the game keeps the stack whole, `engine_ee_fields.rs`); the cost is the engine's (every base-game item agrees with `nwserver`, `engine_item_cost.rs`), and every change stores it in `Cost` within the same undo step | ✅ [item_editor_adds_properties_and_keeps_the_cost] (Required Level and Lore read itemvalue.2da and skillvsitemcost.2da as described, not yet checked against Aurora) |
| Item Appearance: simple (icon grid), layered (+ colors), composite weapons (top/middle/bottom model and color), armor (18 parts and robe, color chooser) | the item's inventory icon (simple and layered `i<class>_<n>`, PLTs in the item's colors; a weapon's three parts; an armor's pelvis, chest, belt, shoulders and robe), and Aurora's icon grid for simple and layered items, whose numbers are those with an icon (as Aurora); weapon parts by shape and color, armor parts from parts_*.2da, colors from their palettes' swatches (pal_cloth01, pal_leath01, pal_armor01/02: each color's tones; 176 to choose from); EE part twins (`xModelPart1`, `xArmorPart_*`) read and written | ✅ [items_show_their_icons_and_choose_appearances_by_icon], [colours_are_chosen_from_palette_swatches] |
| Item Properties (`tvMaster`, `tvAssigned`, `TdlgPropEdit`): available by the base item (itemprops.2da), add, remove, edit subtype, cost value, parameter, chance; Identified, Undroppable | the same, the parameters edited in place under the assigned list | ✅ (`item_properties.rs`: every property of every base-game item is offered as the editor lists it) |
| Item Description: unidentified, identified, the base type's description, the property text | the same; Variables | ✅ |
| Creature (`TdlgCreatureEdit`) Basic: names, tag, race, appearance, phenotype, gender, description, CR, category, portrait, conversation, No Interrupt | the same, Random names from the race's letter tables [creature_names_can_be_random] | ◐ [creature_editor_levels_and_aligns]; Conversation's Edit is New with the field empty (the builder El Grillo's request; Aurora 89.8193.37's Edit there opens a new untitled000 and gives the object the name it is saved under, driven on the oracle 2026-11-24; Moonglow asks the name first) [an_empty_conversation_field_makes_a_new_conversation_with_new, a_named_conversation_is_opened_with_edit] |
| Creature Statistics: abilities with racial modifiers, totals and bonuses; natural AC and AC; saves (class tables, ability and feat modifiers, bonuses); base, bonus and total hit points; movement rate | the same; every change stores `MaxHitPoints` in the same undo step (`creature_stats.rs`: 1,526 of 1,582 base-game creatures agree with their stored maximum, the rest were stored before a later change) | ✅ |
| Creature Appearance: body parts, head, wings, tail, color chooser | parts from the models that exist for the body (`p{gender}{race}{phenotype}_{part}`), EE part twins, wings, tail, skin, hair and tattoo colors from their palettes' swatches | ✅ (Preview shows the model) |
| Creature Classes: alignment presets, 8 classes with levels, package, Levelup Wizard | presets and both axes, up to 8 classes (add, change, remove), package | ✅ (the Levelup Wizard: see below) |
| Creature Skills, Scripts, Advanced (treasure, decay, lootable, disarmable, plot, no permanent death, immortal, faction and Edit Factions, CR adjustment, subrace, deity, sound set, perception range, variables) | the same, but for the faction and the sound set | ✅ |
| Creature Advanced: Faction with Edit Factions, Sound Set | on the Basic page: the Sound Set after the Gender, the Faction after the Category (builders' ask, 2026-11-16: chosen with the rest) | — [a_creature_s_sound_set_and_faction_are_on_its_basic_page] |
| Sound Set Select (`TdlgSoundSetSelect`): soundset.2da filtered by gender and type; a click plays a sample | the Sound Set list with the same gender (Both, Male, Female) and type (soundsettype.2da) filters; Play Sample (an icon beside the list) plays the set's Selected sound, else its first | ✅ [a_creature_s_sound_set_plays_a_sample] |
| Creature Feats (grid with filter, summary) | every feat with a name, a filter, Assigned only, a count | ✅ [creature_editor_lists] (the granted/available marks not yet) |
| Creature Spells: class radio, level filter, prepared counts or known spells, metamagic, summary, clear/save/load class spell list | the spellcasting classes (classes.2da `SpellCaster`), their spells (`SpellTableColumn`), prepared counts (`MemorizesSpells`) or known checkboxes, a level filter and summary; Meta-Magic (none, or a metamagic feat the creature has) for a class that prepares its spells, the spells then prepared with it at their raised level | ✅ [creature_editor_lists], [the_spells_page_prepares_with_the_metamagic_chosen]; Clear, Save and Load Class Spell List (`.ini`, `[Spells<class>]` as the game's `data/scr` lists; a prepared spell goes to its level raised by its metamagic) [class_spell_lists_save_clear_and_load] |
| Creature Special Abilities (spell, caster level) | add from the spell list, caster level, remove | ✅ [creature_editor_lists] |
| Creature inventory (`TdlgInventory`: equipment and natural equipment slots, backpack, Dropable, Pickpocketable, drag and drop) | an Inventory page: the item palette, the 18 slots (an item goes only where its base item's `EquipableSlots` allows) and the backpack at the first free place, each item with its icon; Aurora's Options under the palette: Selected Item, Dropable and Pickpocketable, there and grayed until an item is clicked (written as 1 when set, left out when not, as Aurora and the game's blueprints); Ctrl+click and Shift+click select several, set together in one step, a dash where they differ; a placed creature's held items the same | ✅ [creature_editor_lists] (Equip, and drag and drop between the palette, the slots and the backpack), [creature_items_can_be_dropable_and_pickpocketable], [a_placed_creature_s_items_can_be_dropable_and_pickpocketable], [an_item_aurora_equips_is_neither_dropable_nor_pickpocketable (`mg-corpus-tests/tests/aurora_inventory.rs`)], [the_engine_reads_a_creature_s_item_flags (`mg-corpus-tests/tests/engine_item_flags.rs`)] |
| The CR calculation (Basic and Advanced: Challenge Rating, CR Adjustment) | Aurora's calculation (`mg_rules::challenge`, `docs/research/notes_creature_cr.md`), recalculated with every change (as Aurora does on OK); the calculation shown on hover | ✅ [challenge_ratings_match_aurora (`mg-corpus-tests/tests/aurora_creatures.rs`): 4204 creatures] |
| Levelup Wizard (`TfrmCreatureLevelupWizard`: classes and levels; packages pick hit points, abilities, skills and feats) | the same window, from the Classes page and the area viewer's context menu; one undoable step (`mg_rules::levelup`) | ✅ [levelling_up_matches_aurora (`mg-corpus-tests/tests/aurora_levelup.rs`): 7 levellings, with a new class's memorized spells and package equipment; the_levelup_wizard_levels_a_creature_up_as_aurora_does, the_levelup_wizard_adds_a_class_with_its_spells_and_gear]; a new spontaneous caster's known spells not checked against Aurora |
| Creature Template tab (crtemplates.2da: Half-Dragon, Lich, Vampire, ...) | not shown: Aurora 89.8193.37's Creature Properties hide the tab (its form still has it) | — |

## Setup Area Transition (`TdlgAreaTransition`)

From a door's or an area transition trigger's Area Transition page. What
OK writes is held to Aurora's captures (`transition-triggers/`, 2026-11-12)
by `mg-corpus-tests/tests/aurora_transitions.rs`, and to the rules row by
row by `mg-edit/src/transition.rs`'s tests; the window's tests are in
`transition_setup.rs`.

| Aurora | Moonglow | |
|---|---|---|
| Target Area (`cbAreas`) | Target Area, the module's areas by name; opens on the destination set (a trigger by its `WP_` waypoint's tag), else the object's own area | ✅ [the_lists_hold_transition_triggers_and_two_of_one_tag_are_two, a_link_replaced_is_said_and_a_waypoint_used_again] |
| The area's map (`apArea`), a click picks | the minimap, doors as squares, area transition triggers as diamonds at the middle of their outline, waypoints as dots; a click picks, a double click is OK | ✅ [setup_area_transition_picks_on_the_target_area_s_map, the_lists_hold_transition_triggers_and_two_of_one_tag_are_two] |
| Available Doors / Triggers / Waypoints (`lbAreaDoors`, `lbAreaTriggers`, `lbAreaWaypoints`): every door and waypoint, every trigger of Type 1 (linked or not) but the current one, by tag; two of one tag two rows | the same, each row its tag and name; the row picked is the object linked; Destinations Shown: All, Doors, Waypoints, Triggers | ✅ [the_lists_hold_transition_triggers_and_two_of_one_tag_are_two] |
| Target Type (`rbTargetDoor`, `rbTargetWaypoint`, `rbTargetTrigger`) | the kind of the object picked (no separate choice) | — (one list each, the object picked says it) |
| Connection Type (`rbBothWays` default, `rbCurrentToTarget`, `rbTargetToCurrent`); a waypoint grays the two that link it | the same; grayed too for a blueprint or several objects edited together, and for an object without a tag, with the reason | ✅ [a_waypoint_takes_current_to_target_only, several_objects_take_current_to_target_only, a_blueprint_leads_to_a_trigger_s_waypoint, this_object_s_tag_is_weighed_where_it_names_a_link, one_way_connections_write_one_end, setup_area_transition_picks_on_the_target_area_s_map] |
| OK: links to a trigger name its new `WP_<tag>` waypoint (2, made at the middle of its box), to a door its tag (1), to a waypoint its tag (2) | the same structs, field by field (`aurora_transitions.rs`: eleven steps, the doors' one-way connections among them); one step to undo, both ends and the waypoints | ✅ [two_unlinked_triggers_are_linked_both_ways, doors_and_triggers_lead_to_each_other] |
| A `WP_<tag>` made on every run | one already inside the trigger's box used again | — (Differences from Aurora) [a_link_replaced_is_said_and_a_waypoint_used_again] |
| Target To Current: the target's half lost unless its area is saved later | written | — (Differences from Aurora) [one_way_connections_write_one_end] |
| The current's link written on its Properties' OK | written on the window's OK with the rest, which the Properties' Cancel leaves | — (Differences from Aurora) [two_unlinked_triggers_are_linked_both_ways, the_properties_cancel_leaves_the_transition_whole] |
| Questions: a tag still its blueprint's (current, target), the current area to be saved, a link to be replaced | lines under the lists (no save needed); a read-only area that the connection would change grays OK | — (Differences from Aurora) [a_link_replaced_is_said_and_a_waypoint_used_again, this_object_s_tag_is_weighed_where_it_names_a_link, a_read_only_target_area_takes_nothing, a_waypoint_made_with_a_tag_in_use_is_weighed] |

## Blueprint wizards (`TdlgBlueprintWizard` and each type's)

| Aurora | Moonglow | |
|---|---|---|
| Page order and buttons: a heading over each page, < Back, Next >, Finish and Cancel under it | a step list at the left ("Step N of M": a check on the steps done, the current one highlighted, those to come dimmer; a kind's pages are fixed, so none is off) and a page bar (an icon and the page's title) over each page in place of the heading; the footer row is **< Back**, **Next >** (the main action), **Finish**, **Cancel**, **Finish** the main action and **Next >** off on the last page; Enter presses the main action while it is enabled, Escape is Cancel; resizable (first 640 by 460); the Launch Properties Dialog check box is at the footer's left on the last page | ✅ [the_wizard_dialogs_hold_the_pattern, the_creature_wizard_s_step_list_follows_its_steps] |
| Door, Encounter, Item, Merchant, Placeable, Sound, Trigger and Waypoint Wizards (Wizards menu, palette New) | the same steps: the palette category (sorted by name, leaves only), the type's pages (waypoint tag and appearance; sound timing, positioning and waves; encounter creatures; item base type), the name ("<category> 001" by default), Launch Properties (on after the Sound Wizard only, as in Aurora) | ✅ [store_wizard_makes_aurora_s_store, sound_wizard_steps_through_timing_positioning_and_waves, item_wizard_makes_a_weapon_with_its_cost] |
| What they make: every field, its type, order and value; the resref (letters and digits, a trailing number kept within 16 characters, `blueprint` and a number for names with other characters) and the tag | `mg_module::blueprints` | ✅ (`aurora_blueprints.rs`: 15 blueprints made by Aurora's wizards, identical) |
| Trigger categories: Area Transition makes a transition, the trap strengths a trap of that strength (traps.2da DCs) | the same | ✅ |
| Sound Wizard's sound list filtered by category (`al_`/`as_` and a category name filter) | the picker opens filtered to `al_` or `as_` | ◐ |
| Creature Wizard (race, class and level, appearance, portrait, faction, name, category; abilities, feats, skills and equipment from the class package; the CR) | Wizards › Creature Wizard… and the creature palette's New…: the same pages; the creature from `mg_module::blueprints::creature` (levelled up from nothing by the packages) | ✅ [creature_wizard_matches_aurora (`mg-corpus-tests/tests/aurora_creature_wizard.rs`): 19 creatures of 17 racial types, every field and their order but three odd flags; the_creature_wizard_makes_aurora_s_creature]; random names from the race's letter tables [every_playable_race_has_random_names (`mg-corpus-tests/tests/creature_names.rs`)]; no 3D preview yet |
| Item Wizard's Magical, Item Level and Item Quality (Level and Quality enabled by Magical; Next waits for both) | the same on the Name page; random properties from the base item's itmwiz table, by tier, number and most worth as 127 items Aurora's wizard made show; charges, class limits, unidentified | ✅ [item_wizard_makes_a_magical_item_of_a_level_and_quality; `aurora_item_wizard.rs`: every captured item holds to the rules; `item_wizard.rs`: every base item, level and quality gives valid properties] (Aurora's draws are random: the same rules, not the same items) |

## Area viewer (`TfrmViewerArea`, the main frame's Object Filters and Preview toolbars)

| Aurora | Moonglow | |
|---|---|---|
| Every area opens and draws: tiles (heights, turns, main and source lights, animation loops, day and night animations), placed creatures, doors, items and placeables | `mg_area` model and scene | ✅ (`areas.rs`: all 1462 shipped areas open and render; the only objects without models are the data's own) |
| First view: straight down, north up, the whole area; numpad 5 returns to it | the same | ✅ |
| Object Filters: Show Creatures … Waypoints, Start Location, Show All, Show None | toggles in the area's header (a joined group with All and None on its arrow; under an editor 990 wide a Show popover with the count, "Show 5 of 10"), and each kind's eye in the Outliner under the Left Dock's tabs, the same state | ✅ [area_viewer_selects_moves_and_deletes] [the_eye_and_the_area_toolbar_s_show_toggle_are_one_state] [the_area_header_holds_every_control_from_420_to_1600] [the_area_header_yields_at_990_550_and_490] |
| Preview: Fog, Use Area Lighting; day or night | Fog (off at first, as Aurora's Scene › Fog; the header's Lighting popover), Night (the header's Day \| Night); the area's lighting always (the Lighting popover's switch turns it off, Moonglow's own) | ✅ the fog is the client's: it ends at the fog clip distance and starts the fog amount nearer than 30 m (`client_render.rs`, measured uniforms) |
| Sky (Aurora's view: none; the game: the area's skybox) | the skybox (skyboxes.2da, day or night as shown) around the camera, unlit, behind everything, with the game's horizon fade in the area's fog color: at ground level for every camera the game can have (at most 25 m from what it looks at), and under a view's camera farther off raised to stand as far under it as under the game's farthest at that pitch | ✅ improvement (`render.rs` [the_sky_is_behind_everything_beyond_the_far_plane_and_unfogged (`mg-render/tests/render.rs`)]; `client_render.rs` grass_and_sky_look: the horizon band in the client's color; sky_under_the_game_s_farthest_camera_look: the client's farthest camera fitted within the game's reach, where Moonglow's sky is unchanged; the sky beside the field 6 to 8 of 255 from the client's over the fit's last degree of pitch) [the_sky_under_a_camera_higher_than_the_game_s] |
| Grass (Aurora's view: none; the game: tufts on grass-material walkmesh faces, `[GRASS]` of the tileset, and the area's overrides, `SetAreaGrassOverride`) | 🌿 Grass in the area header's Lighting popover: tufts by face area × density on each material that grows grass (the tileset's on Grass unless turned off, the area's own from the GIT's AreaProperties), on the tiles within 30 m of the view's focus; Area Properties › Grass edits it, grown live | ✅ improvement (`client_render.rs` grass_overrides_look: mean color near the client's camera within 3 of the client's for default, sparse, dense, tall and red; the client's grass ends about 24 m from its camera and gathers in patches; `engine_grass.rs`: where the engine reads it) [grass_is_set_in_area_properties_and_grown_in_the_view] |
| Preview: Play Placed Sounds, Play Ambient Sound, Play Ambient Music (toolbar and View menu) | 🔊 Sounds, Ambient, Music in the area header's Audio popover (the Options › Sounds settings): the area's ambient sound and music (day or night as shown) looped, placed sounds heard from the view's focus (full within Max Volume Distance, fading out at Cutoff Distance, a linear approximation), in order or random, looping, repeating after their interval or once, by their hours | ✅ (`area_audio.rs` tests) [the_area_view_plays_the_area_s_sounds] |
| Display Grid | Grid (tile outlines at each tile's height; with Fog on it fades with the fog as the ground does, to 0.3 of itself past the fog's end, M5's V3), in the header's Overlays popover, whose switch hides the overlays together | ✅ [the_overlays_switch_hides_its_five_and_gives_them_back], [the_grid_fades_in_the_fog_down_to_its_least (`mg-render/tests/render.rs`)] |
| Reorient Camera, Go to Start Location | the same, as buttons under the compass at the view's top right (Reorient Camera a command too, Ctrl+Alt+R, in Quick Open) | ✅ [the_compass_s_buttons_run_their_commands, reorient_by_its_key_and_quick_open] |
| Camera (nwn.wiki's Area Editor page): Ctrl + drag moves, Ctrl + right or middle drag turns, the wheel zooms (Shift or Ctrl: slowly), numpad 4 6 8 2 / 7 9 1 3 / 5 | the same; also a right or middle drag turns, Alt + middle drag moves (the two middle drags swapped by Options › Area › Middle drag moves the view), Alt + drag moves where Alt raises no selection (nothing selected, a read-only area, Paint, Select Tiles, a terrain tool), the arrow keys and W A S D move | ◐ no camera pad [alt_and_a_drag_moves_the_camera_or_raises_the_selection, a_middle_drag_turns_and_moves_the_camera_as_before, a_right_drag_turns_the_camera_and_w_a_s_d_fly_while_held] |
| Select: click, drag a box; move by dragging, Shift + right drag turns, Alt + drag raises | the same; also Ctrl + click adds or removes; each change one undoable command writing only the fields that change | ✅ [area_viewer_places_draws_boxes_and_turns] |
| Objects stand on the walkmesh: creatures always; others keep their height when moved; sounds are placed 1.5 m up | tiles' `.wok` (else the model's walkmesh); an object moved keeps its height above the ground, creatures stay on it | ✅ (`areas.rs`: 97.5% of shipped creatures, 96% of waypoints and stores stand on the computed ground; 72% of sounds exactly 1.5 m above it) |
| Delete (key and context menu) | the same | ✅ |
| Place from the palette: click (Shift + click places more; right click or Escape stops); triggers and encounters drawn point by point, a double click closing them | the same (the palette's chosen blueprint) | ✅ |
| What placing writes: the blueprint read into Aurora's object (defaults for missing fields, EE forms of old fields, parts for part-based creatures, twins), equipment and inventories in full, skill list, sound priority, store pages, encounter creatures by CR, Aurora's field order; no spawn point for encounters | `mg_module::instances` | ✅ (`aurora_instances.rs`: 15 instances placed by Aurora identical; `engine_blueprints.rs`: the engine loads them with what they hold) |
| Doors: placed on tile door hooks, turned as the hook (of the tile under the pointer); nowhere else | on the hook within 4 m, turned as the hook (of the tile under the pointer), as Aurora; with no hook in reach, or with Alt held, freely where clicked or dropped, turned by Q and E as a placeable; a placed door dragged within reach of a hook snaps to it, Alt keeps it free; the hooks show while a door is chosen, the one it would go on bright; a tile change takes only the doors standing on its hooks | — also anywhere (a builder's ask: Aurora's way round was to place on a hook and move it with Adjust Location) [doors_go_on_hooks_in_reach_and_freely_elsewhere, a_placed_door_snaps_to_hooks_in_reach_and_alt_keeps_it_free]; `areas.rs`: 97.1% of shipped doors stand on a computed hook; `aurora_instances.rs`: Aurora's doors on hooks |
| Waypoints, sounds, merchants: shown as models (waypoints and merchants yellow arrows) | waypoints and merchants yellow arrows along their facing; sounds boxes | ◐ |
| Triggers and encounters: colored outlines | outlines: encounters orange; triggers green, area transitions blue, traps red | ✅ |
| Moving an encounter moves its spawn points | they move with it | ? not checked against Aurora |
| Properties of a placed object (double click, context menu): the blueprint's dialog without its palette fields | the blueprint editors on the object's GIT entry (Blueprint ResRef shown read-only, no Category or Comments); cost and hit points kept up to date as for blueprints | ✅ [placed_objects_open_their_properties]; inventories of placed objects hold whole items as Aurora's do [placed_chest_holds_whole_items] |
| Area Properties (`TdlgAreaProperties`): Basic, Visual (lighting schemes; Customize Environment's colors, fog, shadows, day and night, fog clip, sky box, weather), Audio, Events, Advanced, Comments | a tab with the same pages (the environment inline on Visual), and Grass and Variables pages of Moonglow's; a scheme also re-picks the tiles' lights, as Aurora warns | ◐ [area_properties_edit_the_area]; Load/Save Script Set as above; no Load Default; ResRef not editable here (an area is renamed with Rename… in the module tree) |
| Adjust Location (`TdlgLocation`): position, bearing (0 north, counter-clockwise) and the EE visual transform (scale, X/Y/Z rotation, X/Y/Z translation) of the selected objects | the same; what is changed is set on each selected object, one command; the viewer draws and picks models with their visual transforms (also the older `VisualTransform`) | ✅ (`aurora_palette_add.rs`: the same `VisTransformList` and bearing as Aurora's) [adjust_location_and_find_instance]; no bearing dial |
| Find Instance (`TdlgFindInstance`): types, area, blueprint, tag; results (Type, Tag, Area, Template), double click goes to it | Edit › Find Instance…, the same | ✅ [adjust_location_and_find_instance] |
| Copy, Cut, Paste (Ctrl+C, X, V; and Ctrl+Insert, Shift+Delete, Shift+Insert, the classic keys Aurora takes as every Windows program does): the copies follow the pointer, a click places them (keeping their places around each other), a right click cancels; between areas | the same, with both sets of keys (everywhere Ctrl+C, X and V work, text fields included); copies keep their height above the ground (creatures on it) | ✅ [copy_cut_and_paste_objects], [insert_and_delete_keys_copy_cut_and_paste_objects], [insert_and_delete_keys_copy_cut_and_paste_text] |
| Context menu: Initial State (doors: Opened Forward, Opened Backward, Closed; placeables: Default … Deactivated), Reverse Door, Mute / Turn On, Redraw Polygon, Add Spawn Point, Variables | the same (on every selected object of the type); doors and placeables show their state | ✅ [context_menu_sets_states_mutes_and_adds_spawn_points] |
| Add to Palette: a new custom blueprint from the object (and one per item it holds), named as Aurora names it (`nw_` dropped; past the template's number and those in use, one more), keeping the original blueprint's category and comment; the object then names it; Aurora's properties dialogs for each | the same in one command; the new blueprint's editor opens (its items' do not) | ✅ (`aurora_palette_add.rs`: the chest, its two items and the placed chest identical) [add_to_palette_create_waypoint_and_set] |
| Create Waypoint (creatures): a walk waypoint `WP_<tag>_NN` where the menu was opened; Create Set (waypoints): renamed `<name>_NN` | the same | ✅ (`aurora_palette_add.rs`) [add_to_palette_create_waypoint_and_set] |
| Properties of several selected doors or placeables (`TdlgSituatedMultiEditor`): common fields; blank = unchanged | for several objects of any one type: the type's pages, shown as the first; each field changed is set on all (one command) | ✅ [several_objects_edited_together] |
| Preview window (`TfrmPreview`): the chosen blueprint in 3D (items also 2D), name, tag, resref, comments and fields of its type | 👁 The Right Dock's Preview panel, on the tab beside the palette's: the same, items with their inventory icon, stores with their stock by page; drawn only while its tab is in front | ✅ [the_preview_panel_shows_the_chosen_blueprint, the_preview_panel_shows_a_store_s_stock, each_tab_shows_its_panel_and_only_it] |
| Context menu: Conversation (opens the object's), Inventory (its Properties at the inventory page) | the same | ✅ [context_menu_sets_states_mutes_and_adds_spawn_points] |
| Context menu: Setup Store (a creature or placeable: the Store Setup Wizard's conversation, open-store script, store placed where the shopkeeper stands, Merchant faction for a hostile one) | Setup Store…: the same pages, defaults and files (`mg_module::store_setup`); the store chosen from a list of the game's or the module's stores rather than a palette tree | ✅ [store_setup_matches_aurora (`mg-corpus-tests/tests/aurora_store_setup.rs`), setup_store_makes_the_conversation_script_and_store] |
| Context menu: Add Popup Text (a placeable: a conversation of one line, the placeable's own) | Add Popup Text…: the same conversation and assignment, one undoable step | ✅ [popup_text_matches_aurora (`mg-corpus-tests/tests/aurora_store_setup.rs`), add_popup_text_gives_a_placeable_a_one_line_conversation]; Aurora 89.8193.37 loses the assignment unless something else changes the area before saving: Moonglow keeps it |
| Context menu: Levelup Wizard | see the creature's Levelup Wizard | ✅ |
| Terrain palette (Features, Groups, Terrain: terrains, crossers, Eraser, Raise/Lower), sorted by name | Palettes › Tiles: the area's tileset palette, the same entries and order | ✅ (`aurora_terrain.rs`: four tilesets' Terrain branches as Aurora lists them) |
| Terrain brushes: a corner's terrain, the primary rules on its neighbours; Raise/Lower (right click lowers); only the touched tiles change; a stroke nothing fits does nothing | the same; the cursor (the four tiles around the corner) turns red where the stroke would do nothing | ✅ (`aurora_terrain.rs`: 62 steps give Aurora's corners, heights and crossers) [area_viewer_paints_terrain] |
| Crossers dragged across tiles (roads, streams, walls, corridors…), or clicked on one edge (a doorway); bridges where they cross | the same: the crosser goes on the edge of every quarter of a tile the drag passes, or of the quarter clicked; the cursor red where nothing fits; what no tile fits passed over and the rest laid | ✅ (`aurora_terrain.rs`: tic01's clicks, six drags partly refused) |
| Eraser: the tile under the pointer chosen again; its crossers go | the same (crossers that then fit nothing go too, as Aurora's) | ✅ (`aurora_terrain.rs`) |
| Groups and features: a ghost under the pointer (its first tile there), right click turns it; its doors come with it | the same | ✅ (`aurora_terrain.rs`: eight placements and the doors, field for field; `engine_terrain.rs`) |
| Select Terrain: tiles by click and box; Delete; Shift + right click: next variant; Tile Properties (main and source lights from lightcolor.2da, animation loops, Defaults) | Select Tiles (the tool shelf's, F10): the same, its commands in the area header; Tile Properties the Right Dock's lower section, no OK or Cancel, each change one step to undo on every tile chosen (M4's U5) | ◐ [area_viewer_selects_tiles_and_sets_their_properties], [a_change_is_one_step_undone_on_every_tile], [an_unchanged_control_writes_nothing]; Defaults' lights not checked against Aurora indoors; Ctrl + C, X, V copy, cut and paste tiles with their lights (pasted as a group goes in) [tiles_copy_and_paste] |
| Edit › Resize Area (rows and columns, Tiny/Small/Medium/Large), Rotate Area (90/180/270 either way) | the same | ✅ (`aurora_terrain.rs`: grown, shrunk through groups, rotated as Aurora does; objects outside deleted with Aurora's warning) [resize_and_rotate_area_from_the_edit_menu] |
| The main frame's status bar (`sbFrame`, two panels) | the Status Bar: the area view's state's hints as key caps and a word (selected, Shift held, a drag, each tool in hand, with a note of what is in hand or why a step was refused), the area's counts (its size, its objects, what is selected) at its right; they were a line of words under the view's toolbar until M3's T12, which the header (M4's U3) replaced | ✅ improvement [each_area_state_shows_its_hints] |
| Render AABB Nodes (walkmesh); Build › Area Statistics (Resources Used) | Ground walkmesh and Object walkmeshes in the header's Overlays popover (walkable faces green); Build › Area Statistics (tiles, objects, models, triangles, memory, textures) | ✅ [walkmesh_overlay_and_area_statistics] |

## Build menu (`TdlgVerifyModule`, Test Module)

| Aurora | Moonglow | |
|---|---|---|
| Build Module: Unused, Missing Resources, Compile, Spell Check; Advanced Controls (Compile: Scripts, Creature CR, Encounters, Palettes; Missing Resources by type; Unused: Scripts, Conversations, Blueprints; Spell Check by type), Build, results ("No errors found"; double click opens), export | Build › Build Module…: the same options, layout and defaults; the compile passes as one undoable command; the results in the Problems panel and what each pass did in the log | ◐ [build_module_compiles_and_reports]; Creature CR as Aurora's [challenge_ratings_match_aurora (`mg-corpus-tests/tests/aurora_creatures.rs`)]; no Spell Check |
| Encounters pass: each encounter's creature entries (blueprints and placed encounters) given the creature blueprints' CR and appearance; entries whose creature does not exist removed | `mg_module::build::compile_encounters` (after the Creature CR pass, as Aurora) | ✅ [encounters_are_compiled_as_aurora_does (`mg-corpus-tests/tests/aurora_encounters.rs`)] |
| Test Module (F9): saves, then `nwmain -userdirectory <dir> +TestNewModule <module>` | the same; the module must be in the user directory's `modules` folder | ◐ (the command is tested, not run: the game only runs here in the sandbox) |

## Main menus, the Plot Wizard and the item generators (`mmFrame`, `TdlgPlotWizard`, `TdlgItemGeneratorEdit`)

| Aurora | Moonglow | |
|---|---|---|
| Main menu: File, Edit, View, Wizards, Tools, Build, Help, with the area's Edit and Scene menus merged in | File, Edit, View, Wizards, Tools, Build, Plugins, Help, by the keyboard too (the manual's chapter 2 lists them); an area's commands are on its header (its popovers and More) and its context menu | ✅ item by item below, against the inventory's `mmFrame` and `AreaMenu`; the keys Aurora has and Moonglow lacks (Plot Wizard's, Refresh's) are settled in the deferred list |
| Plot Wizard (Wizards menu, Ctrl+Alt+P) | not built | ✗ |
| Item generators (Item Generator Editor, Random Item Generator Chooser; hidden in Aurora: the Random Item Templates button is never shown) | not offered | — |

### The main menus item by item

Aurora's items as `aurora-ui-inventory.md` has them (`mmFrame`, and the
area viewer's `AreaMenu` merged into it), its keys in brackets, against
where Moonglow has each (`crates/mg-ui/src/commands.rs` `MENUS` and
`TOP_BAR`, `keys.rs`; the area header is the manual's chapter 4).
Items Aurora hides are left out but for Save To Savegame and the Module
Wizard.

| Aurora | Moonglow | |
|---|---|---|
| File › New (Ctrl+N), Open (Ctrl+O), Close, Save (Ctrl+S), Save As | File › New Module…, Open Module…, Close, Save, Save As…, with Aurora's keys; also Open Folder…, Recent Modules, Save As nasher Project… | ✅ |
| File › Save To Savegame (hidden, disabled) | none | — Aurora hides it |
| File › Import…, Export… | File › Import…, Export… (and Import Files…, Import Folder…, Export Changed Resources…, Import Conversation…) | ✅ |
| File › Exit (Alt+X) | File › Exit (Alt+X) | ✅ [closing_the_window_asks_first] |
| Edit › Undo (Ctrl+Z), Redo (Ctrl+Shift+Z) | Edit › Undo, Redo, each naming its step; the same keys, and Ctrl+Y | ✅ |
| Edit › Copy (Ctrl+C), Cut (Ctrl+X), Paste (Ctrl+V), the area's too | the same keys wherever there is something to copy (and Ctrl+Insert, Shift+Delete, Shift+Insert); in the area view's and the module tree's context menus, not the Edit menu | ✅ |
| Edit › Resize Area, Rotate Area | Edit › Resize Area…, Rotate Area… | ✅ |
| Edit › Find Instance | Edit › Find Instance… | ✅ |
| Edit › Module Properties | Edit › Module Properties (and Module Properties on the Module panel's tab bar) | ✅ |
| Edit › Area Properties (the area's Edit too) | the area header's Area Properties and the module tree's context menu | ✅ |
| View › Selection Mode › Select Terrain, Select Objects | the area view's Select Tiles (F10, Aurora's Select Terrain's key) | ✅ |
| View › Toolbars › File, Display, Selection Mode, Object Filters, Preview | the Top Bar (the menus, New, Open, Save, Undo, Redo, Quick Open, Compile, Verify, Test Module) and the area header, always shown; the editors' one-row headers, their icon buttons named in their tips and for a screen reader (Options' Toolbar buttons, Icons only, retired in M4's U23: an old settings file's setting is read and dropped) | — |
| View › Interface Panels › Message Log, Module Contents, Palettes | View › Log, Left Dock, Right Dock (each checked while shown), Hide All Panels, Reset Layout; the module tree is the Left Dock's Module panel (its tab bar: Module Properties, New Area, Expand All, Collapse All), beside its Resources panel, the Outliner under them while an area is in front (its placed objects by kind, a seam between), and View › Left Dock (Ctrl+Alt+1) folds the dock on either; the palettes are the Right Dock's Palette panel, which View › Right Dock (Ctrl+Alt+2) folds, with the Preview panel on the tab beside theirs (Tools › Preview); the log is the Bottom Dock's Log panel (Copy Log, Save Log, Clear Log on its tab bar), opened and closed by the Log switch in the Status Bar, which shows the log's last line while it is closed (pressed, it opens the log); a new installation starts with it closed; in a narrow window the side docks fold by themselves (the Left Dock under 1500 points, the Right Dock under 796, 280 wide under 1400) without changing what View says you chose, and come back by hand there; in a window 2560 or wider the Left Dock is 320 and the Right Dock 741 in two columns (the Palette and Preview tabs, then Properties) where not chosen (M3 T14) | ✅ improvement [what_the_shell_shows_at_each_width] [the_wide_dock_s_columns_by_the_keys_and_a_chosen_blueprint] [a_window_narrowed_and_widened_writes_nothing_and_gives_all_back] |
| View › Interface Panels › Toolbar, Visual Camera Controls | none: the camera's keys and the mouse (the manual's chapter 4) | — |
| View › Object Filters › Show All, Show None, Show Creatures… Show Waypoints, Go to Start Location | the area header's Show toggles (the Show popover where the header is compact): each kind, Start, All, None, as in Aurora for every area view; View › Go to Start Location | ✅ |
| View › Full Screen (F11) | Full Screen (F11), from the key and Quick Open | ✅ |
| The main frame's area windows, one for each area open, side by side within it | the editor area's document tabs (each with its kind's icon, its name, a dot where it has unsaved changes and a close button) in editor groups side by side: View › Split Editor (Ctrl+\\, and the button at the right end of the focused group's tab bar) moves the tab in front to another editor group (a new one at its right where there is one); a tab dragged to a group's side makes one; the groups are kept with the module's open tabs | ✅ [split_editor_makes_a_second_group_from_the_front_tab, the_modified_dot_follows_each_tab_s_own_unsaved_changes, editor_groups_are_kept_and_come_back_from_their_places_alone] |
| The main frame's other windows (each Properties, script and conversation in a window of its own within it; Escape as Cancel) | Options › General › Open editors: As Tabs (a tab of the focused editor group; a new installation's) or As Floating Windows (a window of its own over the area); areas always as tabs; a kind moved opens where it was put, until View › Reset Layout; Escape is the Cancel of a window, or of a docked tab, with OK and Cancel; a window has a border and a shadow, a bar of its collapse arrow, tabs, Maximize and Close Window, and a grip in its corner; a dialog's title is at the left of a title row it is moved by, with a Close where Escape closes it | ✅ [editors_open_as_tabs_or_as_windows_as_the_switch_says, a_kind_moved_keeps_its_place_until_reset_layout, escape_cancels_a_docked_tab_with_ok_and_cancel, a_floating_window_s_bar_collapses_maximizes_resizes_and_closes, a_dialog_drags_by_its_title_row_and_not_by_its_body] |
| View › Show Preview Window | Tools › Preview (beside Tools › Palettes: the Right Dock on its Preview tab; the folded dock's Preview button; a key set in Options › Keyboard, by the command's id of before) | ✅ [tools_preview_and_the_strip_show_the_preview] |
| Environment (the area's Scene) › Refresh (F5) | none: the view is drawn as it changes; Tools › Reload Resources reads changed haks and folders | — (its key is settled) |
| Environment › Display Grid, Display Shadows, Fog, Use Area Lighting | the area header: Grid in its Overlays popover; Shadows (with three kinds), Fog and Lighting in its Lighting popover | ✅ |
| Environment › Fade Geometry › Never, Object Mode Only, Always | the area header's Lighting popover: Fade Geometry, the same three | ✅ |
| Environment › Render AABB Nodes | the area header's Overlays popover: Ground walkmesh and Object walkmeshes | ✅ |
| Environment › Play Ambient Sound, Play Ambient Music, Play Placed Sounds | the area header's Audio popover: Ambient, Music, Sounds | ✅ |
| Build › Verify Area | none: Build › Verify Module verifies the whole module, its areas with it | ✗ |
| Build › Build Module | Build › Build Module… | ✅ [build_module_compiles_and_reports] |
| Build › Test Module (F9) | Build › Test Module (F9), and Test Module, Choose Character (Shift+F9) | ✅ |
| Build › Area Statistics | Build › Area Statistics | ✅ |
| Tools › Conversation Editor (Ctrl+Alt+V), Script Editor (Ctrl+Alt+S) | Tools › New Conversation…, New Script…, with Aurora's keys | ✅ |
| Tools › Faction Editor (Ctrl+Alt+F), Journal Editor (Ctrl+Alt+J) | Tools › Faction Editor, Journal Editor, with Aurora's keys | ✅ |
| Tools › Options | Tools › Options… | ✅ |
| Wizards › Module Wizard (Ctrl+Alt+M; hidden: the Welcome dialog's) | File › New Module… (Ctrl+N), the Welcome tab's New Module… | ✅ |
| Wizards › Area Wizard (Ctrl+Alt+A), Creature Wizard (Ctrl+Alt+C) | Wizards › Area Wizard…, Creature Wizard…, with Aurora's keys | ✅ |
| Wizards › Door, Encounter, Item (Ctrl+Alt+I), Merchant, Placeable, Sound, Trigger, Waypoint Wizard | Wizards › the same, the Merchant Wizard as the Store Wizard (Stores everywhere: the manual's chapter 12) | ✅ |
| Wizards › Plot Wizard (Ctrl+Alt+P) | not built (above) | ✗ |
| Help › Neverwinter Nights Website | none | — |
| Help › About | Help › About Moonglow Toolset (and User Manual, Command Palette…, Check for Updates…) | ✅ |
| (none: Aurora finds neither resources nor commands by name) | Quick Open, the Top Bar's field (Ctrl+P): the module's resources, the commands and the load order's resources by name; Help › Command Palette… (Ctrl+Shift+P) is it among the commands | — Moonglow's own [quick_open::a_resource_is_opened_by_its_name] |
