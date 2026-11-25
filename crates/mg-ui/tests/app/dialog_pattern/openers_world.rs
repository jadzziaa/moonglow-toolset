//! Openers for the dialogs of the area view and its tools: Resize, Rotate and
//! Change Tileset, Adjust Location, Edit Areas Together, Create Set, Bake
//! Walkmesh, Select a Color, Custom Light, Save as Prefab and Palette
//! Categories. (Find Instance and Resources Used are the Bottom Dock's
//! since M4's U28: `search_results.rs`, `dock_reports.rs`.)

use super::Opener;
use super::openers_core::clear_field;
use crate::common::*;
use glam::Vec3;
use mg_area::ObjectKind;

/// An area open in a window with the game's module.
fn area() -> Option<Harness<'static, Moonglow>> {
    area_harness(&super::scratch("u27-world")).map(|(h, _)| h)
}

/// A UI-only area open for dialogs that do not inspect the 3D viewport.
fn ui_area() -> Option<Harness<'static, Moonglow>> {
    area_ui_harness(&super::scratch("u27-world-ui")).map(|(h, _)| h)
}

/// The area the window shows.
fn area_of(h: &Harness<'static, Moonglow>) -> ResRef {
    *h.state().area_views.keys().next().expect("an area open")
}

/// An item of the Edit menu.
fn edit_item(h: &mut Harness<'static, Moonglow>, item: &str) {
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label(item).click();
    settle(h);
}

fn right_click(h: &mut Harness<'static, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
}

/// The right-click menu of the area's first waypoint (at 20, 20).
fn waypoint_menu(h: &mut Harness<'static, Moonglow>) {
    let area = area_of(h);
    let at = screen(h, area, Vec3::new(20.0, 20.0, 0.02));
    right_click(h, at);
}

/// The area view with the first two waypoints selected.
fn two_waypoints() -> Option<Harness<'static, Moonglow>> {
    let mut h = area()?;
    let area = area_of(&h);
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    h.run_steps(2);
    Some(h)
}

/// The area wide enough for the Right Dock's Tile Properties.
fn wide() -> Option<Harness<'static, Moonglow>> {
    let (mut h, area) = area_harness(&super::scratch("u27-world-tiles"))?;
    wide_window(&mut h);
    // (Tiles 5 and 6 differ in a field, as Tile Properties' own tests have them.)
    for (index, value) in [(5, 3), (6, 11)] {
        let set = mg_edit::Edit::SetField {
            key: ResKey::new(area, ResType::ARE),
            path: mg_edit::GffPath::root().item("Tile_List", index),
            label: "Tile_MainLight2".into(),
            value: Some(mg_gff::Value::Byte(value)),
        };
        let setup = mg_edit::Command::new("Setup", vec![set]);
        h.state_mut().actions.push(mg_ui::Action::Apply(setup));
        h.run_steps(3);
    }
    Some(h)
}

/// A module of the game's with its palette over a placeable's tab closed.
fn palette() -> Option<Harness<'static, Moonglow>> {
    use mg_module::palette::BlueprintKind;
    let (mut h, key) = blueprint_harness("plc_chest1", "u27_categories", ResType::UTP)?;
    h.run();
    let app = h.state_mut();
    close_tab(app, &Tab::Blueprint(key));
    show_palette(app);
    app.palette.kind = BlueprintKind::Placeable;
    app.palette.tiles = false;
    app.palette.custom = true;
    h.run();
    Some(h)
}

/// A module of the project's bake rules and armoires (Bake Walkmesh).
fn bake() -> Option<Harness<'static, Moonglow>> {
    bake_harness(&super::scratch("u27-world"), Some(RULES)).map(|(h, ..)| h)
}

/// The area's first armoire's Position menu, Bake Walkmesh to Transform.
fn open_bake(h: &mut Harness<'static, Moonglow>) {
    let area = area_of(h);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.run_steps(2);
    let at = screen(h, area, Vec3::new(15.0, 15.0, 0.0));
    right_click(h, at);
    open_submenu(h, "Position");
    h.get_by_label("Bake Walkmesh to Transform…").click();
    h.run_steps(4);
}

/// Baked, unbaked, then saved: the save asks about the bake no placeable uses.
fn open_tidy(h: &mut Harness<'static, Moonglow>) {
    open_bake(h);
    h.get_by_label("Bake").click();
    settle(h);
    let area = area_of(h);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.run_steps(2);
    let at = screen(h, area, Vec3::new(15.0, 15.0, 0.0));
    right_click(h, at);
    open_submenu(h, "Position");
    h.get_by_label("Unbake Walkmesh").click();
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
}

pub(crate) fn openers() -> Vec<Opener> {
    vec![
        Opener::new("area_reshape.rs:resize_window#1", "Resize Area", ui_area, |h| {
            edit_item(h, "Resize Area…")
        }),
        Opener::new("area_reshape.rs:rotate_window#1", "Rotate Area", ui_area, |h| {
            edit_item(h, "Rotate Area…")
        }),
        Opener::new("area_reshape.rs:tileset_window#1", "Change Tileset", ui_area, |h| {
            edit_item(h, "Change Tileset…")
        }),
        Opener::new("area_tools.rs:adjust_window#1", "Adjust Location", area, |h| {
            let area = area_of(h);
            h.state_mut().area_views.get_mut(&area).unwrap().selection =
                vec![(ObjectKind::Waypoint, 0)];
            h.run_steps(2);
            waypoint_menu(h);
            h.get_by_label("Adjust Location…").click();
            settle(h);
        }),
        Opener::new("area_view.rs:set_window#1", "Create Set", ui_area, |h| {
            let area = area_of(h);
            h.state_mut().area_views.get_mut(&area).unwrap().set_name = Some(String::new());
        })
        .field(0)
        .off(|h| clear_field(h, "Create Set", 0)),
        Opener::new("bake_view.rs:save_window#1", "Baked Models No Longer Used", bake, open_tidy),
        Opener::new(
            "tileset_view.rs:minimap_progress#1",
            "Render Minimap Pictures",
            ui_area,
            |h| mg_ui::tileset_view::hold_minimap_for_tests(h.state_mut()),
        )
        .job(),
        Opener::new("tile_select.rs:panel#1", "Select a Color", wide, |h| {
            let area = area_of(h);
            area_click(h, "Select Tiles");
            let at = screen(h, area, Vec3::new(15.0, 15.0, 0.0));
            h.hover_at(at);
            press(h, at, true, egui::Modifiers::NONE);
            press(h, at, false, egui::Modifiers::NONE);
            h.run_steps(3);
            let at = screen(h, area, Vec3::new(25.0, 15.0, 0.0));
            h.hover_at(at);
            press(h, at, true, egui::Modifiers::COMMAND);
            press(h, at, false, egui::Modifiers::COMMAND);
            settle(h);
            // (The tile menu's Tile Properties shows the panel in whatever room there is.)
            right_click(h, at);
            popup_item(h, "Tile Properties").click();
            settle(h);
            h.get_by_label("Main light 1 color").click();
            settle(h);
        })
        .bare(),
        Opener::new("prefabs.rs:save_window#1", "Save as Prefab", two_waypoints, |h| {
            waypoint_menu(h);
            open_submenu(h, "Reuse");
            h.get_by_label("Save as Prefab…").click();
            settle(h);
        })
        .field(0)
        .off(|h| clear_field(h, "Save as Prefab", 0)),
        Opener::new(
            "palette_categories.rs:window#1",
            "Palette Categories: Placeables",
            palette,
            |h| {
                // (Where the window's width folds the Right Dock, unfolded by hand.)
                unfold_right_dock(h);
                palette_more(h).click();
                h.run();
                h.get_by_label("Categories…").click();
                settle(h);
            },
        ),
    ]
}

pub(crate) const RULES: &str =
    "[bake]\nrows = \"50000-50099\"\nhak = \"mg_baked\"\nnames = \"mgbk_\"\n";

/// The area's two armoires (`plc_armoire`, model `plc_a01`): at (15, 15)
/// at Scale 0.5 turned 40 (dynamic), and plain at (27, 15); and at
/// (21, 25), scaled as the first, a placeable of the first row of the
/// game's whose model has no walkmesh.
pub(crate) fn armoires(m: &mut Module, game: &mg_rules::GameData, area: ResRef) {
    use mg_module::instances::{Placement, Placing, instance};
    let key = ResKey::parse("plc_armoire", ResType::UTP).unwrap();
    let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let table = game.table("placeables").unwrap();
    let bare = (0..table.len())
        .find(|&r| {
            table.get(r, "ModelName").is_some_and(|m| {
                game.resman.get_named(m, ResType::MDL).is_ok()
                    && game.resman.get_named(m, ResType::PWK).is_err()
            })
        })
        .expect("a placeable without a walkmesh");
    let mut placed = Vec::new();
    for (x, y, scaled, row) in
        [(15.0, 15.0, true, None), (27.0, 15.0, false, None), (21.0, 25.0, true, Some(bare))]
    {
        let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
        let mut p = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
        p.set("Static", mg_gff::Value::Byte(0));
        if let Some(row) = row {
            p.set("Appearance", mg_gff::Value::Dword(row as u32));
        }
        if scaled {
            let v = mg_area::VisualTransform {
                scale: Vec3::splat(0.5),
                rotate: Vec3::new(40.0, 0.0, 0.0),
                translate: Vec3::ZERO,
            };
            p.set("VisTransformList", mg_area::edit::visual_transform_list(v));
        }
        placed.push(p);
    }
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set("Placeable List", mg_gff::Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
}

/// The area of [`armoires`] in a module folder whose moonglow.toml is
/// `rules` (none: no project file), the game's user folder `user`: the
/// harness, the area and the folder.
pub(crate) fn bake_harness(
    name: &str,
    rules: Option<&str>,
) -> Option<(Harness<'static, Moonglow>, ResRef, std::path::PathBuf, std::path::PathBuf)> {
    bake_harness_with(name, rules, &|_, _| {})
}

/// [`bake_harness`], `more` done in the project's folder (given the area)
/// before it opens.
pub(crate) fn bake_harness_with(
    name: &str,
    rules: Option<&str>,
    more: &dyn Fn(&std::path::Path, ResRef),
) -> Option<(Harness<'static, Moonglow>, ResRef, std::path::PathBuf, std::path::PathBuf)> {
    let user = mg_testkit::scratch_dir(&format!("ui-bake-{name}-user"));
    let (mut app, rs, area) = area_app(name, "ttr01", None, 4, Some(user.clone()), &armoires)?;
    let mut m = app.ws.as_ref()?.module.clone();
    let dir = mg_testkit::scratch_dir(&format!("ui-bake-{name}-project"));
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    if let Some(rules) = rules {
        std::fs::write(dir.join("moonglow.toml"), rules).unwrap();
    }
    more(&dir, area);
    app.open_module(&dir);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    let mut h = area_gpu_harness(app, rs, 1300.0, 900.0);
    h.run_steps(3);
    settle(&mut h);
    Some((h, area, dir, user))
}
