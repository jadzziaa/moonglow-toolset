//! How fast the NUI Creator is on a large window (a hundred controls, a
//! long text and a list). Timing only; run with
//! `cargo test --release -p mg-ui --test nui_perf -- --ignored --nocapture`.
//! `FRAME_COUNT=<n>` sets the frames timed, `NUI_PHASE=idle|edit|elsewhere`
//! times one of the large window's three (to sample its stacks);
//! `a_window_of_text` is a window that is mostly text.

use std::time::{Duration, Instant};

use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Action, Moonglow, NoDialogs, Tab};
use serde_json::{Value, json};

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
    let dir = mg_testkit::scratch_dir(&format!("nui-perf-{name}"));
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let key = ResKey::parse(name, ResType::JUI).unwrap();
    m.set(key, serde_json::to_vec_pretty(window).unwrap());
    m.set(ResKey::parse(name, ResType::TXT).unwrap(), mg_nui::Settings::default().bytes());
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app =
        Moonglow::new(Some(GameInstall::new(root, None, "en")), Box::new(NoDialogs::default()));
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
    report(
        "a chart of 1,000,000 samples, idle frame",
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
