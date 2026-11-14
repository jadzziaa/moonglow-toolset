//! Menus that hold more than rows: a form (the NUI designer's Properties
//! in a narrow window) and headings with a line between their groups
//! (an area's Add Grass to…), opened as a builder opens them, their parts
//! found by name, apart from one another, and working.

use crate::common::*;

/// The NUI designer narrower than 760 px: its Properties, a menu that
/// holds the whole inspector, is a form whose labels and fields keep
/// their gaps (they ran together at the menu rows' 24 px with no gap),
/// and a field in it still edits the element.
#[test]
fn the_nui_designer_s_properties_menu_is_a_form_whose_parts_keep_apart() {
    let key = ResKey::parse("menus", ResType::JUI).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
    app.run(mg_ui::Action::NewNui("menus".into()));
    let mut window = mg_nui::window();
    window["root"]["children"] =
        serde_json::json!([{"type": "button", "id": "ok", "label": "Agree"}]);
    app.ws.as_mut().unwrap().module.set(key, serde_json::to_vec_pretty(&window).unwrap());
    app.settings.window_sizes.retain(|(kind, _)| kind != "nui");
    app.settings.window_sizes.push(("nui".into(), [700, 800]));
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 950.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), crate::common::docks_as_set(app));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key)));
    settle(&mut h);
    h.get_by_label_contains("Canvas Button").click();
    settle(&mut h);
    // (The menu's button, beside Add element and Layers: the main
    // toolbar's Properties is above it.)
    let lowest = |h: &Harness<'_, Moonglow>| {
        let all = h.get_all_by_label("Properties");
        all.max_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).unwrap().rect()
    };
    let button = lowest(&h);
    // (What the window shows before the menu opens: the menu's parts are
    // the rest.)
    let parts = |h: &Harness<'_, Moonglow>| -> Vec<(String, egui::Rect)> {
        use egui::accesskit::Role;
        let kinds = [Role::Button, Role::TextInput, Role::ComboBox, Role::CheckBox, Role::Label];
        h.get_all_by(|n| kinds.contains(&n.role()))
            .filter_map(|n| Some((n.accesskit_node().label()?, n.rect())))
            .collect()
    };
    let before = parts(&h);
    h.get_all_by_label("Properties").find(|n| n.rect() == button).unwrap().click();
    settle(&mut h);
    let menu: Vec<(String, egui::Rect)> =
        parts(&h).into_iter().filter(|p| !before.contains(p)).collect();
    // Its fields (named by their labels) and buttons: none overlapping
    // another, and one above another a gap apart (the rows' 24 with no
    // gap ran them together).
    let id_field = h.get_by_label("Element ID").rect();
    let label_field = menu.iter().find(|(n, _)| n == "Label").expect("Label's field").1;
    assert!(label_field.top() - id_field.bottom() >= 4.0, "{id_field:?} {label_field:?}");
    for (i, (a, ra)) in menu.iter().enumerate() {
        for (b, rb) in &menu[i + 1..] {
            assert!(!ra.shrink(0.5).intersects(rb.shrink(0.5)), "{a} {ra:?} over {b} {rb:?}");
            let stacked = ra.x_range().intersects(rb.x_range());
            let gap = (rb.top() - ra.bottom()).max(ra.top() - rb.bottom());
            assert!(!stacked || gap >= 4.0 || gap < -1.0, "{a} {ra:?} and {b} {rb:?} touch");
        }
    }
    assert!(menu.len() >= 4, "{menu:?}");
    // Inside the menu, not at its edge: the form's 10 px.
    assert!(id_field.left() >= button.left() + 10.0, "{id_field:?} {button:?}");
    // Working: the label typed is the button's.
    let value = |h: &Harness<'_, Moonglow>| {
        h.get_all_by_role(egui::accesskit::Role::TextInput)
            .filter(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
            .find(|n| n.rect() == label_field)
            .and_then(|n| n.accesskit_node().value())
    };
    assert_eq!(value(&h).as_deref(), Some("Agree"));
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .filter(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .find(|n| n.rect() == label_field)
        .unwrap()
        .click();
    settle(&mut h);
    h.key_press(egui::Key::End);
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .filter(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .find(|n| n.rect() == label_field)
        .unwrap()
        .type_text("d");
    settle(&mut h);
    // (Taken with Enter, as the inspector's fields take a value; the menu
    // still open for the next.)
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.query_by_label("Element ID").is_some(), "the menu stays open");
    let ws = h.state_mut().ws.as_mut().unwrap();
    let doc = mg_nui::parse(ws.module.get(&key).unwrap()).unwrap();
    assert_eq!(doc["root"]["children"][0]["label"], "Agreed", "{doc}");
    // A submenu in it (+ Add property): its row chosen adds the property
    // and closes the submenu, as a chosen row does.
    h.get_by_label("+ Add property").hover();
    settle(&mut h);
    h.get_by_label("+ Add property").click();
    settle(&mut h);
    h.get_by_label("Tooltip").click();
    settle(&mut h);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let doc = mg_nui::parse(ws.module.get(&key).unwrap()).unwrap();
    assert!(doc["root"]["children"][0].get("tooltip").is_some(), "{doc}");
    // (And the menu with it, as before: a row chosen is an action done.)
    assert!(h.query_by_label("Margin").is_none(), "the submenu closed");
    assert!(h.query_by_label("Element ID").is_none(), "the menu closed with it");
}

/// An area's Add Grass to…: a heading over each group of materials (the
/// second wrapped), a line between the groups, the rows 24 high under
/// their heading with no gap and none touching; a row chosen adds the
/// area's own grass.
#[test]
fn add_grass_to_has_headings_over_its_rows() {
    use mg_area::grass::AreaGrass;
    let Some((mut h, area)) = area_harness("menus-grass") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    h.run_steps(5);
    // (The window's page: the palette has a Grass terrain too.)
    h.get_all_by_label("Grass").last().unwrap().click();
    h.run_steps(3);
    h.get_by_label("Add Grass to…").click();
    h.run_steps(3);
    let rect = |h: &Harness<'_, Moonglow>, label: &str| h.get_by_label(label).rect();
    let here = rect(&h, "On this area's tiles");
    let elsewhere = rect(&h, "Not on this area's tiles (nothing grows until a tile has it)");
    let (grass, stone) = (rect(&h, "Grass (3)"), rect(&h, "Stone (4)"));
    let last_here = rect(&h, "Nonwalk (7)");
    let dirt = rect(&h, "NotDefined (0)");
    // The heading's own padding (.pop-h: 6 above, 2 below its line), the
    // first row right under it.
    assert_eq!(grass.top(), here.bottom(), "{here:?} {grass:?}");
    assert!(here.height() >= 6.0 + 2.0 + 14.0, "{here:?}");
    assert_eq!(grass.height(), mg_uikit::metrics::MENU_ROW);
    assert_eq!(stone.top(), grass.bottom(), "rows with no gap");
    // The line between the groups: 9 (4, the line, 4), then the heading.
    assert_eq!(elsewhere.top() - last_here.bottom(), mg_uikit::menu::MENU_SEPARATOR);
    assert!(elsewhere.height() > here.height(), "the second heading wraps: {elsewhere:?}");
    assert_eq!(dirt.top(), elsewhere.bottom());
    for (a, b) in [(here, grass), (last_here, elsewhere), (elsewhere, dirt)] {
        assert!(!a.intersects(b) || a.bottom() == b.top(), "{a:?} {b:?}");
    }
    // Headings 10 px in, rows' words 8: neither at the popover's edge.
    assert!(here.left() <= grass.left() + 1.0, "{here:?} {grass:?}");
    // Working: Dirt chosen, the area grows its own grass on dirt.
    h.get_by_label("Dirt (1)").click();
    h.run_steps(3);
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let grown = AreaGrass::read(&ws.doc(&git).unwrap().root);
    assert!(grown.overrides.iter().any(|o| o.material == 1), "{grown:?}");
}
