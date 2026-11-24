//! Menus that hold more than rows: headings with a line between their
//! groups (an area's Add Grass to…), opened as a builder opens them, their
//! parts found by name, apart from one another, and working. (The NUI
//! designer's Properties, a menu in its narrow layout until M4's U18, are
//! a panel since: `nui_panels.rs`.)

use crate::common::*;

/// An area's Add Grass to…: a heading over each group of materials (the
/// second wrapped), a line between the groups, the rows 24 high under
/// their heading with no gap and none touching; a row chosen adds the
/// area's own grass.
#[test]
fn add_grass_to_has_headings_over_its_rows() {
    use mg_area::grass::AreaGrass;
    let Some((mut h, area)) = area_ui_harness("menus-grass") else { return };
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
