//! Bake Walkmesh to Transform (the area view's Position menu): offered
//! with the project's `[bake]` and a visual transform; its window says
//! what it makes and shows the walkmesh before and after; baking writes
//! the hak and the rows file as one step of Undo; Unbake; and a save's
//! question where baked models are no longer used.

use crate::common::*;
use glam::Vec3;
use mg_area::ObjectKind;

const RULES: &str = "[bake]\nrows = \"50000-50099\"\nhak = \"mg_baked\"\nnames = \"mgbk_\"\n";

/// The area's two armoires (`plc_armoire`, model `plc_a01`): at (15, 15)
/// at Scale 0.5 turned 40 (dynamic), and plain at (27, 15); and at
/// (21, 25), scaled as the first, a placeable of the first row of the
/// game's whose model has no walkmesh.
fn armoires(m: &mut Module, game: &mg_rules::GameData, area: ResRef) {
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
fn bake_harness(
    name: &str,
    rules: Option<&str>,
) -> Option<(Harness<'static, Moonglow>, ResRef, std::path::PathBuf, std::path::PathBuf)> {
    bake_harness_with(name, rules, &|_, _| {})
}

/// [`bake_harness`], `more` done in the project's folder (given the area)
/// before it opens.
fn bake_harness_with(
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

fn right_click(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
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

/// Selects placeable `index` and opens its menu's Position.
fn position_menu(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, at: Vec3) {
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Placeable, index)];
    h.run_steps(2);
    let p = screen(h, area, at);
    right_click(h, p);
    open_submenu(h, "Position");
}

fn appearance(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize) -> i64 {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    git.root.list("Placeable List").unwrap()[index].integer("Appearance").unwrap()
}

/// Offered only with the project's `[bake]` and on a placeable with a
/// visual transform; a waypoint's Position has no such item.
#[test]
fn bake_walkmesh_is_offered_with_bake_rules_and_a_transform() {
    let Some((mut h, area, _, _)) = bake_harness("offered-none", None) else { return };
    position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    assert!(h.get_by_label("Bake Walkmesh to Transform…").accesskit_node().is_disabled());
    assert!(h.query_by_label("Unbake Walkmesh").is_none());
    h.key_press(egui::Key::Escape);
    drop(h);
    let Some((mut h, area, _, _)) = bake_harness("offered", Some(RULES)) else { return };
    // No visual transform, and a model with no walkmesh: not offered.
    for (i, at) in [(1, Vec3::new(27.0, 15.0, 0.0)), (2, Vec3::new(21.0, 25.0, 0.0))] {
        position_menu(&mut h, area, i, at);
        assert!(h.get_by_label("Bake Walkmesh to Transform…").accesskit_node().is_disabled());
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
    }
    position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    assert!(!h.get_by_label("Bake Walkmesh to Transform…").accesskit_node().is_disabled());
    h.get_by_label("Bake Walkmesh to Transform…").click();
    h.run_steps(3);
    let d = h.state().bake.draft.as_ref().expect("the window");
    assert_eq!(d.objects, [0]);
    h.get_by_label_contains("plc_a01 is copied as mgbk_0001");
}

/// The window says what it makes, the view shows the walkmesh before and
/// after; Bake writes the hak and the rows file and sets the Appearance,
/// one step that Undo takes back whole and Redo makes again; Unbake sets
/// it back; and the save asks of the bake no placeable uses (Tidy Up).
#[test]
fn a_bake_is_one_step_of_undo_and_tidied_when_unused() {
    let Some((mut h, area, dir, user)) = bake_harness("bake", Some(RULES)) else { return };
    let out = mg_testkit::output_dir("ui-bake-pictures");
    for f in ["before.png", "window.png", "baked.png"] {
        let _ = std::fs::remove_file(out.join(f));
    }
    let original = appearance(&mut h, area, 0);
    {
        // (Close over the scaled armoire, from above at a slant.)
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.object_walkmesh = true;
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.pitch, o.distance) = (Vec3::new(15.0, 15.0, 0.0), 1.1, 7.0);
    }
    settle(&mut h);
    h.render().expect("render").save(out.join("before.png")).unwrap();
    position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    h.get_by_label("Bake Walkmesh to Transform…").click();
    h.run_steps(4);
    h.get_by_label_contains("row 50000");
    h.get_by_label_contains("Writes mgbk_0001.mdl, mgbk_0001.pwk into mg_baked.hak");
    h.get_by_label_contains("Adds the rows to baked_rows.toml");
    let hak = user.join("hak/mg_baked.hak");
    let rows = dir.join("baked_rows.toml");
    assert!(!hak.exists() && !rows.exists(), "nothing written yet");
    h.render().expect("render").save(out.join("window.png")).unwrap();
    h.get_by_label("Bake").click();
    h.run_steps(4);
    settle(&mut h);
    assert!(h.state().bake.draft.is_none());
    assert_eq!(appearance(&mut h, area, 0), 50000);
    assert!(hak.is_file() && rows.is_file());
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Bake Walkmesh to Transform"));
    {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let info = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
        assert_eq!(mg_module::haks_of(&info.root).first().map(String::as_str), Some("mg_baked"));
    }
    // Drawn as the build will make it: the copy, its walkmesh the bake's.
    let view = h.state().area_views.get(&area).unwrap();
    let o = view.model.as_ref().unwrap().objects.iter().find(|o| o.kind == ObjectKind::Placeable);
    let model = o.and_then(|o| o.preview.as_ref()).map(|p| p.base.model.clone());
    assert_eq!(model.as_deref(), Some("mgbk_0001"));
    h.render().expect("render").save(out.join("baked.png")).unwrap();

    // Undo: the Appearance, the hak (it was made) and the rows file go.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    settle(&mut h);
    assert_eq!(appearance(&mut h, area, 0), original);
    assert!(!hak.exists() && !rows.exists(), "taken back with the step");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    h.run_steps(3);
    settle(&mut h);
    assert_eq!(appearance(&mut h, area, 0), 50000);
    assert!(hak.is_file() && rows.is_file(), "written again");

    // Unbake: back to the row it copies.
    position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    h.get_by_label("Unbake Walkmesh").click();
    h.run_steps(3);
    assert_eq!(appearance(&mut h, area, 0), original);
    // Saved: no placeable uses the bake now; asked, and tidied.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    h.get_by_label_contains("No placeable of this module uses these any longer");
    h.get_by_label_contains("This module alone was looked at");
    assert!(h.state().ws.as_ref().unwrap().is_modified(), "held for the answer");
    h.get_by_label("Tidy Up").click();
    h.run_steps(4);
    settle(&mut h);
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "saved");
    let left = mg_module::bake::RowsFile::read(&rows).unwrap().unwrap();
    assert!(left.rows.is_empty(), "{left:?}");
    let models: Vec<String> = mg_module::hak_edit::Hak::open(&hak)
        .unwrap()
        .items()
        .iter()
        .map(|i| i.key.to_string())
        .collect();
    assert!(models.is_empty(), "{models:?}");
}

/// Keep Them: the save goes on, and the same leftovers aren't asked of
/// again.
#[test]
fn leftovers_kept_are_not_asked_of_again() {
    let Some((mut h, area, dir, _)) = bake_harness("keep", Some(RULES)) else { return };
    // A row no placeable uses, as another module's bake would leave it.
    let rows = "[[row]]\nrow = 50005\ncopy_of = 0\nmodel = \"mgbk_0042\"\nlabel = \"x\"\n";
    std::fs::write(dir.join("baked_rows.toml"), rows).unwrap();
    h.state_mut().reload_project_files();
    settle(&mut h);
    // (Something to save.)
    let _ = appearance(&mut h, area, 0);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 1)];
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    h.get_by_label_contains("row 50005 (mgbk_0042)");
    h.get_by_label("Keep Them").click();
    h.run_steps(3);
    assert_eq!(
        mg_module::bake::RowsFile::read(&dir.join("baked_rows.toml")).unwrap().unwrap().rows.len(),
        1
    );
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert!(h.query_by_label("Keep Them").is_none(), "not asked again");
}

/// The save's questions one after another: the area another builder
/// claimed (Save Anyway), then the bake no placeable uses (Keep Them);
/// each asked once, and the save made.
#[test]
fn the_save_s_questions_are_each_asked_once() {
    let rules = format!("{RULES}\n[claims]\nfile = \"claims.2da\"\nothers = \"warn\"\n");
    let claims = |dir: &std::path::Path, area: ResRef| {
        let table = format!(
            "2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n\
             0   {:<16} cd       claimed  2026-10-20  ****\n",
            area.to_string()
        );
        std::fs::write(dir.join("claims.2da"), table).unwrap();
        let rows = "[[row]]\nrow = 50005\ncopy_of = 0\nmodel = \"mgbk_0042\"\nlabel = \"x\"\n";
        std::fs::write(dir.join("baked_rows.toml"), rows).unwrap();
        mg_module::project_file::ProjectFile::set_builder_code(dir, Some("ab")).unwrap();
    };
    let Some((mut h, area, _, _)) = bake_harness_with("chain", Some(&rules), &claims) else {
        return;
    };
    // A change to the area another builder holds.
    let git = ResKey::new(area, ResType::GIT);
    let edit = mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root().item("Placeable List", 1),
        label: "Appearance".into(),
        value: Some(mg_gff::Value::Dword(1)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Change", vec![edit])));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    h.get_by_label("Save Anyway").click();
    h.run_steps(3);
    assert!(h.query_by_label("Save Anyway").is_none());
    h.get_by_label("Keep Them").click();
    h.run_steps(4);
    settle(&mut h);
    assert!(h.query_by_label("Save Anyway").is_none(), "the claims not asked again");
    assert!(h.query_by_label("Keep Them").is_none());
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "saved");
}
