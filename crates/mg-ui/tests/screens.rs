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

/// Build Module's Unused results, a script a line with its files, one
/// chosen for Mark as Used, and Counted as Used open with the project
/// file's `[used]` (Jasperre's report, October 2026).
#[test]
#[ignore]
fn build_unused_and_counted_as_used() {
    use mg_module::verify::{ScriptFiles, Unused};
    use mg_ui::build_view::{BuildWindow, Finding};
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-build-used");
    let project = dir.join("project");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    m.save_as(&mg_module::ModuleLocation::Folder(project.clone())).unwrap();
    std::fs::write(
        project.join("moonglow.toml"),
        "[used]\nscripts = [\"ai_*\", \"j_exec_target\"]\nblueprints = [\"spawn_*\"]\n",
    )
    .unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&project);
    let mut w = BuildWindow::default();
    w.unused = true;
    let unused = |n: &str, t, s| {
        let key = ResKey::parse(n, t).unwrap();
        Unused { key, script: s }
    };
    let found = [
        unused("j_ai_old", ResType::NSS, Some(ScriptFiles::Both)),
        unused("j_debug", ResType::NCS, Some(ScriptFiles::CompiledOnly)),
        unused("j_test", ResType::NSS, Some(ScriptFiles::SourceOnly)),
        unused("guard_spare", ResType::UTC, None),
    ];
    w.results = found
        .iter()
        .map(|u| Finding { text: format!("Unused: {u}"), about: Some(u.key), unused: Some(*u) })
        .collect();
    w.selected = Some(0);
    app.build = Some(w);
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 720.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Counted as Used").click();
    shoot(&mut h, &dir, "build-unused-counted-as-used");
    h.get_by_label("Unused: j_debug.ncs (compiled only (no source))").click_secondary();
    shoot(&mut h, &dir, "build-unused-mark-menu");
}

/// Build › Round-Trip Check in its menu, with its tip, and what it says in
/// the log once run (a module with a file that is no GFF under a GFF's
/// name: kept, and said).
#[test]
#[ignore]
fn build_round_trip_check() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-round-trip");
    let path = dir.join("sample.mod");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    m.set(ResKey::parse("broken", ResType::UTP).unwrap(), b"not a gff".to_vec());
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 720.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Build").click();
    h.run();
    h.get_by_label("Round-Trip Check").hover();
    h.run_steps(30);
    shoot(&mut h, &dir, "build-round-trip-menu");
    h.get_by_label("Round-Trip Check").click();
    h.run();
    shoot(&mut h, &dir, "build-round-trip-log");
}

/// A color button's picker, open, with Paste beside it (a builder's
/// report: the picker copied a color and nothing pasted one back).
#[test]
#[ignore]
fn color_button_paste() {
    mg_testkit::gpu::hold();
    let mut h = Harness::builder().with_size(egui::vec2(420.0, 420.0)).wgpu().build_ui_state(
        |ui, rgb: &mut [u8; 3]| {
            mg_ui::color_button::edit_srgb(ui, rgb);
        },
        [158, 151, 211],
    );
    h.run();
    h.get_by(|n| n.role() == egui::accesskit::Role::ColorWell).click();
    h.run_steps(3);
    let image = h.render().expect("render");
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("../test-output/screens");
    std::fs::create_dir_all(&path).unwrap();
    image.save(path.join("color-button-paste.png")).unwrap();
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
    // The Advanced page's numbers, with Aurora's arrows.
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    shoot(&mut h, &dir, "module-properties-advanced");
}

/// The main window's fix package (`target/test-output/screens-window/`):
/// Module Properties' window with its bar's Maximize button (🗖), the
/// window maximized (🗗 Restore), and Custom Content of an older module
/// that names its one hak in `Mod_Hak`. (Run on the code before the
/// package, the same pictures show the bar without its button and the
/// hak list saying the module has none.)
#[test]
#[ignore]
fn window_bar_maximize_and_an_older_hak() {
    use mg_schema::{ExoString, StructExt, ifo};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-window");
    let user = dir.join("older-user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let empty = mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap();
    std::fs::write(user.join("hak/older.hak"), empty).unwrap();
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_HAK, ExoString::from("older"));
    m.set_info(&info).unwrap();
    let path = dir.join("older.mod");
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
    // (Opened again: in a window of its own, not the middle the module
    // opened it in.)
    h.state_mut().actions.push(mg_ui::Action::CloseTab(mg_ui::Tab::ModuleProperties));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::ModuleProperties));
    h.run_steps(8);
    h.get_by_label("Custom Content").click();
    h.run_steps(4);
    shoot(&mut h, &dir, "window-bar-maximize");
    h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(mg_ui::Tab::ModuleProperties));
    h.run_steps(8);
    shoot(&mut h, &dir, "window-bar-restore");
}

/// Build › Publish to NWSync… (`target/test-output/screens-nwsync/`):
/// the window after publishing (`nwsync-published`), and, where the
/// window has it, after Remove Unused Data took a file no manifest names
/// (`nwsync-pruned`). (Run on the code before the install-and-publish fix
/// package, the window has no Remove Unused Data and the first picture is
/// all there is.)
#[test]
#[ignore]
fn nwsync_publish_and_prune() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-nwsync");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    let table = b"2DA V2.0\n\n   Label\n0  synced\n".to_vec();
    w.add(mg_core::ResRef::from_str("mg_synced").unwrap(), ResType::TWODA, table).unwrap();
    // (Enough to take a while, for a picture of it under way: 3000 files
    // of noise.)
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    for i in 0..3000 {
        let noise: Vec<u8> = (0..24_000)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as u8
            })
            .collect();
        let name = mg_core::ResRef::from_str(&format!("mg_noise{i}")).unwrap();
        w.add(name, ResType::TXT, noise).unwrap();
    }
    std::fs::write(user.join("hak/mg_sync.hak"), w.to_bytes().unwrap()).unwrap();
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_HakList", mg_module::attach::hak_list(&info.root, &["mg_sync".into()]));
    m.set_info(&info).unwrap();
    let path = dir.join("synced.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let repo = dir.join("repo");
    let _ = std::fs::remove_dir_all(&repo);
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.open_palette = false;
    app.background_jobs = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 640.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("Build").click();
    h.run_steps(2);
    h.get_by_label("Publish to NWSync…").click();
    h.run_steps(2);
    h.state_mut().publish.as_mut().unwrap().folder = repo.display().to_string();
    h.run_steps(1);
    h.get_by_label("Publish").click();
    h.run_steps(1);
    std::thread::sleep(std::time::Duration::from_millis(150));
    shoot(&mut h, &dir, "nwsync-publishing");
    for _ in 0..1000 {
        h.run_steps(1);
        if h.state().publish.as_ref().is_some_and(|p| p.written.is_some() || p.error.is_some()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    shoot(&mut h, &dir, "nwsync-published");
    eprintln!("{}", dir.join("nwsync-published.png").display());
    if h.query_by_label("Remove Unused Data").is_some() {
        let unused = repo.join("data/sha1/ab/cd").join("ab".repeat(20));
        std::fs::create_dir_all(unused.parent().unwrap()).unwrap();
        std::fs::write(&unused, b"left by a manifest taken away").unwrap();
        let then = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 86_400);
        std::fs::File::options().write(true).open(&unused).unwrap().set_modified(then).unwrap();
        std::fs::write(repo.join("latest"), "0".repeat(40)).unwrap();
        h.get_by_label("Remove Unused Data").click();
        h.run_steps(4);
        shoot(&mut h, &dir, "nwsync-pruned");
        eprintln!("{}", dir.join("nwsync-pruned.png").display());
    }
}

/// Edit › Undo opened from a hak's editor (`target/test-output/screens-undo/`,
/// `edit-menu-over-a-hak`): the hak's step, named ("Undo Add files"), where
/// before the install-and-publish fix package the menu said the module's
/// step ("Undo A table") and took it.
#[test]
#[ignore]
fn side_editor_undo_in_the_edit_menu() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens-undo");
    let file = dir.join("mg_one.2da");
    std::fs::write(&file, "2DA V2.0\n\n   Label\n0  one\n").unwrap();
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let path = dir.join("undo.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    let table = b"2DA V2.0\n\n   Label\n0  Bread\n".to_vec();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(table) }],
    )));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    mg_ui::hak_view::new_hak(h.state_mut());
    h.run_steps(2);
    h.state_mut().haks[0].hak.add_files(std::slice::from_ref(&file));
    h.run_steps(2);
    h.hover_at(h.get_by_label("Add Files…").rect().center());
    h.run_steps(2);
    h.get_by_label("Edit").click();
    shoot(&mut h, &dir, "edit-menu-over-a-hak");
    eprintln!("{}", dir.join("edit-menu-over-a-hak.png").display());
}

/// The widgets of the deferred sprint's first fix package, each where a
/// builder meets it (`target/test-output/screens-widgets/`): Add hak…
/// opened by the keys, its cursor on the second hak; the Command
/// Palette and Open Script with a name typed (the search fields' ×);
/// Resize Area's and Adjust Location's numbers with their arrows.
#[test]
#[ignore]
fn widgets_keys_searches_and_arrows() {
    use mg_schema::{ExoString, StructExt, ifo};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-widgets");
    let user = dir.join("haks-user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    for name in ["bloodwar", "bmmountainsv102", "cb_tile_atemple"] {
        let empty = mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap();
        std::fs::write(user.join(format!("hak/{name}.hak")), empty).unwrap();
    }
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    let mut item = ifo::MOD_HAK_LIST.new_item();
    item.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from("ghcastlehd"));
    info.root.set(ifo::MOD_HAK_LIST.label, mg_gff::Value::List(vec![item]));
    m.set_info(&info).unwrap();
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(script, b"void main()\n{\n}\n".to_vec());
    for n in ["at_door", "at_gate", "at_inn", "inc_loot", "on_death"] {
        m.set(ResKey::parse(n, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    let path = dir.join("widgets.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("Custom Content").click();
    h.run_steps(3);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Add hak…"))
        .map(|n| n.rect().center())
        .expect("Add hak…");
    h.hover_at(at);
    for _ in 0..2 {
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    }
    h.hover_at(egui::pos2(5.0, 690.0));
    for _ in 0..2 {
        h.key_press(egui::Key::ArrowDown);
        h.run_steps(2);
    }
    shoot(&mut h, &dir, "add-hak-by-the-keys");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);

    let mut finder = mg_ui::commands::Finder::default();
    finder.filter = "module".into();
    h.state_mut().command_palette = Some(finder);
    h.run_steps(3);
    shoot(&mut h, &dir, "command-palette-typed");
    h.state_mut().command_palette = None;
    h.run_steps(2);

    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(script)));
    h.run_steps(3);
    h.get_by_label("Open…").click();
    h.run_steps(3);
    h.state_mut().script_tools.open_script.as_mut().unwrap().filter = "at_".into();
    h.run_steps(3);
    shoot(&mut h, &dir, "open-script-typed");
    h.state_mut().script_tools.open_script = None;
    h.run_steps(2);

    let area = mg_core::ResRef::from_str("area001").unwrap();
    h.state_mut().resize_area =
        Some(mg_ui::area_reshape::ResizeDraft { area, rows: 8, columns: 12, edges: 0 });
    h.run_steps(3);
    shoot(&mut h, &dir, "resize-area-arrows");
    h.state_mut().resize_area = None;
    h.state_mut().adjust = Some(mg_ui::area_tools::AdjustLocation {
        area,
        objects: vec![(mg_area::ObjectKind::Placeable, 0)],
        position: glam::Vec3::new(12.5, 31.25, 0.5),
        bearing: 90.0,
        changed: [false; 4],
        visual: mg_area::VisualTransform::default(),
        visual_changed: false,
        shaped: true,
        fixed: 0,
        fixed_shaped: 0,
        uneven: 0,
        start: false,
    });
    h.run_steps(3);
    shoot(&mut h, &dir, "adjust-location-arrows");
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

/// A talk-table line whose text has CRLF line ends (Jasperre's report),
/// with End pressed on its first line and an X typed. With the `\r` in the
/// field, End went past it and the X came after it, a space's width from
/// the line ("First line X", the line end now `\rX\n`); the text it
/// set is written beside the picture.
#[test]
#[ignore]
fn talk_table_crlf_line() {
    use egui_kittest::kittest::Queryable;
    use mg_module::talk::{Line, Table};
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens");
    let app = Moonglow::new(None, Box::new(NoDialogs::default()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 600.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    let mut table = Table::create("crlf", &dir, mg_core::Language::ENGLISH, false);
    for text in ["First line\r\nSecond line\r\nThird line", "Another"] {
        let line =
            Line { text: text.into(), feminine: None, sound: String::new(), sound_length: 0.0 };
        table.add_line(&line).unwrap();
    }
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().talk_view.selected = Some(0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::TalkTable));
    h.run_steps(8);
    let field = h
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("First line")))
        .expect("the line's text");
    field.click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Home);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("X".into()));
    h.run_steps(2);
    shoot(&mut h, &dir, "talk-table-crlf");
    let text = &h.state().talk.as_ref().unwrap().line(0).text;
    std::fs::write(dir.join("talk-table-crlf.txt"), format!("{text:?}\n")).unwrap();
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
        // The split dragged up, the pointer resting on it.
        h.ctx.data_mut(|d| d.insert_persisted(egui::Id::new("talk-split"), 0.7f32));
        h.run_steps(2);
        let strref = Table::strref(27).0;
        let line = h.get_by_label(&format!("StrRef {strref}")).rect().top() - 5.0;
        h.hover_at(egui::pos2(520.0, line));
        shoot(&mut h, &dir, &format!("talk-table-{name}-split"));
    }
}

/// The talk table editor's New Talk Table in a nasher project: the table
/// is made in the project, with a target of its own.
#[test]
#[ignore]
fn talk_table_new_in_a_project() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-talk-new-project");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let project = dir.join("project");
    m.save_as(&mg_module::ModuleLocation::Project {
        root: project.clone(),
        target: "default".into(),
    })
    .unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&project);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 600.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::TalkTable));
    h.run_steps(3);
    h.state_mut().talk_view.new_name = "mg_new".into();
    shoot(&mut h, &dir, "talk-new-in-project");
    eprintln!("{}", dir.join("talk-new-in-project.png").display());
}

/// The welcome page's Recent modules: a `.mod`, the module folder beside
/// it and a nasher project (GitHub issue 30: the folder showed as its
/// name alone).
#[test]
#[ignore]
fn welcome_recent_modules() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-welcome");
    let modules = dir.join("modules");
    std::fs::create_dir_all(modules.join("castle")).unwrap();
    std::fs::write(modules.join("castle.mod"), b"").unwrap();
    std::fs::create_dir_all(dir.join("projects/castle-project")).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.recent = vec![
        modules.join("castle"),
        modules.join("castle.mod"),
        dir.join("projects/castle-project"),
    ];
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 600.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    shoot(&mut h, &dir, "welcome-recent");
    eprintln!("{}", dir.join("welcome-recent.png").display());
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
    // The graphics backend's choices, opened.
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let automatic =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().value().as_deref() == Some("Automatic");
    h.query_all(combo).find(automatic).expect("the graphics backend's dropdown").click();
    shoot(&mut h, &dir, "options-general-graphics-backend");
    h.key_press(egui::Key::Escape);
    h.run();
    // Further down: the warnings, duplicate tags' among them.
    h.get_by_label("Show duplicate tags warning after a paste or a copy").scroll_to_me();
    shoot(&mut h, &dir, "options-general-warnings");
    // The Area page, tiles of groups free to edit: its warning.
    h.state_mut().options.as_mut().unwrap().edit_group_tiles = true;
    h.get_by_label("Area").click();
    shoot(&mut h, &dir, "options-area");
    h.get_by_label("Script Editor").click();
    shoot(&mut h, &dir, "options-script-editor");
    // Further down: the colors, the background and the preview.
    h.get_by_label("Reset to defaults").scroll_to_me();
    shoot(&mut h, &dir, "options-script-editor-colors");
    // (A line in the log, beside its Clear.)
    h.state_mut().log.info("Verify: 0 error(s), 0 warning(s)");
    let draft = h.state_mut().options.as_mut().unwrap();
    draft.verify_skip = vec!["tilesets".into(), "mdl-text".into(), "references:creatures".into()];
    draft.verify_more = "tdod_tiles01.hak, ttr01.set, tilesetz".into();
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

/// A project's commands in File › Project (with their keys), the Command
/// Palette and Options › Keyboard; and the Builder Code window of a team
/// prefix without a code (`project-commands-menu`, `-palette`,
/// `-keyboard`, `team-prefix-builder-code`).
#[test]
#[ignore]
fn project_commands_and_team_prefix() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-project-commands");
    let project = dir.join("project");
    let mut m = mg_module::Module::new();
    let mut ifo = mg_gff::Gff::new(*b"IFO ");
    ifo.root.set("Mod_Tag", mg_gff::Value::String(b"KREZK".to_vec()));
    m.set_info(&ifo).unwrap();
    m.save_as(&mg_module::ModuleLocation::Folder(project.clone())).unwrap();
    std::fs::write(
        project.join("moonglow.toml"),
        "[resrefs]\npattern = \"tdod_{kind}{name}\"\nkinds = { creature = \"n_\" }\n\n\
         [commands]\nupdate = { run = \"bin/update\", key = \"Ctrl+Shift+U\" }\n\
         submit = { run = \"bin/submit\", label = \"Submit for Review\", writes = false }\n",
    )
    .unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&project);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Project").hover();
    h.run();
    h.get_by_label("Project").click();
    shoot(&mut h, &dir, "project-commands-menu");
    h.key_press(egui::Key::Escape);
    h.key_press(egui::Key::Escape);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
    h.run();
    h.event(egui::Event::Text("pro".into()));
    shoot(&mut h, &dir, "project-commands-palette");
    h.key_press(egui::Key::Escape);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Keyboard").click();
    h.run();
    h.get_by_label("Update").scroll_to_me();
    shoot(&mut h, &dir, "project-commands-keyboard");
    h.state_mut().options = None;
    h.run();
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Builder Code…").click();
    shoot(&mut h, &dir, "team-prefix-builder-code");
}

#[test]
#[ignore]
fn model_viewer() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-model");
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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

/// A model in the model view with the game's shaders on and off, to look
/// at what they draw differently: `MG_SCREEN_MODEL` (the model's name,
/// `c_golemerald` by default) from the game and `MG_TEST_USER`'s
/// `override` (a builder's model copied there).
#[test]
#[ignore]
fn model_view_game_shaders() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-model");
    let name = std::env::var("MG_SCREEN_MODEL").unwrap_or_else(|_| "c_golemerald".into());
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    for on in [true, false] {
        let rs = egui_kittest::wgpu::create_render_state(
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let install = GameInstall::new(&root, user.clone(), "en");
        let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
        app.settings.game_shaders = Some(on);
        app.set_render_state(rs.clone());
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(
            ResKey::parse(&name, ResType::MDL).unwrap(),
        )));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1100.0, 760.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        // (The game's programs are made on threads of their own while the
        // view is drawn: time for them.)
        for _ in 0..50 {
            h.step();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let shaders = if on { "game-shaders" } else { "own-shader" };
        shoot(&mut h, &dir, &format!("model-{name}-{shaders}"));
    }
}

/// Creatures placed in an area, in the area view, with the game's shaders
/// on and off, selected and not, to look at what they draw differently
/// there: appearance.2da rows `MG_SCREEN_APPEARANCES` (comma-separated,
/// a badger's by default) of a builder's haks `MG_SCREEN_HAKS` (names,
/// highest first, found in `MOONGLOW_HAKS`), the camera
/// `MG_SCREEN_DISTANCE` meters from the first and `MG_SCREEN_YAW` degrees
/// round it (from +X). `MG_SCREEN_LIGHT` and `MG_SCREEN_OVERRIDE` below.
/// The whole window and the view about each creature go to
/// `target/test-output/screens-area/`. (The Domains of Dread's dire wolf,
/// 31002, lit from behind: a light line along its fur cards' cut edges
/// with the game's shaders, which the client draws too: `creatures_look`
/// in `client_render.rs`.)
#[test]
#[ignore]
fn area_creatures_game_shaders() {
    use mg_module::ModuleLocation;
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-area");
    let scratch = mg_testkit::scratch_dir("screens-area-creatures");
    let user = scratch.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let haks: Vec<String> = std::env::var("MG_SCREEN_HAKS")
        .map(|v| v.split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    for name in &haks {
        let file = format!("{name}.hak");
        let from = mg_testkit::builder_hak_dirs()
            .into_iter()
            .map(|d| d.join(&file))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("{file} in MOONGLOW_HAKS"));
        mg_testkit::link_or_copy(&from, &user.join("hak").join(&file)).unwrap();
    }
    // `MG_SCREEN_OVERRIDE`: a folder of files for the user folder's
    // override (a marked copy of a game shader, to read what it computes).
    if let Some(from) = std::env::var_os("MG_SCREEN_OVERRIDE") {
        std::fs::create_dir_all(user.join("override")).unwrap();
        for f in std::fs::read_dir(from).unwrap().flatten() {
            std::fs::copy(f.path(), user.join("override").join(f.file_name())).unwrap();
        }
    }
    let rows: Vec<u32> = std::env::var("MG_SCREEN_APPEARANCES")
        .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
        .unwrap_or_else(|_| vec![8]);
    let distance: f32 =
        std::env::var("MG_SCREEN_DISTANCE").ok().and_then(|v| v.parse().ok()).unwrap_or(5.0);
    let yaw: f32 = std::env::var("MG_SCREEN_YAW").ok().and_then(|v| v.parse().ok()).unwrap_or(30.0);
    let install = GameInstall::new(&root, Some(user.clone()), "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Creatures", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    // `MG_SCREEN_LIGHT`: the area's ambient and diffuse colors (as an ARE
    // has them, 0xBBGGRR), by day and night, without the day's cycle.
    if let Ok(light) = std::env::var("MG_SCREEN_LIGHT") {
        let c: Vec<u32> = light
            .split(',')
            .map(|v| u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).unwrap())
            .collect();
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        for (label, v) in [
            ("SunAmbientColor", c[0]),
            ("MoonAmbientColor", c[0]),
            ("SunDiffuseColor", c[1]),
            ("MoonDiffuseColor", c[1]),
        ] {
            are.root.set(label, mg_gff::Value::Dword(v));
        }
        are.root.set("IsNight", mg_gff::Value::Byte(0));
        are.root.set("DayNightCycle", mg_gff::Value::Byte(0));
        m.set_gff(key, &are).unwrap();
    }
    let bp = game.resman.get(&ResKey::parse("nw_badger", ResType::UTC).unwrap()).unwrap();
    let mut bp = mg_gff::Gff::read(&bp).unwrap();
    let none = |_: mg_core::ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let spots: Vec<glam::Vec3> =
        (0..rows.len()).map(|i| glam::Vec3::new(12.0 + 6.0 * i as f32, 20.0, 0.0)).collect();
    let placed: Vec<_> = rows
        .iter()
        .zip(&spots)
        .map(|(&row, p)| {
            bp.root.set("Appearance_Type", mg_gff::Value::Word(row as u16));
            let at = Placement { position: p.to_array(), rotation: 0.0 };
            instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap()
        })
        .collect();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set("Creature List", mg_gff::Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    let ifo_key = ResKey::parse("module", ResType::IFO).unwrap();
    let mut ifo = m.gff(&ifo_key).unwrap().unwrap();
    let list = mg_module::attach::hak_list(&ifo.root, &haks);
    ifo.root.set("Mod_HakList", list);
    m.set_gff(ifo_key, &ifo).unwrap();
    let path = scratch.join("creatures.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    for on in [true, false] {
        let rs = egui_kittest::wgpu::create_render_state(
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let install = GameInstall::new(&root, Some(user.clone()), "en");
        let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
        app.settings.game_shaders = Some(on);
        app.set_render_state(rs.clone());
        app.open_module(&path);
        app.open_palette = false;
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1400.0, 900.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(5);
        {
            let view = h.state_mut().area_views.get_mut(&area).unwrap();
            if let Some(o) = &mut view.orbit {
                o.pitch = 20f32.to_radians();
                o.yaw = yaw.to_radians();
                o.distance = distance;
                o.target = spots[0] + glam::Vec3::new(0.0, 0.0, 0.8);
            }
        }
        for selected in [false, true] {
            h.state_mut().area_views.get_mut(&area).unwrap().selection =
                if selected { vec![(mg_area::ObjectKind::Creature, 0)] } else { Vec::new() };
            // (The game's programs are made on threads of their own while
            // the view is drawn: time for them.)
            for _ in 0..40 {
                h.step();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let shaders = if on { "game-shaders" } else { "own-shader" };
            let sel = if selected { "selected" } else { "unselected" };
            let name = format!("area-creatures-{shaders}-{sel}");
            h.run_steps(3);
            let image = h.render().expect("render");
            image.save(dir.join(format!("{name}.png"))).unwrap();
            let view = &h.state().area_views[&area];
            for (i, p) in spots.iter().enumerate() {
                let Some(c) = view.screen_pos(*p + glam::Vec3::new(0.0, 0.0, 0.8)) else {
                    continue;
                };
                let half = 160.0;
                let r = egui::Rect::from_center_size(c, egui::vec2(2.0 * half, 2.0 * half))
                    .intersect(view.rect);
                let (x, y) = (r.min.x as u32, r.min.y as u32);
                let (w, hh) = (r.width() as u32, r.height() as u32);
                if w == 0 || hh == 0 {
                    continue;
                }
                let crop = image::imageops::crop_imm(&image, x, y, w, hh).to_image();
                crop.save(dir.join(format!("{name}-{}.png", rows[i]))).unwrap();
            }
        }
    }
}

#[test]
#[ignore]
fn palette_hover_preview() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-hover");
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    for name in [
        "nw_aarcl004",
        "nw_cloth027",
        "nw_arhe001",
        "nw_ashsw001",
        "x2_adrowshl001",
        "nw_wswls001",
        "nw_waxbt001",
        "nw_wbwln001",
        "nw_wswdg001",
    ] {
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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

/// The palette's Gallery of long and tall things (`gallery-framed-*`):
/// dragons, long and winged, each in its picture as it stands; banners,
/// statues and rugs; swords lying. The pictures are framed on the model's
/// posed vertices as the picture sees them (the Moonglow Viewer session's
/// proposal, 2026-10-07).
#[test]
#[ignore]
fn gallery_framed() {
    use mg_module::palette::BlueprintKind;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.settings.palette_gallery = true;
    app.settings.game_shaders = Some(false);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    for (kind, filter) in [
        (BlueprintKind::Creature, "dragon"),
        (BlueprintKind::Placeable, "banner"),
        (BlueprintKind::Placeable, "statue"),
        (BlueprintKind::Placeable, "rug"),
        (BlueprintKind::Item, "longsword"),
    ] {
        {
            let p = &mut h.state_mut().palette;
            p.kind = kind;
            (p.tiles, p.custom) = (false, false);
            p.filter = filter.into();
        }
        for _ in 0..600 {
            h.run_steps(1);
            if h.state().thumbnails.waiting() == 0 {
                break;
            }
        }
        h.run_steps(3);
        shoot(&mut h, &dir, &format!("gallery-framed-{filter}"));
    }
}

/// An item copied into an open module folder from elsewhere, before the
/// folder's files are read again and after (`palette-copied-in-before`,
/// `-after`): the Items custom palette lists it in its category
/// (Troubletcat's report: it showed in the module's tree, not in the
/// palette).
#[test]
#[ignore]
fn palette_blueprint_copied_in() {
    use mg_gff::{Gff, Value};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-palette-copied-in");
    let folder = scratch.join("module");
    std::fs::create_dir_all(scratch.join("user")).unwrap();
    let install = GameInstall::new(&root, Some(scratch.join("user")), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let game = app.game.clone().unwrap();
    let sword = game.resman.get_named("nw_wswls001", ResType::UTI).unwrap().to_vec();
    let named = |resref: &str, name: &str| {
        let mut g = Gff::read(&sword).unwrap();
        let text =
            mg_core::LocString::from_text(mg_core::Language::ENGLISH, mg_core::Gender::Male, name);
        g.root.set("LocalizedName", Value::LocString(text));
        g.root.set("TemplateResRef", Value::resref(mg_core::ResRef::from_str(resref).unwrap()));
        g.to_bytes().unwrap()
    };
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = mg_module::new::new_module(&game, "Copied In", &mut rng).unwrap();
    m.set(ResKey::parse("tm_sword", ResType::UTI).unwrap(), named("tm_sword", "Team Sword"));
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&mg_module::ModuleLocation::Folder(folder.clone())).unwrap();
    drop(game);
    app.settings.no_last_area = true;
    app.settings.no_mod_beside_folder = true;
    app.settings.palette_gallery = false;
    app.open_module(&folder);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Item;
    app.palette.tiles = false;
    app.palette.custom = true;
    app.palette.filter = "sword".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    std::fs::write(folder.join("sent_sword.uti"), named("sent_sword", "Sent Sword")).unwrap();
    shoot(&mut h, &dir, "palette-copied-in-before");
    h.state_mut().reload_project_files();
    h.run_steps(4);
    shoot(&mut h, &dir, "palette-copied-in-after");
}

/// A cleric's Spells page with Empower Spell chosen in its Meta-Magic list
/// (the feat given first): the spells with their casting levels.
#[test]
#[ignore]
fn spells_metamagic() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let key = ResKey::parse("nw_mumcleric", ResType::UTC).unwrap();
    let data = game.resman.get(&key).unwrap().into_owned();
    let mut gff = mg_gff::Gff::read(&data).unwrap();
    let mut feats = gff.root.list("FeatList").unwrap_or(&[]).to_vec();
    let mut feat = mg_gff::Struct::new(1);
    feat.set("Feat", mg_gff::Value::Word(11));
    feats.push(feat);
    gff.root.set("FeatList", mg_gff::Value::List(feats));
    let mut m = mg_module::Module::new();
    m.set(key, gff.to_bytes().unwrap());
    let path = dir.join("spells-metamagic.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.blueprint_pages.insert((key, GffPath::root()), "Spells");
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    h.query_all(combo)
        .find(|n| n.accesskit_node().value() == Some("None".into()))
        .expect("the Meta-Magic list")
        .click();
    h.run_steps(3);
    h.get_by_label("Empower Spell").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "spells-metamagic");
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
        ("nw_mumcleric", ResType::UTC, &["Classes", "Spells"][..]),
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

/// The Visuals page's texture and animation pickers (GitHub issue 32) on
/// a placeable (with a replacement, and one of a texture its model lacks),
/// a door, a creature of one model and one of body parts (PLTs), and an
/// item; and Select Texture, open, its pictures made.
#[test]
#[ignore]
fn visuals_pickers() {
    use mg_gff::{Struct, Value};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let pair = |old: &str, new: &str| {
        let mut e = Struct::new(0);
        e.set("OldTexture", Value::resref(mg_core::ResRef::from_str(old).unwrap()));
        e.set("NewTexture", Value::resref(mg_core::ResRef::from_str(new).unwrap()));
        e
    };
    for (name, t, shot) in [
        ("plc_chest1", ResType::UTP, "visuals-chest"),
        ("nw_door_ttr_01", ResType::UTD, "visuals-door"),
        ("nw_bartender", ResType::UTC, "visuals-creature"),
        ("nw_bandit001", ResType::UTC, "visuals-human"),
        ("nw_wswmls010", ResType::UTI, "visuals-item"),
    ] {
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        let game = app.game.as_deref().unwrap();
        let mut g = mg_gff::Gff::read(&game.resman.get_named(name, t).unwrap()).unwrap();
        if shot == "visuals-human" {
            // A human, its own parts 1 (under what it wears).
            g.root.set("Appearance_Type", Value::Word(6));
            for part in [
                "Torso", "Pelvis", "Belt", "Neck", "LThigh", "RThigh", "LShin", "RShin", "LFoot",
                "LFArm", "RFArm", "LBicep", "RBicep", "LHand", "RHand",
            ] {
                g.root.set(&format!("BodyPart_{part}"), Value::Byte(1));
            }
        }
        if shot == "visuals-chest" {
            let p = mg_preview::placeable(game, &g.root).unwrap();
            let own = mg_preview::replaceable::textures(game, &p);
            let mut s = Struct::new(9);
            let first = own.first().map_or("plc_chest1".to_string(), |t| t.name.clone());
            let mut all = game.resman.list(ResType::DDS);
            all.sort();
            let other = all
                .iter()
                .map(|r| r.to_lowercase().to_string())
                .find(|n| n.starts_with("plc_") && !own.iter().any(|t| t.name == *n))
                .unwrap();
            let list = vec![pair(&first, &other), pair("mg_absent", "mg_other")];
            s.set("TextureReplaceLi", Value::List(list));
            g.root.set("TextureReplace", Value::Struct(s));
        }
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse(name, t).unwrap(), g.to_bytes().unwrap());
        let path = dir.join(format!("{name}-visuals.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        let key = ResKey::parse(name, t).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.state_mut().blueprint_pages.insert((key, GffPath::root()), "Visuals");
        shoot(&mut h, &dir, shot);
        if shot == "visuals-human" {
            // Why a PLT has nothing to pick, said over its button.
            let at = h.get_all_by_label("Select Texture…").next().unwrap().rect().center();
            h.hover_at(at);
            h.run_steps(30);
            shoot(&mut h, &dir, "visuals-plt-tooltip");
        }
        if shot == "visuals-chest" {
            h.get_all_by_label("Select Texture…").next().unwrap().click();
            // (Its pictures are made over frames, as they come in sight.)
            for _ in 0..200 {
                if h.try_run().is_ok() {
                    break;
                }
            }
            shoot(&mut h, &dir, "visuals-select-texture");
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

/// A blueprint's window with the game's data and a GPU (pictures are
/// drawn), on its page `page`, `size` large.
fn pictured_blueprint(
    root: &std::path::Path,
    dir: &std::path::Path,
    (name, t): (&str, ResType),
    page: &'static str,
    size: egui::Vec2,
) -> Harness<'static, Moonglow> {
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named(name, t).unwrap().into_owned();
    let mut m = mg_module::Module::new();
    m.set(ResKey::parse(name, t).unwrap(), data);
    let path = dir.join(format!("{name}-pictured.mod"));
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.open_palette = false;
    let key = ResKey::parse(name, t).unwrap();
    app.blueprint_pages.insert((key, GffPath::root()), page);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    Harness::builder()
        .with_size(size)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app)
}

/// Opens the list whose value `value` accepts and rests the pointer on its
/// row `row` (counted among those whose label `rows` accepts).
fn hover_list_row(
    h: &mut Harness<'_, Moonglow>,
    value: &dyn Fn(&str) -> bool,
    rows: &dyn Fn(&str) -> bool,
    row: usize,
) {
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let list = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().as_deref().is_some_and(value))
        .map(|n| n.rect())
        .expect("the list");
    let at = list.center();
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
    let combo = egui::accesskit::Role::ComboBox;
    // (Among the rows in sight, under the list's Filter field, whether it
    // opened above its box or below: those scrolled out of sight are there
    // too.)
    // (A long list's: its Filter field, at once above or below the box.
    // A short list's rows are all in sight.)
    let filter = h
        .query_all_by(|n| n.placeholder() == Some("Filter"))
        .map(|n| n.rect())
        .find(|r| (r.left() - list.left()).abs() < 20.0);
    let in_sight = |r: egui::Rect| match filter {
        Some(f) => r.top() >= f.bottom() && r.bottom() <= f.bottom() + 180.0,
        None => r.top() > 0.0 && !r.intersects(list),
    };
    let at = h
        .get_all_by(|n| n.label().is_some_and(|l| rows(&l)) && n.role() != combo)
        .filter(|n| in_sight(n.rect()))
        .nth(row)
        .map(|n| n.rect().center())
        .expect("the row");
    h.hover_at(at);
    // (Long enough for the tooltip, and for its picture to be drawn.)
    h.run_steps(40);
}

/// The pictures beside the row the pointer rests on in a door's and an
/// item's Appearance lists (GitHub issue 7): a generic door's and a
/// tileset door's, a ring's model, a longsword's blade and an armor's
/// torso, on a man and on a woman (`appearance-hover-*.png`).
#[test]
#[ignore]
fn appearance_hover_pictures() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let size = egui::vec2(1200.0, 800.0);
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let names = |table: &str, name: &str, label: &str| {
        let columns = mg_rules::ChoiceColumns { name: Some(name), label: Some(label) };
        game.choices(table, columns).unwrap().into_iter().map(|c| c.text).collect::<Vec<_>>()
    };
    let generic = names("genericdoors", "Name", "Label");
    let in_generic = |v: &str| generic.iter().any(|g| g == v);
    let mut h = pictured_blueprint(&root, &dir, ("nw_door_strong", ResType::UTD), "Basic", size);
    h.run_steps(4);
    hover_list_row(&mut h, &in_generic, &in_generic, 3);
    shoot(&mut h, &dir, "appearance-hover-door-generic");
    let types = names("doortypes", "StringRefGame", "Label");
    let in_types = |v: &str| types.iter().any(|t| v.starts_with(t.as_str()));
    let mut h = pictured_blueprint(&root, &dir, ("nw_door_ttr_01", ResType::UTD), "Basic", size);
    h.run_steps(4);
    hover_list_row(&mut h, &in_types, &in_types, 4);
    shoot(&mut h, &dir, "appearance-hover-door-type");
    // The type that is the generic door: the generic door chosen.
    let mut h = pictured_blueprint(&root, &dir, ("nw_door_strong", ResType::UTD), "Basic", size);
    h.run_steps(4);
    hover_list_row(&mut h, &in_types, &|v: &str| v == "Use Generic Appearance", 0);
    shoot(&mut h, &dir, "appearance-hover-door-type-generic");
    let number = |v: &str| v.parse::<u32>().is_ok();
    for (name, value, rows, row, woman) in [
        (
            "nw_it_mring001",
            &number as &dyn Fn(&str) -> bool,
            &number as &dyn Fn(&str) -> bool,
            2,
            false,
        ),
        (
            "nw_wswls001",
            &|v: &str| v.starts_with("Model "),
            &|v: &str| v.starts_with("Model "),
            3,
            false,
        ),
        ("nw_aarcl001", &|v: &str| v.contains("(AC "), &|v: &str| v.contains("(AC "), 6, false),
        ("nw_aarcl001", &|v: &str| v.contains("(AC "), &|v: &str| v.contains("(AC "), 6, true),
    ] {
        let mut h = pictured_blueprint(&root, &dir, (name, ResType::UTI), "Appearance", size);
        h.state_mut().settings.armor_on_woman = woman;
        h.run_steps(6);
        hover_list_row(&mut h, value, rows, row);
        let on = if woman { "-woman" } else { "" };
        shoot(&mut h, &dir, &format!("appearance-hover-{name}{on}"));
    }
}

/// Armor's icons in an inventory and its pictures in the palette, on a
/// man and with Options' Show armor and cloaks on a woman
/// (`shown-on-{man,woman}-{inventory,palette}.png`).
#[test]
#[ignore]
fn shown_on_a_woman() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let size = egui::vec2(1200.0, 800.0);
    let mut h = pictured_blueprint(&root, &dir, ("nw_bandit001", ResType::UTC), "Inventory", size);
    h.state_mut().settings.no_inventory_warning = true;
    h.run_steps(4);
    for woman in [false, true] {
        h.state_mut().settings.armor_on_woman = woman;
        h.run_steps(6);
        let on = if woman { "woman" } else { "man" };
        shoot(&mut h, &dir, &format!("shown-on-{on}-inventory"));
    }
    let mut h = pictured_blueprint(&root, &dir, ("nw_aarcl001", ResType::UTI), "General", size);
    h.run_steps(2);
    // (The palette alone: the item's window closed.)
    let item = mg_ui::Tab::Blueprint(ResKey::parse("nw_aarcl001", ResType::UTI).unwrap());
    let tab = h.state().dock.find_tab(&item).unwrap();
    h.state_mut().dock.remove_tab(tab);
    {
        let app = h.state_mut();
        app.settings.palette_gallery = true;
        app.palette.kind = mg_module::palette::BlueprintKind::Item;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.palette.filter = "nw_aarcl0".into();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    }
    for woman in [false, true] {
        h.state_mut().settings.armor_on_woman = woman;
        h.run_steps(60);
        let on = if woman { "woman" } else { "man" };
        shoot(&mut h, &dir, &format!("shown-on-{on}-palette"));
    }
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
                ui.heading("▶ 💡 ◎ ⏵ ⏷ 🗐 📦 ⚙ 🗻 🗑 ↕ 🔁 ⏭ ⏮ ↺ 🏁 🌙 ☁ ⊞ 🍃 🎵 👣 ⛶ ℹ 🎥 🏃");
                // (The main toolbar's and the rest of the area view's.)
                ui.heading("🗋 🗁 💾 ⟲ ⟳ 🗺 🔍 👁 ✔ 🔊 🌿 🎨 ✏ 👤 🚪 ⚔ 🗡 ⛲ 🔉 💰 ⚡ 📍 ◻");
            });
        },
        app,
    );
    h.run();
    h.run();
    shoot(&mut h, &dir, "glyphs");
}

/// A creature's Inventory page: Aurora's Options (Selected Item,
/// Dropable, Pickpocketable) under the palette, before an item is
/// selected and with two selected (a builder could not find them).
#[test]
#[ignore]
fn creature_inventory_options() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let (name, t) = ("nw_bandit001", ResType::UTC);
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_inventory_warning = true;
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named(name, t).unwrap().into_owned();
    let mut m = mg_module::Module::new();
    m.set(ResKey::parse(name, t).unwrap(), data);
    let path = dir.join(format!("{name}-options.mod"));
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    let key = ResKey::parse(name, t).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().blueprint_pages.insert((key, GffPath::root()), "Inventory");
    h.run_steps(3);
    place(&mut h, &mg_ui::Tab::Blueprint(key), (20.0, 60.0), (1150.0, 720.0));
    shoot(&mut h, &dir, "creature-inventory-options");
    let image = egui::accesskit::Role::Image;
    let pick = |h: &mut Harness<'_, Moonglow>, label: &str, ctrl: bool| {
        let node = h.get_all_by_label(label).find(|n| n.accesskit_node().role() != image).unwrap();
        if ctrl {
            node.click_modifiers(egui::Modifiers::COMMAND);
        } else {
            node.click();
        }
    };
    pick(&mut h, "Padded Armor", false);
    h.run_steps(2);
    h.get_by_label("Dropable").click();
    h.run_steps(2);
    // And the sword with it: Dropable mixed.
    pick(&mut h, "Longsword", true);
    h.run_steps(2);
    shoot(&mut h, &dir, "creature-inventory-options-selected");
}

/// The Inventory pages in a window as narrow as one opens beside the
/// area (about 480 points): a placed creature's, the creature
/// blueprint's, a placed store's and a placed chest's
/// (`inventory-narrow-*.png`; `-middle` at 620, `-wide` at the window's own size). At
/// that width the slots' items once were scrolled out of sight to the
/// right (seen testing Sacha's report on 2.5.4).
#[test]
#[ignore]
fn inventory_pages_narrow() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    use mg_module::{Module, ModuleLocation};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let dir = mg_testkit::output_dir("screens");
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m: Module = new_module(&game, "Inventory", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Camp".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let read = |r: mg_core::ResRef, t: ResType| {
        let data = game.resman.get(&ResKey::new(r, t)).ok()?;
        Some(mg_gff::Gff::read(&data).ok()?.root)
    };
    let named = |name: &str, t| read(mg_core::ResRef::from_str(name).unwrap(), t).unwrap();
    let items = |r: mg_core::ResRef| read(r, ResType::UTI);
    let placing = Placing { game: &game, item: &items };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let bandit = instance(&placing, ResType::UTC, &named("nw_bandit001", ResType::UTC), at, &[]);
    let store = instance(&placing, ResType::UTM, &named("nw_storebar01", ResType::UTM), at, &[]);
    let mut chest =
        instance(&placing, ResType::UTP, &named("plc_chest1", ResType::UTP), at, &[]).unwrap();
    let mut sword = named("nw_wswls001", ResType::UTI);
    sword.set("Repos_PosX", mg_gff::Value::Word(0));
    sword.set("Repos_Posy", mg_gff::Value::Word(0));
    chest.set("HasInventory", mg_gff::Value::Byte(1));
    chest.set("ItemList", mg_gff::Value::List(vec![sword]));
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![bandit.unwrap()]));
    g.root.set("StoreList", mg_gff::Value::List(vec![store.unwrap()]));
    g.root.set("Placeable List", mg_gff::Value::List(vec![chest]));
    m.set_gff(git, &g).unwrap();
    let bandit = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    m.set(bandit, game.resman.get(&bandit).unwrap().into_owned());
    let path = mg_testkit::scratch_dir("screens-inventory-narrow").join("inventory.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let placed = |list: &str| mg_ui::Tab::Instance { area, path: GffPath::root().item(list, 0) };
    for (name, tab) in [
        ("creature", placed("Creature List")),
        ("creature-blueprint", mg_ui::Tab::Blueprint(bandit)),
        ("store", placed("StoreList")),
        ("chest", placed("Placeable List")),
    ] {
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        app.settings.no_inventory_warning = true;
        app.open_module(&path);
        app.open_palette = false;
        let (key, at) = match &tab {
            mg_ui::Tab::Instance { path, .. } => (git, path.clone()),
            _ => (bandit, GffPath::root()),
        };
        app.blueprint_pages.insert((key, at), "Inventory");
        app.actions.push(mg_ui::Action::OpenTab(tab.clone()));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(3);
        for (size, width) in [("narrow", 480.0), ("middle", 620.0), ("wide", 820.0)] {
            place(&mut h, &tab, (20.0, 60.0), (width, 672.0));
            h.run_steps(3);
            shoot(&mut h, &dir, &format!("inventory-{size}-{name}"));
        }
    }
}

/// A cleric's Classes page in windows narrow and wide: its two domain
/// pickers in reach, not at (or past) the window's edge.
#[test]
#[ignore]
fn classes_narrow() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let key = ResKey::parse("nw_mumcleric", ResType::UTC).unwrap();
    let mut m = mg_module::Module::new();
    m.set(key, game.resman.get(&key).unwrap().into_owned());
    let path = mg_testkit::scratch_dir("screens-classes-narrow").join("classes.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.open_palette = false;
    app.blueprint_pages.insert((key, GffPath::root()), "Classes");
    let tab = mg_ui::Tab::Blueprint(key);
    app.actions.push(mg_ui::Action::OpenTab(tab.clone()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    for (size, width) in [("narrow", 480.0), ("middle", 620.0), ("wide", 900.0)] {
        place(&mut h, &tab, (20.0, 60.0), (width, 600.0));
        h.run_steps(3);
        shoot(&mut h, &dir, &format!("classes-{size}"));
    }
}

/// The Appearance Gallery opened for a chest whose appearance is two
/// thirds down placeables.2da: scrolled to it, its picture at the top
/// (GitHub issue 32).
#[test]
#[ignore]
fn gallery_far_appearance() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let cols = mg_rules::ChoiceColumns { name: Some("StrRef"), label: Some("Label") };
    let all = game.choices("placeables", cols).unwrap();
    let far = all[all.len() * 2 / 3].clone();
    let mut g = mg_gff::Gff::read(&game.resman.get(&key).unwrap()).unwrap();
    g.root.set("Appearance", mg_gff::Value::Dword(far.row as u32));
    let mut m = mg_module::Module::new();
    m.set(key, g.to_bytes().unwrap());
    let path = mg_testkit::scratch_dir("screens-gallery-far").join("gallery.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.open_palette = false;
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("Gallery…").click();
    for _ in 0..200 {
        if h.try_run().is_ok() {
            break;
        }
    }
    shoot(&mut h, &dir, "gallery-far-appearance");
}

/// A game blueprint viewed: its Tag field clicked and typed in (nothing
/// taken), and a store's inventory viewed, its Find typed in (taken).
#[test]
#[ignore]
fn viewed_blueprint_typing() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let path = mg_testkit::scratch_dir("screens-viewed").join("viewed.mod");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    for (name, t, page) in
        [("nw_waypoint001", ResType::UTW, "Basic"), ("nw_storebar01", ResType::UTM, "Inventory")]
    {
        let key = ResKey::parse(name, t).unwrap();
        {
            let app = h.state_mut();
            let data = app.game.as_ref().unwrap().resman.get(&key).unwrap().into_owned();
            app.ws.as_mut().unwrap().view(key, mg_gff::Gff::read(&data).unwrap());
            app.blueprint_pages.insert((key, GffPath::root()), page);
            app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        }
        h.run_steps(3);
        if t == ResType::UTW {
            h.get_all_by_value("NW_WAYPOINT001").next().unwrap().click();
        } else {
            let find =
                |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
            h.get_all(egui_kittest::kittest::by().predicate(find)).last().unwrap().click();
        }
        h.run_steps(2);
        h.event(egui::Event::Text("torch".into()));
        shoot(&mut h, &dir, &format!("viewed-{name}-typed"));
        let tab = h.state().dock.find_tab(&mg_ui::Tab::Blueprint(key)).unwrap();
        h.state_mut().dock.remove_tab(tab);
        h.run_steps(2);
    }
}

/// Blueprints edited together, "≠" before each field they differ in:
/// two chests (a name, an appearance, hit points, a conversation, No
/// Interrupt, a cursor) and two sounds (rows of choices from the page's
/// edge).
#[test]
#[ignore]
fn mixed_marks() {
    use mg_gff::Value;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    for (name, t, pages) in [
        ("plc_chest1", ResType::UTP, &["Basic", "Advanced", "Visuals"][..]),
        ("animalcriesday", ResType::UTS, &["Basic", "Positioning", "Advanced"][..]),
    ] {
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        let game = app.game.as_deref().unwrap();
        let base = mg_gff::Gff::read(&game.resman.get_named(name, t).unwrap()).unwrap();
        let mut other = base.clone();
        let text = mg_core::LocString::from_text(
            mg_core::Language::ENGLISH,
            mg_core::Gender::Male,
            "Another One",
        );
        for label in ["LocName", "LocalizedName"] {
            if other.root.get(label).is_some() {
                other.root.set(label, Value::LocString(text.clone()));
            }
        }
        if t == ResType::UTP {
            other.root.set("Appearance", Value::Dword(8));
            other.root.set("HP", Value::Short(99));
            other
                .root
                .set("Conversation", Value::resref(mg_core::ResRef::from_str("mg_talk").unwrap()));
            other.root.set("Interruptable", Value::Byte(1));
            let mut misc = mg_gff::Struct::new(8);
            misc.set("MouseCursor", Value::Int(25));
            other.root.set("MiscVisuals", Value::Struct(misc));
        } else {
            other.root.set("Times", Value::Byte(1));
            other
                .root
                .set("Random", Value::Byte(1 - base.root.integer("Random").unwrap_or(0) as u8));
            other.root.set(
                "Positional",
                Value::Byte(1 - base.root.integer("Positional").unwrap_or(0) as u8),
            );
        }
        let keys: Vec<ResKey> =
            ["mg_a", "mg_b"].iter().map(|n| ResKey::parse(n, t).unwrap()).collect();
        let mut m = mg_module::Module::new();
        m.set(keys[0], base.to_bytes().unwrap());
        m.set(keys[1], other.to_bytes().unwrap());
        let path = mg_testkit::scratch_dir("screens-mixed-marks").join(format!("{name}.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        app.open_palette = false;
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprints(keys.clone())));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        for page in pages {
            h.state_mut().blueprint_pages.insert((keys[0], GffPath::root()), page);
            let page_name = page.to_lowercase();
            shoot(&mut h, &dir, &format!("mixed-{name}-{page_name}"));
        }
    }
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

/// A store page of 300 of the game's items (every kind: tall, wide and
/// small icons), scrolled down a little at a time with the wheel, then
/// to its end (`store-scrolled-{top,middle,end}.png`).
#[test]
#[ignore]
fn store_scrolled() {
    use mg_gff::{Struct, Value};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let (name, t) = ("nw_storebar01", ResType::UTM);
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let mut store = mg_gff::Gff::read(&game.resman.get_named(name, t).unwrap()).unwrap();
    let mut items = game.resman.list(ResType::UTI);
    items.sort();
    let entries: Vec<Struct> = (items.iter().step_by(items.len() / 300).take(300).enumerate())
        .map(|(i, r)| {
            let mut s = Struct::new(i as u32);
            s.set("InventoryRes", Value::resref(*r));
            s.set("Repos_PosX", Value::Word((i % 10) as u16));
            s.set("Repos_Posy", Value::Word((i / 10) as u16));
            s.set("Infinite", Value::Byte(0));
            s
        })
        .collect();
    let mut page = Struct::new(4);
    page.set("ItemList", Value::List(entries));
    store.root.set("StoreList", Value::List(vec![page]));
    let mut m = mg_module::Module::new();
    let key = ResKey::parse(name, t).unwrap();
    m.set_gff(key, &store).unwrap();
    let path = mg_testkit::scratch_dir("screens-store-scrolled").join("store.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().blueprint_pages.insert((key, GffPath::root()), "Inventory");
    h.run_steps(3);
    place(&mut h, &mg_ui::Tab::Blueprint(key), (20.0, 60.0), (1150.0, 800.0));
    h.run_steps(3);
    shoot(&mut h, &dir, "store-scrolled-top");
    let at = h.get_all_by_label("Remove").next().unwrap().rect().center() - egui::vec2(200.0, 0.0);
    h.hover_at(at);
    let wheel = |h: &mut Harness<'_, Moonglow>, dy: f32| {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(20);
    };
    for _ in 0..30 {
        wheel(&mut h, -400.0);
    }
    shoot(&mut h, &dir, "store-scrolled-middle");
    wheel(&mut h, -1_000_000.0);
    shoot(&mut h, &dir, "store-scrolled-end");
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

/// The update notice under the toolbar, Help › Check for Updates… (newer,
/// and on a copy Scoop updates) and Options › Updates (on, and on a
/// Flatpak's copy). The answers are canned: no network.
#[test]
#[ignore]
fn update_notice_and_options() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens");
    let answer = |tag: &str| {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/jadzziaa/moonglow-toolset/releases/tag/{tag}","draft":false,"prerelease":false}}"#
        )
    };
    let newer = answer("v2.5.0");
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.updates.fetch = Some(Arc::new(move || Ok(newer.clone())));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    shoot(&mut h, &dir, "update-notice");
    mg_ui::updates::check_now(h.state_mut());
    shoot(&mut h, &dir, "update-check-newer");
    h.state_mut().updates.installer = Some(mg_ui::updates::Installer::Scoop);
    h.get_by_label("Check Again").click();
    shoot(&mut h, &dir, "update-check-scoop");
    h.state_mut().updates.window = false;
    h.state_mut().settings.update_skipped = Some("2.4.9".into());
    h.state_mut().updates.installer = None;
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Updates").click();
    shoot(&mut h, &dir, "options-updates");
    h.state_mut().updates.installer = Some(mg_ui::updates::Installer::Flatpak);
    shoot(&mut h, &dir, "options-updates-flatpak");
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

/// The Edit menu opened from a script typed in, the module's last step
/// elsewhere: its Undo and Redo the script's, plain "Undo" and "Redo".
#[test]
#[ignore]
fn script_edit_menu_undo() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-script-undo");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, b"void main()\n{\n    int a;\n}\n".to_vec());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 600.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let other = ResKey::parse("mg_note", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key: other, data: Some(b"void main() {}\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Earlier", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    h.run();
    h.state_mut().script_tools.jump = Some((key, 25));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.get_by_label("Edit").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "script-edit-menu-undo");
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

/// Help (go to definition) on a function whose comment follows a line of
/// code with a comment of its own (that one is the code's, not the
/// function's), and on a struct's member after `.` (its comment above it,
/// BioWare's `// *`).
#[test]
#[ignore]
fn script_help_member_and_trailing_comment() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-script-member-help");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "struct Mark\n{\n    // * Where it is.\n    vector vPos;\n};\n\n\
                int nLast; // the last one\n// Returns the mark's place.\n\
                vector Place(struct Mark m) { return m.vPos; }\n\n\
                void main()\n{\n    struct Mark m;\n    vector v = Place(m) + m.vPos;\n}\n";
    m.set(key, text.as_bytes().to_vec());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    h.run();
    for (needle, name) in
        [("Place(m) +", "script-help-after-code"), ("vPos;\n}", "script-help-member")]
    {
        let at = text.find(needle).unwrap() + 1;
        let d = h.state_mut().declaration_at(key, at).expect("a declaration");
        h.state_mut().go_to_declaration(&d);
        h.run();
        shoot(&mut h, &dir, name);
    }
}

/// Jasperre's script editor notes: Help on a function a `/* … */` block
/// documents; the Compiler tab after File › Save with compiling on save
/// off, and the log's lines with their times; the editor's context menu
/// (Convert Indentation).
#[test]
#[ignore]
fn script_block_help_save_note_and_indentation() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-script-rest");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "int Hidden(object oSelf);\r\n\r\n//::///////////////\r\n//:: Hidden\r\n\
                //::///////////////\r\n/*\r\n    Returns TRUE if oSelf is hidden either\r\n    \
                magically or via stealth\r\n*/\r\n//:: Created By: Brent\r\n\
                int Hidden(object oSelf)\r\n{\r\n  return FALSE;\r\n}\r\n\r\nvoid main()\r\n{\r\n  \
                Hidden(OBJECT_SELF);\r\n}\r\n";
    m.set(key, text.as_bytes().to_vec());
    let path = dir.join("s.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    h.state_mut().script_tools.filter = "Hidden".into();
    h.run();
    h.run();
    h.get_by_label("Hidden").click();
    shoot(&mut h, &dir, "script-block-help");
    // Typed, then the module saved.
    let at = text.replace("\r\n", "\n").find("void main").unwrap();
    h.state_mut().script_tools.jump = Some((key, at));
    h.run();
    h.event(egui::Event::Text("// x\n".into()));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    shoot(&mut h, &dir, "script-module-save-note");
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("int Hidden")))
        .expect("the script's text")
        .click_secondary();
    shoot(&mut h, &dir, "script-indent-menu");
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

/// The Item Wizard's Name page: Magical ticked, a level and quality
/// chosen (as Aurora's page).
#[test]
#[ignore]
fn item_wizard_magical() {
    use egui_kittest::kittest::Queryable;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.actions.push(mg_ui::Action::NewModule("wiz".into()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().blueprint_wizard = Some(mg_ui::blueprint_wizard::BlueprintWizard::new(
        mg_module::palette::BlueprintKind::Item,
    ));
    h.run();
    h.get_by_label("Longsword").scroll_to_me();
    h.run();
    h.get_by_label("Longsword").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    shoot(&mut h, &dir, "item-wizard-name");
    h.get_by_label("Magical").click();
    h.run();
    h.get_by_label("11-15").click();
    h.get_by_label("High").click();
    h.run();
    shoot(&mut h, &dir, "item-wizard-magical");
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
    // Its resources viewed: a picture, a table (`hak-editor-view-*`).
    for t in [mg_core::ResType::TGA, mg_core::ResType::DDS, mg_core::ResType::TWODA] {
        let items = h.state().haks[0].hak.items().to_vec();
        let Some(item) = items.iter().find(|i| i.key.restype == t) else { continue };
        let name = item.key.to_string();
        h.state_mut().haks[0].filter = name.clone();
        h.run();
        h.get_by_label(&name).click_secondary();
        h.run();
        h.get_all_by_label("View").last().unwrap().click();
        h.run();
        shoot(&mut h, &dir, &format!("hak-editor-view-{t}"));
    }
    h.state_mut().haks[0].filter.clear();
    let tab = h.state().dock.find_tab(&mg_ui::Tab::Hak(h.state().haks[0].id)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().haks.clear();
    // (Chess opens on its start area: Module Properties is opened.)
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::ModuleProperties));
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
    mg_ui::tileset_view::render_minimaps(h.state_mut(), &mut doc);
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
fn exact_tiles_and_tile_height() {
    // The tiles palette's Gallery with its Exact tile switch on, and Tile
    // Properties with its Height, then with All tiles (#29).
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.settings.palette_gallery = true;
    app.settings.exact_tiles = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    h.run_steps(5);
    if h.query_by_label("Tiles").is_none() {
        h.get_by_label("🗻").click();
    }
    h.run_steps(30);
    shoot(&mut h, &dir, "tiles-palette-exact");
    // Tile Properties of a tile, from the tile menu in Select Tiles.
    h.state_mut().area_views.get_mut(&area).unwrap().tile_mode = true;
    h.run_steps(2);
    let open = |h: &mut Harness<'_, Moonglow>, at: glam::Vec3| {
        let pos = h.state().area_views[&area].screen_pos(at).unwrap();
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Default::default(),
            });
        }
        h.run_steps(2);
        h.get_by_label("Tile Properties…").click();
        h.run_steps(30);
    };
    // A tile of a building (a group) in the middle: its Height is locked.
    let middle = h.state().area_views[&area].orbit.unwrap().target;
    open(&mut h, glam::Vec3::new(middle.x, middle.y, 0.0));
    shoot(&mut h, &dir, "tile-properties-group");
    h.get_by_label("Cancel").click();
    h.run_steps(2);
    // A tile of open ground, nearer.
    let ground = {
        let view = &h.state().area_views[&area];
        let model = view.model.as_ref().unwrap();
        let (x, y) = (model.width / 2, model.height / 2 - 3);
        model.tiles[(y * model.width + x) as usize].position
    };
    open(&mut h, ground);
    h.get_by_label("Raise Height").click();
    h.run_steps(30);
    shoot(&mut h, &dir, "tile-properties-height");
    h.get_by_label(mg_ui::tile_select::ALL_TILES).click();
    h.run_steps(3);
    h.get_by_label("↺90°").click();
    h.run_steps(40);
    shoot(&mut h, &dir, "tile-properties-all-tiles");
}

#[test]
#[ignore]
fn area_grid() {
    // The tile grid among buildings and trees: hidden behind them.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
fn toolbar_icons_only() {
    // Options › General › Toolbar buttons: the main toolbar and an area's,
    // with icons and text, then icons only (in the same tab, and in a
    // narrow one), and a glyph's tip.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    h.run_steps(15);
    shoot(&mut h, &dir, "toolbar-icons-and-text");
    h.state_mut().settings.toolbar_icons_only = true;
    h.run_steps(5);
    shoot(&mut h, &dir, "toolbar-icons-only");
    // A tip: the glyph's name, then what it does.
    let center = |h: &Harness<'_, Moonglow>, label: &str| {
        let low = |n: &egui_kittest::Node<'_>| n.rect().top();
        h.get_all_by_label(label).max_by(|a, b| low(a).total_cmp(&low(b))).unwrap().rect().center()
    };
    // (Named by their names, not their glyphs, with Icons only; before
    // that, by their glyphs.)
    let named = |h: &Harness<'_, Moonglow>, name: &str, glyph: &str| {
        if h.query_all_by_label(name).next().is_some() { center(h, name) } else { center(h, glyph) }
    };
    h.hover_at(named(&h, "Shaders", "🎨"));
    h.run_steps(60);
    shoot(&mut h, &dir, "toolbar-icons-only-tip");
    h.hover_at(named(&h, "Palettes", "📦"));
    h.run_steps(60);
    shoot(&mut h, &dir, "toolbar-icons-only-main-tip");
    h.set_size(egui::vec2(1000.0, 700.0));
    h.hover_at(egui::pos2(500.0, 500.0));
    h.run_steps(10);
    shoot(&mut h, &dir, "toolbar-icons-only-narrow");
    // The editors' toolbars: a script's, a conversation's, a model's.
    h.set_size(egui::vec2(1400.0, 700.0));
    let ws = h.state().ws.as_ref().unwrap();
    let script = *ws.module.keys_of(ResType::NSS).next().expect("a script");
    let talk = *ws.module.keys_of(ResType::DLG).next().expect("a conversation");
    let model = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    for (tab, name) in [
        (mg_ui::Tab::Script(script), "script"),
        (mg_ui::Tab::Dialog(talk), "conversation"),
        (mg_ui::Tab::Model(model), "model"),
    ] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run_steps(10);
        for icons in [false, true] {
            h.state_mut().settings.toolbar_icons_only = icons;
            h.run_steps(5);
            let how = if icons { "icons-only" } else { "icons-and-text" };
            shoot(&mut h, &dir, &format!("toolbar-{name}-{how}"));
        }
    }
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
fn raw_fields_of_the_pc_faction() {
    // A creature blueprint of faction 0 in the raw fields view: the
    // warning beside its FactionID.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-raw-pc-faction");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let mut guard = mg_gff::Gff::new(*b"UTC ");
    guard.root.set("Tag", mg_gff::Value::String(b"GUARD".to_vec()));
    guard.root.set("FactionID", mg_gff::Value::Word(0));
    guard.root.set("Appearance_Type", mg_gff::Value::Word(6));
    let key = ResKey::parse("guard", ResType::UTC).unwrap();
    m.set_gff(key, &guard).unwrap();
    let path = scratch.join("pcfac.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Gff(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 400.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    shoot(&mut h, &dir, "raw-fields-pc-faction");
}

#[test]
#[ignore]
fn waypoint_with_no_flag() {
    // A waypoint blueprint whose Appearance is no waypoint.2da row: the
    // warning under its Appearance Type, as Verify words it.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-waypoint-flag");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let mut post = mg_gff::Gff::new(*b"UTW ");
    post.root.set("Tag", mg_gff::Value::String(b"WP_POST".to_vec()));
    post.root.set("Appearance", mg_gff::Value::Byte(7));
    let key = ResKey::parse("post", ResType::UTW).unwrap();
    m.set_gff(key, &post).unwrap();
    let path = scratch.join("flag.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(6);
    shoot(&mut h, &dir, "waypoint-no-flag");
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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

/// The tiles palette of a Medieval City area searched for a tile's model
/// that only a group has (Inn_2x3's `tcm02_s01_05`, #35): in the list,
/// then in the Gallery.
#[test]
#[ignore]
fn tiles_palette_group_search() {
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Group Search", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Street".into(),
        tileset: mg_core::ResRef::from_str("tcm02").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = dir.join("group-search.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
    h.state_mut().palette.filter = "tcm02_s01_05".into();
    h.run_steps(5);
    shoot(&mut h, &dir, "tiles-palette-group-search-list");
    h.state_mut().settings.palette_gallery = true;
    h.run_steps(60);
    shoot(&mut h, &dir, "tiles-palette-group-search-gallery");
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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

/// The tiles palette of a tileset whose palette is the one Moonglow makes
/// of its `.set` (Tileset › Make Palette, `mg tileset-palette`), in the
/// module over the game's: as it was made before (flat) and now
/// (features and groups in a folder a terrain).
#[test]
#[ignore]
fn generated_tileset_palette() {
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let set_key = ResKey::parse("tcm02", ResType::SET).unwrap();
    let set = mg_set::Tileset::parse(&game.resman.get(&set_key).unwrap(), Default::default());
    let (palette, _) = mg_module::tileset::palette(&set.unwrap());
    // (As the generator made it before: every leaf in its category.)
    let mut flat = palette.clone();
    if let Some(mg_gff::Value::List(main)) = flat.root.get_mut("MAIN") {
        for category in main.iter_mut() {
            fn leaves(s: &mg_gff::Struct, out: &mut Vec<mg_gff::Struct>) {
                for i in s.list("LIST").unwrap_or(&[]) {
                    if i.get("LIST").is_some() { leaves(i, out) } else { out.push(i.clone()) }
                }
            }
            let mut out = Vec::new();
            leaves(category, &mut out);
            category.set("LIST", mg_gff::Value::List(out));
        }
    }
    for (name, itp) in [("flat", flat), ("folders", palette)] {
        let mut rng = fastrand::Rng::with_seed(7);
        let mut m = new_module(&game, "Palette", &mut rng).unwrap();
        let spec = AreaSpec {
            name: "Keep".into(),
            tileset: mg_core::ResRef::from_str("tcm02").unwrap(),
            width: 4,
            height: 4,
        };
        let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
        m.set_gff(ResKey::parse("tcm02palstd", ResType::ITP).unwrap(), &itp).unwrap();
        let path = mg_testkit::scratch_dir(&format!("screens-palette-{name}")).join("p.mod");
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        let rs = egui_kittest::wgpu::create_render_state(
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        app.set_render_state(rs.clone());
        app.settings.game_shaders = Some(false);
        app.open_module(&path);
        let mut h = Harness::builder()
            .with_size(egui::vec2(1200.0, 900.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(3);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
        h.run_steps(5);
        h.state_mut().palette.tiles = true;
        for _ in 0..10 {
            h.run_steps(2);
        }
        shoot(&mut h, &dir, &format!("tileset-palette-{name}"));
        // Its first features folder opened.
        if name == "folders" {
            h.get_by_label("Features").click();
            h.run_steps(3);
            // (The folder, above the terrain brush of its name.)
            h.get_all_by_label("Cobble").next().unwrap().click();
            h.run_steps(3);
            shoot(&mut h, &dir, "tileset-palette-folders-open");
        }
    }
}

/// A tile's textures (Leoendethas' request): in Tile Properties, wide and
/// as narrow as it is beside the area, in a tile's view and in the
/// tileset editor; on the game's own `ttr01` and, where `MOONGLOW_HAKS`
/// has it, The Domains of Dread's (`tdod_t_dhd.hak`), whose tiles are
/// drawn with materials (`tile-textures-*.png`).
#[test]
#[ignore]
fn tile_textures() {
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let tdod =
        mg_testkit::builder_hak_dirs().into_iter().find(|d| d.join("tdod_t_dhd.hak").is_file());
    // (The game's ttr01 tile 10; tile 145 of TDoD's, its brick walls and
    // roofs drawn with materials.)
    let mut runs = vec![("base", None, 10)];
    if let Some(haks) = &tdod {
        runs.push(("tdod", Some(haks.join("tdod_t_dhd.hak")), 145));
    } else {
        eprintln!("no tdod_t_dhd.hak in MOONGLOW_HAKS: the game's tiles alone");
    }
    for (name, hak, tile) in runs {
        let user = mg_testkit::scratch_dir(&format!("screens-tile-textures-{name}"));
        std::fs::create_dir_all(user.join("hak")).unwrap();
        if let Some(hak) = &hak {
            mg_testkit::link_or_copy(hak, &user.join("hak/tdod_t_dhd.hak")).unwrap();
        }
        let install = GameInstall::new(&root, Some(user.clone()), "en");
        let game = mg_rules::GameData::open(&install).unwrap();
        let mut rng = fastrand::Rng::with_seed(7);
        let mut m = new_module(&game, "Textures", &mut rng).unwrap();
        let tileset = mg_core::ResRef::from_str("ttr01").unwrap();
        let spec = AreaSpec { name: "Field".into(), tileset, width: 4, height: 4 };
        let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        are.root.list_mut("Tile_List").unwrap()[5].set("Tile_ID", mg_gff::Value::Int(tile));
        m.set_gff(key, &are).unwrap();
        if hak.is_some() {
            let ifo_key = ResKey::parse("module", ResType::IFO).unwrap();
            let mut ifo = m.gff(&ifo_key).unwrap().unwrap();
            let list = mg_module::attach::hak_list(&ifo.root, &["tdod_t_dhd".to_string()]);
            ifo.root.set("Mod_HakList", list);
            m.set_gff(ifo_key, &ifo).unwrap();
        }
        let path = user.join("textures.mod");
        m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        let rs = egui_kittest::wgpu::create_render_state(
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
        // Tile Properties of tile (1, 1), from the tile menu, its
        // Textures opened.
        h.state_mut().area_views.get_mut(&area).unwrap().tile_mode = true;
        h.run_steps(2);
        let pos = h.state().area_views[&area].screen_pos(glam::Vec3::new(15.0, 15.0, 0.0)).unwrap();
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Default::default(),
            });
        }
        h.run_steps(2);
        h.get_by_label("Tile Properties…").click();
        h.run_steps(5);
        h.get_by_label_contains("Textures of ").click();
        h.run_steps(20);
        place(&mut h, &mg_ui::Tab::TileProperties, (500.0, 60.0), (560.0, 760.0));
        h.run_steps(20);
        shoot(&mut h, &dir, &format!("tile-textures-{name}-properties"));
        // As narrow as it stands beside the area.
        place(&mut h, &mg_ui::Tab::TileProperties, (842.0, 60.0), (255.0, 760.0));
        h.run_steps(20);
        shoot(&mut h, &dir, &format!("tile-textures-{name}-properties-narrow"));
        h.get_by_label("Cancel").click();
        h.run_steps(2);
        // The tile's view.
        let view = mg_ui::Tab::Tile { tileset, tile: tile as u32 };
        h.state_mut().actions.push(mg_ui::Action::OpenTab(view.clone()));
        h.run_steps(5);
        place(&mut h, &view, (40.0, 40.0), (1300.0, 820.0));
        for _ in 0..10 {
            h.run_steps(2);
        }
        shoot(&mut h, &dir, &format!("tile-textures-{name}-tile-view"));
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![view]));
        h.run_steps(3);
        // The tileset editor, on a copy of the tileset the game reads.
        let set = h.state().game.as_ref().unwrap().resman.get(&ResKey::new(tileset, ResType::SET));
        let set = set.unwrap().into_owned();
        let set_path = user.join("ttr01.set");
        std::fs::write(&set_path, set).unwrap();
        mg_ui::tileset_view::open_path(h.state_mut(), set_path);
        h.run_steps(3);
        h.state_mut().tilesets[0].page = mg_ui::tileset_view::Page::Tiles;
        h.state_mut().tilesets[0].tile = Some(tile as usize);
        h.run_steps(3);
        let editor = mg_ui::Tab::Tileset(h.state().tilesets[0].id);
        place(&mut h, &editor, (40.0, 20.0), (900.0, 870.0));
        h.run_steps(20);
        shoot(&mut h, &dir, &format!("tile-textures-{name}-tileset-editor"));
        // Its details scrolled down to the textures.
        h.get_by_label_contains("Textures of ").scroll_to_me();
        h.run_steps(20);
        shoot(&mut h, &dir, &format!("tile-textures-{name}-tileset-editor-scrolled"));
        place(&mut h, &editor, (40.0, 20.0), (420.0, 870.0));
        h.run_steps(20);
        h.get_by_label_contains("Textures of ").scroll_to_me();
        h.run_steps(20);
        shoot(&mut h, &dir, &format!("tile-textures-{name}-tileset-editor-narrow"));
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
    // (The palette's: the module tree's Expand All is another ⏷.)
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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

/// A project's plugins (the builder-tags example in a project's
/// `plugins/`): the question when it is opened (`project-plugins-ask`),
/// the Plugins window with it allowed (`project-plugins-window`), and its
/// settings' form (`plugin-settings`), in
/// `target/test-output/screens-project-plugins/`.
#[test]
#[ignore]
fn project_plugins() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-project-plugins");
    let project = dir.join("project");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    m.save_as(&mg_module::ModuleLocation::Folder(project.clone())).unwrap();
    std::fs::write(
        project.join("moonglow.toml"),
        "[plugins.\"example.builder-tags\"]\nprefix = \"KRZ_\"\n",
    )
    .unwrap();
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/plugins/examples/builder-tags");
    let into = project.join("plugins/builder-tags");
    std::fs::create_dir_all(&into).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(example.join(file), into.join(file)).unwrap();
    }
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.plugin_dir =
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mg-plugin/tests/fixtures"));
    app.load_plugins();
    app.open_module(&project);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    shoot(&mut h, &dir, "project-plugins-ask");
    h.get_by_label("Allow").click();
    h.run();
    h.state_mut().plugins.window = true;
    h.run();
    h.get_by_label("This project's plugins").scroll_to_me();
    h.run();
    shoot(&mut h, &dir, "project-plugins-window");
    h.state_mut().plugins.window = false;
    h.state_mut().open_plugin_settings("example.builder-tags");
    if let Some(form) = h.state_mut().plugins.settings_form.as_mut() {
        form.values.insert("code".into(), mg_plugin::SettingValue::Text("ab".into()));
    }
    h.run();
    shoot(&mut h, &dir, "plugin-settings");
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
    // Drawn with the game's skin, pictures and fonts, or the pictures are
    // not the editor's: Resources says where each came from, and what
    // was not found.
    assert!(h.query_by_label_contains("resource issue").is_none(), "the skin and fonts found");
    h.get_by_label("Resources").click();
    h.run();
    for found in ["fnt_maintext.ttf —", "nui_button_a."] {
        assert!(h.query_all_by_label_contains(found).next().is_some(), "{found} found");
    }
    h.get_by_label("Resources").click();
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
    settings.variants.insert("details".into(), json!({"type":"label","value":"Details view"}));
    settings.handlers = vec![mg_nui::Handler {
        event: "click".into(),
        element: "next".into(),
        action: mg_nui::Action::Variant { group: "view_host".into(), variant: "details".into() },
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

/// The Creator's fields: an image without a picture and a dropdown's option
/// without a label, their fields' hints (`nui-hints-image.png`,
/// `nui-hints-options.png`), and Split source while a window is typed in
/// its source, the field not left (`nui-split-typing.png`): the canvas
/// shows what is typed.
#[test]
#[ignore]
fn nui_fields() {
    use serde_json::json;
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens");
    let key = ResKey::parse("fields", ResType::JUI).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
    app.run(mg_ui::Action::NewNui("fields".into()));
    let mut window = mg_nui::window();
    window["root"]["children"] = json!([
        {"type":"image","value":"","height":40.0},
        {"type":"combo","id":"choice","value":0,"height":30.0,"elements":[["",0],["Second",1]]},
        {"type":"button","id":"mg_close","label":"Close","height":30.0}
    ]);
    app.ws.as_mut().unwrap().module.set(key, serde_json::to_vec_pretty(&window).unwrap());
    let mut h = nui_harness(app, key, (1200.0, 800.0));
    h.get_by_label_contains("Canvas Image").click();
    h.run();
    shoot(&mut h, &dir, "nui-hints-image");
    h.get_by_label_contains("Canvas Dropdown").click();
    h.run();
    shoot(&mut h, &dir, "nui-hints-options");
    h.get_by_value("Editor").click();
    h.run();
    h.get_by_label("Split source").click();
    h.run();
    let typed = json!({"version":1,"title":"Typed, not yet left","geometry":{"x":-1.0,"y":-1.0,"w":360.0,"h":220.0},
        "root":{"type":"col","children":[{"type":"label","value":"Shown as it is typed","height":40.0},
        {"type":"button","id":"mg_close","label":"Close","height":30.0}]}});
    let field = h
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains("\"version\"")))
        .expect("the source field");
    field.click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    h.event(egui::Event::Text(serde_json::to_string_pretty(&typed).unwrap()));
    h.run();
    shoot(&mut h, &dir, "nui-split-typing");
}

/// The module tree's Delete… with what names what it deletes: a script a
/// creature spawns with and another script runs (`delete-named-script.png`),
/// an area another area's door leads into (`delete-named-area.png`), and
/// three resources chosen, the start area among them
/// (`delete-several.png`; `DELETE_SHOTS_ONE=1` takes the first two
/// alone).
#[test]
#[ignore]
fn delete_says_what_names_it() {
    use mg_gff::{Gff, Struct, Value};
    use mg_schema::{StructExt, git, ifo};
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-delete-named");
    let key = |name: &str, t| ResKey::parse(name, t).unwrap();
    let rr = |name: &str| mg_core::ResRef::from_str(name).unwrap();
    let tag = |s: &str| Value::String(s.as_bytes().to_vec());
    let mut m = mg_module::Module::new();
    let mut info = Gff::new(*b"IFO ");
    for area in ["start", "yard"] {
        let mut listed = ifo::MOD_AREA_LIST.new_item();
        listed.write(&ifo::mod_area_list::AREA_NAME, rr(area));
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(listed);
        m.set(key(area, ResType::ARE), Gff::new(*b"ARE ").to_bytes().unwrap());
    }
    info.root.write(&ifo::MOD_ENTRY_AREA, rr("start"));
    m.set_info(&info).unwrap();
    let mut start = Gff::new(*b"GIT ");
    for (i, t) in ["GREETER", "GREETER_2", "INNKEEPER"].iter().enumerate() {
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::SCRIPT_SPAWN, rr("hello"));
        c.set("Tag", tag(t));
        c.set("XPosition", Value::Float(i as f32));
        start.root.items_mut(&git::CREATURE_LIST).push(c);
    }
    let mut door = Struct::new(8);
    door.set("Tag", tag("TO_YARD"));
    door.set("LinkedTo", tag("YARD_GATE"));
    start.root.set("Door List", Value::List(vec![door]));
    m.set_gff(key("start", ResType::GIT), &start).unwrap();
    let mut yard = Gff::new(*b"GIT ");
    let mut gate = Struct::new(8);
    gate.set("Tag", tag("YARD_GATE"));
    gate.set("LinkedTo", tag("TO_YARD"));
    yard.root.set("Door List", Value::List(vec![gate]));
    m.set_gff(key("yard", ResType::GIT), &yard).unwrap();
    m.set(key("hello", ResType::NSS), b"void main() { }\n".to_vec());
    m.set(
        key("caller", ResType::NSS),
        b"void main() {\n  ExecuteScript(\"hello\", OBJECT_SELF);\n}\n".to_vec(),
    );
    let path = scratch.join("delete.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().run(mg_ui::Action::DeleteDialog(key("hello", ResType::NSS)));
    shoot(&mut h, &dir, "delete-named-script");
    h.state_mut().confirm_delete = None;
    h.state_mut().run(mg_ui::Action::DeleteDialog(key("yard", ResType::ARE)));
    shoot(&mut h, &dir, "delete-named-area");
    h.state_mut().confirm_delete = None;
    if std::env::var_os("DELETE_SHOTS_ONE").is_none() {
        let chosen =
            [key("start", ResType::ARE), key("yard", ResType::ARE), key("caller", ResType::NSS)];
        h.state_mut().run(mg_ui::Action::DeleteChosenDialog(chosen.to_vec()));
        shoot(&mut h, &dir, "delete-several");
    }
}

/// Delete… on a NUI window whose event script has unsaved text in its
/// editor (`nui-delete-unsaved.png`), and Rename… on that event script,
/// which renames the window (`nui-rename-script.png`).
#[test]
#[ignore]
fn nui_delete_and_rename() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("screens");
    let key = ResKey::parse("shop", ResType::JUI).unwrap();
    let script = ResKey::parse("shop_e", ResType::NSS).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
    app.run(mg_ui::Action::NewNui("shop".into()));
    let mut h = nui_harness(app, key, (1100.0, 700.0));
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.get_by_label("Edit code").click();
    h.run();
    h.state_mut().script_tools.jump = Some((script, 0));
    h.run();
    h.event(egui::Event::Text("// typed\n".into()));
    h.run();
    h.state_mut().run(mg_ui::Action::DeleteDialog(key));
    shoot(&mut h, &dir, "nui-delete-unsaved");
    h.state_mut().confirm_delete = None;
    h.state_mut().rename_dialog(script);
    shoot(&mut h, &dir, "nui-rename-script");
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
    use mg_ui::search::{AdvancedSearch, Condition, Kind};
    let mut d = AdvancedSearch {
        first: Condition {
            kind: Kind::ItemProperty,
            row: Some(52),
            subtype: Some(15),
            ..Default::default()
        },
        ..Default::default()
    };
    h.state_mut().advanced_search(&mut d);
    h.state_mut().advanced_search = d;
    shoot(&mut h, &dir, "advanced-search-property");
    let mut d = AdvancedSearch {
        first: Condition { kind: Kind::Class, row: Some(4), ..Default::default() },
        placed: Some(true),
        ..Default::default()
    };
    h.state_mut().advanced_search(&mut d);
    h.state_mut().advanced_search = d;
    shoot(&mut h, &dir, "advanced-search-class");
    // Fighters of 4 to 10 levels with Alertness (feat 0), and the change
    // to make to them open.
    let mut d = AdvancedSearch {
        first: Condition { kind: Kind::Class, row: Some(4), levels: (4, 10), ..Default::default() },
        and: vec![Condition { kind: Kind::Feat, row: Some(0), ..Default::default() }],
        placed: Some(true),
        ..Default::default()
    };
    h.state_mut().advanced_search(&mut d);
    h.state_mut().advanced_search = d;
    h.run();
    h.query_by_label_contains("Change the ").expect("the change").click();
    h.run();
    shoot(&mut h, &dir, "advanced-search-and");
}

/// Edit › Find and Replace Text… over Chapter 1: "Aribeth" found, the
/// tags ticked too (where the window has them).
#[test]
#[ignore]
fn find_and_replace_text() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-find-replace");
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
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    h.run_steps(2);
    {
        let d = h.state_mut().text_replace.as_mut().unwrap();
        d.find = "Aribeth".into();
        d.with = "Aribeth de Tylmarande".into();
    }
    h.run();
    if let Some(tags) = h.query_by_label("Tags") {
        tags.click_accesskit();
    }
    h.run();
    h.get_by_label("Find").click_accesskit();
    h.run_steps(3);
    shoot(&mut h, &dir, "find-and-replace");
}

/// Find References for a talk-table line a blueprint, a script and a
/// hak's 2DA name (the last two where the tab finds them).
#[test]
#[ignore]
fn references_of_a_talk_table_line() {
    use mg_schema::{StructExt, ifo};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-strref");
    let user = scratch.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    let classes = b"2DA V2.0\n\n   Label Name\n0  Kz 16777300\n".to_vec();
    hak.add(mg_core::ResRef::from_str("classes").unwrap(), ResType::TWODA, classes).unwrap();
    std::fs::write(user.join("hak/kz_classes.hak"), hak.to_bytes().unwrap()).unwrap();
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    let mut item = ifo::MOD_HAK_LIST.new_item();
    item.write(&ifo::mod_hak_list::MOD_HAK, mg_schema::ExoString::from("kz_classes"));
    info.root.items_mut(&ifo::MOD_HAK_LIST).push(item);
    m.set_info(&info).unwrap();
    let mut sword = mg_gff::Gff::new(*b"UTI ");
    let named = mg_core::LocString { strref: mg_core::StrRef(16_777_300), strings: Vec::new() };
    sword.root.set("LocalizedName", mg_gff::Value::LocString(named));
    m.set_gff(ResKey::parse("kz_sword", ResType::UTI).unwrap(), &sword).unwrap();
    let script = b"void main()\n{\n    SpeakStringByStrRef(16777300);\n}\n".to_vec();
    m.set(ResKey::parse("kz_speak", ResType::NSS).unwrap(), script);
    let path = scratch.join("strref.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::FindStrRef(16_777_300));
    h.run_steps(4);
    shoot(&mut h, &dir, "references-strref");
}

/// Edit › Edit Areas Together… over Chapter 1: the areas, then (where the
/// window has it) those holding a value of MusicDay.
#[test]
#[ignore]
fn edit_areas_together() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-areas-together");
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
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Edit Areas Together…").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "edit-areas-together");
    if h.query_by_label("Holding").is_some() {
        h.get_by_value("Anything").click();
        h.run();
        h.get_all_by_label("A field's value").last().unwrap().click();
        h.run();
        for (hint, text) in [("MusicDay, Fog*, OnEnter", "MusicDay"), ("any", "3*")] {
            let by = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some(hint);
            h.get(egui_kittest::kittest::by().predicate(by)).click();
            h.run();
            h.get(egui_kittest::kittest::by().predicate(by)).type_text(text);
            h.run();
        }
        h.run_steps(3);
        shoot(&mut h, &dir, "edit-areas-together-holding");
    }
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
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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

/// Setup Store's pages as Aurora has them and as a project's
/// moonglow.toml makes them (its script, its store tag), and the
/// Duplicate Tags window a paste opens.
#[test]
#[ignore]
fn store_wizard_by_the_project_and_duplicate_tags() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    use mg_module::{Module, ModuleLocation};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let dir = mg_testkit::output_dir("screens");
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m: Module = new_module(&game, "Shop", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Market".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bp = game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap();
    let bp = mg_gff::Gff::read(&bp).unwrap();
    let none = |_: mg_core::ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let keeper = instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![keeper]));
    m.set_gff(git, &g).unwrap();
    let scratch = mg_testkit::scratch_dir("screens-store-project");
    let plain = scratch.join("plain");
    let project = scratch.join("project");
    m.save_as(&ModuleLocation::Folder(plain.clone())).unwrap();
    m.save_as(&ModuleLocation::Folder(project.clone())).unwrap();
    std::fs::write(
        project.join("moonglow.toml"),
        "[store]\nopen_script = \"tdod_vendor_open\"\ntag = \"{area}_STORE_{n}\"\n\
         blueprint = \"nw_storethief001\"\nkeeper_variable = \"tdod_store\"\n",
    )
    .unwrap();
    for (name, folder) in [("aurora", &plain), ("project", &project)] {
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        app.open_module(folder);
        app.open_palette = false;
        mg_ui::store_wizard::open(&mut app, area, "Creature List", 0);
        if name == "aurora" {
            app.store_wizard.as_mut().unwrap().store =
                mg_core::ResRef::from_str("nw_storethief001").ok();
        }
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        shoot(&mut h, &dir, &format!("setup-store-{name}-conversation"));
        h.get_by_label("Next >").click();
        h.run_steps(3);
        shoot(&mut h, &dir, &format!("setup-store-{name}-store"));
        h.get_by_label("Next >").click();
        h.run_steps(3);
        shoot(&mut h, &dir, &format!("setup-store-{name}-finish"));
        if name == "project" {
            h.state_mut().store_wizard = None;
            // (Shown as the builder's option says: Don't warn again.)
            h.state_mut().settings.warn_duplicate_tags = true;
            let made = |i: usize| mg_ui::tag_clash::Made {
                key: git,
                path: GffPath::root().item("WaypointList", i),
            };
            let clash = |tag: &str, count, unique: Option<&str>| mg_module::tags::Clash {
                tag: tag.into(),
                count,
                unique: unique.map(str::to_string),
                area: None,
            };
            // (An area's copy: the tags only counted, Retag All for them;
            // a script that spells one.)
            h.state_mut().tag_clash = Some(mg_ui::tag_clash::TagClash {
                what: "The copy of market".into(),
                clashes: vec![
                    clash("TDOD_START_KREZK", 2, Some("TDOD_START_*")),
                    clash("KREZK_RENTAL_DOOR", 3, None),
                ],
                objects: vec![
                    (made(1), "TDOD_START_KREZK".into()),
                    (made(2), "KREZK_RENTAL_DOOR".into()),
                ],
                others: 2,
                more: vec![(made(3), "WP_MARKET".into()), (made(4), "WP_WELL".into())],
                made: (1..5).map(made).collect(),
                scripts: vec!["kz_rental.nss line 14".into()],
                from_option: true,
            });
            shoot(&mut h, &dir, "duplicate-tags");
        }
    }
}

/// Verify Module's report with a fix beside its findings: a plugin's
/// (the tag-conventions fixture's) and the doctor's (a unique tag on two
/// waypoints); then one made.
#[test]
#[ignore]
fn verify_fixes() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let work = mg_testkit::scratch_dir("screens-verify-fixes").join("chess");
    let mut m = mg_module::Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let area = ResKey::parse("chess", ResType::GIT).unwrap();
    let mut g = m.gff(&area).unwrap().unwrap();
    let mut w = mg_gff::Struct::new(5);
    w.set("Tag", mg_gff::Value::String(b"TDOD_START_KREZK".to_vec()));
    w.set("XPosition", mg_gff::Value::Float(1.0));
    w.set("YPosition", mg_gff::Value::Float(1.0));
    g.root.set("WaypointList", mg_gff::Value::List(vec![w.clone(), w]));
    m.set_gff(area, &g).unwrap();
    // (The chess pieces' tags upper case: one finding of the plugin's.)
    let pieces: Vec<ResKey> = m.keys().filter(|k| k.restype == ResType::UTC).copied().collect();
    for key in pieces {
        let mut piece = m.gff(&key).unwrap().unwrap();
        let tag = piece.root.string("Tag").unwrap_or_default().to_ascii_uppercase();
        piece.root.set("Tag", mg_gff::Value::String(tag));
        m.set_gff(key, &piece).unwrap();
    }
    let mut guard = mg_gff::Gff::new(*b"UTC ");
    guard.root.set("Tag", mg_gff::Value::String(b"gate_guard".to_vec()));
    m.set_gff(ResKey::parse("guard", ResType::UTC).unwrap(), &guard).unwrap();
    m.save_as(&mg_module::ModuleLocation::Folder(work.clone())).unwrap();
    std::fs::write(work.join("moonglow.toml"), "[tags]\nunique = [\"TDOD_START_*\"]\n").unwrap();

    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&work);
    app.plugin_dir =
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mg-plugin/tests/fixtures"));
    app.load_plugins();
    app.enable_plugin("example.tag-conventions", true);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.state_mut().log.clear();
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    shoot(&mut h, &dir, "verify-fixes");
    h.get_all_by_label("Upper-case this tag").next().unwrap().click();
    h.run();
    shoot(&mut h, &dir, "verify-fixed");
}

/// The Variables window's Add Set menu (the saved sets, Add from File…,
/// Manage Sets…, Open Sets Folder; Save to File… beside Save Set…), and
/// the Variable Sets window with a set being renamed and one deleted.
#[test]
#[ignore]
fn variable_sets() {
    use mg_ui::widgets::{FieldTarget, VarRow, VarTableEdit};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let scratch = mg_testkit::scratch_dir("screens-variable-sets");
    let sets = scratch.join("variable-sets");
    let row = |n: &str, k: u32, v: &str| VarRow::new(n, k, v);
    for (name, rows) in [
        ("Loot chest", vec![row("MG_LOOT", 1, "3"), row("MG_LOOT_TABLE", 3, "chest_low")]),
        ("Locked door", vec![row("MG_LOCK_DC", 1, "25")]),
        ("Merchant", vec![row("MG_MARKUP", 2, "1.25")]),
    ] {
        mg_ui::var_sets::save(&sets, name, &rows).unwrap();
    }
    // (A file edited by hand that isn't a set: said so.)
    std::fs::write(sets.join("Guards.vars.json"), "{ \"variables\": [").unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let key = ResKey::parse("nw_waypoint001", ResType::UTW).unwrap();
    let data = game.resman.get_named("nw_waypoint001", ResType::UTW).unwrap().into_owned();
    let mut m = mg_module::Module::new();
    m.set(key, data);
    let path = scratch.join("sets.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.open_palette = false;
    app.var_set_dir = Some(sets.clone());
    let mut edit = VarTableEdit::new(FieldTarget::new(key, GffPath::root(), "VarTable"), &[]);
    edit.rows = vec![row("MG_LOOT", 1, "9"), row("MG_GREETING", 3, "Well met")];
    app.var_edit = Some(edit);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 640.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("Add Set").click();
    shoot(&mut h, &dir, "variables-add-set-menu");
    // What a set holds, as the pointer rests on it.
    h.get_by_label("Loot chest").hover();
    h.run_steps(3);
    shoot(&mut h, &dir, "variables-add-set-tip");
    h.get_by_label("Manage Sets…").click();
    h.run_steps(3);
    h.get_all_by_label("Delete").last().unwrap().click();
    h.run_steps(3);
    h.get_all_by_label("Rename…").next().unwrap().click();
    h.run_steps(3);
    {
        let w = h.state_mut().var_edit.as_mut().unwrap().sets.as_mut().unwrap();
        w.renaming = Some(("Locked door".into(), "loot chest".into()));
    }
    shoot(&mut h, &dir, "variables-sets-window");
}

/// The Variables window without described variables and with a
/// project's (`variables.toml`): the names offered as one is typed, a
/// value chosen from its values, what the description finds wrong; the
/// Builder Code window, and the Area Wizard's ResRef by the code.
#[test]
#[ignore]
fn described_variables_and_the_builder_code() {
    use mg_module::{Module, ModuleLocation};
    use mg_ui::widgets::{FieldTarget, VarRow, VarTableEdit};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let dir = mg_testkit::output_dir("screens");
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m: Module = mg_module::new::new_module(&game, "Krezk", &mut rng).unwrap();
    let spec = mg_module::new::AreaSpec {
        name: "Market".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = mg_module::new::add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Placeable List", mg_gff::Value::List(vec![mg_gff::Struct::new(9)]));
    m.set_gff(git, &g).unwrap();
    let scratch = mg_testkit::scratch_dir("screens-described-variables");
    let plain = scratch.join("plain");
    let project = scratch.join("project");
    m.save_as(&ModuleLocation::Folder(plain.clone())).unwrap();
    m.save_as(&ModuleLocation::Folder(project.clone())).unwrap();
    std::fs::write(
        project.join("variables.toml"),
        "undescribed = [\"tdod_*\"]\n\n\
         [tdod_workbench]\ntype = \"string\"\non = [\"placeable\"]\n\
         values = { forge = \"Smithing\", tanner = \"Leatherworking\", loom = \"Tailoring\", \
         carpenter = \"Woodworking\", fire = \"a kitchen, for Cooking\" }\n\
         help = \"The craft the bench serves: a player works within 5 m of it\"\n\n\
         [tdod_harvest]\ntype = \"string\"\non = [\"placeable\"]\nvalues = [\"herb\", \"ore\", \"timber\"]\n\n\
         [tdod_harvest_dc]\ntype = \"int\"\non = [\"placeable\"]\nmin = 1\nmax = 60\n\n\
         [tdod_light]\ntype = \"int\"\nvalues = { 1 = \"lit\" }\nhelp = \"A natural light against delirium, within 3.33 m\"\n\n\
         [tdod_lit]\ntype = \"int\"\non = [\"placeable\"]\nvalues = { 1 = \"lit\" }\n\n\
         [tdod_banker]\ntype = \"int\"\non = [\"creature\", \"placeable\"]\nvalues = { 1 = \"a banker\" }\n\n\
         [\"tdod_rental_*\"]\ntype = \"int\"\non = [\"placeable\"]\nhelp = \"A room's rent\"\n",
    )
    .unwrap();
    let rows = || {
        vec![
            VarRow::new("tdod_harvest", 3, "Herb"),
            VarRow::new("tdod_lit", 3, "1"),
            VarRow::new("tdod_harvest_dc", 1, "75"),
            VarRow::new("tdod_", 1, "0"),
        ]
    };
    for (name, folder) in [("plain", &plain), ("described", &project)] {
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        app.open_module(folder);
        app.open_palette = false;
        let path = GffPath::root().item("Placeable List", 0);
        let mut edit = VarTableEdit::new(FieldTarget::new(git, path, "VarTable"), &[]);
        edit.rows = rows();
        app.var_edit = Some(edit);
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 640.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(3);
        // The last row's name, being typed.
        h.get_all_by_value("tdod_").next().unwrap().click();
        h.run_steps(3);
        shoot(&mut h, &dir, &format!("variables-{name}-typed"));
        if name == "described" {
            // Down twice: the third offered, marked.
            h.key_press(egui::Key::ArrowDown);
            h.key_press(egui::Key::ArrowDown);
            h.run_steps(3);
            shoot(&mut h, &dir, "variables-described-offer-keys");
            h.get_all_by_value("tdod_").next().unwrap().type_text("w");
            h.run_steps(2);
            h.get_by_label("tdod_workbench").click();
            h.run_steps(3);
            h.get_by_value("(choose)").click();
            h.run_steps(3);
            shoot(&mut h, &dir, "variables-described-choice");
            h.key_press(egui::Key::Escape);
            h.run_steps(3);
            h.state_mut().var_edit = None;
            h.run_steps(2);
            h.state_mut().builder_code =
                Some(mg_ui::builder_code::Draft { code: "ab".into(), root: project.clone() });
            shoot(&mut h, &dir, "builder-code");
            h.get_all_by_label("OK").last().unwrap().click();
            h.run_steps(3);
            h.state_mut().actions.push(mg_ui::Action::AreaWizard);
            h.run_steps(3);
            shoot(&mut h, &dir, "area-wizard-builder-code");
        }
    }
}

/// The Domains of Dread's builder proposal, C6 to C8: an area another
/// builder claimed (its banner, the module tree's marks), a merged area's
/// Tag about to change, a save of an unclaimed area, File › Project, and
/// the question before a project's command first runs.
#[test]
#[ignore]
fn claims_and_project_commands() {
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-claims");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Krezk", &mut rng).unwrap();
    let mut areas = Vec::new();
    for name in ["Krezk Square", "Abbey Gate", "Mill"] {
        let spec = AreaSpec {
            name: name.into(),
            tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
            width: 4,
            height: 4,
        };
        areas.push(add_area(&mut m, &game, &spec, &mut rng).unwrap());
    }
    let project = mg_testkit::scratch_dir("screens-claims").join("krezk");
    m.save_as(&mg_module::ModuleLocation::Folder(project.clone())).unwrap();
    std::fs::write(
        project.join("moonglow.toml"),
        "[claims]\nfile = \"builders/claims.2da\"\nothers = \"read-only\"\nunclaimed = \"warn\"\n\n\
         [commands]\nupdate = \"bin/update\"\nsubmit = { run = \"bin/submit\", label = \"Submit for Review\" }\n",
    )
    .unwrap();
    std::fs::create_dir_all(project.join("builders")).unwrap();
    std::fs::write(
        project.join("builders/claims.2da"),
        format!(
            "2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n\
             0   {:<16} cd       claimed  2026-10-20  ****\n\
             1   {:<16} ab       merged   2026-10-01  ****\n",
            areas[0], areas[1]
        ),
    )
    .unwrap();
    mg_module::project_file::ProjectFile::set_builder_code(&project, Some("ab")).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.settings.game_shaders = Some(false);
    app.settings.no_last_area = true;
    app.open_module(&project);
    app.open_palette = false;
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(areas[0])));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(6);
    shoot(&mut h, &dir, "claims-area-read-only");
    // A merged area: its note, and its Tag about to change.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(areas[1])));
    h.run_steps(6);
    let key = ResKey::new(areas[1], ResType::ARE);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Tag",
        vec![mg_edit::Edit::SetField {
            key,
            path: GffPath::root(),
            label: "Tag".into(),
            value: Some(mg_gff::Value::String(b"ABBEY_GATE_2".to_vec())),
        }],
    )));
    h.run_steps(3);
    shoot(&mut h, &dir, "claims-protected-tag");
    h.get_by_label("Change It Anyway").click();
    h.run_steps(3);
    // An unclaimed area changed, and saved.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(areas[2])));
    h.run_steps(6);
    let key = ResKey::new(areas[2], ResType::ARE);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Tag",
        vec![mg_edit::Edit::SetField {
            key,
            path: GffPath::root(),
            label: "Tag".into(),
            value: Some(mg_gff::Value::String(b"MILL_2".to_vec())),
        }],
    )));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    shoot(&mut h, &dir, "claims-save-unclaimed");
    h.get_by_label("Save Anyway").click();
    h.run_steps(3);
    // File › Project, and the question before a command first runs.
    h.get_by_label("File").click();
    h.run_steps(2);
    h.get_by_label("Project").hover();
    h.run_steps(3);
    shoot(&mut h, &dir, "claims-project-menu");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("update".into()));
    h.run_steps(3);
    shoot(&mut h, &dir, "claims-command-trust");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run_steps(3);
    // The Builder Code window: a free code, and one that holds claims.
    for (code, name) in [("ef", "claims-builder-code-free"), ("cd", "claims-builder-code-taken")] {
        h.state_mut().builder_code =
            Some(mg_ui::builder_code::Draft { code: code.into(), root: project.clone() });
        h.run_steps(3);
        shoot(&mut h, &dir, name);
    }
    h.state_mut().builder_code = None;
    // Edit › Claims Table…: the table whole, Take Over… asked, Hand To….
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Claims Table…").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "claims-table");
    h.get_by_label("Take Over…").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "claims-table-take-over");
    h.get_by_label("Hand To…").click();
    h.run_steps(3);
    shoot(&mut h, &dir, "claims-table-hand-to");
}

/// What a plugin says before a save (plugin API 0.4, `resource-saving`):
/// the House rules example's warning with its fix, and its refusal
/// (`target/test-output/screens-plugin-save/`).
#[test]
#[ignore]
fn plugin_save_warning() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("screens-plugin-save");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let area = |tag: &str| {
        let mut are = mg_gff::Gff::new(*b"ARE ");
        are.root.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
        are.to_bytes().unwrap()
    };
    m.set(ResKey::parse("tavern", ResType::ARE).unwrap(), area("TAVERN"));
    let path = dir.join("rules.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.open_palette = false;
    app.plugin_dir =
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples"));
    app.load_plugins();
    app.enable_plugin("example.house-rules", true);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 720.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    // A new area whose tag is not upper case: a warning, with its fix.
    let key = ResKey::parse("cellar", ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A cellar",
        vec![mg_edit::Edit::SetResource { key, data: Some(area("Cellar")) }],
    )));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Save);
    shoot(&mut h, &dir, "plugin-save-warning");
    h.get_by_label("Cancel").click();
    h.run_steps(2);
    // And the tavern's tag changed too: a refusal, first.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Tag",
        vec![mg_edit::Edit::SetField {
            key: ResKey::parse("tavern", ResType::ARE).unwrap(),
            path: GffPath::root(),
            label: "Tag".into(),
            value: Some(mg_gff::Value::String(b"INN".to_vec())),
        }],
    )));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Save);
    shoot(&mut h, &dir, "plugin-save-refused");
}

/// An armor whose left bicep and forearm are 0 (a builder's report: 0
/// couldn't be chosen): its Appearance page, the bicep's list open, and
/// the armor in the model viewer, the body's bare arm where its parts are
/// 0. `MG_SHOT` names the pictures (`before`, `after`).
#[test]
#[ignore]
fn armor_part_none() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-armor-part-none");
    let shot = std::env::var("MG_SHOT").unwrap_or_else(|_| "after".into());
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named("nw_aarcl001", ResType::UTI).unwrap().into_owned();
    let mut armor = mg_gff::Gff::read(&data).unwrap();
    for label in ["ArmorPart_LBicep", "ArmorPart_LFArm"] {
        armor.root.set(label, mg_gff::Value::Byte(0));
        armor.root.set(&mg_rules::items::wide_label(label), mg_gff::Value::Word(0));
    }
    let key = ResKey::parse("mg_armor_none", ResType::UTI).unwrap();
    let mut m = mg_module::Module::new();
    m.set_gff(key, &armor).unwrap();
    let path = dir.join("armor.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.open_palette = false;
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    app.blueprint_pages.insert((key, GffPath::root()), "Appearance");
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    shoot(&mut h, &dir, &format!("{shot}-appearance"));
    let label = h.get_by_label("Left Bicep").rect();
    h.get_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| {
            let r = n.rect();
            r.min.x > label.max.x && (r.center().y - label.center().y).abs() < 6.0
        })
        .expect("the left bicep's list")
        .click();
    h.run_steps(3);
    shoot(&mut h, &dir, &format!("{shot}-bicep-list"));
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(key)));
    h.run_steps(6);
    shoot(&mut h, &dir, &format!("{shot}-model"));
}

/// The resource browser searching for portraits.2da, which two haks of
/// the module have, the first over the second (a builder's report: only
/// the higher one was listed); and the lower hak's copy opened from its
/// row. `MG_SHOT` names the pictures (`before`, `after`).
#[test]
#[ignore]
fn browser_overridden_copies() {
    use mg_schema::{ExoString, StructExt, ifo};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-browser-copies");
    let out = mg_testkit::output_dir("screens-browser-copies-shots");
    let shot = std::env::var("MG_SHOT").unwrap_or_else(|_| "after".into());
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    for name in ["mg_top", "mg_low"] {
        let mut w = mg_erf::ErfWriter::new(*b"HAK ");
        let table = format!("2DA V2.0\n\n LABEL BaseResRef\n0 {name} po_{name}\n").into_bytes();
        w.add(mg_core::ResRef::from_str("portraits").unwrap(), ResType::TWODA, table).unwrap();
        std::fs::write(user.join(format!("hak/{name}.hak")), w.to_bytes().unwrap()).unwrap();
    }
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("COPIES"));
    for h in ["mg_top", "mg_low"] {
        let mut s = ifo::MOD_HAK_LIST.new_item();
        s.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(h));
        info.root.items_mut(&ifo::MOD_HAK_LIST).push(s);
    }
    m.set_info(&info).unwrap();
    let path = dir.join("copies.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Resources));
    app.browser.filter = "portraits".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    shoot(&mut h, &out, &format!("{shot}-search"));
    // The lower hak's row, where there is one, opened.
    let Some(row) = h.query_by_label_contains("hak:mg_low") else { return };
    let at = h
        .get_all_by_label("portraits.2da")
        .map(|n| n.rect())
        .find(|r| (r.center().y - row.rect().center().y).abs() < 6.0)
        .expect("its name");
    h.get_all_by_label("portraits.2da").find(|n| n.rect() == at).unwrap().click_secondary();
    h.run_steps(2);
    h.get_by_label("Open").click();
    h.run_steps(4);
    shoot(&mut h, &out, &format!("{shot}-copy"));
}

#[test]
#[ignore]
fn panes_folded() {
    // GitHub issue 10: each pane beside the area (the module tree, the
    // palettes, the log) shown and folded away (View's keys: Ctrl+Alt+1, 2
    // and 3; 0 for all), in a wide window, a narrow one and at a high-DPI
    // scale.
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    let shots = [
        ("", (1400.0, 860.0), 1.0),
        ("-narrow", (760.0, 520.0), 1.0),
        ("-hidpi", (1000.0, 700.0), 2.0),
    ];
    for (name, size, scale) in shots {
        let rs = egui_kittest::wgpu::create_render_state(
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        app.set_render_state(rs.clone());
        app.open_module(&root.join("data/nwm/Chapter1.nwm"));
        let mut h = Harness::builder()
            .with_size(egui::vec2(size.0, size.1))
            .with_pixels_per_point(scale)
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(3);
        let area = mg_core::ResRef::from_str("map_m1q1a").unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
        h.run_steps(10);
        shoot(&mut h, &dir, &format!("panes{name}-shown"));
        // (The tree's button under the pointer, lit.)
        h.get_by_label("Fold Module Tree").hover();
        h.run_steps(30);
        shoot(&mut h, &dir, &format!("panes{name}-hover"));
        for (key, pane) in
            [(egui::Key::Num1, "tree"), (egui::Key::Num2, "palettes"), (egui::Key::Num3, "log")]
        {
            h.key_press_modifiers(keys, key);
            h.run_steps(4);
            shoot(&mut h, &dir, &format!("panes{name}-{pane}-folded"));
            h.key_press_modifiers(keys, key);
            h.run_steps(4);
        }
        h.key_press_modifiers(keys, egui::Key::Num0);
        h.run_steps(4);
        shoot(&mut h, &dir, &format!("panes{name}-all-folded"));
        h.key_press_modifiers(keys, egui::Key::Num0);
        h.run_steps(4);
        if !name.is_empty() {
            continue;
        }
        // The palettes split off the middle's left: their button on their
        // right edge, and their strip at the left.
        {
            let dock = &mut h.state_mut().dock;
            let palettes = dock.find_tab(&mg_ui::Tab::Palette).unwrap();
            dock.remove_tab(palettes);
            let node = dock.find_tab(&mg_ui::Tab::Area(area)).unwrap().node;
            dock.main_surface_mut().split_left(node, 0.25, vec![mg_ui::Tab::Palette]);
        }
        h.run_steps(4);
        shoot(&mut h, &dir, "panes-palettes-left-shown");
        h.key_press_modifiers(keys, egui::Key::Num2);
        h.run_steps(4);
        shoot(&mut h, &dir, "panes-palettes-left-folded");
    }
}

/// The module tree with a script's tab chosen (one far down a long list
/// of scripts, opened as from a script field's Edit): its row marked,
/// the Scripts group opened and brought into view.
#[test]
#[ignore]
fn module_tree_follows_the_tab_chosen() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let mut m = mg_module::Module::new();
    for n in 0..40 {
        let key = ResKey::parse(&format!("script_{n:02}"), ResType::NSS).unwrap();
        m.set(key, b"void main()\n{\n}\n".to_vec());
    }
    let path = dir.join("tree-follows.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    // (Options › General's Show the Tab Chosen in the Module Tree off
    // first: the tree as it was before, and as Aurora's is.)
    for (stays, name) in [(true, "tree-follows-script-off"), (false, "tree-follows-script")] {
        h.state_mut().settings.tree_stays = stays;
        let key = ResKey::parse(&format!("script_{}", if stays { 32 } else { 33 }), ResType::NSS);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key.unwrap())));
        h.run_steps(12);
        shoot(&mut h, &dir, name);
    }
}

/// The module tree with names longer than its pane (Chapter 1's areas by
/// name, their ResRefs beside them): a builder's report that only
/// widening the pane showed them whole. Then scrolled to their ends, and
/// gone to by typing a name.
#[test]
#[ignore]
fn module_tree_long_names() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    app.settings.area_names = true;
    app.settings.name_resrefs = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    // (The Areas clicked, which gives the tree the keys, and opened.)
    h.get_by_label_contains("Areas (").click();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowRight);
    h.run_steps(3);
    shoot(&mut h, &dir, "tree-long-names");
    // Scrolled right, with the pointer over the tree.
    h.get_by_label_contains("Areas (").hover();
    h.run_steps(2);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-400.0, 0.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    h.run_steps(5);
    shoot(&mut h, &dir, "tree-long-names-scrolled");
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(400.0, 0.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    h.run_steps(5);
    // A name typed: the row it begins.
    for c in "beggar's nest: t".chars() {
        h.event(egui::Event::Text(c.to_string()));
        h.run_steps(2);
    }
    h.run_steps(5);
    shoot(&mut h, &dir, "tree-long-names-typed");
}

/// The Talk Table editor on a nasher project's table kept as JSON in two
/// languages: the languages' list beside its name, and opened.
#[test]
#[ignore]
fn talk_table_kept_in_a_project() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens").join("talk-project");
    let _ = std::fs::remove_dir_all(&dir);
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_proj".to_vec()));
    m.set_info(&info).unwrap();
    let project = dir.join("project");
    m.save_as(&mg_module::ModuleLocation::Project {
        root: project.clone(),
        target: "default".into(),
    })
    .unwrap();
    let mut cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
    cfg.push_str(
        "\n[target]\nname = \"tlk\"\nfile = \"mg_proj.tlk\"\n  [target.sources]\n  \
         include = \"src/tlk/*.json\"\n\n[target]\nname = \"tlk_de\"\nfile = \"de/mg_proj.tlk\"\n  \
         [target.sources]\n  include = \"src/tlk/de/*.json\"\n",
    );
    std::fs::write(project.join("nasher.cfg"), cfg).unwrap();
    std::fs::create_dir_all(project.join("src/tlk/de")).unwrap();
    std::fs::write(
        project.join("src/tlk/mg_proj.tlk.json"),
        r#"{"language":0,"entries":[{"id":0,"text":"Hello"},{"id":1,"text":"The ferry leaves at dawn."},{"id":2,"text":"","sound":"vs_x"}]}"#,
    )
    .unwrap();
    std::fs::write(
        project.join("src/tlk/de/mg_proj.tlk.json"),
        r#"{"language":2,"entries":[{"id":0,"text":"Hallo"}]}"#,
    )
    .unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&project);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::TalkTable));
    h.run_steps(4);
    h.state_mut().talk_view.selected = Some(1);
    shoot(&mut h, &dir, "talk-table-project");
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let english = (h.query_all(combo))
        .find(|n| n.accesskit_node().value().is_some_and(|v| v == "English"))
        .map(|n| n.rect().center());
    if let Some(at) = english {
        h.hover_at(at);
        let button = egui::PointerButton::Primary;
        for pressed in [true, false] {
            let modifiers = egui::Modifiers::NONE;
            h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
        }
        shoot(&mut h, &dir, "talk-table-project-languages");
    }
}

/// Area Properties › Audio with Ambient Sound, Day open, after a few
/// frames and after many (a builder's report: the open list grew to the
/// window's right edge over a few seconds).
#[test]
#[ignore]
fn area_properties_audio_dropdown() {
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
    let path = dir.join("audio.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.open_palette = false;
    let are = ResKey::new(area, ResType::ARE);
    app.blueprint_pages.insert((are, GffPath::root()), "Audio");
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::AreaProperties(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    place(&mut h, &mg_ui::Tab::AreaProperties(area), (260.0, 70.0), (700.0, 600.0));
    h.run_steps(3);
    let line = h.get_by_label("Ambient Sound, Day").rect().center().y;
    let combo = egui::accesskit::Role::ComboBox;
    let at = h
        .get_all_by_role(combo)
        .find(|n| (n.rect().center().y - line).abs() < 6.0)
        .expect("the Ambient Sound, Day dropdown")
        .rect()
        .center();
    h.hover_at(at);
    let button = egui::PointerButton::Primary;
    let modifiers = egui::Modifiers::NONE;
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
    }
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    // The Filter field's width, and the widest row's under it.
    let widths = |h: &Harness<'_, Moonglow>| {
        let filter = h.get(egui_kittest::kittest::by().predicate(hint)).rect();
        let row = h
            .query_all(egui_kittest::kittest::by().role(egui::accesskit::Role::Button))
            .map(|n| n.rect())
            .filter(|r| r.top() > filter.bottom() && r.left() >= filter.left() - 20.0)
            .map(|r| r.width())
            .fold(0.0, f32::max);
        (filter.width(), row)
    };
    h.run_steps(3);
    eprintln!("after 3 frames: {:?}", widths(&h));
    shoot(&mut h, &dir, "area-audio-dropdown-early");
    for i in 0..500 {
        // (The pointer moves over the list, as a builder's does.)
        h.hover_at(egui::pos2(at.x + (i % 7) as f32, at.y + 60.0));
        h.step();
    }
    eprintln!("after 500 more: {:?}", widths(&h));
    shoot(&mut h, &dir, "area-audio-dropdown-late");
    // Another, lower in the window: Environmental Audio Effects (its
    // rows within its box from the first frame drawn).
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    let line = h.get_by_label("Environmental Audio Effects").rect().center().y;
    let r#box = h
        .get_all_by_role(combo)
        .find(|n| (n.rect().center().y - line).abs() < 6.0)
        .expect("the Environmental Audio Effects dropdown")
        .rect();
    h.hover_at(r#box.center());
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: r#box.center(), button, pressed, modifiers });
    }
    for frame in 1..=4 {
        h.step();
        let rows = h
            .query_all(egui_kittest::kittest::by().role(egui::accesskit::Role::Button))
            .map(|n| n.rect())
            .filter(|r| r.top() > r#box.bottom() && (r.left() - r#box.left()).abs() < 20.0)
            .map(|r| (r.left(), r.right()))
            .fold((f32::MAX, 0.0f32), |(l, r), (a, b)| (l.min(a), r.max(b)));
        eprintln!("frame {frame}: the box {:?}, its rows {rows:?}", (r#box.left(), r#box.right()));
    }
    shoot(&mut h, &dir, "area-audio-dropdown-eax");
}

/// A placeable's Appearance list open (long, and wider than its box), on
/// the first frame it is drawn and 300 frames later: the same width.
#[test]
#[ignore]
fn appearance_dropdown_frames() {
    use mg_module::ModuleLocation;
    use mg_module::new::new_module;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Chest", &mut rng).unwrap();
    let path = dir.join("chest.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.open_palette = false;
    let data = app.game.as_deref().unwrap().resman.get_named("plc_chest1", ResType::UTP).unwrap();
    let key = ResKey::parse("chest_width", ResType::UTP).unwrap();
    let edit = mg_edit::Edit::SetResource { key, data: Some(data.into_owned()) };
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    let at = h
        .get_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("Chest")))
        .expect("the Appearance Type dropdown")
        .rect()
        .center();
    h.hover_at(at);
    let button = egui::PointerButton::Primary;
    let modifiers = egui::Modifiers::NONE;
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
    }
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    let filter =
        |h: &Harness<'_, Moonglow>| h.get(egui_kittest::kittest::by().predicate(hint)).rect();
    // (The click opens it in a pass that measures it, unseen.)
    h.step();
    h.step();
    eprintln!("the first frame drawn: the Filter field {:?}", filter(&h));
    let image = h.render().expect("render");
    image.save(dir.join("appearance-dropdown-first.png")).unwrap();
    h.run_steps(300);
    eprintln!("300 frames later: the Filter field {:?}", filter(&h));
    let image = h.render().expect("render");
    image.save(dir.join("appearance-dropdown-300.png")).unwrap();
}

/// The area view's toolbar with the Shadows button's menu open (held or
/// right-clicked: Sun and Moon Shadows, Creature Shadows, Light Shadows),
/// with the buttons labelled and as icons only, in
/// `target/test-output/screens-area/shadows-menu-*.png`.
#[test]
#[ignore]
fn area_shadows_menu() {
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens-area");
    let scratch = mg_testkit::scratch_dir("screens-area-shadows");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Shadows", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let path = scratch.join("shadows.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    for icons_only in [false, true] {
        let rs = egui_kittest::wgpu::create_render_state(
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        app.settings.toolbar_icons_only = icons_only;
        app.set_render_state(rs.clone());
        app.open_module(&path);
        app.open_palette = false;
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1400.0, 700.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(5);
        let label = if icons_only { "◑" } else { "◑ Shadows" };
        h.get_by_label(label).click_secondary();
        let name = if icons_only { "shadows-menu-icons" } else { "shadows-menu" };
        shoot(&mut h, &dir, name);
    }
}

/// The main window's log and module tree as the deferred sprint's third
/// package left them (`w3-` pictures): the log's corner buttons; the tree
/// following a script far down the list (brought up from below), Module
/// Properties' tab, a tab of several blueprints and a script the Filter
/// hides; the Filter finding placed objects in areas not opened out; rows
/// chosen with Shift and Ctrl, and their menu.
#[test]
#[ignore]
fn main_window_log_and_tree() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_palette = false;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    for n in 0..30 {
        app.log.info(format!("Line {n} of what was said"));
    }
    app.log.warn("A warning, in its color");
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    shoot(&mut h, &dir, "w3-log");
    // (Each window to the right, the tree in sight.)
    let open = |h: &mut Harness<'_, Moonglow>, tab: mg_ui::Tab| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        h.run_steps(2);
        if h.state().dock.find_tab(&tab).is_some_and(|p| !p.surface.is_main()) {
            place(h, &tab, (480.0, 60.0), (560.0, 460.0));
        }
        h.run_steps(14);
    };
    let script = |name: &str| mg_ui::Tab::Script(ResKey::parse(name, ResType::NSS).unwrap());
    h.state_mut().actions.push(mg_ui::Action::CloseTab(mg_ui::Tab::ModuleProperties));
    h.run_steps(3);
    open(&mut h, script("m1q5a09bertclg"));
    shoot(&mut h, &dir, "w3-tree-from-below");
    open(&mut h, mg_ui::Tab::ModuleProperties);
    shoot(&mut h, &dir, "w3-tree-module-properties");
    let utc = |name: &str| ResKey::parse(name, ResType::UTC).unwrap();
    open(&mut h, mg_ui::Tab::Blueprints(vec![utc("attendant"), utc("bertrand")]));
    shoot(&mut h, &dir, "w3-tree-several");
    // The Filter typed: placed objects found, then a script it hides
    // chosen.
    // (The tree's Filter is the first text field.)
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().click();
    h.run_steps(2);
    h.event(egui::Event::Text("gangleader".into()));
    h.run_steps(8);
    shoot(&mut h, &dir, "w3-tree-filter-objects");
    open(&mut h, script("m1s1gillclg09"));
    shoot(&mut h, &dir, "w3-tree-filtered-tab");
    // Rows chosen with Shift and Ctrl, and their menu.
    h.get_all_by_label("×").next().expect("the Filter's clear button").click();
    h.run_steps(4);
    let click = |h: &mut Harness<'_, Moonglow>, label: &str, m: egui::Modifiers| {
        let at = h.get_by_label(label).rect().center();
        h.hover_at(at);
        h.event(egui::Event::ModifiersChanged(m));
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: m,
            });
        }
        h.run_steps(2);
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(20);
    };
    click(&mut h, "estate", egui::Modifiers::NONE);
    click(&mut h, "hodge_estate", egui::Modifiers::SHIFT);
    click(&mut h, "m1q3b", egui::Modifiers::COMMAND);
    shoot(&mut h, &dir, "w3-tree-several-chosen");
    h.get_by_label("m1q3b").click_secondary();
    h.run_steps(3);
    shoot(&mut h, &dir, "w3-tree-several-menu");
}

/// The names typed for new resources in a project with a builder code
/// (`ab`): New Script… as it opened before (empty) and as it opens now
/// (with the code), and the Store Wizard's Name page with a ResRef typed
/// without the code, and the button that puts it in.
#[test]
#[ignore]
fn names_with_the_builder_code() {
    use egui_kittest::kittest::Queryable;
    use mg_module::{Module, ModuleLocation};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let out = mg_testkit::output_dir("screens");
    let dir = mg_testkit::scratch_dir("screens-builder-code").join("project");
    let mut m = Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    mg_module::project_file::ProjectFile::set_builder_code(&dir, Some("ab")).unwrap();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&dir);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().new_script = Some(String::new());
    shoot(&mut h, &out, "new-script-name-before");
    h.state_mut().new_script = None;
    h.run();
    h.get_by_label("Tools").click();
    h.run();
    h.get_by_label_contains("New Script…").click();
    shoot(&mut h, &out, "new-script-name-builder-code");
    h.state_mut().new_script = None;
    h.state_mut().blueprint_wizard = Some(mg_ui::blueprint_wizard::BlueprintWizard::new(
        mg_module::palette::BlueprintKind::Store,
    ));
    h.run();
    h.get_by_label("Merchants").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.state_mut().blueprint_wizard.as_mut().unwrap().resref = "shop".into();
    shoot(&mut h, &out, "store-wizard-resref-without-code");
}

/// A copy of the game's appearance.2da in the 2DA editor: its rows 562 to
/// 568 keep the game's footstep 30 (of 18), not outlined; row 0 typed so,
/// outlined and said.
#[test]
#[ignore]
fn table_game_s_own_wrong_cells() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let out = mg_testkit::output_dir("screens");
    let dir = mg_testkit::scratch_dir("screens-table-game-s-own");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let key = ResKey::parse("appearance", ResType::TWODA).unwrap();
    let file = dir.join("appearance.2da");
    std::fs::write(&file, game.resman.get(&key).unwrap()).unwrap();
    let footsteps = game.table("appearance").unwrap().column("FOOTSTEPTYPE").unwrap();
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    let go = |h: &mut Harness<'_, Moonglow>, row: usize| {
        h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(row, footsteps);
    };
    go(&mut h, 562);
    shoot(&mut h, &out, "table-appearance-game-s-footsteps");
    {
        let d = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        d.edit.set(0, footsteps, Some("30".into())).unwrap();
    }
    h.get_by_label("Add Row").click();
    h.run_steps(3);
    go(&mut h, 0);
    shoot(&mut h, &out, "table-appearance-typed-footstep");
}

/// Open Script with scripts selected by Ctrl+A, and a script's right-click
/// menu (`open-script-selected`, `open-script-menu`).
#[test]
#[ignore]
fn open_script_menu() {
    mg_testkit::gpu::hold();
    let out = mg_testkit::output_dir("screens");
    let dir = mg_testkit::scratch_dir("screens-open-script-menu");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    for name in ["hello", "gate_open", "gate_close", "guard_spawn", "inc_guard"] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    let path = dir.join("scripts.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.settings.tree_stays = true;
    app.open_module(&path);
    app.open_palette = false;
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(hello)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("Open…").click();
    h.run_steps(3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    shoot(&mut h, &out, "open-script-selected");
    h.get_by_label("gate_open").click();
    h.run_steps(30);
    h.get_by_label("guard_spawn").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(30);
    h.get_by_label("guard_spawn").click_secondary();
    shoot(&mut h, &out, "open-script-menu");
    eprintln!("{}", out.display());
}

/// A script's lines selected and the editor's right-click menu over them:
/// the selection kept for Convert Indentation of those lines
/// (`script-selection-menu`).
#[test]
#[ignore]
fn script_selection_menu() {
    mg_testkit::gpu::hold();
    let out = mg_testkit::output_dir("screens");
    let dir = mg_testkit::scratch_dir("screens-script-selection-menu");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let text = "void main()\n{\n  int n;\n  if (n)\n    n++;\n}\n";
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, text.as_bytes().to_vec());
    let path = dir.join("script.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.settings.tree_stays = true;
    app.open_module(&path);
    app.open_palette = false;
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Script(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 600.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let id = egui::Id::new(("script", key));
    let mut state = egui::text_edit::TextEditState::load(&h.ctx, id).unwrap_or_default();
    let (from, to) = (text.find("  if").unwrap(), text.find("    n++").unwrap() + 2);
    let range =
        egui::text::CCursorRange::two(egui::text::CCursor::new(from), egui::text::CCursor::new(to));
    state.cursor.set_char_range(Some(range));
    state.store(&h.ctx, id);
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.run_steps(2);
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("void main")))
        .expect("the script's text")
        .click_secondary();
    shoot(&mut h, &out, "script-selection-menu");
    eprintln!("{}", out.display());
}

/// The journal editor's lower half (the selection's fields): a category
/// chosen, the pane dragged taller, then Root chosen, which has one line
/// to show. The pane stays as tall as it was dragged to; it went back to
/// that one line (as the conversation editor's did, fixed before).
#[test]
#[ignore]
fn journal_lower_half_kept() {
    mg_testkit::gpu::hold();
    let out = mg_testkit::output_dir("screens");
    let dir = mg_testkit::scratch_dir("screens-journal-lower-half");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let path = dir.join("j.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Journal));
    h.run_steps(4);
    h.get_by_label("Add").click();
    h.run_steps(4);
    shoot(&mut h, &out, "journal-category");
    let panel = egui::Id::new("journal-fields");
    let r = egui::containers::panel::PanelState::load(&h.ctx, panel).unwrap().outer_rect;
    let from = egui::pos2(r.center().x, r.top() + 2.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.hover_at(from);
    h.run_steps(1);
    h.event(button(from, true));
    h.run_steps(1);
    for k in 1..=6 {
        h.hover_at(from - egui::vec2(0.0, 25.0 * k as f32));
        h.run_steps(1);
    }
    h.event(button(from - egui::vec2(0.0, 150.0), false));
    shoot(&mut h, &out, "journal-dragged-taller");
    h.get_by_label("Root").click();
    shoot(&mut h, &out, "journal-root-chosen");
}

/// Setup Store's Choose Store page in a project whose `[store]
/// blueprint` is the last store of the game's list: the list opens
/// scrolled to it.
#[test]
#[ignore]
fn setup_store_project_blueprint_far_down() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::output_dir("screens");
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut stores: Vec<(String, mg_core::ResRef)> = game
        .resman
        .list(ResType::UTM)
        .into_iter()
        .map(|r| {
            let data = game.resman.get(&ResKey::new(r, ResType::UTM)).unwrap();
            let g = mg_gff::Gff::read(&data).unwrap();
            let name = g.root.locstring("LocName").and_then(|l| game.locstring(l));
            (name.unwrap_or_default(), r)
        })
        .collect();
    stores.sort();
    let last = stores.last().unwrap().1;
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Shop", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: mg_core::ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bandit = game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap();
    let none = |_: mg_core::ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let bandit = mg_gff::Gff::read(&bandit).unwrap().root;
    let keeper = instance(&placing, ResType::UTC, &bandit, at, &[]);
    let key = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&key).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![keeper.unwrap()]));
    m.set_gff(key, &g).unwrap();
    let project = mg_testkit::scratch_dir("screens-setup-store").join("shop");
    m.save_as(&mg_module::ModuleLocation::Folder(project.clone())).unwrap();
    std::fs::write(project.join("moonglow.toml"), format!("[store]\nblueprint = \"{last}\"\n"))
        .unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&project);
    app.open_palette = false;
    mg_ui::store_wizard::open(&mut app, area, "Creature List", 0);
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Next >").click();
    // (The list's scroll is animated: run until it is done.)
    h.run();
    shoot(&mut h, &dir, "setup-store-project-blueprint");
}
