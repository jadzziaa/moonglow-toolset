//! Screenshots of windows for looking at their layout, rendered with wgpu
//! into `target/test-output/screens/`. Not checks; run by hand:
//! `cargo test -p mg-ui --test screens -- --ignored`.

use std::sync::Arc;

use egui_kittest::Harness;
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
    let dir = mg_testkit::scratch_dir("screens");
    let lists = Arc::new(Lists::load(app.game.as_ref().unwrap()));
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

#[test]
#[ignore]
fn options_window() {
    mg_testkit::gpu::hold();
    use egui_kittest::kittest::Queryable;
    let dir = mg_testkit::scratch_dir("screens-options");
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.script_style.colors[3] = Some([255, 128, 0]);
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
    let dir = mg_testkit::scratch_dir("screens-model");
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
    app.palette.filter = "plc_chest1".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let chest = h.state().game.as_ref().unwrap().string(mg_core::StrRef(5348)).unwrap();
    let at = h.get_all_by_label(&chest).next().unwrap().rect().center();
    h.hover_at(at);
    h.run_steps(30);
    shoot(&mut h, &dir, "palette-hover");
}

#[test]
#[ignore]
fn blueprint_preview() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-model");
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
    let dir = mg_testkit::scratch_dir("screens");
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
    let dir = mg_testkit::scratch_dir("screens");
    for (name, t, pages) in [
        ("nw_waypoint001", ResType::UTW, &["Basic", "Advanced"][..]),
        ("animalcriesday", ResType::UTS, &["Basic", "Positioning", "Advanced"][..]),
        ("x0_trapavg_shuri", ResType::UTT, &["Basic", "Trap", "Advanced", "Visuals"][..]),
        ("nw_mumcleric", ResType::UTC, &["Classes"][..]),
        ("nw_verminbeet", ResType::UTE, &["Basic", "Creature List", "Advanced"][..]),
        ("nw_storebar01", ResType::UTM, &["Basic", "Inventory", "Restrictions"][..]),
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
        let game = app.game.as_ref().unwrap();
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
    mg_ui::tileset_view::open_path(h.state_mut(), path.clone());
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
