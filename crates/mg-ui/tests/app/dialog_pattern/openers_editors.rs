//! Openers for the dialogs of the "editors" group (see `dialog_pattern.rs`):
//! the conversation, faction, script, table and NUI editors' own dialogs.

use super::Opener;
use super::openers_core::{clear_field, click_reached};
use crate::common::*;
use serde_json::json;

/// A conversation, "Hello there", open in its editor.
fn conversation() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-editors-conversation"));
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "Hello there");
    let mut m = Module::open(&path).unwrap();
    m.set(key, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.dialog_hide_names = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = harness(app, 1100.0, 800.0);
    settle(&mut h);
    h.get_by_label("Hello there").click();
    settle(&mut h);
    Some(h)
}

/// A module with custom factions Guards (5) and Thieves (6), the factions
/// editor open.
fn factions() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-editors-factions"));
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut f = mg_module::factions::Factions::of_module(&m);
    f.add("Guards", true, 4);
    f.add("Thieves", true, 2);
    m.set(ResKey::parse("repute", ResType::FAC).unwrap(), f.to_gff().to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    Some(h)
}

fn nss(name: &str) -> ResKey {
    ResKey::parse(name, ResType::NSS).unwrap()
}

/// The sample module's `hello` script open in the script editor.
fn script() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-editors-script"));
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("hello"))));
    settle(&mut h);
    Some(h)
}

/// Two scripts, `inc_a` declaring `Twice` and `main_a` using it; `main_a` open.
fn rename_script() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-editors-rename"));
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    m.set(nss("inc_a"), b"int Twice(int n) { return n * 2; }\n".to_vec());
    m.set(nss("main_a"), b"#include \"inc_a\"\nvoid main() { int x = Twice(2); }\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("main_a"))));
    settle(&mut h);
    Some(h)
}

const TABLE: &[u8] = b"2DA V2.0\n\n    Label Cost\n0   Bread 5\n1   Cheese 9\n";

/// The module's table `mystore` open in the table editor.
fn table() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-editors-table"));
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    mg_ui::table_view::open_table_window(h.state_mut());
    h.run_steps(3);
    h.get_by_label("mystore.2da").click();
    h.run_steps(4);
    Some(h)
}

/// A module with the module's tables to open.
fn tables() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-editors-tables"));
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    Some(h)
}

/// With the game's data: a feat.2da lacking the game's last 25 rows open
/// (so the game's copy has rows to take).
fn feat_table() -> Option<Harness<'static, Moonglow>> {
    let root = mg_testkit::nwn_root().or_else(|| {
        eprintln!("skipped: no game install");
        None
    })?;
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-editors-take"));
    let game = game_data(&root);
    let mut t = (*game.table("feat").unwrap()).clone();
    let whole = t.len();
    t.rows.truncate(whole - 25);
    let file = dir.join("feat.2da");
    std::fs::write(&file, t.to_bytes(mg_core::Codepage::default()).unwrap()).unwrap();
    let mut h = harness(game_app(&root, None), 1280.0, 800.0);
    h.run_steps(3);
    mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    Some(h)
}

/// A window of ours open in the NUI designer: a label, a text input, two
/// buttons, with a bind.
fn designer() -> Option<Harness<'static, Moonglow>> {
    let key = ResKey::parse("crier", ResType::JUI).unwrap();
    let mut w = mg_nui::window();
    w["title"] = json!("The Town Crier");
    w["root"]["children"] = json!([
        {"type":"label","id":"title","value":"Hear ye","height":30.0,"text_halign":0,"text_valign":0},
        {"type":"textedit","id":"message","label":"Write your message","value":{"bind":"message"},"height":30.0},
        {"type":"row","height":40.0,"children":[
            {"type":"button","id":"send","label":"Send","height":34.0,"width":100.0},
            {"type":"button","id":"cancel","label":"Cancel","height":34.0,"width":100.0}
        ]}
    ]);
    let mut settings = mg_nui::Settings::default();
    settings
        .bindings
        .insert("message".into(), mg_nui::Binding { value: json!(""), ..Default::default() });
    let mut app = app_with(Vec::new());
    let mut module = Module::new();
    module.set(key, serde_json::to_vec_pretty(&w).unwrap());
    module.set(ResKey::new(key.resref, ResType::TXT), settings.bytes());
    app.ws = Some(mg_edit::Workspace::new(module));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Nui(key)));
    let mut h = harness(app, 1920.0, 900.0);
    settle(&mut h);
    Some(h)
}

/// The designer's header's More, opened, and its control `name` clicked.
fn nui_more(h: &mut Harness<'static, Moonglow>, name: &str) {
    use mg_ui::nui_names::NUI_HEADER;
    let more = mg_uikit::header::more_name(NUI_HEADER);
    if h.query_by_label(&more).is_none() && h.query_by_label(name).is_none() {
        h.get_by_label(NUI_HEADER).get_by_label(mg_uikit::header::MORE).click();
        settle(h);
    }
    h.get_all_by_label(name).next().unwrap_or_else(|| panic!("{name}")).click();
}

pub(crate) fn openers() -> Vec<Opener> {
    vec![
        Opener::new("dialog_view.rs:editor#1", "Select Token", conversation, |h| {
            click_reached(h, "Token…")
        }),
        Opener::new(
            "dialog_view.rs:test_window#1",
            "Conversation Test: mg_talk.dlg",
            conversation,
            |h| {
                // (The header's Test, which a window at the least size
                // leaves under the tab bar.)
                let key = ResKey::parse("mg_talk", ResType::DLG).unwrap();
                h.state_mut().dialog_views.get_mut(&key).unwrap().test = Some(Vec::new());
            },
        ),
        Opener::new("dialog_view.rs:text_panel#1", "Input Text", conversation, |h| {
            dialog_click(h, "Add")
        }),
        Opener::new("faction_view.rs:editor#1", "Add Faction", factions, |h| {
            h.get_by_label("Add Faction…").click()
        })
        .field(0)
        .off(|h| clear_field(h, "Add Faction", 0)),
        Opener::new("faction_view.rs:editor#2", "Change Faction Name", factions, |h| {
            h.state_mut().faction_view.selected = 5;
            h.run();
            h.get_by_label("Change Name…").click();
        })
        .field(0)
        .off(|h| clear_field(h, "Change Faction Name", 0)),
        Opener::new("script_nav.rs:rename_window#1", "Rename Twice", rename_script, |h| {
            let text = "#include \"inc_a\"\nvoid main() { int x = Twice(2); }\n";
            let at = text.find("Twice").unwrap() + 2;
            let d = h.state_mut().declaration_at(nss("main_a"), at).expect("a declaration");
            h.state_mut().script_nav.rename = Some((d, "Thrice".into()));
        })
        .field(0)
        .off(|h| clear_field(h, "Rename Twice", 0)),
        Opener::new("script_view.rs:open_script_ui#1", "Open Script", script, |h| {
            script_click(h, "Open…")
        })
        .field(0)
        // (As at the tip: with nothing selected, Open is off and Enter opens
        // the first script listed, as Quick Open's does.)
        .enter_when_off("opens the first script listed", |h| !super::is_shown(h, "Open Script")),
        Opener::new("script_view.rs:windows#1", "Find Text", script, |h| {
            h.state_mut().script_tools.search.script = Some(nss("hello"));
            h.state_mut().script_tools.search.open = true;
            h.state_mut().script_tools.search.find = "nowhere".into();
        })
        // (A word the script has not: the search says so in the log.)
        .enter_does("searches the script, and stays open", |h| {
            let said = h.state().log.entries.last().map(|(_, m)| m.as_str());
            super::is_shown(h, "Find Text") && said == Some("\"nowhere\" not found in hello.nss")
        })
        .field(0),
        Opener::new("table_view.rs:column_window#1", "Add Column", table, |h| {
            h.state_mut().tables[0].new_column = Some(String::new());
        })
        .field(0)
        .off(|h| clear_field(h, "Add Column", 0)),
        Opener::new("table_view.rs:open_table_ui#1", "Open 2DA", tables, |h| {
            mg_ui::table_view::open_table_window(h.state_mut())
        })
        .field(0),
        Opener::new(
            "table_view.rs:taking_window#1",
            "Take Rows from Another Copy",
            feat_table,
            |h| {
                // (The header's Take Rows…, in its More at the least window.)
                h.state_mut().tables[0].taking = Some(Default::default());
            },
        ),
        Opener::new("nui_view.rs:how_to_open#1", "How to Open It", designer, |h| {
            nui_more(h, mg_ui::nui_names::HOW_TO_OPEN)
        }),
        Opener::new("nui_view/bindings.rs:inspector#1", "New Bind for a Property", designer, |h| {
            h.get_by_label("Canvas Label · Hear ye").click();
            settle(h);
            click_reached(h, "New Bind for a Property")
        })
        .field(0)
        .off(|h| clear_field(h, "New Bind for a Property", 0)),
        Opener::new("nui_view/layouts.rs:create#1", "New Variant", designer, |h| {
            h.get_by_label("Canvas Button · Send").click();
            settle(h);
            click_reached(h, "Swap Layout");
            settle(h);
            click_reached(h, "Add Variant")
        })
        .field(0)
        .off(|h| clear_field(h, "New Variant", 0)),
        Opener::new("nui_view/workflow.rs:create_bind#1", "New Bind", designer, |h| {
            click_reached(h, "New Bind")
        })
        .field(0)
        .off(|h| clear_field(h, "New Bind", 0)),
    ]
}
