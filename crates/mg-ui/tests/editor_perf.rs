//! How fast the editors are on very large files: the script editor on the
//! game's nwscript.nss (~14,000 lines), held to the 144 fps budget, and
//! the 2DA editor and Select Texture, timed. Run with
//! `cargo test --release -p mg-ui --test editor_perf -- --ignored --nocapture`
//! (`EDITOR_FRAMES=<n>` times that many frames of each, for a profiler to
//! sample, `EDITOR_ONLY=<part of a name>` only those, and `EDITOR_DETAIL=1`
//! prints each frame's time); `scripts_at_scale` the review of November
//! 2026's script editor findings at their sizes, each against its own rest
//! (`EDITOR_VIEW=<part of a name>` picks them).

use std::time::{Duration, Instant};

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use mg_core::ResType;
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Moonglow, NoDialogs, Tab};

#[allow(dead_code)]
mod timing;

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
    // (As Tabs, as the other cases: the script in the main window's
    // editor area, as wide as the screen gives it, not a window left at
    // the size the harness gave it.)
    app.settings.open_editors = mg_ui::Settings::new_installation().open_editors;
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
    // (The pointer over the script's text, so that the wheel scrolls it:
    // it moves down a few turns of the wheel, else this measures another
    // tab's frames.)
    // (The view at rest first: the page keys before leave it gliding to
    // the cursor for a few frames, which a wheel's turn would be measured
    // against, M4's U13.)
    for _ in 0..30 {
        w.frame(Vec::new());
    }
    let before = w.app.script_scroll_for_test(script).unwrap_or(0.0);
    for _ in 0..4 {
        w.frame(vec![
            Event::PointerMoved(middle_of),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -120.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    for _ in 0..30 {
        w.frame(Vec::new());
    }
    let after = w.app.script_scroll_for_test(script).unwrap_or(0.0);
    assert!(after > before + 200.0, "the wheel scrolled the script: {before} to {after}");
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
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
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

/// The Creature Wizard (M4's U27: its step list and page bar) over the
/// boards' module at the development machine's screen, on each of its
/// nine steps with what the step before asks for chosen: plain egui
/// frames, idle and with the pointer moving over it, each step's medians
/// printed and held to the 144 fps budget. Laid out alike in the tree
/// before U27 (it sets the wizard's state, it clicks nothing by name).
#[test]
#[ignore]
fn the_creature_wizard_at_each_step() {
    let root = mg_testkit::corpus!();
    let install = GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let dir = mg_testkit::scratch_dir("creature-wizard-perf");
    let path = dir.join("Chapter1.nwm");
    std::fs::copy(root.join("data/nwm/Chapter1.nwm"), &path).unwrap();
    app.open_module(&path);
    let count: usize =
        std::env::var("EDITOR_FRAMES").ok().and_then(|n| n.parse().ok()).unwrap_or(200);
    let mut w = Window { app, ctx: egui::Context::default(), clock: 0.0 };
    w.frame(Vec::new());
    let mut medians = Vec::new();
    for page in 0..9 {
        w.app.creature_wizard = Some(mg_ui::creature_wizard::CreatureWizard {
            page,
            race: Some(6),
            classes: vec![(4, 4)],
            chosen_class: Some(4),
            portrait: Some(1),
            first_name: "Perf".into(),
            category: Some(0),
            ..Default::default()
        });
        // (Its pictures read, its lists measured: a few seconds' frames.)
        for _ in 0..300 {
            w.frame(Vec::new());
        }
        let what = format!("the Creature Wizard, step {} of 9", page + 1);
        let idle = w.frames(&format!("{what}, idle"), count, |_| Vec::new());
        let hover = w.frames(&format!("{what}, the pointer moving"), count, |n| {
            let x = 1100.0 + (n as f32 * 13.0) % 1200.0;
            let y = 300.0 + (n as f32 * 7.0) % 800.0;
            vec![Event::PointerMoved(egui::pos2(x, y))]
        });
        medians.push((what, idle, hover));
    }
    w.app.creature_wizard = None;
    for (what, idle, hover) in medians {
        for (how, median) in [("idle", idle), ("the pointer moving", hover)] {
            assert!(
                median.as_secs_f64() <= BUDGET.as_secs_f64() * MARGIN,
                "{what}, {how}: median {:.2} ms past the budget",
                ms(median)
            );
        }
    }
}

/// The boards' creature's Properties docked across the development
/// machine's screen (3440×1400, As Tabs): its fourteen pages side by side
/// in columns, every one drawn each frame (M4's U10, the WideCreature
/// board), held to the 144 fps budget idle and with the pointer moving
/// over them.
#[test]
#[ignore]
fn a_wide_creature_every_page_drawn() {
    let root = mg_testkit::corpus!();
    let install = GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    // (The boards' creature, Faribeth of Chapter1: a copy of the game's
    // module in a scratch folder.)
    let key = ResKey::parse("m1q1faribeth", ResType::UTC).unwrap();
    let dir = mg_testkit::scratch_dir("wide-creature-perf");
    let path = dir.join("Chapter1.nwm");
    std::fs::copy(root.join("data/nwm/Chapter1.nwm"), &path).unwrap();
    app.settings.open_editors = mg_ui::Settings::new_installation().open_editors;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    let count: usize =
        std::env::var("EDITOR_FRAMES").ok().and_then(|n| n.parse().ok()).unwrap_or(200);
    let mut w = Window { app, ctx: egui::Context::default(), clock: 0.0 };
    // (Its pictures read and its pages measured: a few seconds' frames.)
    let t = Instant::now();
    w.frame(Vec::new());
    println!("the wide creature, first frame: {:.2} ms", ms(t.elapsed()));
    for _ in 0..300 {
        w.frame(Vec::new());
    }
    let idle = w.frames("the wide creature, idle, every page drawn", count, |_| Vec::new());
    let hover = w.frames("the wide creature, the pointer moving over its pages", count, |n| {
        let x = 600.0 + (n as f32 * 13.0) % 2600.0;
        let y = 150.0 + (n as f32 * 7.0) % 1100.0;
        vec![Event::PointerMoved(egui::pos2(x, y))]
    });
    for (what, median) in [("idle", idle), ("the pointer moving", hover)] {
        assert!(
            median.as_secs_f64() <= BUDGET.as_secs_f64() * MARGIN,
            "{what}: median {:.2} ms past the budget",
            ms(median)
        );
    }
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
    // (Docked, As Tabs, a cell of a talk-table string's column chosen, so
    // that the docks' panels a table brings are drawn beside its grid each
    // frame where it brings them: M4's U16's Cell, References and the
    // Outliner's columns; and the Left Dock's Module panel over them.)
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    // (A module open, as the board's: the Left Dock's Module panel over
    // the Outliner; without one the dock holds the Outliner alone.)
    let module = dir.join("Chapter1.nwm");
    std::fs::copy(root.join("data/nwm/Chapter1.nwm"), &module).unwrap();
    app.settings.no_last_area = true;
    app.open_module(&module);
    let id = mg_ui::table_view::open_path(&mut app, &file).unwrap();
    let strings = t.column("STRING_REF").unwrap();
    app.tables.iter_mut().find(|d| d.id == id).unwrap().selected = Some((9, strings));
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        // (The Table board's 1600 by 900: from 1400 the Right Dock stands
        // in its sections, Cell over References, beside the Outliner.)
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0))),
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
    assert_eq!(app.active_editor(), Some(&Tab::Table(id)), "the table docked, in front");
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

/// The script header's control `name`, through its More where it went
/// there (as `tests/app`'s `script_tool`).
fn script_tool<'t>(h: &'t mut Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    use mg_uikit::header::{self, Place};
    let records = header::records(&h.ctx, mg_ui::SCRIPT_HEADER);
    let rec = records.into_iter().next().expect("the script header");
    let place = rec.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p);
    static MORE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let more = MORE.get_or_init(|| header::more_name(mg_ui::SCRIPT_HEADER)).as_str();
    if place == Some(Place::More) && h.query_by_label(more).is_none() {
        h.get_by_label(mg_ui::SCRIPT_HEADER).get_by_label(header::MORE).click();
        h.run_steps(3);
    }
    let h: &'t Harness<'_, Moonglow> = h;
    match place {
        Some(Place::More) => h.get_by_label(more).get_by_label(name),
        Some(Place::Row | Place::Icon) => h.get_by_label(mg_ui::SCRIPT_HEADER).get_by_label(name),
        other => panic!("{name} in the script header: {other:?}"),
    }
}

/// How many options the plugin panel's choices have: the most a choice
/// may have (`MAX_CHOICES`), and two.
const CHOICES: [usize; 2] = [2, 2_000];

/// A plugin of two panels, `few` and `many`, each a choice of so many
/// options ([`CHOICES`]), written to `dir`.
fn choice_plugin(dir: &std::path::Path) {
    let plugin = dir.join("choices");
    std::fs::create_dir_all(&plugin).unwrap();
    let mut cfg = "[plugin]\nid = \"test.choices\"\nname = \"Choices\"\nversion = \"0.0.1\"\n\
                   api = \"0.9\"\nlicense = \"GPL-3.0-or-later\"\n"
        .to_owned();
    let mut code = "local mg = require(\"@moonglow\")\n".to_owned();
    for (id, n) in ["few", "many"].iter().zip(CHOICES) {
        cfg += &format!("\n[panel]\nid = \"{id}\"\ntitle = \"Choice of {n}\"\n");
        code += &format!(
            "mg.panel(\"{id}\", function(ctx)\n\
             local choices = {{}}\n\
             for i = 1, {n} do choices[i] = \"Option \" .. i end\n\
             return {{ {{ type = \"field\", kind = \"choice\", id = \"c\", label = \"Choice\", \
             choices = choices, value = \"Option 1\" }} }}\n\
             end)\n"
        );
    }
    std::fs::write(plugin.join("plugin.cfg"), cfg).unwrap();
    std::fs::write(plugin.join("main.luau"), code).unwrap();
}

/// The review of November 2026's script editor findings at the sizes they
/// name (the deferred list's "performance fixes without a timing view"),
/// each against the same view at a small size, timed in the same run, its
/// rest timed twice for the noise, in a module of 8,000 scripts beside a
/// copy of the game's `nwscript.nss` (a word at its first line and at its
/// last): Search Results after Find In Files for a word on 56,000 of their
/// lines (10,000 kept) against one on 100, more than are in sight (X8); Open Script with All
/// Resources (the game's scripts and the 8,000), everything selected
/// (Ctrl+A) against nothing (X10); the copy's last line with a completion
/// list open against its first (X18); and a plugin's panel holding a
/// choice of 2,000 options, closed, against one of two (X5).
/// `EDITOR_VIEW=<part of a name>` picks the groups ("Search Results",
/// "Open Script", "completion", "choice"), `EDITOR_FRAMES` the frames.
#[test]
#[ignore]
fn scripts_at_scale() {
    let root = mg_testkit::corpus!();
    let mut v = timing::Views::new("EDITOR_VIEW", "EDITOR_FRAMES");
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let nwscript = game.resman.get_named("nwscript", ResType::NSS).unwrap().into_owned();
    drop(game);
    let dir = mg_testkit::scratch_dir("editor-perf-scripts");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let big = ResKey::parse("big", ResType::NSS).unwrap();
    let mut text = b"int a = GetLoc\n".to_vec();
    text.extend_from_slice(&nwscript);
    text.extend_from_slice(b"\nint n = GetLoc");
    m.set(big, text);
    for i in 0..8_000 {
        let key = ResKey::parse(&format!("mg_s_{i:04}"), ResType::NSS).unwrap();
        let mut body: String = (0..7).map(|j| format!("// word {i} {j}\n")).collect();
        if i % 80 == 0 {
            body += "// rarely\n";
        }
        body += "void main() { }\n";
        m.set(key, body.into_bytes());
    }
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let plugins = dir.join("plugins");
    choice_plugin(&plugins);

    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.open_module(&path);
    app.plugin_dir = Some(plugins);
    app.load_plugins();
    app.enable_plugin("test.choices", true);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(big)));
    let mut recorded = false;
    let mut h = Harness::builder().with_size(SCREEN).build_ui_state(
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
    h.run_steps(10);

    // X18: a completion list open at the first line, then at the last.
    let name = "a completion list";
    if v.wanted(name) {
        let id = egui::Id::new(("script", big));
        let chars = h.state().script_text(big).unwrap().chars().count();
        let complete_at = |h: &mut Harness<'_, Moonglow>, at: usize| {
            let mut state = egui::text_edit::TextEditState::load(&h.ctx, id).unwrap_or_default();
            let c = egui::text::CCursor::new(at);
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(c)));
            state.store(&h.ctx, id);
            h.ctx.memory_mut(|m| m.request_focus(id));
            h.state_mut().script_tools.completion = None;
            h.run_steps(4);
            // (Again until the script's names are read, on a thread.)
            let begun = Instant::now();
            while h.state().script_tools.completion.is_none() {
                h.key_press(Key::F2);
                h.run_steps(4);
                assert!(begun.elapsed() < Duration::from_secs(60), "{name}: no list at {at}");
            }
        };
        complete_at(&mut h, "int a = GetLoc".len());
        let rest = v.rest(&mut h);
        complete_at(&mut h, chars);
        let what = format!("{name} at the last of {chars} characters (against the first line)");
        v.against(&mut h, &what, rest, |_| Vec::new());
        assert!(h.state().script_tools.completion.is_some(), "{name}: still open");
        h.state_mut().script_tools.completion = None;
        h.key_press(Key::Escape);
        h.run_steps(4);
    }

    // X8: Search Results after Find In Files.
    let name = "Search Results";
    if v.wanted(name) {
        let find_all = |h: &mut Harness<'_, Moonglow>, word: &str| {
            let search = &mut h.state_mut().script_tools.search;
            (search.open, search.script, search.in_files) = (true, Some(big), true);
            search.find = word.into();
            h.run_steps(3);
            h.get_by_label("Find All").click();
            h.run_steps(4);
            h.state_mut().script_tools.search.open = false;
            h.run_steps(6);
            h.state().search_results.hits.len()
        };
        assert_eq!(find_all(&mut h, "rarely"), 100, "{name}: the rare word's lines");
        let rest = v.rest(&mut h);
        assert_eq!(find_all(&mut h, "word"), 10_000, "{name}: the common word's, cut");
        // (A row: the script's name, then its line.)
        let rows = h.query_all_by_label_contains("mg_s_").count();
        assert!((1..400).contains(&rows), "{name}: {rows} rows in sight");
        let what = format!("{name} of 10,000 lines kept of 56,000 found (against 100)");
        v.against(&mut h, &what, rest, |_| Vec::new());
    }

    // X10: Open Script with All Resources, everything selected.
    let name = "Open Script";
    if v.wanted(name) {
        script_tool(&mut h, "Open…").click();
        h.run_steps(4);
        h.get_by_label("All Resources").click();
        h.run_steps(4);
        let listed = h.state().script_tools.open_script.as_ref().unwrap().scripts.len();
        assert!(listed > 12_000, "{name}: {listed} scripts");
        let rest = v.rest(&mut h);
        h.key_press_modifiers(Modifiers::COMMAND, Key::A);
        h.run_steps(4);
        let selected = h.state().script_tools.open_script.as_ref().unwrap().selected.len();
        assert!(selected > 12_000, "{name}: {selected} of {listed} selected");
        let what = format!("{name}, All Resources, {selected} selected (against none)");
        v.against(&mut h, &what, rest, |_| Vec::new());
        h.key_press(Key::Escape);
        h.run_steps(2);
        h.key_press(Key::Escape);
        h.run_steps(4);
        assert!(h.state().script_tools.open_script.is_none(), "{name}: closed");
    }

    // X5: a plugin's panel with a choice of 2,000 options, closed.
    let name = "a plugin panel's choice";
    if v.wanted(name) {
        let mut rests = Vec::new();
        for (id, n) in ["few", "many"].iter().zip(CHOICES) {
            let tab = Tab::PluginPanel { plugin: "test.choices".into(), panel: (*id).into() };
            h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
            let begun = Instant::now();
            // (Its field's label: the box shows its value.)
            while h.query_all_by_label("Choice").next().is_none() {
                h.run_steps(1);
                assert!(begun.elapsed() < Duration::from_secs(60), "{name}: {id} not drawn");
            }
            h.run_steps(4);
            match rests.pop() {
                None => rests.push(v.rest(&mut h)),
                Some(rest) => {
                    let what = format!("{name} of {n} options, closed (against 2)");
                    v.against(&mut h, &what, rest, |_| Vec::new());
                }
            }
            let at = h.state().dock.find_tab(&tab).unwrap();
            h.state_mut().dock.remove_tab(at);
            h.run_steps(4);
        }
    }

    println!(
        "against their resting views (the line {} µs, the budget {:.2} ms):\n  {}",
        timing::UI_LINE.as_micros(),
        ms(timing::BUDGET),
        v.lines.join("\n  ")
    );
}
