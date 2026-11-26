//! Bake Walkmesh to Transform (the area view's Position menu): offered
//! with the project's `[bake]` and a visual transform; its window says
//! what it makes and shows the walkmesh before and after; baking writes
//! the hak and the rows file as one step of Undo; Unbake; and a save's
//! question where baked models are no longer used.

use crate::common::*;
use crate::dialog_pattern::openers_world::{RULES, bake_harness, bake_harness_with};
use glam::Vec3;
use mg_area::ObjectKind;

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
    // (The review's A20: what the menu says is worked out as it opens,
    // each placeable's struct and walkmesh read once, not every frame.)
    let worked = h.state().bake.menus_worked;
    h.run_steps(12);
    assert_eq!(h.state().bake.menus_worked, worked, "not worked out again while it is open");
    assert!(!h.get_by_label("Bake Walkmesh to Transform…").accesskit_node().is_disabled());
    // (With the header's Overlays switch off: the window shows the
    // walkmeshes before and after all the same, the switch turned on.)
    h.state_mut().area_views.get_mut(&area).unwrap().hide_overlays = true;
    h.get_by_label("Bake Walkmesh to Transform…").click();
    h.run_steps(3);
    let d = h.state().bake.draft.as_ref().expect("the window");
    assert_eq!(d.objects, [0]);
    let view = &h.state().area_views[&area];
    assert!(view.overlay(view.object_walkmesh), "the objects' walkmeshes drawn");
    assert!(!h.state().settings.hide_overlays, "the switch kept on");
    h.get_by_label_contains("plc_a01 is copied as mgbk_0001");
}

/// The window says what it makes, the view shows the walkmesh before and
/// after; Bake writes the hak and the rows file and sets the Appearance,
/// one step that Undo takes back whole and Redo makes again; Unbake sets
/// it back; and the save asks of the bake no placeable uses (Tidy Up).
#[test]
fn a_bake_is_one_step_of_undo_and_tidied_when_unused() {
    let Some((mut h, area, dir, user)) = bake_harness("bake", Some(RULES)) else { return };
    let out = shot_dir("ui-bake");
    if let Some(ref out) = out {
        for f in ["before.png", "window.png", "baked.png"] {
            let _ = std::fs::remove_file(out.join(f));
        }
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
    if let Some(ref out) = out {
        h.render().expect("render").save(out.join("before.png")).unwrap();
    }
    position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    h.get_by_label("Bake Walkmesh to Transform…").click();
    h.run_steps(4);
    h.get_by_label_contains("row 50000");
    h.get_by_label_contains("Writes mgbk_0001.mdl, mgbk_0001.pwk into mg_baked.hak");
    h.get_by_label_contains("Adds the rows to baked_rows.toml");
    let hak = user.join("hak/mg_baked.hak");
    let rows = dir.join("baked_rows.toml");
    assert!(!hak.exists() && !rows.exists(), "nothing written yet");
    if let Some(ref out) = out {
        h.render().expect("render").save(out.join("window.png")).unwrap();
    }
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
    if let Some(ref out) = out {
        h.render().expect("render").save(out.join("baked.png")).unwrap();
    }

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

/// Bake Walkmesh to Transform is a section of the Right Dock (M4's U29;
/// a window before): its command shows the hidden dock with it in front
/// and gives it the keys, held over the area's view under the pointer;
/// Escape is its Cancel (the view keeps its selection: the Escape is not
/// its too), Enter its Bake, neither under a dialog; its area's tab closed,
/// it closes.
#[test]
fn bake_walkmesh_is_a_right_dock_section_with_its_keys() {
    let Some((mut h, area, _, _)) = bake_harness("section", Some(RULES)) else { return };
    let original = appearance(&mut h, area, 0);
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    assert!(h.state().right_dock_shows().is_none(), "hidden");
    let open = |h: &mut Harness<'static, Moonglow>| {
        position_menu(h, area, 0, Vec3::new(15.0, 15.0, 0.0));
        h.get_by_label("Bake Walkmesh to Transform…").click();
        settle(h);
        // (The pointer over the area's view: the keys stay the section's.)
        h.hover_at(screen(h, area, Vec3::new(20.0, 20.0, 0.0)));
        settle(h);
    };
    // At the least window: at rest, and each control Tab reaches in sight.
    let size = h.ctx.content_rect().size();
    h.set_size(mg_ui::LEAST_WINDOW_SIZE);
    settle(&mut h);
    open(&mut h);
    h.try_run().unwrap_or_else(|e| panic!("the section at rest, a frame asks another: {e}"));
    let dock = right_dock_rect(&h);
    let reached = crate::tool_sections::tabbed_in_sight(&mut h, dock, 400);
    assert!(reached >= 2, "Tab reached {reached} of the section's controls");
    h.state_mut().bake.draft = None;
    h.set_size(size);
    settle(&mut h);
    open(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Bake));
    h.get_by_label("1 placeable");
    h.get_by_label_contains("Writes mgbk_0001.mdl");
    // Escape: Cancel, and only that.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().bake.draft.is_none(), "Escape: Cancel");
    assert_eq!(
        h.state().area_views[&area].selection,
        vec![(ObjectKind::Placeable, 0)],
        "the view's selection kept"
    );
    assert_ne!(h.state().right_dock_shows(), Some(mg_ui::Panel::Bake), "the section gone");
    // Under a dialog, Enter and Escape are the dialog's.
    open(&mut h);
    h.state_mut().about = true;
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().about, "the dialog closed");
    assert!(h.state().bake.draft.is_some(), "the section stays");
    assert_eq!(appearance(&mut h, area, 0), original, "nothing baked under the dialog");
    // Its area's tab closed: it closes.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Area(area)]));
    settle(&mut h);
    assert!(h.state().bake.draft.is_none(), "closed with its area");
    // Enter: Bake.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    settle(&mut h);
    open(&mut h);
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    settle(&mut h);
    assert!(h.state().bake.draft.is_none());
    assert_eq!(appearance(&mut h, area, 0), 50000, "Enter: Bake");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Bake Walkmesh to Transform"));
}

/// A press on Properties over Bake's section (1600 by 900, the placeable
/// selected) takes its keys (M4's U29 review): neither Enter (Bake) nor
/// Escape (Cancel) is its after it, nothing baked, the draft kept, its
/// outline gone. (Bake has no field: Escape in one does not apply.)
#[test]
fn a_press_on_properties_takes_bake_s_keys() {
    let Some((mut h, area, _, _)) = bake_harness("press-properties", Some(RULES)) else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    let original = appearance(&mut h, area, 0);
    position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    h.get_by_label("Bake Walkmesh to Transform…").click();
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Bake", "the section's keys");
    let dock = right_dock_rect(&h);
    let row = (h.get_all_by_label("Bake").map(|n| n.rect()))
        .find(|r| dock.contains_rect(*r))
        .expect("Bake in the dock");
    // (Over the section's painted header: Properties, the placeable's.)
    click_pos(&h, egui::pos2(dock.center().x, row.top() - 50.0));
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Properties", "Properties took them");
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    settle(&mut h);
    assert_eq!(appearance(&mut h, area, 0), original, "Enter: nothing baked");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().bake.draft.is_some(), "Escape: the draft kept");
    // A press in the section holds them again; one on the area's view
    // (no pane's) takes them from it too.
    let row = (h.get_all_by_label("Bake").map(|n| n.rect()))
        .find(|r| dock.contains_rect(*r))
        .expect("Bake in the dock");
    click_pos(&h, egui::pos2(row.right() + 60.0, row.bottom() + 40.0));
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Bake", "a press in it");
    click_pos(&h, egui::pos2(600.0, 450.0));
    settle(&mut h);
    assert_ne!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Bake", "a press on the view");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().bake.draft.is_some(), "Escape after it: the draft kept");
}

/// Bake Walkmesh to Transform pictured (M4's U29): its section with one
/// placeable; the section in error (its placeable given the appearance
/// of one with no walkmesh while it is open: why, under its actions);
/// the view's menu item refused for two, before any section opens (the
/// second has no walkmesh, as its tip says: named `bake-menu-refused`
/// since the review, `bake-several-refused` having said what the section
/// does not show); and the frame after Bake, in both themes at the
/// boards' sizes and the least window. By hand:
/// `MG_U29_PICS=<folder> cargo test -p mg-ui --test app u29_pictures -- --ignored`.
#[test]
#[ignore]
fn u29_pictures_bake() {
    mg_testkit::gpu::hold();
    for light in [false, true] {
        for size in crate::tool_sections::PICTURE_SIZES {
            let Some((mut h, area, _, _)) = bake_harness("u29-pics", Some(RULES)) else { return };
            h.state_mut().settings.light_theme = light;
            h.set_size(size);
            settle(&mut h);
            position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
            h.get_by_label("Bake Walkmesh to Transform…").click();
            settle(&mut h);
            let enabled = |h: &Harness<'_, Moonglow>| {
                let dock = right_dock_rect(h);
                h.get_all_by_label("Bake")
                    .any(|n| dock.contains_rect(n.rect()) && !n.accesskit_node().is_disabled())
            };
            crate::tool_sections::picture_of(&mut h, "bake-one", "Bake enabled", enabled);
            // (In error: the placeable given the second's appearance, which
            // has no walkmesh, the plan made again.)
            let none = appearance(&mut h, area, 2);
            let edit = mg_edit::Edit::SetField {
                key: ResKey::new(area, ResType::GIT),
                path: "/Placeable List[0]".parse().unwrap(),
                label: "Appearance".into(),
                value: Some(mg_gff::Value::Dword(none as u32)),
            };
            let command = mg_edit::Command::new("No walkmesh", vec![edit]);
            h.state_mut().actions.push(mg_ui::Action::Apply(command));
            settle(&mut h);
            assert_eq!(appearance(&mut h, area, 0), none, "the appearance set");
            // (Its plan in error, or nothing in it to bake: why, either way.)
            let draft = h.state().bake.draft.as_ref().expect("still open");
            let why = match &draft.plan {
                Err(e) => e.clone(),
                Ok(_) => "Nothing here has a walkmesh to bake.".to_string(),
            };
            crate::tool_sections::picture_of(&mut h, "bake-error", "why, in sight", |h| {
                crate::tool_sections::label_in(h, right_dock_rect(h), &why)
            });
            h.key_press(egui::Key::Escape);
            settle(&mut h);
            h.state_mut().actions.push(mg_ui::Action::Undo);
            settle(&mut h);
            h.state_mut().area_views.get_mut(&area).unwrap().selection =
                vec![(ObjectKind::Placeable, 0), (ObjectKind::Placeable, 2)];
            h.run_steps(2);
            let p = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
            right_click(&mut h, p);
            open_submenu(&mut h, "Position");
            // (Not offered: the second has no walkmesh; its tip says so.)
            h.get_by_label("Bake Walkmesh to Transform…").hover();
            h.run_steps(8);
            crate::tool_sections::picture_of(&mut h, "bake-menu-refused", "its tip", |h| {
                let item = "Bake Walkmesh to Transform…";
                h.get_all_by_label(item).any(|n| n.accesskit_node().is_disabled())
                    && h.query_all_by_label_contains("walkmesh").any(|n| {
                        crate::tool_sections::words(&n)
                            .is_some_and(|l| l != item && l.contains(':'))
                    })
            });
            h.key_press(egui::Key::Escape);
            settle(&mut h);
            position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
            h.get_by_label("Bake Walkmesh to Transform…").click();
            settle(&mut h);
            h.key_press(egui::Key::Enter);
            h.run_steps(1);
            crate::tool_sections::picture_of(&mut h, "bake-done", "the section closed", |h| {
                h.state().bake.draft.is_none()
            });
        }
    }
}

/// The section's tab where the Right Dock's tabs are icons alone (1280 by
/// 720, Bake the fourth): named "Bake Walkmesh to Transform" to the
/// screen reader, and its tip says it under the pointer (M4's U29 review:
/// it was an icon with no name in sight).
#[test]
fn bake_s_tab_has_a_name_and_a_tip_where_tabs_are_icons() {
    let Some((mut h, area, _, _)) = bake_harness("tab-icon", Some(RULES)) else { return };
    h.set_size(egui::vec2(1280.0, 720.0));
    settle(&mut h);
    position_menu(&mut h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    h.get_by_label("Bake Walkmesh to Transform…").click();
    settle(&mut h);
    crate::tool_sections::tab_named_with_tip(&mut h, "Bake Walkmesh to Transform");
}

/// Bake opened on its placeable (selected, so Properties shows it).
fn open_bake(h: &mut Harness<'_, Moonglow>, area: ResRef) {
    position_menu(h, area, 0, Vec3::new(15.0, 15.0, 0.0));
    h.get_by_label("Bake Walkmesh to Transform…").click();
    settle(h);
}

/// Bake's section in the Right Dock's column (M4's U29 review): at 1600
/// by 900, Properties showing the placeable, it takes two thirds of the
/// column; its seam dragged, the share is kept for the run (closed and
/// opened again, the same); its note whole in sight, at 1600 by 900 and
/// at 1280 by 720 (one tab bar); outlined whole while it holds the keys.
#[test]
fn bake_takes_two_thirds_of_the_column_and_its_note_is_whole() {
    let Some((mut h, area, _, _)) = bake_harness("split", Some(RULES)) else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    open_bake(&mut h, area);
    let section = h.state().right_section_rect(mg_ui::Panel::Bake).expect("stacked at 1600");
    let props = h.state().right_section_rect(mg_ui::Panel::Properties).expect("Properties");
    let palette = h.state().right_section_rect(mg_ui::Panel::Palette);
    let column = palette.map_or(props, |p| p.union(props)).union(section);
    let share = section.height() / column.height();
    assert!((share - 2.0 / 3.0).abs() < 0.03, "two thirds: {share} ({section:?} of {column:?})");
    let note = h.get_by_label_contains("only what it blocks changes").rect();
    assert!(section.contains_rect(note), "the note whole: {note:?} in {section:?}");
    crate::tool_sections::outlined_whole(&mut h, section.with_min_y(section.top() + 30.0));
    // Its seam dragged up by 60 points: kept for the run.
    let seam = egui::pos2(section.center().x, section.top() - 0.5);
    h.hover_at(seam);
    h.run();
    press(&h, seam, true, egui::Modifiers::NONE);
    h.run();
    for step in 1..=6 {
        h.event(egui::Event::PointerMoved(seam - egui::vec2(0.0, 10.0 * step as f32)));
        h.run();
    }
    press(&h, seam - egui::vec2(0.0, 60.0), false, egui::Modifiers::NONE);
    settle(&mut h);
    let dragged = h.state().right_section_rect(mg_ui::Panel::Bake).unwrap();
    assert!(
        (dragged.top() - (section.top() - 60.0)).abs() < 3.0,
        "dragged: {dragged:?} from {section:?}"
    );
    h.state_mut().bake.draft = None;
    settle(&mut h);
    open_bake(&mut h, area);
    let again = h.state().right_section_rect(mg_ui::Panel::Bake).unwrap();
    assert!((again.height() - dragged.height()).abs() < 2.0, "kept: {again:?}, {dragged:?}");
    // At 1280 by 720: one tab bar, the note whole.
    h.set_size(egui::vec2(1280.0, 720.0));
    settle(&mut h);
    let dock = right_dock_rect(&h);
    let note = h.get_by_label_contains("only what it blocks changes").rect();
    assert!(dock.contains_rect(note), "1280: the note whole: {note:?} in {dock:?}");
}

/// Where Bake cannot bake, why is said under its actions, in sight (M4's
/// U29 review: it was said at the foot of its scroll), Bake disabled.
#[test]
fn bake_says_why_it_is_disabled_under_its_actions() {
    let Some((mut h, area, _, _)) = bake_harness("why", Some(RULES)) else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    open_bake(&mut h, area);
    h.state_mut().bake.draft.as_mut().unwrap().plan = Err("The placeable has no walkmesh.".into());
    settle(&mut h);
    let dock = right_dock_rect(&h);
    let main = (h.get_all_by_label("Bake").map(|n| n.rect()))
        .find(|r| dock.contains_rect(*r))
        .expect("Bake in the dock");
    let bake = h.get_all_by_label("Bake").find(|n| dock.contains_rect(n.rect())).unwrap();
    assert!(bake.accesskit_node().is_disabled(), "Bake disabled");
    let why = h.get_by_label("The placeable has no walkmesh.").rect();
    assert!(
        why.top() >= main.bottom() && why.top() <= main.bottom() + 30.0,
        "under the row: {why:?}, {main:?}"
    );
    assert!(dock.contains_rect(why), "in sight: {why:?}");
}

/// The ground's and the objects' walkmesh overlays are made once and kept
/// while nothing they show changes (a frame at rest draws the kept one),
/// and made again when the camera moves, the view's rect changes, the
/// area is edited (an object moved) or, for the objects', the selection
/// changes; a kept overlay draws as the one just made (pixel for pixel).
#[test]
fn the_walkmesh_overlays_are_kept_until_what_they_show_changes() {
    let Some((mut h, area, _, _)) = bake_harness("kept", None) else { return };
    let out = mg_testkit::output_dir("ui-walkmesh-kept");
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        (view.walkmesh, view.object_walkmesh) = (true, true);
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.pitch, o.distance) = (Vec3::new(20.0, 18.0, 0.0), 1.0, 22.0);
    }
    settle(&mut h);
    let builds = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].overlay_builds;
    // (The picture at one clock: the area's animations as they were.)
    let picture = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().area_views.get_mut(&area).unwrap().set_clock(10.0);
        h.run_steps(1);
        h.render().expect("render")
    };
    let made = builds(&h);
    assert!(made[0] > 0 && made[1] > 0, "both drawn: {made:?}");
    let fresh = picture(&mut h);
    // At rest: frames go by (the clock held: what the area's emitters
    // show stays as it was), nothing made again, the same picture.
    for _ in 0..6 {
        picture(&mut h);
    }
    let kept = picture(&mut h);
    assert_eq!(builds(&h), made, "at rest");
    fresh.save(out.join("made.png")).unwrap();
    kept.save(out.join("kept.png")).unwrap();
    assert!(fresh.as_raw() == kept.as_raw(), "a kept overlay draws as the one made");
    // Each change makes them again (the ground's: not the selection).
    let again = |h: &mut Harness<'_, Moonglow>, what: &str, ground: bool| {
        let was = builds(h);
        settle(h);
        h.run_steps(2);
        let now = builds(h);
        assert!(now[1] > was[1], "{what}: the objects' made again ({was:?} -> {now:?})");
        assert_eq!(now[0] > was[0], ground, "{what}: the ground's ({was:?} -> {now:?})");
        let at_rest = builds(h);
        h.run_steps(4);
        assert_eq!(builds(h), at_rest, "{what}: then kept");
    };
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw += 0.2;
    again(&mut h, "the camera turned", true);
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().distance = 18.0;
    again(&mut h, "the camera nearer", true);
    h.set_size(egui::vec2(1200.0, 860.0));
    again(&mut h, "the window smaller", true);
    let git = ResKey::new(area, ResType::GIT);
    let edit = mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root().item("Placeable List", 1),
        label: "X".into(),
        value: Some(mg_gff::Value::Float(25.0)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Move", vec![edit])));
    again(&mut h, "an armoire moved", true);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 1)];
    again(&mut h, "an armoire selected", false);
}

/// The boxes of the objects selected are made once and kept while nothing
/// they show changes (frames at rest, the pointer moving over the view:
/// 5,000 selected took 3 ms a frame to make them), and made again when the
/// camera moves, the area is edited or the selection changes, and in every
/// frame of a drag (which poses what it moves); kept boxes draw as those
/// just made (pixel for pixel).
#[test]
fn the_selection_s_boxes_are_kept_until_what_they_show_changes() {
    let Some((mut h, area, _, _)) = bake_harness("boxes-kept", None) else { return };
    let out = mg_testkit::output_dir("ui-boxes-kept");
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        let model = view.model.as_ref().unwrap();
        view.selection = (model.objects.iter())
            .filter(|o| o.kind == ObjectKind::Placeable)
            .map(|o| (o.kind, o.index))
            .collect();
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.pitch, o.distance) = (Vec3::new(20.0, 18.0, 0.0), 1.0, 22.0);
    }
    let selected = h.state().area_views[&area].selection.clone();
    assert!(selected.len() > 1, "the placeables selected: {selected:?}");
    settle(&mut h);
    let builds = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].box_builds.get();
    // (The picture at one clock: the area's animations as they were.)
    let picture = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().area_views.get_mut(&area).unwrap().set_clock(10.0);
        h.run_steps(1);
        h.render().expect("render")
    };
    let made = builds(&h);
    assert!(made > 0, "drawn");
    let fresh = picture(&mut h);
    for _ in 0..6 {
        picture(&mut h);
    }
    let kept = picture(&mut h);
    assert_eq!(builds(&h), made, "at rest");
    fresh.save(out.join("made.png")).unwrap();
    kept.save(out.join("kept.png")).unwrap();
    assert!(fresh.as_raw() == kept.as_raw(), "kept boxes draw as those made");
    // The pointer moving over the view: kept.
    let middle = h.state().area_views[&area].rect.center();
    for k in 0..4 {
        h.hover_at(middle + egui::vec2(12.0 * k as f32, 6.0));
        h.run_steps(1);
    }
    assert_eq!(builds(&h), made, "the pointer moving");
    let again = |h: &mut Harness<'_, Moonglow>, what: &str| {
        let was = builds(h);
        settle(h);
        h.run_steps(2);
        assert!(builds(h) > was, "{what}: made again");
        let at_rest = builds(h);
        h.run_steps(4);
        assert_eq!(builds(h), at_rest, "{what}: then kept");
    };
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw += 0.2;
    again(&mut h, "the camera turned");
    let git = ResKey::new(area, ResType::GIT);
    let edit = mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root().item("Placeable List", 1),
        label: "X".into(),
        value: Some(mg_gff::Value::Float(25.0)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Move", vec![edit])));
    again(&mut h, "an armoire moved");
    h.state_mut().area_views.get_mut(&area).unwrap().selection = selected[1..].to_vec();
    again(&mut h, "one let go of");
    // Merchant signs (which stores draw as a sign or an arrow, and their
    // boxes): made again either way.
    h.state_mut().settings.merchant_signs ^= true;
    again(&mut h, "Merchant signs switched");
    h.state_mut().settings.merchant_signs ^= true;
    again(&mut h, "Merchant signs switched back");
    // A drag of the selection: made again each frame it moves them.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = selected.clone();
    settle(&mut h);
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().unwrap();
    let first = model.objects.iter().position(|o| (o.kind, o.index) == selected[0]).unwrap();
    let grip = view.move_grip(first).expect("a place to hold the first by");
    h.hover_at(grip);
    h.run_steps(1);
    press(&h, grip, true, egui::Modifiers::NONE);
    h.run_steps(1);
    // (The drag begins with the first move, posing them from the next.)
    h.hover_at(grip + egui::vec2(12.0, 0.0));
    h.run_steps(1);
    let was = builds(&h);
    for k in 2..=5 {
        h.hover_at(grip + egui::vec2(12.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    assert!(builds(&h) >= was + 4, "made in every frame of the drag ({was} -> {})", builds(&h));
    press(&h, grip + egui::vec2(60.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let undo = h.state().ws.as_ref().and_then(|w| w.can_undo().map(str::to_owned));
    assert_eq!(undo.as_deref(), Some("Move"), "the drag moved them");
    // Adjust Location open on them, X changed: made again as it changes,
    // kept while it rests.
    let adjust = |h: &mut Harness<'_, Moonglow>, x: f32| {
        let mut a = mg_ui::area_tools::adjust(&h.state().area_views[&area]).expect("Adjust");
        (a.position.x, a.changed[0]) = (x, true);
        h.state_mut().adjust = Some(a);
    };
    adjust(&mut h, 30.0);
    again(&mut h, "Adjust Location's X");
    adjust(&mut h, 32.0);
    again(&mut h, "Adjust Location's X again");
    h.state_mut().adjust = None;
    again(&mut h, "Adjust Location closed");
}
