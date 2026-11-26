//! How fast the NUI Creator is on a large window (a hundred controls, a
//! long text and a list). Timing only; run with
//! `cargo test --release -p mg-ui --test nui_perf -- --ignored --nocapture`.
//! `FRAME_COUNT=<n>` sets the frames timed, `NUI_PHASE=idle|edit|elsewhere`
//! times one of the large window's three (to sample its stacks);
//! `a_window_of_text` is a window that is mostly text;
//! `three_hundred_controls_and_a_bound_list` the Design page of 304
//! controls and a list bound to 1,000 rows; `the_creator_at_scale` the
//! review of November 2026's findings at their sizes, each against its
//! own rest (`NUI_VIEW=<part of a name>` picks them).

use std::time::{Duration, Instant};

use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Action, Moonglow, NoDialogs, Tab};
use serde_json::{Value, json};
use timing::{BUDGET, UI_LINE, Views};

#[allow(dead_code)]
mod timing;

/// A window of `rows` rows of eight controls, a long text and a list.
fn window(rows: usize, width: f64) -> Value {
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 1100.0, "h": 760.0});
    let kinds = ["label", "button", "check", "textedit", "slider", "combo", "progress", "image"];
    let mut children = Vec::new();
    for r in 0..rows {
        let mut row = mg_nui::template("row");
        let cells: Vec<Value> = kinds
            .iter()
            .enumerate()
            .map(|(c, kind)| {
                let mut v = mg_nui::template(kind);
                v["id"] = format!("c_{r}_{c}").into();
                v["width"] = json!(width);
                match *kind {
                    "label" => v["value"] = format!("Row {r}").into(),
                    "button" | "check" => v["label"] = format!("Control {r}").into(),
                    "image" => v["value"] = "gui_error".into(),
                    _ => {}
                }
                v
            })
            .collect();
        row["children"] = cells.into();
        children.push(row);
    }
    let mut text = mg_nui::template("text");
    text["value"] = "The quick brown fox jumps over the lazy dog. ".repeat(40).into();
    text["height"] = json!(90.0);
    children.push(text);
    let mut list = mg_nui::template("list");
    list["row_count"] = 20.into();
    list["row_template"] = json!([[mg_nui::template("label"), 0.0, true]]);
    children.push(list);
    window["root"]["children"] = children.into();
    window
}

fn report(what: &str, mut frames: Vec<Duration>) {
    frames.sort();
    println!("{what}: median {:?}, worst {:?}", frames[frames.len() / 2], frames.last().unwrap());
}

/// The app with `window` open in the NUI Creator, and a frame of it timed.
fn open(
    root: &std::path::Path,
    name: &str,
    window: &Value,
) -> (Moonglow, ResKey, impl Fn(&mut Moonglow) -> Duration + use<>) {
    open_with(root, name, window, &mg_nui::Settings::default())
}

/// [`open`] with the window's binds' values.
fn open_with(
    root: &std::path::Path,
    name: &str,
    window: &Value,
    settings: &mg_nui::Settings,
) -> (Moonglow, ResKey, impl Fn(&mut Moonglow) -> Duration + use<>) {
    let dir = mg_testkit::scratch_dir(&format!("nui-perf-{name}"));
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let key = ResKey::parse(name, ResType::JUI).unwrap();
    m.set(key, serde_json::to_vec_pretty(window).unwrap());
    m.set(ResKey::parse(name, ResType::TXT).unwrap(), settings.bytes());
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app =
        Moonglow::new(Some(GameInstall::new(root, None, "en")), Box::new(NoDialogs::default()));
    // (Docked, as the boards draw it: the designer's panels in the docks.)
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.open_module(&path);
    app.actions.push(Action::OpenTab(Tab::Nui(key)));

    // Plain egui frames with tessellation, without the accessibility tree
    // (which the desktop app builds only for a screen reader).
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))),
        ..Default::default()
    };
    let frame = move |app: &mut Moonglow| {
        let t = Instant::now();
        let out = ctx.run_ui(input(), |ui| app.ui(ui));
        let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
        t.elapsed()
    };
    (app, key, frame)
}

/// The designer of `key` drawn with its panels docked (M4's U18): the Right
/// Dock showing its Components (over Properties and Bindings), the Left
/// Dock its Module panel over the Outliner's Layers.
fn docked(app: &Moonglow, key: ResKey) {
    assert!(app.nui_chosen(&key).is_some(), "the designer drawn");
    assert_eq!(app.right_dock_shows(), Some(mg_ui::Panel::Components), "its panels docked");
    assert_eq!(app.left_dock_shows(), Some(mg_ui::Panel::Module), "the Left Dock open");
}

/// A window of text: `labels` rows of four labels of a sentence each and a
/// text block of `paragraphs` sentences, some 4,000 characters in all.
fn text_window(labels: usize, paragraphs: usize) -> Value {
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 1100.0, "h": 760.0});
    let mut children = Vec::new();
    for r in 0..labels {
        let mut row = mg_nui::template("row");
        let cells: Vec<Value> = (0..4)
            .map(|c| {
                let mut v = mg_nui::template("label");
                v["value"] = format!("Żółć {r}.{c}: the quick brown fox jumps").into();
                v
            })
            .collect();
        row["children"] = cells.into();
        row["height"] = json!(22.0);
        children.push(row);
    }
    let mut text = mg_nui::template("text");
    text["value"] = "The quick brown fox jumps over the lazy dog.\n".repeat(paragraphs).into();
    text["height"] = json!(300.0);
    children.push(text);
    window["root"]["children"] = children.into();
    window
}

#[test]
#[ignore]
fn a_window_of_text() {
    let root = mg_testkit::corpus!();
    let (mut app, _, frame) = open(&root, "texty", &text_window(16, 40));
    let count = std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(60);
    let opening: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    println!("text opening: {opening:?}");
    docked(&app, ResKey::parse("texty", ResType::JUI).unwrap());
    report("text-heavy idle frame", (0..count).map(|_| frame(&mut app)).collect());
}

/// A chart of a million samples (a log a script would fill): its line
/// drawn by what its pixels show, not a point and a mark a sample (the
/// review's N16).
#[test]
#[ignore]
fn a_chart_of_a_million_samples() {
    let root = mg_testkit::corpus!();
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 1100.0, "h": 760.0});
    let mut chart = mg_nui::template("chart");
    let data: Vec<f64> = (0..1_000_000).map(|i| (f64::from(i) * 0.001).sin()).collect();
    chart["value"][0]["data"] = json!(data);
    chart["height"] = json!(400.0);
    window["root"]["children"] = json!([chart]);
    let (mut app, _, frame) = open(&root, "chart", &window);
    let count = std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(30);
    let opening: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    println!("chart opening: {opening:?}");
    docked(&app, ResKey::parse("chart", ResType::JUI).unwrap());
    report(
        "a chart of 1,000,000 samples, idle frame",
        (0..count).map(|_| frame(&mut app)).collect(),
    );
}

/// The Design page of 300 controls and a list bound to 1,000 rows (the
/// review's N13: the window and its settings copied once a frame).
#[test]
#[ignore]
fn three_hundred_controls_and_a_bound_list() {
    let root = mg_testkit::corpus!();
    let mut window = window(38, 120.0);
    let children = window["root"]["children"].as_array_mut().unwrap();
    let list = children.last_mut().unwrap();
    list["row_count"] = json!({"bind": "names"});
    list["row_template"][0][0]["value"] = json!({"bind": "names"});
    let mut settings = mg_nui::Settings::default();
    let names: Vec<String> = (0..1000).map(|i| format!("Row {i}")).collect();
    settings
        .bindings
        .insert("names".into(), mg_nui::Binding { value: json!(names), ..Default::default() });
    let (mut app, key, frame) = open_with(&root, "many", &window, &settings);
    let count = std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(60);
    let opening: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    println!("300 controls opening: {opening:?}");
    docked(&app, key);
    report(
        "300 controls and a list bound to 1,000 rows, idle frame",
        (0..count).map(|_| frame(&mut app)).collect(),
    );
}

#[test]
#[ignore]
fn a_large_window() {
    let root = mg_testkit::corpus!();
    let (mut app, key, frame) = open(&root, "big", &window(12, 120.0));
    let count = std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(60);
    let phase = std::env::var("NUI_PHASE").ok();
    let timed = |name: &str| phase.as_deref().is_none_or(|p| p == name);
    let opening: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    println!("opening: {opening:?}");
    docked(&app, key);
    if timed("idle") {
        report("idle frame", (0..count).map(|_| frame(&mut app)).collect());
    }

    // A control's width dragged: the window changes with every frame.
    let mut frames = Vec::new();
    for i in 0..if timed("edit") { count } else { 0 } {
        let data = serde_json::to_vec_pretty(&window(12, 120.0 + f64::from(i % 20))).unwrap();
        app.actions.push(Action::Apply(Command::new(
            "Edit NUI big",
            vec![Edit::SetResource { key, data: Some(data) }],
        )));
        frames.push(frame(&mut app));
    }
    if timed("edit") {
        report("frame after an edit of the window", frames);
    }

    // Something else in the module edited (a script typed in beside it).
    let script = ResKey::parse("other", ResType::NSS).unwrap();
    let mut frames = Vec::new();
    for i in 0..if timed("elsewhere") { count } else { 0 } {
        let data = format!("void main() {{ int n = {i}; }}").into_bytes();
        app.actions.push(Action::Apply(Command::new(
            "Edit other",
            vec![Edit::SetResource { key: script, data: Some(data) }],
        )));
        frames.push(frame(&mut app));
    }
    if timed("elsewhere") {
        report("frame after an edit elsewhere in the module", frames);
    }
}

/// The large window with a swap layout whose variant (the large window's
/// rows again) is edited in its slot: idle frames.
#[test]
#[ignore]
fn a_variant_edited() {
    let root = mg_testkit::corpus!();
    let mut w = window(12, 120.0);
    let variant = w["root"].clone();
    w["root"]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"group","id":"content","width":900.0,"height":300.0,
            "children":[{"type":"col","children":[{"type":"label","value":"Initial"}]}]}));
    let (mut app, key, frame) = open(&root, "swapped", &w);
    let mut s = mg_nui::Settings::default();
    s.variants.insert("details".into(), variant);
    s.variant_targets.insert("details".into(), "content".into());
    let txt = ResKey::parse("swapped", ResType::TXT).unwrap();
    app.actions.push(Action::Apply(Command::new(
        "Settings",
        vec![Edit::SetResource { key: txt, data: Some(s.bytes()) }],
    )));
    let count = std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(60);
    let opening: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    app.nui_edit_variant(&key, "details");
    let editing: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    println!("variant opening: {opening:?} {editing:?}");
    assert_eq!(app.nui_chosen(&key).as_deref(), Some("/root"), "the variant edited");
    report("variant edited, idle frame", (0..count).map(|_| frame(&mut app)).collect());
}

/// The large window with its source beside it (M4's U19: the source group
/// open at the design's right): idle frames.
#[test]
#[ignore]
fn a_large_window_with_its_source() {
    let root = mg_testkit::corpus!();
    let (mut app, key, frame) = open(&root, "sourced", &window(12, 120.0));
    let count = std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(60);
    let opening: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    app.actions.push(Action::NuiSource(key, true));
    let sourced: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    println!("source opening: {opening:?} {sourced:?}");
    docked(&app, key);
    assert!(app.dock.find_tab(&Tab::NuiSource(key)).is_some(), "the source group open");
    report("source beside it, idle frame", (0..count).map(|_| frame(&mut app)).collect());
}

/// The chart of a million samples with a bind, chosen in the Bindings table
/// (M4's U19 review: its uses were walked each frame a bind was chosen).
#[test]
#[ignore]
fn a_chart_with_a_bind_chosen() {
    let root = mg_testkit::corpus!();
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 1100.0, "h": 760.0});
    let mut chart = mg_nui::template("chart");
    let data: Vec<f64> = (0..1_000_000).map(|i| (f64::from(i) * 0.001).sin()).collect();
    chart["value"][0]["data"] = json!(data);
    chart["height"] = json!(400.0);
    window["root"]["children"] = json!([chart, {"type": "label", "value": {"bind": "caption"}}]);
    let (mut app, key, frame) = open(&root, "charted", &window);
    let mut s = mg_nui::Settings::default();
    s.bindings.insert(
        "caption".into(),
        mg_nui::Binding { value: json!("A caption"), ..Default::default() },
    );
    let txt = ResKey::parse("charted", ResType::TXT).unwrap();
    app.actions.push(Action::Apply(Command::new(
        "Settings",
        vec![Edit::SetResource { key: txt, data: Some(s.bytes()) }],
    )));
    let count = std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(30);
    let opening: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    app.nui_choose_bind(&key, "caption");
    let chosen: Vec<_> = (0..5).map(|_| frame(&mut app)).collect();
    println!("chart-bind opening: {opening:?} {chosen:?}");
    docked(&app, key);
    report(
        "a chart of 1,000,000 samples, a bind chosen, idle frame",
        (0..count).map(|_| frame(&mut app)).collect(),
    );
}

// The review of November 2026's NUI Creator findings at the sizes they
// name (the deferred list's "performance fixes without a timing view"),
// each view against its own resting view, timed in the same run.

/// The window the views are drawn in (as the other cases here).
const SIZE: egui::Vec2 = egui::vec2(1600.0, 1000.0);

/// The NUI Creator's window: the module of `windows` (name, window, its
/// settings), the first one's tab open, docked as the boards draw it, with
/// pictures read apart as the desktop app reads them; with `user` (a user
/// folder) and `haks`, the module takes those haks.
fn creator(
    root: &std::path::Path,
    name: &str,
    windows: &[(&str, Value, mg_nui::Settings)],
    user: Option<&std::path::Path>,
    haks: Vec<mg_gff::Struct>,
) -> (egui_kittest::Harness<'static, Moonglow>, Vec<ResKey>) {
    let dir = mg_testkit::scratch_dir(&format!("nui-perf-{name}"));
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    if !haks.is_empty() {
        info.root.set("Mod_HakList", mg_gff::Value::List(haks));
    }
    m.set_info(&info).unwrap();
    let mut keys = Vec::new();
    for (name, window, settings) in windows {
        let key = ResKey::parse(name, ResType::JUI).unwrap();
        m.set(key, serde_json::to_vec_pretty(window).unwrap());
        m.set(ResKey::parse(name, ResType::TXT).unwrap(), settings.bytes());
        keys.push(key);
    }
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let install = GameInstall::new(root, user.map(std::path::Path::to_path_buf), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.open_module(&path);
    app.actions.push(Action::OpenTab(Tab::Nui(keys[0])));
    let mut recorded = false;
    let mut h = egui_kittest::Harness::builder().with_size(SIZE).build_ui_state(
        move |ui, app: &mut Moonglow| {
            // (The header's places, for its controls to be found.)
            if !recorded {
                recorded = true;
                mg_uikit::header::record_headers(ui.ctx());
            }
            app.ui(ui);
        },
        app,
    );
    h.state_mut().background_jobs = true;
    h.run_steps(8);
    docked(h.state(), keys[0]);
    (h, keys)
}

/// The tab of `key` the one NUI window shown, drawn a few frames.
fn show_only(h: &mut egui_kittest::Harness<'_, Moonglow>, key: ResKey) {
    let shown: Vec<Tab> = (h.state().dock.iter_all_tabs())
        .map(|(_, t)| t.clone())
        .filter(|t| matches!(t, Tab::Nui(_)))
        .collect();
    for t in shown.iter().filter(|t| **t != Tab::Nui(key)) {
        if let Some(at) = h.state().dock.find_tab(t) {
            h.state_mut().dock.remove_tab(at);
        }
    }
    h.state_mut().actions.push(Action::OpenTab(Tab::Nui(key)));
    h.run_steps(8);
    assert!(h.state().nui_chosen(&key).is_some(), "the designer of {key} drawn");
}

/// The designer's header's control `name`, as `tests/nui.rs`'s `nui_tool`
/// finds it (through More where it went there).
fn header_tool<'t>(
    h: &'t mut egui_kittest::Harness<'_, Moonglow>,
    name: &'t str,
) -> egui_kittest::Node<'t> {
    use egui_kittest::kittest::Queryable;
    use mg_uikit::header::{self, Place};
    let header_name = mg_ui::nui_names::NUI_HEADER;
    let records = header::records(&h.ctx, header_name);
    let rec = records.into_iter().next().expect("the designer's header");
    let place = rec.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p);
    static MORE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let more = MORE.get_or_init(|| header::more_name(header_name)).as_str();
    if place == Some(Place::More) && h.query_by_label(more).is_none() {
        h.get_by_label(header_name).get_by_label(header::MORE).click();
        h.run_steps(2);
    }
    let h: &'t egui_kittest::Harness<'_, Moonglow> = h;
    match place {
        Some(Place::More) => h.get_by_label(more).get_by_label(name),
        Some(Place::Row | Place::Icon) => h.get_by_label(header_name).get_by_label(name),
        other => panic!("{name} in the designer's header: {other:?}"),
    }
}

/// The page `name` chosen in the header's switch.
fn page(h: &mut egui_kittest::Harness<'_, Moonglow>, name: &str) {
    use egui_kittest::kittest::Queryable;
    header_tool(h, mg_ui::nui_names::PAGE).get_by_label(name).click();
    h.run_steps(4);
}

/// A group holding a column of `rows` rows of two controls (the review's
/// N12: each row looked at every later row's width).
fn column_window(rows: usize) -> Value {
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 900.0, "h": 700.0});
    let rows: Vec<Value> = (0..rows)
        .map(|r| {
            let mut label = mg_nui::template("label");
            label["value"] = format!("Entry {r}").into();
            label["width"] = json!(200.0);
            let mut button = mg_nui::template("button");
            button["label"] = format!("Buy {r}").into();
            button["width"] = json!(120.0);
            let mut row = mg_nui::template("row");
            row["children"] = json!([label, button]);
            row["height"] = json!(30.0);
            row
        })
        .collect();
    let mut col = mg_nui::template("col");
    col["children"] = rows.into();
    let mut group = mg_nui::template("group");
    group["children"] = json!([col]);
    group["height"] = json!(500.0);
    window["root"]["children"] = json!([group]);
    window
}

/// A combo of `entries` entries (the review's N14).
fn combo_window(entries: usize) -> Value {
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 600.0, "h": 400.0});
    let mut combo = mg_nui::template("combo");
    let elements: Vec<Value> =
        (0..entries).map(|i| json!([format!("Appearance {i}"), i])).collect();
    combo["elements"] = elements.into();
    combo["width"] = json!(300.0);
    window["root"]["children"] = json!([combo]);
    window
}

/// 2,000 text inputs, `wrong` of them with a mistake (a length of 0: an
/// error each), the rest bound and of a length (the review's N15).
fn findings_window(wrong: usize) -> Value {
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 900.0, "h": 700.0});
    let controls: Vec<Value> = (0..2_000)
        .map(|i| match i < wrong {
            true => json!({"type": "textedit", "label": format!("Name {i}"), "max": 0}),
            false => json!({"type": "textedit", "label": format!("Name {i}"), "max": 40,
                "value": {"bind": "name"}}),
        })
        .collect();
    window["root"]["children"] = controls.into();
    window
}

/// Settings whose bind `name` holds `value`.
fn binds(values: impl IntoIterator<Item = (String, Value)>) -> mg_nui::Settings {
    let mut s = mg_nui::Settings::default();
    for (name, value) in values {
        s.bindings.insert(name, mg_nui::Binding { value, ..Default::default() });
    }
    s
}

/// The large window of 304 controls with `bound` texts bound, each to a
/// bind of its own: a row's label and its button's, row by row (the
/// review's X22).
fn bound_window(bound: usize) -> (Value, mg_nui::Settings) {
    let mut w = window(38, 120.0);
    let rows = w["root"]["children"].as_array_mut().unwrap();
    for b in 0..bound {
        let (cell, field) = if b % 2 == 0 { (0, "value") } else { (1, "label") };
        rows[b / 2]["children"][cell][field] = json!({"bind": format!("caption_{b}")});
    }
    let settings = binds((0..bound).map(|r| (format!("caption_{r}"), json!(format!("Row {r}")))));
    (w, settings)
}

/// 511 controls nested eight deep (a column or a row of two each, 256
/// labels at the leaves), or the 256 labels in 16 rows of 16 (the review's
/// X23: Layers wrote each control out with all under it, every frame).
fn tree_window(deep: bool) -> Value {
    fn label(n: usize) -> Value {
        let mut label = mg_nui::template("label");
        label["value"] = format!("Leaf {n}").into();
        label["width"] = json!(60.0);
        label
    }
    fn level(depth: usize, n: &mut usize) -> Value {
        if depth == 8 {
            *n += 1;
            return label(*n);
        }
        let mut c = mg_nui::template(if depth.is_multiple_of(2) { "col" } else { "row" });
        c["children"] = json!([level(depth + 1, n), level(depth + 1, n)]);
        c
    }
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 1100.0, "h": 760.0});
    let children: Vec<Value> = match deep {
        true => vec![level(0, &mut 0)],
        false => (0..16)
            .map(|r| {
                let mut row = mg_nui::template("row");
                row["children"] = (0..16).map(|c| label(r * 16 + c)).collect::<Vec<_>>().into();
                row
            })
            .collect(),
    };
    window["root"]["children"] = children.into();
    window
}

/// A list of `rows` rows whose cell's text is bound to a list of that
/// many (the review's X24).
fn list_window(rows: usize) -> (Value, mg_nui::Settings) {
    let mut window = mg_nui::window();
    window["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 900.0, "h": 700.0});
    let mut list = mg_nui::template("list");
    list["row_count"] = json!({"bind": "names"});
    list["row_template"] = json!([[{"type": "label", "id": "cell",
        "value": {"bind": "names"}}, 0.0, true]]);
    window["root"]["children"] = json!([list]);
    let names: Vec<String> = (0..rows).map(|i| format!("Row {i}")).collect();
    (window, binds([("names".to_owned(), json!(names))]))
}

/// A window of one picture named `name` and a row of eight controls.
fn picture_window(name: &str) -> Value {
    let mut w = window(1, 120.0);
    let mut image = mg_nui::template("image");
    image["value"] = name.into();
    image["width"] = json!(128.0);
    image["height"] = json!(128.0);
    w["root"]["children"].as_array_mut().unwrap().insert(0, image);
    w
}

/// The review of November 2026's NUI Creator findings at the sizes they
/// name, each against the same view at a small size (or without what the
/// finding was of), timed in the same run, its rest timed twice for the
/// noise: two Creator tabs side by side, one naming a picture the other
/// doesn't, against two naming the same (N3), and Split source with a
/// picture's name typed against its text unchanged (N3); a group's column
/// of 1,000 rows against 10 (N12); in Interact, a combo of 10,000 entries,
/// closed, against 10 (N14); 2,000 findings against 100 (more than are in
/// sight), the list open (N15); the Bindings page of 60 binds against 12,
/// 304 controls (X22); Design of 511 controls nested eight deep against
/// their 256 labels in 16 rows (X23); a list's cell of 5,000 rows chosen
/// against 100, then New Bind for a Property's dialog (X24); Choose image
/// with a name typed against nothing typed, over the install's pictures
/// and the persistent world's haks where it is built, and a key a frame in
/// its Find (X11, X20). `NUI_VIEW=<part of a name>` picks the groups
/// ("tabs", "Split source", "column", "combo", "findings", "Bindings",
/// "Design of", "a list's cell", "Choose").
#[test]
#[ignore]
fn the_creator_at_scale() {
    use egui_kittest::kittest::Queryable;
    let root = mg_testkit::corpus!();
    let mut v = Views::new("NUI_VIEW", "FRAME_COUNT");
    let none = mg_nui::Settings::default;

    // N3: two tabs, side by side.
    let name = "two Creator tabs side by side";
    if v.wanted(name) {
        let other = "gui_mp_examineu";
        let mut named = window(12, 120.0);
        named["root"]["children"][0]["children"][7]["value"] = other.into();
        let windows = [
            ("nui_a", window(12, 120.0), none()),
            ("nui_b", window(12, 120.0), none()),
            ("nui_c", named, none()),
        ];
        let (mut h, keys) = creator(&root, "tabs", &windows, None, Vec::new());
        let beside = |h: &mut egui_kittest::Harness<'_, Moonglow>, other: ResKey| {
            show_only(h, keys[0]);
            let node = h.state().dock.find_tab(&Tab::Nui(keys[0])).unwrap().node;
            h.state_mut().dock.main_surface_mut().split_right(node, 0.5, vec![Tab::Nui(other)]);
            h.run_steps(8);
            assert!(h.state().nui_chosen(&other).is_some(), "{other} drawn beside");
        };
        beside(&mut h, keys[1]);
        let rest = v.rest(&mut h);
        beside(&mut h, keys[2]);
        v.against(&mut h, &format!("{name}, one naming {other}"), rest, |_| Vec::new());
    }

    // N3: Split source, a picture's name typed in its text.
    let name = "Split source";
    if v.wanted(name) {
        let w = picture_window("gui_error");
        let (mut h, keys) = creator(&root, "source", &[("nui_a", w.clone(), none())], None, vec![]);
        h.state_mut().actions.push(Action::NuiSource(keys[0], true));
        h.run_steps(8);
        h.get_by_label(mg_ui::nui_names::SOURCE_TEXT).click();
        h.run_steps(2);
        let typed = |h: &mut egui_kittest::Harness<'_, Moonglow>, w: &Value| {
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
            h.run_steps(2);
            h.event(egui::Event::Text(serde_json::to_string_pretty(w).unwrap()));
            h.run_steps(6);
        };
        typed(&mut h, &w);
        let rest = v.rest(&mut h);
        let mut renamed = w.clone();
        renamed["root"]["children"][0]["value"] = json!("gui_mp_examineu");
        typed(&mut h, &renamed);
        assert!(
            h.query_by_label(mg_ui::nui_names::SOURCE_TEXT).is_some(),
            "{name}: the source's text shown"
        );
        v.against(&mut h, &format!("{name}, a picture's name typed"), rest, |_| Vec::new());
    }

    // N12: a column of 1,000 rows.
    let name = "a group's column";
    if v.wanted(name) {
        let windows =
            [("nui_few", column_window(10), none()), ("nui_many", column_window(1_000), none())];
        let (mut h, keys) = creator(&root, "column", &windows, None, Vec::new());
        let rest = v.rest(&mut h);
        show_only(&mut h, keys[1]);
        v.against(&mut h, &format!("{name} of 1,000 rows (against 10)"), rest, |_| Vec::new());
    }

    // N14: Interact, a combo of 10,000 entries, closed.
    let name = "a combo in Interact";
    if v.wanted(name) {
        let windows =
            [("nui_few", combo_window(10), none()), ("nui_many", combo_window(10_000), none())];
        let (mut h, keys) = creator(&root, "combo", &windows, None, Vec::new());
        let interact = |h: &mut egui_kittest::Harness<'_, Moonglow>, key: ResKey| {
            header_tool(h, mg_ui::nui_names::INTERACT).click();
            h.run_steps(6);
            assert!(h.state().nui_interacts(&key), "Interact on");
        };
        interact(&mut h, keys[0]);
        let rest = v.rest(&mut h);
        show_only(&mut h, keys[1]);
        interact(&mut h, keys[1]);
        v.against(&mut h, &format!("{name}, 10,000 entries (against 10)"), rest, |_| Vec::new());
    }

    // N15: 2,000 findings, the list open.
    let name = "findings";
    if v.wanted(name) {
        let windows = [
            ("nui_few", findings_window(100), none()),
            ("nui_many", findings_window(2_000), none()),
        ];
        let (mut h, keys) = creator(&root, "findings", &windows, None, Vec::new());
        let errors =
            |h: &egui_kittest::Harness<'_, Moonglow>| h.query_all_by_label("Error").count();
        assert!(errors(&h) >= 3, "{name}: the list open on 100 errors");
        let rest = v.rest(&mut h);
        show_only(&mut h, keys[1]);
        assert!((3..60).contains(&errors(&h)), "{name}: {} lines laid out", errors(&h));
        v.against(&mut h, &format!("2,000 {name}, the list open (against 100)"), rest, |_| vec![]);
    }

    // X22: the Bindings page of 60 binds.
    let name = "the Bindings page";
    if v.wanted(name) {
        let (one, one_s) = bound_window(12);
        let (many, many_s) = bound_window(60);
        let windows = [("nui_few", one, one_s), ("nui_many", many, many_s)];
        let (mut h, keys) = creator(&root, "binds", &windows, None, Vec::new());
        page(&mut h, "Bindings");
        let rest = v.rest(&mut h);
        show_only(&mut h, keys[1]);
        page(&mut h, "Bindings");
        assert!(h.query_all_by_label_contains("caption_59").next().is_some(), "{name}: the 60th");
        let what = format!("{name} of 60 binds, 304 controls (against 12)");
        v.against(&mut h, &what, rest, |_| Vec::new());
    }

    // X23: Design with a deep window.
    let name = "Design of 511 controls";
    if v.wanted(name) {
        let windows =
            [("nui_flat", tree_window(false), none()), ("nui_deep", tree_window(true), none())];
        let (mut h, keys) = creator(&root, "deep", &windows, None, Vec::new());
        let rest = v.rest(&mut h);
        show_only(&mut h, keys[1]);
        let what = format!("{name} nested eight deep (against its 256 labels in 16 rows)");
        v.against(&mut h, &what, rest, |_| Vec::new());
    }

    // X24: a list's cell of 5,000 rows chosen.
    let name = "a list's cell";
    if v.wanted(name) {
        let (few, few_s) = list_window(100);
        let (many, many_s) = list_window(5_000);
        let windows = [("nui_few", few, few_s), ("nui_many", many, many_s)];
        let (mut h, keys) = creator(&root, "list", &windows, None, Vec::new());
        let cell = "/root/children/0/row_template/0/0";
        let choose = |h: &mut egui_kittest::Harness<'_, Moonglow>, key: ResKey| {
            h.state_mut().nui_choose(&key, cell);
            h.run_steps(6);
            assert_eq!(h.state().nui_chosen(&key).as_deref(), Some(cell), "the cell chosen");
        };
        let new_bind = |h: &mut egui_kittest::Harness<'_, Moonglow>| {
            let before = h.query_all_by_label("New Bind for a Property").count();
            let button = h.get_all_by_label("New Bind for a Property").next().unwrap();
            button.scroll_to_me();
            h.run_steps(6);
            h.get_all_by_label("New Bind for a Property").next().unwrap().click();
            h.run_steps(6);
            let after = h.query_all_by_label("New Bind for a Property").count();
            println!("  ({name}: New Bind for a Property's labels {before}, then {after})");
        };
        choose(&mut h, keys[0]);
        let rest = v.rest(&mut h);
        new_bind(&mut h);
        let rest_bind = v.rest(&mut h);
        h.key_press(egui::Key::Escape);
        h.run_steps(4);
        show_only(&mut h, keys[1]);
        choose(&mut h, keys[1]);
        v.against(&mut h, &format!("{name} of 5,000 rows chosen (against 100)"), rest, |_| vec![]);
        new_bind(&mut h);
        let what = format!("{name} of 5,000 rows, New Bind for a Property open (against 100)");
        v.against(&mut h, &what, rest_bind, |_| Vec::new());
    }

    // X11, X20: Choose image with a name typed, over the install's pictures
    // and the persistent world's haks where it is built.
    let name = "Choose image";
    if v.wanted(name) {
        let world = mg_testkit::scratch_dir("nui-perf-choose-user");
        let (user, haks) = match world_haks(&world) {
            Some(haks) => (Some(world.as_path()), haks),
            None => (None, Vec::new()),
        };
        let windows = [("nui_a", picture_window("gui_error"), none())];
        let (mut h, keys) = creator(&root, "choose", &windows, user, haks);
        let pictures = {
            let game = h.state().game.clone().unwrap();
            let kinds = [ResType::TGA, ResType::DDS, ResType::PNG, ResType::JPG, ResType::BMP];
            let mut names: Vec<_> = (game.resman.entries().into_iter())
                .filter(|(k, _)| kinds.contains(&k.restype))
                .map(|(k, _)| k.resref)
                .collect();
            names.sort();
            names.dedup();
            names.len()
        };
        println!("  ({name}: {pictures} picture names in the load order)");
        h.state_mut().nui_choose(&keys[0], "/root/children/0");
        h.run_steps(6);
        let header = h.get_by_label("Choose image");
        header.scroll_to_me();
        h.run_steps(6);
        h.get_by_label("Choose image").click();
        h.run_steps(6);
        let find = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::TextInput
                && n.placeholder().is_some_and(|p| p.contains("Find a picture"))
        };
        let rest = v.rest(&mut h);
        h.query(egui_kittest::kittest::by().predicate(find))
            .unwrap_or_else(|| panic!("{name}: no Find a picture…"))
            .click();
        h.run_steps(2);
        h.event(egui::Event::Text("gui_error".into()));
        h.run_steps(8);
        v.against(&mut h, &format!("{name}, \"gui_error\" typed"), rest, |_| Vec::new());
        // A key a frame: each changes what is found (a letter that finds
        // nothing, then taken back).
        let what = format!("{name}, a key a frame in its Find");
        v.against(&mut h, &what, rest, |n| match n % 2 {
            0 => vec![egui::Event::Text("q".into())],
            _ => vec![egui::Event::Key {
                key: egui::Key::Backspace,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        });
    }

    println!(
        "against their resting views (the line {} µs, the budget {:.2} ms):\n  {}",
        UI_LINE.as_micros(),
        BUDGET.as_secs_f64() * 1000.0,
        v.lines.join("\n  ")
    );
}

/// The persistent world's haks (`world.rs`'s, which `frame_perf` builds)
/// linked into the user folder `user`: the module's hak list, `None` where
/// the world isn't built.
fn world_haks(user: &std::path::Path) -> Option<Vec<mg_gff::Struct>> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-output/pw-world/user");
    let module = base.join("modules/pw_world.mod");
    if !module.is_file() {
        println!("  (the persistent world isn't built: the install's pictures alone)");
        return None;
    }
    let hak = user.join("hak");
    if !hak.exists() {
        #[cfg(unix)]
        std::os::unix::fs::symlink(base.join("hak"), &hak).unwrap();
        #[cfg(not(unix))]
        return None;
    }
    let w = mg_module::Module::open(&module).ok()?;
    Some(w.info().ok()?.root.list("Mod_HakList").unwrap_or(&[]).to_vec())
}
