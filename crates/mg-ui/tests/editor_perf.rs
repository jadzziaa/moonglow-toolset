//! How fast the editors are on very large files: the script editor on the
//! game's nwscript.nss (~14,000 lines), held to the 144 fps budget, and
//! the 2DA editor and Select Texture, timed. Run with
//! `cargo test --release -p mg-ui --test editor_perf -- --ignored --nocapture`
//! (`EDITOR_FRAMES=<n>` times that many frames of each, for a profiler to
//! sample, `EDITOR_ONLY=<part of a name>` only those, and `EDITOR_DETAIL=1`
//! prints each frame's time).

use std::time::{Duration, Instant};

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use mg_core::ResType;
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Moonglow, NoDialogs, Tab};

/// A frame at 144 a second, for all the frame does on the CPU (as
/// `frame_perf`'s).
const BUDGET: Duration = Duration::from_micros(6_944);
/// How far past the budget a median goes before the test fails (as
/// `frame_perf`'s: the same build's times move from run to run).
const MARGIN: f64 = 1.25;
/// The development machine's screen (as `frame_perf`'s).
const SCREEN: egui::Vec2 = egui::vec2(3440.0, 1400.0);

/// A window run as the desktop app runs it: plain egui frames (no
/// accessibility tree), tessellated, the clock moving on by a 144th of a
/// second a frame.
struct Window {
    app: Moonglow,
    ctx: egui::Context,
    clock: f64,
}

impl Window {
    /// One frame with these events; how long it took.
    fn frame(&mut self, events: Vec<Event>) -> Duration {
        self.clock += 1.0 / 144.0;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            time: Some(self.clock),
            predicted_dt: 1.0 / 144.0,
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let t = Instant::now();
        let out = self.ctx.run_ui(input, |ui| app.ui(ui));
        let _ = self.ctx.tessellate(out.shapes, out.pixels_per_point);
        t.elapsed()
    }

    /// `count` frames, each with the events `each` gives for it: the median
    /// and the worst, printed.
    fn frames(
        &mut self,
        what: &str,
        count: usize,
        mut each: impl FnMut(usize) -> Vec<Event>,
    ) -> Duration {
        let only = std::env::var("EDITOR_ONLY").ok();
        if only.is_some_and(|o| !what.contains(&o)) {
            return Duration::ZERO;
        }
        let mut took: Vec<Duration> = (0..count).map(|n| self.frame(each(n))).collect();
        if std::env::var_os("EDITOR_DETAIL").is_some() {
            println!("  {what}, each frame: {:?}", took.iter().map(|d| ms(*d)).collect::<Vec<_>>());
        }
        took.sort();
        let median = took[took.len() / 2];
        println!("{what}: median {:.2} ms, worst {:.2} ms", ms(median), ms(*took.last().unwrap()));
        median
    }

    /// [`Window::frames`] at a typist's pace: `count` keys, each followed
    /// by `gap` frames with nothing (18 keys a second at a gap of 7). The
    /// medians of the keys' frames and of the frames after them, printed
    /// with the worst of each.
    fn paced(
        &mut self,
        what: &str,
        count: usize,
        gap: usize,
        mut each: impl FnMut(usize) -> Vec<Event>,
    ) -> (Duration, Duration) {
        let only = std::env::var("EDITOR_ONLY").ok();
        if only.is_some_and(|o| !what.contains(&o)) {
            return (Duration::ZERO, Duration::ZERO);
        }
        let (mut keys, mut after) = (Vec::new(), Vec::new());
        for n in 0..count {
            keys.push(self.frame(each(n)));
            after.push(self.frame(Vec::new()));
            for _ in 1..gap {
                self.frame(Vec::new());
            }
        }
        let said = |took: &mut Vec<Duration>| {
            took.sort();
            (took[took.len() / 2], *took.last().unwrap())
        };
        let ((key, key_worst), (next, next_worst)) = (said(&mut keys), said(&mut after));
        println!(
            "{what}: the key's frame median {:.2} ms (worst {:.2}), the frame after it {:.2} ms \
             (worst {:.2})",
            ms(key),
            ms(key_worst),
            ms(next),
            ms(next_worst)
        );
        (key, next)
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

/// The script editor on the game's nwscript.nss, as large as scripts come
/// (its ~14,000 lines, 650 KB): typing, moving the cursor and the selection
/// over lines out of sight, scrolling, pasting and undoing, each frame within
/// the 144 fps budget, at the development machine's screen (3440×1400). The
/// editor lays out again only the lines a change touched
/// (`script_layout::LaidOut`); laid out whole, a key took 12 to 15 ms, and
/// 98 ms at a typist's pace.
#[test]
#[ignore]
fn editing_a_huge_script() {
    let root = mg_testkit::corpus!();
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let text = game.resman.get_named("nwscript", ResType::NSS).unwrap().into_owned();
    drop(game);
    let lines = text.split(|b| *b == b'\n').count();
    let dir = mg_testkit::scratch_dir("editor-perf");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let script = ResKey::parse("big", ResType::NSS).unwrap();
    m.set(script, text.clone());
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let count: usize =
        std::env::var("EDITOR_FRAMES").ok().and_then(|n| n.parse().ok()).unwrap_or(40);

    // With the accessibility tree (a screen reader's, kittest's), for the
    // record: it describes every line. (With the game: its nwscript.nss
    // read for the side lists, and for the check as you type.)
    let install = GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(script)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    let t = Instant::now();
    h.run();
    println!("open: {:.2} ms ({lines} lines)", ms(t.elapsed()));
    let editor = || egui_kittest::kittest::by().role(egui::accesskit::Role::MultilineTextInput);
    h.get(editor()).click();
    h.run();
    let mut typed = Vec::new();
    for _ in 0..10 {
        let t = Instant::now();
        h.get(editor()).type_text("x");
        h.step();
        typed.push(t.elapsed());
    }
    typed.sort();
    println!("with the accessibility tree, typing a character: median {:.2} ms", ms(typed[5]));

    // As the desktop app runs: held to the budget.
    let mut app = h.into_state();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(script)));
    let mut w = Window { app, ctx: egui::Context::default(), clock: 0.0 };
    let id = egui::Id::new(("script", script));
    let at = |w: &mut Window, from: usize, to: usize| {
        let mut state = egui::text_edit::TextEditState::load(&w.ctx, id).unwrap_or_default();
        let c = egui::text::CCursorRange::two(
            egui::text::CCursor::new(from),
            egui::text::CCursor::new(to),
        );
        state.cursor.set_char_range(Some(c));
        state.store(&w.ctx, id);
        w.ctx.memory_mut(|m| m.request_focus(id));
    };
    let t = Instant::now();
    w.frame(Vec::new());
    println!("first frame: {:.2} ms", ms(t.elapsed()));
    for _ in 0..5 {
        w.frame(Vec::new());
    }
    // The cursor halfway down.
    let middle = w.app.script_text(script).unwrap().chars().count() / 2;
    at(&mut w, middle, middle);
    w.frame(Vec::new());
    let mut medians = Vec::new();
    let mut held = |what: &str, median: Duration| medians.push((what.to_owned(), median));
    held("idle", w.frames("idle", count, |_| Vec::new()));
    held("typing", w.frames("typing a character", count, |_| vec![Event::Text("x".into())]));
    let (keys, after) =
        w.paced("typing at a typist's pace", count, 7, |_| vec![Event::Text("x".into())]);
    held("typing at a typist's pace", keys);
    held("the frame after a key", after);
    let (keys, _) =
        w.paced("a bookmark set and cleared", 10, 3, |_| vec![key(Key::F5, Modifiers::NONE)]);
    held("a bookmark", keys);
    held(
        "deleting",
        w.frames("deleting a character", count, |_| vec![key(Key::Backspace, Modifiers::NONE)]),
    );
    held("new line", w.frames("a new line", count, |_| vec![key(Key::Enter, Modifiers::NONE)]));
    held(
        "cursor",
        w.frames("the cursor down a line", count, |_| vec![key(Key::ArrowDown, Modifiers::NONE)]),
    );
    held(
        "page",
        w.frames("the cursor down a page", count, |_| vec![key(Key::PageDown, Modifiers::NONE)]),
    );
    held(
        "selecting",
        w.frames("the selection a line longer", count, |_| {
            vec![key(Key::ArrowDown, Modifiers::SHIFT)]
        }),
    );
    held(
        "selecting pages",
        w.frames("the selection a page longer", count, |_| {
            vec![key(Key::PageUp, Modifiers::SHIFT)]
        }),
    );
    let middle_of = egui::pos2(SCREEN.x * 0.3, SCREEN.y * 0.5);
    held(
        "scrolling",
        w.frames("scrolling", count, |n| {
            let down = if n < count / 2 { -1.0 } else { 1.0 };
            vec![
                Event::PointerMoved(middle_of),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, 120.0 * down),
                    phase: egui::TouchPhase::Move,
                    modifiers: Modifiers::NONE,
                },
            ]
        }),
    );
    w.frame(vec![Event::PointerGone]);
    held(
        "composing",
        w.frames("composing (IME)", count, |n| match n % 3 {
            0 => vec![Event::Ime(egui::ImeEvent::Preedit {
                text: "k".into(),
                active_range_chars: None,
            })],
            1 => vec![Event::Ime(egui::ImeEvent::Preedit {
                text: "ka".into(),
                active_range_chars: None,
            })],
            _ => vec![Event::Ime(egui::ImeEvent::Commit("か".into()))],
        }),
    );
    held(
        "undoing",
        w.frames("undoing", count, |_| vec![key(Key::Z, Modifiers::COMMAND | Modifiers::CTRL)]),
    );
    // A page pasted (the end of the script's first 500 lines).
    let block: String =
        String::from_utf8_lossy(&text).lines().take(500).collect::<Vec<_>>().join("\n");
    let mid = w.app.script_text(script).unwrap().chars().count() / 2;
    at(&mut w, mid, mid);
    w.frame(Vec::new());
    let before = w.app.script_text(script).unwrap().len();
    let t = w.frame(vec![Event::Paste(block.clone())]);
    println!("pasting 500 lines: {:.2} ms", ms(t));
    let after = w.app.script_text(script).unwrap().len();
    assert_eq!(after, before + block.len(), "the paste is in the text");
    held(
        "after a paste",
        w.frames("typing after a paste", count, |_| vec![Event::Text("y".into())]),
    );
    // Everything selected and replaced, then taken back.
    let all = w.app.script_text(script).unwrap();
    let t =
        w.frame(vec![key(Key::A, Modifiers::COMMAND | Modifiers::CTRL), Event::Text("z".into())]);
    println!("everything selected and typed over: {:.2} ms", ms(t));
    assert_eq!(w.app.script_text(script).unwrap(), "z");
    // (A frame between, as egui's undo keeps a step only once the text has
    // stayed the same a frame. The step taken back goes back further than
    // the whole text, egui's steps being a second of typing apart, and the
    // test's frames a 144th of a second.)
    w.frame(Vec::new());
    let t = w.frame(vec![key(Key::Z, Modifiers::COMMAND | Modifiers::CTRL)]);
    println!("that taken back (laid out again whole, as when opened): {:.2} ms", ms(t));
    assert!(w.app.script_text(script).unwrap().len() > all.len() / 2, "the text back");
    // Typing paused: the check as you type compiles the script (once a
    // pause, half a second after the last key), in the frame after.
    w.frame(vec![Event::Text("x".into())]);
    std::thread::sleep(Duration::from_millis(600));
    let t = w.frame(Vec::new());
    println!("the frame the check as you type compiles in: {:.2} ms", ms(t));

    let over: Vec<String> = medians
        .iter()
        .filter(|(_, m)| *m > BUDGET.mul_f64(MARGIN))
        .map(|(what, m)| format!("{what}: {:.2} ms", ms(*m)))
        .collect();
    assert!(
        over.is_empty(),
        "over the budget ({:.2} ms) by more than {MARGIN}:\n  {}",
        ms(BUDGET),
        over.join("\n  ")
    );
}

/// Opening Select Texture on a placeable's Visuals page (GitHub issue 32):
/// the frame that lists every texture of the game, then the frames that
/// make the pictures in sight (a few milliseconds' worth each), and one
/// after typing a name to find.
#[test]
#[ignore]
fn opening_select_texture() {
    let root = mg_testkit::corpus!();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let data = game.resman.get_named("plc_chest1", ResType::UTP).unwrap().into_owned();
    let dir = mg_testkit::scratch_dir("select-texture-perf");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.set(key, data);
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), "Visuals");
    let t = Instant::now();
    h.run();
    println!("the Visuals page, first frame (its models read): {:?}", t.elapsed());
    let t = Instant::now();
    h.step();
    println!("the Visuals page, a frame after: {:?}", t.elapsed());

    h.get_all_by_label("Select Texture…").next().unwrap().click();
    let t = Instant::now();
    h.step();
    println!("the click's frame: {:?}", t.elapsed());
    let t = Instant::now();
    h.step();
    let opened = t.elapsed();
    // (Frames until three in a row make nothing: quick ones.)
    let (mut frames, started) = (Vec::new(), Instant::now());
    while frames.len() < 300
        && (frames.len() < 3
            || frames[frames.len() - 3..].iter().any(|d| *d > std::time::Duration::from_millis(2)))
    {
        let t = Instant::now();
        h.step();
        frames.push(t.elapsed());
    }
    let making = &frames[..frames.len() - 3];
    let worst = making.iter().max().copied().unwrap_or_default();
    println!(
        "opened (every texture listed): {opened:?}; pictures in sight made over {} more \
         frames in {:?}, the longest {worst:?}",
        making.len(),
        started.elapsed() - frames[frames.len() - 3..].iter().sum::<std::time::Duration>()
    );
    let t = Instant::now();
    h.step();
    println!("a frame with them all made: {:?}", t.elapsed());
    // Scrolled a page at a time: the new pictures in sight made.
    let at = h.get_by_label("Select Texture").rect().left_top() + egui::vec2(300.0, 300.0);
    h.hover_at(at);
    let mut scrolled = Vec::new();
    for _ in 0..20 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -700.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        for _ in 0..3 {
            let t = Instant::now();
            h.step();
            scrolled.push(t.elapsed());
        }
    }
    scrolled.sort();
    println!(
        "scrolling a page at a time, 20 pages: frames median {:?}, worst {:?}",
        scrolled[scrolled.len() / 2],
        scrolled.last().unwrap()
    );
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("🔍 Find");
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    h.step();
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text("wood");
    let t = Instant::now();
    h.step();
    println!("a frame after typing \"wood\": {:?}", t.elapsed());
    // Closed and opened again: the game's textures are not listed again.
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.step();
    h.step();
    h.get_all_by_label("Select Texture…").next().unwrap().click();
    let t = Instant::now();
    h.step();
    println!("the click's frame, opened again (the list kept): {:?}", t.elapsed());
}

/// The 2DA editor on a large table, as a persistent world's appearance.2da
/// is (the game's, its rows repeated to 12,000), with a Find typed: plain
/// egui frames (no accessibility tree), idle and with a cell set at each.
/// Find's rows are worked out once for the table and the Find as they
/// are (they were looked for in every cell at each frame).
#[test]
#[ignore]
fn editing_a_large_table() {
    let root = mg_testkit::corpus!();
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let data = game.resman.get_named("appearance", ResType::TWODA).unwrap().into_owned();
    let mut t = mg_2da::TwoDa::parse(&data, game.codepage()).unwrap();
    let base = t.rows.clone();
    while t.rows.len() < 12_000 {
        t.rows.extend(base.iter().cloned());
    }
    let dir = mg_testkit::scratch_dir("editor-perf-table");
    let file = dir.join("appearance.2da");
    std::fs::write(&file, t.to_bytes(game.codepage()).unwrap()).unwrap();
    drop(game);
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    let id = mg_ui::table_view::open_path(&mut app, &file).unwrap();
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        ..Default::default()
    };
    let frames = |app: &mut Moonglow, n: usize, edit: bool| {
        let mut out = Vec::new();
        for i in 0..n {
            if edit {
                let doc = app.tables.iter_mut().find(|d| d.id == id).unwrap();
                doc.edit.set(i % 50, 1, Some(format!("x{i}"))).unwrap();
            }
            let t = Instant::now();
            let o = ctx.run_ui(input(), |ui| app.ui(ui));
            let _ = ctx.tessellate(o.shapes, o.pixels_per_point);
            out.push(t.elapsed());
        }
        out.sort();
        (out[n / 2], *out.last().unwrap())
    };
    let _ = frames(&mut app, 5, false);
    println!("{} rows: idle {:?}", t.len(), frames(&mut app, 20, false));
    app.tables.iter_mut().find(|d| d.id == id).unwrap().filter = "dragon".into();
    println!("Find typed, idle: {:?}", frames(&mut app, 20, false));
    println!("Find typed, a cell set each frame: {:?}", frames(&mut app, 20, true));
    // The same table with a custom StrRef in every row and no module (so
    // no custom talk table): every one of them a fault, each cell's looked
    // up in a map (they were each a scan of the whole list: the review's
    // D20).
    let column = t.column("STRING_REF").unwrap();
    for (i, row) in t.rows.iter_mut().enumerate() {
        row[column] = Some((16_777_216 + i).to_string());
    }
    let codepage = mg_core::Codepage::default();
    let faults = dir.join("faults").join("appearance.2da");
    std::fs::create_dir_all(faults.parent().unwrap()).unwrap();
    std::fs::write(&faults, t.to_bytes(codepage).unwrap()).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    let id = mg_ui::table_view::open_path(&mut app, &faults).unwrap();
    assert!(app.tables.iter().any(|d| d.id == id));
    let _ = frames(&mut app, 5, false);
    println!("every StrRef a fault ({} rows): idle {:?}", t.len(), frames(&mut app, 20, false));
}
