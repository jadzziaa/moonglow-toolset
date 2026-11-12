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
    let h = Harness::builder()
        .with_size(egui::vec2(width, height))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    // (Which icons are drawn where, for a test to ask: [`icon_drawn`].)
    mg_uikit::record_icons(&h.ctx);
    h
}

/// The game's data at `root`, in English, with no user folder: read
/// before an application is (to make a module for it).
pub(crate) fn game_data(root: &std::path::Path) -> mg_rules::GameData {
    game_data_in(root, None)
}

/// [`game_data`] with `user` as the game's user folder (its haks and
/// override read as the application given it reads them).
pub(crate) fn game_data_in(
    root: &std::path::Path,
    user: Option<std::path::PathBuf>,
) -> mg_rules::GameData {
    mg_rules::GameData::open(&mg_resman::GameInstall::new(root, user, "en")).unwrap()
}

/// [`harness`] at kittest's own size, 800 by 600 points: the window as
/// most of its tests draw it.
pub(crate) fn window_harness(app: Moonglow) -> Harness<'static, Moonglow> {
    harness(app, 800.0, 600.0)
}

/// A scratch user folder for the game: `dir`'s `user`, with `subfolders`
/// (`hak`, `override`, `tlk`...) made in it.
pub(crate) fn user_folder(dir: &std::path::Path, subfolders: &[&str]) -> std::path::PathBuf {
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    for sub in subfolders {
        std::fs::create_dir_all(user.join(sub)).unwrap();
    }
    user
}

/// The GPU for a test that draws (the area view, a model view, a
/// preview): takes it for the rest of the test (`mg_testkit::gpu::hold`,
/// a few devices open at a time in this binary) and makes a render state
/// for the application (`Moonglow::set_render_state`) and
/// [`gpu_harness`]; `None`, said, without a GPU adapter.
pub(crate) fn gpu() -> Option<egui_wgpu::RenderState> {
    mg_testkit::gpu::hold();
    if !adapter() {
        eprintln!("skipped: no GPU adapter");
        return None;
    }
    Some(egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    ))
}

/// Whether this machine has a GPU adapter (CI's Linux runner has none):
/// a test whose pictures are only to look at takes them when it has.
/// (Asked once: a device opened to ask it was a second device for every
/// test that draws.)
pub(crate) fn adapter() -> bool {
    static ADAPTER: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ADAPTER.get_or_init(|| {
        mg_testkit::gpu::hold();
        mg_render::Gpu::headless().is_some()
    })
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
    let h = Harness::builder()
        .with_size(egui::vec2(width, height))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    mg_uikit::record_icons(&h.ctx);
    h
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
    let h = Harness::builder()
        .with_size(egui::vec2(width, height))
        .with_step_dt(1.0 / 60.0)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    mg_uikit::record_icons(&h.ctx);
    h
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
    blueprint_harness_with(name, copy, t, None)
}

/// [`blueprint_harness`] with the game's user folder `user` (content in
/// its override).
pub(crate) fn blueprint_harness_with(
    name: &str,
    copy: &str,
    t: ResType,
    user: Option<std::path::PathBuf>,
) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let dir = mg_testkit::scratch_dir(&format!("ui-bp-{copy}"));
    let path = sample_module(&dir);
    let mut app = game_app(&root, user);
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

/// The palette's types, by the names its row of icons and its dropdown
/// give them.
pub(crate) const PALETTE_TYPES: [&str; 13] = [
    "Tiles",
    "Creatures",
    "Doors",
    "Encounters",
    "Items",
    "Placeables",
    "Sounds",
    "Stores",
    "Triggers",
    "Waypoints",
    "Prefabs",
    "Lights",
    "Start Location",
];

/// Chooses a type in the palette's header by its name: its icon in the
/// row of them (a group of its own, "Palette types": the area toolbar
/// names the kinds too), or, in a pane too narrow for the row, its name in
/// the dropdown.
pub(crate) fn palette_type(h: &mut Harness<'_, Moonglow>, name: &str) {
    if let Some(row) = h.query_by_label("Palette types") {
        row.get_by_label(name).click();
        return;
    }
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let is_type = |n: &egui_kittest::Node<'_>| {
        let value = n.accesskit_node().value().unwrap_or_default();
        PALETTE_TYPES.contains(&value.as_str())
    };
    h.query_all(combo).find(is_type).expect("the palette's type dropdown").click();
    settle(h);
    // (The open list, named as the row is. The last of them are below
    // the dropdown's edge until scrolled to.)
    h.get_by_label("Palette types").get_by_label(name).scroll_to_me();
    settle(h);
    h.get_by_label("Palette types").get_by_label(name).click();
    settle(h);
}

/// The Top Bar's button `name` (found within it: the editors have a
/// Save, an Undo, a Compile of their own).
#[track_caller]
pub(crate) fn tool<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    h.get_by_label("Top Bar").get_by_label(name)
}

/// Chooses `row` in the Top Bar's menu `menu` ("View", "Go to Start
/// Location"): the menu opened by its title, the row clicked (a button or
/// a ticked row, named by its words, or by its words and its keys: of
/// those that begin so, the shortest; not a bar's control of that name,
/// the Log's switch say).
#[track_caller]
pub(crate) fn menu_command(h: &mut Harness<'_, Moonglow>, menu: &str, row: &str) {
    h.get_by_label("Top Bar").get_by_label(menu).click();
    settle(h);
    let rect = menu_row(h, row).expect("the menu's row").1;
    click_pos(h, rect.center());
    settle(h);
}

/// The row `row` of the menu that is open ([`menu_command`]'s): its
/// label and where it is.
pub(crate) fn menu_row(h: &Harness<'_, Moonglow>, row: &str) -> Option<(String, egui::Rect)> {
    use egui::accesskit::Role;
    let keyed = format!("{row} ");
    let is_row = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        matches!(n.role(), Role::Button | Role::CheckBox)
            && n.label().is_some_and(|l| l == row || l.starts_with(&keyed))
    };
    let bars = ["Top Bar", "Status Bar", "Left Dock", "Bottom Dock"];
    h.query_all_by(is_row)
        .filter(|n| !bars.iter().any(|b| in_group(n, b)))
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
        .min_by_key(|(l, _)| l.len())
}

/// The Module panel's header button `name` (today's toolbar's Properties
/// and New Area), found within the Left Dock's tab bar.
#[track_caller]
pub(crate) fn module_tool<'t>(
    h: &'t Harness<'_, Moonglow>,
    name: &'t str,
) -> egui_kittest::Node<'t> {
    h.get_by_label("Left Dock").get_by_label(name)
}

/// The buttons `name` that are not the Top Bar's, the Module panel
/// header's or the area view's (an editor's own Save, Undo or Compile, a
/// dialog's Open, a page's Properties, the palette's Grass): the nodes of
/// that name that have no "Top Bar", "Left Dock" or "Area toolbar" group
/// above them.
pub(crate) fn apart_all<'t>(
    h: &'t Harness<'_, Moonglow>,
    name: &'t str,
) -> impl Iterator<Item = egui_kittest::Node<'t>> {
    h.query_all_by_label(name).filter(|n| !in_toolbar(n))
}

/// Whether `n` is in the group named `group` (`mg_uikit::name_group`).
pub(crate) fn in_group(n: &egui_kittest::Node<'_>, group: &str) -> bool {
    let mut up = n.accesskit_node().parent();
    while let Some(p) = up {
        if p.label().as_deref() == Some(group) {
            return true;
        }
        up = p.parent();
    }
    false
}

/// Whether `n` is on the Top Bar, the Left Dock's tab bar (the Module
/// panel's header buttons) or the area view's toolbar.
pub(crate) fn in_toolbar(n: &egui_kittest::Node<'_>) -> bool {
    let mut up = n.accesskit_node().parent();
    while let Some(p) = up {
        if matches!(p.label().as_deref(), Some("Top Bar" | "Left Dock" | "Area toolbar")) {
            return true;
        }
        up = p.parent();
    }
    false
}

/// The one of them ([`apart_all`]): exactly one, as `get_by_label` asks,
/// or it panics naming how many there are.
#[track_caller]
pub(crate) fn apart<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    let mut found = apart_all(h, name);
    let Some(first) = found.next() else { panic!("no {name} apart from the bars") };
    let more = found.count();
    assert!(more == 0, "{} named {name:?} apart from the toolbars, not one", more + 1);
    first
}

/// The area view's toolbar's button `name` (found within it: the palette
/// names the kinds of objects too). The objects' filters are "Show
/// Creatures"…, "Show Start"; the switch that plays the placed sounds is
/// "Play Sounds".
#[track_caller]
pub(crate) fn area_tool<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    h.get_by_label("Area toolbar").get_by_label(name)
}

/// The tooltip shown with the pointer at `rect`'s middle: the words of
/// what appears there (the nodes that were not there before), or `None`
/// if nothing appears in a second and a half of frames (a tip waits a
/// moment; on a busy machine the same moment is more frames, so the wait
/// is long for an absent tip and short for a present one).
pub(crate) fn tip_at(h: &mut Harness<'_, Moonglow>, rect: egui::Rect) -> Option<String> {
    use std::collections::HashMap;
    fn words(h: &Harness<'_, Moonglow>) -> HashMap<String, usize> {
        let mut out = HashMap::new();
        for n in h.query_all_by(|n| n.bounding_box().is_some()) {
            let a = n.accesskit_node();
            let text = a.label().or(a.value()).unwrap_or_default();
            if !text.trim().is_empty() {
                *out.entry(text).or_default() += 1;
            }
        }
        out
    }
    // (Away first, so that a tip shown before is gone.)
    h.hover_at(egui::pos2(1.0, 1.0));
    h.run_steps(2);
    let before = words(h);
    h.hover_at(rect.center());
    for _ in 0..90 {
        h.run_steps(1);
        let now = words(h);
        let mut new: Vec<String> = (now.into_iter())
            .filter(|(t, n)| *n > before.get(t).copied().unwrap_or(0))
            .map(|(t, _)| t)
            .collect();
        if !new.is_empty() {
            new.sort();
            return Some(new.join("\n"));
        }
    }
    None
}

/// Whether the icon `icon` is drawn inside `rect` in the last frame (the
/// harness records them: `mg_uikit::record_icons`, set up by [`harness`]).
pub(crate) fn icon_drawn(
    h: &Harness<'_, Moonglow>,
    icon: mg_uikit::Icon,
    rect: egui::Rect,
) -> bool {
    mg_uikit::icons_drawn(&h.ctx)
        .iter()
        .any(|(i, r)| *i == icon && rect.expand(1.0).contains_rect(*r))
}

/// The module tree's Filter's accessible name (it says "Filter" while
/// empty, as a dropdown's does).
pub(crate) const TREE_FILTER: &str = "Filter Module Tree";

/// Whether `n` is a dropdown's or a list's Filter field (its placeholder
/// "Filter"), not the module tree's (which says the same, named
/// [`TREE_FILTER`]).
pub(crate) fn list_filter(n: &egui_kittest::kittest::AccessKitNode<'_>) -> bool {
    n.placeholder() == Some("Filter") && n.label().as_deref() != Some(TREE_FILTER)
}

/// Whether `n` is the module tree's Filter field.
pub(crate) fn tree_filter(n: &egui_kittest::kittest::AccessKitNode<'_>) -> bool {
    n.label().as_deref() == Some(TREE_FILTER)
}

/// Types `text` into the module tree's Filter, and waits for what it shows.
pub(crate) fn type_into_tree_filter(h: &mut Harness<'_, Moonglow>, text: &str) {
    h.get(egui_kittest::kittest::by().predicate(tree_filter)).click();
    settle(h);
    h.get(egui_kittest::kittest::by().predicate(tree_filter)).type_text(text);
    settle(h);
}

/// The module tree's row `label` (a resource's, a group's "Areas (1)",
/// Module Properties: not the Module panel header's button of that name).
#[track_caller]
pub(crate) fn tree_row<'t>(h: &'t Harness<'_, Moonglow>, label: &'t str) -> egui_kittest::Node<'t> {
    h.get_by_label("Module Tree Rows").get_by_label(label)
}

/// Types into the text field with placeholder `hint` (a live filter), and
/// waits for what it shows ([`settle`]: a gallery's pictures, made a few
/// milliseconds' worth a frame). A "Filter" is a dropdown's or a list's,
/// not the module tree's ([`type_into_tree_filter`]).
pub(crate) fn type_into_hint(h: &mut Harness<'_, Moonglow>, hint: &str, text: &str) {
    // ("Find" is a list's search: the palette's says what it looks in.
    // A search field's glass is drawn, not in its hint.)
    let palette = "Search name, tag, resref";
    let has = |h: &Harness<'_, Moonglow>, hint: &str| {
        let by = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some(hint);
        h.query_all(egui_kittest::kittest::by().predicate(by)).next().is_some()
    };
    let hint = if !has(h, hint) && hint == "Find" { palette } else { hint };
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.placeholder() == Some(hint) && (hint != "Filter" || list_filter(n))
    };
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    settle(h);
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text(text);
    settle(h);
}

pub(crate) const ANIMATIONS: (&str, &str, &str, &str) =
    ("AnimationReplace", "AnimationReplace", "OldAnimation", "NewAnimation");

/// How long [`settle`] waits for the window to settle: far longer than
/// any of its work takes on a busy machine, short of a test that hangs.
pub(crate) const SETTLE_TIME: std::time::Duration = std::time::Duration::from_secs(30);

/// Frames until the UI stops asking for more and the window's own work is
/// done ([`Moonglow::under_way`]: a gallery makes its pictures a few
/// milliseconds' worth a frame, fewer on a busy machine; a look at the
/// module's files runs on a thread, which takes longer on Windows), with a
/// little time between tries, for at most [`SETTLE_TIME`].
pub(crate) fn settle(h: &mut Harness<'_, Moonglow>) {
    let started = std::time::Instant::now();
    loop {
        let ran = h.try_run();
        let under_way = h.state().under_way();
        if ran.is_ok() && under_way.is_none() {
            return;
        }
        if started.elapsed() > SETTLE_TIME {
            match (ran, under_way) {
                (Err(e), _) => panic!("the UI keeps repainting: {e}"),
                (Ok(_), what) => panic!("the window's work never ended: {}", what.unwrap()),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
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
    let (app, rs, area) = area_app(name, tileset, tile, side, user, prepare)?;
    let mut h = area_gpu_harness(app, rs, 1100.0, 800.0);
    h.run_steps(3);
    Some((h, area))
}

/// What [`area_harness_with`] puts in its harness: the application with
/// the module open and its area's tab asked for, the GPU's render state
/// (given to it) and the area.
pub(crate) fn area_app(
    name: &str,
    tileset: &str,
    tile: Option<i32>,
    side: u32,
    user: Option<std::path::PathBuf>,
    prepare: &dyn Fn(&mut mg_module::Module, &mg_rules::GameData, ResRef),
) -> Option<(Moonglow, egui_wgpu::RenderState, ResRef)> {
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
    Some((app, rs, area))
}

/// [`area_harness`]'s area with a chest at 12, 20, in a project folder
/// whose claims table says `cd` holds the area and whose moonglow.toml
/// keeps others' areas read-only, the builder `ab`: an area read-only
/// here.
pub(crate) fn read_only_area_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let chest = |m: &mut Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::instances::{Placement, Placing, instance};
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [12.0, 20.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("Placeable List", mg_gff::Value::List(vec![item]));
        m.set_gff(git_key, &git).unwrap();
    };
    let (mut app, rs, area) = area_app(name, "ttr01", None, 4, None, &chest)?;
    // The same module as a project folder that names a claims table.
    let mut m = app.ws.as_ref()?.module.clone();
    let dir = mg_testkit::scratch_dir(&format!("ui-area-{name}-project"));
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let rules = "[claims]\nfile = \"claims.2da\"\nothers = \"read-only\"\n";
    std::fs::write(dir.join("moonglow.toml"), rules).unwrap();
    let table = format!(
        "2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n\
         0   {:<16} cd       claimed  2026-10-20  ****\n",
        area.to_string()
    );
    std::fs::write(dir.join("claims.2da"), table).unwrap();
    mg_module::project_file::ProjectFile::set_builder_code(&dir, Some("ab")).unwrap();
    app.open_module(&dir);
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

/// Steps until Q and E's turning in `area` is made a command (when the
/// keys rest): shown at once, it is not one yet.
pub(crate) fn rest_turn_keys(h: &mut Harness<'_, Moonglow>, area: ResRef) {
    for _ in 0..2000 {
        if !h.state().area_views[&area].key_turning() {
            h.run_steps(2);
            return;
        }
        h.run_steps(1);
    }
    panic!("Q and E's turning was never made a command");
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

/// The text field holding `value`, of a window that asks for a name.
pub(crate) fn typed_name<'h>(
    h: &'h Harness<'_, Moonglow>,
    value: &'h str,
) -> egui_kittest::Node<'h> {
    let field = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some(value)
    };
    h.get(egui_kittest::kittest::by().predicate(field))
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

/// Clicks a tab's close button as the pointer does (pressed, then let go a frame
/// on). (egui_dock's close button has no name: it is the square at the
/// right end of its tab, the tabs being what can be clicked and dragged
/// in their pane's tab bar, in their order.)
pub(crate) fn click_tab_close(h: &mut Harness<'_, Moonglow>, tab: &Tab) {
    let (band, index) = h
        .state()
        .dock
        .iter_leaves()
        .find_map(|(_, l)| {
            let index = l.tabs.iter().position(|t| t == tab)?;
            let band =
                egui::Rect::from_x_y_ranges(l.rect.x_range(), l.rect.top()..=l.viewport.top());
            Some((band, index))
        })
        .expect("the tab is open");
    let (mut tabs, mut squares) = (Vec::new(), Vec::new());
    h.ctx.viewport(|v| {
        for (_, widgets) in v.prev_pass.widgets.layers() {
            for w in widgets.iter().filter(|w| band.contains_rect(w.interact_rect)) {
                let r = w.interact_rect;
                if w.sense.senses_click() && w.sense.senses_drag() {
                    tabs.push(r);
                } else if w.sense.senses_click() && (r.width() - r.height()).abs() < 0.5 {
                    squares.push(r);
                }
            }
        }
    });
    tabs.sort_by(|a, b| a.left().total_cmp(&b.left()));
    let own = tabs.get(index).expect("its tab");
    let close = squares.iter().find(|r| own.contains_rect(**r) && r.right() >= own.right() - 0.5);
    let at = close.expect("its close button").center();
    h.hover_at(at);
    h.run_steps(1);
    press(h, at, true, egui::Modifiers::NONE);
    h.run_steps(1);
    press(h, at, false, egui::Modifiers::NONE);
    h.run_steps(1);
}

/// Clicks the close button of the window `tab` is in (egui_dock's, at
/// the right end of its tab bar, unnamed: what can be clicked, not
/// dragged, at the bar's right edge; Maximize is before it).
pub(crate) fn click_window_close(h: &mut Harness<'_, Moonglow>, tab: &Tab) {
    let band = h
        .state()
        .dock
        .iter_leaves()
        .find(|(_, l)| l.tabs.contains(tab))
        .map(|(_, l)| {
            egui::Rect::from_x_y_ranges(l.rect.x_range(), l.rect.top()..=l.viewport.top())
        })
        .expect("the tab is open");
    let mut close = None;
    h.ctx.viewport(|v| {
        for (_, widgets) in v.prev_pass.widgets.layers() {
            for w in widgets.iter().filter(|w| band.contains_rect(w.interact_rect)) {
                let r = w.interact_rect;
                let clicks = w.sense.senses_click() && !w.sense.senses_drag();
                if clicks && r.right() >= band.right() - 1.0 && r.width() < 40.0 {
                    close = Some(r);
                }
            }
        }
    });
    let at = close.expect("its close button").center();
    h.hover_at(at);
    h.run_steps(1);
    press(h, at, true, egui::Modifiers::NONE);
    h.run_steps(1);
    press(h, at, false, egui::Modifiers::NONE);
    h.run_steps(1);
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

/// Scrolls the widget labeled `label` into its scroll area's view and
/// waits until it stays still there (egui's scrolling is animated). A
/// node's rect is where it is laid out, in sight or not, and kittest
/// clicks at its center: out of sight, that is whatever is drawn there.
pub(crate) fn bring_in_sight(h: &mut Harness<'_, Moonglow>, label: &str) {
    apart(h, label).scroll_to_me();
    at_rest(h, label, &|h| apart(h, label).rect());
}

/// Frames until the widget whose place `rect` gives (`what` names it)
/// stays still: a scroll asked for begins a frame or two later and is
/// animated, so still for a few frames in a row is at rest.
pub(crate) fn at_rest(
    h: &mut Harness<'_, Moonglow>,
    what: &str,
    rect: &dyn Fn(&Harness<'_, Moonglow>) -> egui::Rect,
) {
    let started = std::time::Instant::now();
    let (mut last, mut still) = (None, 0);
    while still < 4 {
        h.run_steps(1);
        let now = rect(h);
        still = if last == Some(now) { still + 1 } else { 0 };
        assert!(started.elapsed() < SETTLE_TIME, "{what} never came to rest");
        last = Some(now);
    }
}

/// Turns the mouse wheel down over `at`, in a scroll area that lays out
/// only the rows in sight (the module tree), a little at a time, until
/// the row labeled `label` is laid out, then brings it into sight.
pub(crate) fn wheel_to(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, label: &str) {
    wheel_until(h, at, -20.0, label);
}

fn wheel_until(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, dy: f32, label: &str) {
    let started = std::time::Instant::now();
    h.hover_at(at);
    while h.query_by_label(label).is_none() {
        assert!(started.elapsed() < SETTLE_TIME, "{label} never came into sight");
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(2);
    }
    bring_in_sight(h, label);
}

/// A click on the widget labeled `label`, brought into sight first
/// ([`bring_in_sight`]).
pub(crate) fn click_in_sight(h: &mut Harness<'_, Moonglow>, label: &str) {
    bring_in_sight(h, label);
    apart(h, label).click();
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

/// The item `name` of the open right-click menu: a popup is a layer of its
/// own, with the fewest containers above it (the toolbars, an editor and
/// the palette name some of the same words); of those as shallow, the last.
#[track_caller]
pub(crate) fn popup_item<'t>(
    h: &'t Harness<'_, Moonglow>,
    name: &'t str,
) -> egui_kittest::Node<'t> {
    let depth = |n: &egui_kittest::Node<'_>| {
        std::iter::successors(n.accesskit_node().parent(), |p| p.parent()).count()
    };
    let all: Vec<_> = h.get_all_by_label(name).collect();
    all.into_iter().rev().min_by_key(depth).unwrap_or_else(|| panic!("no menu item {name}"))
}

/// Opens the submenu `name` of the right-click menu that is open (the
/// palette's Copy, the area view's Position…): the pointer brought to it
/// and a frame run, as a builder's would be, then a click. (A submenu's
/// button is named by its words; its arrow is drawn.)
pub(crate) fn open_submenu(h: &mut Harness<'_, Moonglow>, name: &str) {
    let item = |h: &Harness<'_, Moonglow>| popup_item(h, name).rect();
    h.hover_at(item(h).center());
    h.run_steps(1);
    click_pos(h, item(h).center());
    h.run_steps(2);
}

/// The items of submenu `name` of the open right-click menu, top to
/// bottom, opened first: `item`, one of them, finds its popup.
pub(crate) fn submenu_items(h: &mut Harness<'_, Moonglow>, name: &str, item: &str) -> Vec<String> {
    open_submenu(h, name);
    menu_items(h, item)
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

/// Sets a number field (a `DragValue`) to `v`, as a screen reader does
/// (accesskit's SetValue): the field takes it as a number typed.
pub(crate) fn set_number(h: &Harness<'_, Moonglow>, node: &egui_kittest::Node<'_>, v: f64) {
    let (target_node, target_tree) = node.accesskit_node().locate();
    h.event(egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::SetValue,
        target_node,
        target_tree,
        data: Some(egui::accesskit::ActionData::NumericValue(v)),
    }));
}

/// The number field (a SpinButton) on the row of the widget `label`
/// names, the `n`th from the left: a list page's rank, level or uses.
pub(crate) fn number_beside<'h>(
    h: &'h Harness<'_, Moonglow>,
    label: &str,
    n: usize,
) -> egui_kittest::Node<'h> {
    let row = h.get_by_label(label).rect();
    let mut on: Vec<_> = h
        .get_all_by_role(egui::accesskit::Role::SpinButton)
        .filter(|s| (s.rect().center().y - row.center().y).abs() < row.height().max(8.0))
        .collect();
    on.sort_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x));
    on.into_iter().nth(n).unwrap_or_else(|| panic!("no number beside {label}"))
}

/// Puts a dynamic chest at 12, 20 in the area of `git`, first of its
/// placeables, tinted white (The Domains of Dread's `tdod_tint`), as one
/// command: for the tests of a tint being picked.
pub(crate) fn place_tinted_chest(app: &mut Moonglow, git: ResKey) {
    use mg_module::instances::{Placement, Placing, instance};
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get(&ResKey::parse("plc_chest1", ResType::UTP).unwrap()).unwrap();
    let chest = Gff::read(&data).unwrap().root;
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let at = Placement { position: [12.0, 20.0, 0.0], rotation: 0.0 };
    let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
    item.set("Static", mg_gff::Value::Byte(0));
    let mut v = mg_gff::Struct::new(0);
    v.set("Name", mg_gff::Value::String(b"tdod_tint".to_vec()));
    v.set("Type", mg_gff::Value::Dword(3));
    v.set("Value", mg_gff::Value::String(b"#ffffff".to_vec()));
    item.set("VarTable", mg_gff::Value::List(vec![v]));
    let edit = mg_edit::Edit::InsertItem {
        key: git,
        path: mg_edit::GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item,
    };
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
}

/// The tint the area view draws the first placeable of `area` with (its
/// whole object's, `tdod_tint`), as the renderer is given it.
pub(crate) fn drawn_tint(h: &Harness<'_, Moonglow>, area: ResRef) -> Option<Vec<f32>> {
    let view = h.state().area_views.get(&area)?;
    let chest = view.model.as_ref()?.object(mg_area::ObjectKind::Placeable, 0)?;
    let values = &chest.preview.as_ref()?.base.material_values;
    values.iter().find(|v| v.material.is_none()).and_then(|v| match &v.value {
        mg_image::mtr::Param::Float(f) => Some(f.clone()),
        mg_image::mtr::Param::Int(_) => None,
    })
}

/// Drags across the color picker's square opened from the swatch at
/// `well`, from `from` to `to` points along it, a frame a step, and lets
/// go: the picker stays open. (The square found by the picker's own
/// parts, its #rrggbb field at its left under it and Paste Color beside
/// that: the popover opens under the swatch, or beside it where there is
/// no room under it.)
pub(crate) fn drag_picker(h: &mut Harness<'_, Moonglow>, well: egui::Rect, from: f32, to: f32) {
    let pastes: Vec<egui::Rect> = h.query_all_by_label("Paste Color").map(|n| n.rect()).collect();
    let hex = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .filter(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with('#')))
        .map(|n| n.rect())
        // (The picker's own, with Paste Color beside it: a variable's
        // value can be a #rrggbb too.)
        .filter(|r| {
            pastes.iter().any(|p| (p.center().y - r.center().y).abs() < 4.0 && p.left() > r.left())
        })
        .min_by(|a, b| {
            let d = |r: &egui::Rect| r.center().distance(well.center());
            d(a).total_cmp(&d(b))
        })
        .expect("the picker's #rrggbb field");
    // (Above the field: the hue strip, then the square's lower part.)
    let point = |x: f32| egui::pos2(hex.left() + 20.0 + x, hex.top() - 100.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(egui::Event::PointerMoved(point(from)));
    h.event(button(point(from), true));
    h.run();
    for i in 1..=10 {
        h.event(egui::Event::PointerMoved(point(from + (to - from) * i as f32 / 10.0)));
        h.run();
    }
    h.event(button(point(to), false));
    h.run_steps(3);
}

/// A click away from everything at `at`, as one closes a popup with.
pub(crate) fn click_away(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.event(egui::Event::PointerMoved(at));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(4);
}

/// Every shape of the frame just drawn, groups opened.
pub(crate) fn drawn(h: &Harness<'_, Moonglow>) -> Vec<egui::Shape> {
    fn add(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| add(s, out)),
            s => out.push(s.clone()),
        }
    }
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|c| add(&c.shape, &mut out));
    out
}
