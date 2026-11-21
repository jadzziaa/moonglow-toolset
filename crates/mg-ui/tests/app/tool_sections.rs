//! The Right Dock's tool sections (M4's U29): Edit Areas Together, Custom
//! Light and Bake Walkmesh to Transform, once dialogs. The command that
//! opens one shows the dock (hidden or folded by the window's width) with
//! the section in front, and gives it the keys, held over what is under the
//! pointer until a press elsewhere; its actions are at its top, as the
//! Properties sections' (Enter its main one, Escape its Cancel or Close),
//! and none of its keys reach it under a menu, a popover or a dialog, or
//! while a field of it is typed in. Edit Areas Together's own keys move in
//! its checklist and tick; over thousands of areas nothing walks them every
//! frame.

use crate::common::*;
use egui::Key;

/// A module of `count` areas, `area0000` on (named "Area 0" on), their
/// flags by threes: none, interior, interior and underground.
pub(crate) fn areas_module(dir: &std::path::Path, count: usize) -> std::path::PathBuf {
    use mg_gff::{Struct, Value};
    let path = dir.join("areas.mod");
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    let mut list = Vec::new();
    for n in 0..count {
        let name = format!("area{n:04}");
        let r = ResRef::from_str(&name).unwrap();
        let mut item = Struct::new(6);
        item.set("Area_Name", Value::resref(r));
        list.push(item);
        let mut are = Gff::new(*b"ARE ");
        let title = LocString::from_text(Language::ENGLISH, Gender::Male, format!("Area {n}"));
        are.root.set("Name", Value::LocString(title));
        are.root.set("ResRef", Value::resref(r));
        are.root.set("Tag", Value::String(name.to_uppercase().into_bytes()));
        are.root.set("Flags", Value::Dword([0, 1, 3][n % 3]));
        are.root.set("DayNightCycle", Value::Byte(1));
        are.root.set("IsNight", Value::Byte(0));
        m.set(ResKey::new(r, ResType::ARE), are.to_bytes().unwrap());
        let mut git = Gff::new(*b"GIT ");
        git.root.set("AreaProperties", Value::Struct(Struct::new(100)));
        m.set(ResKey::new(r, ResType::GIT), git.to_bytes().unwrap());
    }
    info.root.set("Mod_Area_list", Value::List(list));
    m.set_info(&info).unwrap();
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// The window on a module of `count` areas, `w` by `h`, the Right Dock
/// hidden (View › Right Dock off).
pub(crate) fn together_harness(
    name: &str,
    count: usize,
    w: f32,
    h: f32,
) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(name);
    let path = areas_module(&dir, count);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, w, h);
    settle(&mut h);
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    assert!(h.state().right_dock_shows().is_none(), "the Right Dock hidden");
    h
}

/// Edit › Edit Areas Together….
pub(crate) fn open_together(h: &mut Harness<'_, Moonglow>) {
    menu_command(h, "Edit", "Edit Areas Together…");
    settle(h);
}

/// Whether the Right Dock's tab `name` is in front.
pub(crate) fn in_front(h: &Harness<'_, Moonglow>, name: &str) -> bool {
    h.get_all_by_role(egui::accesskit::Role::Tab)
        .map(|n| n.accesskit_node())
        .any(|n| n.label().as_deref() == Some(name) && n.is_selected() == Some(true))
}

/// The areas ticked, by name.
pub(crate) fn ticked(h: &Harness<'_, Moonglow>) -> Vec<String> {
    let (ticked, _) = h.state().areas_together_ticked().expect("Edit Areas Together open");
    ticked.iter().map(ResRef::to_string).collect()
}

/// The section's page `page` chosen on its rail (not a page of the same
/// name another form shows).
pub(crate) fn rail(h: &mut Harness<'_, Moonglow>, page: &str) {
    let dock = right_dock_rect(h);
    let at = (h.get_all_by_label(page).map(|n| n.rect()))
        .find(|r| dock.contains_rect(*r))
        .unwrap_or_else(|| panic!("no page {page} on the section's rail"));
    click_pos(h, at.center());
    settle(h);
}

/// A key pressed and its frames run.
pub(crate) fn key(h: &mut Harness<'_, Moonglow>, k: Key) {
    h.key_press(k);
    settle(h);
}

/// Edit Areas Together's command shows the Right Dock where it was hidden,
/// and where the window's width folds it (the least window), with the
/// section in front and its controls in sight; Close puts the dock's front
/// back as it was.
#[test]
fn edit_areas_together_shows_the_right_dock_with_its_section() {
    for (name, size) in [
        ("u29-together-hidden", egui::vec2(1280.0, 720.0)),
        ("u29-together-least", mg_ui::LEAST_WINDOW_SIZE),
    ] {
        let mut h = together_harness(name, 4, size.x, size.y);
        open_together(&mut h);
        assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::AreasTogether), "{name}");
        assert!(in_front(&h, "Edit Areas Together"), "{name}: its tab in front");
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for control in ["Open in Tab", "Close", "Tick Shown", "area0000 (Area 0)"] {
            // (At the least window the page scrolls to them.)
            h.get_by_label(control).scroll_to_me();
            settle(&mut h);
            let r = h.get_by_label(control).rect();
            assert!(screen.contains_rect(r), "{name}: {control} at {r:?}, in {screen:?}");
        }
        h.get_by_label("No areas ticked");
        h.get_by_label("Close").click();
        settle(&mut h);
        assert!(h.state().areas_together_ticked().is_none(), "{name}: closed");
        assert!(h.query_by_label("Edit Areas Together").is_none(), "{name}: its tab gone");
    }
    // Shown beside the Properties: Close brings them to the front again.
    let mut h = together_harness("u29-together-front", 4, 1280.0, 720.0);
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Properties));
    settle(&mut h);
    let before = h.state().right_dock_shows();
    open_together(&mut h);
    assert!(in_front(&h, "Edit Areas Together"));
    h.get_by_label("Close").click();
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), before, "the front as it was");
}

/// The keys the command gives: Down and Up move the row chosen, Space
/// ticks it, Home and End go to the first and last; Enter opens the areas
/// ticked in a tab (two or more), Escape closes the section.
#[test]
fn edit_areas_together_s_keys_move_tick_open_and_close() {
    let mut h = together_harness("u29-together-keys", 4, 1280.0, 720.0);
    open_together(&mut h);
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::Space);
    assert_eq!(ticked(&h), ["area0000"], "the first row, ticked");
    h.get_by_label("1 area ticked");
    key(&mut h, Key::End);
    key(&mut h, Key::Space);
    assert_eq!(ticked(&h), ["area0000", "area0003"], "the last");
    key(&mut h, Key::ArrowUp);
    key(&mut h, Key::Space);
    key(&mut h, Key::Home);
    key(&mut h, Key::Space);
    assert_eq!(ticked(&h), ["area0002", "area0003"], "the third ticked, the first unticked");
    key(&mut h, Key::Enter);
    let two: Vec<ResRef> = ["area0002", "area0003"].map(|n| ResRef::from_str(n).unwrap()).to_vec();
    assert!(h.state().dock.find_tab(&Tab::AreasProperties(two)).is_some(), "Enter: Open in Tab");
    assert!(h.state().areas_together_ticked().is_none(), "and the section closed");

    open_together(&mut h);
    key(&mut h, Key::Escape);
    assert!(h.state().areas_together_ticked().is_none(), "Escape closes it");
}

/// None of the section's keys reach it under a menu, a popover or a dialog,
/// or while a field of it has the keyboard (Escape leaves the field, the
/// section stays); a press in it gives them back; a press on a pane that
/// takes the keys (the Log) takes them from it.
#[test]
fn edit_areas_together_s_keys_wait_under_menus_popovers_dialogs_and_fields() {
    let mut h = together_harness("u29-together-guards", 4, 1280.0, 720.0);
    open_together(&mut h);
    key(&mut h, Key::ArrowDown);
    // A menu.
    menu(&h, "View").click();
    settle(&mut h);
    key(&mut h, Key::Space);
    key(&mut h, Key::Escape);
    assert_eq!(ticked(&h), Vec::<String>::new(), "under a menu: nothing ticked");
    // A popover: the Tileset box's list (Space its own: it takes the
    // choice, and the list closes).
    h.get_by_value("Any").click();
    settle(&mut h);
    key(&mut h, Key::Space);
    assert_eq!(ticked(&h), Vec::<String>::new(), "under a popover: nothing ticked");
    h.get_by_value("Any").click();
    settle(&mut h);
    key(&mut h, Key::Escape);
    assert!(h.state().areas_together_ticked().is_some(), "Escape closed the list alone");
    // A dialog.
    h.state_mut().about = true;
    settle(&mut h);
    key(&mut h, Key::Space);
    key(&mut h, Key::Enter);
    assert!(h.state().areas_together_ticked().is_some(), "not closed under the dialog");
    key(&mut h, Key::Escape);
    assert!(!h.state().about, "Escape was the dialog's");
    assert!(h.state().areas_together_ticked().is_some(), "and not the section's");
    assert_eq!(ticked(&h), Vec::<String>::new(), "under a dialog: nothing ticked");
    // A field: Space is typed, Escape leaves the field.
    h.get(egui_kittest::kittest::by().predicate(|n| n.placeholder() == Some("Name or ResRef")))
        .click();
    settle(&mut h);
    key(&mut h, Key::Space);
    assert_eq!(ticked(&h), Vec::<String>::new(), "in a field: nothing ticked");
    key(&mut h, Key::Escape);
    assert!(h.state().areas_together_ticked().is_some(), "Escape left the field alone");
    // A press in the section gives them back; one elsewhere takes them.
    h.get_by_label_contains("Tick the areas to edit").click();
    settle(&mut h);
    key(&mut h, Key::Space);
    assert_eq!(ticked(&h).len(), 1, "a press in it: its keys again");
    // (The Log, which a click gives the keys.)
    let log = h.get_by_label_contains("Opened ").rect().center();
    click_pos(&h, log);
    settle(&mut h);
    assert!(mg_ui::palette_view::keys_are_the_logs(&h.ctx), "the Log's keys");
    key(&mut h, Key::Space);
    assert_eq!(ticked(&h).len(), 1, "a press elsewhere: not its keys");
}

/// The shared pages in the section are Area Properties' own: a change is set
/// on every area ticked, one step to undo; fields they differ in are marked
/// as the Properties' several of one type ("≠"); with Area Properties of one
/// of them open in a tab, the page says so and Show It brings that tab.
#[test]
fn the_ticked_areas_are_edited_together_in_the_section() {
    let mut h = together_harness("u29-together-pages", 4, 1280.0, 720.0);
    open_together(&mut h);
    for _ in 0..2 {
        key(&mut h, Key::ArrowDown);
        key(&mut h, Key::Space);
    }
    assert_eq!(ticked(&h), ["area0000", "area0001"]);
    rail(&mut h, "Visual");
    h.get_by_label_contains("Shown as the first, area0000;");
    // (Every control of the page whole across the dock: the day and night
    // choices wrap.)
    let dock = right_dock_rect(&h);
    for choice in ["Cycle Day and Night", "Always Bright", "Always Dark"] {
        let r = h.get_by_label(choice).rect();
        assert!(r.right() <= dock.right() + 0.5, "{choice} cut at the dock's edge: {r:?}");
    }
    click_in_sight(&mut h, "Always Dark");
    settle(&mut h);
    let night = |h: &mut Harness<'_, Moonglow>, n: &str| {
        let key = ResKey::parse(n, ResType::ARE).unwrap();
        h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().root.integer("IsNight")
    };
    assert_eq!(
        ["area0000", "area0001", "area0002"].map(|n| night(&mut h, n)),
        [Some(1), Some(1), Some(0)]
    );
    // Their flags differ (none, interior): marked.
    rail(&mut h, "Advanced");
    assert!(h.query_all_by_value("mixed").count() >= 1, "the flags they differ in, marked");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert_eq!(night(&mut h, "area0001"), Some(0), "one undo takes it from both");
    assert_eq!(night(&mut h, "area0000"), Some(0));
    // Their Area Properties in a tab: Show It.
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    let one = Tab::AreaProperties(ResRef::from_str("area0001").unwrap());
    h.state_mut().actions.push(mg_ui::Action::OpenTab(one.clone()));
    settle(&mut h);
    // (Behind another tab.)
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Plugins));
    settle(&mut h);
    let front = |h: &Harness<'_, Moonglow>| {
        let s = h.state();
        let leaf = s.dock.iter_leaves().find(|(_, l)| l.tabs.contains(&one)).unwrap().1;
        leaf.tabs[leaf.active.0] == one
    };
    assert!(!front(&h));
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::AreasTogether));
    settle(&mut h);
    h.get_by_label("Area Properties of these areas are open in a tab or window of their own.");
    let dock = right_dock_rect(&h);
    let fields = h
        .query_all_by_role(egui::accesskit::Role::CheckBox)
        .filter(|n| dock.contains_rect(n.rect()));
    assert_eq!(fields.count(), 0, "no fields twice");
    h.get_by_label("Show It").click();
    settle(&mut h);
    assert!(front(&h), "Show It brings it");
}

/// Over 3,000 areas: the list, the rows shown and the ticked areas' set are
/// worked out when the module or the ticks change, never every frame, and
/// no walk of the areas is made in a frame that changes nothing, on every
/// shared page; End, Page Up and Home bring the row chosen into the list's
/// sight; at rest, no frame asks for another.
#[test]
fn three_thousand_areas_are_walked_only_when_they_change() {
    let mut h = together_harness("u29-together-3000", 3000, 1280.0, 720.0);
    open_together(&mut h);
    let room = right_dock_rect(&h);
    for (k, row) in [(Key::End, "area2999"), (Key::PageUp, ""), (Key::Home, "area0000")] {
        key(&mut h, Key::ArrowDown);
        key(&mut h, k);
        let (_, at) = h.state().areas_together_ticked().unwrap();
        let at = at.expect("a row chosen").to_string();
        let shown = h
            .query_all_by_label_contains(&format!("{at} ("))
            .map(|n| n.rect())
            .find(|r| room.contains_rect(*r));
        assert!(shown.is_some(), "{k:?}: {at} in sight");
        if !row.is_empty() {
            assert!(row.contains(&at), "{k:?}: {at}");
        }
    }
    h.get_by_label("Tick Shown").click();
    settle(&mut h);
    h.get_by_label("3000 areas ticked");
    for page in ["Areas", "Visual", "Grass", "Audio", "Events", "Advanced", "Variables"] {
        rail(&mut h, page);
        let before = h.state().areas_together_counts();
        for _ in 0..5 {
            h.try_run().unwrap_or_else(|e| panic!("{page}: at rest, a frame asks another: {e}"));
        }
        assert_eq!(h.state().areas_together_counts(), before, "{page}: nothing walked again");
    }
    let (shown, set, _) = h.state().areas_together_counts();
    assert!(shown <= 3 && set <= 2, "worked out once each: {shown}, {set}");
}

/// At the Right Dock's least width, nothing of any page of the section
/// runs past the dock's edge: a page goes one field under the other, each
/// under its label, where its form's grid of two columns has no room
/// (Visual's weather, Audio, Events, Advanced), the Areas page's filters
/// stay a grid with their choices' words cut to the room, and the
/// Variables page keeps its note of several in sight.
#[test]
fn every_page_fits_the_least_right_dock() {
    let mut h = together_harness("u29-together-fits", 4, 1600.0, 900.0);
    open_together(&mut h);
    for _ in 0..2 {
        key(&mut h, Key::ArrowDown);
        key(&mut h, Key::Space);
    }
    h.state_mut().settings.right_dock_width = Some(0);
    settle(&mut h);
    let dock = right_dock_rect(&h);
    for page in ["Areas", "Visual", "Grass", "Audio", "Events", "Advanced", "Variables"] {
        rail(&mut h, page);
        // (What is drawn in the dock and in sight: its left in it, its top
        // above the dock's foot.)
        let past: Vec<String> = h
            .query_all(egui_kittest::kittest::by())
            .filter(|n| n.accesskit_node().bounding_box().is_some())
            .map(|n| (n.accesskit_node().label().unwrap_or_default().to_string(), n.rect()))
            .filter(|(_, r)| r.is_positive() && dock.x_range().contains(r.left()))
            .filter(|(_, r)| r.top() > dock.top() && r.top() < dock.bottom())
            .filter(|(_, r)| r.right() > dock.right() + 0.5)
            .map(|(l, r)| format!("{l:?} to {}", r.right()))
            .collect();
        assert!(past.is_empty(), "{page}: past the dock's edge at {}: {past:?}", dock.right());
    }
    // The note under the Variables list of several, in sight.
    let note = h.get_by_label_contains("edited together, shown as the first").rect();
    assert!(note.bottom() <= dock.bottom(), "the note at {note:?}, the dock to {}", dock.bottom());
}

/// Tab pressed round the whole window, until the focus comes back to the
/// first control it reached (at most `most` presses): each control it
/// reaches in `room` (the section's dock, the tab's body) is shown whole,
/// neither cut by a scroll nor by the panel it is in (its rect inside the
/// clip it is drawn in) nor out of the window. How many it reached there.
pub(crate) fn tabbed_in_sight(
    h: &mut Harness<'_, Moonglow>,
    room: egui::Rect,
    most: usize,
) -> usize {
    let screen = h.ctx.content_rect();
    let (mut inside, mut first) = (0, None);
    for i in 0..most {
        h.key_press(Key::Tab);
        settle(h);
        let Some(id) = h.ctx.memory(|m| m.focused()) else { continue };
        if first == Some(id) {
            return inside;
        }
        first.get_or_insert(id);
        let Some(r) = h.ctx.read_response(id) else { continue };
        // In the room: its clip there, which a scroll does not move (a
        // control left below a scroll's foot is clipped at the foot, one
        // above at its top); not an editor tab's title under the dock,
        // whose clip ends left of it, nor the Bottom Dock under the tab.
        let seen = r.interact_rect;
        let clip_in = |a: f32, b: f32, lo: f32, hi: f32| (a >= lo && a < hi) || (b > lo && b <= hi);
        if seen.max.x <= room.left()
            || seen.min.x >= room.right()
            || !clip_in(seen.min.y, seen.max.y, room.top(), room.bottom())
        {
            continue;
        }
        let whole = r.rect.shrink(1.0);
        assert!(
            seen.contains_rect(whole) && screen.contains_rect(whole),
            "Tab {i}: a control at {:?} shown only at {seen:?}",
            r.rect
        );
        inside += 1;
    }
    panic!("Tab did not come back to the first control in {most} presses");
}

/// At the least window, each control Tab reaches in Edit Areas Together
/// (its Areas, Visual and Advanced pages) is brought into sight whole.
#[test]
fn tab_brings_each_control_of_the_section_into_sight_at_the_least_window() {
    let least = mg_ui::LEAST_WINDOW_SIZE;
    for (page, least_reached) in [("Areas", 10), ("Visual", 20), ("Advanced", 20)] {
        let mut h = together_harness("u29-together-tab", 4, least.x, least.y);
        open_together(&mut h);
        for _ in 0..2 {
            key(&mut h, Key::ArrowDown);
            key(&mut h, Key::Space);
        }
        rail(&mut h, page);
        let dock = right_dock_rect(&h);
        let reached = tabbed_in_sight(&mut h, dock, 400);
        assert!(
            reached >= least_reached,
            "{page}: Tab reached {reached} of the section's controls"
        );
    }
}

/// At the least window, each control Tab reaches in the Plugins tab is
/// brought into sight whole.
#[test]
fn tab_brings_each_control_of_the_plugins_tab_into_sight_at_the_least_window() {
    let least = mg_ui::LEAST_WINDOW_SIZE;
    let mut h = together_harness("u29-plugins-tab", 4, least.x, least.y);
    h.state_mut().plugin_dir = Some(plugin_fixtures());
    h.state_mut().load_plugins();
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    menu_command(&mut h, "Plugins", "Manage Plugins…");
    let tab = h.state().tab_rect(&Tab::Plugins).expect("the Plugins tab");
    let editors = h.state().editor_area_rect().expect("the editor area");
    let body = egui::Rect::from_min_max(egui::pos2(editors.left(), tab.bottom()), editors.max);
    let reached = tabbed_in_sight(&mut h, body, 400);
    // (Its time, three buttons, each fixture's check box and Console.)
    assert!(reached >= 10, "Plugins: Tab reached {reached} of its controls");
}

/// Where M4's U29 pictures go: `MG_U29_PICS`, else the test output's
/// `u29-pics`.
fn pics_dir() -> std::path::PathBuf {
    std::env::var_os("MG_U29_PICS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| mg_testkit::test_output().join("u29-pics"))
}

/// The sizes pictured: the boards' two, the narrow layout (900 by 700, as
/// U28's) and the least window.
pub(crate) const PICTURE_SIZES: [egui::Vec2; 4] = [
    egui::vec2(1600.0, 900.0),
    egui::vec2(1280.0, 720.0),
    egui::vec2(900.0, 700.0),
    mg_ui::LEAST_WINDOW_SIZE,
];

/// `h` at rest, drawn into `<pics>/<theme>/<w>x<h>/<name>.png` (M4's U29).
pub(crate) fn picture(h: &mut Harness<'_, Moonglow>, name: &str) {
    let theme = if h.state().settings.light_theme { "light" } else { "dark" };
    let size = h.ctx.content_rect().size();
    let dir = pics_dir().join(theme).join(format!("{}x{}", size.x, size.y));
    std::fs::create_dir_all(&dir).unwrap();
    settle(h);
    h.run_steps(3);
    h.render().expect("drawn").save(dir.join(format!("{name}.png"))).unwrap();
}

/// [`picture`], once `shown` says that what the picture is named for is
/// on screen (M4's U29 review: three pictures' names said what they did
/// not show).
#[track_caller]
pub(crate) fn picture_of(
    h: &mut Harness<'_, Moonglow>,
    name: &str,
    what: &str,
    shown: impl Fn(&Harness<'_, Moonglow>) -> bool,
) {
    settle(h);
    h.run_steps(3);
    let size = h.ctx.content_rect().size();
    if !shown(h) {
        // (Drawn under another name, to see why.)
        picture(h, &format!("{name}-NOT-SHOWN"));
        panic!("{name} at {size:?}: {what} not on screen");
    }
    picture(h, name);
}

/// The wheel turned down at `at` until `shown` says so (30 turns at most),
/// the pointer then moved off to the window's corner (no tip in the
/// picture).
pub(crate) fn wheel_until(
    h: &mut Harness<'_, Moonglow>,
    at: egui::Pos2,
    shown: impl Fn(&Harness<'_, Moonglow>) -> bool,
) {
    for _ in 0..30 {
        if shown(h) {
            break;
        }
        h.hover_at(at);
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -40.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        settle(h);
    }
    h.hover_at(egui::pos2(2.0, 2.0));
    settle(h);
}

/// Whether a node `pick` chooses lies whole in `room` (and the window).
pub(crate) fn shown_in(
    h: &Harness<'_, Moonglow>,
    room: egui::Rect,
    pick: impl Fn(&egui_kittest::Node<'_>) -> bool,
) -> bool {
    let room = room.intersect(h.ctx.content_rect());
    h.query_all_by(|n| n.bounding_box().is_some()).any(|n| pick(&n) && room.contains_rect(n.rect()))
}

/// A node's words: its label, or a label's value (egui's labels name
/// themselves by their value, as kittest's label queries read them).
pub(crate) fn words(n: &egui_kittest::Node<'_>) -> Option<String> {
    let a = n.accesskit_node();
    a.label().or_else(|| a.value())
}

/// Whether a node labeled `label` lies whole in `room`.
pub(crate) fn label_in(h: &Harness<'_, Moonglow>, room: egui::Rect, label: &str) -> bool {
    shown_in(h, room, |n| words(n).as_deref() == Some(label))
}

/// Brings the first node `pick` chooses into sight (its scroll's).
pub(crate) fn scroll_to(
    h: &mut Harness<'_, Moonglow>,
    pick: impl Fn(&egui_kittest::Node<'_>) -> bool,
) {
    if let Some(n) = h.query_all_by(|n| n.bounding_box().is_some()).find(|n| pick(n)) {
        n.scroll_to_me();
    }
    settle(h);
}

/// Whether words with `mark` in them lie whole in the Right Dock.
fn page_shows(h: &Harness<'_, Moonglow>, mark: &str) -> bool {
    shown_in(h, right_dock_rect(h), |n| words(n).is_some_and(|l| l.contains(mark)))
}

/// Whether words with `mark` in them lie whole in the window.
fn on_screen(h: &Harness<'_, Moonglow>, mark: &str) -> bool {
    shown_in(h, h.ctx.content_rect(), |n| words(n).is_some_and(|l| l.contains(mark)))
}

/// Whether words with `mark` in them lie whole in the editor area (above
/// the Bottom Dock, between the docks).
fn in_editor(h: &Harness<'_, Moonglow>, mark: &str) -> bool {
    let room = h.state().editor_area_rect().expect("the editor area drawn");
    shown_in(h, room, |n| words(n).is_some_and(|l| l.contains(mark)))
}

/// Whether `n` is one of Edit Areas Together's area rows, ticked.
fn ticked_row(n: &egui_kittest::Node<'_>) -> bool {
    let a = n.accesskit_node();
    words(n).is_some_and(|l| l.contains("(Area ") || l.contains("(area"))
        && a.toggled() == Some(egui::accesskit::Toggled::True)
}

/// Whether `n` is a ≠ mark (its value "mixed").
fn mixed(n: &egui_kittest::Node<'_>) -> bool {
    n.accesskit_node().value().as_deref() == Some("mixed")
}

/// Edit Areas Together pictured: empty, three ticked, a shared page, a
/// page whose values differ, in both themes at the boards' sizes and the
/// least window, and at 1600 with the Right Dock at its least and a wide
/// width. By hand:
/// `MG_U29_PICS=<folder> cargo test -p mg-ui --test app u29_pictures -- --ignored`.
#[test]
#[ignore]
fn u29_pictures_edit_areas_together() {
    mg_testkit::gpu::hold();
    for light in [false, true] {
        for size in PICTURE_SIZES {
            let mut h = together_harness("u29-pics-together", 12, size.x, size.y);
            h.state_mut().settings.light_theme = light;
            settle(&mut h);
            open_together(&mut h);
            picture_of(&mut h, "together-empty", "its intro", |h| {
                label_in(h, right_dock_rect(h), "Tick the areas to edit together.")
            });
            for _ in 0..3 {
                key(&mut h, Key::ArrowDown);
                key(&mut h, Key::Space);
            }
            // (Its list in sight: at the least window it is under the
            // filters, the section's scroll moved to it.)
            let filled = |h: &Harness<'_, Moonglow>| {
                h.state().areas_together_ticked().is_some_and(|(t, _)| t.len() == 3)
                    && shown_in(h, right_dock_rect(h), ticked_row)
            };
            // (Over the filters: the section's page scrolls, not the list.)
            let over = right_dock_rect(&h).center_top() + egui::vec2(20.0, 110.0);
            wheel_until(&mut h, over, filled);
            picture_of(&mut h, "together-filled", "a ticked row", filled);
            rail(&mut h, "Visual");
            let dark = |h: &Harness<'_, Moonglow>| {
                shown_in(h, right_dock_rect(h), |n| {
                    words(n).is_some_and(|l| l.contains("Always Dark"))
                })
            };
            // (Over the page, right of the rail.)
            let page = right_dock_rect(&h).center() + egui::vec2(20.0, 0.0);
            wheel_until(&mut h, page, dark);
            picture_of(&mut h, "together-visual", "the Visual page's Always Dark", dark);
            rail(&mut h, "Advanced");
            scroll_to(&mut h, mixed);
            let marked = |h: &Harness<'_, Moonglow>| shown_in(h, right_dock_rect(h), mixed);
            wheel_until(&mut h, page, marked);
            picture_of(&mut h, "together-mixed", "a ≠ mark", marked);
            if size != PICTURE_SIZES[0] {
                continue;
            }
            for (name, width) in [("least", 0), ("wide", 640)] {
                h.state_mut().settings.right_dock_width = Some(width);
                // (Laid out at that width before the rail is looked for.)
                settle(&mut h);
                for (page, mark) in [
                    ("Areas", "Tick the areas"),
                    ("Visual", "Fog Amount"),
                    ("Grass", "grass"),
                    ("Audio", "Ambient Sound"),
                    ("Events", "OnEnter"),
                    ("Advanced", "Check Modifier"),
                    ("Variables", "Add Set"),
                ] {
                    rail(&mut h, page);
                    let what = format!("the {page} page's {mark}");
                    let file = format!("together-dock-{name}-{}", page.to_lowercase());
                    picture_of(&mut h, &file, &what, |h| page_shows(h, mark));
                }
            }
        }
    }
}

/// `from` copied into `to`, folders and all.
fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (f, t) = (e.path(), to.join(e.file_name()));
        if f.is_dir() { copy_tree(&f, &t) } else { std::fs::copy(&f, &t).map(drop).unwrap() }
    }
}

/// A copy of the plugin fixtures `names` in a scratch folder of their own.
fn plugins_of(name: &str, names: &[&str]) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    for n in names {
        copy_tree(&plugin_fixtures().join(n), &dir.join(n));
    }
    dir
}

/// An enabled plugin's buttons (Remove…, Settings…, Run Tests) wrap in a
/// narrow Plugins tab, none past its edge (M4's U29 review: at the least
/// window Settings… was cut and Run Tests out of reach).
#[test]
fn a_plugin_s_buttons_wrap_in_a_narrow_plugins_tab() {
    let dir = mg_testkit::scratch_dir("ui-plugins-narrow");
    let examples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples");
    copy_tree(&examples.join("builder-tags"), &dir.join("builder-tags"));
    let size = mg_ui::LEAST_WINDOW_SIZE;
    let mut h = together_harness("ui-plugins-narrow-module", 4, size.x, size.y);
    h.state_mut().plugin_dir = Some(dir.clone());
    h.state_mut().load_plugins();
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    h.state_mut().enable_plugin("example.builder-tags", true);
    settle(&mut h);
    menu_command(&mut h, "Plugins", "Manage Plugins…");
    let screen = h.ctx.content_rect();
    for button in ["Remove…", "Settings…", "Run Tests"] {
        let is = |n: &egui_kittest::Node<'_>| words(n).as_deref() == Some(button);
        scroll_to(&mut h, is);
        let at = h.query_all_by(|n| n.bounding_box().is_some()).find(|n| is(n)).map(|n| n.rect());
        let at = at.unwrap_or_else(|| panic!("{button} drawn"));
        assert!(at.right() <= screen.right() - 4.0, "{button} whole: {at:?} in {screen:?}");
    }
}

/// The Plugins tab with two of the manual's example plugins enabled, each
/// with settings and tests of its own (Settings…, Run Tests), one with a
/// question allowed before (Forget), in both themes at the four sizes.
#[test]
#[ignore]
fn u29_pictures_plugins_enabled() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("u29-pics-plugins-enabled");
    let examples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples");
    for n in ["builder-tags", "house-rules"] {
        copy_tree(&examples.join(n), &dir.join(n));
    }
    for light in [false, true] {
        for size in PICTURE_SIZES {
            let mut h = together_harness("u29-pics-plugins-enabled-module", 4, size.x, size.y);
            h.state_mut().plugin_dir = Some(dir.clone());
            h.state_mut().load_plugins();
            h.state_mut().settings.light_theme = light;
            h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
            let ids = ["example.builder-tags", "example.house-rules"];
            for id in ids {
                h.state_mut().enable_plugin(id, true);
            }
            let version = (h.state().plugins.installed.iter())
                .filter_map(|i| i.plugin.as_ref().ok())
                .find(|p| p.manifest.id == ids[0])
                .map(|p| p.manifest.version.clone())
                .expect("builder-tags read");
            h.state_mut().settings.plugin_grants =
                vec![(format!("{} {version}", ids[0]), "q".into(), "yes".into())];
            settle(&mut h);
            menu_command(&mut h, "Plugins", "Manage Plugins…");
            let screen = h.state().editor_area_rect().expect("the editor area drawn");
            let label =
                |l: &'static str| move |n: &egui_kittest::Node<'_>| words(n).as_deref() == Some(l);
            scroll_to(&mut h, label("Run Tests"));
            picture_of(&mut h, "plugins-enabled", "Settings… and Run Tests", |h| {
                label_in(h, screen, "Settings…") && label_in(h, screen, "Run Tests")
            });
            let allowed = "Allowed before, and not asked again: 1";
            scroll_to(&mut h, label(allowed));
            picture_of(&mut h, "plugins-enabled-allowed", "what was allowed, Forget", |h| {
                label_in(h, screen, allowed) && label_in(h, screen, "Forget")
            });
        }
    }
}

/// The Plugins tab pictured: none installed, several, one in error;
/// docked, split beside the module's first tab, and a window of its own.
#[test]
#[ignore]
fn u29_pictures_plugins_tab() {
    mg_testkit::gpu::hold();
    let sets = [
        ("none", plugins_of("u29-pics-plugins-none", &[])),
        ("several", plugins_of("u29-pics-plugins-several", &["asks", "forms", "tag-conventions"])),
        ("error", plugins_of("u29-pics-plugins-error", &["asks"])),
    ];
    // (One whose manifest does not read: the list's error line.)
    let unread = sets[2].1.join("unread");
    std::fs::create_dir_all(&unread).unwrap();
    std::fs::write(unread.join("plugin.cfg"), "[plugin]\nid = \"test.unread\"\nname = \n").unwrap();
    for light in [false, true] {
        for size in PICTURE_SIZES {
            for (name, dir) in &sets {
                let mut h = together_harness("u29-pics-plugins", 4, size.x, size.y);
                // (At the least window the log closed: open, it left the
                // tab three lines, under the error line's two.)
                h.state_mut().settings.hide_log = size == mg_ui::LEAST_WINDOW_SIZE;
                h.state_mut().plugin_dir = Some(dir.clone());
                h.state_mut().load_plugins();
                h.state_mut().settings.light_theme = light;
                h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
                settle(&mut h);
                menu_command(&mut h, "Plugins", "Manage Plugins…");
                // (What each is named for: the empty list's words, a
                // plugin's row, the line of the one that does not read.)
                let mark = match *name {
                    "none" => "No plugins are installed",
                    "several" => "Asks 0.0.1",
                    _ => "plugin.cfg",
                };
                scroll_to(&mut h, |n| words(n).is_some_and(|l| l.contains(mark)));
                picture_of(&mut h, &format!("plugins-{name}"), mark, |h| in_editor(h, mark));
                if *name != "several" {
                    continue;
                }
                {
                    let dock = &mut h.state_mut().dock;
                    let at = dock.find_tab(&Tab::Plugins).unwrap();
                    dock.remove_tab(at);
                    let other = (dock.iter_all_tabs())
                        .find(|(at, _)| at.surface.is_main())
                        .map(|(_, t)| t.clone())
                        .expect("a tab beside it");
                    let node = dock.find_tab(&other).unwrap().node;
                    dock.main_surface_mut().split_right(node, 0.5, vec![Tab::Plugins]);
                }
                settle(&mut h);
                picture_of(&mut h, "plugins-split", "the Plugins tab beside another", |h| {
                    let dock = &h.state().dock;
                    let at = dock.find_tab(&Tab::Plugins);
                    let leaves = dock.iter_leaves().filter(|(p, _)| p.surface.is_main()).count();
                    at.is_some_and(|a| a.surface.is_main())
                        && leaves >= 2
                        && h.state()
                            .tab_rect(&Tab::Plugins)
                            .is_some_and(|r| h.ctx.content_rect().contains_rect(r))
                });
                h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Plugins]));
                settle(&mut h);
                h.state_mut().settings.open_editors = mg_ui::OpenEditors::Floating;
                menu_command(&mut h, "Plugins", "Manage Plugins…");
                picture_of(&mut h, "plugins-floating", "the Plugins tab in a window", |h| {
                    let at = h.state().dock.find_tab(&Tab::Plugins);
                    at.is_some_and(|a| !a.surface.is_main()) && on_screen(h, "Install from File")
                });
            }
        }
    }
}

/// One area's Area Properties, as a window of its default size and as a
/// tab, on its Advanced and Visual pages, at the boards' two sizes: the
/// pages the section shares, drawn where they are not narrow.
#[test]
#[ignore]
fn u29_pictures_area_properties() {
    mg_testkit::gpu::hold();
    let area = ResRef::from_str("area0000").unwrap();
    for size in &PICTURE_SIZES[..2] {
        for (mode, name) in
            [(mg_ui::OpenEditors::Floating, "window"), (mg_ui::OpenEditors::Tabs, "tab")]
        {
            let mut h = together_harness("u29-pics-area-props", 4, size.x, size.y);
            h.state_mut().settings.open_editors = mode;
            h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
            settle(&mut h);
            for (page, mark) in [("Advanced", "Check Modifier"), ("Visual", "Fog Amount")] {
                h.get_all_by_label(page).last().expect("the page on the rail").click();
                settle(&mut h);
                let file = format!("area-properties-{name}-{}", page.to_lowercase());
                picture_of(&mut h, &file, mark, |h| on_screen(h, mark));
            }
        }
    }
}

/// The Right Dock's tab `name` drawn as an icon alone (its words not in
/// it), named `name` to the screen reader, and under the pointer its tip
/// saying `name`.
#[track_caller]
pub(crate) fn tab_named_with_tip(h: &mut Harness<'_, Moonglow>, name: &str) {
    let dock = h.state().right_dock_rect().expect("the Right Dock open");
    let tab = h
        .get_all_by_role(egui::accesskit::Role::Tab)
        .find(|n| {
            n.accesskit_node().label().as_deref() == Some(name) && dock.contains_rect(n.rect())
        })
        .unwrap_or_else(|| panic!("a tab named {name} in the Right Dock"))
        .rect();
    let words = h.ctx.fonts_mut(|f| {
        f.layout_no_wrap(name.into(), egui::FontId::proportional(13.0), egui::Color32::WHITE)
            .size()
            .x
    });
    assert!(tab.width() < words, "{name}: an icon alone ({tab:?}), as the test means");
    h.hover_at(tab.center());
    settle(h);
    wait_for(h, "its tip", |h| h.get_all_by_label(name).count() >= 2);
}

/// Frames until `done` says so, for ten seconds at most, failing with
/// `what`.
pub(crate) fn wait_for(
    h: &mut Harness<'_, Moonglow>,
    what: &str,
    done: impl Fn(&Harness<'_, Moonglow>) -> bool,
) {
    let started = std::time::Instant::now();
    while !done(h) {
        assert!(started.elapsed().as_secs() < 10, "never: {what}");
        h.step();
    }
}

/// Whether the keys' outline (the accent, 1 point inside the edge) is
/// drawn round the whole of `rect`, a tool's section: along its left and
/// right edges at half and nine tenths of its height, and along its foot
/// (M4's U29 review: Edit Areas Together's list and rail were drawn over
/// it, and only its actions' row showed it).
#[track_caller]
pub(crate) fn outlined_whole(h: &mut Harness<'_, Moonglow>, rect: egui::Rect) {
    mg_testkit::gpu::hold();
    let accent = mg_uikit::tokens(&h.ctx).accent;
    let ppp = h.ctx.pixels_per_point();
    let img = h.render().expect("drawn");
    let near = |at: egui::Pos2, along_x: bool| {
        (-3..=3).any(|d| {
            let p = if along_x {
                at + egui::vec2(d as f32, 0.0)
            } else {
                at + egui::vec2(0.0, d as f32)
            };
            let (x, y) = ((p.x * ppp) as u32, (p.y * ppp) as u32);
            if x >= img.width() || y >= img.height() {
                return false;
            }
            let c = img.get_pixel(x, y).0;
            let dist = (c[0] as i32 - accent.r() as i32).abs()
                + (c[1] as i32 - accent.g() as i32).abs()
                + (c[2] as i32 - accent.b() as i32).abs();
            dist < 40
        })
    };
    for f in [0.5, 0.9] {
        let y = rect.top() + rect.height() * f;
        assert!(near(egui::pos2(rect.left() + 1.5, y), true), "left edge at {f}: {rect:?}");
        assert!(near(egui::pos2(rect.right() - 1.5, y), true), "right edge at {f}: {rect:?}");
    }
    let foot = egui::pos2(rect.center().x, rect.bottom() - 1.5);
    assert!(near(foot, false), "foot: {rect:?}");
}
