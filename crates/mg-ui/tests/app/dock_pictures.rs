//! The seven of M4's U28 pictured where they now are (Find Instance and
//! Find and Replace Text in Search Results; Build Module's dialog and what
//! it found in Problems; Verify's findings in Problems; Hak Pak Conflict
//! Analysis, Resources Used and the Claims Table as the Bottom Dock's
//! tabs): each empty, filled, running and with a row chosen at 1280 by
//! 720, and filled at 1600 by 900, 1280 by 720, the narrow layout (900 by
//! 700) and the least window with the dock at its least and tall; Dark and
//! Light. By hand:
//! `MG_U28_PICS=<folder> cargo test -p mg-ui --test app dock_pictures -- --ignored
//! --test-threads=1` (one at a time: their set-ups share scratch folders)
//! (`MG_U28_ONLY` keeps those whose name holds one of its `|`-separated
//! words).

use crate::common::*;
use mg_ui::problems::{Go, Problem, Source};

type H = Harness<'static, Moonglow>;

fn out() -> std::path::PathBuf {
    std::env::var_os("MG_U28_PICS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| mg_testkit::test_output().join("u28-pics"))
}

fn save(h: &mut H, theme: &str, size: egui::Vec2, name: &str) {
    // (The pointer gone: it hovers nothing.)
    h.event(egui::Event::PointerGone);
    settle(h);
    drawn(h, theme, size, name);
}

/// `h` drawn as it is now (a held job still under way).
fn drawn(h: &mut H, theme: &str, size: egui::Vec2, name: &str) {
    let dir = out().join(theme).join(format!("{}x{}", size.x, size.y));
    std::fs::create_dir_all(&dir).unwrap();
    h.render().expect("drawn").save(dir.join(format!("{name}.png"))).unwrap();
}

/// What a tab is set up from: its module open (`None` without what it
/// needs), then emptied, filled, chosen in and run.
struct Subject {
    name: &'static str,
    setup: fn() -> Option<H>,
    empty: fn(&mut H),
    fill: fn(&mut H),
    choose: fn(&mut H),
    /// The job a search or check runs as, held under way.
    job: Option<&'static str>,
}

/// A held job called off by its window's Cancel, where one runs.
fn cancel(h: &mut H) {
    if h.state().busy() {
        h.get_all_by_label("Cancel").last().unwrap().click();
        settle(h);
    }
}

/// A job titled `title` held under way until called off.
fn held(h: &mut H, title: &str) {
    h.state_mut().start_job_for_tests(title, |progress| {
        progress.step(2, 7);
        let started = std::time::Instant::now();
        while !progress.cancelled() && started.elapsed().as_secs() < 30 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
}

/// A scratch folder of its own for each set-up (the two tests run at once
/// and a folder taken again is emptied).
fn scratch(name: &str) -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    mg_testkit::scratch_dir(&format!("{name}-{n}"))
}

fn sample() -> Option<H> {
    let dir = scratch("u28-pics-sample");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    Some(harness(app, 1280.0, 720.0))
}

fn edit_menu(h: &mut H, item: &str) {
    menu_command(h, "Edit", item);
}

fn click_first_row(h: &mut H, starts: &str) {
    let dock = h.state().bottom_dock_rect().expect("open");
    let at = h
        .get_all_by(|n| n.label().is_some_and(|l| l.contains(starts)))
        .find(|n| n.accesskit_node().bounding_box().is_some() && dock.contains_rect(n.rect()))
        .unwrap_or_else(|| panic!("a row {starts}…"))
        .rect();
    click_pos(h, at.center());
    settle(h);
}

fn find_fill(h: &mut H) {
    let areas = ["start", "harbor", "market", "temple"];
    let kinds = [mg_area::ObjectKind::Placeable, mg_area::ObjectKind::Creature];
    let mut f = mg_ui::area_tools::FindInstance::default();
    f.results = (0..120)
        .map(|i| mg_ui::area_tools::Found {
            kind: kinds[i % 2],
            index: i,
            area: ResRef::from_str(areas[i % 4]).unwrap(),
            tag: format!("PLAGUE_VICTIM_{i}"),
            template: if i % 2 == 0 { "plc_plaguepyre".into() } else { "nw_plaguevict".into() },
        })
        .collect();
    f.tag = "plague".into();
    f.searched = true;
    h.state_mut().find_instance = Some(f);
    edit_menu(h, "Find Instance…");
}

fn text_setup() -> Option<H> {
    use mg_edit::{Command, Edit};
    let dir = scratch("u28-pics-text");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let edits = (0..30)
        .map(|i| {
            let key = ResKey::parse(&format!("camp_{i:02}"), ResType::UTW).unwrap();
            let mut g = Gff::new(*b"UTW ");
            let words = format!("Orc camp by the old road, the {i}th of the warband's fires");
            let name = LocString::from_text(Language::ENGLISH, Gender::Male, words.as_str());
            g.root.set("LocalizedName", mg_gff::Value::LocString(name));
            Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    Some(harness(app, 1280.0, 720.0))
}

fn text_fill(h: &mut H) {
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(h);
    {
        let d = h.state_mut().text_replace.as_mut().unwrap();
        d.find = "orc".into();
        d.with = "Goblin".into();
    }
    settle(h);
    // (By its action: at the least window it is in the form's scrolled
    // part.)
    h.get_by_label("Find").click_accesskit();
    settle(h);
}

fn problems_of(source: &Source, n: usize) -> Vec<Problem> {
    use mg_module::doctor::Severity;
    (0..n)
        .map(|i| {
            let key = ResKey::parse(&format!("j_script_{i:02}"), ResType::NSS).unwrap();
            let (severity, message) = match (source, i % 3) {
                (Source::Build, 0) => {
                    (Severity::Error, "ERROR: UNDEFINED IDENTIFIER (GetLastSpeaker2)")
                }
                (Source::Build, _) => (Severity::Warning, "Unused (source and compiled)"),
                (_, 0) => (Severity::Error, "Script: its source does not compile"),
                _ => (Severity::Warning, "Tag: tag \"gate_guard\" is not upper case"),
            };
            let mut p =
                Problem::new(severity, key.to_string(), message.into(), Some(Go::Resource(key)));
            if *source == Source::Build && i % 3 != 0 {
                p.unused = Some(mg_module::verify::Unused {
                    key,
                    script: Some(mg_module::verify::ScriptFiles::Both),
                });
            }
            p
        })
        .collect()
}

fn into_problems(h: &mut H, source: Source, n: usize) {
    let checked = h.state().ws.as_ref().map(|ws| ws.revision());
    let problems = problems_of(&source, n);
    h.state_mut().problems.replace(source, problems, checked);
    if h.state().bottom_dock_shows() != Some(mg_ui::Panel::Problems) {
        h.state_mut().settings.hide_log = false;
        h.get_by_label("Status Bar").get_by_label("Problems").click();
    }
    settle(h);
}

fn build_empty(h: &mut H) {
    h.state_mut().build = Some(Default::default());
    settle(h);
}

fn build_fill(h: &mut H) {
    build_empty(h);
    into_problems(h, Source::Build, 14);
}

fn verify_empty(h: &mut H) {
    h.state_mut().settings.hide_log = false;
    h.get_by_label("Status Bar").get_by_label("Problems").click();
    settle(h);
}

fn verify_fill(h: &mut H) {
    into_problems(h, Source::Verify, 14);
}

fn choose_problem(h: &mut H) {
    click_first_row(h, "Error: ");
}

fn hak_report(n: usize) -> mg_module::haks::HakReport {
    let mut r = mg_module::haks::HakReport::default();
    let haks = ["cep2_top_v28", "cep2_add_doors", "tdod_tiles", "tdod_creatures"];
    for h in haks {
        r.haks.push((h.into(), Some(format!("/home/builder/hak/{h}.hak").into())));
    }
    r.haks.push(("missing_hak".into(), None));
    for i in 0..n {
        let key = ResKey::parse(&format!("tdc01_x{i:03}"), ResType::MDL).unwrap();
        let by = vec![haks[i % 4].to_string(), haks[(i + 1) % 4].to_string()];
        r.resources.insert(key, by.clone());
        if i % 5 == 0 {
            r.overrides.insert(key, vec![by[0].clone()]);
        }
    }
    r
}

fn hak_empty(h: &mut H) {
    let mut r = hak_report(0);
    r.haks.clear();
    h.state_mut().show_hak_report(r);
    settle(h);
}

fn hak_fill(h: &mut H) {
    h.state_mut().show_hak_report(hak_report(80));
    settle(h);
}

fn used_setup() -> Option<H> {
    let (h, _) = area_harness("u28-pics-used")?;
    Some(h)
}

fn used_fill(h: &mut H) {
    menu_command(h, "Build", "Area Statistics");
}

fn claims_setup() -> Option<H> {
    let table = format!(
        "{}1   kz_gone          ****     merged   2026-10-01  KZ_GONE\n\
         2   harbor           ab       claimed  2026-11-02  ****\n",
        claims_table("cd", "claimed")
    );
    let (_, app) = claimed_project("u28-pics-claims", "[claims]\nfile = \"claims.2da\"\n", &table);
    Some(harness(app, 1280.0, 720.0))
}

fn claims_fill(h: &mut H) {
    edit_menu(h, "Claims Table…");
}

fn nothing(_: &mut H) {}

fn subjects() -> Vec<Subject> {
    vec![
        Subject {
            name: "12-Find_Instance",
            setup: sample,
            empty: |h| edit_menu(h, "Find Instance…"),
            fill: find_fill,
            choose: |h| click_first_row(h, " PLAGUE_VICTIM_"),
            job: Some("Find Instance"),
        },
        Subject {
            name: "23-Find_and_Replace_Text",
            setup: text_setup,
            empty: |h| {
                h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
                settle(h);
            },
            fill: text_fill,
            choose: |h| click_first_row(h, "camp_"),
            job: Some("Find and Replace Text"),
        },
        Subject {
            name: "21-Build_Module",
            setup: sample,
            empty: build_empty,
            fill: build_fill,
            choose: choose_problem,
            job: Some("Build Module"),
        },
        Subject {
            name: "xx-Verify",
            setup: sample,
            empty: verify_empty,
            fill: verify_fill,
            choose: choose_problem,
            job: Some("Verify Module"),
        },
        Subject {
            name: "42-Hak_Pak_Conflict_Analysis",
            setup: sample,
            empty: hak_empty,
            fill: hak_fill,
            choose: |h| click_first_row(h, "tdc01_x"),
            job: Some("Hak Pak Conflict Analysis"),
        },
        Subject {
            name: "14-Resources_Used",
            setup: used_setup,
            empty: nothing,
            fill: used_fill,
            choose: |h| click_first_row(h, "Tiles"),
            job: None,
        },
        Subject {
            name: "27-Claims_Table",
            setup: claims_setup,
            empty: nothing,
            fill: claims_fill,
            choose: |h| click_first_row(h, "start"),
            job: None,
        },
    ]
}

fn wanted(name: &str) -> bool {
    let only = std::env::var("MG_U28_ONLY").unwrap_or_default();
    only.split('|').any(|o| name.contains(o))
}

/// Each subject's states at 1280 by 720, the dock as it opens.
#[test]
#[ignore]
fn every_tab_s_states_pictured() {
    mg_testkit::gpu::hold();
    let size = egui::vec2(1280.0, 720.0);
    for s in subjects().into_iter().filter(|s| wanted(s.name)) {
        for light in [false, true] {
            let theme = if light { "light" } else { "dark" };
            let shot = |state: &str, steps: &[fn(&mut H)]| {
                let Some(mut h) = (s.setup)() else { return };
                h.state_mut().settings.light_theme = light;
                h.set_size(size);
                settle(&mut h);
                for step in steps {
                    step(&mut h);
                }
                save(&mut h, theme, size, &format!("{} ({state})", s.name));
                // (A held job called off: the window rests.)
                cancel(&mut h);
            };
            shot("empty", &[s.empty]);
            shot("filled", &[s.fill]);
            shot("chosen", &[s.fill, s.choose]);
            if let Some(job) = s.job {
                let Some(mut h) = (s.setup)() else { continue };
                h.state_mut().settings.light_theme = light;
                h.set_size(size);
                settle(&mut h);
                (s.empty)(&mut h);
                held(&mut h, job);
                h.event(egui::Event::PointerGone);
                crate::dialog_pattern::rested(&mut h, job, true);
                assert!(h.state().busy(), "{job} still under way");
                drawn(&mut h, theme, size, &format!("{} (running)", s.name));
                cancel(&mut h);
            }
        }
    }
}

/// Each subject filled at every size, the dock at its least and tall.
#[test]
#[ignore]
fn every_tab_at_every_size_pictured() {
    mg_testkit::gpu::hold();
    let sizes = [
        egui::vec2(1600.0, 900.0),
        egui::vec2(1280.0, 720.0),
        egui::vec2(900.0, 700.0),
        mg_ui::LEAST_WINDOW_SIZE,
    ];
    for s in subjects().into_iter().filter(|s| wanted(s.name)) {
        for light in [false, true] {
            let theme = if light { "light" } else { "dark" };
            for size in sizes {
                for (dock, high) in [("least", 0), ("tall", 2000)] {
                    let Some(mut h) = (s.setup)() else { return };
                    h.state_mut().settings.light_theme = light;
                    h.set_size(size);
                    settle(&mut h);
                    (s.fill)(&mut h);
                    dock_high(&mut h, high);
                    settle(&mut h);
                    save(&mut h, theme, size, &format!("{} (dock {dock})", s.name));
                }
            }
        }
    }
}
