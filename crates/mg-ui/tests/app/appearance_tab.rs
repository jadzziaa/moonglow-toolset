//! The Appearance Gallery tab and the pickers (M4's U26): the tab's
//! Status Bar hints and count, its rest in each state, the pictures it
//! asks for, and Select Portrait's justified rows and keys.

use crate::common::*;

/// What the Status Bar says: each hint as a screen reader says it, then
/// each count.
fn said(h: &Harness<'_, Moonglow>) -> Vec<String> {
    h.state().status_said().to_vec()
}

/// Frames until the Status Bar says each of `what` and none of `not`
/// (ten seconds at most).
fn says(h: &mut Harness<'_, Moonglow>, what: &[&str], not: &[&str]) {
    let started = std::time::Instant::now();
    loop {
        let now = said(h);
        let has = |w: &&str| now.iter().any(|s| s == w);
        if what.iter().all(has) && !not.iter().any(has) {
            return;
        }
        assert!(started.elapsed().as_secs() < 10, "never said {what:?} without {not:?}: {now:?}");
        h.step();
    }
}

/// The interface's size (Options › General › Interface size): `zoom`
/// times the usual, as the app sets it, the window's frames run until it
/// is in effect.
fn text_size(h: &mut Harness<'_, Moonglow>, zoom: f32) {
    h.state_mut().settings.ui_scale = Some((zoom * 100.0).round() as u16);
    h.run_steps(3);
    settle(h);
}

/// The Appearance Gallery opened on its own (Tools › Appearance Gallery).
fn open_gallery(h: &mut Harness<'_, Moonglow>) {
    h.state_mut().actions.push(mg_ui::Action::PlaceableGallery);
    settle(h);
    assert!(h.state().dock.find_tab(&Tab::PlaceableGallery).is_some(), "the gallery's tab");
}

/// The kind shown, as the header's switch would set it.
fn kind(h: &mut Harness<'_, Moonglow>, kind: mg_ui::appearance_gallery::Kind) {
    let g = h.state_mut().placeable_gallery.get_or_insert_with(Default::default);
    (g.kind, g.filter) = (kind, String::new());
    settle(h);
}

/// Each hint is true in the state at hand and absent elsewhere: opened
/// for one creature, a click gives it the appearance; Use the Selection
/// (nothing selected) leaves only looking, no Click hint; placeables on
/// their own are taken to place and dragged into an area. The double
/// click's wizard is the kind's, and the count is the table's.
#[test]
fn the_gallery_s_hints_are_those_of_the_state_at_hand() {
    use mg_ui::appearance_gallery::Kind;
    let Some((mut h, _)) = blueprint_harness("nw_bandit001", "u26_hints", ResType::UTC) else {
        return;
    };
    settle(&mut h);
    h.get_by_label("Gallery…").click();
    settle(&mut h);
    let give = "Click: Give the appearance";
    let wizard = "Double Click: Creature Wizard with it";
    let (take, drag) = ("Click: Take to place", "Drag: Place in an area");
    says(&mut h, &[give, wizard], &[take, drag]);
    let count = said(&h).into_iter().find(|s| s.ends_with(" appearances"));
    let count = count.expect("the count at the Status Bar's right");
    let n: usize = count.split(' ').next().unwrap().parse().unwrap();
    assert!(n > 500, "appearance.2da's rows: {count}");
    // Given to the selection, and nothing selected: looking only.
    h.get_by_label("Use the Selection").click();
    settle(&mut h);
    says(&mut h, &[wizard], &[give, take, drag]);
    // Placeables on their own.
    kind(&mut h, Kind::Placeable);
    says(&mut h, &[take, drag, "Double Click: Placeable Wizard with it"], &[give, wizard]);
    // Doors: no drag, the Door Wizard.
    kind(&mut h, Kind::Door);
    says(&mut h, &["Double Click: Door Wizard with it"], &[give, take, drag]);
    // Another tab in front: none of them.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    says(&mut h, &[], &[give, take, drag, "Double Click: Door Wizard with it"]);
    assert!(!said(&h).iter().any(|s| s.ends_with(" appearances")), "{:?}", said(&h));
}

/// The tab comes to rest for each kind, filtered, scrolled, a picture
/// clicked (its Preview drawn, paused), docked and floating, with the
/// pointer resting on a picture (its tooltip shown: egui's tooltip asks
/// for a frame after its delay, which is no frame of the gallery's).
#[test]
fn the_appearance_gallery_comes_to_rest() {
    use mg_ui::appearance_gallery::Kind;
    let Some((mut h, _)) = area_harness("u26-gallery-rests") else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    open_gallery(&mut h);
    for k in [Kind::Placeable, Kind::Creature, Kind::Door] {
        kind(&mut h, k);
        assert_eq!(h.state().thumbnails.waiting(), 0, "{k:?}: the pictures in sight made");
    }
    kind(&mut h, Kind::Creature);
    h.state_mut().placeable_gallery.as_mut().unwrap().filter = "dwarf".into();
    settle(&mut h);
    let cell = |h: &Harness<'_, Moonglow>| {
        // (Wholly in the window and under the gallery's header and hint
        // strip: one scrolled partly under them is not where a click goes.)
        let screen = h.ctx.content_rect().with_min_y(140.0);
        let tile =
            |n: &egui_kittest::Node<'_>| n.rect().height() > 60.0 && screen.contains_rect(n.rect());
        h.query_all_by_label_contains("Dwarf").find(tile).map(|n| n.rect())
    };
    let at = cell(&h).expect("a dwarf's picture");
    // Scrolled.
    h.hover_at(at.center());
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -120.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    // A picture clicked: the Preview draws its model, paused.
    let at = cell(&h).expect("a dwarf's picture");
    h.get_all_by_label_contains("Dwarf").find(|n| n.rect() == at).unwrap().click();
    settle(&mut h);
    assert!(h.query_all_by_label("Preview").next().is_some(), "the Preview panel");
    // The pointer resting on a picture: its tooltip shown, then at rest.
    // (Where it is now: the panels shown beside, the cells moved.)
    let at = cell(&h).expect("a dwarf's picture");
    assert!(tip_at(&mut h, at).is_some(), "the picture's tooltip");
    settle(&mut h);
    // Floating.
    own_window(&mut h, &Tab::PlaceableGallery, egui::pos2(40.0, 40.0), egui::vec2(1200.0, 760.0));
    settle(&mut h);
    kind(&mut h, Kind::Placeable);
    settle(&mut h);
}

/// Opened for one creature, the gallery pictures it in each appearance;
/// the pictures let go of (Largest textures changed, as the Shaders switch
/// and the game data read again do), they are made again of it, not left
/// with nothing to draw (the review's C-1).
#[test]
fn a_creature_s_pictures_are_made_again_once_let_go_of() {
    use mg_ui::appearance_gallery::Kind;
    let Some((mut h, _)) = area_harness("c1-dressed-again") else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    let key = ResKey::parse("c1_bandit", ResType::UTC).unwrap();
    let game = h.state().game.clone().unwrap();
    let data = game.resman.get_named("nw_bandit001", ResType::UTC).unwrap().into_owned();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
    )));
    settle(&mut h);
    open_gallery(&mut h);
    {
        let g = h.state_mut().placeable_gallery.get_or_insert_with(Default::default);
        (g.kind, g.filter) = (Kind::Creature, String::new());
        g.target = Some((key, mg_edit::GffPath::root()));
    }
    settle(&mut h);
    // (With MG_SHOT set: the gallery before and after, to look at.)
    let dir = shot_dir("c1-dressed-again");
    let shot = |h: &mut Harness<'_, Moonglow>, name: &str| {
        if let Some(dir) = &dir {
            h.render().expect("render").save(dir.join(name)).unwrap();
        }
    };
    let made = h.state().thumbnails.creature_pictures();
    assert!(made.0 > 0 && made.1 == 0, "the creature's pictures made: {made:?}");
    shot(&mut h, "before.png");
    h.state_mut().settings.texture_limit = Some(256);
    settle(&mut h);
    shot(&mut h, "after.png");
    let again = h.state().thumbnails.creature_pictures();
    assert!(again.0 > 0 && again.1 == 0, "made again, with something to draw: {again:?}");
}

/// Rows of an authored placeables.2da, `MANY` long, in a user folder's
/// override: names "ThingNNNN", models no file has.
const MANY: usize = 5000;

fn user_with_many_placeables(name: &str) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let user = user_folder(&dir, &["override"]);
    let mut table = String::from("2DA V2.0\n\n Label StrRef ModelName\n");
    for i in 0..MANY {
        table.push_str(&format!("{i} Thing{i:04} **** mg_none{i:04}\n"));
    }
    std::fs::write(user.join("override/placeables.2da"), table).unwrap();
    user
}

/// The gallery's cells drawn, by name and place.
fn things(h: &Harness<'_, Moonglow>) -> Vec<egui::Rect> {
    h.query_all_by(|n| {
        n.role() == egui::accesskit::Role::Button
            && n.label().is_some_and(|l| l.starts_with("Thing"))
    })
    .map(|n| n.rect())
    .collect()
}

/// Of 5,000 placeables, the gallery asks for the pictures of the cells in
/// sight and a row ahead, no more, at the top and scrolled far.
#[test]
fn of_five_thousand_appearances_the_pictures_in_sight_are_asked_for() {
    let user = user_with_many_placeables("u26-many");
    let Some((mut h, _)) =
        blueprint_harness_with("plc_chest1", "u26_many", ResType::UTP, Some(user))
    else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 900.0));
    open_gallery(&mut h);
    let check = |h: &mut Harness<'_, Moonglow>, at: &str| {
        h.step();
        let drawn = things(h);
        let mut tops: Vec<i32> = drawn.iter().map(|r| r.top() as i32).collect();
        tops.dedup();
        let across = drawn.len() / tops.len().max(1);
        assert!(!drawn.is_empty() && drawn.len() < 300, "{at}: {} drawn", drawn.len());
        let asked = h.state().thumbnails.shown_now();
        assert!(
            asked <= drawn.len() + 2 * across && asked >= drawn.len(),
            "{at}: {asked} pictures asked for, {} drawn, {across} a row",
            drawn.len()
        );
    };
    check(&mut h, "at the top");
    let at = things(&h)[0].center();
    for _ in 0..20 {
        h.hover_at(at);
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -4000.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    settle(&mut h);
    check(&mut h, "scrolled far");
}

/// The rects of the portraits drawn in Select Portrait, by row.
fn portrait_rows(h: &Harness<'_, Moonglow>) -> Vec<Vec<egui::Rect>> {
    let grid = h.get_by_label("Portraits").rect();
    let mut cells: Vec<egui::Rect> = h
        .query_all_by(|n| n.label().is_some_and(|l| l.starts_with("po_")))
        .map(|n| n.rect())
        .filter(|r| grid.contains(r.center()) && r.width() < 80.0)
        .collect();
    cells.sort_by(|a, b| (a.top(), a.left()).partial_cmp(&(b.top(), b.left())).unwrap());
    let mut rows: Vec<Vec<egui::Rect>> = Vec::new();
    for c in cells {
        match rows.last_mut() {
            Some(row) if (row[0].top() - c.top()).abs() < 1.0 => row.push(c),
            _ => rows.push(vec![c]),
        }
    }
    rows
}

/// Select Portrait's rows are justified at every width it takes, from
/// its least (480) to its greatest (900), at each text size (zoom 1,
/// 1.25 and 1.5): each full row's portraits keep their 54 by 84 exactly,
/// the room left over goes between them, and the last of a row reaches
/// the grid's right edge, which is the scroll bar's (no gap left before
/// it).
#[test]
fn justified_rows_leave_no_gap_before_the_scroll_bar() {
    let Some((mut h, _)) = blueprint_harness("nw_bandit001", "u26_justified", ResType::UTC) else {
        return;
    };
    settle(&mut h);
    h.get_by_label("Portraits…").click();
    settle(&mut h);
    let mut widths = Vec::new();
    for zoom in [1.0_f32, 1.25, 1.5] {
        text_size(&mut h, zoom);
        // (The dialog is 80% of the window's width, 480 to 900 points.)
        for points in [560.0_f32, 700.0, 820.0, 960.0, 1125.0, 1400.0] {
            h.set_size(egui::vec2(points * zoom, 800.0 * zoom));
            settle(&mut h);
            settle(&mut h);
            let what = format!("{points} points wide at {zoom}");
            let grid = h.get_by_label("Portraits").rect();
            widths.push(grid.width().round() as i32);
            let rows = portrait_rows(&h);
            let full: Vec<&Vec<egui::Rect>> = rows.iter().filter(|r| r.len() > 3).collect();
            assert!(full.len() >= 2, "{what}: rows drawn: {rows:?}");
            let across = full.iter().map(|r| r.len()).max().unwrap();
            for row in full.iter().filter(|r| r.len() == across) {
                for c in row.iter() {
                    let (w, t) = (c.width(), c.height());
                    assert!((w - 54.0).abs() < 0.05 && (t - 84.0).abs() < 0.05, "{what}: {c:?}");
                }
                let gaps: Vec<f32> = row.windows(2).map(|w| w[1].left() - w[0].right()).collect();
                let (least, most) =
                    gaps.iter().fold((f32::MAX, 0.0f32), |(l, m), g| (l.min(*g), m.max(*g)));
                assert!(least >= 8.0 - 0.5 && most - least < 1.5, "{what}: gaps {gaps:?}");
                let last = row.last().unwrap();
                let gap = grid.right() - last.right();
                assert!(
                    gap.abs() < 1.0,
                    "{what}: {gap} before the scroll bar: {last:?} in {grid:?}"
                );
                assert!((row[0].left() - grid.left()).abs() < 1.0, "{what}: from the grid's left");
            }
        }
    }
    widths.sort_unstable();
    widths.dedup();
    assert!(widths.len() >= 5, "the dialog's widths swept: {widths:?}");
}

/// Select Portrait by the keys: it opens at the blueprint's portrait with
/// the grid's keys; Home goes to the first and chooses it, Enter is OK;
/// opened again, Esc is Cancel (the one chosen dropped).
#[test]
fn select_portrait_s_enter_and_esc_are_its_ok_and_cancel() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_keys", ResType::UTC) else {
        return;
    };
    settle(&mut h);
    let portrait = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&key).unwrap().root.integer("PortraitId")
    };
    let before = portrait(&mut h);
    assert_ne!(before, Some(1), "not the first already");
    h.get_by_label("Portraits…").click();
    settle(&mut h);
    // (At its own: the chosen cell drawn.)
    let ring = |h: &Harness<'_, Moonglow>| {
        h.query_all_by(|n| {
            n.label().is_some_and(|l| l.starts_with("po_")) && n.is_selected() == Some(true)
        })
        .count()
    };
    assert_eq!(ring(&h), 1, "opened at the blueprint's portrait");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Select Portrait").is_none(), "Esc closes it");
    assert_eq!(portrait(&mut h), before, "Esc chose nothing");
    h.get_by_label("Portraits…").click();
    settle(&mut h);
    h.key_press(egui::Key::Home);
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.query_by_label("Select Portrait").is_none(), "Enter closes it");
    assert_eq!(portrait(&mut h), Some(1), "Enter took the first");
    // The footer: the count at its left.
    h.get_by_label("Portraits…").click();
    settle(&mut h);
    let note = h.get_by_label_contains(" portraits").rect();
    let ok = h.get_all_by_label("OK").last().unwrap().rect();
    assert!((note.center().y - ok.center().y).abs() < 4.0, "on the footer's row: {note:?} {ok:?}");
    assert!(note.right() < ok.left(), "at its left");
}

/// The pickers come to rest: Select Portrait, Select Texture (each opened
/// and a picture chosen by the keys) and the Creature Wizard's Appearance
/// and Portrait, whose wells are as wide as its page (a well wider than
/// its room grew the wizard's window two points a frame).
#[test]
fn the_pickers_come_to_rest() {
    let Some((mut h, _)) = blueprint_harness("nw_bandit001", "u26_rest_face", ResType::UTC) else {
        return;
    };
    settle(&mut h);
    h.get_by_label("Portraits…").click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "u26_rest_texture", ResType::UTP)
    else {
        return;
    };
    h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), "Visuals");
    settle(&mut h);
    h.get_all_by_label("Select Texture…").next().expect("Select Texture…").click();
    settle(&mut h);
    assert!(h.query_by_label("Select Texture").is_some());
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    // The Creature Wizard's fourth step.
    let Some(mut h) = game_harness("u26-rest-wizard") else { return };
    h.set_size(egui::vec2(1600.0, 720.0));
    h.state_mut().creature_wizard = Some(Default::default());
    settle(&mut h);
    for at in 0..3 {
        if at == 1 {
            h.get_by_label("Human").click();
            settle(&mut h);
        }
        h.get_by_label("Next >").click();
        settle(&mut h);
    }
    let well = h.get_by_label("Appearances").rect().width();
    for _ in 0..3 {
        settle(&mut h);
    }
    assert_eq!(h.get_by_label("Appearances").rect().width(), well, "the wizard keeps its width");
}

// The U26 review's tests (2026-11-19).

/// A placed chest (Placeable List 0) and bandit (Creature List 0) in the
/// area, for [`area_harness_with`].
fn chest_and_bandit(m: &mut Module, game: &mg_rules::GameData, area: ResRef) {
    use mg_module::instances::{Placement, Placing, instance};
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    for (bp, t, list, x) in [
        ("plc_chest1", ResType::UTP, "Placeable List", 12.0),
        ("nw_bandit001", ResType::UTC, "Creature List", 16.0),
    ] {
        let key = ResKey::parse(bp, t).unwrap();
        let root = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let at = Placement { position: [x, 20.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, t, &root, at, &[]).unwrap();
        git.root.set(list, mg_gff::Value::List(vec![item]));
    }
    m.set_gff(git_key, &git).unwrap();
}

/// The kit grid named `name` (a list to a screen reader): its rect and
/// how many cells it has.
fn grid_of(h: &Harness<'_, Moonglow>, name: &str) -> (egui::Rect, usize) {
    let g = h
        .query_all_by(|n| {
            n.role() == egui::accesskit::Role::List && n.label().as_deref() == Some(name)
        })
        .next()
        .unwrap_or_else(|| panic!("no grid {name}"));
    (g.rect(), g.accesskit_node().data().size_of_set().unwrap_or(0))
}

/// The cells of grid `name` built, by place (from 1): their rects.
fn cells_of(h: &Harness<'_, Moonglow>, name: &str) -> Vec<(usize, egui::Rect)> {
    let (_, count) = grid_of(h, name);
    let mut cells: Vec<(usize, egui::Rect)> = h
        .query_all_by(|n| {
            n.role() != egui::accesskit::Role::List
                && n.data().size_of_set() == Some(count)
                && n.data().position_in_set().is_some()
        })
        .map(|n| (n.accesskit_node().data().position_in_set().unwrap(), n.rect()))
        .collect();
    cells.sort_by_key(|c| c.0);
    cells.dedup_by_key(|c| c.0);
    cells
}

/// The cell at `place` in grid `name`, clicked (`secondary`: right).
fn click_at(h: &mut Harness<'_, Moonglow>, name: &str, place: usize, secondary: bool) {
    let (_, count) = grid_of(h, name);
    let cell = h.get_by(|n| {
        n.role() != egui::accesskit::Role::List
            && n.data().size_of_set() == Some(count)
            && n.data().position_in_set() == Some(place)
    });
    if secondary {
        cell.click_secondary()
    } else {
        cell.click()
    }
    settle(h);
}

/// The `nth` (from 0) of grid `name`'s cells wholly in the window,
/// clicked (`secondary`: right): the galleries open at the one chosen.
fn click_nth(h: &mut Harness<'_, Moonglow>, name: &str, nth: usize, secondary: bool) {
    let screen = h.ctx.content_rect();
    let seen: Vec<usize> =
        cells_of(h, name).into_iter().filter(|c| screen.contains_rect(c.1)).map(|c| c.0).collect();
    let place = *seen.get(nth).unwrap_or_else(|| panic!("{name}: {seen:?} in sight"));
    click_at(h, name, place, secondary);
}

/// The pointer over grid `name`, pressed in the gap over its first row
/// in sight (no cell's): the Status Bar names the hints of the editor
/// under the pointer or pressed last.
fn over(h: &mut Harness<'_, Moonglow>, name: &str) {
    let screen = h.ctx.content_rect();
    let first = (cells_of(h, name).into_iter().map(|c| c.1))
        .find(|r| screen.contains_rect(*r))
        .expect("cells in sight");
    let at = egui::pos2(first.left() + 20.0, first.top() - 3.0);
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    settle(h);
}

/// The hint strip's words for each mode.
const STRIPS: [&str; 5] = [
    ".utc the appearance",
    "A click takes a placeable to place. Right click gives it to the 1 placeable selected.",
    "1 creature selected in the area. A click gives it the appearance.",
    "A click takes a placeable to place in an area. A double click makes a blueprint of it.",
    "Select creatures in an area, then click a picture to give them its appearance.",
];

/// The strip says `STRIPS[at]` and none of the others.
fn strip_says(h: &Harness<'_, Moonglow>, at: usize) {
    for (i, words) in STRIPS.iter().enumerate() {
        let shown = h.query_by_label_contains(words).is_some();
        assert_eq!(shown, i == at, "the strip with {:?}: {words:?}", STRIPS[at]);
    }
}

/// Every mode reached: opened for a creature (a click gives it, one Undo
/// step), placeables on their own (a click takes one to place, nothing
/// changed), with a placeable selected (the click still takes it; Right
/// Click Menu, whose Give to the Selection is one Undo step), creatures
/// with a creature selected (a click gives it, one step), and with none
/// (looking only: no Click hint, a click changes nothing). Each strip and
/// each hint true in its mode and absent in the others.
#[test]
fn each_mode_s_hints_are_its_own_and_a_change_is_one_undo_step() {
    use mg_area::ObjectKind;
    use mg_ui::appearance_gallery::Kind;
    let (give, take, menu) =
        ("Click: Give the appearance", "Click: Take to place", "Right Click: Menu");
    let depth = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().undo_depth();
    // Opened for one creature.
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_mode_target", ResType::UTC)
    else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    strip_says(&h, 0);
    over(&mut h, "Creatures");
    says(&mut h, &[give], &[take, menu]);
    let before = depth(&h);
    click_nth(&mut h, "Creatures", 1, false);
    assert_eq!(depth(&h), before + 1, "a click given: one Undo step");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Appearance"));
    // In an area: placeables on their own.
    let Some((mut h, area)) =
        area_harness_with("u26-modes", "ttr01", None, 4, None, &chest_and_bandit)
    else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 900.0));
    open_gallery(&mut h);
    let select = |h: &mut Harness<'_, Moonglow>, s: Vec<(ObjectKind, usize)>| {
        h.state_mut().area_views.get_mut(&area).unwrap().selection = s;
        settle(h);
    };
    kind(&mut h, Kind::Placeable);
    strip_says(&h, 3);
    over(&mut h, "Placeables");
    says(&mut h, &[take], &[give, menu]);
    let before = depth(&h);
    click_nth(&mut h, "Placeables", 1, false);
    assert!(h.state().palette.placing_appearance.is_some(), "taken to place");
    assert_eq!(depth(&h), before, "taken, nothing changed");
    h.state_mut().palette.placing_appearance = None;
    // A placeable selected.
    select(&mut h, vec![(ObjectKind::Placeable, 0)]);
    strip_says(&h, 1);
    over(&mut h, "Placeables");
    says(&mut h, &[take, menu], &[give]);
    click_nth(&mut h, "Placeables", 2, false);
    assert_eq!(depth(&h), before, "a click takes it: nothing changed");
    h.state_mut().palette.placing_appearance = None;
    click_nth(&mut h, "Placeables", 2, true);
    h.get_by_label("Give to the Selection").click();
    settle(&mut h);
    assert_eq!(depth(&h), before + 1, "given from the menu: one Undo step");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Appearance"));
    // Creatures, one selected.
    kind(&mut h, Kind::Creature);
    select(&mut h, vec![(ObjectKind::Creature, 0)]);
    strip_says(&h, 2);
    over(&mut h, "Creatures");
    says(&mut h, &[give], &[take, menu]);
    let before = depth(&h);
    click_nth(&mut h, "Creatures", 1, false);
    assert_eq!(depth(&h), before + 1, "given to the selection: one Undo step");
    // None selected: looking only.
    select(&mut h, Vec::new());
    strip_says(&h, 4);
    over(&mut h, "Creatures");
    says(&mut h, &[], &[give, take, menu]);
    let before = depth(&h);
    click_nth(&mut h, "Creatures", 2, false);
    assert_eq!(depth(&h), before, "looked at, nothing changed");
}

/// The Status Bar's count is the header's (without the table's name):
/// "838 appearances", and under Find "12 of 838 appearances".
#[test]
fn the_status_bar_s_count_is_the_header_s() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_count", ResType::UTC) else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    over(&mut h, "Creatures");
    let both = |h: &Harness<'_, Moonglow>| {
        let bar = said(h).into_iter().find(|s| s.ends_with(" appearances")).expect("its count");
        let header = h
            .query_all_by(|n| {
                n.value().is_some_and(|l| l.ends_with(" appearances (appearance.2da)"))
            })
            .next()
            .expect("the header's count")
            .accesskit_node()
            .value()
            .unwrap();
        (bar, header)
    };
    let (bar, header) = both(&h);
    assert_eq!(format!("{bar} (appearance.2da)"), header);
    assert!(!bar.contains(" of "), "{bar}");
    h.state_mut().placeable_gallery.as_mut().unwrap().filter = "dwarf".into();
    settle(&mut h);
    let (bar, header) = both(&h);
    assert_eq!(format!("{bar} (appearance.2da)"), header);
    assert!(bar.contains(" of "), "under Find: {bar}");
}

/// The window 1600 by 900 with editors opened as tabs (the boards'), and
/// the blueprint's Gallery… clicked: the gallery a tab of the main
/// window, its Preview and Appearance panels in the Right Dock; the
/// blueprint's window put narrow over the Left Dock, clear of the
/// gallery.
fn gallery_in_tabs(h: &mut Harness<'_, Moonglow>, key: ResKey) {
    h.set_size(egui::vec2(1600.0, 900.0));
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    settle(h);
    h.get_by_label("Gallery…").click();
    settle(h);
    own_window(h, &Tab::Blueprint(key), egui::pos2(0.0, 90.0), egui::vec2(262.0, 700.0));
    settle(h);
}

/// The Appearance panel's value beside `label`.
fn fact(h: &Harness<'_, Moonglow>, label: &str) -> Option<String> {
    let at = h.query_all_by_label(label).map(|n| n.rect()).find(|r| r.height() < 40.0)?;
    h.query_all_by(|n| n.role() == egui::accesskit::Role::Label && n.value().is_some())
        .find(|n| {
            let r = n.rect();
            (r.center().y - at.center().y).abs() < 4.0 && r.left() > at.right()
        })
        .and_then(|n| n.accesskit_node().value())
}

/// The appearance the Preview shows (its source's row).
fn preview_row(h: &Harness<'_, Moonglow>) -> Option<u32> {
    h.state().model_views.keys().find_map(|s| match s {
        mg_ui::model_view::Source::Appearance { row, .. } => Some(*row),
        _ => None,
    })
}

/// F6 reaches the gallery's Appearance panel in the Right Dock, one stop
/// of the panes' (U26: F6 there takes the keys from the panes; the
/// review's C-8: it was left out of the stops).
#[test]
fn f6_reaches_the_gallery_s_appearance_panel() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "c8_f6", ResType::UTC) else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    let mut stops = Vec::new();
    for _ in 0..12 {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        stops.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
    }
    assert!(stops.iter().any(|s| s == "AppearanceFacts"), "F6's stops: {stops:?}");
}

/// An arrow key moves the gallery's keys: the Preview shows the picture
/// moved to, and the Appearance panel its row. The panel reads a row's
/// facts once for it, not for each frame drawn.
#[test]
fn an_arrow_key_moves_the_preview_and_the_appearance_panel() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_arrows", ResType::UTC) else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    click_nth(&mut h, "Creatures", 1, false);
    let (row, shown) = (fact(&h, "Row").expect("the panel's row"), preview_row(&h));
    assert_eq!(shown.map(|r| r.to_string()).as_deref(), Some(row.as_str()), "the Preview's");
    let read = mg_ui::appearance_gallery::facts_read(&h.ctx);
    for i in 0..10 {
        h.hover_at(egui::pos2(300.0 + i as f32 * 7.0, 400.0));
        h.step();
    }
    assert_eq!(mg_ui::appearance_gallery::facts_read(&h.ctx), read, "read again unasked");
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    let now = fact(&h, "Row").expect("the panel's row");
    assert_ne!(now, row, "the panel follows the keys");
    assert_eq!(preview_row(&h).map(|r| r.to_string()), Some(now), "the Preview follows them");
    assert_eq!(mg_ui::appearance_gallery::facts_read(&h.ctx), read + 1, "read once for it");
}

/// The Appearance panel's rows are 28 apart (the board's), and a
/// creature's last, Blood color, is in the panel at the board's size.
#[test]
fn the_appearance_panel_s_rows_are_the_board_s_28_apart() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_pitch", ResType::UTC) else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    let labels = [
        "Row",
        "Name",
        "Label",
        "Model",
        "Model type",
        "Size",
        "Movement rate",
        "Personal space",
        "Height",
        "Blood color",
    ];
    let dock = right_dock_rect(&h);
    let tops: Vec<f32> = labels
        .iter()
        .map(|l| {
            let r = h.query_all_by_label(l).map(|n| n.rect()).find(|r| dock.contains(r.center()));
            r.unwrap_or_else(|| panic!("no {l} in the panel")).center().y
        })
        .collect();
    for (w, l) in tops.windows(2).zip(labels.windows(2)) {
        assert!((w[1] - w[0] - 28.0).abs() < 0.6, "{l:?}: {} apart", w[1] - w[0]);
    }
    let blood =
        h.query_all_by_label("Blood color").map(|n| n.rect()).find(|r| dock.contains(r.center()));
    assert!(dock.contains_rect(blood.unwrap()), "Blood color in the panel: {blood:?} in {dock:?}");
}

/// Whether grid `name` has the keys (its ring the accent): the grid, or
/// its keys' cell (its active one, which a screen reader is told has
/// the focus), is focused.
fn has_keys(h: &Harness<'_, Moonglow>, name: &str) -> bool {
    let grid = h.query_all_by(|n| {
        n.role() == egui::accesskit::Role::List && n.label().as_deref() == Some(name)
    });
    let Some(count) = grid.map(|g| g.accesskit_node().data().size_of_set()).next() else {
        return false;
    };
    let count = count.unwrap_or(0);
    h.query_all_by(|n| n.is_focused()).any(|n| {
        let n = n.accesskit_node();
        (n.role() == egui::accesskit::Role::List && n.label().as_deref() == Some(name))
            || (n.role() != egui::accesskit::Role::List && n.data().size_of_set() == Some(count))
    })
}

/// The gallery's grid takes the keys as the tab opens and as it comes to
/// the front again (the ring the accent, as on the board), and never
/// while Find is typed in, another tab is in front, or a field in another
/// window has them. The Creature Wizard's appearances take them as its
/// page opens.
#[test]
fn the_grid_takes_the_keys_as_the_tab_comes_to_the_front() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_focus", ResType::UTC) else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    assert!(has_keys(&h, "Creatures"), "opened: the grid has the keys");
    // Find typed in keeps them.
    let find = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    h.get(egui_kittest::kittest::by().predicate(find)).click();
    settle(&mut h);
    h.get(egui_kittest::kittest::by().predicate(find)).type_text("dw");
    for _ in 0..5 {
        settle(&mut h);
    }
    assert!(!has_keys(&h, "Creatures"), "Find keeps the keys while typed in");
    let typed = h.query_all_by(|n| n.is_focused()).any(|n| n.value().as_deref() == Some("dw"));
    assert!(typed, "Find has them, \"dw\" in it");
    // Another tab in front (the area's, in the gallery's pane), then the
    // gallery's again.
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    h.state_mut().actions.push(mg_ui::Action::OpenTab(area));
    settle(&mut h);
    assert_ne!(h.state().active_editor(), Some(&Tab::PlaceableGallery));
    assert!(!has_keys(&h, "Creatures"), "not the tab in front");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::PlaceableGallery));
    settle(&mut h);
    assert!(has_keys(&h, "Creatures"), "in front again: the grid has the keys");
    // A field of the blueprint's (in its floating window) given the keys:
    // they stay there, the gallery drawn beside it.
    let field = h
        .query_all_by(|n| n.role() == egui::accesskit::Role::TextInput)
        .map(|n| n.rect())
        .find(|r| r.right() < 266.0 && r.top() > 90.0)
        .expect("a field of the blueprint's");
    h.hover_at(field.center());
    h.event(egui::Event::PointerButton {
        pos: field.center(),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: field.center(),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    for _ in 0..5 {
        settle(&mut h);
    }
    assert!(!has_keys(&h, "Creatures"), "the field keeps the keys");
    let focused = h
        .query_all_by(|n| n.role() == egui::accesskit::Role::TextInput)
        .any(|n| n.accesskit_node().is_focused() && n.rect().center() == field.center());
    assert!(focused, "the blueprint's field has them");
    // The Creature Wizard's Appearance page.
    let Some(mut h) = game_harness("u26-focus-wizard") else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    let mut w = mg_ui::creature_wizard::CreatureWizard::default();
    (w.page, w.race, w.appearance) = (3, Some(6), 6);
    w.classes = vec![(4, 1)];
    h.state_mut().creature_wizard = Some(w);
    settle(&mut h);
    assert!(has_keys(&h, "Appearances"), "the page opened: its appearances have the keys");
}

/// Down in Find gives the grid the keys, its cursor on the first picture
/// shown (not where the one chosen is); Down and End go on from there.
#[test]
fn down_in_find_gives_the_grid_the_keys_at_its_first_picture() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_find_down", ResType::UTC)
    else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    let find = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    h.get(egui_kittest::kittest::by().predicate(find)).click();
    h.get(egui_kittest::kittest::by().predicate(find)).type_text("dw");
    for _ in 0..5 {
        settle(&mut h);
    }
    assert!(!has_keys(&h, "Creatures"), "Find has the keys while typed in");
    h.key_press(egui::Key::ArrowDown);
    for _ in 0..5 {
        settle(&mut h);
    }
    assert!(has_keys(&h, "Creatures"), "Down: the grid has the keys");
    let cursor = |h: &Harness<'_, Moonglow>| crate::gallery_keys::grid_cursor(h, "Creatures");
    assert_eq!(cursor(&h).map(|c| c.1), Some(1), "its cursor on the first picture");
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(cursor(&h).map(|c| c.1), Some(2), "Right goes on from it");
}

/// After Down from Find and a move to another picture, typing more in Find
/// (the pictures' number changes) puts the cursor back on the chosen
/// picture, not on the first.
#[test]
fn typing_in_find_after_down_keeps_the_cursor_on_the_chosen_picture() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_find_retype", ResType::UTC)
    else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    for _ in 0..5 {
        settle(&mut h);
    }
    let cursor = |h: &Harness<'_, Moonglow>| crate::gallery_keys::grid_cursor(h, "Creatures");
    let chosen = cursor(&h).expect("opened at the chosen picture").0;
    let find = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    h.get(egui_kittest::kittest::by().predicate(find)).click();
    h.get(egui_kittest::kittest::by().predicate(find)).type_text("human");
    for _ in 0..5 {
        settle(&mut h);
    }
    h.key_press(egui::Key::ArrowDown);
    h.key_press(egui::Key::ArrowRight);
    for _ in 0..5 {
        settle(&mut h);
    }
    assert_eq!(cursor(&h).map(|c| c.1), Some(2), "Down, then Right: the second picture");
    // More typed: the pictures' number changes, the cursor is the chosen's.
    let typed = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("human")
    };
    h.get_by(typed).click();
    h.get_by(typed).type_text(" male");
    for _ in 0..5 {
        settle(&mut h);
    }
    let now = cursor(&h).expect("a cursor");
    assert_eq!(now.0, chosen, "on the chosen picture");
}

/// A user folder whose portraits.2da has `count` creature portraits
/// (human males, "mgpNNNN"), each with a tiny picture in its override,
/// then `extra` rows as given (`BaseResRef Sex Race InanimateType Plot`).
fn user_with_portraits(name: &str, count: usize, extra: &[&str]) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let user = user_folder(&dir, &["override"]);
    let mut table = String::from("2DA V2.0\n\n BaseResRef Sex Race InanimateType Plot LowGore\n");
    let tga = [0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 24, 0, 90, 60, 30];
    for i in 0..count {
        table.push_str(&format!("{i} mgp{i:04} 0 6 **** 0 ****\n"));
        std::fs::write(user.join(format!("override/po_mgp{i:04}m.tga")), tga).unwrap();
    }
    for (j, row) in extra.iter().enumerate() {
        table.push_str(&format!("{} {row} ****\n", count + j));
    }
    std::fs::write(user.join("override/portraits.2da"), table).unwrap();
    user
}

/// Of 5,000 portraits, Select Portrait and the Creature Wizard's All
/// Portraits ask for the pictures of those in sight and a row ahead, at
/// the top and scrolled far.
#[test]
fn of_five_thousand_portraits_those_in_sight_are_asked_for() {
    let user = user_with_portraits("u26-portraits-many", MANY, &[]);
    let check = |h: &mut Harness<'_, Moonglow>, at: &str| {
        h.step();
        let (grid, count) = grid_of(h, "Portraits");
        assert_eq!(count, MANY, "{at}: the portraits listed");
        let drawn: Vec<egui::Rect> = cells_of(h, "Portraits")
            .into_iter()
            .map(|c| c.1)
            .filter(|r| r.intersects(grid))
            .collect();
        let mut tops: Vec<i32> = drawn.iter().map(|r| r.top() as i32).collect();
        tops.dedup();
        let across = drawn.len() / tops.len().max(1);
        let asked = Moonglow::portraits_now(&h.ctx);
        assert!(!drawn.is_empty() && drawn.len() < 400, "{at}: {} drawn", drawn.len());
        assert!(
            asked >= across && asked <= drawn.len() + 2 * across,
            "{at}: {asked} portraits asked for, {} built, {across} a row",
            drawn.len()
        );
    };
    let scroll_far = |h: &mut Harness<'_, Moonglow>| {
        let at = cells_of(h, "Portraits")[0].1.center();
        for _ in 0..20 {
            h.hover_at(at);
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -4000.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
            h.step();
        }
        settle(h);
    };
    let Some((mut h, _)) =
        blueprint_harness_with("nw_bandit001", "u26_po_many", ResType::UTC, Some(user.clone()))
    else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    h.get_by_label("Portraits…").click();
    settle(&mut h);
    check(&mut h, "Select Portrait");
    scroll_far(&mut h);
    check(&mut h, "Select Portrait scrolled far");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // The Creature Wizard's All Portraits.
    let mut w = mg_ui::creature_wizard::CreatureWizard::default();
    (w.page, w.race, w.appearance, w.all_portraits) = (3, Some(6), 6, true);
    w.classes = vec![(4, 1)];
    h.state_mut().creature_wizard = Some(w);
    settle(&mut h);
    check(&mut h, "the wizard");
    scroll_far(&mut h);
    check(&mut h, "the wizard scrolled far");
}

/// Select Portrait's list never changes under the keys: opened at a
/// portrait with no picture, or a plot character's (not among Characters
/// and Creatures), it lists the same portraits as ever, the one opened
/// at shown in the preview alone (named, no cell, none ringed), and the
/// keys moving leave the list as it was.
#[test]
fn select_portrait_keeps_its_list_under_the_keys() {
    let user =
        user_with_portraits("u26-portraits-odd", 12, &["mgqnone 0 6 **** 0", "mgp0001 0 6 **** 1"]);
    let Some((mut h, key)) =
        blueprint_harness_with("nw_bandit001", "u26_po_odd", ResType::UTC, Some(user))
    else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    let ringed = |h: &Harness<'_, Moonglow>| {
        h.query_all_by(|n| {
            n.label().is_some_and(|l| l.starts_with("po_")) && n.is_selected() == Some(true)
        })
        .count()
    };
    for (row, base) in [(12, "mgqnone"), (13, "mgp0001")] {
        let edit = mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "PortraitId".into(),
            value: Some(mg_gff::Value::Word(row)),
        };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("po", vec![edit])));
        settle(&mut h);
        h.get_by_label("Portraits…").click();
        settle(&mut h);
        let what = format!("opened at {base} (row {row})");
        assert_eq!(grid_of(&h, "Portraits").1, 12, "{what}: the portraits listed");
        assert!(h.query_by_label("12 portraits").is_some(), "{what}: the footer's count");
        assert!(h.query_by_label(base).is_some(), "{what}: named in the preview");
        assert_eq!(ringed(&h), 0, "{what}: no cell ringed");
        let before = cells_of(&h, "Portraits").len();
        h.key_press(egui::Key::ArrowRight);
        settle(&mut h);
        assert_eq!(grid_of(&h, "Portraits").1, 12, "{what}: the keys moved, the list kept");
        assert_eq!(cells_of(&h, "Portraits").len(), before, "{what}: the cells kept");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
    }
}

/// Select Texture opened with no replacement: the texture replaced is in
/// the preview with its name, its cell ringed and the keys' cursor on it;
/// OK disabled and Enter doing nothing until another is chosen; then
/// Enter on the grid is OK (the replacement made, one Undo step).
#[test]
fn select_texture_s_enter_is_ok_only_with_one_chosen() {
    let Some((mut h, _)) = blueprint_harness("plc_chest1", "u26_tex_enter", ResType::UTP) else {
        return;
    };
    page(&mut h, "Visuals");
    settle(&mut h);
    h.get_all_by_label("Select Texture…").next().unwrap().click();
    settle(&mut h);
    let to_draw = h
        .query_all_by(|n| n.value().is_some_and(|l| l.starts_with("To draw ")))
        .next()
        .expect("To draw NAME with")
        .accesskit_node()
        .value()
        .unwrap();
    let old = to_draw.trim_start_matches("To draw ").trim_end_matches(" with").to_lowercase();
    // (The preview's: right of the grid, not the Visuals page's list behind.)
    let (grid, _) = grid_of(&h, "Textures");
    let preview = h
        .query_all_by(|n| {
            n.role() == egui::accesskit::Role::Label && n.value().as_deref() == Some(old.as_str())
        })
        .filter(|n| n.rect().left() > grid.right());
    assert!(preview.count() >= 1, "{old} named in the preview");
    texture_gaps(&h);
    // (Its cell ringed, the keys' cursor on it, if it is in the list: a
    // DDS or TGA the game has. The chest's first is drawn with none.)
    let listed = {
        let game = h.state().game.as_deref().unwrap();
        [ResType::DDS, ResType::TGA]
            .into_iter()
            .any(|t| game.resman.get(&ResKey::new(ResRef::from_str(&old).unwrap(), t)).is_ok())
    };
    let ringed = h.query_all_by(|n| {
        n.label().as_deref() == Some(old.as_str()) && n.is_selected() == Some(true)
    });
    assert_eq!(ringed.count(), usize::from(listed), "{old}'s cell ringed (listed: {listed})");
    let cursor = crate::gallery_keys::grid_cursor(&h, "Textures").map(|c| c.0);
    assert_eq!(cursor, listed.then(|| old.clone()), "the keys' cursor");
    let ok_enabled = |h: &Harness<'_, Moonglow>| {
        !h.get_all_by_label("OK").last().unwrap().accesskit_node().is_disabled()
    };
    assert!(!ok_enabled(&h), "OK disabled until another is chosen");
    let depth = h.state().ws.as_ref().unwrap().undo_depth();
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.query_by_label("Select Texture").is_some(), "Enter with OK disabled: still open");
    assert_eq!(h.state().ws.as_ref().unwrap().undo_depth(), depth, "nothing changed");
    // Another chosen (the grid given the keys): Enter is OK.
    let other = cells_of(&h, "Textures").into_iter().map(|c| c.0).find(|&p| {
        h.query_by(|n| {
            n.data().position_in_set() == Some(p) && n.label().as_deref() != Some(old.as_str())
        })
        .is_some()
    });
    click_at(&mut h, "Textures", other.expect("another texture"), false);
    assert!(ok_enabled(&h), "OK enabled with another chosen");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.query_by_label("Select Texture").is_none(), "Enter is OK");
    assert_eq!(h.state().ws.as_ref().unwrap().undo_depth(), depth + 1, "the replacement made");
}

/// Select Texture's cells the board's 10 apart across (where it has two
/// columns) and 14 down, under one line of name each (6 and 15 points
/// under the tile).
fn texture_gaps(h: &Harness<'_, Moonglow>) {
    let cells: Vec<egui::Rect> = cells_of(h, "Textures").into_iter().map(|c| c.1).collect();
    let first = *cells
        .iter()
        .min_by(|a, b| (a.top(), a.left()).partial_cmp(&(b.top(), b.left())).unwrap())
        .expect("its cells");
    let across =
        cells.iter().find(|r| (r.top() - first.top()).abs() < 0.5 && r.left() > first.right());
    if let Some(across) = across {
        let gap = across.left() - first.right();
        assert!((gap - 10.0).abs() < 0.5, "{first:?} then {across:?}: {gap} apart, not 10");
    }
    let down =
        cells.iter().find(|r| (r.left() - first.left()).abs() < 0.5 && r.top() > first.bottom());
    let down = down.expect("a second row");
    let gap = down.top() - first.bottom();
    assert!((gap - 14.0).abs() < 0.5, "{first:?} over {down:?}: {gap} apart, not 14");
    let label = first.height() - first.width();
    assert!((label - 21.0).abs() < 0.5, "{first:?}: {label} under the tile, not one line");
}

/// Select Texture at its least size (a window 560 wide: the dialog's 480)
/// is laid out from one width: the toolbar wraps in it, never cut nor
/// widening the dialog, and the well takes the dialog's width beside the
/// preview (no room left empty).
#[test]
fn select_texture_at_its_least_size_keeps_one_width() {
    let Some((mut h, _)) = blueprint_harness("plc_chest1", "u26_tex_least", ResType::UTP) else {
        return;
    };
    h.set_size(egui::vec2(1000.0, 800.0));
    page(&mut h, "Visuals");
    settle(&mut h);
    // (Its least: the window made small before the dialog opens.)
    h.set_size(egui::vec2(560.0, 700.0));
    settle(&mut h);
    h.get_all_by_label("Select Texture…").next().unwrap().click();
    settle(&mut h);
    let (grid, _) = grid_of(&h, "Textures");
    texture_gaps(&h);
    let cancel = h.get_all_by_label("Cancel").last().unwrap().rect();
    let screen = h.ctx.content_rect();
    // (The well's frame: 10 points of room and its line round the grid,
    // and its scroll bar at the right; the preview 200 wide, 16 from it.)
    let (body, well) = (grid.left() - 11.0, grid.right() + 11.0);
    assert!(cancel.right() <= body + 480.0 + 1.0, "the dialog widened: {cancel:?} from {body}");
    let empty = cancel.right() - (well + 16.0 + 200.0);
    assert!(empty < 12.0, "{empty} points left empty: the well to {well}, {cancel:?}");
    let find = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    // (Its Find: the one just over its grid, not the blueprint's.)
    let mut finds: Vec<egui::Rect> =
        h.query_all(egui_kittest::kittest::by().predicate(find)).map(|n| n.rect()).collect();
    finds.retain(|r| r.bottom() <= grid.top() && r.top() > grid.top() - 120.0);
    let mut toolbar = vec![*finds.first().expect("its Find")];
    // (The count, "N of M textures", and Interface textures too.)
    toolbar.extend(h.query_all_by_label_contains(" textures").map(|n| n.rect()));
    assert!(toolbar.len() >= 3, "the toolbar's controls: {toolbar:?}");
    for r in toolbar {
        assert!(screen.contains_rect(r), "{r:?} cut by the window");
        assert!(
            r.left() >= body - 1.0 && r.right() <= cancel.right() + 1.0,
            "{r:?} out of the dialog"
        );
    }
}

/// With the pointer over the floating gallery, the Status Bar names its
/// hints and its keys act on it; its arrows do not move the main window's
/// area view.
#[test]
fn the_floating_gallery_has_its_hints_and_its_keys() {
    let Some((mut h, area)) = area_harness("u26-floating") else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    open_gallery(&mut h);
    kind(&mut h, mg_ui::appearance_gallery::Kind::Placeable);
    own_window(&mut h, &Tab::PlaceableGallery, egui::pos2(900.0, 60.0), egui::vec2(660.0, 700.0));
    settle(&mut h);
    // (The pointer over it, no press: the floating window under it is the
    // editor in use. Its grid took the keys as it opened.)
    let first = (cells_of(&h, "Placeables").into_iter().map(|c| c.1))
        .find(|r| h.ctx.content_rect().contains_rect(*r))
        .expect("a cell in sight");
    h.hover_at(first.center());
    settle(&mut h);
    says(&mut h, &["Click: Take to place", "Drag: Place in an area"], &[]);
    assert!(has_keys(&h, "Placeables"), "the floating gallery's grid has the keys");
    let was = crate::gallery_keys::grid_cursor(&h, "Placeables").map(|c| c.1);
    let camera = |h: &Harness<'_, Moonglow>| format!("{:?}", h.state().area_views[&area].camera());
    let (before, view) = (camera(&h), h.state().area_views[&area].camera().is_some());
    assert!(view, "the area's camera");
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    let cursor = crate::gallery_keys::grid_cursor(&h, "Placeables").map(|c| c.1);
    assert_eq!(cursor, Some(was.map_or(1, |c| c + 1)), "the gallery's keys moved");
    says(&mut h, &["Up or Down: Row of pictures"], &[]);
    for _ in 0..3 {
        h.key_press(egui::Key::ArrowRight);
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
    }
    assert_eq!(camera(&h), before, "the area's view did not move");
}

/// The pointer resting on a picture until its tooltip shows: then no
/// further frames are asked for (none within a second; the gallery a tab
/// in front of the area's, whose animated tiles ask for their own).
#[test]
fn the_gallery_rests_under_a_tooltip() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "u26_tip_rest", ResType::UTC) else {
        return;
    };
    gallery_in_tabs(&mut h, key);
    let screen = h.ctx.content_rect();
    let seen: Vec<egui::Rect> = (cells_of(&h, "Creatures").into_iter().map(|c| c.1))
        .filter(|r| screen.contains_rect(*r))
        .collect();
    let cell = *seen.get(2).expect("cells in sight");
    let tip = tip_at(&mut h, cell).expect("the picture's tooltip");
    settle(&mut h);
    for _ in 0..3 {
        h.step();
        let again = h.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        let why = h.ctx.repaint_causes();
        assert!(again >= std::time::Duration::from_secs(1), "a frame asked in {again:?}: {why:?}");
    }
    let shown = h.query_all_by(|n| {
        n.label().as_deref() == Some(tip.as_str()) || n.value().as_deref() == Some(tip.as_str())
    });
    assert!(shown.count() > 0, "the tooltip {tip:?} still shown");
}

/// An armor's torso pictures (a number over its armor class, "AC N")
/// show two whole rows at every page width and text size: the second
/// row's armor class wholly in sight above the bottom of the scroll that
/// holds them (the height was taken from the least cell, and the grid
/// grows its cells with the page: the second row's "AC N" was cut).
#[test]
fn an_armor_s_torso_pictures_show_two_whole_rows() {
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "u26_armor_rows", ResType::UTI)
    else {
        return;
    };
    h.set_size(egui::vec2(1500.0, 1200.0));
    own_window(&mut h, &Tab::Blueprint(key), egui::pos2(10.0, 10.0), egui::vec2(980.0, 1180.0));
    page(&mut h, "Appearance");
    settle(&mut h);
    h.get_by(|n| {
        n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some("Torso")
    })
    .click();
    settle(&mut h);
    let name = "Pictures of Torso";
    for zoom in [1.0_f32, 1.25, 1.5] {
        text_size(&mut h, zoom);
        for width in [700.0_f32, 1000.0, 1400.0] {
            let what = format!("{width} wide at {zoom}");
            let size = egui::vec2(width, 1180.0) / zoom;
            own_window(&mut h, &Tab::Blueprint(key), egui::pos2(10.0, 10.0) / zoom, size);
            settle(&mut h);
            // (Its pictures listed once they are read.)
            let start = std::time::Instant::now();
            while h
                .query_by(|n| {
                    n.role() == egui::accesskit::Role::List && n.label().as_deref() == Some(name)
                })
                .is_none()
            {
                assert!(start.elapsed().as_secs() < 20, "{what}: no torso pictures");
                settle(&mut h);
            }
            // (The page scrolled, a little at a time, until the pictures
            // are near the top of the window: the pointer on the page above
            // them, not on them.)
            let to_pictures = |h: &mut Harness<'_, Moonglow>| {
                let (start, mut last) = (std::time::Instant::now(), None);
                loop {
                    // (Near the top, or the page at its foot.)
                    let (grid, _) = grid_of(h, name);
                    if grid.top() <= 400.0 / zoom || last == Some(grid.top()) {
                        break;
                    }
                    last = Some(grid.top());
                    assert!(start.elapsed().as_secs() < 20, "{what}: never scrolled to {grid:?}");
                    h.hover_at(egui::pos2(250.0, 100.0));
                    h.event(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -60.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    });
                    settle(h);
                }
            };
            to_pictures(&mut h);
            // (The scroll's bar runs its whole height, beside the grid: the
            // shortest there, not the page's.)
            let bar = |h: &Harness<'_, Moonglow>| {
                let (grid, _) = grid_of(h, name);
                h.query_all_by(|n| n.role() == egui::accesskit::Role::ScrollBar)
                    .map(|n| n.rect())
                    .filter(|r| r.left() >= grid.right() - 20.0 && r.left() <= grid.right() + 20.0)
                    .filter(|r| r.top() >= grid.top() - 1.0 && r.top() < grid.bottom())
                    .min_by(|a, b| a.height().total_cmp(&b.height()))
                    .unwrap_or_else(|| panic!("{what}: no scroll bar beside {grid:?}"))
            };
            // (Its pictures scrolled to their top: they open at the armor's
            // own, a row partly above.)
            let seen = bar(&h);
            h.hover_at(egui::pos2(seen.left() - 40.0, seen.center().y));
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 4000.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
            settle(&mut h);
            // (The page under it scrolled too, at the pictures' top.)
            to_pictures(&mut h);
            let (bar, (grid, _)) = (bar(&h), grid_of(&h, name));
            assert!((bar.top() - grid.top()).abs() < 1.0, "{what}: at its top: {grid:?} {bar:?}");
            // (The rows in sight: from the scroll's top.)
            let cells: Vec<(usize, egui::Rect)> =
                (cells_of(&h, name).into_iter()).filter(|c| c.1.top() >= bar.top() - 0.5).collect();
            let mut tops: Vec<f32> = cells.iter().map(|c| c.1.top()).collect();
            tops.dedup_by(|a, b| (*a - *b).abs() < 0.5);
            assert!(
                tops.len() >= 2,
                "{what}: two rows built: {tops:?} in {bar:?} of {grid:?}: {:?}",
                cells_of(&h, name)
            );
            let second: Vec<egui::Rect> =
                cells.iter().map(|c| c.1).filter(|r| (r.top() - tops[1]).abs() < 0.5).collect();
            for r in second {
                assert!(r.bottom() <= bar.bottom() + 0.5, "{what}: {r:?} cut at {}", bar.bottom());
            }
            // (No third row in sight: two rows' height, not more.)
            if let Some(third) = tops.get(2) {
                assert!(*third >= bar.bottom() - 1.0, "{what}: a third row at {third} in {bar:?}");
            }
        }
    }
    text_size(&mut h, 1.0);
}

/// The Appearance Gallery's tab, floating in a window of its own, from
/// its least width to the greatest at each text size: its header holds
/// every control (none cut), the hint strip's words stand inside the
/// tab, and every cell in sight is inside it.
#[test]
fn the_gallery_holds_from_least_to_greatest_width() {
    use mg_uikit::header::{self, Place};
    let Some((mut h, _)) = area_harness("u26-sweep-tab") else { return };
    h.set_size(egui::vec2(1700.0, 1000.0));
    open_gallery(&mut h);
    kind(&mut h, mg_ui::appearance_gallery::Kind::Placeable);
    for zoom in [1.0_f32, 1.25, 1.5] {
        text_size(&mut h, zoom);
        let screen = egui::vec2(1700.0, 1000.0) / zoom;
        for width in [360.0_f32, 480.0, 640.0, 900.0, 1200.0, 1600.0] {
            let width = width.min(screen.x - 40.0);
            let what = format!("{width} wide at {zoom}");
            let size = egui::vec2(width, (760.0_f32).min(screen.y - 60.0));
            own_window(&mut h, &Tab::PlaceableGallery, egui::pos2(20.0, 30.0), size);
            settle(&mut h);
            // (The tab's pane in its window, as the dock has it.)
            let window = (h.state().dock.iter_leaves())
                .find(|(p, _)| !p.surface.is_main())
                .map(|(_, leaf)| leaf.rect)
                .expect("the gallery's window")
                .expand(1.0);
            let rec = header::records(&h.ctx, mg_ui::appearance_gallery::HEADER);
            let rec = rec.first().unwrap_or_else(|| panic!("{what}: no header"));
            assert!(!rec.places.iter().any(|(_, p)| *p == Place::Cut), "{what}: {:?}", rec.places);
            assert!(window.contains_rect(rec.row), "{what}: the header {:?}", rec.row);
            // (Its words, cut with an ellipsis where narrow.)
            let strip = h.query_all_by_label_contains("A click takes a placeable").next();
            let strip = strip.unwrap_or_else(|| panic!("{what}: no strip")).rect();
            assert!(
                window.contains_rect(strip),
                "{what}: the strip's words {strip:?} in {window:?}"
            );
            let (grid, _) = grid_of(&h, "Placeables");
            for (_, c) in cells_of(&h, "Placeables") {
                assert!(c.left() >= window.left() && c.right() <= window.right(), "{what}: {c:?}");
                assert!(c.left() >= grid.left() - 0.5 && c.right() <= grid.right() + 0.5);
            }
        }
    }
    text_size(&mut h, 1.0);
}

/// Select Portrait, Select Texture and the Creature Wizard from the
/// least window to the greatest at each text size: OK and Cancel (the
/// wizard's Next and Cancel) whole inside the window, and never taller
/// where wider.
#[test]
fn the_pickers_hold_from_least_to_greatest_width() {
    let open = |h: &mut Harness<'_, Moonglow>, which: &str| match which {
        "Select Portrait" => {
            // (Brought into sight first: below the Sound set and Faction
            // rows, which develop's Builders' asks put on the Basic page,
            // it is under the fold of a short window.)
            h.get_by_label("Portraits…").scroll_to_me();
            settle(h);
            h.get_by_label("Portraits…").click();
            settle(h);
        }
        "Select Texture" => {
            h.get_all_by_label("Select Texture…").next().unwrap().click();
            settle(h);
        }
        _ => {
            let mut w = mg_ui::creature_wizard::CreatureWizard::default();
            (w.page, w.race, w.appearance) = (3, Some(6), 6);
            w.classes = vec![(4, 1)];
            h.state_mut().creature_wizard = Some(w);
            settle(h);
        }
    };
    let close = |h: &mut Harness<'_, Moonglow>, which: &str| {
        if which == "the Creature Wizard" {
            h.state_mut().creature_wizard = None;
        } else {
            h.key_press(egui::Key::Escape);
        }
        settle(h);
    };
    for (bp, t, which) in [
        ("nw_bandit001", ResType::UTC, "Select Portrait"),
        ("plc_chest1", ResType::UTP, "Select Texture"),
        ("nw_bandit001", ResType::UTC, "the Creature Wizard"),
    ] {
        // (A window for each text size: AccessKit's boxes are read in
        // the points of the size the harness was last built at.)
        for zoom in [1.0_f32, 1.25, 1.5] {
            let Some((mut h, _)) = blueprint_harness(bp, "u26_sweep_picker", t) else { return };
            h.set_size(egui::vec2(1600.0, 900.0));
            if which == "Select Texture" {
                page(&mut h, "Visuals");
            }
            settle(&mut h);
            text_size(&mut h, zoom);
            let mut last: Option<(f32, f32)> = None;
            for points in [560.0_f32, 700.0, 900.0, 1200.0, 1600.0] {
                let what = format!("{which}, {points} points wide at {zoom}");
                h.set_size(egui::vec2(points, 720.0) * zoom);
                h.run_steps(3);
                settle(&mut h);
                open(&mut h, which);
                let screen = h.ctx.content_rect();
                let (ok, cancel) = match which {
                    "the Creature Wizard" => ("Next >", "Cancel"),
                    _ => ("OK", "Cancel"),
                };
                // (Its own: the lowest of the name, the blueprint's window
                // has an OK and a Cancel at its top.)
                let lowest = |h: &Harness<'_, Moonglow>, n: &str| {
                    (h.query_all_by_label(n).map(|n| n.rect()))
                        .max_by(|a, b| a.bottom().total_cmp(&b.bottom()))
                        .unwrap_or_else(|| panic!("{what}: no {n}"))
                };
                let (ok, cancel) = (lowest(&h, ok), lowest(&h, cancel));
                for (n, r) in [("OK", ok), ("Cancel", cancel)] {
                    assert!(screen.contains_rect(r), "{what}: {n} {r:?} cut in {screen:?}");
                }
                // (Its grid whole across: in the window and inside the
                // footer's width, not cut at the dialog's right.)
                let name = if which == "Select Texture" { "Textures" } else { "Portraits" };
                let (grid, _) = grid_of(&h, name);
                // (A picker's Cancel is its footer's rightmost; the wizard's
                // footer holds its buttons left of its preview.)
                let right = match which {
                    "the Creature Wizard" => screen.right(),
                    _ => cancel.right().min(screen.right()),
                };
                assert!(
                    grid.left() >= screen.left() - 1.0 && grid.right() <= right + 1.0,
                    "{what}: its grid {grid:?} cut (Cancel {cancel:?}, {screen:?})"
                );
                // (Select Texture's preview beside it, named, whole too.)
                if which == "Select Texture" {
                    let to_draw = h
                        .query_all_by(|n| n.value().is_some_and(|l| l.starts_with("To draw ")))
                        .next()
                        .unwrap_or_else(|| panic!("{what}: no To draw NAME with"))
                        .accesskit_node()
                        .value()
                        .unwrap();
                    let old = to_draw.trim_start_matches("To draw ").trim_end_matches(" with");
                    let old = old.to_lowercase();
                    let named = h
                        .query_all_by(|n| n.value().as_deref() == Some(old.as_str()))
                        .map(|n| n.rect())
                        .find(|r| r.left() > grid.right())
                        .unwrap_or_else(|| panic!("{what}: the preview's name not shown"));
                    assert!(
                        named.right() <= right + 1.0,
                        "{what}: the preview's name {named:?} cut (Cancel {cancel:?})"
                    );
                }
                // (Its height: from its title to its footer.)
                let title = h.query_all_by_label(which.trim_start_matches("the ")).last();
                let top = title.unwrap_or_else(|| panic!("{what}: not open")).rect().top();
                let tall = ok.bottom() - top;
                if let Some((w, t)) = last {
                    assert!(tall <= t + 1.0, "{what}: {tall} tall, {t} at {w}");
                }
                last = Some((points, tall));
                close(&mut h, which);
            }
        }
    }
}
