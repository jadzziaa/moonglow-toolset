//! The tool sections' keys as the U29 review found them (M4): Enter and
//! Escape in a field of a section are the field's (Enter commits it, Escape
//! gives it up, and the section stays); Enter is Edit Areas Together's Open
//! in Tab on its Areas page alone; the list's hints only where its keys
//! are; a module opened over a section closes it; the ticked areas' set
//! worked out again where what it says changes (the names, the claims),
//! never each frame.

use crate::common::*;
use crate::tool_sections::*;
use egui::Key;
use egui::accesskit::Role;

/// Edit Areas Together on `count` areas with the first `ticked` ticked.
fn ticked_harness(name: &str, count: usize, ticked: usize) -> Harness<'static, Moonglow> {
    let mut h = together_harness(name, count, 1280.0, 720.0);
    open_together(&mut h);
    for _ in 0..ticked {
        key(&mut h, Key::ArrowDown);
        key(&mut h, Key::Space);
    }
    h
}

/// The `n`th field of `role` in the Right Dock, top to bottom.
fn dock_field(h: &Harness<'_, Moonglow>, role: Role, n: usize) -> egui::Rect {
    let dock = right_dock_rect(h);
    let mut rects: Vec<egui::Rect> =
        (h.query_all_by_role(role).map(|f| f.rect())).filter(|r| dock.contains_rect(*r)).collect();
    rects.sort_by(|a, b| a.top().total_cmp(&b.top()));
    *rects.get(n).unwrap_or_else(|| panic!("no field {n} of {role:?} in the dock: {rects:?}"))
}

/// The area `name`'s ARE.
fn are(h: &mut Harness<'_, Moonglow>, name: &str) -> mg_gff::Struct {
    let key = ResKey::parse(name, ResType::ARE).unwrap();
    h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().root.clone()
}

fn open_in_tab(h: &Harness<'_, Moonglow>) -> bool {
    h.state().dock.iter_all_tabs().any(|(_, t)| matches!(t, Tab::AreasProperties(_)))
}

/// Text typed into the field at `at` (clicked first), then `k`.
fn typed_then(h: &mut Harness<'_, Moonglow>, at: egui::Rect, text: &str, k: Key) {
    click_pos(h, at.center());
    settle(h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    h.event(egui::Event::Text(text.into()));
    settle(h);
    key(h, k);
}

/// Enter in a field of a shared page commits the field to every area
/// ticked and the section stays, no tab opened: a script's field (Events),
/// a number (Visual's Fog Amount, Advanced's Listen), a one-line text
/// (a variable's name). The shared pages have no field of several lines.
/// Enter with no field is Open in Tab on the Areas page alone, not on a
/// shared page; and not on the frame the Filter lets go of the keyboard.
#[test]
fn enter_in_a_field_of_a_shared_page_commits_it_and_the_section_stays() {
    let mut h = ticked_harness("u29f-enter", 4, 2);
    rail(&mut h, "Events");
    let script = dock_field(&h, Role::TextInput, 0);
    typed_then(&mut h, script, "my_enter", Key::Enter);
    assert!(h.state().areas_together_ticked().is_some(), "Events: the section stays");
    assert!(!open_in_tab(&h), "Events: no tab opened");
    let enter = ResRef::from_str("my_enter").unwrap();
    for a in ["area0000", "area0001"] {
        assert_eq!(are(&mut h, a).resref("OnEnter"), Some(enter), "{a}: the script set");
    }
    // A second Enter, no field: still not Open in Tab on a shared page.
    key(&mut h, Key::Enter);
    assert!(h.state().areas_together_ticked().is_some() && !open_in_tab(&h), "Events: 2nd Enter");
    rail(&mut h, "Visual");
    let fog = dock_field(&h, Role::SpinButton, 0);
    typed_then(&mut h, fog, "5", Key::Enter);
    assert!(h.state().areas_together_ticked().is_some(), "Visual: the section stays");
    assert!(!open_in_tab(&h), "Visual: no tab opened");
    for a in ["area0000", "area0001"] {
        assert_eq!(are(&mut h, a).integer("SunFogAmount"), Some(5), "{a}: the number set");
    }
    rail(&mut h, "Advanced");
    let listen = dock_field(&h, Role::SpinButton, 0);
    typed_then(&mut h, listen, "7", Key::Enter);
    assert!(h.state().areas_together_ticked().is_some() && !open_in_tab(&h), "Advanced");
    assert_eq!(are(&mut h, "area0001").integer("ModListenCheck"), Some(7), "Listen set");
    // The Areas page: Enter in the Filter is the Filter's...
    rail(&mut h, "Areas");
    let filter = h
        .get(egui_kittest::kittest::by().predicate(|n| n.placeholder() == Some("Name or ResRef")))
        .rect();
    typed_then(&mut h, filter, "area", Key::Enter);
    assert!(h.state().areas_together_ticked().is_some() && !open_in_tab(&h), "the Filter's");
    // ...and with no field, Open in Tab.
    key(&mut h, Key::Enter);
    assert!(open_in_tab(&h), "Areas page, no field: Open in Tab");
}

/// Escape in a field of Edit Areas Together gives up the field alone (a
/// script's name typed, a number), the section stays; a second Escape, no
/// field holding the keyboard, closes it.
#[test]
fn escape_in_a_field_of_edit_areas_together_gives_up_the_field_alone() {
    let mut h = ticked_harness("u29f-escape", 4, 2);
    rail(&mut h, "Events");
    let script = dock_field(&h, Role::TextInput, 1);
    typed_then(&mut h, script, "half", Key::Escape);
    assert!(h.state().areas_together_ticked().is_some(), "a script's field: the section stays");
    assert!(h.ctx.memory(|m| m.focused()).is_none(), "the field given up");
    rail(&mut h, "Visual");
    let fog = dock_field(&h, Role::SpinButton, 1);
    typed_then(&mut h, fog, "3", Key::Escape);
    assert!(h.state().areas_together_ticked().is_some(), "a number: the section stays");
    key(&mut h, Key::Escape);
    assert!(h.state().areas_together_ticked().is_none(), "Escape again: closed");
}

/// The list's hints (Up or Down, Space) only on the Areas page with rows
/// to move in, the keys the section's; Enter's Open in Tab only where
/// Enter opens it.
#[test]
fn the_list_s_hints_only_where_its_keys_are() {
    let has = |h: &Harness<'_, Moonglow>, w: &str| h.state().status_said().iter().any(|s| s == w);
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said();
    let mut h = ticked_harness("u29f-hints", 4, 2);
    for w in ["Up or Down: Move in the list", "Space: Check", "Enter: Open in Tab"] {
        assert!(has(&h, w), "the Areas page, rows: {w} in {:?}", said(&h));
    }
    rail(&mut h, "Visual");
    for w in ["Up or Down: Move in the list", "Space: Check", "Enter: Open in Tab"] {
        assert!(!has(&h, w), "a shared page: {w} in {:?}", said(&h));
    }
    rail(&mut h, "Areas");
    let filter = h
        .get(egui_kittest::kittest::by().predicate(|n| n.placeholder() == Some("Name or ResRef")))
        .rect();
    typed_then(&mut h, filter, "nothing matches this", Key::Tab);
    // (The keys given back with a press in the section, off the Filter.)
    h.get_by_label_contains("Check the areas to edit").click();
    settle(&mut h);
    assert!(!has(&h, "Up or Down: Move in the list"), "no rows: {:?}", said(&h));
    assert!(!has(&h, "Space: Check"), "no rows: {:?}", said(&h));
}

/// A module opened over Edit Areas Together (one with areas of the same
/// names) closes it: no ticks kept by name, nothing set in the new module.
#[test]
fn another_module_opened_closes_the_sections() {
    let mut h = ticked_harness("u29f-module", 4, 2);
    rail(&mut h, "Visual");
    let dir = mg_testkit::scratch_dir("u29f-module-two");
    let other = areas_module(&dir, 4);
    h.state_mut().open_module(&other);
    settle(&mut h);
    assert!(h.state().areas_together_ticked().is_none(), "closed with the module");
    assert_ne!(h.state().right_dock_shows(), Some(mg_ui::Panel::AreasTogether));
    // (Nothing to type into: its fields are gone, and the new module is as
    // it was opened.)
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "the new module untouched");
}

/// Toggling whether areas are said by their names says the first area
/// again as it is now named, on the shared page.
#[test]
fn the_shared_page_says_the_area_as_the_names_setting_says() {
    let mut h = ticked_harness("u29f-names", 4, 2);
    rail(&mut h, "Visual");
    let first = |h: &Harness<'_, Moonglow>| {
        let n = h.get_by_label_contains("Shown as the first,");
        n.accesskit_node()
            .label()
            .map(|l| l.to_string())
            .or(n.accesskit_node().value().map(|v| v.to_string()))
            .unwrap_or_default()
    };
    let before = first(&h);
    let names = h.state().settings.area_names;
    h.state_mut().settings.area_names = !names;
    settle(&mut h);
    let after = first(&h);
    assert_ne!(before, after, "said again as the setting says");
}

/// Edit Areas Together on a project folder of `count` areas that keeps
/// claims (others' areas read-only, the builder's code "ab"), with the first
/// `ticked` ticked; `claims` the table's rows ("area builder").
fn claimed_harness(name: &str, count: usize, ticked: usize) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(name);
    let archive = areas_module(&dir, count);
    let mut m = Module::open(&archive).unwrap();
    let project = dir.join("project");
    m.save_as(&mg_module::ModuleLocation::Folder(project.clone())).unwrap();
    let rules = "[claims]\nfile = \"claims.2da\"\nothers = \"read-only\"\n";
    std::fs::write(project.join("moonglow.toml"), rules).unwrap();
    write_claims(&project, &[]);
    mg_module::project_file::ProjectFile::set_builder_code(&project, Some("ab")).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&project);
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    open_together(&mut h);
    for _ in 0..ticked {
        key(&mut h, Key::ArrowDown);
        key(&mut h, Key::Space);
    }
    h
}

/// The project's claims table: `rows` (area, builder), each claimed.
fn write_claims(project: &std::path::Path, rows: &[(&str, &str)]) {
    let mut table =
        String::from("2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n");
    for (i, (area, who)) in rows.iter().enumerate() {
        table.push_str(&format!("{i}   {area:<16} {who:<8} claimed  2026-10-20  ****\n"));
    }
    std::fs::write(project.join("claims.2da"), table).unwrap();
}

/// The project folder of the module open.
fn project_of(h: &Harness<'_, Moonglow>) -> std::path::PathBuf {
    match h.state().ws.as_ref().unwrap().module.location.clone() {
        Some(mg_module::ModuleLocation::Folder(p)) => p,
        other => panic!("not a folder: {other:?}"),
    }
}

/// With claims kept: the shared pages edit the areas ticked while none is
/// another builder's; one claimed by another while the section is open
/// (the project's files read again) makes them read-only at once, and a
/// change is refused for every one of them (the gate's confirmation and
/// refusal as before: nothing set). Who may edit is asked as the set is
/// made, never each frame.
#[test]
fn a_claim_taken_while_the_section_is_open_makes_its_pages_read_only() {
    let mut h = claimed_harness("u29f-claims", 4, 2);
    rail(&mut h, "Visual");
    assert!(h.query_by_label_contains("read-only here").is_none(), "free: editable");
    click_in_sight(&mut h, "Always Dark");
    settle(&mut h);
    let night = |h: &mut Harness<'_, Moonglow>, a: &str| {
        let key = ResKey::parse(a, ResType::ARE).unwrap();
        h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().root.integer("IsNight")
    };
    assert_eq!([night(&mut h, "area0000"), night(&mut h, "area0001")], [Some(1), Some(1)]);
    let walks = |h: &Harness<'_, Moonglow>| h.state().areas_together_counts().2;
    let before = walks(&h);
    for _ in 0..10 {
        h.hover_at(right_dock_rect(&h).center());
        h.run_steps(1);
    }
    assert_eq!(walks(&h), before, "not asked again each frame");
    // Another builder claims area0001; this builder claims area0002, which
    // reads the project's files again.
    let project = project_of(&h);
    write_claims(&project, &[("area0001", "cd")]);
    h.state_mut().actions.push(mg_ui::Action::Claim(ResRef::from_str("area0002").unwrap()));
    settle(&mut h);
    // (Once as the claim reads the files again, once more where the
    // folder's watch reads them too.)
    let after = walks(&h);
    assert!((before + 1..=before + 2).contains(&after), "asked again, the claims read: {after}");
    h.get_by_label_contains("read-only here");
    click_in_sight(&mut h, "Always Bright");
    settle(&mut h);
    assert_eq!(
        [night(&mut h, "area0000"), night(&mut h, "area0001")],
        [Some(1), Some(1)],
        "nothing set on either"
    );
}

/// Over 3,000 areas in a project that keeps claims: who may edit them is
/// asked once as the set is made, not in each frame of a shared page.
#[test]
fn three_thousand_claimed_areas_are_asked_once() {
    let mut h = claimed_harness("u29f-claims-3000", 3000, 0);
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::Space);
    key(&mut h, Key::End);
    key(&mut h, Key::Space);
    // (Every area checked: Check Shown.)
    click_in_sight(&mut h, "Check Shown");
    settle(&mut h);
    rail(&mut h, "Visual");
    let walks = |h: &Harness<'_, Moonglow>| h.state().areas_together_counts().2;
    let before = walks(&h);
    for page in ["Audio", "Events", "Advanced", "Visual"] {
        rail(&mut h, page);
        for _ in 0..5 {
            h.hover_at(right_dock_rect(&h).center());
            h.run_steps(1);
        }
    }
    assert_eq!(walks(&h), before, "asked once for the set, not each frame");
}

/// An area ticked renamed, then another deleted, under the section: nothing
/// panics, a change is set on the areas still ticked and there, and none on
/// the renamed one.
#[test]
fn an_area_ticked_renamed_or_deleted_under_the_section() {
    let mut h = ticked_harness("u29f-renamed", 4, 3);
    rail(&mut h, "Visual");
    let are_key = |n: &str| ResKey::parse(n, ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::RenameResource {
        from: are_key("area0001"),
        to: ResRef::from_str("renamed01").unwrap(),
        literals: false,
    });
    settle(&mut h);
    h.get_by_label("2 areas checked");
    click_in_sight(&mut h, "Always Dark");
    settle(&mut h);
    assert_eq!(are(&mut h, "area0000").integer("IsNight"), Some(1));
    assert_eq!(are(&mut h, "area0002").integer("IsNight"), Some(1));
    assert_eq!(are(&mut h, "renamed01").integer("IsNight"), Some(0), "the renamed one: untouched");
    h.state_mut().actions.push(mg_ui::Action::DeleteResource(are_key("area0002")));
    settle(&mut h);
    // (One area left of the three: the page says to tick two.)
    h.get_by_label_contains("Check two areas or more");
    assert_eq!(are(&mut h, "area0003").integer("IsNight"), Some(0), "the unticked one: untouched");
}

/// The controls of the Right Dock's stacked section showing `panel` that
/// lie in `rect` cut by its edges: (name, rect) of each shown in part.
fn cut_in(
    h: &Harness<'_, Moonglow>,
    rect: egui::Rect,
    roles: &[Role],
) -> Vec<(String, egui::Rect)> {
    h.query_all_by(|n| roles.contains(&n.role()))
        .filter(|n| n.accesskit_node().bounding_box().is_some())
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
        .filter(|(_, r)| r.intersects(rect) && !rect.expand(0.5).contains_rect(*r))
        .collect()
}

/// The rows of the section's list of areas wholly in `rect`.
fn rows_whole_in(h: &Harness<'_, Moonglow>, rect: egui::Rect) -> usize {
    h.query_all_by(|n| n.label().is_some_and(|l| l.starts_with("area") && l.contains("(Area ")))
        .filter(|n| n.accesskit_node().bounding_box().is_some())
        .filter(|n| rect.contains_rect(n.rect()))
        .count()
}

/// The Right Dock with Edit Areas Together beside an area (M4's U29
/// review's done mark): at 1600 by 900, stacked under the Palette and
/// Properties, the section takes two thirds of the column and, Properties
/// having nothing selected, Properties is its header alone and the section
/// takes the rest; at 1280 by 720 one tab bar, the section the whole dock.
/// Either way at least three rows of its list in sight, its filters and
/// actions whole, its one line of words with the rest as its tip.
#[test]
fn edit_areas_together_shows_three_rows_beside_an_area() {
    let header = mg_uikit::metrics::DOCK_TABS;
    for size in [egui::vec2(1600.0, 900.0), egui::vec2(1280.0, 720.0)] {
        let mut h = together_harness("u29f-split-together", 30, size.x, size.y);
        h.state_mut().settings.hide_palettes = false;
        let area = ResRef::from_str("area0000").unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        settle(&mut h);
        open_together(&mut h);
        let section = match h.state().right_section_rect(mg_ui::Panel::AreasTogether) {
            Some(r) => {
                assert!(size.x >= 1400.0, "{size:?}: stacked only from 1400");
                let props =
                    h.state().right_section_rect(mg_ui::Panel::Properties).expect("Properties");
                assert!(
                    (props.height() - header).abs() < 1.0,
                    "{size:?}: Properties' header alone: {props:?}"
                );
                let dock = right_dock_rect(&h);
                assert!(
                    r.height() >= dock.height() * 2.0 / 3.0 - 2.0,
                    "{size:?}: {r:?} of {dock:?}"
                );
                r
            }
            None => {
                assert!(size.x < 1400.0, "{size:?}: one tab bar under 1400");
                right_dock_rect(&h)
            }
        };
        let rows = rows_whole_in(&h, section);
        assert!(rows >= 3, "{size:?}: {rows} rows of the list in sight in {section:?}");
        let list_top = (h
            .query_all_by(|n| n.label().is_some_and(|l| l.starts_with("area0000 (Area")))
            .next()
            .expect("the first row"))
        .rect()
        .top();
        let above = egui::Rect::from_x_y_ranges(section.x_range(), section.top()..=list_top - 1.0);
        let roles =
            [Role::Button, Role::CheckBox, Role::ComboBox, Role::TextInput, Role::SearchInput];
        let cut = cut_in(&h, above, &roles);
        assert!(cut.is_empty(), "{size:?}: cut in half: {cut:?}");
        h.get_by_label("Check the areas to edit together.");
    }
}

/// Copy This Page to Every Area on one line at the Right Dock's default
/// width, at 1280 and 1600 (M4's U29 review: "Give Every Area This Page's
/// Advanced Settings" wrapped there).
#[test]
fn copy_this_page_to_every_area_is_one_line_at_the_default_width() {
    for size in [egui::vec2(1280.0, 720.0), egui::vec2(1600.0, 900.0)] {
        let mut h = together_harness("u29f-copy-page", 4, size.x, size.y);
        open_together(&mut h);
        for _ in 0..2 {
            key(&mut h, Key::ArrowDown);
            key(&mut h, Key::Space);
        }
        for page in ["Visual", "Advanced", "Audio", "Events"] {
            rail(&mut h, page);
            // (By the words both the button's names share, so that its old
            // name fails here on its height, not on its name.)
            let r = h.get_by_label_contains("Every Area").rect();
            assert!(r.height() <= 26.0, "{size:?} {page}: {r:?} on more than one line");
        }
    }
}

/// The "≠" of a field that differs among the areas ticked: a mark of the
/// body text's height (not 5 by 7 pixels), before the pair of choices it
/// marks (the whole group's), said "mixed" to the screen reader (M4's U29
/// review).
#[test]
fn the_mixed_mark_is_readable_on_the_group_and_says_mixed() {
    let mut h = ticked_harness("u29f-mixed", 4, 3);
    rail(&mut h, "Advanced");
    let marks: Vec<egui::Rect> = h
        .query_all_by(|n| {
            n.label().as_deref() == Some("≠") && n.value().as_deref() == Some("mixed")
        })
        .map(|n| n.rect())
        .collect();
    assert!(!marks.is_empty(), "marked, said mixed");
    let exterior = h.get_by_label("Exterior").rect();
    for m in &marks {
        assert!(m.height() >= 13.0 && m.width() >= 13.0, "readable: {m:?}");
    }
    let before = marks.iter().any(|m| {
        (m.center().y - exterior.center().y).abs() < 6.0 && m.right() <= exterior.left() + 1.0
    });
    assert!(before, "before Exterior's pair: {marks:?}, {exterior:?}");
}

/// The keys' outline round the whole section while it holds them, at
/// 1600 by 900 stacked and at 1280 by 720 in the one tab bar (M4's U29
/// review: one rule for all three sections; Edit Areas Together outlined
/// its actions' row alone).
#[test]
fn edit_areas_together_is_outlined_whole_while_it_holds_the_keys() {
    if no_adapter() {
        return;
    }
    for size in [egui::vec2(1600.0, 900.0), egui::vec2(1280.0, 720.0)] {
        let mut h = together_harness("u29f-outline", 30, size.x, size.y);
        h.state_mut().settings.hide_palettes = false;
        let area = ResRef::from_str("area0000").unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        settle(&mut h);
        open_together(&mut h);
        let dock = right_dock_rect(&h);
        let header = mg_uikit::metrics::DOCK_TABS;
        let section = h.state().right_section_rect(mg_ui::Panel::AreasTogether).unwrap_or(dock);
        let main = (h.get_all_by_label("Open in Tab").map(|n| n.rect()))
            .filter(|r| section.contains_rect(*r))
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("its Open in Tab");
        let top = main.top() - 6.0;
        let body = section.with_min_y(top.max(section.top() + header));
        outlined_whole(&mut h, body);
    }
}

/// Edit Areas Together at the three zooms (100, 125 and 150%) at 1280 by
/// 720: its Areas page and a shared page, each control Tab reaches in its
/// room shown whole (M4's U29 review: untested).
#[test]
fn edit_areas_together_at_the_three_zooms() {
    for scale in [100, 125, 150] {
        let mut h = ticked_harness("u29f-zooms", 6, 2);
        h.state_mut().settings.ui_scale = (scale != 100).then_some(scale);
        h.run_steps(3);
        settle(&mut h);
        let dock = right_dock_rect(&h);
        let reached = tabbed_in_sight(&mut h, dock, 400);
        assert!(reached >= 8, "{scale}%: Tab reached {reached} on the Areas page");
        rail(&mut h, "Advanced");
        let dock = right_dock_rect(&h);
        let reached = tabbed_in_sight(&mut h, dock, 400);
        assert!(reached >= 6, "{scale}%: Tab reached {reached} on Advanced");
    }
}

/// F6 stops at an open tool section as at the dock's other sections, its
/// keys given back to it (the manual's 02, "Narrow and wide windows": M4's
/// U29 review), at 1600 where it is a section of its own and at 1280 where
/// it is a tab of the one tab bar.
#[test]
fn f6_stops_at_an_open_tool_section() {
    for size in [egui::vec2(1600.0, 900.0), egui::vec2(1280.0, 720.0)] {
        let mut h = together_harness("u29f-f6", 6, size.x, size.y);
        h.state_mut().settings.hide_palettes = false;
        let area = ResRef::from_str("area0000").unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        settle(&mut h);
        open_together(&mut h);
        // (The keys taken from it by a press on the view.)
        click_pos(&h, egui::pos2(400.0, size.y / 2.0));
        settle(&mut h);
        assert_ne!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "AreasTogether");
        let mut stops = Vec::new();
        for _ in 0..10 {
            h.key_press(egui::Key::F6);
            settle(&mut h);
            stops.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
            if stops.last().is_some_and(|s| s == "AreasTogether") {
                break;
            }
        }
        assert_eq!(stops.last().map(String::as_str), Some("AreasTogether"), "{size:?}: {stops:?}");
    }
}
