//! When the game client draws an area transition trigger's outline, and
//! what in the trigger's GIT struct decides it (a builder's report:
//! transition triggers drawn in Moonglow and linked by hand were
//! invisible in the game until their polygons were redrawn, and still
//! worked). A row of transition triggers ahead of the player, each one
//! field away from the first; the client's window shot with nothing
//! held, with the pointer over each trigger, and with Tab held. The
//! pictures go to `target/test-output/client_reading_transitions_*`.
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --test client_render -- --ignored transition_outlines --nocapture`.

use super::readings::Reading;
use super::*;
use mg_gff::Struct;
use mg_module::instances::{Placement, Placing, instance};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Where the player enters (looking north), metres.
const PC: [f32; 2] = [20.0, 14.0];
/// The row's south edge, its triggers' depth and width, the gap between
/// them, metres.
const ROW_Y: f32 = 19.0;
const DEPTH: f32 = 2.4;
const WIDTH: f32 = 2.0;
const GAP: f32 = 0.6;
/// The pointer's sweep down the window (pixels): from, to, step.
const SWEEP_FROM: i32 = 120;
const SWEEP_TO: i32 = 560;
const SWEEP: usize = 20;
/// Where a row's trigger is clicked to walk through it: the window row
/// where the sweeps light them.
const CLICK_Y: i32 = 280;
/// Where the ground beyond a row's trigger is clicked to cross it.
const THROUGH_Y: i32 = 200;
/// Where the triggers lead (a waypoint in the same area).
const DEST: [f32; 2] = [8.0, 34.0];

/// One trigger of the row: what it is called in the pictures and how it
/// differs from an anticlockwise `newtransition` instance.
struct Variant {
    name: &'static str,
    change: fn(&mut Struct),
}

/// Reverses a trigger's outline (anticlockwise to clockwise), the stand
/// point kept where it is.
fn clockwise(s: &mut Struct) {
    let mut points = s.list("Geometry").unwrap().to_vec();
    points.reverse();
    s.set("Geometry", Value::List(points));
}

/// Sets every point's `PointZ` to `z`.
fn point_z(s: &mut Struct, z: f32) {
    let mut points = s.list("Geometry").unwrap().to_vec();
    for p in &mut points {
        p.set("PointZ", Value::Float(z));
    }
    s.set("Geometry", Value::List(points));
}

/// A `newtransition` instance at `x` along the row (its south-west corner
/// its position, the outline anticlockwise from there, on the flat
/// ground at Aurora's 0.025), leading to `WP_TT_DEST`.
fn transition_at(game: &GameData, x: f32, tag: &str) -> Struct {
    let bp = Gff::read(&game.resman.get_named("newtransition", ResType::UTT).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let at = Placement { position: [x, ROW_Y, 0.0], rotation: 0.0 };
    let outline =
        [[0.0, 0.0, 0.025], [WIDTH, 0.0, 0.025], [WIDTH, DEPTH, 0.025], [0.0, DEPTH, 0.025]];
    let mut t = instance(&placing, ResType::UTT, &bp.root, at, &outline).unwrap();
    t.set("Tag", Value::String(tag.as_bytes().to_vec()));
    t.set("LinkedTo", Value::String(b"WP_TT_DEST".to_vec()));
    t.set("LinkedToFlags", Value::Byte(2));
    t
}

/// Makes the trigger Aurora's `TT_A1` as Aurora painted and linked it
/// (the oracle's capture `transition-triggers/07-s2-both.mod`), led to
/// `WP_TT_DEST` instead; unchanged without the capture.
fn aurora(s: &mut Struct) {
    let Some(path) = mg_testkit::aurora_capture("transition-triggers/07-s2-both.mod") else {
        eprintln!("no Aurora capture: the control is Moonglow's");
        return;
    };
    let m = mg_module::Module::open(&path).unwrap();
    let git = m.gff(&ResKey::parse("alpha", ResType::GIT).unwrap()).unwrap().unwrap();
    let a1 =
        git.root.list("TriggerList").unwrap().iter().find(|t| t.string("Tag") == Some(b"TT_A1"));
    *s = a1.expect("TT_A1 in the capture").clone();
    s.set("LinkedTo", Value::String(b"WP_TT_DEST".to_vec()));
}

/// The client's window position of ground point `p` for the entry
/// script's camera (`SetCameraFacing(90, 12, 45)`, fitted: `FIT_*`) with
/// the player at `pc`, in a `size` window.
fn on_screen(pc: Vec3, p: Vec3, size: (u32, u32)) -> (i32, i32) {
    let camera = fitted_camera(pc, FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let clip = camera.projection(size.0 as f32 / size.1 as f32) * camera.view() * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let x = (ndc.x + 1.0) / 2.0 * size.0 as f32;
    let y = (1.0 - ndc.y) / 2.0 * size.1 as f32;
    (x.round() as i32, y.round() as i32)
}

/// What the client is made to do once the scene is ready, one step a
/// line: `move X Y` (window coordinates), `keydown K`, `keyup K`,
/// `sleep S`, `shot NAME` (the window, as `NAME.png` in the scratch
/// directory). Run by Python with Xlib (as `tools/aurora/xdrive.py`).
const DRIVER: &str = r#"
import sys, time
sys.path.insert(0, sys.argv[1])
import xdrive
from Xlib import X
from Xlib.ext import xtest
d = xdrive.d
win = [w for w in xdrive.viewable_windows() if "Neverwinter Nights" in w[1]][0]
ox, oy = win[2], win[3]
for line in open(sys.argv[2]):
    p = line.split()
    if not p:
        continue
    if p[0] == "move":
        xdrive.move(ox + int(p[1]), oy + int(p[2]))
    elif p[0] in ("keydown", "keyup"):
        code = xdrive.keycode(p[1])
        xtest.fake_input(d, X.KeyPress if p[0] == "keydown" else X.KeyRelease, code)
        d.sync()
    elif p[0] == "click":
        xdrive.click(ox + int(p[1]), oy + int(p[2]))
    elif p[0] == "sleep":
        time.sleep(float(p[1]))
    elif p[0] == "shot":
        xdrive.winshot("Neverwinter Nights", sys.argv[3] + "/" + p[1] + ".png")
    elif p[0] == "sweep":
        # sweep NAME X1 X2 DX Y1 Y2 DY: the pointer over each point of the
        # grid; the picture with the most blue pixels kept as NAME.png and
        # where it was, with its count, in NAME.txt.
        from PIL import Image
        x1, x2, dx, y1, y2, dy = map(int, p[2:8])
        best = (-1, 0, 0, None)
        lit_at = []
        for x in range(x1, x2 + 1, dx):
            for y in range(y1, y2 + 1, dy):
                xdrive.move(ox + x, oy + y)
                time.sleep(0.1)
                xdrive.move(ox + x + 1, oy + y)
                time.sleep(0.3)
                w = win[0]
                g = w.get_geometry()
                raw = w.get_image(0, 0, g.width, g.height, X.ZPixmap, 0xFFFFFFFF)
                img = Image.frombytes("RGB", (g.width, g.height), raw.data, "raw", "BGRX")
                scene = img.crop((0, 0, g.width * 9 // 10, g.height * 8 // 10))
                n = sum(1 for (r, gg, b) in scene.getdata() if b - (r + gg) / 2 > 40)
                if n > best[0]:
                    best = (n, x, y, img)
                if n > 200:
                    lit_at.append((x, y))
        best[3].save(sys.argv[3] + "/" + p[1] + ".png")
        open(sys.argv[3] + "/" + p[1] + ".txt", "w").write("%d %d %d\n" % best[:3])
    elif p[0] == "clicklit":
        # The lit point nearest the middle of the last sweep's lit points clicked (the
        # highlight outlasts the pointer a moment, so the most lit
        # picture may be one step past it); the given point if none lit.
        if lit_at:
            xs = sorted(x for x, _ in lit_at)
            ys = sorted(y for _, y in lit_at)
            mx, my = xs[len(xs) // 2], ys[len(ys) // 2]
            # (A lit point itself, the nearest that middle: a thin
            # strip's middle may be off it.)
            x, y = min(lit_at, key=lambda q: (q[0] - mx) ** 2 + (q[1] - my) ** 2)
            xdrive.click(ox + x, oy + y)
        else:
            xdrive.click(ox + int(p[1]), oy + int(p[2]))
"#;

/// Runs the client on the module `MgScene` of `dir`, waits for
/// `MG_READY`, runs `steps` ([`DRIVER`]), waits `after` (the entry
/// script's later lines), and stops the client. Whether it got there.
fn client_driven(dir: &Path, steps: &str, after: Duration) -> bool {
    let log = dir.join("user/logs/nwclientLog1.txt");
    let _ = std::fs::remove_file(&log);
    std::fs::write(dir.join("driver.py"), DRIVER).unwrap();
    std::fs::write(dir.join("steps.txt"), steps).unwrap();
    let mut child = Command::new(common::repo().join("tools/nwclient/run-client.sh"))
        .arg(dir)
        .args(["+TestNewModule", "MgScene"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    eprintln!("the client's sandbox: pid {}", child.id());
    let start = Instant::now();
    let mut ready = false;
    while start.elapsed() < Duration::from_secs(180) {
        if std::fs::read_to_string(&log).is_ok_and(|l| l.contains("MG_READY")) {
            ready = true;
            break;
        }
        if child.try_wait().is_ok_and(|s| s.is_some()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    if ready {
        std::thread::sleep(Duration::from_secs(2));
        let ok = Command::new("python3")
            .arg(dir.join("driver.py"))
            .arg(common::repo().join("tools/aurora"))
            .arg(dir.join("steps.txt"))
            .arg(dir)
            .status()
            .is_ok_and(|s| s.success());
        ready &= ok;
        std::thread::sleep(after);
    }
    let _ = child.kill();
    let _ = child.wait();
    ready
}

/// The client's settings: the tests' ([`SETTINGS`]) with the game's own
/// highlighting (`graphics.hilite`) left at its default, on.
fn settings() -> String {
    let off = "\t[graphics.hilite]\n\t\tenabled = false\n";
    assert!(SETTINGS.contains(off));
    SETTINGS.replace(off, "")
}

/// Shoots a row of `variants` (west to east) and returns the scratch
/// directory and each trigger's middle on screen. With `walk`, the player
/// then walks to the middle of trigger `walk` and the log says where it
/// ended (`MG_AFTER`).
fn shoot_row(
    name: &str,
    variants: &[Variant],
    walk: Option<(usize, i32)>,
) -> (PathBuf, Vec<(i32, i32)>) {
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new(&format!("transitions_{name}"), "ttr01", light, &[]);
    // (Each made at x 0 and changed, then moved to its place along the
    // row by its own width: the Aurora trigger is wider.)
    let mut made = Vec::new();
    for v in variants {
        let mut t = transition_at(&r.game, 0.0, &format!("TT_{}", v.name.to_uppercase()));
        (v.change)(&mut t);
        let xs: Vec<f32> = (t.list("Geometry").unwrap().iter())
            .map(|p| p.float("PointX").unwrap_or(0.0))
            .collect();
        let (low, high) = xs.iter().fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
        made.push((t, low, high));
    }
    let width: f32 = made.iter().map(|(_, low, high)| high - low + GAP).sum::<f32>() - GAP;
    let mut west = PC[0] - width / 2.0;
    let mut triggers = Vec::new();
    let mut middles = Vec::new();
    let pc = Vec3::new(PC[0], PC[1], 0.0);
    for (mut t, low, high) in made {
        let x = west - low;
        west += high - low + GAP;
        t.set("XPosition", Value::Float(x));
        t.set("YPosition", Value::Float(ROW_Y));
        // (The pointer over the outline as drawn: at its points' height.)
        let points = t.list("Geometry").unwrap();
        let n = points.len() as f32;
        let sum = points.iter().fold(Vec3::ZERO, |a, p| {
            let f = |l| p.float(l).unwrap_or(0.0);
            a + Vec3::new(f("PointX"), f("PointY"), f("PointZ"))
        });
        let middle = Vec3::new(x, ROW_Y, t.float("ZPosition").unwrap_or(0.0)) + sum / n;
        middles.push((middle, on_screen(pc, middle, (1024, 768))));
        triggers.push(t);
    }
    r.git.root.set("TriggerList", Value::List(triggers));
    let wp = Gff::read(&r.game.resman.get_named("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let at = Placement { position: [DEST[0], DEST[1], 0.0], rotation: 0.0 };
    let mut w = instance(&placing, ResType::UTW, &wp.root, at, &[]).unwrap();
    w.set("Tag", Value::String(b"WP_TT_DEST".to_vec()));
    r.git.root.set("WaypointList", Value::List(vec![w]));
    let mut info = r.m.info().unwrap();
    info.root.write(&ifo::MOD_ENTRY_X, PC[0]);
    info.root.write(&ifo::MOD_ENTRY_Y, PC[1]);
    info.root.write(&ifo::MOD_ENTRY_Z, 0.0);
    r.m.set_info(&info).unwrap();
    // (Walking: the player's place written to the log every two seconds
    // from the sixth, for the trigger clicked ([`shoot_row`]'s steps).)
    let body = match walk {
        Some(_) => (3..20)
            .map(|k| {
                format!(
                    "DelayCommand({}.0, Log(\"MG_AFTER \" + FloatToString(GetPosition(pc).x, 0, 2) \
                     + \" \" + FloatToString(GetPosition(pc).y, 0, 2)));",
                    2 * k
                )
            })
            .collect::<Vec<_>>()
            .join("\n    "),
        None => super::readings::INVISIBLE.to_string(),
    };
    let enter = super::readings::enter(&body, (90.0, 12.0, 45.0));
    r.save(&enter, &settings());
    let dir = r.dir.clone();
    // Away (the pointer on the game's GUI, bottom right), each trigger
    // under the pointer, Tab held. Walking: the trigger clicked where the
    // client lights it (`CLICK_Y`), as a player goes through one.
    let mut steps = String::from("move 1000 740\nsleep 1\nshot plain\n");
    if let Some((i, y)) = walk {
        let x = middles[i].1.0;
        steps += &format!("move {x} {y}\nsleep 0.5\nshot before_click\nclick {x} {y}\n");
    } else {
        // (Down a column through each trigger's middle, a shot every
        // [`SWEEP`] pixels: where the client picks it is not quite where
        // the fitted camera puts it.)
        for (i, (_, (x, _))) in middles.iter().enumerate() {
            for (k, y) in (SWEEP_FROM..SWEEP_TO).step_by(SWEEP).enumerate() {
                steps += &format!(
                    "move {x} {y}\nsleep 0.1\nmove {} {y}\nsleep 0.3\nshot hover_{i}_{k}\n",
                    x + 1
                );
            }
        }
        steps += "move 1000 740\nsleep 0.5\nkeydown Tab\nsleep 1\nshot tab\nkeyup Tab\nsleep 0.5\n";
    }
    let after = if walk.is_some() { Duration::from_secs(20) } else { Duration::from_secs(1) };
    assert!(client_driven(&dir, &steps, after), "the client did not reach the scene: {dir:?}");
    super::readings::print_log(&dir);
    (dir, middles.into_iter().map(|m| m.1).collect())
}

/// How many pixels of the scene (the window above the game's GUI) are
/// blue: blue over the mean of red and green by more than 40, which the
/// grass and the sky never are.
fn blue_pixels(img: &Rgba) -> usize {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut n = 0;
    for y in 0..h * 80 / 100 {
        for x in 0..w * 90 / 100 {
            let i = (y * w + x) * 4;
            let [r, g, b] = [0, 1, 2].map(|c| f32::from(img.data[i + c]));
            n += usize::from(b - (r + g) / 2.0 > 40.0);
        }
    }
    n
}

/// Whether each trigger of a row [`shoot_row`] shot was lit under the
/// pointer (at some point of its sweep more than 200 blue pixels, which
/// the picture with the pointer away lacks), and whether any was with
/// Tab held; printed. The sweep's most lit picture of each is kept as
/// `hover_<i>.png`, the others removed.
fn lit(dir: &Path, variants: &[Variant]) -> (Vec<bool>, bool) {
    let away = blue_pixels(&common::read_png(&dir.join("plain.png")));
    let tab = blue_pixels(&common::read_png(&dir.join("tab.png")));
    let mut hovered = Vec::new();
    for (i, v) in variants.iter().enumerate() {
        let mut best = (0, 0);
        for k in 0..(SWEEP_FROM..SWEEP_TO).step_by(SWEEP).len() {
            let path = dir.join(format!("hover_{i}_{k}.png"));
            let n = blue_pixels(&common::read_png(&path));
            if n >= best.0 {
                best = (n, k);
            }
        }
        for k in 0..(SWEEP_FROM..SWEEP_TO).step_by(SWEEP).len() {
            let path = dir.join(format!("hover_{i}_{k}.png"));
            if k == best.1 {
                std::fs::rename(&path, dir.join(format!("hover_{i}.png"))).unwrap();
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        let y = SWEEP_FROM + (best.1 * SWEEP) as i32;
        eprintln!(
            "{:12} blue pixels: {} away, at most {} under the pointer (y {y})",
            v.name, away, best.0
        );
        hovered.push(best.0 > away + 200);
    }
    eprintln!("with Tab held: {tab} blue pixels");
    (hovered, tab > away + 200)
}

/// Under the pointer, the client fills a transition trigger's outline
/// with blue when its points run anticlockwise (seen from above, as
/// Aurora always saves them), and draws nothing when they run clockwise;
/// `Cursor` and `HighlightHeight` change neither. Tab shows none of them.
/// (The control, Aurora's own painted and linked `TT_A1`, comes first
/// when the oracle's capture is there.)
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_by_field() {
    let variants = [
        Variant { name: "ccw", change: |_| {} },
        Variant { name: "cw", change: clockwise },
        Variant { name: "ccw_cursor0", change: |s| s.set("Cursor", Value::Byte(0)) },
        Variant {
            name: "cw_cursor0",
            change: |s| {
                clockwise(s);
                s.set("Cursor", Value::Byte(0));
            },
        },
        Variant { name: "ccw_high3", change: |s| s.set("HighlightHeight", Value::Float(3.0)) },
        Variant {
            name: "cw_high3",
            change: |s| {
                clockwise(s);
                s.set("HighlightHeight", Value::Float(3.0));
            },
        },
    ];
    let (dir, _) = shoot_row("fields", &variants, None);
    let (hovered, tabbed) = lit(&dir, &variants);
    assert_eq!(hovered, [true, false, true, false, true, false], "{}", dir.display());
    assert!(!tabbed, "Tab showed one: {}", dir.display());
}

/// The control: Aurora's own painted and linked `TT_A1` (from the
/// oracle's capture, as it is: drawn on raised ground, its points at 5.025;
/// and with them at the ground's 0.025 here), beside Moonglow's
/// anticlockwise and clockwise ones.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_control() {
    let variants = [
        Variant { name: "aurora", change: aurora },
        Variant {
            name: "aurora_flat",
            change: |s| {
                aurora(s);
                point_z(s, 0.025);
            },
        },
        Variant { name: "ccw", change: |_| {} },
        Variant { name: "cw", change: clockwise },
    ];
    let (dir, _) = shoot_row("control", &variants, None);
    let (hovered, _) = lit(&dir, &variants);
    // (Aurora's as it is stands 5 m above this flat ground: not drawn, as
    // `transition_outlines_by_height` finds of any.)
    let control = mg_testkit::aurora_capture("transition-triggers/07-s2-both.mod").is_some();
    let aurora = if control { [false, true] } else { [true, true] };
    assert_eq!(hovered[..2], aurora, "{}", dir.display());
    assert_eq!(hovered[2..], [true, false], "{}", dir.display());
}

/// An anticlockwise outline whose points are off the ground (`PointZ`
/// a metre or five below it, three above: a trigger moved to other ground
/// with its old heights kept) is not drawn under the pointer either.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_by_height() {
    let variants = [
        Variant { name: "ccw", change: |_| {} },
        Variant { name: "z_below1", change: |s| point_z(s, -1.0) },
        Variant { name: "z_below5", change: |s| point_z(s, -5.0) },
        Variant { name: "z_above3", change: |s| point_z(s, 3.0) },
        Variant { name: "cw", change: clockwise },
    ];
    let (dir, _) = shoot_row("heights", &variants, None);
    let (hovered, _) = lit(&dir, &variants);
    assert_eq!(hovered, [true, false, false, false, false], "{}", dir.display());
}

/// Where the player ended after walking onto the one trigger `variant`
/// (`MG_AFTER` in the client's log).
fn walked(name: &str, variant: Variant, y: i32) -> glam::Vec2 {
    let variants = [variant];
    let (dir, _) = shoot_row(name, &variants, Some((0, y)));
    let log = std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap();
    let after: Vec<f32> = (log.lines().rev().find_map(|l| l.split("MG_AFTER ").nth(1)))
        .expect("MG_AFTER in the log")
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect();
    let to = glam::Vec2::new(after[0], after[1]);
    eprintln!("{name}: the player ended at {to} ({})", dir.display());
    to
}

/// What the transitions the client does not draw still do (nothing): each of an
/// anticlockwise one (the control), a clockwise one and one whose points
/// are a metre above the ground, clicked (where the client lights the
/// control) and crossed (the ground beyond it clicked, the player's path
/// through it): whether the player was taken to the waypoint ([`DEST`]).
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transitions_not_drawn_clicked_and_crossed() {
    let dest = glam::Vec2::from(DEST);
    let mut taken = Vec::new();
    for (how, y) in [("clicked", CLICK_Y), ("crossed", THROUGH_Y)] {
        for v in [
            Variant { name: "ccw", change: |_| {} },
            Variant { name: "cw", change: clockwise },
            Variant { name: "z_above1", change: |s| point_z(s, 1.0) },
        ] {
            let name = v.name;
            let to = walked(&format!("walk_{how}_{name}"), v, y);
            taken.push((how, name, to.distance(dest) < 3.0));
        }
    }
    for t in &taken {
        eprintln!("{t:?}");
    }
    // Clicked, the anticlockwise one takes the player through, the others
    // do not (the player walks to the ground clicked, inside them); and
    // walking across none of them does (this client: transitions are
    // clicked).
    let through: Vec<bool> = taken.iter().map(|t| t.2).collect();
    assert_eq!(through, [true, false, false, false, false, false]);
}

/// How near the ground an anticlockwise outline's points must be to be
/// drawn: at 0, 0.025 (Aurora's lift) and 0.03 it is; 0.1 above or below
/// it is not. (At 0.05 it fights the ground: drawn in streaks or not.)
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_height_tolerance() {
    let variants = [
        Variant { name: "z_0", change: |s| point_z(s, 0.0) },
        Variant { name: "z_01", change: |s| point_z(s, 0.1) },
        Variant { name: "ccw", change: |_| {} },
        Variant { name: "z_m01", change: |s| point_z(s, -0.1) },
        Variant { name: "z_003", change: |s| point_z(s, 0.03) },
        Variant { name: "z_03", change: |s| point_z(s, 0.3) },
    ];
    let (dir, _) = shoot_row("tolerance", &variants, None);
    let (hovered, _) = lit(&dir, &variants);
    assert_eq!(hovered, [true, false, true, false, true, false], "{}", dir.display());
}

/// Whether the client adds `ZPosition` to the points' `PointZ`: on flat
/// ground, a trigger stood at 1 with its points at 0.025, and one stood
/// at -1 with its points at 1.025.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_by_z_position() {
    let variants = [
        Variant { name: "ccw", change: |_| {} },
        Variant { name: "zpos1", change: |s| s.set("ZPosition", Value::Float(1.0)) },
        Variant {
            name: "zpos_m1_z1025",
            change: |s| {
                s.set("ZPosition", Value::Float(-1.0));
                point_z(s, 1.025);
            },
        },
        Variant { name: "z1025", change: |s| point_z(s, 1.025) },
    ];
    let (dir, _) = shoot_row("zposition", &variants, None);
    let (hovered, _) = lit(&dir, &variants);
    // (Added: the points' height is `ZPosition` + `PointZ`.)
    assert_eq!(hovered, [true, false, true, false], "{}", dir.display());
}

/// A trigger of a module as the client shows it: the module copied (as
/// `MgScene` in the scratch user folder), the player entering `back`
/// metres south of the trigger's middle in area `area`, facing north, the
/// camera held as the rows' ([`super::readings::enter`]); the window swept
/// with the pointer in three columns around the middle, the most lit
/// picture kept (`hover.png`), then clicked there. Whether it was lit
/// (blue pixels over 200) and whether the click took the player to
/// where it leads (its `LinkedTo` door or waypoint, within 5 m, in its
/// area); the area's own scripts and the module's load script are left
/// out.
fn shoot_in_module(name: &str, src: &Path, area: &str, tag: &str, back: f32) -> (bool, bool) {
    shoot_in_module_with(name, src, area, tag, back, &|_, _| {})
}

/// The area's walkable ground as Verify reads it (`mg_module::ground`),
/// with the module's haks.
fn verify_ground(m: &mg_module::Module, area: ResRef) -> Option<mg_module::ground::AreaGround> {
    let root = mg_testkit::nwn_root()?;
    let gi = GameInstall::new(&root, None, "en");
    let mut rm = mg_resman::ResMan::for_game(&gi).ok()?;
    let _ = m.stack_onto(&mut rm, &gi);
    mg_module::ground::Grounds::default().of(&rm, m, area)
}

/// [`shoot_in_module`] with the trigger changed first by `change` (given
/// the area's ground as Verify reads it).
fn shoot_in_module_with(
    name: &str,
    src: &Path,
    area: &str,
    tag: &str,
    back: f32,
    change: &dyn Fn(&mut Struct, Option<&mg_module::ground::AreaGround>),
) -> (bool, bool) {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let dir = scratch_dir(&format!("client_reading_transitions_{name}"));
    std::fs::create_dir_all(dir.join("user/modules")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let mut m = mg_module::Module::open(src).unwrap();
    let area_ref = ResRef::from_str(area).unwrap();
    let git_key = ResKey::new(area_ref, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let ground = verify_ground(&m, area_ref);
    let triggers = git.root.list_mut("TriggerList").unwrap();
    let i = (triggers.iter().position(|t| t.string("Tag") == Some(tag.as_bytes())))
        .unwrap_or_else(|| panic!("{tag} in {area}"));
    change(&mut triggers[i], ground.as_ref());
    let t = triggers[i].clone();
    m.set_gff(git_key, &git).unwrap();
    print_heights(&t, ground.as_ref());
    let f = |s: &Struct, l: &str| s.float(l).unwrap_or(0.0);
    let points = t.list("Geometry").unwrap();
    let mean = points
        .iter()
        .fold(Vec3::ZERO, |a, p| a + Vec3::new(f(p, "PointX"), f(p, "PointY"), f(p, "PointZ")))
        / points.len() as f32;
    let middle = Vec3::new(f(&t, "XPosition"), f(&t, "YPosition"), f(&t, "ZPosition")) + mean;
    // Where it leads.
    let link = t.string("LinkedTo").unwrap_or_default().to_vec();
    let mut dest = None;
    for key in m.keys_of(ResType::GIT).copied().collect::<Vec<_>>() {
        let g = m.gff(&key).unwrap().unwrap();
        for (list, x, y) in [("WaypointList", "XPosition", "YPosition"), ("Door List", "X", "Y")] {
            for o in g.root.list(list).unwrap_or(&[]) {
                if o.string("Tag").is_some_and(|tg| tg.eq_ignore_ascii_case(&link)) {
                    dest = Some((key.resref, glam::Vec2::new(f(o, x), f(o, y))));
                }
            }
        }
    }
    eprintln!("{tag}: middle {middle}, leads to {dest:?}");
    let are_key = ResKey::new(area_ref, ResType::ARE);
    let mut are = m.gff(&are_key).unwrap().unwrap();
    for label in ["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"] {
        are.root.set(label, Value::resref(ResRef::EMPTY));
    }
    m.set_gff(are_key, &are).unwrap();
    // (The player's area and place written every two seconds from the
    // sixth.)
    let logs: Vec<String> = (3..25)
        .map(|k| {
            format!(
                "DelayCommand({}.0, Log(\"MG_AFTER \" + GetTag(GetArea(pc)) + \" \" + \
                 GetResRef(GetArea(pc)) + \" \" + FloatToString(GetPosition(pc).x, 0, 2) + \" \" \
                 + FloatToString(GetPosition(pc).y, 0, 2) + \" \" + FloatToString(GetPosition(pc).z, 0, 2)));",
                2 * k
            )
        })
        .collect();
    let enter = super::readings::enter(&logs.join("\n    "), (90.0, 12.0, 45.0));
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let mut info = m.info().unwrap();
    let pc = middle - Vec3::Y * back;
    info.root.set("Mod_Entry_Area", Value::resref(area_ref));
    info.root.write(&ifo::MOD_ENTRY_X, pc.x);
    info.root.write(&ifo::MOD_ENTRY_Y, pc.y);
    info.root.write(&ifo::MOD_ENTRY_Z, pc.z);
    info.root.write(&ifo::MOD_ENTRY_DIR_X, 0.0);
    info.root.write(&ifo::MOD_ENTRY_DIR_Y, 1.0);
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    info.root.set("Mod_OnModLoad", Value::resref(ResRef::EMPTY));
    eprintln!("haks: {:?}", info.root.list("Mod_HakList").map(<[_]>::len));
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    std::fs::write(dir.join("user/settings.tml"), settings()).unwrap();
    drop(game);
    let steps = "move 1000 740\nsleep 1\nshot plain\nsweep hover 432 592 80 120 560 20\n\
                 clicklit 512 300\nsleep 1\nshot clicked\n";
    assert!(client_driven(&dir, steps, Duration::from_secs(20)), "no scene: {}", dir.display());
    super::readings::print_log(&dir);
    let found = std::fs::read_to_string(dir.join("hover.txt")).unwrap();
    let blue: usize = found.split_whitespace().next().unwrap().parse().unwrap();
    let away = blue_pixels(&common::read_png(&dir.join("plain.png")));
    let log = std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap();
    let last = log.lines().rev().find_map(|l| l.split("MG_AFTER ").nth(1)).unwrap_or_default();
    let w: Vec<&str> = last.split_whitespace().collect();
    let taken = dest.is_some_and(|(a, at)| {
        w.len() >= 4
            && w[w.len() - 4].eq_ignore_ascii_case(&a.to_string())
            && at.distance(glam::Vec2::new(
                w[w.len() - 3].parse().unwrap_or(-99.0),
                w[w.len() - 2].parse().unwrap_or(-99.0),
            )) < 5.0
    });
    eprintln!(
        "{name}: {blue} blue pixels at most under the pointer ({away} away), {found:?}; last {last:?}; taken {taken}"
    );
    (blue > away + 200, taken)
}

/// A south-east Alt rectangle as Moonglow 4.1.0 saved it, before the
/// fix: its corners in the order `rectangle_to` gave them, `[from,
/// (to.x, from.y), to, (from.x, to.y)]`, dragged from the north-west
/// corner to the south-east one, which runs clockwise. (4.1.0 stood the
/// trigger at its first corner; here it stands at the row's south-west
/// corner, the order kept: the order is what the client reads.)
fn south_east_as_4_1_0(s: &mut Struct) {
    let corners = [[0.0, DEPTH], [WIDTH, DEPTH], [WIDTH, 0.0], [0.0, 0.0]];
    let points = corners
        .iter()
        .map(|[x, y]| {
            let mut p = Struct::new(3);
            p.set("PointX", Value::Float(*x));
            p.set("PointY", Value::Float(*y));
            p.set("PointZ", Value::Float(mg_module::instances::OUTLINE_LIFT));
            p
        })
        .collect();
    s.set("Geometry", Value::List(points));
}

/// Before and after the fix of 2026-11-12. Before: the south-east Alt
/// rectangle as 4.1.0 saved it ([`south_east_as_4_1_0`]), beside its
/// anticlockwise twin, is neither shown nor clicked through. After: the
/// fixed build's module (`target/transition-triggers/after/after-fix.mod`,
/// written by the ignored UI test
/// `transition_triggers::the_module_for_the_after_picture`), its three
/// transitions (drawn clockwise, as a south-east Alt rectangle, and
/// moved) each shown and clicked through; skipped, saying so, without
/// that module.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_before_and_after_the_fix() {
    let before = || {
        [
            Variant { name: "ccw", change: |_| {} },
            Variant { name: "se_as_4_1_0", change: south_east_as_4_1_0 },
        ]
    };
    let (dir, _) = shoot_row("before_fix", &before(), None);
    let (hovered, _) = lit(&dir, &before());
    assert_eq!(hovered, [true, false], "{}", dir.display());
    let [_, se] = before();
    let to = walked("before_fix_se_clicked", se, CLICK_Y);
    assert!(to.distance(glam::Vec2::from(DEST)) > 3.0, "4.1.0's taken through: {to}");
    let after = common::repo().join("target/transition-triggers/after/after-fix.mod");
    if !after.is_file() {
        eprintln!(
            "skipped the after half: no {} (written by the ignored UI test \
             transition_triggers::the_module_for_the_after_picture: cargo test -p mg-ui --test app \
             transition_triggers::the_module_for_the_after_picture -- --ignored)",
            after.display()
        );
        return;
    }
    let mut seen = Vec::new();
    for tag in ["TT_CW", "TT_SE", "TT_MOVED"] {
        let name = format!("after_fix_{}", tag.to_lowercase());
        seen.push((name.clone(), shoot_in_module(&name, &after, "field", tag, 6.0)));
    }
    for s in &seen {
        eprintln!("{s:?}");
    }
    let got: Vec<(bool, bool)> = seen.iter().map(|s| s.1).collect();
    assert_eq!(got, [(true, true); 3]);
}

/// Sets corner `i`'s `PointZ` to `z`.
fn corner_z(s: &mut Struct, i: usize, z: f32) {
    let mut points = s.list("Geometry").unwrap().to_vec();
    points[i].set("PointZ", Value::Float(z));
    s.set("Geometry", Value::List(points));
}

/// One corner of an anticlockwise outline on flat ground off it (+0.3,
/// −0.3, +0.05), and two of four +0.3: shown under the pointer, and
/// taken through when clicked where the anticlockwise one is lit.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_with_a_lone_corner() {
    fn variants() -> [Variant; 5] {
        [
            Variant { name: "ccw", change: |_| {} },
            Variant { name: "corner_up03", change: |s| corner_z(s, 2, 0.325) },
            Variant { name: "corner_down03", change: |s| corner_z(s, 2, -0.275) },
            Variant { name: "corner_up005", change: |s| corner_z(s, 2, 0.075) },
            Variant {
                name: "two_up03",
                change: |s| {
                    corner_z(s, 2, 0.325);
                    corner_z(s, 3, 0.325);
                },
            },
        ]
    }
    let (dir, _) = shoot_row("lone_corner", &variants(), None);
    let (hovered, _) = lit(&dir, &variants());
    let dest = glam::Vec2::from(DEST);
    let taken: Vec<bool> = (variants().into_iter())
        .map(|v| {
            let name = format!("click_{}", v.name);
            walked(&name, v, CLICK_Y).distance(dest) < 3.0
        })
        .collect();
    eprintln!("lit {hovered:?}, taken {taken:?} ({})", dir.display());
    // Drawn in part: what is near the ground (the whole, a corner's
    // quarter left out at -0.3 and +0.05, an edge's strip at one corner
    // +0.3, a line at two); all of them clicked through.
    assert!(hovered[0] && hovered[2] && hovered[3], "{hovered:?} ({})", dir.display());
    assert_eq!(taken, [true; 5], "{}", dir.display());
}

/// The game's own transitions whose outlines are wholly 0.23 to 0.31 m
/// off the walkable ground (`mg_module::ground`), picked by area and tag,
/// each in a scratch copy of its module with the player beside it,
/// beside a transition of the same area on the ground: the client shows
/// them (all but `AR0900_AR1000_EXIT`, which a cliff hides from this
/// camera, as it does that area's `AR0900_AR1800_EXIT`; a click there
/// takes the player to the world map all the same). The drawn tile
/// surface under them is the walkmesh
/// ([`shipped_transitions_against_the_drawn_tiles`]): how far off the
/// client still shows an outline depends on the place, which is why
/// Verify's `transition-hidden` finds only outlines more than 1 m off.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn shipped_transitions_off_the_walkmesh_are_shown() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let nwm = |name: &str| root.join(format!("data/nwm/{name}.nwm"));
    let interlude = nwm("XP1-Interlude");
    let dod = nwm("Neverwinter Nights - Darkness over Daggerford");
    let cases: [(&str, &Path, &str, &str); 6] = [
        ("xp1i_formian_lair2_off", &interlude, "area007", "Q3BAT_Ruins2_FormianLair2"),
        ("xp1i_formian_lair_control", &interlude, "area007", "Q2BAT_Ruins_FormianLair"),
        ("dod_ar0700_off", &dod, "c1ar0900_wildhil", "AR0900_AR0700_EXIT"),
        ("dod_ar1000_off", &dod, "c1ar0900_wildhil", "AR0900_AR1000_EXIT"),
        ("dod_ar1800_control", &dod, "c1ar0900_wildhil", "AR0900_AR1800_EXIT"),
        ("dod_lizard_off", &dod, "c1ar1900_lizard", "AR1900_AR1800_EXIT"),
    ];
    let mut seen = Vec::new();
    for (name, module, area, tag) in cases {
        seen.push((name, shoot_in_module(name, module, area, tag, 6.0)));
    }
    for s in &seen {
        eprintln!("{s:?}");
    }
    let lit: Vec<bool> = seen.iter().map(|s| s.1.0).collect();
    assert!(lit[0] && lit[1] && lit[2] && lit[5], "{seen:?}");
}

/// Each point of trigger `t`: its height in the game (`ZPosition` +
/// `PointZ`) and the walkable ground's under it (Verify's).
fn print_heights(t: &Struct, ground: Option<&mg_module::ground::AreaGround>) {
    let f = |s: &Struct, l: &str| s.float(l).unwrap_or(0.0);
    let (x, y, z) = (f(t, "XPosition"), f(t, "YPosition"), f(t, "ZPosition"));
    for p in t.list("Geometry").unwrap() {
        let (px, py, pz) = (x + f(p, "PointX"), y + f(p, "PointY"), z + f(p, "PointZ"));
        let under = ground.and_then(|g| g.height(px, py, pz));
        eprintln!("  point ({px:.2}, {py:.2}): at {pz:.3}, walkmesh {under:?}");
    }
}

/// Sets every point of `t` to `by` above the walkable ground under it
/// plus Aurora's lift (in the game's height, `ZPosition` + `PointZ`; a
/// point with no ground under it kept).
fn above_walkmesh(t: &mut Struct, ground: Option<&mg_module::ground::AreaGround>, by: f32) {
    let f = |s: &Struct, l: &str| s.float(l).unwrap_or(0.0);
    let (x, y, z) = (f(t, "XPosition"), f(t, "YPosition"), f(t, "ZPosition"));
    let mut points = t.list("Geometry").unwrap().to_vec();
    for p in &mut points {
        let (px, py, pz) = (x + f(p, "PointX"), y + f(p, "PointY"), z + f(p, "PointZ"));
        if let Some(under) = ground.and_then(|g| g.height(px, py, pz)) {
            p.set("PointZ", Value::Float(under + mg_module::instances::OUTLINE_LIFT + by - z));
        }
    }
    t.set("Geometry", Value::List(points));
}

/// Moves every point of `t` by `by`.
fn shifted(t: &mut Struct, by: f32) {
    let mut points = t.list("Geometry").unwrap().to_vec();
    for p in &mut points {
        let z = p.float("PointZ").unwrap_or(0.0);
        p.set("PointZ", Value::Float(z + by));
    }
    t.set("Geometry", Value::List(points));
}

/// The game's own transitions off the walkable ground
/// ([`shipped_transitions_off_the_walkmesh_are_shown`]), at heights between
/// the walkmesh (where Moonglow puts an outline: placing, Redraw
/// Polygon, moves, Verify's fix) and where they were shipped: shown or
/// not, and clicked through. `sign` is the side of the walkmesh the
/// shipped outline is on.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn shipped_transitions_on_the_walkmesh() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let nwm = |name: &str| root.join(format!("data/nwm/{name}.nwm"));
    let interlude = nwm("XP1-Interlude");
    let dod = nwm("Neverwinter Nights - Darkness over Daggerford");
    let cases: [(&str, &Path, &str, &str, f32); 3] = [
        ("xp1i", &interlude, "area007", "Q3BAT_Ruins2_FormianLair2", 1.0),
        ("lizard", &dod, "c1ar1900_lizard", "AR1900_AR1800_EXIT", 1.0),
        ("ar0700", &dod, "c1ar0900_wildhil", "AR0900_AR0700_EXIT", -1.0),
    ];
    let mut seen = Vec::new();
    for (name, module, area, tag, sign) in cases {
        type Change = Box<dyn Fn(&mut Struct, Option<&mg_module::ground::AreaGround>)>;
        let heights: Vec<(&str, Change)> = vec![
            ("shipped", Box::new(|_, _| {})),
            (
                "moonglow_fix",
                Box::new(|t, g| {
                    let (points, z) = mg_module::ground::outline_put_right(t, g);
                    t.set("Geometry", Value::List(points));
                    t.set("ZPosition", Value::Float(z));
                }),
            ),
            ("walkmesh_01", Box::new(move |t, g| above_walkmesh(t, g, 0.1 * sign))),
            ("walkmesh_02", Box::new(move |t, g| above_walkmesh(t, g, 0.2 * sign))),
            ("walkmesh_03", Box::new(move |t, g| above_walkmesh(t, g, 0.3 * sign))),
            ("shipped_up01", Box::new(|t, _| shifted(t, 0.1))),
            ("shipped_down01", Box::new(|t, _| shifted(t, -0.1))),
        ];
        for (height, change) in &heights {
            let run = format!("{name}_{height}");
            let got = shoot_in_module_with(&run, module, area, tag, 6.0, change.as_ref());
            seen.push((run, got));
        }
    }
    for s in &seen {
        eprintln!("{s:?}");
    }
    // On the walkmesh as Moonglow puts it, each is shown (2026-11-12:
    // every height here was shown on Interlude's area007 and the lizard
    // village; at AR0900_AR0700_EXIT, 0.3 below hid it).
    for s in seen.iter().filter(|s| s.0.ends_with("moonglow_fix")) {
        assert!(s.1.0, "{s:?}");
    }
}

/// The heights of the drawn faces of the tile under (`x`, `y`) (its
/// model's rendered meshes, not its walkmesh), in the area's space, from
/// highest to lowest; none off the tiles.
fn drawn_heights(rm: &mg_resman::ResMan, are: &Struct, x: f32, y: f32) -> Vec<f32> {
    let Ok(set) = mg_module::minimap::tileset(rm, are) else { return Vec::new() };
    let width = are.integer("Width").unwrap_or(0).max(1) as u32;
    let (column, row) = ((x / 10.0) as u32, (y / 10.0) as u32);
    let list = are.list("Tile_List").unwrap_or(&[]);
    let Some(t) = list.get((row * width + column) as usize) else { return Vec::new() };
    let Some(tile) =
        usize::try_from(t.integer("Tile_ID").unwrap_or(-1)).ok().and_then(|i| set.tiles.get(i))
    else {
        return Vec::new();
    };
    let Ok(data) = rm.get_named(&tile.model.to_ascii_lowercase(), ResType::MDL) else {
        return Vec::new();
    };
    let Ok(model) = Model::read(&data) else { return Vec::new() };
    let up = t.integer("Tile_Height").unwrap_or(0) as f32 * set.general.transition;
    let (dx, dy) = (x - (column as f32 + 0.5) * 10.0, y - (row as f32 + 0.5) * 10.0);
    let (lx, ly) = match t.integer("Tile_Orientation").unwrap_or(0).rem_euclid(4) {
        0 => (dx, dy),
        1 => (dy, -dx),
        2 => (-dx, -dy),
        _ => (-dy, dx),
    };
    let mut out = Vec::new();
    for (i, n) in model.nodes.iter().enumerate() {
        let mg_mdl::NodeKind::Mesh(mesh) = &n.kind else { continue };
        if !mesh.render || matches!(mesh.extra, mg_mdl::MeshExtra::Aabb(_)) {
            continue;
        }
        for [a, b, c] in mesh.triangles() {
            let corner = |k: u32| {
                mesh.vertices
                    .get(k as usize)
                    .map(|v| mg_mdl::walkmesh::to_model_space(&model, i, *v))
            };
            let (Some(a), Some(b), Some(c)) = (corner(a), corner(b), corner(c)) else { continue };
            let cross = |p: [f32; 2], q: [f32; 2]| p[0] * q[1] - p[1] * q[0];
            let (ab, ac, ap) =
                ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]], [lx - a[0], ly - a[1]]);
            let d = cross(ab, ac);
            if d.abs() < 1e-9 {
                continue;
            }
            let (u, v) = (cross(ap, ac) / d, cross(ab, ap) / d);
            if u >= -1e-4 && v >= -1e-4 && u + v <= 1.0 + 1e-4 {
                out.push(a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]) + up);
            }
        }
    }
    out.sort_by(|a, b| b.total_cmp(a));
    out.dedup_by(|a, b| (*a - *b).abs() < 0.005);
    out
}

/// For each point of the game's own transitions off the walkable ground
/// ([`shipped_transitions_off_the_walkmesh_are_shown`]): its
/// height in the game, the walkable ground's under it (Verify's) and the
/// tile's drawn faces' (its rendered meshes), printed. No client needed.
#[test]
#[ignore = "a reading, run by hand"]
fn shipped_transitions_against_the_drawn_tiles() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let nwm = |name: &str| root.join(format!("data/nwm/{name}.nwm"));
    let cases = [
        ("XP1-Interlude", "area007", "Q3BAT_Ruins2_FormianLair2"),
        ("Neverwinter Nights - Darkness over Daggerford", "c1ar1900_lizard", "AR1900_AR1800_EXIT"),
        ("Neverwinter Nights - Darkness over Daggerford", "c1ar0900_wildhil", "AR0900_AR0700_EXIT"),
    ];
    for (module, area, tag) in cases {
        let m = mg_module::Module::open(&nwm(module)).unwrap();
        let gi = GameInstall::new(&root, None, "en");
        let mut rm = mg_resman::ResMan::for_game(&gi).unwrap();
        let _ = m.stack_onto(&mut rm, &gi);
        let area = ResRef::from_str(area).unwrap();
        let are = m.gff(&ResKey::new(area, ResType::ARE)).unwrap().unwrap();
        let git = m.gff(&ResKey::new(area, ResType::GIT)).unwrap().unwrap();
        let ground = mg_module::ground::Grounds::default().of(&rm, &m, area);
        let t = (git.root.list("TriggerList").unwrap().iter())
            .find(|t| t.string("Tag") == Some(tag.as_bytes()))
            .unwrap();
        let f = |s: &Struct, l: &str| s.float(l).unwrap_or(0.0);
        let (x, y, z) = (f(t, "XPosition"), f(t, "YPosition"), f(t, "ZPosition"));
        eprintln!("{tag} ({area}):");
        for p in t.list("Geometry").unwrap() {
            let (px, py, pz) = (x + f(p, "PointX"), y + f(p, "PointY"), z + f(p, "PointZ"));
            let walk = ground.as_ref().and_then(|g| g.height(px, py, pz));
            let drawn = drawn_heights(&rm, &are.root, px, py);
            let near =
                drawn.iter().copied().min_by(|a, b| (a - pz).abs().total_cmp(&(b - pz).abs()));
            eprintln!(
                "  ({px:6.2}, {py:6.2}) point {pz:.3}  walkmesh {:?}  drawn nearest {:?}  drawn all {:?}",
                walk.map(|w| (w * 1000.0).round() / 1000.0),
                near.map(|w| (w * 1000.0).round() / 1000.0),
                drawn.iter().map(|d| (d * 100.0).round() / 100.0).collect::<Vec<_>>()
            );
        }
    }
}

/// Scales an outline's X and Y by `k` (about its position).
fn scaled(s: &mut Struct, k: f32) {
    let mut points = s.list("Geometry").unwrap().to_vec();
    for p in &mut points {
        for l in ["PointX", "PointY"] {
            let v = p.float(l).unwrap_or(0.0);
            p.set(l, Value::Float(v * k));
        }
    }
    s.set("Geometry", Value::List(points));
}

/// Whether an outline's size changes how far off the ground it may be:
/// on flat ground, a 2 m one and a 6 m one, each 0.3 m up, beside a 6 m
/// one on the ground.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_by_size_off_the_ground() {
    let variants = [
        Variant { name: "small_up03", change: |s| point_z(s, 0.325) },
        Variant {
            name: "big_up03",
            change: |s| {
                scaled(s, 3.0);
                point_z(s, 0.325);
            },
        },
        Variant { name: "big_on", change: |s| scaled(s, 3.0) },
    ];
    let (dir, _) = shoot_row("size", &variants, None);
    let (hovered, _) = lit(&dir, &variants);
    assert_eq!(hovered, [false, false, true], "{}", dir.display());
}

/// Darkness over Daggerford's `AR0900_AR0700_EXIT` put on the walkmesh as
/// Verify's fix puts it, clicked again (once the click missed); and, on
/// flat ground, outlines 0.3 m up by a `ZPosition` of their own, beside
/// one 0.3 m up by `PointZ` alone.
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn transition_outlines_fix_clicked_and_raised_by_position() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let dod = root.join("data/nwm/Neverwinter Nights - Darkness over Daggerford.nwm");
    let fixed = shoot_in_module_with(
        "ar0700_moonglow_fix_again",
        &dod,
        "c1ar0900_wildhil",
        "AR0900_AR0700_EXIT",
        6.0,
        &|t, g| {
            let (points, z) = mg_module::ground::outline_put_right(t, g);
            t.set("Geometry", Value::List(points));
            t.set("ZPosition", Value::Float(z));
        },
    );
    eprintln!("AR0900_AR0700_EXIT on the walkmesh: {fixed:?}");
    assert_eq!(fixed, (true, true), "shown and clicked through");
    let variants = [
        Variant { name: "pointz_up03", change: |s| point_z(s, 0.325) },
        Variant {
            name: "zpos02_up03",
            change: |s| {
                s.set("ZPosition", Value::Float(0.2));
                point_z(s, 0.125);
            },
        },
        Variant {
            name: "zpos15_up03",
            change: |s| {
                s.set("ZPosition", Value::Float(1.5));
                point_z(s, -1.175);
            },
        },
        Variant { name: "ccw", change: |_| {} },
    ];
    let (dir, _) = shoot_row("zpos_raised", &variants, None);
    let (hovered, _) = lit(&dir, &variants);
    assert_eq!(hovered, [false, false, false, true], "{}", dir.display());
}

/// The shipped transitions farther off their walkmesh: how far the
/// client lets them be. (2026-11-12: the lizard village's shown at +0.6,
/// hidden at +1.0 and -0.6; Interlude's area007 lit at all three, but a
/// fight there, with its spells' light, and a talking NPC, keep its
/// pictures and clicks from settling anything.)
#[test]
#[ignore = "needs the game client and the off-screen display"]
fn shipped_transitions_far_off_the_walkmesh() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let nwm = |name: &str| root.join(format!("data/nwm/{name}.nwm"));
    let interlude = nwm("XP1-Interlude");
    let dod = nwm("Neverwinter Nights - Darkness over Daggerford");
    let mut seen = Vec::new();
    for by in [0.6f32, 1.0, -0.6] {
        for (name, module, area, tag) in [
            ("xp1i", &interlude, "area007", "Q3BAT_Ruins2_FormianLair2"),
            ("lizard", &dod, "c1ar1900_lizard", "AR1900_AR1800_EXIT"),
        ] {
            let run = format!("{name}_walkmesh_{by:+.1}");
            let change = move |t: &mut Struct, g: Option<&mg_module::ground::AreaGround>| {
                above_walkmesh(t, g, by);
            };
            seen.push((run.clone(), shoot_in_module_with(&run, module, area, tag, 6.0, &change)));
        }
    }
    for s in &seen {
        eprintln!("{s:?}");
    }
}
