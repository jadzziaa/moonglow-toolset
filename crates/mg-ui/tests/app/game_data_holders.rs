//! Who holds the game data (issue #49): once the window has let go of it
//! (`release_game_data`, before every change of its layers: a Save's
//! module layer, New Module, the haks), the window alone holds it,
//! whatever was used before. A surface that keeps a copy of the game
//! data (egui's memory, a thread left running) made every Save wait ten
//! seconds for it, and left the haks of a module opened after it out.

use crate::common::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How many hold the game data besides the window once it has let go of
/// it (as Tools › Reload Resources has it let go), and how long that took.
fn others_after_release(h: &mut Harness<'_, Moonglow>) -> (usize, Duration) {
    settle(h);
    assert!(!h.state().busy(), "no job runs");
    let started = Instant::now();
    h.state_mut().reload_resources(false);
    let waited = started.elapsed();
    (Arc::strong_count(h.state().game.as_ref().expect("game data")) - 1, waited)
}

/// One surface used, then what held the game data after it.
fn after(
    h: &mut Harness<'_, Moonglow>,
    found: &mut Vec<(&'static str, usize, Duration)>,
    what: &'static str,
) {
    let (others, waited) = others_after_release(h);
    found.push((what, others, waited));
}

/// A copy of the standard blueprint `name` in the module, as the module's
/// own `copy`.
fn copied(h: &mut Harness<'_, Moonglow>, name: &str, copy: &str, t: ResType) -> ResKey {
    let app = h.state_mut();
    let data = app.game.as_deref().unwrap().resman.get_named(name, t).unwrap().into_owned();
    let key = ResKey::parse(copy, t).unwrap();
    let edit = mg_edit::Edit::SetResource { key, data: Some(data) };
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("copy", vec![edit])));
    settle(h);
    key
}

/// The surfaces a builder uses with an area open, each in turn (the
/// palette's placeables, the Placeable Wizard and the Creature Wizard
/// shown and closed, the Appearance Gallery, a script, a creature's and an
/// item's Properties, New Module and its window): after each, the window
/// let go of the game data alone holds it, at once. On a GPU-free window:
/// what the 3D views hold is `release_game_data`'s own to let go (the GPU
/// tests in `area_view.rs`).
#[test]
fn only_the_window_holds_the_game_data_once_it_lets_go() {
    let Some((mut h, _area)) = area_ui_harness("holders") else { return };
    let mut found = Vec::new();
    after(&mut h, &mut found, "an area open");

    // The palette's placeables, one taken to place.
    {
        let app = h.state_mut();
        show_palette(app);
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.tiles = false;
        app.palette.filter = "plc_chest1".into();
    }
    settle(&mut h);
    let name = {
        let game = h.state().game.as_deref().unwrap();
        let utp = game.resman.get_named("plc_chest1", ResType::UTP).unwrap();
        let utp = Gff::read(&utp).unwrap();
        mg_module::palette::blueprint_name(
            mg_module::palette::BlueprintKind::Placeable,
            &utp.root,
            game,
        )
    };
    palette_row(&h, &name).click();
    settle(&mut h);
    after(&mut h, &mut found, "the palette's placeables, one taken to place");

    // The Placeable Wizard, through its pages, then closed.
    h.state_mut().blueprint_wizard = Some(mg_ui::blueprint_wizard::BlueprintWizard::new(
        mg_module::palette::BlueprintKind::Placeable,
    ));
    settle(&mut h);
    for _ in 0..4 {
        if let Some(next) = h.query_by_label("Next >") {
            next.click();
            settle(&mut h);
        }
    }
    after(&mut h, &mut found, "the Placeable Wizard open");
    h.state_mut().blueprint_wizard = None;
    settle(&mut h);
    after(&mut h, &mut found, "the Placeable Wizard closed");

    // The Creature Wizard's appearance page, then closed.
    let mut w = mg_ui::creature_wizard::CreatureWizard::default();
    (w.page, w.race, w.appearance) = (3, Some(6), 6);
    w.classes = vec![(4, 1)];
    h.state_mut().creature_wizard = Some(w);
    settle(&mut h);
    after(&mut h, &mut found, "the Creature Wizard open");
    h.state_mut().creature_wizard = None;
    settle(&mut h);
    after(&mut h, &mut found, "the Creature Wizard closed");

    // The Appearance Gallery.
    h.state_mut().actions.push(mg_ui::Action::PlaceableGallery);
    settle(&mut h);
    after(&mut h, &mut found, "the Appearance Gallery");

    // A script.
    let nss = ResKey::parse("hello", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource {
        key: nss,
        data: Some(b"void main() { PrintString(\"hi\"); }\n".to_vec()),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("script", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss)));
    settle(&mut h);
    after(&mut h, &mut found, "a script open");

    // A creature's and an item's Properties.
    let utc = copied(&mut h, "nw_bandit001", "holder_bandit", ResType::UTC);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(utc)));
    settle(&mut h);
    after(&mut h, &mut found, "a creature's Properties");
    let uti = copied(&mut h, "nw_it_torch001", "holder_torch", ResType::UTI);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(uti)));
    settle(&mut h);
    after(&mut h, &mut found, "an item's Properties");

    // Save (the issue's Ctrl+S: the module layer made again, the game
    // data let go of first), timed.
    let started = Instant::now();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(2);
    let save = started.elapsed();
    settle(&mut h);
    assert!(!h.state().has_unsaved_work(), "saved: {:?}", h.state().log.entries);
    after(&mut h, &mut found, "Save");

    // New Module, its window shown, a while.
    let started = Instant::now();
    h.state_mut().actions.push(mg_ui::Action::NewModule("Fresh".into()));
    h.run_steps(2);
    let new_module = started.elapsed();
    settle(&mut h);
    assert!(h.state().ws.is_some() && h.state().module_path().is_none(), "the new module open");
    after(&mut h, &mut found, "New Module's window");
    let in_use = (h.state().log.entries.iter()).filter(|(_, m)| m.contains("in use")).count();

    let report: Vec<String> = (found.iter())
        .map(|(what, others, waited)| format!("{what}: {others} other(s), let go in {waited:.0?}"))
        .collect();
    eprintln!("game data holders:\n  {}", report.join("\n  "));
    eprintln!("Save took {save:.0?}, New Module {new_module:.0?}");
    assert!(save < Duration::from_secs(5), "Save took {save:.0?}");
    assert!(
        found.iter().all(|(_, others, waited)| *others == 0 && *waited < Duration::from_secs(3)),
        "the game data held after:\n  {}",
        report.join("\n  ")
    );
    assert!(new_module < Duration::from_secs(5), "New Module took {new_module:.0?}");
    assert_eq!(in_use, 0, "{:?}", h.state().log.entries);
}

/// A picture of one color, 8 by 8 pixels.
pub(crate) fn swatch(rgba: [u8; 4]) -> Vec<u8> {
    let mut img = mg_image::Rgba::new(8, 8);
    for px in img.data.chunks_mut(4) {
        px.copy_from_slice(&rgba);
    }
    mg_image::write_tga(&img)
}

pub(crate) const RED: [u8; 4] = [255, 0, 0, 255];
pub(crate) const BLUE: [u8; 4] = [0, 0, 255, 255];
pub(crate) const GREEN: [u8; 4] = [0, 255, 0, 255];

/// A hak of ours, `name`, in `user`'s `hak` folder, with texture
/// `texture` of one color.
pub(crate) fn swatch_hak(user: &std::path::Path, name: &str, texture: &str, rgba: [u8; 4]) {
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    w.add(ResRef::from_str(texture).unwrap(), ResType::TGA, swatch(rgba)).unwrap();
    std::fs::create_dir_all(user.join("hak")).unwrap();
    std::fs::write(user.join(format!("hak/{name}.hak")), w.to_bytes().unwrap()).unwrap();
}

/// `module` (a module file) made to list `haks`, in that order.
pub(crate) fn list_haks(module: &std::path::Path, haks: &[&str]) {
    let mut m = Module::open(module).unwrap();
    let mut info = m.info().unwrap();
    let haks: Vec<String> = haks.iter().map(|h| h.to_string()).collect();
    info.root.set("Mod_HakList", mg_module::attach::hak_list(&info.root, &haks));
    m.set_info(&info).unwrap();
    m.save().unwrap();
}

/// The game data's layers of a module: its haks and itself, in order.
fn module_layers(app: &Moonglow) -> Vec<String> {
    let game = app.game.as_deref().expect("the game data");
    (game.resman.layers().iter())
        .filter(|l| l.priority == mg_resman::priority::MODULE || l.label.starts_with("hak:"))
        .map(|l| l.label.clone())
        .collect()
}

/// The texture of the name the haks share, as the game data has it.
fn texture(app: &Moonglow) -> Option<Vec<u8>> {
    let game = app.game.as_deref()?;
    game.resman.get_named(SWATCH, ResType::TGA).ok().map(|d| d.into_owned())
}

const SWATCH: &str = "mg_swatch";

/// Module A, listing a hak whose `mg_swatch` is red, closed and module B
/// opened (its two haks: blue, then green) while a job holds the game
/// data (issue #50, where a copy kept by mistake held it): nothing is
/// waited for, the log says once what waits for the game data, and the
/// Status Bar while it waits; the job done, B's haks are layered in their
/// order and A's are gone, as an open with nothing holding it has them,
/// and the log says so.
#[test]
fn a_module_opened_while_the_game_data_is_held_has_its_haks_once_it_is_free() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-haks-wait");
    let user = user_folder(&dir, &["hak"]);
    swatch_hak(&user, "mg_red", SWATCH, RED);
    swatch_hak(&user, "mg_blue", SWATCH, BLUE);
    swatch_hak(&user, "mg_green", SWATCH, GREEN);
    let (a_dir, b_dir) = (dir.join("a"), dir.join("b"));
    std::fs::create_dir_all(&a_dir).unwrap();
    std::fs::create_dir_all(&b_dir).unwrap();
    let (a, b) = (sample_module(&a_dir), sample_module(&b_dir));
    list_haks(&a, &["mg_red"]);
    list_haks(&b, &["mg_blue", "mg_green"]);
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&a);
    let mut h = window_harness(app);
    settle(&mut h);
    assert_eq!(module_layers(h.state()), ["hak:mg_red", "module"]);
    assert_eq!(texture(h.state()), Some(swatch(RED)));

    // A job holds the game data until it is let go.
    let (go, held) = std::sync::mpsc::channel::<()>();
    h.state_mut().start_job_for_tests("Held", move |_| {
        let _ = held.recv();
    });
    assert!(h.state().busy());
    let logged = h.state().log.entries.len();
    let started = Instant::now();
    h.state_mut().close();
    h.state_mut().open_module(&b);
    let took = started.elapsed();
    assert!(took < Duration::from_secs(3), "waited {took:.0?} under a job");
    let said: Vec<String> =
        h.state().log.entries[logged..].iter().map(|(_, m)| m.clone()).collect();
    let waits: Vec<&String> =
        said.iter().filter(|m| m.starts_with("The game data is still in use")).collect();
    assert_eq!(waits.len(), 1, "said once: {said:#?}");
    assert!(waits[0].contains("the module's haks and resources"), "{}", waits[0]);
    assert!(h.state().game_changes_waiting());
    // (Not yet: A's haks are still there.)
    assert_eq!(module_layers(h.state()), ["hak:mg_red", "module"]);
    h.run_steps(2);
    let note = "Waiting for the game data to be free…";
    assert!(
        h.get_by_label("Status Bar").query_by_label(note).is_some(),
        "the Status Bar says so: {:?}",
        h.state().status_said()
    );

    // The job ends: the waiting changes are made.
    go.send(()).unwrap();
    let started = Instant::now();
    while h.state().busy() || h.state().game_changes_waiting() {
        assert!(started.elapsed() < Duration::from_secs(20), "never made");
        std::thread::sleep(Duration::from_millis(5));
        h.step();
    }
    let layered = module_layers(h.state());
    assert_eq!(layered, ["hak:mg_blue", "hak:mg_green", "module"]);
    assert_eq!(texture(h.state()), Some(swatch(BLUE)), "B's first hak's, not A's red");
    let entries = &h.state().log.entries;
    assert!(
        entries.iter().any(|(_, m)| m.starts_with("The game data is free again")),
        "{entries:#?}"
    );
    h.run_steps(2);
    assert!(h.get_by_label("Status Bar").query_by_label(note).is_none(), "nothing waits");

    // As B opened with nothing holding the game data has them.
    h.state_mut().close();
    assert_eq!(module_layers(h.state()), Vec::<String>::new());
    h.state_mut().open_module(&b);
    assert_eq!(module_layers(h.state()), layered);
    assert!(!h.state().game_changes_waiting());
}
