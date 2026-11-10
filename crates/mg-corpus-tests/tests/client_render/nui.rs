//! Readings of NUI windows in the client, for what the NUI Creator's
//! preview does not know (`docs/deferred.md`'s "Where the preview is
//! known to differ from the game" and NUI entries): each window opened
//! alone in its turn at a fixed place, its controls found on the client's
//! picture by draw lists of a color of their own clipped to them (a
//! filled rectangle larger than any control, `draw_list_scissor` on), so
//! that a control's box is the box of its color.
//!
//! - rows of buttons without widths beside a control with a width (an
//!   image, then a button, 100, 200 and 400 wide, above and below a row
//!   of one, two and three buttons): each control's width;
//! - a column of ten buttons 30 high with margins 2 and 5: each one's top
//!   edge (at 100%, and with `MG_NUI_SCALE=1.11` at 111%,
//!   `ui.unconstrain-scale`);
//! - labels bound to 1/3 at `number_precision` 16, 17, 20 and 30, and to
//!   0.5 and 2.5 at 0: read on the picture;
//! - a 16-bit TGA whose descriptor declares an alpha bit (`mg16.tga` in
//!   the override): the 32 levels of red in patches 8 pixels wide with the
//!   bit set, and the same in green with it clear, drawn at its size
//!   (`NUI_ASPECT_EXACT`);
//! - a window whose events go to the module's own event script
//!   (`event_function`): its button clicked (a click on the client's
//!   window), the module's OnNuiEvent logging what `NAME_OnEvent()`
//!   returned and the bind its handler sets, and another window's watch
//!   event beside it.
//!
//! What was read is in `nui_readings_in_the_client`'s comment.
//! Pictures in `target/test-output/client_reading_nui[_<scale>]/`.

use super::*;
use mg_module::build::{ScriptSelection, compile_scripts};
use mg_nui::{Action, Binding, Handler, Settings};
use serde_json::{Value as Json, json};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Seconds before the first window, and a window's turn.
const FIRST: f32 = 10.0;
const TURN: f32 = 6.0;

/// Added to the client's settings, as `client_nui.rs` has them: frames at
/// a steady 30 a second, not waiting on the off-screen display (waiting,
/// the picture of the window ran seconds behind the client).
const FRAMES: &str = "\t\t[graphics.video.vsync]\n\t\t\tmode = \"off\"\n\t[graphics.framelimiter]\n\t\tenabled = true\n\t\t[graphics.framelimiter.background]\n\t\t\ttarget-fps = 30\n\t\t[graphics.framelimiter.foreground]\n\t\t\ttarget-fps = 30\n\t\t[graphics.framelimiter.non-interactive]\n\t\t\ttarget-fps = 30\n";

/// The colors of the controls, in the order given: none of them is in the
/// game's interface or the area behind.
const COLORS: [[u8; 3]; 13] = [
    [255, 0, 255],
    [0, 255, 0],
    [0, 255, 255],
    [64, 0, 255],
    [255, 0, 128],
    [128, 0, 255],
    [0, 128, 255],
    [255, 128, 0],
    [0, 255, 128],
    [128, 255, 0],
    [255, 0, 0],
    [0, 0, 255],
    [255, 128, 255],
];

fn rgba(c: [u8; 3]) -> Json {
    json!({"r":c[0],"g":c[1],"b":c[2],"a":255})
}

/// `control` painted all over in the color `i`.
fn painted(mut control: Json, i: usize) -> Json {
    control["draw_list_scissor"] = json!(true);
    control["draw_list"] = json!([{"type":7,"enabled":true,"color":rgba(COLORS[i]),"fill":true,
        "line_thickness":1.0,"order":1,"render":0,"rect":{"x":0.0,"y":0.0,"w":4000.0,"h":4000.0}}]);
    control
}

/// One window of the readings: its name, its JUI and settings.
struct Win {
    name: String,
    window: Json,
    settings: Settings,
    /// How many colored controls it has.
    colors: usize,
}

fn win(name: &str, root: Json, w: f64, h: f64, colors: usize) -> Win {
    let mut window = mg_nui::window();
    window["title"] = name.into();
    window["geometry"] = json!({"x":40.0,"y":40.0,"w":w,"h":h});
    window["resizable"] = false.into();
    window["collapsed"] = false.into();
    window["root"] = root;
    Win { name: name.into(), window, settings: Settings::default(), colors }
}

/// The tag of the window `i`: a color drawn in a corner of its root.
fn tag(i: usize) -> [u8; 3] {
    [3 + 6 * i as u8, 77, 33]
}

/// The window whose tag is on `img`, if any.
fn tag_of(img: &Rgba) -> Option<usize> {
    let px = img.data.as_chunks::<4>().0;
    let w = img.width as usize;
    // (Four pixels of it in a row, not one of the area's.)
    (0..px.len().saturating_sub(4)).find_map(|k| {
        let p = px[k];
        let i = (p[0] as usize).checked_sub(3)?;
        let tagged = i % 6 == 0 && p[1] == 77 && p[2] == 33 && k % w + 4 < w;
        (tagged && px[k + 1..k + 4].iter().all(|q| q[..3] == p[..3])).then_some(i / 6)
    })
}

/// Each window's root tagged with its place in `wins`.
fn tagged(wins: &mut [Win]) {
    for (i, w) in wins.iter_mut().enumerate() {
        w.window["root"]["draw_list_scissor"] = json!(false);
        w.window["root"]["draw_list"] = json!([{"type":7,"enabled":true,"color":rgba(tag(i)),
            "fill":true,"line_thickness":1.0,"order":1,"render":0,
            "rect":{"x":0.0,"y":0.0,"w":6.0,"h":6.0}}]);
    }
}

/// `control` marked in the color `i` by a square of 8 at its top-left
/// corner, unclipped: where it starts. (Draw lists clipped to their
/// controls fill each one's cell, margins and all, and a window whose
/// last draw list is clipped may draw nothing, as `client_nui.rs` found.)
fn marked(mut control: Json, i: usize) -> Json {
    control["draw_list_scissor"] = json!(false);
    control["draw_list"] = json!([{"type":7,"enabled":true,"color":rgba(COLORS[i]),"fill":true,
        "line_thickness":1.0,"order":1,"render":0,"rect":{"x":0.0,"y":0.0,"w":8.0,"h":8.0}}]);
    control
}

fn button(i: usize) -> Json {
    painted(json!({"type":"button","label":"b","value":null}), i)
}

/// The windows of widths: a control (`image` or `button`) `width` wide
/// above (or below) a row of `n` buttons without widths.
fn widths() -> Vec<Win> {
    let mut out = Vec::new();
    for kind in ["image", "button"] {
        for first in [true, false] {
            for width in [100.0, 200.0, 400.0] {
                for n in 1..=3 {
                    let mut c = if kind == "image" {
                        json!({"type":"image","value":"mg16","image_aspect":5,"image_halign":0,
                            "image_valign":0,"height":30.0})
                    } else {
                        json!({"type":"button","label":"w","value":null})
                    };
                    c["width"] = json!(width);
                    let c = painted(c, 0);
                    let row =
                        json!({"type":"row","children":(1..=n).map(button).collect::<Vec<_>>()});
                    let children = if first { json!([c, row]) } else { json!([row, c]) };
                    let name = format!(
                        "w_{}{}_{}_{n}",
                        &kind[..1],
                        if first { "a" } else { "b" },
                        width as u32
                    );
                    out.push(win(
                        &name,
                        json!({"type":"col","children":children}),
                        600.0,
                        200.0,
                        n + 1,
                    ));
                }
            }
        }
    }
    out
}

/// The windows of margins: ten buttons 30 high, each with `margin`.
fn margins() -> Vec<Win> {
    [2.0, 5.0]
        .map(|m| {
            let children: Vec<Json> = (0..10)
                .map(|i| {
                    let mut b = marked(json!({"type":"button","label":"b","value":null}), i);
                    b["height"] = json!(30.0);
                    b["width"] = json!(150.0);
                    b["margin"] = json!(m);
                    b
                })
                .collect();
            win(
                &format!("m_{}", m as u32),
                json!({"type":"col","children":children}),
                300.0,
                560.0,
                10,
            )
        })
        .into()
}

/// The window of numbers: labels bound to numbers at a precision.
fn numbers() -> Win {
    // (The third batch's: ties at 2 places, each exact in a 32-bit float.)
    let cases: [(&str, f64, i64); 9] = [
        ("p16", 1.0 / 3.0, 16),
        ("p17", 1.0 / 3.0, 17),
        ("p20", 1.0 / 3.0, 20),
        ("p30", 1.0 / 3.0, 30),
        ("h0", 0.5, 0),
        ("t0", 2.5, 0),
        ("a2", 42.125, 2),
        ("n2", -42.125, 2),
        ("e2", 0.125, 2),
    ];
    let rows: Vec<Json> = cases
        .iter()
        .map(|(bind, _, p)| {
            json!({"type":"row","height":30.0,"children":[
                {"type":"label","value":format!("{bind}:"),"width":60.0},
                {"type":"label","value":{"bind":bind,"number_flags":0,"number_precision":p,"text_flags":0},
                    "width":480.0}]})
        })
        .collect();
    let mut w = win("numbers", json!({"type":"col","children":rows}), 600.0, 400.0, 0);
    for (bind, v, _) in cases {
        w.settings.bindings.insert(bind.into(), Binding { value: json!(v), ..Default::default() });
    }
    w
}

/// The window of the 16-bit TGA, at its size.
fn tga() -> Win {
    let image = json!({"type":"image","value":"mg16","image_aspect":3,"image_halign":0,
        "image_valign":0,"width":256.0,"height":64.0});
    win("tga16", json!({"type":"col","children":[image]}), 300.0, 140.0, 0)
}

/// The window whose events go to the module's event script: a button
/// `go` whose click sets the bind `said`, shown in a label.
fn evented() -> Win {
    let root = json!({"type":"col","children":[
        marked(json!({"type":"button","id":"go","label":"Go","value":null,"width":150.0,"height":40.0}), 0),
        {"type":"label","value":{"bind":"said"},"height":30.0}]});
    let mut w = win("mgev", root, 300.0, 160.0, 1);
    w.settings.event_function = true;
    w.settings
        .bindings
        .insert("said".into(), Binding { value: json!("before"), ..Default::default() });
    w.settings.handlers.push(Handler::new(
        "click",
        "go",
        Action::Set { bind: "said".into(), value: json!("clicked") },
    ));
    w
}

/// A 16-bit TGA, 256 × 64, top-left first, its descriptor declaring one
/// alpha bit: a white band (rows 0 to 7, the bit set), then red level `c`
/// in columns 8c to 8c + 7 (rows 8 to 35, the bit set), then green level
/// `c` with the bit clear (rows 36 to 63).
pub(super) fn tga16() -> Vec<u8> {
    let (w, h) = (256u16, 64u16);
    let mut out = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    out.extend(w.to_le_bytes());
    out.extend(h.to_le_bytes());
    out.extend([16, 0x21]);
    for y in 0..h {
        for x in 0..w {
            let c = x / 8;
            let px: u16 = if y < 8 {
                0x8000 | 0x7fff
            } else if y < 36 {
                0x8000 | (c << 10)
            } else {
                c << 5
            };
            out.extend(px.to_le_bytes());
        }
    }
    out
}

/// The module's OnClientEnter: each window opened in its turn by its
/// opener and closed at the end of it, `MG_SHOW <name>` logged; the
/// evented window and another left open at the end.
fn enter_script(wins: &[Win]) -> String {
    let mut s = String::from("#include \"nw_inc_nui\"\n");
    for w in wins {
        s += &format!("#include \"{}_o\"\n", w.name);
    }
    s += r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
void Close(object pc)
{
    int t = NuiGetNthWindow(pc, 0);
    if (t != 0) NuiDestroy(pc, t);
}
void Other(object pc)
{
    json root = NuiCol(JsonArray());
    json w = NuiWindow(root, JsonString("other"), NuiRect(400.0, 40.0, 200.0, 100.0), JsonBool(FALSE), JsonBool(FALSE), JsonBool(FALSE), JsonBool(FALSE), JsonBool(TRUE));
    int t = NuiCreate(pc, w, "other");
    NuiSetBindWatch(pc, t, "seen", TRUE);
    NuiSetBind(pc, t, "seen", JsonInt(1));
    NuiSetBind(pc, t, "seen", JsonInt(2));
}
void Open(object pc, int n)
{
"#;
    let last = wins.len() - 1;
    for (i, w) in wins.iter().enumerate() {
        let name = &w.name;
        s += &format!("    if (n == {i})\n    {{\n        Open_{name}(pc);\n");
        if i == last {
            s += "        Other(pc);\n";
        }
        s += &format!("        Log(\"MG_SHOW {name} .\");\n    }}\n");
    }
    // (Closed a second before the next opens: opened in the same moment,
    // every other window was placed elsewhere.)
    s += "}\n\nvoid Show(object pc, int n)\n{\n    Close(pc);\n    DelayCommand(1.0, Open(pc, n));\n}\n";
    s += "\nvoid main()\n{\n    object pc = GetEnteringObject();\n    if (!GetIsPC(pc)) return;\n";
    s += "    ApplyEffectToObject(DURATION_TYPE_PERMANENT, EffectVisualEffect(VFX_DUR_CUTSCENE_INVISIBILITY), pc);\n";
    for i in 0..wins.len() {
        s += &format!("    DelayCommand({:.1}, Show(pc, {i}));\n", FIRST + TURN * i as f32);
    }
    s += "}\n";
    s
}

/// The module's OnNuiEvent: what `mgev_OnEvent()` returns for each event,
/// with the window's ID and the bind `said`.
const ON_EVENT: &str = r#"#include "mgev_e"
void main()
{
    object pc = NuiGetEventPlayer();
    int t = NuiGetEventWindow();
    int r = mgev_OnEvent();
    WriteTimestampedLogEntry("MG_EV " + NuiGetEventType() + " " + NuiGetWindowId(pc, t) + " "
        + NuiGetEventElement() + " returned=" + IntToString(r)
        + " said=" + JsonDump(NuiGetBind(pc, t, "said")));
}
"#;

/// The boxes of each color on `img`: (left, top, right, bottom), inclusive.
fn boxes(img: &Rgba, n: usize) -> Vec<Option<[u32; 4]>> {
    let mut out: Vec<Option<[u32; 4]>> = vec![None; n];
    for y in 0..img.height {
        for x in 0..img.width {
            let i = ((y * img.width + x) * 4) as usize;
            let p = &img.data[i..i + 3];
            for (k, c) in COLORS.iter().take(n).enumerate() {
                if (0..3).all(|j| p[j].abs_diff(c[j]) <= 3) {
                    let b = out[k].get_or_insert([x, y, x, y]);
                    *b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
                }
            }
        }
    }
    out
}

fn winshot(path: &Path) -> Option<Rgba> {
    Command::new("python3")
        .arg(common::repo().join("tools/aurora/xdrive.py"))
        .args(["winshot", "Neverwinter Nights: Enhanced Edition"])
        .arg(path)
        .status()
        .is_ok_and(|s| s.success())
        .then(|| common::read_png(path))
}

/// A click at (x, y) of the client's window, the pointer moved there
/// first (a bare click is not taken).
pub(super) fn click_in_window(x: u32, y: u32) {
    let xdrive = common::repo().join("tools/aurora/xdrive.py");
    let out = Command::new("python3").arg(&xdrive).arg("windows").output().unwrap();
    let listed = String::from_utf8_lossy(&out.stdout).into_owned();
    let line = listed.lines().find(|l| l.contains("Neverwinter Nights")).unwrap();
    let at = line.split('\t').nth(1).unwrap().split(' ').next().unwrap();
    let (ox, oy) = at.split_once(',').unwrap();
    let (px, py) = (ox.parse::<u32>().unwrap() + x, oy.parse::<u32>().unwrap() + y);
    let run = |args: &[String]| {
        Command::new("python3").arg(&xdrive).args(args).status().unwrap();
        std::thread::sleep(Duration::from_millis(300));
    };
    run(&["move".into(), (px - 20).to_string(), (py - 10).to_string()]);
    run(&["move".into(), px.to_string(), py.to_string()]);
    run(&["press".into(), px.to_string(), py.to_string()]);
    run(&["release".into(), px.to_string(), py.to_string()]);
}

/// The readings (seen on 2026-10-29, 89.8193.37; see the entries).
#[test]
#[ignore]
fn nui_readings_in_the_client() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let scale = std::env::var("MG_NUI_SCALE").ok();
    let dir = scratch_dir(&match &scale {
        Some(s) => format!("client_reading_nui_{s}"),
        None => "client_reading_nui".into(),
    });
    let over = dir.join("user/override");
    std::fs::create_dir_all(&over).unwrap();
    std::fs::write(over.join("mg16.tga"), tga16()).unwrap();
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();

    let mut wins = Vec::new();
    wins.extend(widths());
    wins.push(numbers());
    wins.push(tga());
    wins.extend(margins());
    wins.push(evented());
    // (`MG_NUI_ONLY`: the windows named, in that order. A window of ten
    // buttons 30 high stops the client's picture once shown: put it last.)
    if let Ok(only) = std::env::var("MG_NUI_ONLY") {
        let mut all: Vec<Option<Win>> = wins.into_iter().map(Some).collect();
        wins = only
            .split(',')
            .filter_map(|n| {
                all.iter_mut().find(|w| w.as_ref().is_some_and(|w| w.name == n))?.take()
            })
            .collect();
    }
    tagged(&mut wins);

    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(&game, "MgNui", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Scene".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 4,
        height: 4,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let key = |n: &str, t| ResKey::parse(n, t).unwrap();
    for w in &wins {
        let opener = mg_nui::opener_source(&w.name, &w.window, &w.settings).unwrap();
        m.set(key(&format!("{}_o", w.name), ResType::NSS), opener.into_bytes());
        if w.settings.event_function {
            let events = mg_nui::event_source(&w.name, &w.settings).unwrap();
            m.set(key(&format!("{}_e", w.name), ResType::NSS), events.into_bytes());
        }
    }
    m.set(key("mg_enter", ResType::NSS), enter_script(&wins).into_bytes());
    m.set(key("mg_nuiev", ResType::NSS), ON_EVENT.as_bytes().to_vec());
    for r in compile_scripts(&mut m, &game.resman, ScriptSelection::All) {
        // (The event include has no main: it compiles to nothing.)
        if r.script.to_string().starts_with("mgev_e") {
            continue;
        }
        assert!(r.result.is_ok(), "{}: {:?}", r.script, r.result);
    }
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    info.root.write(&ifo::MOD_ON_NUI_EVENT, ResRef::from_str("mg_nuiev").unwrap());
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgNui.mod"))).unwrap();
    let mut settings = format!("{SETTINGS}{FRAMES}");
    if let Some(s) = &scale {
        settings += &format!("[ui]\n\tscale = {s}\n\tunconstrain-scale = true\n");
    }
    std::fs::write(dir.join("user/settings.tml"), settings).unwrap();

    let log = dir.join("user/logs/nwclientLog1.txt");
    let read = || String::from_utf8_lossy(&std::fs::read(&log).unwrap_or_default()).into_owned();
    let mut child = Command::new(common::repo().join("tools/nwclient/run-client.sh"))
        .arg(&dir)
        .args(["+TestNewModule", "MgNui"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let mut done = vec![false; wins.len()];
    let limit = Duration::from_secs((FIRST + TURN * wins.len() as f32) as u64 + 300);
    let mut clicked = false;
    let mut k = 0;
    while start.elapsed() < limit && !(clicked || done.iter().all(|d| *d)) {
        std::thread::sleep(Duration::from_millis(200));
        // The evented window, the last, stays open: its button clicked
        // once it is (the click reaches the client as it is, whatever the
        // picture shows).
        let path = dir.join(format!("shot_{k}.png"));
        let Some(img) = winshot(&path) else { continue };
        k += 1;
        // The window on the picture, by its tag: pictures run seconds
        // behind the client.
        let Some(i) = tag_of(&img).filter(|&i| i < wins.len() && !done[i]) else {
            // (The last picture of none kept, to look at.)
            let _ = std::fs::rename(&path, dir.join("last.png"));
            continue;
        };
        let name = &wins[i].name;
        std::fs::rename(&path, dir.join(format!("{name}.png"))).unwrap();
        done[i] = true;
        let found = boxes(&img, wins[i].colors);
        let text: Vec<String> = found
            .iter()
            .map(|b| match b {
                Some([l, t, r, b]) => format!("x {l} y {t} w {} h {}", r - l + 1, b - t + 1),
                None => "none".into(),
            })
            .collect();
        eprintln!("{name}: {}", text.join("; "));
        if wins[i].settings.event_function
            && let Some([l, t, ..]) = found[0]
        {
            click_in_window(l + 75, t + 20);
            clicked = true;
            std::thread::sleep(Duration::from_secs(6));
            winshot(&dir.join(format!("{name}_clicked.png")));
        }
    }
    std::thread::sleep(Duration::from_secs(2));
    let _ = child.kill();
    let _ = child.wait();
    for line in read().lines().filter(|l| l.contains("MG_EV")) {
        eprintln!("{line}");
    }
    eprintln!("pictures: {}", dir.display());
    assert!(done.iter().all(|d| *d), "not every window was seen: {done:?}");
}

/// The mouse button pressed at (x, y) of the client's window and held
/// `secs` seconds (`xdrive.py hold`).
fn hold_in_window(x: u32, y: u32, secs: f32) {
    let xdrive = common::repo().join("tools/aurora/xdrive.py");
    let out = Command::new("python3").arg(&xdrive).arg("windows").output().unwrap();
    let listed = String::from_utf8_lossy(&out.stdout).into_owned();
    let line = listed.lines().find(|l| l.contains("Neverwinter Nights")).unwrap();
    let at = line.split('\t').nth(1).unwrap().split(' ').next().unwrap();
    let (ox, oy) = at.split_once(',').unwrap();
    let (px, py) = (ox.parse::<u32>().unwrap() + x, oy.parse::<u32>().unwrap() + y);
    let ok = Command::new("python3")
        .arg(&xdrive)
        .args(["hold", &px.to_string(), &py.to_string(), &secs.to_string()])
        .status()
        .unwrap();
    assert!(ok.success(), "the press");
}

/// The width of each scrolled control of [`scrolled`].
const SCROLLED_WIDTH: u32 = 150;

/// A scrolled control of [`scrolled`]: its name, its JUI and its height.
struct Scrolled {
    name: &'static str,
    control: Json,
    height: u32,
}

/// A list of 40 rows `row` high, `height` high.
fn scrolled_list(name: &'static str, row: f64, height: u32) -> Scrolled {
    let mut list = mg_nui::template("list");
    list["row_template"] = json!([[{"type":"label","label":null,
        "value":{"bind":"rows"},"text_halign":0,"text_valign":1},150.0,true]]);
    list["row_count"] = 40.into();
    list["row_height"] = json!(row);
    list["scrollbars"] = 2.into();
    list["width"] = f64::from(SCROLLED_WIDTH).into();
    list["height"] = f64::from(height).into();
    Scrolled { name, control: list, height }
}

/// A group of 40 labels `row` high, `height` high.
fn scrolled_group(name: &'static str, row: f64, height: u32) -> Scrolled {
    let labels: Vec<Json> = (0..40)
        .map(|i| {
            json!({"type":"label","label":null,"value":format!("Group {i}"),
            "text_halign":0,"text_valign":1,"height":row})
        })
        .collect();
    let group = json!({"type":"group","label":null,"value":null,"border":true,"scrollbars":2,
        "children":[{"type":"col","label":null,"value":null,"children":labels}],
        "width":f64::from(SCROLLED_WIDTH),"height":f64::from(height)});
    Scrolled { name, control: group, height }
}

/// A text of 40 lines, `height` high.
fn scrolled_text(name: &'static str, height: u32) -> Scrolled {
    let lines: Vec<String> = (0..40).map(|i| format!("Text {i}")).collect();
    let mut text = mg_nui::template("text");
    text["value"] = lines.join("\n").into();
    text["scrollbars"] = 2.into();
    text["width"] = f64::from(SCROLLED_WIDTH).into();
    text["height"] = f64::from(height).into();
    Scrolled { name, control: text, height }
}

/// The window of [`reading_scrollbar_end_buttons`] and its controls, side
/// by side, each marked at its top-left corner (colors 0, 1...):
/// `kinds` (a list of rows 30 high, a group of labels 30 high, a text, a
/// list of rows 50 high, a group of labels 50 high, each 300 high) or
/// `heights` (groups 150, 300 and 600 high, then a text 300 high).
fn scrolled(set: &str) -> (Win, Vec<(&'static str, u32)>) {
    let controls = match set {
        "heights" => vec![
            scrolled_group("group150", 30.0, 150),
            scrolled_group("group300", 30.0, 300),
            scrolled_group("group600", 30.0, 600),
            scrolled_text("text", 300),
        ],
        _ => vec![
            scrolled_list("list", 30.0, 300),
            scrolled_group("group", 30.0, 300),
            scrolled_text("text", 300),
            scrolled_list("list50", 50.0, 300),
            scrolled_group("group50", 50.0, 300),
        ],
    };
    let tallest = controls.iter().map(|c| c.height).max().unwrap_or(0);
    let shown = controls.iter().map(|c| (c.name, c.height)).collect();
    let n = controls.len();
    let children: Vec<Json> =
        controls.into_iter().enumerate().map(|(i, c)| marked(c.control, i)).collect();
    let root = json!({"type":"row","label":null,"value":null,"children":children});
    let width = (SCROLLED_WIDTH as usize + 5) * n + 40;
    let mut win = win("mgscroll", root, width as f64, f64::from(tallest) + 80.0, n);
    let rows: Vec<String> = (0..40).map(|i| format!("Row {i}")).collect();
    win.settings
        .bindings
        .insert("rows".into(), Binding { value: json!(rows), ..Default::default() });
    (win, shown)
}

/// How far the pixels in `area` (left, top, right, bottom) of `after`
/// are those of `before` moved up: the shift (up to the area's height
/// less 20) that matches them best, and how well (the mean difference a
/// pixel).
fn moved_up(before: &Rgba, after: &Rgba, area: [u32; 4]) -> (u32, f64) {
    let px = |img: &Rgba, x: u32, y: u32| {
        let i = ((y * img.width + x) * 4) as usize;
        [img.data[i] as i32, img.data[i + 1] as i32, img.data[i + 2] as i32]
    };
    let [l, t, r, b] = area;
    let mut best = (0, f64::MAX);
    for shift in 0..(b - t).saturating_sub(20) {
        let (mut sum, mut n) = (0i64, 0i64);
        for y in t..b - shift {
            for x in l..r {
                let (p, q) = (px(before, x, y + shift), px(after, x, y));
                sum += (0..3).map(|k| (p[k] - q[k]).abs() as i64).sum::<i64>();
                n += 1;
            }
        }
        let mean = sum as f64 / n.max(1) as f64;
        if mean < best.1 {
            best = (shift, mean);
        }
    }
    best
}

/// What a click on a scrollbar's end button scrolls by in the client (the
/// review's N9: the NUI Creator's preview scrolled a group 20 points, a
/// text a font's height, and took a list's as a click on the track's
/// end): [`scrolled`]'s controls (`MG_SCROLL_SET`), each one's down
/// button pressed and held for each of `MG_HOLDS` seconds (default 0.3,
/// 0.3, 1, 2) and its up button for the first, a picture after each
/// (`<name>_<n>.png`), and how far its content stands from where it
/// started (`before.png`), by matching pixels, at `MG_SCROLL_FPS` frames
/// a second (default 10). The end button is found `MG_END_BUTTON`
/// (`dx,dy`, default 15,15) up and left of the control's bottom-right
/// corner.
///
/// Seen (2026-11-10): no step a click. An end button scrolls each frame it
/// is held (Nuklear's repeating button), and a press and release between
/// two frames scrolls nothing. Groups 150, 300 and 600 high scrolled 1, 2
/// and 5 pixels a frame down and 2, 3 and 6 up (a hundredth of the bar,
/// the offset kept in whole pixels: 30, 60 and 150 pixels a second at 30
/// frames a second, 20 at 10 for the one 300 high), whatever the rows'
/// height; a text 300 high 2 a frame down and 3 up, as the group of its
/// height (some of its presses moved nothing: not settled why); a list a
/// whole row (row and spacing, 34) at a time, about 4 rows a second held
/// at 30 frames a second. The pictures run seconds behind the client: the
/// offsets are read against `before.png`, and a run's later presses are
/// the surer.
#[test]
#[ignore]
fn reading_scrollbar_end_buttons() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let set = std::env::var("MG_SCROLL_SET").unwrap_or_else(|_| "kinds".into());
    let dir = scratch_dir(&format!("client_reading_nui_scroll_{set}"));
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let (win, shown) = scrolled(&set);
    let mut wins = vec![win];
    tagged(&mut wins);
    let w = &wins[0];
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(&game, "MgNui", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Scene".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 4,
        height: 4,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let key = |n: &str, t| ResKey::parse(n, t).unwrap();
    let opener = mg_nui::opener_source(&w.name, &w.window, &w.settings).unwrap();
    m.set(key("mgscroll_o", ResType::NSS), opener.into_bytes());
    let enter = format!(
        "#include \"mgscroll_o\"\nvoid Log(string s) {{ WriteTimestampedLogEntry(s); }}\n\
         void Open(object pc) {{ Open_mgscroll(pc); Log(\"MG_SHOW mgscroll .\"); }}\n\
         void main()\n{{\n    object pc = GetEnteringObject();\n    if (!GetIsPC(pc)) return;\n    \
         ApplyEffectToObject(DURATION_TYPE_PERMANENT, \
         EffectVisualEffect(VFX_DUR_CUTSCENE_INVISIBILITY), pc);\n    \
         DelayCommand({FIRST:.1}, Open(pc));\n}}\n"
    );
    m.set(key("mg_enter", ResType::NSS), enter.into_bytes());
    for r in compile_scripts(&mut m, &game.resman, ScriptSelection::All) {
        assert!(r.result.is_ok(), "{}: {:?}", r.script, r.result);
    }
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgNui.mod"))).unwrap();
    let fps: u32 = std::env::var("MG_SCROLL_FPS").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let holds: Vec<f32> = std::env::var("MG_HOLDS")
        .ok()
        .map(|v| v.split(',').filter_map(|h| h.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![0.3, 0.3, 1.0, 2.0]);
    let frames = FRAMES.replace("target-fps = 30", &format!("target-fps = {fps}"));
    std::fs::write(dir.join("user/settings.tml"), format!("{SETTINGS}{frames}")).unwrap();
    let (dx, dy) = std::env::var("MG_END_BUTTON")
        .ok()
        .and_then(|v| {
            let (x, y) = v.split_once(',')?;
            Some((x.trim().parse::<u32>().ok()?, y.trim().parse::<u32>().ok()?))
        })
        .unwrap_or((15, 15));
    eprintln!("{fps} frames a second, the down button held {holds:?} s");

    let mut child = Command::new(common::repo().join("tools/nwclient/run-client.sh"))
        .arg(&dir)
        .args(["+TestNewModule", "MgNui"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let begun = Instant::now();
    let mut found = None;
    while begun.elapsed() < Duration::from_secs(240) && found.is_none() {
        std::thread::sleep(Duration::from_millis(500));
        let Some(img) = winshot(&dir.join("last.png")) else { continue };
        if tag_of(&img) == Some(0) {
            let marks = boxes(&img, shown.len());
            if marks.iter().all(Option::is_some) {
                found = Some(marks);
            }
        }
    }
    let Some(marks) = found else {
        let _ = child.kill();
        let _ = child.wait();
        panic!("the window was not seen (see {})", dir.display());
    };
    // (The picture runs behind the client: settle, then the one before.)
    std::thread::sleep(Duration::from_secs(3));
    let start = winshot(&dir.join("before.png")).unwrap();
    for (k, &(name, height)) in shown.iter().enumerate() {
        let [l, t, ..] = marks[k].unwrap();
        // (The content beside the scrollbar, under the corner's mark.)
        let area = [l + 12, t + 12, l + SCROLLED_WIDTH - 30, t + height - 12];
        let (right, bottom) = (l + SCROLLED_WIDTH - dx, t + height - dy);
        let mut presses: Vec<(u32, u32, f32)> = holds.iter().map(|&h| (right, bottom, h)).collect();
        presses.push((right, t + dy, holds[0]));
        for (n, (x, y, hold)) in presses.into_iter().enumerate() {
            hold_in_window(x, y, hold);
            // (The picture runs behind: taken a few seconds after.)
            std::thread::sleep(Duration::from_secs(4));
            let after = winshot(&dir.join(format!("{name}_{n}.png"))).unwrap();
            let (at, miss) = moved_up(&start, &after, area);
            eprintln!(
                "{name}: press {n} held {hold} s: {at} px from the start (mean difference {miss:.2})"
            );
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    eprintln!("pictures: {}", dir.display());
}
