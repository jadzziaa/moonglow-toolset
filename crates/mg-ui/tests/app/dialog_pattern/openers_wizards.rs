//! Openers for the dialogs of the "wizards" group: the Script Wizard, Store
//! Setup, the blueprint wizards, Levelup, and the single-page dialogs of
//! the same sources (see `dialog_pattern.rs`).

use super::Opener;
use super::openers_core::clear_field;
use crate::common::*;
use mg_edit::GffPath;
use mg_gff::{Struct, Value};

/// An empty window with no module open.
fn plain() -> Option<Harness<'static, Moonglow>> {
    Some(harness(app_with(Vec::new()), 1280.0, 720.0))
}

/// A module of the game's open.
fn game() -> Option<Harness<'static, Moonglow>> {
    game_harness(&super::scratch("u27-wizards"))
}

/// A module whose start area's GIT holds four creatures and four
/// placeables, open in a window (no game needed).
fn placed() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-wizards-placed"));
    let path = sample_module(&dir);
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    let mut m = Module::open(&path).unwrap();
    let mut g = Gff::new(*b"GIT ");
    let one = |tag: &str| {
        let mut s = Struct::new(9);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s
    };
    g.root.set("Placeable List", Value::List(["A", "B", "C", "D"].map(one).to_vec()));
    g.root.set("Creature List", Value::List(["a", "b", "c", "d"].map(one).to_vec()));
    m.set(git, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = window_harness(app);
    h.run();
    Some(h)
}

fn start() -> ResRef {
    ResRef::from_str("start").unwrap()
}

/// A bandit blueprint with no feats, its Inventory page shown.
fn featless_bandit() -> Option<Harness<'static, Moonglow>> {
    let (mut h, key) = blueprint_harness("nw_bandit001", "u27_feats", ResType::UTC)?;
    let edit = mg_edit::Edit::SetField {
        key,
        path: GffPath::root(),
        label: "FeatList".into(),
        value: Some(Value::List(Vec::new())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("no feats", vec![edit])));
    h.run();
    page(&mut h, "Inventory");
    h.run();
    // The question is asked here, in the window's roomy first size (a
    // window at the least size shows a part of the page): the item chosen
    // and its slot's Equip pressed.
    type_into_hint(&mut h, "Find", "nw_waxbt001");
    h.get_by_label("Battleaxe").click();
    h.run();
    h.get_all_by_label("Equip").nth(5).unwrap().click();
    h.run();
    Some(h)
}

/// A start area with a door placed in it (and the game's).
fn door_area() -> Option<Harness<'static, Moonglow>> {
    let prepare = |m: &mut Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::instances::{Placement, Placing, instance};
        let key = ResKey::parse("nw_door_normal", ResType::UTD).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [4.0, 4.0, 0.0], rotation: 0.0 };
        let mut door = instance(&placing, ResType::UTD, &bp, at, &[]).unwrap();
        door.set("Tag", Value::String(b"U27_DOOR".to_vec()));
        let key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&key).unwrap().unwrap();
        git.root.set("Door List", Value::List(vec![door]));
        m.set_gff(key, &git).unwrap();
    };
    area_ui_harness_with(&super::scratch("u27-wizards-door"), "ttr01", None, 4, None, &prepare)
        .map(|(h, _)| h)
}

pub(crate) fn openers() -> Vec<Opener> {
    vec![
        Opener::new("script_wizard.rs:window#1", "Script Wizard", plain, |h| {
            let wizard = mg_ui::script_wizard::ScriptWizard::new(
                ResKey::parse("u27dlg", ResType::DLG).unwrap(),
                GffPath::root(),
                "Active",
                true,
                Default::default(),
                "u27script".into(),
            );
            h.state_mut().script_wizard = Some(wizard);
        })
        // (The Skill Check page, the widest: the review's find.)
        .states(vec![("on its Skill Check page", |h| {
            let w = h.state_mut().script_wizard.as_mut().expect("the wizard");
            w.pages[7] = true;
            w.step = mg_ui::script_wizard::Step::Page(7);
        })])
        // (Nothing ticked on its first page: Next goes to the name, the last.)
        .enter_does("goes to its next page", |h| {
            super::is_shown(h, "Script Wizard") && h.query_by_label_contains("Step 13 of").is_some()
        }),
        Opener::new("store_wizard.rs:window#1", "Store Setup Wizard", placed, |h| {
            mg_ui::store_wizard::open(h.state_mut(), start(), "Creature List", 3);
        })
        .enter_does("goes to its next page", |h| {
            super::is_shown(h, "Store Setup Wizard")
                && h.query_by_label_contains("Step 2 of").is_some()
        })
        // (The conversation's file name: the greeting's box is multiline.)
        .field(2)
        .off(|h| clear_field(h, "Store Setup Wizard", 2)),
        Opener::new("store_wizard.rs:popup_window#1", "Add Popup Text", placed, |h| {
            let app = h.state_mut();
            app.popup_text = Some(mg_ui::store_wizard::PopupText::of(app, start(), 3));
            app.popup_text.as_mut().unwrap().text = "Welcome".into();
        })
        .field(0)
        .off(|h| clear_field(h, "Add Popup Text", 0)),
        Opener::new("blueprint_wizard.rs:ui#1", "Item Wizard", game, |h| {
            h.state_mut().blueprint_wizard = Some(mg_ui::blueprint_wizard::BlueprintWizard::new(
                mg_module::palette::BlueprintKind::Item,
            ));
        }),
        Opener::new("levelup_view.rs:window#1", "Creature Levelup Wizard", placed, |h| {
            h.state_mut().levelup = Some(mg_ui::levelup_view::LevelupWizard::new(
                ResKey::new(start(), ResType::GIT),
                GffPath::root().item("Creature List", 2),
                &Struct::new(0),
            ));
        }),
        Opener::new("side_doc.rs:closing_window#1", "Unsaved Changes", plain, |h| {
            let dir = mg_testkit::scratch_dir(&super::scratch("u27-wizards-side"));
            let file = dir.join("loose.2da");
            std::fs::write(&file, "2DA V2.0\n\n Label Cost\n0 Bread 1\n").unwrap();
            let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
            h.run_steps(3);
            let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
            doc.edit.set(0, 0, Some("Loaf".into())).unwrap();
            h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(id)]));
        }),
        Opener::new(
            "blueprint/creature_lists.rs:inventory#1",
            "Missing Feats",
            featless_bandit,
            |h| {
                // (Asked already by the set-up; asked again where it was answered.)
                if h.query_by_label("Missing Feats").is_none() {
                    h.get_all_by_label("Equip").nth(5).unwrap().click();
                    h.run();
                }
            },
        ),
        Opener::new(
            "blueprint/situated.rs:setup_transition#1",
            "Setup Area Transition",
            door_area,
            |h| {
                let area = h.state().ws.as_ref().unwrap().module.areas().unwrap()[0];
                let path = GffPath::root().item("Door List", 0);
                h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path }));
                settle(h);
                h.get_all_by_label("Area Transition").next().unwrap().click();
                settle(h);
                h.get_by_label("Setup Area Transition…").click();
                settle(h);
            },
        ),
    ]
}
