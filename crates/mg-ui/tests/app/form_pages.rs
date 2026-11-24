//! A form window's pages (M4's U9; the Creature board): the page list at
//! its left, the pages one after another on one scroll drawn only where
//! in sight, OK and Cancel at the header's right end; the blueprints',
//! Module Properties' and Area Properties' windows.

use crate::common::*;

/// The part of the page `name` in sight (its group's bounds), `None`
/// where it is not drawn.
fn seen(h: &Harness<'_, Moonglow>, name: &str) -> Option<egui::Rect> {
    let group = format!("{name}{}", mg_uikit::pages::PAGE_GROUP);
    h.query_all_by_label(&group).last().map(|n| n.rect())
}

fn toggled(h: &Harness<'_, Moonglow>, name: &str) -> Option<egui::accesskit::Toggled> {
    page_row(h, name).and_then(|r| r.accesskit_node().toggled())
}

/// A creature's fourteen pages beside their list, docked and floating:
/// the first at the top and the list saying so, the last out of sight and
/// not drawn; a page chosen lands its bar at the top, the first then out
/// of sight; OK and Cancel at the header's right end, over the pages.
#[test]
fn a_creature_s_pages_stand_on_one_scroll_beside_their_list() {
    for mode in MODES {
        let Some((mut h, _key)) =
            blueprint_harness_in(mode, "nw_bandit001", "pages_one_scroll", ResType::UTC, None)
        else {
            return;
        };
        settle(&mut h);
        let pages = mg_ui::blueprint::pages(ResType::UTC);
        let list = h.get_by_label(mg_uikit::pages::PAGES);
        for p in pages {
            assert!(list.query_by_label(p).is_some(), "{mode:?}: {p} in the list");
        }
        let on = Some(egui::accesskit::Toggled::True);
        assert_eq!(toggled(&h, "Basic"), on, "{mode:?}: the first page at the top");
        assert_ne!(toggled(&h, "Inventory"), on);
        let top = seen(&h, "Basic").expect("the first page drawn").top();
        assert!(seen(&h, "Comments").is_none(), "{mode:?}: the last out of sight, not drawn");
        // OK and Cancel at the header's right end, over the pages, the list
        // at their left.
        let ok = h.get_by_label("OK").rect();
        let cancel = h.get_by_label("Cancel").rect();
        assert!(cancel.bottom() <= top && ok.bottom() <= top, "{mode:?}: over the pages");
        assert!(ok.right() < cancel.left(), "{mode:?}: OK, then Cancel");
        let basic_row = page_row(&h, "Basic").unwrap().rect();
        // (The list flush with the pane's left edge, as Creature.png.)
        let tab = Tab::Blueprint(_key);
        let pane = (h.state().dock.iter_leaves())
            .find(|(_, leaf)| leaf.tabs.contains(&tab))
            .map(|(_, leaf)| leaf.rect)
            .unwrap();
        assert!(
            (basic_row.left() - pane.left()).abs() <= 1.0,
            "{mode:?}: flush: {basic_row:?} in {pane:?}"
        );
        let basic = seen(&h, "Basic").unwrap();
        assert!(basic_row.right() <= basic.left() + 1.0, "{mode:?}: the list at the left");
        assert!(cancel.right() > basic.right() - 40.0, "{mode:?}: at the right end");
        // A page chosen: its bar at the top, the first page out of sight.
        page(&mut h, "Inventory");
        let inventory = seen(&h, "Inventory").expect("the page chosen drawn");
        assert_eq!(inventory.top(), top, "{mode:?}: its bar at the top");
        assert!(seen(&h, "Basic").is_none(), "{mode:?}: the first page out of sight");
        assert_ne!(toggled(&h, "Basic"), on);
        // The last page too, though shorter than the room.
        page(&mut h, "Comments");
        assert_eq!(seen(&h, "Comments").unwrap().top(), top, "{mode:?}: the last at the top");
    }
}

/// The page list takes the keyboard (named "Pages", its pages buttons):
/// Up and Down bring the page before or after to the top. Tab goes from
/// field to field down the pages in order, each brought into sight as the
/// keyboard comes to it.
#[test]
fn the_keyboard_goes_down_the_pages_and_along_the_list() {
    let Some((mut h, _key)) = blueprint_harness("nw_waypoint001", "pages_keys", ResType::UTW)
    else {
        return;
    };
    settle(&mut h);
    let on = Some(egui::accesskit::Toggled::True);
    page_row(&h, "Basic").unwrap().focus();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(toggled(&h, "Advanced"), on, "Down: the next page at the top");
    h.key_press(egui::Key::ArrowUp);
    settle(&mut h);
    assert_eq!(toggled(&h, "Basic"), on, "Up: back");
    // Tab from the first field, on down the pages.
    let pages = mg_ui::blueprint::pages(ResType::UTW);
    let tag = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some("NW_WAYPOINT001"))
        .map(|n| n.rect().center());
    let at = tag.unwrap_or_else(|| {
        h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().rect().center()
    });
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let mut visited: Vec<usize> = Vec::new();
    for _ in 0..80 {
        h.key_press(egui::Key::Tab);
        h.run_steps(3);
        let focused = h.query(egui_kittest::kittest::by().predicate(|n| n.is_focused()));
        let Some(focused) = focused.map(|n| n.rect()) else { continue };
        // (The page whose part in sight holds it: brought into view.)
        let on_page =
            pages.iter().position(|p| seen(&h, p).is_some_and(|r| r.contains(focused.center())));
        if let Some(i) = on_page
            && visited.last() != Some(&i)
        {
            visited.push(i);
        }
        if visited.last() == Some(&(pages.len() - 1)) {
            break;
        }
    }
    assert!(visited.windows(2).all(|w| w[0] < w[1]), "page after page: {visited:?}");
    assert_eq!(visited.last(), Some(&(pages.len() - 1)), "down to the last: {visited:?}");
}

/// Module Properties and Area Properties draw their pages the same way:
/// the list (Area Properties' named "Area Properties pages": the area
/// header's Audio has the name), OK and Cancel at the header's right
/// end, a page chosen at the top; Cancel still sets back what the window
/// changed, as one step.
#[test]
fn module_and_area_properties_list_their_pages_under_their_header() {
    let Some((mut h, area)) = area_ui_harness("pages-props") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    for name in ["Basic", "Events", "Advanced", "Variables", "Description", "Custom Content"] {
        assert!(h.get_by_label(mg_uikit::pages::PAGES).query_by_label(name).is_some(), "{name}");
    }
    let top = seen(&h, "Basic").expect("Basic drawn").top();
    assert!(h.get_all_by_label("Cancel").last().unwrap().rect().bottom() <= top, "over the pages");
    page(&mut h, "Custom Content");
    assert_eq!(seen(&h, "Custom Content").unwrap().top(), top, "at the top");
    close_tab(h.state_mut(), &Tab::ModuleProperties);
    settle(&mut h);

    let are = ResKey::new(area, ResType::ARE);
    let comments = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &are);
        s.string("Comments").map(|c| String::from_utf8_lossy(c).into_owned()).unwrap_or_default()
    };
    let before = comments(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    settle(&mut h);
    let list = h.get_by_label("Area Properties pages");
    for p in ["Basic", "Visual", "Grass", "Audio", "Events", "Advanced", "Variables", "Comments"] {
        assert!(list.query_by_label(p).is_some(), "{p}");
    }
    let top = seen(&h, "Basic").expect("Basic drawn").top();
    let cancel = h.get_all_by_label("Cancel").last().unwrap().rect();
    assert!(cancel.bottom() <= top, "OK and Cancel over the pages: {cancel:?}, {top}");
    page(&mut h, "Comments");
    let memo = egui_kittest::kittest::by().role(egui::accesskit::Role::MultilineTextInput);
    let at = in_page(&h, "Comments").get(memo).rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.event(egui::Event::Text("Kept for later.".into()));
    h.run_steps(2);
    page(&mut h, "Basic");
    settle(&mut h);
    assert!(comments(&mut h).contains("Kept for later."), "set as the page was left");
    h.get_all_by_label("Cancel").last().unwrap().click();
    settle(&mut h);
    assert_eq!(comments(&mut h), before, "Cancel sets it back");
    assert!(h.state().dock.find_tab(&Tab::AreaProperties(area)).is_none(), "closed");
}

/// A window too narrow for the page list's words lists its pages as the
/// Properties panel's rail of icons, leaving the form its room.
#[test]
fn a_narrow_window_lists_its_pages_as_a_rail() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "pages_narrow", ResType::UTP) else {
        return;
    };
    settle(&mut h);
    let tab = Tab::Blueprint(key);
    let width = |h: &mut Harness<'_, Moonglow>, w: f32| {
        let at = h.state().dock.find_tab(&tab).unwrap();
        if let Some(state) = h.state_mut().dock.get_window_state_mut(at.surface) {
            state.set_position(egui::pos2(20.0, 40.0)).set_size(egui::vec2(w, 600.0));
        }
        settle(h);
        page_row(h, "Basic").unwrap().rect().width()
    };
    assert_eq!(width(&mut h, 900.0), mg_uikit::pages::PAGE_LIST - 1.0, "words beside icons");
    assert_eq!(width(&mut h, 520.0), mg_uikit::pages::PAGE_BUTTON, "the rail's icons");
    page(&mut h, "Scripts");
    assert!(seen(&h, "Scripts").is_some(), "its pages still reached");
}

/// The painted texts of the frame, as drawn (a text cut short ends in its
/// ellipsis).
fn painted(h: &Harness<'_, Moonglow>) -> Vec<String> {
    (h.output().shapes.iter())
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => {
                Some(t.galley.rows.iter().flat_map(|r| r.glyphs.iter().map(|g| g.chr)).collect())
            }
            _ => None,
        })
        .collect()
}

/// Opens the blueprint `key`'s window anew in a screen `size` large, at
/// its first size (a window opens at most 85% of the main pane wide: a
/// screen wide enough for its whole width).
fn reopen(h: &mut Harness<'_, Moonglow>, key: ResKey, size: egui::Vec2) {
    h.set_size(size);
    close_tab(h.state_mut(), &Tab::Blueprint(key));
    settle(h);
    // (Not at the size the window had in the smaller screen.)
    h.state_mut().settings.window_sizes.clear();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    settle(h);
}

/// A header too short for the title gives it up a part at a time: the
/// resref first, then the name cut short with an ellipsis, the kind's icon
/// kept, OK and Cancel whole at its right end; its accessible name the
/// whole title.
#[test]
fn a_long_name_yields_in_a_narrow_header() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "long_name_chest", ResType::UTP)
    else {
        return;
    };
    settle(&mut h);
    let name = "The Chest of the Seventh Sorrowful Bandit King of the Misty Moor";
    let title = LocString::from_text(Language::ENGLISH, Gender::Male, name);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "name",
        vec![mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "LocName".into(),
            value: Some(mg_gff::Value::LocString(title)),
        }],
    )));
    settle(&mut h);
    let tab = Tab::Blueprint(key);
    let width = |h: &mut Harness<'_, Moonglow>, w: f32| {
        let at = h.state().dock.find_tab(&tab).unwrap();
        if let Some(state) = h.state_mut().dock.get_window_state_mut(at.surface) {
            state.set_position(egui::pos2(20.0, 40.0)).set_size(egui::vec2(w, 600.0));
        }
        settle(h);
    };
    width(&mut h, 960.0);
    assert!(painted(&h).iter().any(|t| t == name), "the whole name where it fits");
    assert!(painted(&h).iter().any(|t| t == "long_name_chest"), "and its resref");
    for w in [520.0, 440.0] {
        width(&mut h, w);
        let texts = painted(&h);
        assert!(!texts.iter().any(|t| t == "long_name_chest"), "at {w} the resref given up first");
        let cut = texts.iter().find(|t| t.starts_with("The Chest") && t.ends_with('…'));
        assert!(cut.is_some(), "at {w} the name cut short: {texts:?}");
        let whole = format!("{name} long_name_chest");
        let ok = h.get_by_label("OK").rect();
        let icon = h.get_by_label(&whole).rect();
        assert!(icon.right() < ok.left(), "at {w} the icon kept, named by the whole title");
        assert!(h.get_by_label("Cancel").rect().right() <= 20.0 + w, "Cancel whole");
    }
}

/// The windows open wide enough beside the page list for what they showed
/// before it (the coordinator's ruling at U9's review): a placeable's form
/// as wide as U9's 980 gave it and its Preview's pane beside it (M4's
/// U11: the model is the pane's, no longer its Basic page's), an
/// encounter's creatures with their Unique boxes, no sideways scroll.
#[test]
fn a_window_opens_wide_enough_beside_its_page_list() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "wide_chest", ResType::UTP) else {
        return;
    };
    // (The form's room: from the page list's left to the header's Cancel
    // at its right end.)
    let room = |h: &Harness<'_, Moonglow>| {
        let left = h.get_all_by_label("Basic").map(|n| n.rect().left()).fold(f32::MAX, f32::min);
        (left, h.get_by_label("Cancel").rect().right())
    };
    reopen(&mut h, key, egui::vec2(2600.0, 900.0));
    let (left, right) = room(&h);
    let alone = right - left;
    assert!(h.query_by_label_contains("model viewer").is_none(), "no pane: no model");
    // (Its window opens wider than U9's 980 by the pane, where the kind
    // has a model: the form alone has more than 980 here; the room 980
    // gave it is that less the pane.)
    // (A screen wide enough that the window is not held to 85% of the
    // main pane.)
    assert!(alone > 980.0 + 200.0, "opened wider than 980: the form alone {alone}");
    let at_980 = alone - (mg_ui::RIGHT_DOCK_LEAST + 1.0);
    h.state_mut().palette_dock = true;
    reopen(&mut h, key, egui::vec2(2600.0, 900.0));
    let basic = in_page(&h, "Basic");
    assert!(basic.query_by_label_contains("model viewer").is_none(), "no model in the page");
    let model = h.query_by_label_contains("model viewer");
    let model = model.expect("the model (here its want of a GPU said) in the window's pane").rect();
    let (left, right) = room(&h);
    assert!(model.left() >= right, "the pane at the form's right: {right} {model:?}");
    // (The window opens as much wider than U9's 980 as the pane is: the
    // form has what 980 gave it.)
    assert!(
        (right - left - at_980).abs() <= 2.0,
        "the form keeps U9's room beside the pane: {} beside it, {at_980} at 980",
        right - left
    );
    drop(h);
    let Some((mut h, key)) = blueprint_harness("nw_verminbeet", "wide_encounter", ResType::UTE)
    else {
        return;
    };
    reopen(&mut h, key, egui::vec2(2000.0, 900.0));
    page(&mut h, "Creature List");
    let list = in_page(&h, "Creature List");
    let seen = list.rect();
    let boxes: Vec<egui::Rect> =
        list.get_all_by_role(egui::accesskit::Role::CheckBox).map(|n| n.rect()).collect();
    assert!(!boxes.is_empty(), "the Unique boxes");
    for b in boxes {
        assert!(b.right() <= seen.right(), "{b:?} in sight, in {seen:?}");
    }
}

/// Area Properties of several areas keep their page by the whole set of
/// areas: two sets beginning with the same area, open at once, each keep
/// the page at their own top.
#[test]
fn each_set_of_areas_keeps_its_own_page() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-pages-area-sets");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    for name in ["cave1", "cave2", "cave3"] {
        let mut are = Gff::new(*b"ARE ");
        let title = LocString::from_text(Language::ENGLISH, Gender::Male, name);
        are.root.set("Name", mg_gff::Value::LocString(title));
        m.set(ResKey::parse(name, ResType::ARE).unwrap(), are.to_bytes().unwrap());
        let mut git = Gff::new(*b"GIT ");
        git.root.set("AreaProperties", mg_gff::Value::Struct(mg_gff::Struct::new(100)));
        m.set(ResKey::parse(name, ResType::GIT).unwrap(), git.to_bytes().unwrap());
    }
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let set = |names: &[&str]| -> Vec<ResRef> {
        names.iter().map(|n| ResRef::from_str(n).unwrap()).collect()
    };
    let first = Tab::AreasProperties(set(&["cave1", "cave2"]));
    app.actions.push(mg_ui::Action::OpenTab(first.clone()));
    let mut h = window_harness(app);
    settle(&mut h);
    page(&mut h, "Audio");
    let second = Tab::AreasProperties(set(&["cave1", "cave3"]));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(second));
    settle(&mut h);
    // (Each list's page at the top, the two windows' lists in turn.)
    let on = Some(egui::accesskit::Toggled::True);
    let tops: Vec<String> = (h.get_all_by_label("Area Properties pages"))
        .flat_map(|list| {
            ["Visual", "Grass", "Audio", "Events", "Advanced", "Variables"]
                .into_iter()
                .filter(|p| {
                    list.query_by_label(p).is_some_and(|r| r.accesskit_node().toggled() == on)
                })
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect();
    let mut sorted = tops.clone();
    sorted.sort();
    assert_eq!(sorted, ["Audio", "Visual"], "each its own: {tops:?}");
}

/// No form page has its scroll drawn again frame after frame (M4's U11
/// review: a page whose height followed what of it was in sight, the
/// Appearance page's model beside its fields, had every frame drawn twice,
/// egui warning of a discard a thousand frames in a row): every kind's
/// pages scrolled through a step at a time, docked at 1200 and at 3440 (in
/// columns) and floating at 1000, each step drawn again at most twice in
/// the ten frames after it.
#[test]
fn no_form_page_is_drawn_again_every_frame() {
    let kinds = [
        ("nw_aarcl001", ResType::UTI),
        ("nw_bartender", ResType::UTC),
        ("plc_chest1", ResType::UTP),
        ("nw_storebar01", ResType::UTM),
        ("nw_door_ttr_01", ResType::UTD),
        ("nw_verminbeet", ResType::UTE),
    ];
    let sizes = [
        (Mode::Tabs, egui::vec2(1200.0, 900.0)),
        (Mode::Floating, egui::vec2(1000.0, 800.0)),
        (Mode::Tabs, egui::vec2(3440.0, 1440.0)),
    ];
    for (mode, size) in sizes {
        for (name, t) in kinds {
            let copy = format!("d{}", &name[name.len().saturating_sub(12)..]);
            let Some((mut h, _)) = blueprint_harness_in(mode, name, &copy, t, None) else { return };
            h.set_size(size);
            settle(&mut h);
            for step in 0..8 {
                h.hover_at(egui::pos2(size.x * 0.6, size.y * 0.5));
                let delta = if step < 6 { -300.0 } else { 400.0 };
                h.event(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, delta),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                });
                h.run_steps(2);
                let (p, f) = (h.ctx.cumulative_pass_nr(), h.ctx.cumulative_frame_nr());
                h.run_steps(6);
                let again = (h.ctx.cumulative_pass_nr() - p) - (h.ctx.cumulative_frame_nr() - f);
                assert!(
                    again <= 2,
                    "{name} {mode:?} {size:?} step {step}: drawn again {again} times in 10 frames"
                );
            }
        }
    }
}
