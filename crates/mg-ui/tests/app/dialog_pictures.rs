//! Every dialog of the inventory pictured (M4, U27's pictures): each
//! opener of `dialog_pattern`'s tables in Dark and Light, at 1600 by 900,
//! 1280 by 720 and the least window, the Creature Wizard at each of its
//! nine steps and the job window running and done. By hand:
//! `MG_U27_PICS=<folder> cargo test -p mg-ui --test app dialog_pictures -- --ignored`
//! (`MG_U27_ONLY` keeps the dialogs whose title holds it, or one of its
//! `|`-separated words).

use crate::common::*;
use crate::dialog_pattern::{Opener, rested};

fn out() -> std::path::PathBuf {
    std::env::var_os("MG_U27_PICS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| mg_testkit::test_output().join("u27-pics"))
}

/// Shown and at rest: [`rested`] waits for the window's work (a job's
/// window by its own stillness, as it repaints while it runs), and the
/// fade in, which asks for frames until it ends (one test step of 0.25 s
/// is longer than egui's fade), with it.
fn shown(h: &mut Harness<'static, Moonglow>, title: &str, job: bool) {
    rested(h, title, job);
}

/// `h` drawn into `<out>/<theme>/<size>/<name>.png`.
fn save(h: &mut Harness<'static, Moonglow>, theme: &str, size: egui::Vec2, name: &str) {
    let dir = out().join(theme).join(format!("{}x{}", size.x, size.y));
    std::fs::create_dir_all(&dir).unwrap();
    let file: String =
        name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    h.render().expect("drawn").save(dir.join(format!("{file}.png"))).unwrap();
}

fn sizes() -> [egui::Vec2; 3] {
    [egui::vec2(1600.0, 900.0), egui::vec2(1280.0, 720.0), mg_ui::LEAST_WINDOW_SIZE]
}

fn picture(op: &Opener, n: usize) {
    // (Several titles separated by `|`.)
    let only = std::env::var("MG_U27_ONLY").unwrap_or_default();
    if !only.split('|').any(|o| op.title.contains(o)) {
        return;
    }
    for light in [false, true] {
        let theme = if light { "light" } else { "dark" };
        for size in sizes() {
            let Some(mut h) = (op.setup)() else { return };
            h.state_mut().settings.light_theme = light;
            h.set_size(size);
            settle(&mut h);
            (op.open)(&mut h);
            shown(&mut h, op.title, op.job);
            save(&mut h, theme, size, &format!("{n:02}-{}", op.title));
            // (Its further states, as the table holds them.)
            for (state, step) in &op.states {
                step(&mut h);
                shown(&mut h, op.title, op.job);
                save(&mut h, theme, size, &format!("{n:02}-{} ({state})", op.title));
            }
        }
    }
}

#[test]
#[ignore]
fn every_dialog_pictured() {
    mg_testkit::gpu::hold();
    for (n, op) in crate::dialog_pattern::all().iter().enumerate() {
        picture(op, n);
    }
}

/// The Creature Wizard at each of its nine steps, everything chosen (as
/// `editor_perf`'s view), in both themes at both sizes and the least
/// window.
#[test]
#[ignore]
fn the_creature_wizard_s_steps_pictured() {
    mg_testkit::gpu::hold();
    for light in [false, true] {
        let theme = if light { "light" } else { "dark" };
        for size in sizes() {
            let Some(mut h) = game_harness("u27-pics-steps") else { return };
            h.state_mut().settings.light_theme = light;
            h.set_size(size);
            settle(&mut h);
            for page in 0..9 {
                h.state_mut().creature_wizard = Some(mg_ui::creature_wizard::CreatureWizard {
                    page,
                    race: Some(6),
                    classes: vec![(4, 4)],
                    chosen_class: Some(4),
                    portrait: Some(1),
                    first_name: "Perf".into(),
                    category: Some(0),
                    ..Default::default()
                });
                shown(&mut h, "Creature Wizard", false);
                save(&mut h, theme, size, &format!("creature-wizard-step-{}", page + 1));
            }
        }
    }
}

/// The job window running (two of five), and done (gone: the Status Bar
/// and the log say so), in both themes at both sizes.
#[test]
#[ignore]
fn the_job_window_pictured() {
    mg_testkit::gpu::hold();
    for light in [false, true] {
        let theme = if light { "light" } else { "dark" };
        for size in sizes() {
            let mut app = app_with(Vec::new());
            app.settings.light_theme = light;
            let mut h = harness(app, size.x, size.y);
            settle(&mut h);
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            h.state_mut().start_job_for_tests("Compiling Scripts", move |progress| {
                progress.step(2, 5);
                progress.say("nw_c2_default9.nss");
                let _ = rx.recv_timeout(std::time::Duration::from_secs(30));
            });
            // (Until its thread's step and words are shown, then at rest.)
            let started = std::time::Instant::now();
            while h.query_by_label_contains("nw_c2_default9.nss").is_none() {
                assert!(started.elapsed().as_secs() < 20, "the job's words shown");
                h.step();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            shown(&mut h, "Compiling Scripts", true);
            save(&mut h, theme, size, "job-running");
            tx.send(()).unwrap();
            settle(&mut h);
            save(&mut h, theme, size, "job-done");
        }
    }
}
