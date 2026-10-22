//! Screenshots of windows for looking at their layout, rendered with wgpu
//! into `target/test-output/screens/` (kept from run to run: each test adds
//! its pictures). Not checks; run by hand:
//! `cargo test -p mg-ui --test screens -- --ignored`.

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use mg_core::ResType;
use mg_edit::GffPath;
use mg_module::script_wizard::{ClassLevel, Lists, Perform};
use mg_resman::{GameInstall, ResKey};
use mg_ui::script_wizard::{ScriptWizard, Step};
use mg_ui::{Moonglow, NoDialogs};

fn shoot(h: &mut Harness<'_, Moonglow>, dir: &std::path::Path, name: &str) {
    // Steps rather than runs: animated views keep repainting.
    h.run_steps(3);
    let image = h.render().expect("render");
    image.save(dir.join(format!("{name}.png"))).unwrap();
}

#[test]
#[ignore]
fn script_wizard_pages() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let dir = mg_testkit::output_dir("screens");
    let lists = Arc::new(Lists::load(app.game.as_deref().unwrap()));
    let key = ResKey::parse("dlg", ResType::DLG).unwrap();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    for condition in [true, false] {
        let mut w = ScriptWizard::new(
            key,
            GffPath::root(),
            "Active",
            condition,
            lists.clone(),
            "sc_001".into(),
        );
        w.classes.push(ClassLevel { class: None, level: Some(5) });
        w.items.push("key_1".into());
        w.give_items.push("nw_wswls001".into());
        w.perform = Perform::Store { tag: "store_1".into(), appraise: true };
        let pages = w.pages.len();
        let prefix = if condition { "condition" } else { "action" };
        h.state_mut().script_wizard = Some(w.clone());
        shoot(&mut h, &dir, &format!("wizard-{prefix}-choose"));
        for i in 0..pages {
            w.step = Step::Page(i);
            h.state_mut().script_wizard = Some(w.clone());
            shoot(&mut h, &dir, &format!("wizard-{prefix}-{i:02}"));
        }
        w.step = Step::Name;
        h.state_mut().script_wizard = Some(w.clone());
        shoot(&mut h, &dir, &format!("wizard-{prefix}-name"));
    }
}

/// Module Properties › Custom Content: the hak list, two rows chosen and
/// some haks not found.
#[test]
#[ignore]
fn module_properties_haks() {
    use mg_schema::{ExoString, StructExt, ifo};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let user = dir.join("haks-user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let names = ["bloodwar", "bmmountainsv102", "cb_tile_atemple", "dia_udrkspid3", "ghcastlehd"];
    for name in &names[..3] {
        let empty = mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap();
        std::fs::write(user.join(format!("hak/{name}.hak")), empty).unwrap();
    }
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    let haks = names
        .iter()
        .map(|n| {
            let mut item = ifo::MOD_HAK_LIST.new_item();
            item.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(*n));
            item
        })
        .collect();
    info.root.set(ifo::MOD_HAK_LIST.label, mg_gff::Value::List(haks));
    m.set_info(&info).unwrap();
    let path = dir.join("haks.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("Custom Content").click();
    h.run_steps(2);
    h.state_mut().hak_list.selected = [1, 2].into_iter().collect();
    shoot(&mut h, &dir, "module-properties-haks");
}

/// The 2DA editor on a copy of the game's feat.2da kept as a file: a
/// cell that names a feat that is not there, selected; and Take Rows,
/// with the game's copy to take from.
#[test]
#[ignore]
fn table_editor() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-table");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    // The game's table with a feat changed, one wrong, and its last rows
    // not there.
    let mut t = (*game.table("feat").unwrap()).clone();
    let (prereq, label) = (t.column("PREREQFEAT1").unwrap(), t.column("LABEL").unwrap());
    t.rows.truncate(t.len() - 25);
    t.set(3, prereq, Some("4000".into())).unwrap();
    t.set(5, label, Some("MyOwnDodge".into())).unwrap();
    let file = dir.join("feat.2da");
    std::fs::write(&file, t.to_bytes(mg_core::Codepage::default()).unwrap()).unwrap();
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(6);
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.go_to(3, prereq);
    }
    h.run_steps(6);
    shoot(&mut h, &dir, "table-editor");
    // Typing in a cell.
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.go_to(5, label);
        doc.typing = Some("MyOwnDodge".into());
    }
    h.run_steps(4);
    shoot(&mut h, &dir, "table-editor-typing");
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.typing = None;
        doc.taking = Some(Default::default());
    }
    h.run_steps(6);
    h.get_by_label("New").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "table-editor-take-rows");
    // Several cells selected, and a column's menu.
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.taking = None;
        doc.go_to(2, 0);
        doc.select_to((7, 2));
    }
    h.run_steps(4);
    let name = h.get_by_label("DESCRIPTION").rect().center();
    h.hover_at(name);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: name,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    shoot(&mut h, &dir, "table-editor-selection");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    // The window's close button, with nothing unsaved.
    h.state_mut().actions.push(mg_ui::Action::CloseWindow);
    h.run_steps(3);
    shoot(&mut h, &dir, "close-question");
    h.state_mut().closing = false;
    // Where a row is named, and a row of another table to choose from.
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.go_to(3, prereq);
    }
    h.run_steps(4);
    h.get_by_label("Find References to Row 3").click();
    h.run_steps(4);
    shoot(&mut h, &dir, "table-editor-references");
    // The resource browser's Edit… on a table of the game's.
    let key = ResKey::parse("skills", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Resource(key)));
    h.run_steps(5);
    h.get_by_label("Edit…").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "table-editor-edit-menu");
}

/// The game's spells.2da read in the resource browser (not editable): its
/// scroll bars, as it opens and scrolled down and to the right; and the
/// 2DA editor on a copy of appearance.2da, a column widened by dragging
/// the edge of its name.
#[test]
#[ignore]
fn table_read_only_and_column_widths() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-table-widths");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let file = dir.join("appearance.2da");
    let t = game.table("appearance").unwrap();
    std::fs::write(&file, t.to_bytes(mg_core::Codepage::default()).unwrap()).unwrap();
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let key = ResKey::parse("spells", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Resource(key)));
    h.run_steps(5);
    shoot(&mut h, &dir, "2da-read-only");
    // The wheel over the rows: down, and sideways.
    let at = h.get_by_label("Acid_Fog").rect().center();
    h.hover_at(at);
    for delta in [egui::vec2(0.0, -2000.0), egui::vec2(-1500.0, 0.0)] {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(60);
    }
    shoot(&mut h, &dir, "2da-read-only-scrolled");
    let tab = h.state().dock.find_tab(&mg_ui::Tab::Resource(key)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(6);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(4, 1);
    h.run_steps(4);
    shoot(&mut h, &dir, "table-editor-widths");
    // The edge at the right of STRING_REF's name dragged 120 points on.
    let edge = h.get_by_label("Width of STRING_REF").rect().center();
    h.hover_at(edge);
    h.run_steps(2);
    h.drag_at(edge);
    h.run_steps(1);
    for step in 1..=6 {
        h.hover_at(edge + egui::vec2(20.0 * step as f32, 0.0));
        h.run_steps(1);
    }
    h.drop_at(edge + egui::vec2(120.0, 0.0));
    h.run_steps(4);
    shoot(&mut h, &dir, "table-editor-widened");
    // A double click on the edge of NAME's: as wide as its widest cell,
    // the pointer still over the edge.
    let edge = h.get_by_label("Width of NAME").rect().center();
    h.hover_at(edge);
    h.run_steps(2);
    // (Two clicks, a frame each.)
    for pressed in [true, false, true, false] {
        h.event(egui::Event::PointerButton {
            pos: edge,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
    h.run_steps(3);
    h.hover_at(h.get_by_label("Width of NAME").rect().center());
    h.run_steps(2);
    shoot(&mut h, &dir, "table-editor-fitted");
    // The same table read in the resource browser: as wide as made here.
    let key = ResKey::parse("appearance", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Resource(key)));
    h.run_steps(5);
    shoot(&mut h, &dir, "2da-read-only-widths");
}

/// A table with a description beside it: values by name, a cell that is
/// none of its list, and a flags cell being ticked (the flags are the
/// game's own `DAMAGE_TYPE_` constants, read from `nwscript`).
#[test]
#[ignore]
fn described_table() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-described");
    let install = GameInstall::new(&root, None, "en");
    // (A project of its own: what the description names is looked for
    // here, not through this repository.)
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    let file = dir.join("my_ai_spells.2da");
    std::fs::write(
        &file,
        "2DA V2.0\r\n\r\n  Label Level DamageType AOEShape Duration ClassRequired Cures\r\n\
         0 Acid_Fog 6 0x0010 0 2 **** ****\r\n1 Aid 2 **** **** 3 **** ****\r\n\
         2 Animate_Dead 3 **** **** 4 2 ****\r\n3 Barkskin 2 **** **** 4 **** ****\r\n\
         4 Bestow_Curse 3 **** **** 5 **** ****\r\n5 Blade_Barrier 6 0x0006 1 2 **** ****\r\n\
         6 Bless 1 **** **** 3 **** ****\r\n7 Bless_Weapon 12 **** **** 9 **** ****\r\n\
         8 Blindness_and_Deafness 2 **** **** #N/A 99 ****\r\n\
         9 Bulls_Strength 2 **** **** 4 **** ****\r\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("my_ai_spells.2da.toml"),
        "rows = \"spells\"\n\n[columns.Level]\nkind = \"int\"\nmin = -1\nmax = 10\n\n\
         [columns.DamageType]\nkind = \"flags\"\nconstants = \"nwscript\"\nprefix = \
         \"DAMAGE_TYPE_\"\n\n[columns.AOEShape]\nkind = \"list\"\nvalues = { Sphere = 0, \
         Wall = 1, Cone = 3 }\n\n[columns.Duration]\nkind = \"list\"\nvalues = { Instant = 1, \
         Short = 2, Medium = 3, Long = 4, Permanent = 5 }\n\n[columns.ClassRequired]\nkind = \
         \"row\"\ntable = \"classes\"\n",
    )
    .unwrap();
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(6);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(5, 2);
    h.run_steps(4);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v == "2 flags"))
        .map(|n| n.rect().center())
        .expect("the flags' list");
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(4);
    shoot(&mut h, &dir, "described-table");
}

/// A 2DA cell that names a row of spells.2da, far down: its list opened,
/// at that row, with a Filter field above.
#[test]
#[ignore]
fn table_row_list_opened() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-table-row");
    let install = GameInstall::new(&root, None, "en");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    let file = dir.join("my_scrolls.2da");
    std::fs::write(&file, "2DA V2.0\r\n\r\n  Label Spell\r\n0 Scroll_A 300\r\n1 Scroll_B 2\r\n")
        .unwrap();
    std::fs::write(
        dir.join("my_scrolls.2da.toml"),
        "[columns.Spell]\nkind = \"row\"\ntable = \"spells\"\n",
    )
    .unwrap();
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(6);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(0, 1);
    h.run_steps(4);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("300")))
        .map(|n| n.rect().center())
        .expect("the rows' list");
    // (The pointer comes to the box before it clicks.)
    h.hover_at(at);
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(4);
    shoot(&mut h, &dir, "table-row-list-opened");
}

/// The conversation editor's Text Appears When page, with a script that
/// has parameters.
#[test]
#[ignore]
fn dialog_script_parameters() {
    use mg_module::dialog::{Parent, add_node, new_dialog, params_value};
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-dialog-params");
    let mut g = new_dialog();
    add_node(&mut g, Parent::Root, "What do you want made?");
    {
        let link = &mut g.root.list_mut("StartingList").unwrap()[0];
        link.set("Active", mg_gff::Value::resref("cond_weapmod".parse().unwrap()));
        let pairs = [
            ("tag".to_string(), "weficort".to_string()),
            ("apariencia".to_string(), "05".to_string()),
        ];
        link.set("ConditionParams", params_value(&pairs));
    }
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let key = ResKey::parse("smith", ResType::DLG).unwrap();
    m.set_gff(key, &g).unwrap();
    m.set(
        ResKey::parse("cond_weapmod", ResType::NSS).unwrap(),
        b"// TRUE where the weapon is the one named\nint StartingConditional()\n{\n    return TRUE;\n}\n".to_vec(),
    );
    let path = dir.join("d.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Dialog(key)));
    h.run_steps(4);
    h.get_by_label_contains("What do you want made?").click();
    h.run_steps(4);
    shoot(&mut h, &dir, "dialog-script-parameters");
    // (And at a larger size of its own: Options › Conversation Editor.)
    h.state_mut().settings.dialog_font_size = Some(20);
    shoot(&mut h, &dir, "dialog-font-20");
    // (Its right-click menu, at that size too.)
    let at = h.get_by_label_contains("What do you want made?").rect().center();
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    shoot(&mut h, &dir, "dialog-font-20-menu");
}

/// The Talk Table editor on a line with a long text: plain, and with a
/// feminine table.
#[test]
#[ignore]
fn talk_table_editor() {
    use mg_module::talk::{Line, Table};
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens");
    for feminine in [false, true] {
        let app = Moonglow::new(None, Box::new(NoDialogs::default()));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1100.0, 760.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        let name = if feminine { "long_f" } else { "long" };
        let mut table = Table::create(name, &dir, mg_core::Language::ENGLISH, feminine);
        let long: Vec<String> = (1..=50).map(|n| format!("Line {n} of a long text.")).collect();
        for row in 0..60 {
            let text = if row == 27 { long.join("\n") } else { format!("Text {row}") };
            let line = Line {
                feminine: feminine.then(|| text.clone()),
                text,
                sound: String::new(),
                sound_length: 0.0,
            };
            table.add_line(&line).unwrap();
        }
        h.state_mut().talk = Some(table);
        h.state_mut().talk_view.outside = true;
        h.state_mut().talk_view.selected = Some(27);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::TalkTable));
        // (The list's height settles over a few frames.)
        h.run_steps(8);
        shoot(&mut h, &dir, &format!("talk-table-{name}"));
        // The pointer over a line: the whole row shows it.
        h.hover_at(egui::pos2(520.0, 472.0));
        shoot(&mut h, &dir, &format!("talk-table-{name}-hover"));
    }
}

#[test]
#[ignore]
fn options_window() {
    mg_testkit::gpu::hold();
    use egui_kittest::kittest::Queryable;
    let dir = mg_testkit::scratch_dir("screens-options");
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.script_style.colors[3] = Some([255, 128, 0]);
    app.settings.script_style.background = Some([24, 32, 48]);
    app.options = Some(mg_ui::OptionsDraft::from_settings(&app.settings));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 900.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    shoot(&mut h, &dir, "options-folders");
    h.get_by_label("General").click();
    shoot(&mut h, &dir, "options-general");
    h.get_by_label("Script Editor").click();
    shoot(&mut h, &dir, "options-script-editor");
    // Further down: the colors, the background and the preview.
    h.get_by_label("Reset to defaults").scroll_to_me();
    shoot(&mut h, &dir, "options-script-editor-colors");
    // (A line in the log, beside its Clear.)
    h.state_mut().log.info("Verify: 0 error(s), 0 warning(s)");
    h.state_mut().options.as_mut().unwrap().verify_skip = vec!["tilesets".into()];
    h.get_by_label("Verify").click();
    shoot(&mut h, &dir, "options-verify");
    h.get_by_label("Keyboard").click();
    h.run();
    let draft = h.state_mut().options.as_mut().unwrap();
    draft.keymap.set(
        mg_ui::keys::Cmd::Manual,
        vec![egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S)],
    );
    shoot(&mut h, &dir, "options-keyboard");
}

#[test]
#[ignore]
fn model_viewer() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-model");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    for name in ["plc_a01", "c_golemerald"] {
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(
            ResKey::parse(name, ResType::MDL).unwrap(),
        )));
    }
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    shoot(&mut h, &dir, "model-viewer");
}

#[test]
#[ignore]
fn palette_hover_preview() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-hover");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
    app.palette.tiles = false;
    app.palette.filter = "plc_chest1".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let chest = h.state().game.as_deref().unwrap().string(mg_core::StrRef(5348)).unwrap();
    let at = h.get_all_by_label(&chest).next().unwrap().rect().center();
    h.hover_at(at);
    h.run_steps(30);
    shoot(&mut h, &dir, "palette-hover");
}

#[test]
#[ignore]
fn item_previews() {
    // Items in the preview tab, as their blueprint rows' Preview opens
    // them: an armor, a helmet, a shield and a Drow shield (a builder's
    // report after 1.33.0: armor showed a head and pelvis, the Drow
    // shields bags, helmets and shields turned oddly).
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-model");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    for name in ["nw_aarcl004", "nw_cloth027", "nw_arhe001", "nw_ashsw001", "x2_adrowshl001"] {
        let key = ResKey::parse(name, ResType::UTI).unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(key)));
        h.run_steps(4);
        shoot(&mut h, &dir, &format!("item-preview-{name}"));
        let tab = h.state().dock.find_tab(&mg_ui::Tab::Model(key)).unwrap();
        h.state_mut().dock.remove_tab(tab);
        h.run_steps(2);
    }
}

#[test]
#[ignore]
fn blueprint_preview() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-model");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(
        ResKey::parse("nw_halfdra002", ResType::UTC).unwrap(),
    )));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    shoot(&mut h, &dir, "blueprint-preview");
}

#[test]
#[ignore]
fn palettes() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    app.palette.filter = "bear".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    shoot(&mut h, &dir, "palettes");
}

#[test]
#[ignore]
fn blueprint_editors() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    for (name, t, pages) in [
        ("nw_waypoint001", ResType::UTW, &["Basic", "Advanced"][..]),
        ("animalcriesday", ResType::UTS, &["Basic", "Positioning", "Advanced"][..]),
        ("x0_trapavg_shuri", ResType::UTT, &["Basic", "Trap", "Advanced", "Visuals"][..]),
        ("nw_mumcleric", ResType::UTC, &["Classes"][..]),
        ("nw_verminbeet", ResType::UTE, &["Basic", "Creature List", "Advanced"][..]),
        ("nw_storebar01", ResType::UTM, &["Basic", "Inventory", "Restrictions"][..]),
        ("x2_storethief001", ResType::UTM, &["Inventory"][..]),
        ("nw_door_ttr_01", ResType::UTD, &["Basic", "Lock", "Area Transition", "Advanced"][..]),
        ("plc_chest1", ResType::UTP, &["Basic", "Scripts", "Advanced", "Visuals"][..]),
        ("nw_wswmls010", ResType::UTI, &["General", "Appearance", "Properties"][..]),
        ("nw_aarcl004", ResType::UTI, &["General", "Appearance"][..]),
        ("nw_maarcl002", ResType::UTI, &["Appearance"][..]),
        (
            "nw_bartender",
            ResType::UTC,
            &["Basic", "Statistics", "Appearance", "Classes", "Skills", "Advanced"][..],
        ),
        (
            "db_tanarukk_do",
            ResType::UTC,
            &["Feats", "Spells", "Special Abilities", "Inventory"][..],
        ),
    ] {
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        let game = app.game.as_deref().unwrap();
        let data = game.resman.get_named(name, t).unwrap().into_owned();
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse(name, t).unwrap(), data);
        let path = dir.join(format!("{name}.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        let key = ResKey::parse(name, t).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        for page in pages {
            h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), page);
            let page_name = page.to_lowercase().replace(' ', "-");
            shoot(&mut h, &dir, &format!("{name}-{page_name}"));
        }
    }
}

/// A placeable's Illuminated box (a brazier's, and a chest's, which has no
/// light), and a creature's faction: of the PC faction, kept and named so,
/// and the list opened without it.
#[test]
#[ignore]
fn illumination_and_pc_faction() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    for (name, t, pc) in [
        ("plc_brazier", ResType::UTP, false),
        ("plc_chest1", ResType::UTP, false),
        ("nw_bandit001", ResType::UTC, true),
    ] {
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        let game = app.game.as_deref().unwrap();
        let mut data = game.resman.get_named(name, t).unwrap().into_owned();
        if pc {
            let mut g = mg_gff::Gff::read(&data).unwrap();
            g.root.set("FactionID", mg_gff::Value::Word(0));
            data = g.to_bytes().unwrap();
        }
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse(name, t).unwrap(), data);
        let path = dir.join(format!("{name}-light.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        let key = ResKey::parse(name, t).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.state_mut().blueprint_pages.insert((key, GffPath::root()), "Advanced");
        shoot(&mut h, &dir, &format!("{name}-advanced-light-faction"));
        if pc {
            h.get_by_value("PC (not for objects)").click();
            shoot(&mut h, &dir, &format!("{name}-faction-list"));
        }
    }
}

/// An item's General page at several widths: its two columns side by
/// side only where the first one's fields fit beside the second.
#[test]
#[ignore]
fn item_general_widths() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let (name, t) = ("nw_cloth009", ResType::UTI);
    for width in [820.0, 960.0, 1100.0, 1240.0, 1500.0] {
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        let game = app.game.as_deref().unwrap();
        let data = game.resman.get_named(name, t).unwrap().into_owned();
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse(name, t).unwrap(), data);
        let path = dir.join(format!("{name}.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        let key = ResKey::parse(name, t).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(width, 420.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        shoot(&mut h, &dir, &format!("item-general-{width}"));
    }
}

/// A weapon's Appearance page: its parts' pictures, at two window sizes.
#[test]
#[ignore]
fn weapon_parts() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let t = ResType::UTI;
    for (name, size) in [
        ("nw_wswgs001", egui::vec2(1300.0, 1500.0)),
        ("nw_wswgs001", egui::vec2(1000.0, 640.0)),
        ("nw_wswdg001", egui::vec2(1000.0, 640.0)),
    ] {
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        let game = app.game.as_deref().unwrap();
        let data = game.resman.get_named(name, t).unwrap().into_owned();
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse(name, t).unwrap(), data);
        let path = dir.join(format!("{name}.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        let key = ResKey::parse(name, t).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        let mut h = Harness::builder()
            .with_size(size)
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(4);
        h.get_by_label("Appearance").click();
        h.run_steps(6);
        shoot(&mut h, &dir, &format!("weapon-parts-{name}-{}", size.y));
    }
}

/// Export Resources: the module kind by kind.
#[test]
#[ignore]
fn export_resources() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-export");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    for (name, t) in [
        ("guard", ResType::UTC),
        ("smith", ResType::UTC),
        ("longsword", ResType::UTI),
        ("chest", ResType::UTP),
        ("on_enter", ResType::NSS),
        ("on_enter", ResType::NCS),
        ("spells", ResType::TWODA),
    ] {
        m.set(ResKey::parse(name, t).unwrap(), Vec::new());
    }
    let path = dir.join("e.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 640.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::ExportDialog(Vec::new()));
    h.run_steps(4);
    shoot(&mut h, &dir, "export-resources");
}

/// A creature's Appearance list, opened: at the appearance it has, its
/// Filter field above the rows.
#[test]
#[ignore]
fn appearance_list_opened() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    app.open_palette = false;
    let utc = ResKey::parse("m1q1faribeth", ResType::UTC).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(utc)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    // (The lists of the Basic page in order: the appearance's is the one
    // whose value is an appearance's name.)
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains("Aribeth")))
        .map(|n| n.rect().center())
        .expect("the Appearance dropdown");
    h.hover_at(at);
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(4);
    shoot(&mut h, &dir, "appearance-list-opened");
}

/// A potion's Cast Spell property, its Sub-Property list opened: the
/// hundreds of spells of iprp_spells, by name, with a Filter field above.
#[test]
#[ignore]
fn cast_spell_subtypes_opened() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let (name, t) = ("nw_it_mpotion001", ResType::UTI);
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named(name, t).unwrap().into_owned();
    let mut m = mg_module::Module::new();
    m.set(ResKey::parse(name, t).unwrap(), data);
    let path = dir.join(format!("{name}.mod"));
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.open_palette = false;
    let key = ResKey::parse(name, t).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    h.get_by_label("Properties").click();
    h.run_steps(3);
    // The assigned property, chosen: its parameters show under it.
    h.get_all_by_label_contains("Cure Light Wounds").last().unwrap().click();
    h.run_steps(3);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains("Cure Light Wounds")))
        .map(|n| n.rect().center())
        .expect("the Sub-Property dropdown");
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(4);
    shoot(&mut h, &dir, "cast-spell-subtypes-opened");
}

/// The model beside the fields, in a window wide enough for both.
#[test]
#[ignore]
fn model_beside_the_fields() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-model-beside");
    for (name, t, page) in [
        ("plc_chest1", ResType::UTP, "Basic"),
        ("nw_door_ttr_01", ResType::UTD, "Basic"),
        ("nw_wswmls010", ResType::UTI, "Appearance"),
    ] {
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        let game = app.game.as_deref().unwrap();
        let data = game.resman.get_named(name, t).unwrap().into_owned();
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse(name, t).unwrap(), data);
        let path = dir.join(format!("{name}.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        let key = ResKey::parse(name, t).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1500.0, 860.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), page);
        h.run();
        shoot(&mut h, &dir, &format!("{name}-{}", page.to_lowercase()));
    }
}

/// The symbols the interface uses, as the fonts draw them (a square is a
/// glyph the fonts lack).
#[test]
#[ignore]
fn glyphs() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-glyphs");
    let app = Moonglow::new(None, Box::new(NoDialogs::default()));
    let mut h = Harness::builder().with_size(egui::vec2(700.0, 200.0)).wgpu().build_ui_state(
        |ui, app: &mut Moonglow| {
            // (The application's fonts.)
            app.ui(ui);
            egui::Window::new("glyphs").fixed_pos([20.0, 60.0]).show(ui.ctx(), |ui| {
                ui.heading("▶ 💡 ◎ ⏵ ⏷ 🗐 📦 ⚙ 🗻 🗑 ↕ 🔁 🏁 🌙 ☁ ⊞ 🍃 🎵 👣 ⛶ ℹ 🎥 🏃");
            });
        },
        app,
    );
    h.run();
    h.run();
    shoot(&mut h, &dir, "glyphs");
}

#[test]
#[ignore]
fn store_chosen_item() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let (name, t) = ("nw_storebar01", ResType::UTM);
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named(name, t).unwrap().into_owned();
    // A magic armor: statistics and properties.
    let armor = game.resman.get_named("nw_maarcl002", ResType::UTI).unwrap();
    let armor = mg_gff::Gff::read(&armor).unwrap().root;
    let armor = game.locstring(armor.locstring("LocalizedName").unwrap()).unwrap();
    let mut m = mg_module::Module::new();
    m.set(ResKey::parse(name, t).unwrap(), data);
    let path = dir.join(format!("{name}-chosen.mod"));
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    let key = ResKey::parse(name, t).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().blueprint_pages.insert((key, GffPath::root()), "Inventory");
    h.run_steps(3);
    place(&mut h, &mg_ui::Tab::Blueprint(key), (20.0, 60.0), (1150.0, 800.0));
    h.run_steps(3);
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    h.run_steps(2);
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text("nw_maarcl002");
    h.run_steps(3);
    let row = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some(armor.as_str())
    };
    h.get(egui_kittest::kittest::by().predicate(row)).click();
    shoot(&mut h, &dir, "store-chosen-item");
    // Added, on the Armor page beside the bar's potions.
    h.get_by_label("Add Item").click();
    shoot(&mut h, &dir, "store-armor-added");
}

#[test]
#[ignore]
fn user_manual() {
    mg_testkit::gpu::hold();
    use egui_kittest::kittest::{NodeT, Queryable};
    let dir = mg_testkit::scratch_dir("screens-manual");
    let app = Moonglow::new(None, Box::new(NoDialogs::default()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Manual));
    shoot(&mut h, &dir, "manual-contents");
    let listed =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Button;
    h.get_all_by_label("Areas").find(|n| listed(n)).unwrap().click();
    shoot(&mut h, &dir, "manual-areas");
    h.get_all_by_label("Coming from Aurora").find(|n| listed(n)).unwrap().click();
    shoot(&mut h, &dir, "manual-coming-from-aurora");
    // (The menu bar has a Plugins too: the chapter is the later one.)
    for (chapter, shot) in [
        ("The main window", "manual-main-window"),
        ("Plugins", "manual-plugins"),
        ("Writing plugins", "manual-writing-plugins"),
        ("Plugin API reference", "manual-plugin-api"),
    ] {
        h.get_all_by_label(chapter).rfind(|n| listed(n)).unwrap().click();
        shoot(&mut h, &dir, shot);
    }
    h.state_mut().about = true;
    shoot(&mut h, &dir, "about");
}

#[test]
#[ignore]
fn script_beside_the_palette_pane() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-script-narrow");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 700.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    for tab in [
        mg_ui::Tab::Palette,
        mg_ui::Tab::Manual,
        mg_ui::Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap()),
    ] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run();
    }
    shoot(&mut h, &dir, "script-800");
}

#[test]
#[ignore]
fn script_editor() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-script");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main()\n{\n\n}\n".to_vec());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1700.0, 900.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let script = mg_ui::Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    h.state_mut().actions.push(mg_ui::Action::OpenTab(script));
    h.state_mut().script_tools.side = mg_ui::script_tools::SideTab::Constants;
    h.state_mut().script_tools.filter = "PROPERTY_".into();
    h.run();
    h.run();
    shoot(&mut h, &dir, "script-editor");
}

/// The script editor with no error and with one found as you type: the
/// error's line is there either way, so the text does not move (Jasperre's
/// report: it jumped down as an error came and went).
#[test]
#[ignore]
fn script_error_line() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-script-error");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let good = ResKey::parse("good", ResType::NSS).unwrap();
    let bad = ResKey::parse("bad", ResType::NSS).unwrap();
    m.set(good, b"void main()\r\n{\r\n    int n = 1;\r\n}\r\n".to_vec());
    let long = "x".repeat(300);
    let text = format!("void main()\r\n{{\r\n    int n = {long};\r\n}}\r\n");
    m.set(bad, text.into_bytes());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 600.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    for (key, name) in [(good, "script-error-none"), (bad, "script-error-shown")] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
        h.run();
        h.run();
        shoot(&mut h, &dir, name);
    }
    // A background of the builder's own (Options › Script Editor).
    h.state_mut().settings.script_style.background = Some([24, 32, 48]);
    shoot(&mut h, &dir, "script-background");
}

/// Help as `nwscript.nss` has it: the comment with its `//` and columns,
/// in the editor's colors and font, then the declaration; and the
/// Compiler tab's lines with their times.
#[test]
#[ignore]
fn script_help() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-script-help");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, b"void main()\r\n{\r\n    int n = ;\r\n}\r\n".to_vec());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    h.state_mut().script_tools.filter = "GetFirstObjectInShape".into();
    h.run();
    h.run();
    h.get_by_label("GetFirstObjectInShape").click();
    shoot(&mut h, &dir, "script-help");
    // Compile twice: two lines, each with its time.
    h.get_by_label("Compile").click();
    h.run();
    shoot(&mut h, &dir, "script-compiler");
}

/// A selection in the script editor: whole lines (to their line ends),
/// and part of one.
#[test]
#[ignore]
fn script_selection() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-script-selection");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let text = "void main()\n{\n    int nGold = 5;\n    string sName = \"x\";\n\n    return;\n}\n";
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, text.as_bytes().to_vec());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 420.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    h.run();
    // From the start of "int nGold" line to the start of the empty line:
    // two whole lines with their line ends.
    let from = text.find("    int").unwrap();
    let to = text.find("\n\n").unwrap() + 1;
    for (name, range) in [("lines", (from, to)), ("part", (from + 4, from + 13))] {
        let id = egui::Id::new(("script", key));
        let mut state = egui::text_edit::TextEditState::load(&h.ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(range.0),
            egui::text::CCursor::new(range.1),
        )));
        state.store(&h.ctx, id);
        h.ctx.memory_mut(|m| m.request_focus(id));
        h.run_steps(3);
        shoot(&mut h, &dir, &format!("script-selection-{name}"));
    }
}

/// The Open Script window with several scripts selected (a click, a
/// Shift+click and a Ctrl+click), to open together.
#[test]
#[ignore]
fn open_script_several() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-open-script");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    for n in ["at_door", "at_gate", "at_inn", "inc_loot", "on_death", "on_enter", "on_spawn"] {
        m.set(ResKey::parse(n, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 640.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    h.run();
    h.get_by_label("Open…").click();
    h.run_steps(3);
    h.get_by_label("at_gate").click();
    h.run_steps(2);
    h.get_by_label("inc_loot").click_modifiers(egui::Modifiers::SHIFT);
    h.run_steps(2);
    h.get_by_label("on_enter").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    h.hover_at(egui::pos2(5.0, 5.0));
    h.run_steps(2);
    shoot(&mut h, &dir, "open-script-several");
}

#[test]
#[ignore]
fn script_page_down() {
    // The text follows the cursor a page at a time.
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-script-page");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let text: String = (0..200).map(|n| format!("// line {n}\n")).collect();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, text.into_bytes());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 420.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    h.run();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    shoot(&mut h, &dir, "script-page-0");
    for n in 1..=3 {
        h.key_press(egui::Key::PageDown);
        shoot(&mut h, &dir, &format!("script-page-{n}"));
    }
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::PageUp);
    shoot(&mut h, &dir, "script-page-shift-up");
}

#[test]
#[ignore]
fn creature_wizard() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-creature-wizard");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.actions.push(mg_ui::Action::NewModule("wiz".into()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().creature_wizard = Some(Default::default());
    h.run();
    for page in 0..8 {
        if page == 1 {
            h.get_by_label("Human").click();
            h.run();
        }
        if page == 3 {
            // As it opens: the first portrait chosen.
            shoot(&mut h, &dir, "page-3-opened");
            h.get_by_label("po_hu_m_01_").click();
            h.run();
        }
        if page == 6 {
            h.get_by_label("Tutorial").click();
            h.run();
        }
        shoot(&mut h, &dir, &format!("page-{page}"));
        h.get_by_label("Next >").click();
        h.run();
    }
}

#[test]
#[ignore]
fn references() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-references");
    let path = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &path).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut()
        .actions
        .push(mg_ui::Action::FindReferences(ResKey::parse("pawn_b", ResType::UTC).unwrap()));
    h.run();
    shoot(&mut h, &dir, "references");
    h.state_mut()
        .actions
        .push(mg_ui::Action::RenameDialog(ResKey::parse("pawn_b", ResType::UTC).unwrap()));
    h.run();
    shoot(&mut h, &dir, "rename");
}

#[test]
#[ignore]
fn talk_table_and_2da_layers() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-talk");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    // A talk table with a feminine table, named by a copy of Chess.
    let mut t = mg_module::talk::Table::create(
        "chess_tlk",
        &user.join("tlk"),
        mg_core::Language::ENGLISH,
        true,
    );
    let lines = [
        ("Knight-Errant", "Knight-Errant", "vs_hello"),
        ("Welcome to the board, <FirstName>.", "Welcome to the board, my lady.", ""),
        ("The pawns advance at dawn.", "The pawns advance at dawn.", ""),
    ];
    for (text, f, sound) in lines {
        let line = mg_module::talk::Line {
            text: text.into(),
            feminine: Some(f.into()),
            sound: sound.into(),
            sound_length: if sound.is_empty() { 0.0 } else { 1.2 },
        };
        t.add_line(&line).unwrap();
    }
    t.save().unwrap();
    let path = dir.join("chess.mod");
    let mut m = mg_module::Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let mut ifo = m.info().unwrap();
    ifo.root.set("Mod_CustomTlk", mg_gff::Value::String(b"chess_tlk".to_vec()));
    m.set_info(&ifo).unwrap();
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    // A 2DA in override over the game's: a row changed, one added.
    let game = mg_resman::ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let key = ResKey::parse("ambientmusic", ResType::TWODA).unwrap();
    let text = String::from_utf8(game.get(&key).unwrap().into_owned()).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let last = lines.iter().rposition(|l| !l.trim().is_empty()).unwrap();
    let n: usize = lines[last].split_whitespace().next().unwrap().parse().unwrap();
    let row1 = lines.iter().position(|l| l.split_whitespace().next() == Some("1")).unwrap();
    let mut cells: Vec<String> = lines[row1].split_whitespace().map(str::to_string).collect();
    cells[2] = "mus_mine".into();
    lines[row1] = cells.join(" ");
    cells[0] = (n + 1).to_string();
    cells[2] = "mus_new".into();
    lines.insert(last + 1, cells.join(" "));
    std::fs::write(user.join("override/ambientmusic.2da"), lines.join("\n")).unwrap();

    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::TalkTable));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().talk_view.selected = Some(1);
    h.run();
    shoot(&mut h, &dir, "talk-table");
    let tab = h.state().dock.find_tab(&mg_ui::Tab::TalkTable).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Resource(key)));
    h.run();
    shoot(&mut h, &dir, "2da-layers");
}

#[test]
#[ignore]
fn hak_editor_and_attach() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-hak");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    // A shipped hak, opened in the editor, with a file added.
    let shipped = root.join("data/hk/id_resources.hak");
    let added = dir.join("mg_chair.mdl");
    std::fs::write(&added, b"model").unwrap();
    let path = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &path).unwrap();
    let tlk = dir.join("mg_text.tlk");
    std::fs::write(&tlk, mg_tlk::Tlk::new(mg_core::Language::ENGLISH).to_bytes().unwrap()).unwrap();
    let dialogs = NoDialogs {
        open: vec![shipped.clone()],
        open_many: vec![vec![shipped, tlk]],
        ..Default::default()
    };
    let mut app = Moonglow::new(Some(GameInstall::new(&root, Some(user), "en")), Box::new(dialogs));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    mg_ui::hak_view::open_hak(h.state_mut());
    h.state_mut().haks[0].hak.add_files(&[added]);
    h.run();
    shoot(&mut h, &dir, "hak-editor");
    let tab = h.state().dock.find_tab(&mg_ui::Tab::Hak(h.state().haks[0].id)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().haks.clear();
    h.run();
    use egui_kittest::kittest::Queryable;
    h.get_by_label("Custom Content").click();
    h.run();
    h.get_by_label("Add Haks and Talk Table…").click();
    h.run();
    shoot(&mut h, &dir, "attach");
}

#[test]
#[ignore]
fn object_walkmeshes() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-walkmesh");
    let path = dir.join("contest.mod");
    std::fs::copy(root.join("data/mod/Contest Of Champions 0492.mod"), &path).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    // The area with the most placeables and doors.
    let ws = app.ws.as_mut().unwrap();
    let areas = ws.module.areas().unwrap();
    let count = |ws: &mut mg_edit::Workspace, a: mg_core::ResRef| {
        let git = ws.doc(&ResKey::new(a, ResType::GIT)).unwrap();
        ["Placeable List", "Door List"]
            .iter()
            .map(|l| match git.root.get(l) {
                Some(mg_gff::Value::List(v)) => v.len(),
                _ => 0,
            })
            .sum::<usize>()
    };
    let area = *areas.iter().max_by_key(|a| count(ws, **a)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    view.walkmesh = true;
    view.object_walkmesh = true;
    h.run_steps(5);
    shoot(&mut h, &dir, "object-walkmeshes");
}

#[test]
#[ignore]
fn tileset_minimap_pictures() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-minimaps");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let install = GameInstall::new(&root, None, "en");
    let game = mg_resman::ResMan::for_game(&install).unwrap();
    // A copy of a game tileset, its pictures rendered beside it.
    let name = "tcn01";
    let set = game.get(&ResKey::parse(name, ResType::SET).unwrap()).unwrap().into_owned();
    let set_dir = dir.join("set");
    let _ = std::fs::remove_dir_all(&set_dir);
    std::fs::create_dir_all(&set_dir).unwrap();
    let path = set_dir.join(format!("{name}.set"));
    std::fs::write(&path, &set).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    mg_ui::tileset_view::open_path(h.state_mut(), path);
    h.run();
    h.state_mut().tilesets[0].page = mg_ui::tileset_view::Page::Tiles;
    h.state_mut().tilesets[0].tile = Some(3);
    h.run();
    shoot(&mut h, &dir, "tileset-editor");
    let mut doc = h.state_mut().tilesets.remove(0);
    let tiles = doc.tileset().unwrap().tiles.clone();
    mg_ui::tileset_view::render_minimaps(h.state_mut(), &mut doc, 32);
    // A sheet: the game's picture beside Moonglow's, for the first tiles.
    let shown: Vec<_> = tiles.iter().filter(|t| t.image_map_2d.is_some()).take(12).collect();
    let mut sheet = mg_image::Rgba::new(32 * 2 + 8, 40 * shown.len() as u32);
    let mut lit = 0;
    for (row, t) in shown.iter().enumerate() {
        let pic = t.image_map_2d.clone().unwrap().to_lowercase();
        let ours = std::fs::read(set_dir.join(format!("{pic}.tga"))).unwrap();
        let ours = mg_image::read(ResType::TGA, &ours).unwrap().to_rgba();
        lit += usize::from(
            ours.data.as_chunks::<4>().0.iter().any(|p| p[0] > 20 || p[1] > 20 || p[2] > 20),
        );
        let theirs = game
            .texture(mg_core::ResRef::from_str(&pic).unwrap())
            .and_then(|(t, d)| mg_image::read(t, &d).ok())
            .map(|t| t.to_rgba());
        let y0 = sheet.height - 40 * (row as u32 + 1) + 4;
        for (x0, img) in [(0u32, theirs.as_ref()), (40, Some(&ours))] {
            let Some(img) = img else { continue };
            for y in 0..32 {
                for x in 0..32 {
                    let p = img.pixel(x * img.width / 32, y * img.height / 32);
                    let i = (((y0 + y) * sheet.width + x0 + x) * 4) as usize;
                    sheet.data[i..i + 4].copy_from_slice(&p);
                }
            }
        }
    }
    assert!(lit >= shown.len() - 1, "{lit} of {} pictures have anything in them", shown.len());
    // Laid out as the game's pictures: for each tile, Moonglow's picture
    // correlates best with the game's as it is, not flipped or turned.
    let lum = |img: &mg_image::Rgba, flip_x: bool, flip_y: bool| -> Vec<f32> {
        let mut v = Vec::with_capacity(256);
        for y in 0..16 {
            for x in 0..16 {
                let (sx, sy) = (if flip_x { 15 - x } else { x }, if flip_y { 15 - y } else { y });
                let p = img.pixel(sx * img.width / 16, sy * img.height / 16);
                v.push(0.3 * f32::from(p[0]) + 0.59 * f32::from(p[1]) + 0.11 * f32::from(p[2]));
            }
        }
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        let sd =
            (v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32).sqrt().max(1e-3);
        v.iter().map(|x| (x - mean) / sd).collect()
    };
    let corr =
        |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>() / a.len() as f32;
    let (mut best_as_is, mut judged) = (0, 0);
    for t in tiles.iter().filter(|t| t.image_map_2d.is_some()).take(60) {
        let pic = t.image_map_2d.clone().unwrap().to_lowercase();
        let Ok(ours) = std::fs::read(set_dir.join(format!("{pic}.tga"))) else { continue };
        let ours = mg_image::read(ResType::TGA, &ours).unwrap().to_rgba();
        let Some(theirs) = game
            .texture(mg_core::ResRef::from_str(&pic).unwrap())
            .and_then(|(t, d)| mg_image::read(t, &d).ok())
            .map(|t| t.to_rgba())
        else {
            continue;
        };
        let reference = lum(&theirs, false, false);
        let scores: Vec<f32> = [(false, false), (false, true), (true, false), (true, true)]
            .iter()
            .map(|&(fx, fy)| corr(&lum(&ours, fx, fy), &reference))
            .collect();
        // Symmetric pictures say nothing about the way up.
        if scores.iter().cloned().fold(f32::MIN, f32::max)
            - scores.iter().cloned().fold(f32::MAX, f32::min)
            < 0.2
        {
            continue;
        }
        judged += 1;
        best_as_is += usize::from(scores[1..].iter().all(|s| scores[0] >= *s));
    }
    println!("{best_as_is} of {judged} pictures match best as they are");
    assert!(judged >= 20 && best_as_is * 10 >= judged * 8, "{best_as_is} of {judged}");
    std::fs::write(dir.join("minimaps.png"), mg_module::minimap::png(&sheet).unwrap()).unwrap();
}

/// Candidate areas for the README: the first outdoor areas of a campaign
/// module, from a three-quarter view. `MG_README_MODULE` picks the module
/// (default Chapter 1).
#[test]
#[ignore]
fn readme_area_candidates() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("readme-candidates");
    let module = std::env::var("MG_README_MODULE").unwrap_or("Chapter1.nwm".into());
    let path = root.join("data/nwm").join(&module);
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    let areas: Vec<_> = {
        let ws = app.ws.as_mut().unwrap();
        let all = ws.module.areas().unwrap();
        all.into_iter()
            .filter(|a| {
                let are = ws.doc(&ResKey::new(*a, ResType::ARE)).unwrap();
                let interior = are.root.integer("Flags").unwrap_or(0) & 1 != 0;
                !interior
            })
            .take(10)
            .collect()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 960.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    for area in areas {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
        h.run_steps(5);
        if let Some(view) = h.state_mut().area_views.get_mut(&area)
            && let Some(o) = &mut view.orbit
        {
            o.pitch = 38f32.to_radians();
            o.yaw = -60f32.to_radians();
            o.distance *= 0.55;
        }
        h.run_steps(8);
        shoot(&mut h, &dir, &format!("{area}"));
    }
}

/// The README's screenshots, of the original campaign's first chapter,
/// into `target/test-output/readme/`. They're cropped above the log pane
/// (which shows the install's path) when copied to `docs/images`.
#[test]
#[ignore]
fn area_grid() {
    // The tile grid among buildings and trees: hidden behind them.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.grid = true;
        if let Some(o) = &mut view.orbit {
            o.pitch = 30f32.to_radians();
            o.yaw = -55f32.to_radians();
            o.distance *= 0.3;
            o.target.x -= 8.0;
        }
    }
    h.run_steps(10);
    shoot(&mut h, &dir, "area-grid");
}

#[test]
#[ignore]
fn area_toolbar_narrow() {
    // The area's toolbar in a narrow tab: its rows end at the tab's edge.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(5);
    h.run_steps(10);
    shoot(&mut h, &dir, "area-toolbar-narrow");
}

#[test]
#[ignore]
fn lighting_off_and_sound_ranges() {
    // The area with the most placed sounds, at night: with the area's
    // lighting, then without it and with the sounds' ranges, the camera
    // raised.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let ws = app.ws.as_mut().unwrap();
    let areas = ws.module.areas().unwrap();
    let sounds = |ws: &mut mg_edit::Workspace, a: mg_core::ResRef| {
        let git = ws.doc(&ResKey::new(a, ResType::GIT)).unwrap();
        git.root.list("SoundList").map_or(0, <[_]>::len)
    };
    let area = *areas.iter().max_by_key(|a| sounds(ws, **a)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.night = true;
        if let Some(o) = &mut view.orbit {
            o.pitch = 50f32.to_radians();
            o.distance *= 0.6;
        }
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "lighting-on");
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.lit = false;
        view.sound_ranges = true;
        if let Some(o) = &mut view.orbit {
            o.target.z += 5.0;
        }
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "lighting-off-sound-ranges");
}

#[test]
#[ignore]
fn areas_by_name_and_area_properties() {
    // Options › General › Show areas by name: the tree and the tabs; and
    // Area Properties' Visual and Audio pages (headings, field labels).
    mg_testkit::gpu::hold();
    use egui_kittest::kittest::Queryable;
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    app.settings.area_names = true;
    app.settings.name_resrefs = true;
    app.settings.light_theme = std::env::var_os("MG_LIGHT").is_some();
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::AreaProperties(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    h.get_by_label("Visual").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "area-names-visual");
    h.get_by_label("Audio").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "area-names-audio");
}

#[test]
#[ignore]
fn store_markers() {
    // A store among what stands around it: its arrow, or the game's
    // sign, is in the scene, hidden by what is in front of it.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let ws = app.ws.as_mut().unwrap();
    let areas = ws.module.areas().unwrap();
    let store = |ws: &mut mg_edit::Workspace, a: mg_core::ResRef| {
        let git = ws.doc(&ResKey::new(a, ResType::GIT)).unwrap();
        let s = git.root.list("StoreList")?.first()?;
        Some(glam::Vec3::new(s.float("XPosition")?, s.float("YPosition")?, s.float("ZPosition")?))
    };
    let (area, at) = areas.iter().find_map(|a| store(ws, *a).map(|p| (*a, p))).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.lit = false;
        if let Some(o) = &mut view.orbit {
            o.target = at;
            o.pitch = 50f32.to_radians();
            o.yaw = -2.2;
            o.distance = 6.0;
        }
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "store-arrow");
    // Options › Area: the game's $ sign.
    h.state_mut().settings.merchant_signs = true;
    h.run_steps(5);
    shoot(&mut h, &dir, "store-sign");
}

#[test]
#[ignore]
fn areas_edited_together() {
    // Edit › Edit Areas Together…: the chooser, every area ticked, and the
    // Area Properties of those ticked.
    mg_testkit::gpu::hold();
    use egui_kittest::kittest::Queryable;
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    app.settings.area_names = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Edit Areas Together…").click();
    h.run_steps(2);
    h.get_by_label("Tick Shown").click();
    h.run_steps(2);
    shoot(&mut h, &dir, "areas-chooser");
    h.get_all_by_label_contains(" Together").last().unwrap().click();
    h.run_steps(3);
    h.get_all_by_label("Audio").last().unwrap().click();
    h.run_steps(3);
    shoot(&mut h, &dir, "areas-together-audio");
}

#[test]
#[ignore]
fn raw_fields_with_row_names() {
    // Fields of an area's GIT: its music and sounds by name.
    mg_testkit::gpu::hold();
    use egui_kittest::kittest::Queryable;
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let git = ResKey::parse("map_m1q1a", ResType::GIT).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Gff(git)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    h.get_all_by_label_contains(" fields").next().unwrap().click();
    h.run_steps(4);
    shoot(&mut h, &dir, "raw-fields");
}

#[test]
#[ignore]
fn spawn_point_markers() {
    // An encounter's spawn points: the game's marker, its post and the
    // arrow of its facing.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let ws = app.ws.as_mut().unwrap();
    let areas = ws.module.areas().unwrap();
    let spawn = |ws: &mut mg_edit::Workspace, a: mg_core::ResRef| {
        let git = ws.doc(&ResKey::new(a, ResType::GIT)).unwrap();
        let p = git
            .root
            .list("Encounter List")?
            .iter()
            .find_map(|e| e.list("SpawnPointList")?.first())?;
        Some(glam::Vec3::new(p.float("X")?, p.float("Y")?, p.float("Z")?))
    };
    let (area, at) = areas.iter().find_map(|a| spawn(ws, *a).map(|p| (*a, p))).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.lit = false;
        if let Some(o) = &mut view.orbit {
            o.target = at;
            o.pitch = 40f32.to_radians();
            o.distance = 8.0;
        }
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "spawn-points");
}

#[test]
#[ignore]
fn creature_appearance_inline() {
    // A creature's Appearance page with its model in the page.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    app.open_palette = false;
    let utc = ResKey::parse("m1q1faribeth", ResType::UTC).unwrap();
    app.blueprint_pages.insert((utc, mg_edit::GffPath::root()), "Appearance");
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(utc)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    place(&mut h, &mg_ui::Tab::Blueprint(utc), (250.0, 60.0), (800.0, 900.0));
    h.run_steps(20);
    shoot(&mut h, &dir, "creature-appearance");
}

#[test]
#[ignore]
fn crosser_cursor() {
    // The Road brush's quarter under a pointer resting on the ground of a
    // rural area.
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Crossers", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = dir.join("crossers.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    h.get_by_label("🗻").click();
    h.run_steps(2);
    // Corners (1, 1) and (1, 2) raised twice: tile (1, 1) slopes up to its
    // west.
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    for corner in [(10.0, 10.0), (10.0, 20.0)] {
        for _ in 0..2 {
            let at = h.state().area_views[&area]
                .screen_pos(glam::Vec3::new(corner.0, corner.1, 0.0))
                .unwrap();
            h.hover_at(at);
            for pressed in [true, false] {
                let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
                h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
            }
            h.run_steps(3);
        }
    }
    h.get_by_label("Road").click();
    h.run_steps(2);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        if let Some(o) = &mut view.orbit {
            o.pitch = 40f32.to_radians();
            o.yaw = -60f32.to_radians();
        }
    }
    h.run_steps(3);
    // The pointer over tile (1, 1), near its west edge: the triangle should
    // be under it.
    let at = h.state().area_views[&area].screen_pos(glam::Vec3::new(13.0, 15.0, 2.0)).unwrap();
    h.hover_at(at);
    h.run_steps(3);
    shoot(&mut h, &dir, "crosser-cursor");
}

#[test]
#[ignore]
fn area_fog() {
    // Medieval City 2's thick fog, from a toolset's usual distance.
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module, tilesets};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let city = tilesets(&game).into_iter().find(|t| t.name == "Medieval City 2").unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Fog", &mut rng).unwrap();
    let spec = AreaSpec { name: "City".into(), tileset: city.resref, width: 16, height: 16 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = dir.join("fog.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.fog = true;
        if let Some(o) = &mut view.orbit {
            o.pitch = 25f32.to_radians();
            o.distance = 60.0;
        }
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "area-fog");
}

#[test]
#[ignore]
fn city_tile_selection() {
    // Medieval City 2 builds its ground above its tiles' heights: the
    // selection lies on it, not under it.
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module, tilesets};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let city = tilesets(&game).into_iter().find(|t| t.name == "Medieval City 2").unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Tiles", &mut rng).unwrap();
    let spec = AreaSpec { name: "City".into(), tileset: city.resref, width: 8, height: 8 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = dir.join("city-tiles.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.tile_mode = true;
        view.tile_selection = (1..7).flat_map(|x| [(x, 2), (x, 3)]).collect();
        if let Some(o) = &mut view.orbit {
            o.pitch = 12f32.to_radians();
            o.distance = 70.0;
        }
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "city-tile-selection");
}

#[test]
#[ignore]
fn tile_preview() {
    // A barn under the pointer before it is placed.
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Preview", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 6,
        height: 6,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = dir.join("preview.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        if let Some(o) = &mut view.orbit {
            o.pitch = 40f32.to_radians();
            o.yaw = -60f32.to_radians();
            o.distance = 55.0;
        }
    }
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();
    h.run_steps(2);
    let at = h.state().area_views[&area].screen_pos(glam::Vec3::new(25.0, 25.0, 0.0)).unwrap();
    h.hover_at(at);
    h.run_steps(5);
    shoot(&mut h, &dir, "tile-preview");
}

#[test]
#[ignore]
fn area_properties_advanced() {
    // The shader flags in rows of their own, clear of Variables.
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Props", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = dir.join("props.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.open_palette = false;
    let are = ResKey::new(area, ResType::ARE);
    app.blueprint_pages.insert((are, GffPath::root()), "Advanced");
    // (A loading screen with a picture, to see it under the choice.)
    let edit = mg_edit::Edit::SetField {
        key: are,
        path: GffPath::root(),
        label: "LoadScreenID".into(),
        value: Some(mg_gff::Value::Word(2)),
    };
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::AreaProperties(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    place(&mut h, &mg_ui::Tab::AreaProperties(area), (260.0, 70.0), (780.0, 650.0));
    h.run_steps(3);
    shoot(&mut h, &dir, "area-properties-advanced");
}

#[test]
#[ignore]
fn area_properties_grass() {
    // The Grass page beside the area's view: the tileset's grass off, the
    // area's own on grass (red) and on stone.
    use mg_area::grass::{GrassLook, GrassOverride};
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Props", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = dir.join("grass.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.open_palette = false;
    let are = ResKey::new(area, ResType::ARE);
    app.blueprint_pages.insert((are, GffPath::root()), "Grass");
    let git = ResKey::new(area, ResType::GIT);
    let look = |texture: &str, color: [f32; 3]| GrassLook {
        texture: texture.into(),
        density: 6.0,
        height: 1.0,
        ambient: color,
        diffuse: color,
    };
    let red = GrassOverride { material: 3, look: look("grass", [0.8, 0.2, 0.1]) };
    let stone = GrassOverride { material: 4, look: look("ttz_grass", [0.4, 0.7, 0.3]) };
    let mut overrides = mg_gff::Struct::new(mg_area::grass::OVERRIDES_ID);
    overrides.set(mg_area::grass::LIST, mg_gff::Value::List(vec![red.item(), stone.item()]));
    let edits = vec![
        mg_area::grass::set_disabled(git, true),
        mg_edit::Edit::SetField {
            key: git,
            path: mg_area::grass::properties_path(),
            label: mg_area::grass::OVERRIDES.into(),
            value: Some(mg_gff::Value::Struct(overrides)),
        },
    ];
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::AreaProperties(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    if let Some(o) = &mut h.state_mut().area_views.get_mut(&area).unwrap().orbit {
        o.target = glam::Vec3::new(20.0, 20.0, 0.0);
        (o.pitch, o.yaw, o.distance) = (35f32.to_radians(), -60f32.to_radians(), 14.0);
    }
    place(&mut h, &mg_ui::Tab::AreaProperties(area), (800.0, 70.0), (580.0, 760.0));
    h.run_steps(3);
    shoot(&mut h, &dir, "area-properties-grass");
}

#[test]
#[ignore]
fn area_ghost() {
    // A placeable chosen in the palette: see-through under the pointer.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        if let Some(o) = &mut view.orbit {
            // Close, on open cobbles, the square behind it. (Under the
            // middle of the view the other area shots have, it stood
            // half in the tower's wall, before a banner, and hardly
            // showed.)
            o.pitch = 45f32.to_radians();
            o.yaw = 125f32.to_radians();
            o.distance = 16.0;
            o.target = glam::Vec3::new(50.3, 37.4, 4.0);
        }
    }
    h.state_mut().palette.selected = Some(ResKey::parse("plc_statue1", ResType::UTP).unwrap());
    h.run_steps(3);
    let rect = h.state().area_views[&area].rect;
    h.event(egui::Event::PointerMoved(rect.center()));
    h.run_steps(5);
    shoot(&mut h, &dir, "area-ghost");
}

#[test]
#[ignore]
fn area_items() {
    // Items lying in an area, selected: a cloak (the game's folded one, in
    // the cloak's colors), a healer's kit, a gem and a potion, each with
    // its box.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(5);
    let git = ResKey::new(area, ResType::GIT);
    let first = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git).unwrap().root.list("List").map_or(0, <[_]>::len)
    };
    let mut edits = Vec::new();
    {
        let app = h.state();
        let game = app.game.as_deref().unwrap();
        let item =
            |r: mg_core::ResRef| mg_module::gff_root(None, game, &ResKey::new(r, ResType::UTI));
        let placing = mg_module::instances::Placing { game, item: &item };
        let blueprint = |name: &str| item(mg_core::ResRef::from_str(name).unwrap()).unwrap();
        let mut cloak = blueprint("nw_it_medkit001");
        cloak.set("BaseItem", mg_gff::Value::Int(80));
        cloak.set("ModelPart1", mg_gff::Value::Byte(1));
        // (Shiny colors: their palettes' alpha, about half, is how much
        // of the environment map shows, as the game draws them.)
        for (field, color) in
            [("Cloth1Color", 56), ("Cloth2Color", 57), ("Leather1Color", 58), ("Leather2Color", 59)]
        {
            cloak.set(field, mg_gff::Value::Byte(color));
        }
        let items = [
            cloak,
            blueprint("nw_it_medkit001"),
            blueprint("nw_it_gem001"),
            blueprint("nw_it_mpotion001"),
        ];
        for (i, uti) in items.iter().enumerate() {
            let at = mg_module::instances::Placement {
                position: [49.0 + i as f32 * 1.2, 37.4, 4.0],
                rotation: 0.0,
            };
            let mut s =
                mg_module::instances::instance(&placing, ResType::UTI, uti, at, &[]).unwrap();
            // (On the cobbles there.)
            s.set("ZPosition", mg_gff::Value::Float(4.0));
            edits.push(mg_edit::Edit::insert_item(git, GffPath::root(), "List", first + i, s));
        }
    }
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Items", edits)));
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        // (By their places in the area's list of items.)
        view.selection = (0..4).map(|i| (mg_area::ObjectKind::Item, first + i)).collect();
        if let Some(o) = &mut view.orbit {
            o.pitch = 40f32.to_radians();
            o.yaw = 125f32.to_radians();
            o.distance = 6.0;
            o.target = glam::Vec3::new(50.8, 37.4, 4.0);
        }
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "area-items");
}

#[test]
#[ignore]
fn area_dropped_cloaks() {
    // Aurora's probe of cloaks lying in an area (captures/items/, made for
    // a builder's report after 1.32.1): the game's cloak, colors 0, the
    // shiny 56 to 59, 61 (all environment map) and a potion, in a row
    // across the middle of a grass area, from above as Aurora's
    // `cloaks-top.png` shows them.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let module = mg_testkit::aurora_capture!("items/zz_cloaks.mod");
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&module);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("field").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(5);
    for (name, pitch, distance) in [("top", 89.0f32, 9.0), ("tilted", 55.0, 9.0)] {
        {
            let view = h.state_mut().area_views.get_mut(&area).unwrap();
            if let Some(o) = &mut view.orbit {
                o.pitch = pitch.to_radians();
                o.yaw = -90f32.to_radians();
                o.distance = distance;
                o.target = glam::Vec3::new(20.5, 20.0, 0.0);
            }
        }
        h.run_steps(5);
        shoot(&mut h, &dir, &format!("area-dropped-cloaks-{name}"));
    }
}

#[test]
#[ignore]
fn tile_gallery() {
    // The tiles palette's Gallery: each terrain's and crosser's tiles as
    // pictures under it; one chosen.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    app.settings.palette_gallery = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(5);
    h.state_mut().palette.tiles = true;
    for _ in 0..40 {
        h.run_steps(2);
    }
    shoot(&mut h, &dir, "tile-gallery");
    // Every heading opened: the tiles as pictures, as the blueprint
    // palettes' Gallery draws theirs.
    // (The palette's: the log's fold button is another ⏷.)
    let palette = h.get_by_label("Tiles").rect().right();
    let fold = h.get_all_by_label("⏷").find(|n| n.rect().left() > palette - 300.0).unwrap();
    fold.click();
    // The features first: groups, as pictures of their tiles together.
    for _ in 0..60 {
        h.run_steps(2);
    }
    shoot(&mut h, &dir, "tile-gallery-groups");
    // A group of several tiles, each in its place.
    h.state_mut().palette.filter = "2x2".into();
    for _ in 0..60 {
        h.run_steps(2);
    }
    shoot(&mut h, &dir, "tile-gallery-2x2");
    h.state_mut().palette.filter.clear();
    h.run_steps(4);
    h.get_by_label("Bridge").scroll_to_me();
    for _ in 0..40 {
        h.run_steps(2);
    }
    shoot(&mut h, &dir, "tile-gallery-open");
    // A tile's view, as a double click on its picture opens it.
    let tileset = mg_core::ResRef::from_str("tcn01").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Tile { tileset, tile: 10 }));
    for _ in 0..10 {
        h.run_steps(2);
    }
    shoot(&mut h, &dir, "tile-view");
}

#[test]
#[ignore]
fn area_paint() {
    // The placeable brush: two placeables chosen, Paint on; its circle on
    // the ground and the dab a press would lay, see-through.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        if let Some(o) = &mut view.orbit {
            o.pitch = 35f32.to_radians();
            o.yaw = -55f32.to_radians();
            o.distance *= 0.25;
            o.target.x -= 8.0;
        }
    }
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.selected = Some(ResKey::parse("plc_tree", ResType::UTP).unwrap());
        app.palette.chosen = vec![ResKey::parse("plc_shrub", ResType::UTP).unwrap()];
        app.paint = true;
        app.settings.paint.radius_cm = 600;
        app.settings.paint.density = 3;
    }
    h.run_steps(3);
    // A stroke across the square, let go; then the pointer beside it.
    let rect = h.state().area_views[&area].rect;
    let (from, to) =
        (rect.center() + egui::vec2(-260.0, 40.0), rect.center() + egui::vec2(-60.0, 120.0));
    let button = |h: &mut Harness<'_, Moonglow>, pos, pressed| {
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.event(egui::Event::PointerButton { pos, button, pressed, modifiers });
        h.run_steps(1);
    };
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(2);
    button(&mut h, from, true);
    for k in 1..=10 {
        h.event(egui::Event::PointerMoved(from + (to - from) * (k as f32 / 10.0)));
        h.run_steps(1);
    }
    button(&mut h, to, false);
    h.event(egui::Event::PointerMoved(rect.center() + egui::vec2(-120.0, -60.0)));
    h.run_steps(5);
    shoot(&mut h, &dir, "area-paint");
    // A row of benches dragged out.
    {
        let app = h.state_mut();
        app.palette.selected = Some(ResKey::parse("plc_bench", ResType::UTP).unwrap());
        app.palette.chosen.clear();
        app.settings.paint.row = true;
        app.settings.paint.gap_cm = 40;
    }
    let (from, to) =
        (rect.center() + egui::vec2(-160.0, 170.0), rect.center() + egui::vec2(100.0, 200.0));
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(2);
    button(&mut h, from, true);
    for k in 1..=10 {
        h.event(egui::Event::PointerMoved(from + (to - from) * (k as f32 / 10.0)));
        h.run_steps(1);
    }
    h.run_steps(3);
    // (The row is shot laid: see-through, it hardly shows here.)
    button(&mut h, to, false);
    h.event(egui::Event::PointerMoved(to + egui::vec2(40.0, -40.0)));
    h.run_steps(3);
    shoot(&mut h, &dir, "area-paint-row");
    // A fill of barrels and woodpiles being dragged out, its grid turned
    // 30° by E: its rectangle on the ground, the pieces see-through, each
    // by its own size.
    {
        let app = h.state_mut();
        app.palette.selected = Some(ResKey::parse("plc_barrel", ResType::UTP).unwrap());
        app.palette.chosen = vec![ResKey::parse("plc_woodpile", ResType::UTP).unwrap()];
        (app.settings.paint.row, app.settings.paint.fill) = (false, true);
        app.settings.paint.gap_cm = 20;
    }
    let (from, to) =
        (rect.center() + egui::vec2(-330.0, 60.0), rect.center() + egui::vec2(-170.0, 170.0));
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(2);
    for _ in 0..2 {
        h.key_press(egui::Key::E);
        h.run_steps(1);
    }
    button(&mut h, from, true);
    for k in 1..=10 {
        h.event(egui::Event::PointerMoved(from + (to - from) * (k as f32 / 10.0)));
        h.run_steps(1);
    }
    h.run_steps(3);
    shoot(&mut h, &dir, "area-paint-fill");
    button(&mut h, to, false);
    h.run_steps(3);
}

#[test]
#[ignore]
fn readme_screenshots() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("readme");
    let chapter = root.join("data/nwm/Chapter1.nwm");
    let harness = || {
        let rs = egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        app.set_render_state(rs.clone());
        app.open_module(&chapter);
        // (The area's shot opens the palette beside it; the others are
        // windows alone.)
        app.open_palette = false;
        let mut h = Harness::builder()
            .with_size(egui::vec2(1600.0, 960.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(3);
        h
    };
    let key = |name: &str, t: ResType| ResKey::parse(name, t).unwrap();

    // The area view, with the palette beside it.
    let mut h = harness();
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    h.state_mut().palette.kind = mg_module::palette::BlueprintKind::Placeable;
    h.state_mut().palette.tiles = false;
    h.run_steps(5);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        for k in [
            mg_area::ObjectKind::Trigger,
            mg_area::ObjectKind::Encounter,
            mg_area::ObjectKind::Sound,
            mg_area::ObjectKind::Waypoint,
        ] {
            view.show[k.index()] = false;
        }
        view.show_start = false;
        view.grid = false;
        if let Some(o) = &mut view.orbit {
            o.pitch = 34f32.to_radians();
            o.yaw = -55f32.to_radians();
            o.distance *= 0.42;
            o.target.x -= 8.0;
        }
    }
    h.run_steps(10);
    shoot(&mut h, &dir, "area");

    // Aribeth's conversation.
    let mut h = harness();
    let dlg = mg_ui::Tab::Dialog(key("m1q1faribeth", ResType::DLG));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(dlg.clone()));
    h.run_steps(3);
    place(&mut h, &dlg, (250.0, 70.0), (1320.0, 725.0));
    h.run_steps(5);
    shoot(&mut h, &dir, "conversation");

    // Aribeth: her blueprint beside her model.
    let mut h = harness();
    let utc = key("m1q1faribeth", ResType::UTC);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(utc)));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(utc)));
    h.run_steps(3);
    for (tab, x) in [(mg_ui::Tab::Blueprint(utc), 250.0), (mg_ui::Tab::Model(utc), 960.0)] {
        let path = h.state().dock.find_tab(&tab).unwrap();
        if let Some(w) = h.state_mut().dock.get_window_state_mut(path.surface) {
            w.set_position(egui::pos2(x, 70.0))
                .set_size(egui::vec2(if x < 500.0 { 700.0 } else { 600.0 }, 725.0));
        }
    }
    h.run_steps(20);
    shoot(&mut h, &dir, "creature");

    // A script.
    let mut h = harness();
    let script = mg_ui::Tab::Script(key("m1q3_formosa_9", ResType::NSS));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(script.clone()));
    h.run_steps(3);
    place(&mut h, &script, (250.0, 70.0), (1320.0, 725.0));
    h.run_steps(5);
    let _ = h.query_by_label("Compile");
    shoot(&mut h, &dir, "script");
}

/// Moves and sizes a tab's window.
fn place(h: &mut Harness<'_, Moonglow>, tab: &mg_ui::Tab, at: (f32, f32), size: (f32, f32)) {
    let path = h.state().dock.find_tab(tab).unwrap();
    if let Some(w) = h.state_mut().dock.get_window_state_mut(path.surface) {
        w.set_position(egui::pos2(at.0, at.1)).set_size(egui::vec2(size.0, size.1));
    }
}

/// The Command Palette over the window, something typed
/// (`target/test-output/screens/command-palette.png`).
#[test]
#[ignore]
fn command_palette() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-command-palette");
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    let mut finder = mg_ui::commands::Finder::default();
    finder.filter = "mod".into();
    app.command_palette = Some(finder);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    shoot(&mut h, &dir, "command-palette");
}

/// The Plugins window with the plugin host's fixture plugins installed and
/// one enabled, the console open (`target/test-output/screens-plugins/`).
#[test]
#[ignore]
fn plugins_window() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-plugins");
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.plugin_dir =
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mg-plugin/tests/fixtures"));
    app.load_plugins();
    app.enable_plugin("example.tag-conventions", true);
    app.plugins.window = true;
    app.plugins.console = "return ctx.module:resources(\"utc\")".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Console").click();
    h.run();
    shoot(&mut h, &dir, "plugins");
    h.state_mut().plugins.window = false;
    h.run();
    h.get_by_label("Plugins").click();
    h.run();
    shoot(&mut h, &dir, "plugins-menu");
    h.key_press(egui::Key::Escape);
    h.run();

    // Install from File: what it said, and the question before it
    // replaces one installed (in a plugins folder of this test's own).
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mg-plugin/tests/fixtures/tag-conventions");
    let plugin = mg_plugin::Plugin::load(&fixture).unwrap();
    let archive = dir.join("tag-conventions-1.0.0.zip");
    std::fs::write(&archive, mg_plugin::pack(&plugin).unwrap()).unwrap();
    h.state_mut().plugin_dir = Some(dir.join("plugins"));
    h.state_mut().load_plugins();
    h.state_mut().install_plugin_from(&archive);
    h.run();
    shoot(&mut h, &dir, "plugins-installed");
    h.state_mut().install_plugin_from(&archive);
    h.run();
    h.run();
    shoot(&mut h, &dir, "plugins-replace");
}

/// The application with `app`'s NUI window `key` open in a window of
/// `size`, at the top left of the room under the toolbar.
fn nui_harness(app: Moonglow, key: ResKey, size: (f32, f32)) -> Harness<'static, Moonglow> {
    let mut app = app;
    let tab = mg_ui::Tab::Nui(key);
    app.actions.push(mg_ui::Action::OpenTab(tab.clone()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(size.0 + 40.0, size.1 + 110.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    place(&mut h, &tab, (20.0, 90.0), size);
    h.run_steps(3);
    h
}

/// The NUI Creator on a window of settings (`nui-editor.png`), the preview
/// in its Hover state at 150% (`nui-hover-150.png`), a list
/// (`nui-list.png`), Interact (`nui-interact.png`, `-narrow`), groups,
/// a chart and draw layers (`nui-controls.png`), Events & variants
/// (`nui-events.png`) and a narrow window (`nui-compact.png`). Pictures of
/// the editor to look at: what the game draws is the client's to say.
#[test]
#[ignore]
fn nui_creator() {
    use mg_nui::{Binding, Settings};
    use serde_json::json;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let key = ResKey::parse("nui_test", ResType::JUI).unwrap();
    let config_key = ResKey::parse("nui_test", ResType::TXT).unwrap();
    let new = |install: Option<GameInstall>| {
        let mut app = Moonglow::new(install, Box::new(NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
        app.run(mg_ui::Action::NewNui("nui_test".into()));
        app
    };
    let mut sample = new(Some(GameInstall::new(&root, None, "en")));
    let mut window = mg_nui::window();
    window["title"] = "Character settings".into();
    window["geometry"]["w"] = 460.0.into();
    window["geometry"]["h"] = 330.0.into();
    window["root"]["children"][0]["value"] = "Customize your interface".into();
    let mut input = mg_nui::template("textedit");
    input["value"] = json!({"bind":"player_name"});
    let mut check = mg_nui::template("check");
    check["label"] = "Show tooltips".into();
    check["value"] = json!({"bind":"show_tooltips"});
    let mut button = mg_nui::template("button");
    button["label"] = "Save settings".into();
    button["id"] = "save_settings".into();
    let mut combo = mg_nui::template("combo");
    combo["elements"] = json!([["Polski", 0], ["English", 1]]);
    let mut picture = mg_nui::template("image");
    picture["value"] = "nui_button_a".into();
    picture["height"] = 42.0.into();
    window["root"]["children"]
        .as_array_mut()
        .unwrap()
        .splice(1..2, [input, check, combo, mg_nui::template("progress"), picture, button]);
    let mut settings = Settings::default();
    settings.bindings.insert(
        "player_name".into(),
        Binding { value: "Łukasz — Żółw".into(), ..Default::default() },
    );
    settings
        .bindings
        .insert("show_tooltips".into(), Binding { value: true.into(), ..Default::default() });
    let set = |app: &mut Moonglow, window: &serde_json::Value, settings: &Settings| {
        let module = &mut app.ws.as_mut().unwrap().module;
        module.set(key, serde_json::to_vec_pretty(window).unwrap());
        module.set(config_key, settings.bytes());
    };
    set(&mut sample, &window, &settings);
    let tab = mg_ui::Tab::Nui(key);
    let mut h = nui_harness(sample, key, (1350.0, 960.0));
    place(&mut h, &tab, (20.0, 90.0), (1200.0, 850.0));
    h.run_steps(3);
    h.get_by_label("Canvas Button · Save settings").click();
    h.run();
    shoot(&mut h, &dir, "nui-editor");
    h.get_by_label_contains("State: ").click();
    h.run();
    h.get_by_label("Hover").click();
    h.run();
    h.get_by_label("UI scale 100%").click();
    h.run();
    h.get_by_label("150%").click();
    h.run();
    h.get_by_label("Clean view").click();
    h.run();
    shoot(&mut h, &dir, "nui-hover-150");

    // A list and controls that take input.
    let mut list = mg_nui::template("list");
    list["row_count"] = 5.into();
    list["row_height"] = 28.0.into();
    list["row_template"] = json!([
        [{"type":"label","value":{"bind":"items"},"text_halign":1},220.0,true],
        [{"type":"check","label":"Active","value":{"bind":"active"}},110.0,false]
    ]);
    window["geometry"]["h"] = 380.0.into();
    window["root"]["children"] = json!([
        {"type":"label","value":"Inventory settings","height":30.0},
        list,
        mg_nui::template("slider"),
        {"type":"button","label":"Apply","id":"apply","height":30.0}
    ]);
    settings.bindings.insert(
        "items".into(),
        Binding {
            value: json!(["Miecz", "Łuk", "Tarcza", "Zbroja", "Pierścień"]),
            ..Default::default()
        },
    );
    settings.bindings.insert(
        "active".into(),
        Binding { value: json!([true, false, false, true, false]), ..Default::default() },
    );
    set(h.state_mut(), &window, &settings);
    h.get_by_label("UI scale 150%").click();
    h.run();
    h.get_all_by_label("100%").last().unwrap().click();
    h.run();
    h.get_by_label("Clean view").click();
    h.run();
    h.get_by_label("Canvas List").click();
    h.run();
    h.get_by_label("Collapse List").click();
    h.run();
    shoot(&mut h, &dir, "nui-list");
    h.get_by_label("Interact").click();
    h.run();
    h.get_by_label("Canvas Checkbox · Active · row 1").click();
    h.run();
    shoot(&mut h, &dir, "nui-interact");
    place(&mut h, &tab, (20.0, 90.0), (1100.0, 800.0));
    h.run_steps(3);
    h.get_by_label("Canvas window").click();
    h.run();
    shoot(&mut h, &dir, "nui-interact-narrow");

    // A group that shows a variant, a chart, a color picker and draw layers.
    window["title"] = json!("Views, charts and drawing");
    window["geometry"] = json!({"x":-1.0,"y":-1.0,"w":480.0,"h":450.0});
    window["root"]["children"] = json!([
        {"type":"group","id":"view_host","height":70.0,"children":[{"type":"label","value":"Main view · native group"}]},
        {"type":"row","children":[{"type":"button","id":"next","label":"Show details","height":32.0},{"type":"button","id":"mg_close","label":"Close","height":32.0}]},
        {"type":"chart","height":100.0,"value":[{"type":0,"legend":"XP","color":{"r":80,"g":180,"b":240,"a":255},"data":[0.0,2.0,1.0,4.0]}]},
        {"type":"color_picker","height":45.0,"value":{"r":80,"g":180,"b":240,"a":255}},
        {"type":"spacer","height":100.0,"draw_list_scissor":true,"draw_list":[
            {"type":7,"rect":{"x":8.0,"y":8.0,"w":160.0,"h":75.0},"color":{"r":173,"g":142,"b":96,"a":255},"line_thickness":2.0},
            {"type":4,"rect":{"x":18.0,"y":26.0,"w":145.0,"h":40.0},"text":"Draw layer","font":""},
            {"type":0,"points":[195.0,70.0,260.0,20.0,320.0,65.0,395.0,15.0],"color":{"r":80,"g":180,"b":240,"a":255},"line_thickness":3.0}
        ]}
    ]);
    settings.views.insert("details".into(), json!({"type":"label","value":"Details view"}));
    settings.actions = vec![mg_nui::Route {
        event: "click".into(),
        element: "next".into(),
        action: mg_nui::Action::View { group: "view_host".into(), view: "details".into() },
        extra: Default::default(),
    }];
    set(h.state_mut(), &window, &settings);
    place(&mut h, &tab, (20.0, 90.0), (1350.0, 960.0));
    h.run_steps(3);
    h.get_by_label("Canvas Button · Show details").click();
    h.run();
    shoot(&mut h, &dir, "nui-controls");
    h.get_by_label("Events & variants").click();
    h.run();
    shoot(&mut h, &dir, "nui-events");

    // One GPU device at a time: this harness goes before the next is made.
    drop(h);
    let mut compact = nui_harness(new(None), key, (640.0, 720.0));
    compact.get_by_label("Properties").click();
    compact.run();
    shoot(&mut compact, &dir, "nui-compact");
}

/// Properties on a dropdown with a long option, in the Creator's window as
/// it opens and in a narrow one (`nui-inspector-options.png`, `-narrow`):
/// Properties stays beside the canvas, not under it.
#[test]
#[ignore]
fn nui_inspector_options() {
    use serde_json::json;
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens");
    let key = ResKey::parse("options", ResType::JUI).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
    app.run(mg_ui::Action::NewNui("options".into()));
    let mut window = mg_nui::window();
    window["root"]["children"] = json!([{
        "type":"row","children":[{
            "type":"combo","id":"language","value":0,"height":30.0,
            "elements":[["First",0],["An intentionally long option label that must remain editable inside Properties",2147483647]]
        }]
    }]);
    app.ws.as_mut().unwrap().module.set(key, serde_json::to_vec_pretty(&window).unwrap());
    let mut h = nui_harness(app, key, (1130.0, 760.0));
    h.get_by_label_contains("Canvas Dropdown").click();
    h.run();
    shoot(&mut h, &dir, "nui-inspector-options");
    place(&mut h, &mg_ui::Tab::Nui(key), (20.0, 90.0), (800.0, 760.0));
    shoot(&mut h, &dir, "nui-inspector-options-narrow");
}

/// A module's NUI windows for [`nui_saved_window`] and [`nui_saved_windows`]:
/// the module at NUI_PREVIEW_MODULE, read and never written, and its file
/// as it was.
fn nui_saved_module() -> (std::path::PathBuf, Vec<u8>, Vec<ResKey>) {
    let path =
        std::path::PathBuf::from(std::env::var("NUI_PREVIEW_MODULE").expect("NUI_PREVIEW_MODULE"));
    let before = std::fs::read(&path).unwrap();
    let only = std::env::var("NUI_PREVIEW_RESREF").ok();
    let keys = mg_module::Module::open(&path)
        .unwrap()
        .keys()
        .filter(|k| k.restype == ResType::JUI)
        .filter(|k| only.as_ref().is_none_or(|n| k.resref.to_string() == *n))
        .copied()
        .collect();
    (path, before, keys)
}

/// The application on the module at `path`, held in memory.
fn nui_saved_app(root: &std::path::Path, path: &std::path::Path) -> Moonglow {
    let mut app =
        Moonglow::new(Some(GameInstall::new(root, None, "en")), Box::new(NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::open(path).unwrap()));
    app
}

/// The Creator on a window of a module made with it, as it opens: the
/// window NUI_PREVIEW_RESREF of the module NUI_PREVIEW_MODULE, to the file
/// NUI_PREVIEW_PNG. The module is not written.
#[test]
#[ignore]
fn nui_saved_window() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let output = std::env::var("NUI_PREVIEW_PNG").expect("NUI_PREVIEW_PNG");
    let (path, before, keys) = nui_saved_module();
    let [key] = keys[..] else { panic!("NUI_PREVIEW_RESREF names one window of the module") };
    let mut h = nui_harness(nui_saved_app(&root, &path), key, (1600.0, 1040.0));
    h.run_steps(3);
    h.render().expect("render").save(output).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before, "the module is not written");
}

/// Every window of a module (NUI_PREVIEW_MODULE; one with
/// NUI_PREVIEW_RESREF) as the clean preview alone, at 100%, a PNG each in
/// NUI_PREVIEW_DIR: what is put beside the game client's screenshots.
/// NUI_PREVIEW_CLICK="Canvas Dropdown#2" clicks the second such control in
/// Interact first.
#[test]
#[ignore]
fn nui_saved_windows() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = std::path::PathBuf::from(std::env::var("NUI_PREVIEW_DIR").expect("NUI_PREVIEW_DIR"));
    std::fs::create_dir_all(&dir).unwrap();
    let (path, before, keys) = nui_saved_module();
    for key in keys {
        // (One GPU device at a time: a window's harness goes with its turn.)
        let mut h = nui_harness(nui_saved_app(&root, &path), key, (1300.0, 1150.0));
        h.get_by_value("Editor").click();
        h.run();
        h.get_by_label("Preview only").click();
        h.run();
        h.get_by_label("Clean view").click();
        h.run();
        h.get_by_label("100%").click();
        h.run_steps(3);
        if let Ok(click) = std::env::var("NUI_PREVIEW_CLICK") {
            let (label, nth) = click.split_once('#').unwrap_or((&click, "1"));
            h.get_by_label("Interact").click();
            h.run();
            h.get_all_by_label(label).nth(nth.parse::<usize>().unwrap() - 1).unwrap().click();
            h.run_steps(3);
        }
        shoot(&mut h, &dir, &key.resref.to_string());
    }
    assert_eq!(std::fs::read(&path).unwrap(), before, "the module is not written");
}

/// The Advanced Search tab on a copy of the Original Campaign's first
/// chapter: the items giving a bonus to Set Trap, then its fighters.
#[test]
#[ignore]
fn advanced_search() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-advanced-search");
    let path = scratch.join("chapter1.mod");
    std::fs::copy(root.join("data/nwm/Chapter1.nwm"), &path).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::AdvancedSearch));
    h.run_steps(3);
    let mut d = mg_ui::search::AdvancedSearch {
        kind: mg_ui::search::Kind::ItemProperty,
        row: Some(52),
        subtype: Some(15),
        ..Default::default()
    };
    h.state_mut().advanced_search(&mut d);
    h.state_mut().advanced_search = d;
    shoot(&mut h, &dir, "advanced-search-property");
    let mut d = mg_ui::search::AdvancedSearch {
        kind: mg_ui::search::Kind::Class,
        row: Some(4),
        placed: Some(true),
        ..Default::default()
    };
    h.state_mut().advanced_search(&mut d);
    h.state_mut().advanced_search = d;
    shoot(&mut h, &dir, "advanced-search-class");
}

/// Right-click menus, Properties last in each (Aurora's order): a
/// placed waypoint's in the area view, and the area's row in the module
/// tree.
#[test]
#[ignore]
fn context_menus() {
    use mg_module::ModuleLocation;
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Menus", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bp = game.resman.get(&ResKey::parse("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
    let bp = mg_gff::Gff::read(&bp).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: mg_core::ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.3 };
    let waypoint = instance(&placing, ResType::UTW, &bp.root, at, &[]).unwrap();
    git.root.set("WaypointList", mg_gff::Value::List(vec![waypoint]));
    m.set_gff(git_key, &git).unwrap();
    let path = dir.join("context-menus.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    let right_click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    let on_it = h.state().area_views[&area].screen_pos(glam::Vec3::new(20.0, 20.0, 0.0));
    right_click(&mut h, on_it.expect("in view"));
    shoot(&mut h, &dir, "menu-area-object");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    let row = h.get_all_by_label_contains(&area.to_string()).last().unwrap().rect().center();
    right_click(&mut h, row);
    shoot(&mut h, &dir, "menu-tree-area");
}
