//! L5 check of the NUI Creator's checks: what `mg_nui::validate` says the
//! game does with a window, the game client must do.
//!
//! One scratch module opens a table of small windows in the client, one
//! after another, each through the opener `mg_nui::opener_source` writes
//! for it (compiled by the module's compile path), and each is held to
//! what the checks say of it:
//!
//! - **shown**: the client builds the window and draws it;
//! - **refused**: the client cannot build it, and shows "Error constructing
//!   window from json" in its place ("The constraint can not be
//!   satisfied", or what its JSON reader says);
//! - **blank**: the client builds the window and draws nothing in it.
//!
//! How the three are told, the client writing no line of its own about any
//! of them (not in its logs, not on its standard error):
//!
//! - Every window's `geometry` is a bind, centered (`x` and `y` −1). The
//!   client writes the place it gave a window it built back into the bind;
//!   for one its layout solver refused it writes a rectangle of zeros, and
//!   for one its JSON reader refused nothing. The module's script logs the
//!   bind (`NuiGetBind`) before it closes the window.
//! - Every window draws filled magenta rectangles (draw lists), a color
//!   nothing else on the screen has: a window with none of it on the
//!   client's screenshot is blank.
//!
//! The cases cover the checks that cite the client and can be seen this
//! way: the layouts it refuses, Clip to control on the last draw list, the
//! labels of Options read from a bind. The warnings about what a control
//! does when used (a Toggle button with a fixed value, a disabled slider,
//! a text field's `max` below 4) need a mouse and a keyboard, and are not
//! here; that Clip to control clips nothing is. The cases marked as probes record what the client does where the
//! checks only warn: constants out of range, a root that is no row, column
//! or group, a window ID with punctuation, the shape of an arc's center.
//!
//! The client runs sandboxed (`tools/nwclient/run-client.sh`: no Steam, no
//! network, no sound, a scratch user directory) on the off-screen display
//! (`tools/aurora/headless.sh start`), once, under a timeout and (where
//! there is `systemd-run`) a memory cap of 8 GB.
//!
//! Needs the game, a GPU and the off-screen display; takes two and a half
//! minutes;
//! run by hand:
//! `DISPLAY=:1 cargo test -p mg-corpus-tests --release --test client_nui -- --ignored --nocapture`.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use mg_core::{ResRef, ResType};
use mg_module::ModuleLocation;
use mg_module::build::{ScriptSelection, compile_scripts};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_nui::{Binding, Settings, Severity};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::{corpus, scratch_dir};
use serde_json::{Value, json};

mod common;
use common::{SETTINGS, read_png, repo};

/// Seconds before the first window, a window's turn, how long it stays
/// open, and when a variant is swapped in.
const FIRST: f32 = 8.0;
const TURN: f32 = 4.0;
const HOLD: f32 = 3.4;
const SWAP: f32 = 0.8;

/// Added to the client's settings (under `[graphics.video]`, where
/// [`SETTINGS`] ends): frames at a steady 30 a second, not waiting on the
/// off-screen display's refresh. Waiting on it, the picture of the client's
/// window ran one to four seconds behind what the client was showing.
const FRAMES: &str = r#"		[graphics.video.vsync]
			mode = "off"
	[graphics.framelimiter]
		enabled = true
		[graphics.framelimiter.background]
			target-fps = 30
		[graphics.framelimiter.foreground]
			target-fps = 30
		[graphics.framelimiter.non-interactive]
			target-fps = 30
"#;

/// What the client does with a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Client {
    Shown,
    Refused,
    Blank,
}
use Client::{Blank, Refused, Shown};

struct Case {
    name: &'static str,
    window: Value,
    settings: Settings,
    /// A part of the error `validate` must give; `None`: no error.
    error: Option<&'static str>,
    client: Client,
    /// The variant shown in place of the whole window once it is open.
    swap: Option<&'static str>,
    /// Whether the green of [`arc`] must be on the screen, where asked.
    green: Option<bool>,
    /// The fewest magenta pixels there must be, where asked.
    magenta: usize,
}

const MAGENTA: [u8; 3] = [255, 0, 255];
const GREEN: [u8; 3] = [0, 255, 0];

fn color(c: [u8; 3]) -> Value {
    json!({"r":c[0],"g":c[1],"b":c[2],"a":255})
}

/// A filled magenta rectangle, the color the screenshot is searched for.
fn rect(order: i64, render: i64) -> Value {
    json!({"type":7,"enabled":true,"color":color(MAGENTA),"fill":true,"line_thickness":1.0,
        "order":order,"render":render,"rect":{"x":0.0,"y":0.0,"w":120.0,"h":40.0}})
}

/// A spacer that draws `items`, Clip to control as given.
fn spacer(clip: Value, items: Value) -> Value {
    json!({"type":"spacer","width":120.0,"height":40.0,"draw_list_scissor":clip,"draw_list":items})
}

/// A spacer that draws the rectangle, Clip to control as given.
fn painted(clip: Value) -> Value {
    spacer(clip, json!([rect(1, 0)]))
}

/// The rectangle unclipped: in every window that has no draw list of its
/// own to be seen by.
fn mark() -> Value {
    painted(json!(false))
}

fn button(key: &str, size: f64, margin: Option<f64>) -> Value {
    let mut b = json!({"type":"button","label":"b","value":null});
    b[key] = json!(size);
    if let Some(m) = margin {
        b["margin"] = json!(m);
    }
    b
}

fn row(height: f64, child: Value) -> Value {
    json!({"type":"row","height":height,"children":[child]})
}

fn col(width: f64, child: Value) -> Value {
    json!({"type":"col","width":width,"children":[child]})
}

/// A spacer that draws a filled green arc with the center given.
fn arc(center: Value) -> Value {
    json!({"type":"spacer","width":120.0,"height":80.0,"draw_list_scissor":false,
        "draw_list":[{"type":3,"enabled":true,"color":color(GREEN),"fill":true,"line_thickness":2.0,
            "order":1,"render":0,"c":center,"radius":30.0,"amin":0.0,"amax":6.0}]})
}

/// A window with `root`, its geometry the bind `geo`.
fn case(name: &'static str, root: Value, error: Option<&'static str>, client: Client) -> Case {
    let mut window = mg_nui::window();
    window["title"] = name.into();
    window["geometry"] = json!({"bind":"geo"});
    window["resizable"] = false.into();
    window["collapsed"] = false.into();
    window["root"] = root;
    let mut settings = Settings::default();
    settings.bindings.insert(
        "geo".into(),
        Binding { value: json!({"x":-1.0,"y":-1.0,"w":400.0,"h":300.0}), ..Default::default() },
    );
    Case { name, window, settings, error, client, swap: None, green: None, magenta: 0 }
}

/// A window whose root is a column of `children`.
fn column(
    name: &'static str,
    children: Value,
    error: Option<&'static str>,
    client: Client,
) -> Case {
    case(name, json!({"type":"col","children":children}), error, client)
}

impl Case {
    fn bind(mut self, name: &str, value: Value) -> Case {
        self.settings.bindings.insert(name.into(), Binding { value, ..Default::default() });
        self
    }
    fn id(mut self, id: &str) -> Case {
        self.settings.window_id = Some(id.into());
        self
    }
    fn magenta(mut self, least: usize) -> Case {
        self.magenta = least;
        self
    }
    fn green(mut self, seen: bool) -> Case {
        self.green = Some(seen);
        self
    }
}

fn cases() -> Vec<Case> {
    let fits = None;
    let unfit = Some("constraint");
    let blank = Some("blank");
    let label = || json!({"type":"label","value":"Label","height":30.0});
    let high = |room: f64, size: f64, margin| row(room, button("height", size, margin));
    let wide = |room: f64, size: f64| col(room, button("width", size, None));
    let on = || painted(json!(true));
    let gap = || json!({"type":"spacer","height":30.0});
    let over = || {
        spacer(
            json!(true),
            json!([{"type":7,"enabled":true,"color":color(MAGENTA),"fill":true,
                "line_thickness":1.0,"order":1,"render":0,
                "rect":{"x":-20.0,"y":-20.0,"w":160.0,"h":80.0}}]),
        )
    };
    let mut all = vec![
        column("plain", json!([mark(), label(), button("height", 30.0, None)]), None, Shown),
        // Across a row or a column, a child's size and margins must fit:
        // the windows of `a_size_across_a_row_or_column_that_the_client_
        // cannot_fit_is_an_error` (mg-nui), each under the mark.
        column("row60_80", json!([mark(), high(60.0, 80.0, None)]), unfit, Refused),
        column("row30_30", json!([mark(), high(30.0, 30.0, None)]), unfit, Refused),
        column("row33_30", json!([mark(), high(33.0, 30.0, None)]), unfit, Refused),
        column("row30_29", json!([mark(), high(30.0, 29.0, None)]), unfit, Refused),
        column("col100_150", json!([mark(), wide(100.0, 150.0)]), unfit, Refused),
        column("col150_150", json!([mark(), wide(150.0, 150.0)]), unfit, Refused),
        column("row34_30", json!([mark(), high(34.0, 30.0, None)]), fits, Shown),
        column("row30_30m0", json!([mark(), high(30.0, 30.0, Some(0.0))]), fits, Shown),
        column("col154_150", json!([mark(), wide(154.0, 150.0)]), fits, Shown),
        column(
            "row_group",
            json!([mark(), row(60.0, json!({"type":"group","height":40.0,"children":[label()]}))]),
            fits,
            Shown,
        ),
        // Along a row, children may overflow it; a group scrolls a taller child.
        column(
            "row_along",
            json!([mark(), {"type":"row","width":300.0,"children":[
                button("width", 200.0, None), button("width", 200.0, None)]}]),
            fits,
            Shown,
        ),
        column(
            "group_tall",
            json!([mark(), {"type":"group","height":60.0,
                "children":[button("height", 100.0, None)]}]),
            fits,
            Shown,
        ),
        // Clip to control on the draw list painted last blanks the window;
        // on an earlier one it does nothing.
        column("clip_last", json!([on(), mark(), on()]), blank, Blank),
        column(
            "clip_button",
            json!([mark(), {"type":"button","label":"b","height":40.0,"draw_list_scissor":true,
                "draw_list":[rect(1, 0)]}]),
            blank,
            Blank,
        ),
        column("clip_bound", json!([mark(), painted(json!({"bind":"clip"}))]), blank, Blank)
            .bind("clip", json!(true)),
        column("clip_first", json!([on(), mark()]), None, Shown),
        column("clip_none", json!([mark(), mark()]), None, Shown),
        // It clips nothing: a rectangle 20 pixels past the spacer on every
        // side (160 by 80, where the spacer is 120 by 40) is drawn past it
        // (16,000 pixels with the mark's: all but what the window's own
        // left edge cuts; clipped, 9,600).
        column("clip_overflow", json!([gap(), over(), gap(), mark()]), None, Shown).magenta(14_000),
        // A draw list with nothing in it is not the last one painted: the
        // clipped one before it is, and the window is blank.
        column(
            "clip_empty",
            json!([gap(), over(), gap(), spacer(json!(false), json!([]))]),
            blank,
            Blank,
        ),
        // The labels of Options are the constructor's own: a bind there
        // "fails in the native client parser".
        column(
            "options_bound",
            json!([mark(), {"type":"options","direction":0,"elements":{"bind":"labels"},
                "value":0,"height":30.0}]),
            Some("static"),
            Refused,
        )
        .bind("labels", json!(["First", "Second"])),
        // Probes: constants out of range are warned of, not refused.
        column(
            "halign_9",
            json!([mark(), {"type":"label","value":"Label","height":30.0,
                "text_halign":9,"text_valign":9}]),
            None,
            Shown,
        ),
        column("render_9", json!([spacer(json!(false), json!([rect(1, 9)]))]), None, Shown),
        column("order_5", json!([spacer(json!(false), json!([rect(5, 0)]))]), None, Shown),
        // Probes: a root that is no row, column or group.
        case("root_spacer", mark(), None, Shown),
        case(
            "root_label",
            json!({"type":"label","value":"Label","draw_list_scissor":false,
                "draw_list":[rect(1, 0)]}),
            None,
            Shown,
        ),
        case(
            "root_button",
            json!({"type":"button","label":"b","draw_list_scissor":false,
                "draw_list":[rect(1, 0)]}),
            None,
            Shown,
        ),
        // Probes: a window ID that is not short and alphanumeric.
        column("id_punct", json!([mark()]), None, Shown).id("a.b-c:d e/f!"),
        column("id_unicode", json!([mark()]), None, Shown).id("okno \u{17c}\u{f3}\u{142}wia"),
        // Probes: an arc's center as a point, and as the rectangle
        // nw_inc_nui documents.
        column("arc_point", json!([mark(), arc(json!({"x":60.0,"y":40.0}))]), None, Shown)
            .green(true),
        column(
            "arc_rect",
            json!([mark(), arc(json!({"x":60.0,"y":40.0,"w":0.0,"h":0.0}))]),
            None,
            Shown,
        )
        .green(true),
    ];
    // A variant shown in place of the whole window is its last drawing.
    let mut variant = column("clip_variant", json!([mark()]), Some("when this variant"), Blank);
    variant
        .settings
        .variants
        .insert("cover".into(), json!({"type":"col","children":[mark(), on()]}));
    variant.settings.variant_targets.insert("cover".into(), "_window_".into());
    variant.swap = Some("cover");
    all.push(variant);
    all
}

/// The module's OnClientEnter: each case's window opened in its turn by its
/// opener, and closed again with a line of what the server knows of it.
fn enter_script(cases: &[Case]) -> String {
    let mut s = String::new();
    for c in cases {
        s += &format!("#include \"{}_o\"\n", c.name);
    }
    s += r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

// The one window open: found by its ID, the ID read back, the geometry
// bind as the client left it.
void Close(object pc, string name, string id)
{
    int t = NuiGetNthWindow(pc, 0);
    Log("MG_END " + name + " found=" + IntToString(t != 0 && NuiFindWindow(pc, id) == t)
        + " id=" + IntToString(NuiGetWindowId(pc, t) == id)
        + " geo=" + JsonDump(NuiGetBind(pc, t, "geo")));
    if (t != 0) NuiDestroy(pc, t);
}

void Swap(object pc, string name, json layout)
{
    NuiSetGroupLayout(pc, NuiGetNthWindow(pc, 0), "_window_", layout);
    Log("MG_SWAP " + name + " .");
}

void Show(object pc, int n)
{
"#;
    for (i, c) in cases.iter().enumerate() {
        let name = c.name;
        let id = mg_nui::string_expr(c.settings.window_id.as_deref().unwrap_or(name));
        s += &format!("    if (n == {i})\n    {{\n        Open_{name}(pc);\n");
        s += &format!("        Log(\"MG_SHOW {name} .\");\n");
        if let Some(view) = c.swap {
            let layout = mg_nui::variant_function(name, view);
            s += &format!("        DelayCommand({SWAP:.1}, Swap(pc, \"{name}\", {layout}()));\n");
        }
        s += &format!("        DelayCommand({HOLD:.1}, Close(pc, \"{name}\", {id}));\n    }}\n");
    }
    s += "}\n\nvoid main()\n{\n    object pc = GetEnteringObject();\n    if (!GetIsPC(pc)) return;\n";
    for i in 0..cases.len() {
        s += &format!("    DelayCommand({:.1}, Show(pc, {i}));\n", FIRST + TURN * i as f32);
    }
    let end = FIRST + TURN * cases.len() as f32;
    s += &format!("    DelayCommand({end:.1}, Log(\"MG_DONE\"));\n}}\n");
    s
}

fn key(name: &str, ty: ResType) -> ResKey {
    ResKey::parse(name, ty).unwrap()
}

/// The module: an area to stand in, each case's opener, the entry script.
fn build_module(game: &GameData, cases: &[Case]) -> mg_module::Module {
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(game, "MgNui", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Scene".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 4,
        height: 4,
    };
    add_area(&mut m, game, &spec, &mut rng).unwrap();
    for c in cases {
        // (Build itself refuses a window with errors: the opener is
        // written as Build writes it, and compiled as Build compiles.)
        let opener = mg_nui::opener_source(c.name, &c.window, &c.settings).unwrap();
        m.set(key(&format!("{}_o", c.name), ResType::NSS), opener.into_bytes());
    }
    m.set(key("mg_enter", ResType::NSS), enter_script(cases).into_bytes());
    for r in compile_scripts(&mut m, &game.resman, ScriptSelection::All) {
        assert!(r.result.is_ok(), "{}: {:?}", r.script, r.result);
    }
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    m.set_info(&info).unwrap();
    m
}

/// The client on the module, under a timeout and, where there is
/// `systemd-run`, a memory cap, in a scope of its own; stopped when dropped.
struct Running {
    child: Child,
    unit: Option<String>,
}

impl Running {
    fn start(dir: &Path, module: &str, seconds: u64) -> Running {
        let display = std::env::var("DISPLAY").unwrap_or_default();
        assert!(
            !display.is_empty() && display != ":0",
            "run on the off-screen display (tools/aurora/headless.sh start)"
        );
        let scoped = Command::new("systemd-run")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        let unit = scoped.then(|| format!("mg-client-nui-{}", std::process::id()));
        let mut command = match &unit {
            Some(unit) => {
                let mut c = Command::new("systemd-run");
                c.args(["--user", "--scope", "--quiet", "--collect"])
                    .arg(format!("--unit={unit}"))
                    .args(["-p", "MemoryMax=8G", "-p", "MemorySwapMax=0", "timeout"]);
                c
            }
            None => Command::new("timeout"),
        };
        let child = command
            .args(["-k", "5", &seconds.to_string()])
            .arg(repo().join("tools/nwclient/run-client.sh"))
            .arg(dir)
            .args(["+TestNewModule", module])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start the client");
        Running { child, unit }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // The scope holds the sandbox and the client in it.
        if let Some(unit) = &self.unit {
            let _ = Command::new("systemctl")
                .args(["--user", "stop", &format!("{unit}.scope")])
                .stderr(Stdio::null())
                .status();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The pixels of each of `colors` on the client's window now; the picture
/// is left at `path`.
fn pixels(path: &Path, colors: [[u8; 3]; 2]) -> Option<[usize; 2]> {
    let ok = Command::new("python3")
        .arg(repo().join("tools/aurora/xdrive.py"))
        .args(["winshot", "Neverwinter Nights: Enhanced Edition"])
        .arg(path)
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        return None;
    }
    let img = read_png(path);
    let near = |p: &[u8; 4], c: [u8; 3]| (0..3).all(|i| p[i].abs_diff(c[i]) < 40);
    Some(colors.map(|c| img.data.as_chunks::<4>().0.iter().filter(|p| near(p, c)).count()))
}

/// The last `MG_` line of the log: `(tag, case)`.
fn last_line(log: &str) -> Option<(&str, &str)> {
    let line = log.lines().rfind(|l| l.contains("MG_"))?;
    let mut words = line[line.find("MG_")?..].split(' ');
    Some((words.next()?, words.next().unwrap_or_default()))
}

#[test]
#[ignore]
fn the_client_does_what_the_checks_say() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let dir = scratch_dir("client_nui");
    let cases = cases();
    let mut m = build_module(&game, &cases);
    std::fs::create_dir_all(dir.join("user")).unwrap();
    std::fs::write(dir.join("user/settings.tml"), format!("{SETTINGS}{FRAMES}")).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgNui.mod"))).unwrap();

    // In single player the game's server logs to the client's log.
    let log = dir.join("user/logs/nwclientLog1.txt");
    let read = || String::from_utf8_lossy(&std::fs::read(&log).unwrap_or_default()).into_owned();
    let limit = (FIRST + TURN * cases.len() as f32) as u64 + 90;
    let client = Running::start(&dir, "MgNui", limit + 30);
    let start = Instant::now();
    // The screenshots of each case while it was in its last state (shown,
    // or swapped where it swaps): the pixels of each color.
    let mut seen: Vec<Vec<[usize; 2]>> = vec![Vec::new(); cases.len()];
    let mut text = String::new();
    while start.elapsed() < Duration::from_secs(limit) && !text.contains("MG_DONE") {
        std::thread::sleep(Duration::from_millis(250));
        text = read();
        let Some((tag, name)) = last_line(&text) else { continue };
        let Some(i) = cases.iter().position(|c| c.name == name) else { continue };
        if tag != if cases[i].swap.is_some() { "MG_SWAP" } else { "MG_SHOW" } {
            continue;
        }
        let Some(count) = pixels(&dir.join(format!("{name}.png")), [MAGENTA, GREEN]) else {
            continue;
        };
        // (Taken while the case was still in that state.)
        if last_line(&read()) == Some((tag, name)) {
            seen[i].push(count);
        }
    }
    drop(client);
    let text = read();
    assert!(text.contains("MG_DONE"), "the client did not get through (see {})", dir.display());

    let mut failures = Vec::new();
    eprintln!("{:14} {:24} {:8} {:8} server", "case", "validate", "expected", "client");
    for (c, seen) in cases.iter().zip(&seen) {
        let errors: Vec<String> = mg_nui::validate(&c.window, &c.settings)
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.message)
            .collect();
        let said = match c.error {
            None => errors.is_empty(),
            Some(part) => errors.iter().any(|m| m.contains(part)),
        };
        if !said {
            failures
                .push(format!("{}: validate should say {:?}, says {errors:?}", c.name, c.error));
        }
        let tag = format!("MG_END {} ", c.name);
        let Some(end) = text.lines().find_map(|l| l.find(&tag).map(|i| &l[i + tag.len()..])) else {
            failures.push(format!("{}: never closed", c.name));
            continue;
        };
        let (flags, geo) = end.split_once(" geo=").unwrap_or((end, "null"));
        let geo: Value = serde_json::from_str(geo).unwrap_or(Value::Null);
        // Placed by the client: no longer centered by −1, and not zeros.
        let at = |k: &str| geo[k].as_f64().unwrap_or(-1.0);
        let built = at("x") >= 0.0 && at("y") >= 0.0 && at("w") > 0.0;
        // The last three pictures (a second), which must agree: the first
        // ones of a case can still be of the window before it.
        let [.., first, second, last] = seen[..] else {
            failures.push(format!("{}: fewer than three pictures", c.name));
            continue;
        };
        let painted = last[0] > 0;
        let client = match (built, painted) {
            (true, true) => Shown,
            (true, false) => Blank,
            (false, _) => Refused,
        };
        let green = last[1] > 0;
        eprintln!(
            "{:14} {:24} {:8} {:8} {flags} geo={geo} magenta={} green={}",
            c.name,
            c.error.map_or("accepts".to_string(), |e| format!("error: {e}")),
            format!("{:?}", c.client),
            format!("{client:?}"),
            last[0],
            last[1],
        );
        if (first[0] > 0) != painted || (second[0] > 0) != painted {
            failures.push(format!("{}: the picture was not settled: {seen:?}", c.name));
        }
        if !built && painted {
            failures.push(format!("{}: refused, and yet drawn", c.name));
        }
        if client != c.client {
            failures.push(format!("{}: the client: {client:?}, expected {:?}", c.name, c.client));
        }
        if last[0] < c.magenta {
            failures.push(format!("{}: {} magenta pixels of {}", c.name, last[0], c.magenta));
        }
        if c.green.is_some_and(|g| g != green) {
            failures.push(format!("{}: green seen: {green}", c.name));
        }
        // A window is found by its ID and gives it back, whatever the ID.
        if flags != "found=1 id=1" {
            failures.push(format!("{}: {flags}", c.name));
        }
    }
    eprintln!("took {:.0?}", start.elapsed());
    assert!(failures.is_empty(), "{failures:#?}");
}
