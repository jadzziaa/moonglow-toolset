//! What the subjects share: the sample module, the harnesses and the
//! input helpers.

pub(crate) use egui_kittest::Harness;
pub(crate) use egui_kittest::kittest::{NodeT, Queryable};
pub(crate) use mg_core::{Gender, Language, LocString, ResRef, ResType};
pub(crate) use mg_gff::Gff;
pub(crate) use mg_module::{Module, ModuleLocation};
pub(crate) use mg_resman::ResKey;
pub(crate) use mg_schema::{ExoString, StructExt, ifo};
pub(crate) use mg_ui::{Moonglow, NoDialogs, Tab};

pub(crate) fn sample_module(dir: &std::path::Path) -> std::path::PathBuf {
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("SAMPLE"));
    info.root.write(&ifo::MOD_XP_SCALE, 10);
    info.root.write(
        &ifo::MOD_DESCRIPTION,
        LocString::from_text(Language::ENGLISH, Gender::Male, "Line one\r\nLine two"),
    );
    let mut a = ifo::MOD_AREA_LIST.new_item();
    a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str("start").unwrap());
    info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
    m.set_info(&info).unwrap();
    m.set(ResKey::parse("start", ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    let path = dir.join("sample.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

pub(crate) fn app_with(open: Vec<std::path::PathBuf>) -> Moonglow {
    Moonglow::new(None, Box::new(NoDialogs { open, ..Default::default() }))
}

/// The application on the game installed at `root`, in English, with
/// `user` as the game's user folder (`None`: none), its dialogs answered
/// by `dialogs`.
pub(crate) fn game_app_with(
    root: impl Into<std::path::PathBuf>,
    user: Option<std::path::PathBuf>,
    dialogs: NoDialogs,
) -> Moonglow {
    Moonglow::new(Some(mg_resman::GameInstall::new(root, user, "en")), Box::new(dialogs))
}

/// [`game_app_with`] answering no dialogs.
pub(crate) fn game_app(
    root: impl Into<std::path::PathBuf>,
    user: Option<std::path::PathBuf>,
) -> Moonglow {
    game_app_with(root, user, NoDialogs::default())
}

/// `app`'s window in a harness `width` by `height` points, drawn (by
/// `render`, `snapshot`) on a GPU device of kittest's own: a test that
/// draws takes the GPU first (`mg_testkit::gpu::hold`).
pub(crate) fn harness(app: Moonglow, width: f32, height: f32) -> Harness<'static, Moonglow> {
    Harness::builder()
        .with_size(egui::vec2(width, height))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app)
}

/// The GPU for a test that draws (the area view, a model view, a
/// preview): takes it for the rest of the test (`mg_testkit::gpu::hold`,
/// a few devices open at a time in this binary) and makes a render state
/// for the application (`Moonglow::set_render_state`) and
/// [`gpu_harness`]; `None`, said, without a GPU adapter.
pub(crate) fn gpu() -> Option<egui_wgpu::RenderState> {
    mg_testkit::gpu::hold();
    // (Asked once: a device opened to ask it was a second device for
    // every test that draws.)
    static ADAPTER: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ADAPTER.get_or_init(|| mg_render::Gpu::headless().is_some()) {
        eprintln!("skipped: no GPU adapter");
        return None;
    }
    Some(egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    ))
}

/// [`harness`] drawn on the GPU's render state `rs` (from [`gpu`], given
/// to `app` too). Takes the GPU as well, so that no harness draws without
/// it.
pub(crate) fn gpu_harness(
    app: Moonglow,
    rs: egui_wgpu::RenderState,
    width: f32,
    height: f32,
) -> Harness<'static, Moonglow> {
    mg_testkit::gpu::hold();
    Harness::builder()
        .with_size(egui::vec2(width, height))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app)
}

/// [`gpu_harness`] for an area view: frames a 60th of a second apart, so
/// that clicks can be double clicks.
pub(crate) fn area_gpu_harness(
    app: Moonglow,
    rs: egui_wgpu::RenderState,
    width: f32,
    height: f32,
) -> Harness<'static, Moonglow> {
    mg_testkit::gpu::hold();
    Harness::builder()
        .with_size(egui::vec2(width, height))
        .with_step_dt(1.0 / 60.0)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app)
}

/// What a key pressed with `modifiers` came to in its frame (pressed in
/// one frame, let go in the next): the text copied to the clipboard, and
/// whether the backend was asked for the clipboard's text (a paste).
pub(crate) fn press_now(
    h: &mut Harness<'_, Moonglow>,
    modifiers: egui::Modifiers,
    key: egui::Key,
) -> (Vec<String>, bool) {
    let event =
        |pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
    let input = h.input_mut();
    input.events.push(egui::Event::ModifiersChanged(modifiers));
    input.events.push(event(true));
    h.step();
    let out = h.output();
    let copied = out
        .platform_output
        .commands
        .iter()
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    let paste = out
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|v| v.commands.contains(&egui::ViewportCommand::RequestPaste));
    let input = h.input_mut();
    input.events.push(event(false));
    input.events.push(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.step();
    (copied, paste)
}

pub(crate) fn set_tag(app: &mut Moonglow, tag: &str) {
    let key = ResKey::parse("module", ResType::IFO).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Module tag",
        vec![mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(tag.as_bytes().to_vec())),
        }],
    )));
}

/// The files under `dir`, by their paths.
pub(crate) fn walk(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p.display().to_string());
        }
    }
    out
}

/// A module with an edit copy of a standard blueprint, open in its editor
/// (`None` without the game).
pub(crate) fn blueprint_harness(
    name: &str,
    copy: &str,
    t: ResType,
) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let dir = mg_testkit::scratch_dir(&format!("ui-bp-{copy}"));
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named(name, t).unwrap().into_owned();
    let key = ResKey::parse(copy, t).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
    )));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    // The editor alone on screen (Module Properties, docked when the module
    // opened, and the palette would show beside its window).
    close_tab(&mut app, &Tab::ModuleProperties);
    app.open_palette = false;
    let h = harness(app, 1000.0, 800.0);
    Some((h, key))
}

pub(crate) fn field(h: &mut Harness<'_, Moonglow>, key: &ResKey) -> mg_gff::Struct {
    h.state_mut().ws.as_mut().unwrap().doc(key).unwrap().root.clone()
}

pub(crate) fn filter_type(h: &mut Harness<'_, Moonglow>, text: &str) {
    h.event(egui::Event::Text(text.into()));
    h.run_steps(2);
}

/// The palette's types' icons, as its dropdown shows the one chosen.
pub(crate) const PALETTE_TYPES: [&str; 11] =
    ["🗻", "👤", "🚪", "⚔", "🗡", "⛲", "🔉", "💰", "⚡", "📍", "🗐"];

/// Chooses a type in the palette's header: its icon in the row of them,
/// or, in a pane too narrow for the row, its name in the dropdown.
pub(crate) fn palette_type(h: &mut Harness<'_, Moonglow>, glyph: &str, name: &str) {
    if h.query_by_label(glyph).is_some() {
        h.get_by_label(glyph).click();
        return;
    }
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let is_type = |n: &egui_kittest::Node<'_>| {
        let value = n.accesskit_node().value().unwrap_or_default();
        PALETTE_TYPES.iter().any(|glyph| value.starts_with(glyph))
    };
    h.query_all(combo).find(is_type).expect("the palette's type dropdown").click();
    h.run();
    // (The last of them are below the dropdown's edge until scrolled to.)
    h.get_by_label(&format!("{glyph} {name}")).scroll_to_me();
    h.run();
    h.get_by_label(&format!("{glyph} {name}")).click();
    h.run();
}

/// Types into the text field with placeholder `hint` (a live filter).
pub(crate) fn type_into_hint(h: &mut Harness<'_, Moonglow>, hint: &str, text: &str) {
    // ("Find" is a list's search: the palette's says what it looks in.)
    let palette = "🔍 Search name, tag, resref";
    let has = |h: &Harness<'_, Moonglow>, hint: &str| {
        let by = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some(hint);
        h.query_all(egui_kittest::kittest::by().predicate(by)).next().is_some()
    };
    let hint = if hint == "Find" && !has(h, hint) { palette } else { hint };
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some(hint);
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    h.run();
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text(text);
    h.run();
}

pub(crate) const ANIMATIONS: (&str, &str, &str, &str) =
    ("AnimationReplace", "AnimationReplace", "OldAnimation", "NewAnimation");

/// Frames until the UI stops asking for more (a gallery makes its pictures
/// a few milliseconds' worth a frame).
pub(crate) fn settle(h: &mut Harness<'_, Moonglow>) {
    for _ in 0..100 {
        if h.try_run().is_ok() {
            return;
        }
    }
    panic!("the UI keeps repainting");
}

/// The sample module with the game's data (`None` without the game).
pub(crate) fn game_harness(name: &str) -> Option<Harness<'static, Moonglow>> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let dir = mg_testkit::scratch_dir(&format!("ui-wizard-{name}"));
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    Some(harness(app, 1000.0, 800.0))
}

pub(crate) fn module_gff(h: &mut Harness<'_, Moonglow>, name: &str, t: ResType) -> Option<Gff> {
    let key = ResKey::parse(name, t).unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    ws.module.gff(&key).map(|g| g.unwrap())
}

/// A module with a 4 by 4 rural area holding two waypoints, at (20, 20)
/// and (30, 20), with the game's data and a GPU (`None` without either).
pub(crate) fn area_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    area_harness_on(name, "ttr01", None)
}

/// [`area_harness`] on another tileset, with tile 5 (at column 1, row 1)
/// replaced by tile `tile` of that tileset when given.
pub(crate) fn area_harness_on(
    name: &str,
    tileset: &str,
    tile: Option<i32>,
) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    // (A look test on a builder's content: an area with room for its
    // groups.)
    let side = std::env::var("MG_TEST_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
    area_harness_sized(name, tileset, tile, side)
}

/// [`area_harness_on`] with an area `side` tiles each way.
pub(crate) fn area_harness_sized(
    name: &str,
    tileset: &str,
    tile: Option<i32>,
    side: u32,
) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    // (A look test on a builder's content: a user folder with it in its
    // override.)
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    area_harness_with(name, tileset, tile, side, user, &|_, _, _| {})
}

/// [`area_harness_sized`] with the game's user folder `user` (an
/// authored model in its override), and the module changed by `prepare`
/// (given the game and the area) before it is opened.
pub(crate) fn area_harness_with(
    name: &str,
    tileset: &str,
    tile: Option<i32>,
    side: u32,
    user: Option<std::path::PathBuf>,
    prepare: &dyn Fn(&mut mg_module::Module, &mg_rules::GameData, ResRef),
) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let root = mg_testkit::nwn_root()?;
    let rs = gpu()?;
    let install = mg_resman::GameInstall::new(&root, user, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Area View", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str(tileset).unwrap(),
        width: side,
        height: side,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    if let Some(id) = tile {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        let t = &mut are.root.list_mut("Tile_List").unwrap()[5];
        t.set("Tile_ID", mg_gff::Value::Int(id));
        t.set("Tile_Orientation", mg_gff::Value::Int(0));
        t.set("Tile_Height", mg_gff::Value::Int(0));
        m.set_gff(key, &are).unwrap();
    }
    let bp = game.resman.get(&ResKey::parse("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
    let bp = Gff::read(&bp).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let waypoints = [20.0, 30.0]
        .map(|x| {
            let at = Placement { position: [x, 20.0, 0.0], rotation: 0.3 };
            instance(&placing, ResType::UTW, &bp.root, at, &[]).unwrap()
        })
        .to_vec();
    git.root.set("WaypointList", mg_gff::Value::List(waypoints));
    m.set_gff(git_key, &git).unwrap();
    prepare(&mut m, &game, area);
    let path = mg_testkit::scratch_dir(&format!("ui-area-{name}")).join("area.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    // (Moonglow's own shader, chosen: the area view tests' pictures don't
    // change as the game's programs are made.)
    app.settings.game_shaders = Some(false);
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    let mut h = area_gpu_harness(app, rs, 1100.0, 800.0);
    h.run_steps(3);
    Some((h, area))
}

/// A waypoint's position in the workspace's GIT.
pub(crate) fn waypoint(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    index: usize,
) -> Option<(f32, f32, u32)> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    let w = git.root.list("WaypointList")?.get(index)?;
    Some((w.float("XPosition")?, w.float("YPosition")?, w.float("XOrientation")?.to_bits()))
}

pub(crate) fn screen(h: &Harness<'_, Moonglow>, area: ResRef, p: glam::Vec3) -> egui::Pos2 {
    h.state().area_views[&area].screen_pos(p).expect("in view")
}

pub(crate) fn press(
    h: &Harness<'_, Moonglow>,
    pos: egui::Pos2,
    pressed: bool,
    modifiers: egui::Modifiers,
) {
    let button = egui::PointerButton::Primary;
    h.event_modifiers(egui::Event::PointerButton { pos, button, pressed, modifiers }, modifiers);
}

/// A `true` program to stand in for an external editor: the first on
/// `PATH` (NixOS has no `/bin/true`).
pub(crate) fn true_program() -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join("true"))
        .find(|p| p.is_file())
        .map(|p| p.display().to_string())
}

/// Widgets drawn over each other: pairs of named widgets in one layer (a
/// window, the main panels) whose visible rectangles (clipped by their scroll
/// areas, as egui records them for the frame) overlap by more than a quarter
/// of the smaller, as when a row of widgets is squeezed into a narrow
/// column. Their names, for the failure message.
pub(crate) fn overlapping(h: &Harness<'_, Moonglow>) -> Vec<String> {
    use egui_kittest::kittest::{AccessKitNode, NodeT};
    use std::collections::HashMap;
    // The named widgets (role and name), by egui id.
    fn names(n: AccessKitNode<'_>, out: &mut HashMap<u64, String>) {
        let role = n.role();
        let name = n.label().or_else(|| n.value()).unwrap_or_default();
        let part = matches!(
            role,
            egui::accesskit::Role::Unknown | egui::accesskit::Role::GenericContainer
        );
        // Widgets, not the windows and groups that hold them.
        let leaf = n.children().next().is_none();
        if leaf && !name.trim().is_empty() && !part {
            out.insert(n.locate().0.0, format!("{role:?} {name:?}"));
        }
        for k in n.children() {
            names(k, out);
        }
    }
    let mut named = HashMap::new();
    names(h.root().accesskit_node(), &mut named);
    let mut out = Vec::new();
    h.ctx.viewport(|v| {
        for (_, widgets) in v.prev_pass.widgets.layers() {
            let shown: Vec<(egui::Rect, &String)> = widgets
                .iter()
                .filter(|w| w.interact_rect.area() > 1.0)
                .filter_map(|w| Some((w.interact_rect, named.get(&w.id.value())?)))
                .collect();
            for (i, (a, name_a)) in shown.iter().enumerate() {
                for (b, name_b) in &shown[i + 1..] {
                    let both = a.intersect(*b);
                    let smaller = a.area().min(b.area());
                    if both.is_positive() && both.area() > 0.25 * smaller {
                        out.push(format!("{name_a} over {name_b}"));
                    }
                }
            }
        }
    });
    out
}

/// Closes a tab, wherever it is.
pub(crate) fn close_tab(app: &mut Moonglow, tab: &Tab) {
    if let Some(path) = app.dock.find_tab(tab) {
        app.dock.remove_tab(path);
    }
}

/// Closes the editors' windows (the tabs not in the main dock), back to the
/// area.
pub(crate) fn close_windows(h: &mut Harness<'_, Moonglow>) {
    let app = h.state_mut();
    let open: Vec<Tab> = app
        .dock
        .iter_all_tabs()
        .filter(|(p, _)| !p.surface.is_main())
        .map(|(_, t)| t.clone())
        .collect();
    for t in open {
        close_tab(app, &t);
    }
}

/// The plugin host's fixture plugins, as an installed plugins folder.
pub(crate) fn plugin_fixtures() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mg-plugin/tests/fixtures")
}

/// The sample module with a guard whose tag is in lower case, and the
/// fixture plugins installed (none enabled).
pub(crate) fn plugin_harness(
    name: &str,
    game: bool,
) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut app = if game {
        let root = mg_testkit::nwn_root()?;
        game_app(&root, None)
    } else {
        app_with(Vec::new())
    };
    app.open_module(&path);
    app.open_palette = false;
    let key = ResKey::parse("guard", ResType::UTC).unwrap();
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", mg_gff::Value::String(b"gate_guard".to_vec()));
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A guard",
        vec![mg_edit::Edit::SetResource { key, data: Some(guard.to_bytes().unwrap()) }],
    )));
    app.plugin_dir = Some(plugin_fixtures());
    app.load_plugins();
    let h = harness(app, 1200.0, 900.0);
    Some((h, key))
}

pub(crate) fn tag_of(h: &mut Harness<'_, Moonglow>, key: &ResKey) -> String {
    String::from_utf8(field(h, key).string("Tag").unwrap().to_vec()).unwrap()
}

/// Takes `tab` out of the window it opened in (beside the creature's) into
/// a window of its own, at `at`, `size` big: both in sight.
pub(crate) fn own_window(
    h: &mut Harness<'_, Moonglow>,
    tab: &Tab,
    at: egui::Pos2,
    size: egui::Vec2,
) {
    let dock = &mut h.state_mut().dock;
    let found = dock.find_tab(tab).unwrap();
    let tab = dock.remove_tab(found).unwrap();
    let surface = dock.add_window(vec![tab]);
    dock.get_window_state_mut(surface).unwrap().set_position(at).set_size(size);
    h.run_steps(4);
}

/// A click of the pointer's first button at `pos`.
pub(crate) fn click_pos(h: &Harness<'_, Moonglow>, pos: egui::Pos2) {
    h.hover_at(pos);
    press(h, pos, true, egui::Modifiers::NONE);
    press(h, pos, false, egui::Modifiers::NONE);
}

pub(crate) const SMALL_TABLE: &[u8] =
    b"2DA V2.0\r\n\r\n   Label   Cost\r\n0  Bread   1\r\n1  Cheese  2\r\n";

pub(crate) fn table_of(h: &Harness<'_, Moonglow>, id: u32) -> mg_2da::TwoDa {
    h.state().tables.iter().find(|d| d.id == id).unwrap().edit.table().clone()
}

/// A drag from `from` to `to`, a little at a time.
pub(crate) fn drag(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(1);
    for step in 1..=8 {
        h.hover_at(from + (to - from) * (step as f32 / 8.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(3);
}

/// The items of the open right-click menu whose last item is `last`, top
/// to bottom: the buttons of the popup `last` is in (the menu, drawn
/// above the rest, is the last listed).
pub(crate) fn menu_items(h: &Harness<'_, Moonglow>, last: &str) -> Vec<String> {
    use egui_kittest::kittest::AccessKitNode;
    let item = h.get_all_by_label(last).last().expect("the menu's item");
    // The popup: the nearest of its containers with a place on the screen.
    let mut menu = item.accesskit_node().parent();
    while let Some(n) = menu.filter(|n| n.bounding_box().is_none()) {
        menu = n.parent();
    }
    fn buttons(n: AccessKitNode<'_>, into: &mut Vec<(f64, String)>) {
        if n.role() == egui::accesskit::Role::Button {
            let top = n.bounding_box().map_or(0.0, |b| b.y0);
            into.push((top, n.label().unwrap_or_default()));
        }
        for child in n.children() {
            buttons(child, into);
        }
    }
    let mut items = Vec::new();
    buttons(menu.expect("in a popup"), &mut items);
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    items.into_iter().map(|(_, label)| label).collect()
}

/// A module folder (a project: a `.mod` has none) of the sample module's
/// resources, with these files beside it.
pub(crate) fn project_folder(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let scratch = mg_testkit::scratch_dir(name);
    let m = Module::open(&sample_module(&scratch)).unwrap();
    let dir = scratch.join("project");
    let mut m = m;
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

/// A claims table of the sample's area `start`: `start` held by `cd` and
/// merged as it is (`merged`), or held by nobody.
pub(crate) fn claims_table(holder: &str, status: &str) -> String {
    format!(
        "2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n\
         0   start            {holder:<8} {status:<8} 2026-10-20  ****\n"
    )
}

/// The sample module as a project folder whose moonglow.toml names a
/// claims table, the builder `ab`; opened on its Properties.
pub(crate) fn claimed_project(
    name: &str,
    rules: &str,
    table: &str,
) -> (std::path::PathBuf, Moonglow) {
    let dir = project_folder(name, &[("moonglow.toml", rules), ("claims.2da", table)]);
    mg_module::project_file::ProjectFile::set_builder_code(&dir, Some("ab")).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    (dir, app)
}

/// An edit of the area `start`'s name, as Area Properties makes one.
pub(crate) fn rename_start(name: &str) -> mg_ui::Action {
    let key = ResKey::parse("start", ResType::ARE).unwrap();
    mg_ui::Action::Apply(mg_edit::Command::new(
        "Name",
        vec![mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "Name".into(),
            value: Some(mg_gff::Value::LocString(LocString::from_text(
                Language::ENGLISH,
                Gender::Male,
                name,
            ))),
        }],
    ))
}

/// An area's tag, as the module has it now.
pub(crate) fn area_tag(h: &mut Harness<'_, Moonglow>, area: &str) -> String {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let doc = ws.doc(&ResKey::parse(area, ResType::ARE).unwrap()).unwrap();
    String::from_utf8_lossy(doc.root.string("Tag").unwrap_or_default()).into_owned()
}
