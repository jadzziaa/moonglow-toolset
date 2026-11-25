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

/// The same on the GPU, with the 3D views drawing (issues #49 and #50).
/// These take the GPU (`gpu()`, `area_harness`): the session that may
/// use it runs them, `cargo test -p mg-ui --test app game_data_holders::gpu::`.
mod gpu {
    use super::*;

    /// An area's window on the GPU, its area read and drawn apart from the
    /// window as in the desktop app (`background_jobs`), with the game's
    /// user folder `user`; not yet drawn.
    fn drawing_area(
        name: &str,
        user: Option<std::path::PathBuf>,
    ) -> Option<(Harness<'static, Moonglow>, ResRef)> {
        let (mut app, rs, area) = area_app(name, "ttr01", None, 4, user, &|_, _, _| {})?;
        app.background_jobs = true;
        app.settings.no_last_area = true;
        Some((area_gpu_harness(app, rs, AREA_HARNESS_WIDTH, 800.0), area))
    }

    /// Frames until `area`'s view has drawn a picture with every texture
    /// read (30 seconds at most).
    fn drawn(h: &mut Harness<'_, Moonglow>, area: ResRef) {
        let started = Instant::now();
        loop {
            h.step();
            let app = h.state();
            let pictures = app.area_views.get(&area).map_or(0, |v| v.pictures);
            let pending = app.viewport.as_ref().map_or(0, |v| v.renderer.drawn.textures_pending);
            if pictures > 0 && pending == 0 && app.under_way().is_none() {
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(30), "never drawn");
            std::thread::sleep(Duration::from_millis(5));
        }
        h.run_steps(3);
    }

    /// The area view's part of the window's picture.
    fn view_picture(h: &mut Harness<'_, Moonglow>, area: ResRef) -> image::RgbaImage {
        h.run_steps(1);
        let rect = h.state().area_views[&area].rect;
        let img = h.render().expect("render");
        let ppp = h.ctx.pixels_per_point();
        let (x, y) = ((rect.min.x * ppp) as u32, (rect.min.y * ppp) as u32);
        let (w, hh) = ((rect.width() * ppp) as u32, (rect.height() * ppp) as u32);
        image::imageops::crop_imm(&img, x, y, w.min(img.width() - x), hh.min(img.height() - y))
            .to_image()
    }

    /// The picture's mean red and blue, 0 to 255.
    fn red_and_blue(img: &image::RgbaImage) -> (f64, f64) {
        let n = f64::from(img.width() * img.height()).max(1.0);
        let (r, b) = img
            .pixels()
            .fold((0.0, 0.0), |(r, b), p| (r + f64::from(p.0[0]), b + f64::from(p.0[2])));
        (r / n, b / n)
    }

    /// Save, timed: how long its frame took, and that nothing waits.
    fn timed_save(h: &mut Harness<'_, Moonglow>) -> Duration {
        let started = Instant::now();
        h.state_mut().actions.push(mg_ui::Action::Save);
        h.step();
        let took = started.elapsed();
        let entries = &h.state().log.entries;
        assert!(
            !entries.iter().any(|(_, m)| m.starts_with("The game data is still in use")),
            "{entries:#?}"
        );
        assert!(!h.state().game_changes_waiting());
        took
    }

    /// Issue #49: Save while the area's textures are still being read
    /// apart from the window. The readers let go at once: Save takes well
    /// under the ten seconds the window waits at most, and once the window
    /// lets go of the game data it alone holds it.
    #[test]
    fn save_does_not_wait_for_an_area_s_textures_being_read() {
        let Some((mut h, area)) = drawing_area("holders-save-textures", None) else { return };
        let started = Instant::now();
        let reading = |app: &Moonglow| {
            app.area_views.get(&area).is_some_and(|v| v.pictures > 0)
                && app.viewport.as_ref().unwrap().renderer.textures_read() > 0
        };
        while !reading(h.state()) {
            assert!(started.elapsed() < Duration::from_secs(60), "never drawn");
            std::thread::sleep(Duration::from_millis(5));
            h.step();
        }
        let pending = h.state().viewport.as_ref().unwrap().renderer.drawn.textures_pending;
        eprintln!("textures still to come as Save ran: {pending}");
        let took = timed_save(&mut h);
        eprintln!("Save took {took:.0?}");
        assert!(took < Duration::from_secs(3), "Save took {took:.0?}");
        let (others, waited) = others_after_release(&mut h);
        assert_eq!(others, 0, "the game data held after Save");
        assert!(waited < Duration::from_secs(3), "let go in {waited:.0?}");
    }

    /// Issue #49: Save while the area's first reading is still on its
    /// thread (which holds the game data until it is done).
    #[test]
    fn save_does_not_wait_for_an_area_s_first_reading() {
        let Some((mut h, area)) = drawing_area("holders-save-reading", None) else { return };
        // (The tab opened, then the reading started.)
        h.run_steps(2);
        let reading = h.state().under_way() == Some("an area being read");
        eprintln!("the area still read as Save ran: {reading}");
        let took = timed_save(&mut h);
        eprintln!("Save took {took:.0?}");
        assert!(took < Duration::from_secs(3), "Save took {took:.0?}");
        drawn(&mut h, area);
        let (others, waited) = others_after_release(&mut h);
        assert_eq!(others, 0, "the game data held after Save");
        assert!(waited < Duration::from_secs(3), "let go in {waited:.0?}");
    }

    /// The Appearance Gallery making its pictures (their models read apart
    /// on threads that hold the game data): the window lets go of it at
    /// once, and it alone holds it then.
    #[test]
    fn a_gallery_making_pictures_lets_go_of_the_game_data() {
        let Some((mut h, area)) = drawing_area("holders-gallery", None) else { return };
        drawn(&mut h, area);
        h.state_mut().actions.push(mg_ui::Action::PlaceableGallery);
        h.run_steps(4);
        eprintln!("under way as it was let go of: {:?}", h.state().under_way());
        let started = Instant::now();
        h.state_mut().reload_resources(false);
        let waited = started.elapsed();
        let others = Arc::strong_count(h.state().game.as_ref().unwrap()) - 1;
        assert_eq!(others, 0, "the game data held by the gallery");
        assert!(waited < Duration::from_secs(3), "let go in {waited:.0?}");
        // (And its pictures are made after all.)
        settle(&mut h);
    }

    /// The haks `mg_all_red` and `mg_all_blue` in `user`'s `hak` folder:
    /// every texture of the rural tileset (`ttr01*`, each DDS or TGA the
    /// game has) red in one, blue in the other.
    fn red_and_blue_haks(root: &std::path::Path, user: &std::path::Path) {
        let game = game_data(root);
        let mut names: Vec<ResRef> = [ResType::DDS, ResType::TGA]
            .into_iter()
            .flat_map(|t| game.resman.list(t))
            .filter(|r| r.to_string().to_lowercase().starts_with("ttr01"))
            .collect();
        names.sort();
        names.dedup();
        assert!(names.len() > 5, "the rural tileset's textures: {names:?}");
        for (name, rgba) in [("mg_all_red", RED), ("mg_all_blue", BLUE)] {
            let mut w = mg_erf::ErfWriter::new(*b"HAK ");
            for t in &names {
                w.add(*t, ResType::TGA, swatch(rgba)).unwrap();
            }
            std::fs::create_dir_all(user.join("hak")).unwrap();
            std::fs::write(user.join(format!("hak/{name}.hak")), w.to_bytes().unwrap()).unwrap();
        }
    }

    /// Two copies of the area's module, beside it: A listing the red hak,
    /// B the blue one.
    fn red_and_blue_modules(
        area_module: &std::path::Path,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = area_module.parent().unwrap();
        let (a, b) = (dir.join("red.mod"), dir.join("blue.mod"));
        std::fs::copy(area_module, &a).unwrap();
        std::fs::copy(area_module, &b).unwrap();
        list_haks(&a, &["mg_all_red"]);
        list_haks(&b, &["mg_all_blue"]);
        (a, b)
    }

    /// How much redder than blue a picture is (its mean red less its mean
    /// blue): what is drawn with the rural tileset's textures counts, the
    /// rest (the sky, an object) the same in both.
    fn redder(img: &image::RgbaImage) -> f64 {
        let (r, b) = red_and_blue(img);
        r - b
    }

    /// `module` opened in place of the one open, its area's tab opened and
    /// drawn, and its picture saved as `name`.png in the test's folder.
    fn opened_and_pictured(
        h: &mut Harness<'_, Moonglow>,
        module: &std::path::Path,
        area: ResRef,
        name: &str,
    ) -> image::RgbaImage {
        h.state_mut().close();
        h.state_mut().open_module(module);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        drawn(h, area);
        let picture = view_picture(h, area);
        let dir = mg_testkit::test_output().join("holders-haks-follow");
        std::fs::create_dir_all(&dir).unwrap();
        picture.save(dir.join(format!("{name}.png"))).unwrap();
        picture
    }

    /// Issue #50: module A, its hak's ground red, closed and module B, its
    /// hak's ground blue under the same names, opened: B's area is drawn
    /// blue, A's textures forgotten. (Pictures in
    /// `holders-haks-follow/`: red.png, blue.png.)
    #[test]
    fn the_next_module_s_hak_textures_are_drawn() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("ui-holders-haks-follow");
        let user = user_folder(&dir, &["hak"]);
        red_and_blue_haks(&root, &user);
        let Some((mut h, area)) = drawing_area("holders-haks-follow", Some(user)) else {
            return;
        };
        let path = h.state().module_path().expect("the area's module");
        let (a, b) = red_and_blue_modules(&path);
        let red = redder(&opened_and_pictured(&mut h, &a, area, "red"));
        let blue = redder(&opened_and_pictured(&mut h, &b, area, "blue"));
        eprintln!("red less blue: A {red:.0}, B {blue:.0}");
        assert!(red > 0.0, "A's area red: {red:.0}");
        assert!(red - blue > 40.0, "B's area blue, not A's red: A {red:.0}, B {blue:.0}");
    }

    /// Issue #50 as it was met: B opened while something still held the
    /// game data (a job here; a copy kept by mistake in 4.2.1). Once it is
    /// free, B's haks are layered, and its area, drawn meanwhile with what
    /// the game data had, is drawn again blue. (Pictures in
    /// `holders-haks-follow/`: held-red.png, held-blue.png.)
    #[test]
    fn the_next_module_s_hak_textures_are_drawn_once_the_game_data_is_free() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("ui-holders-haks-held");
        let user = user_folder(&dir, &["hak"]);
        red_and_blue_haks(&root, &user);
        let Some((mut h, area)) = drawing_area("holders-haks-held", Some(user)) else {
            return;
        };
        let path = h.state().module_path().expect("the area's module");
        let (a, b) = red_and_blue_modules(&path);
        let red = redder(&opened_and_pictured(&mut h, &a, area, "held-red"));
        assert!(red > 0.0, "A's area red: {red:.0}");
        let (go, held) = std::sync::mpsc::channel::<()>();
        h.state_mut().start_job_for_tests("Held", move |_| {
            let _ = held.recv();
        });
        h.state_mut().close();
        h.state_mut().open_module(&b);
        assert!(h.state().game_changes_waiting(), "B's haks wait for the game data");
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        h.run_steps(4);
        go.send(()).unwrap();
        let started = Instant::now();
        while h.state().busy() || h.state().game_changes_waiting() {
            assert!(started.elapsed() < Duration::from_secs(30), "never made");
            std::thread::sleep(Duration::from_millis(5));
            h.step();
        }
        // (A picture drawn after the haks changed: not one from before.)
        let pictures = |app: &Moonglow| app.area_views.get(&area).map_or(0, |v| v.pictures);
        let was = pictures(h.state());
        while pictures(h.state()) == was {
            assert!(started.elapsed() < Duration::from_secs(60), "never drawn again");
            std::thread::sleep(Duration::from_millis(5));
            h.step();
        }
        drawn(&mut h, area);
        let blue = view_picture(&mut h, area);
        let out = mg_testkit::test_output().join("holders-haks-follow");
        std::fs::create_dir_all(&out).unwrap();
        blue.save(out.join("held-blue.png")).unwrap();
        let blue = redder(&blue);
        eprintln!("red less blue: A {red:.0}, B {blue:.0}");
        assert!(red - blue > 40.0, "B's area blue, not A's red: A {red:.0}, B {blue:.0}");
    }
}
