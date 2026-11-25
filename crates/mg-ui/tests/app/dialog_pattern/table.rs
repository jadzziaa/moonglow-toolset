//! The dialog pattern's table (see `dialog_pattern.rs`): each dialog's
//! footer in the order the board gives it, the exceptions to "the main
//! action first" named, and the dialogs Enter is withheld from.

/// Each dialog's footer as the board orders it: its key (an opener's), its
/// buttons' words left to right, and, where a button stands before the
/// main action, why (named here, not read from the words).
pub(crate) const ORDERS: &[(&str, &[&str], Option<&str>)] = &[
    ("area_reshape.rs:resize_window#1", &["OK", "Cancel"], None),
    ("area_reshape.rs:rotate_window#1", &["OK", "Cancel"], None),
    ("area_reshape.rs:tileset_window#1", &["OK", "Cancel"], None),
    ("area_tools.rs:adjust_window#1", &["OK", "Apply", "Cancel"], None),
    ("area_view.rs:set_window#1", &["OK", "Cancel"], None),
    ("bake_view.rs:save_window#1", &["Tidy Up", "Keep Them", "Cancel"], None),
    ("blueprint/creature_lists.rs:inventory#1", &["Yes", "No"], None),
    ("blueprint/replacements.rs:gallery#1", &["OK", "Cancel"], None),
    ("blueprint/situated.rs:portrait#1", &["OK", "Cancel"], None),
    ("blueprint/situated.rs:setup_transition#1", &["OK", "Cancel"], None),
    ("blueprint_wizard.rs:ui#1", &["< Back", "Next >", "Finish", "Cancel"], Some(WIZARD)),
    ("builder_code.rs:window#1", &["OK", "No Code", "Cancel"], None),
    ("build_view.rs:window#1", &["Build", "Mark as Used", "Export…", "Done"], None),
    ("bulk.rs:update_window#1", &["Update 2", "Cancel"], None),
    ("claims.rs:protected_window#1", &["Change It Anyway", "Make a Copy Instead…", "Cancel"], None),
    ("claims.rs:save_window#1", &["Save Anyway", "Cancel"], None),
    ("copy_as.rs:window#1", &["Create Copy", "Cancel"], None),
    ("creature_wizard.rs:window#1", &["< Back", "Next >", "Finish", "Cancel"], Some(WIZARD)),
    ("dialog_view.rs:editor#1", &["Cancel"], None),
    ("dialog_view.rs:test_window#1", &["< Back", "Done"], None),
    ("dialog_view.rs:text_panel#1", &["OK", "Cancel"], None),
    ("faction_view.rs:editor#1", &["OK", "Cancel"], None),
    ("faction_view.rs:editor#2", &["OK", "Cancel"], None),
    ("jobs.rs:window#1", &["Cancel"], None),
    ("jobs.rs:window#1 Confirm", &["Yes", "No"], None),
    ("jobs.rs:window#1 File", &["Choose File…", "Cancel"], None),
    ("jobs.rs:window#1 Folder", &["Choose Folder…", "Cancel"], None),
    ("jobs.rs:window#1 Form", &["OK", "Cancel"], None),
    ("jobs.rs:window#1 Message", &["OK"], None),
    ("jobs.rs:window#1 Page", &["< Back", "Next >", "Cancel"], Some(PAGE)),
    ("levelup_view.rs:window#1", &["OK", "Cancel"], None),
    ("manual.rs:about_window#1", &[], None),
    ("module_folder.rs:ui#1", &["Open Directory", "Open .mod File", "Cancel"], None),
    ("module_props.rs:attach_window#1", &["OK", "Cancel"], None),
    ("nui_view/bindings.rs:inspector#1", &["Create", "Cancel"], None),
    ("nui_view/layouts.rs:create#1", &["Create", "Cancel"], None),
    ("nui_view.rs:how_to_open#1", &["Copy", "Close"], None),
    ("nui_view/workflow.rs:create_bind#1", &["Create", "Cancel"], None),
    ("nwsync_view.rs:window#1", &["Publish", "Remove Unused Data", "Upload"], None),
    ("outside.rs:window#1", &["Keep Moonglow's", "Take the Files'", "Later"], None),
    ("palette_categories.rs:window#1", &["OK", "Use the Game's Categories", "Cancel"], None),
    ("plugin_events.rs:window#1", &["Save Anyway", "Cancel"], None),
    ("plugins.rs:project_window#1", &["Allow All", "Decide Later"], None),
    ("plugins.rs:remove_window#1", &["Remove", "Cancel"], None),
    ("plugins.rs:replace_window#1", &["Replace", "Cancel"], None),
    ("plugins.rs:settings_window#1", &["Save", "Defaults", "Cancel"], None),
    ("prefabs.rs:save_window#1", &["Save", "Cancel"], None),
    ("project_commands.rs:output_window#1", &["Copy", "Close"], None),
    ("project_commands.rs:window#1", &["Allow and Run", "Cancel"], None),
    ("recovery.rs:window#1", &["Later"], None),
    ("references.rs:delete_window#1", &["Delete", "Cancel"], None),
    ("references.rs:rename_window#1", &["Rename", "Cancel"], None),
    ("script_nav.rs:rename_window#1", &["Rename", "Cancel"], None),
    ("script_view.rs:open_script_ui#1", &["Open", "Cancel"], None),
    ("script_view.rs:windows#1", &["Find Next"], None),
    ("script_wizard.rs:window#1", &["< Back", "Next >", "Finish", "Cancel"], Some(WIZARD)),
    ("side_doc.rs:closing_window#1", &["Save", "Don't Save", "Cancel"], None),
    ("store_wizard.rs:popup_window#1", &["OK", "Cancel"], None),
    ("store_wizard.rs:window#1", &["< Back", "Next >", "Finish", "Cancel"], Some(WIZARD)),
    ("table_view.rs:column_window#1", &["Add", "Cancel"], None),
    ("table_view.rs:open_table_ui#1", &["Open", "Browse…", "Cancel"], None),
    ("table_view.rs:taking_window#1", &["Take 0 Rows", "Cancel"], None),
    ("tag_clash.rs:window#1", &["Retag", "Keep Tags"], None),
    ("talk_typed.rs:window#1", &["Use Line 0", "Add a Line", "Keep in the File"], None),
    ("tile_select.rs:panel#1", &[], None),
    ("tileset_view.rs:minimap_progress#1", &["Cancel"], None),
    ("transfer.rs:ask_ui#1", &["Append", "Overwrite", "Cancel"], None),
    ("transfer.rs:ui#1", &["Export 1…", "Export 1 as Files…", "Cancel"], None),
    ("transfer.rs:ui#2", &["Import", "Cancel"], None),
    ("updates.rs:window#1", &["Check Again", "Close"], None),
    ("widgets.rs:files_already#1", &["Replace 1", "Skip Those", "Cancel"], None),
    ("widgets.rs:sets_window#1", &["Close"], None),
    ("widgets.rs:show#1", &["Create", "Cancel"], None),
    ("widgets.rs:ui#1", &["OK", "Cancel"], None),
    ("widgets.rs:ui#2", &["OK", "Cancel"], None),
    ("widgets.rs:ui#3", &["None", "Cancel"], None),
    ("wizards.rs:ui#1", &["Create", "Cancel"], None),
    ("wizards.rs:ui#2", &["< Back", "Next >", "Finish", "Cancel"], Some(WIZARD)),
    ("wizards.rs:ui#3", &["Close Moonglow", "Cancel"], None),
    ("wizards.rs:ui#4", &["Save", "Don't Save", "Cancel"], None),
];

/// A wizard's moves stand before Finish, as at the tip and in Aurora's
/// wizards: `< Back`, `Next >` (the main action until the last page), then
/// Finish (the main action on the last).
const WIZARD: &str = "a wizard's moves before Finish";

/// A plugin's wizard page (a Page question): `< Back` before `Next >`, its
/// main action.
const PAGE: &str = "a plugin page's Back before Next";

/// The orchestrator's ruling (U27's review): Enter is withheld from a main
/// action that grants trust, overrides a protection, overwrites or quits.
/// Enter does what it did at the tip: nothing, unless the main action holds
/// the focus. Every other dialog's main action is Enter's.
pub(crate) const ENTER_WITHHELD: &[&str] = &[
    // Close Moonglow? (Close Moonglow: quits).
    "wizards.rs:ui#3",
    // Protected Area (Change It Anyway: overrides a protection).
    "claims.rs:protected_window#1",
    // Areas Not Yours (Save Anyway: overrides the claims).
    "claims.rs:save_window#1",
    // Before Saving (Save Anyway: overrides a plugin's warning).
    "plugin_events.rs:window#1",
    // Files Already in the Hak (Replace N: overwrites).
    "widgets.rs:files_already#1",
    // This Project Comes with Plugins (Allow All: grants trust).
    "plugins.rs:project_window#1",
    // Remove a plugin (Remove), Replace a plugin (Replace: overwrites).
    "plugins.rs:remove_window#1",
    "plugins.rs:replace_window#1",
    // Run a Project Command? (Allow and Run: grants trust).
    "project_commands.rs:window#1",
    // A plugin's Confirm (Yes: grants what it asks).
    "jobs.rs:window#1 Confirm",
    // Changed Outside Moonglow (Keep Moonglow's: overwrites the file).
    "outside.rs:window#1",
];

/// The order and the exception the table gives the dialog `key`.
pub(crate) fn order(key: &str) -> Option<(Vec<&'static str>, Option<&'static str>)> {
    ORDERS.iter().find(|(k, _, _)| *k == key).map(|(_, words, why)| (words.to_vec(), *why))
}
