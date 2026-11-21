//! A form's sections and columns (M4's U10; the Creature and WideCreature
//! boards): the creature's Basic page in its sections, the strref inside
//! the name's field, sections flowing into columns of 440 to 520 by the
//! width, every page side by side in columns at 3440, a number field's
//! arrows sized with it.

use crate::common::*;

/// The Basic page's sections and the labels under each, in order.
const SECTIONS: [(&str, &[&str]); 5] = [
    ("Identity", &["First name", "Last name", "Tag"]),
    ("Race and Appearance", &["Race", "Appearance", "Phenotype", "Gender"]),
    ("Palette", &["Challenge rating", "Category"]),
    ("Portrait", &["Portrait"]),
    ("Conversation", &["Conversation", "No interrupt"]),
];

/// The rectangles of what is named `name` on the page `page`, top first.
fn named(h: &Harness<'_, Moonglow>, page: &str, name: &str) -> Vec<egui::Rect> {
    let mut all: Vec<egui::Rect> =
        in_page(h, page).get_all_by_label(name).map(|n| n.rect()).collect();
    all.sort_by(|a, b| a.top().total_cmp(&b.top()));
    all
}

/// A section's heading: the topmost of its name on the page (a heading
/// and a label may share it: Portrait, Conversation).
fn heading(h: &Harness<'_, Moonglow>, name: &str) -> egui::Rect {
    named(h, "Basic", name).first().copied().unwrap_or_else(|| panic!("no heading {name}"))
}

/// Each section's labels under its heading, in its column, before the
/// next heading of that column; the description under them all.
fn each_label_under_its_heading(h: &Harness<'_, Moonglow>, what: &str) {
    let headings: Vec<egui::Rect> = SECTIONS.iter().map(|(s, _)| heading(h, s)).collect();
    for (i, (section, labels)) in SECTIONS.iter().enumerate() {
        let head = headings[i];
        // (The next heading below it in its column.)
        let next = (headings.iter())
            .filter(|r| (r.left() - head.left()).abs() < 1.0 && r.top() > head.top())
            .map(|r| r.top())
            .fold(f32::INFINITY, f32::min);
        for label in *labels {
            let found = named(h, "Basic", label);
            let at = found
                .iter()
                .find(|r| r.top() > head.bottom() && r.top() < next)
                .unwrap_or_else(|| panic!("{what}: {label} under {section}: {found:?} {head:?}"));
            // (A label at the column's left, a check box in its values.)
            assert!(
                at.left() >= head.left() - 1.0 && at.left() < head.left() + 440.0 - 100.0,
                "{what}: {label} in {section}'s column: {at:?} {head:?}"
            );
        }
    }
    // (Named with the language edited, which it shows after it.)
    let description = heading(h, "Description (English)");
    for (s, labels) in SECTIONS {
        for label in labels {
            let last = *named(h, "Basic", label).last().unwrap();
            assert!(last.bottom() < description.top(), "{what}: {s}'s {label} over Description");
        }
    }
}

/// The creature's Basic page in the board's sections: each section's
/// labels under its heading, in two columns in a docked editor at 1600
/// (Identity and Race and Appearance, then Palette, Portrait and
/// Conversation) and one in a floating window at its first width; the
/// description under them across the page; the strref inside the first
/// name's field, its "…" and Random Name after the field.
#[test]
fn a_creature_s_basic_page_stands_in_its_sections() {
    for mode in MODES {
        let Some((mut h, _key)) =
            blueprint_harness_in(mode, "nw_bartender", "basic_sections", ResType::UTC, None)
        else {
            return;
        };
        if mode == Mode::Tabs {
            h.set_size(egui::vec2(1600.0, 900.0));
        }
        settle(&mut h);
        let what = format!("{mode:?}");
        each_label_under_its_heading(&h, &what);
        let (identity, palette) = (heading(&h, "Identity"), heading(&h, "Palette"));
        let race = heading(&h, "Race and Appearance");
        match mode {
            Mode::Tabs => {
                assert_eq!(identity.top(), palette.top(), "{what}: two columns, side by side");
                assert!(palette.left() >= identity.left() + 440.0 + 32.0, "{what}: 440 apart");
                assert_eq!(race.left(), identity.left(), "{what}: Race under Identity");
                let portrait = heading(&h, "Portrait");
                assert_eq!(portrait.left(), palette.left(), "{what}: Portrait under Palette");
            }
            Mode::Floating => {
                assert_eq!(palette.left(), identity.left(), "{what}: one column");
                assert!(palette.top() > race.top(), "{what}: in their order");
            }
        }
        // The strref inside the first name's field, at its right end; the
        // field's "…" and Random Name after it on its row.
        let strref = in_page(&h, "Basic").get_by_label("StrRef 12374").rect();
        let first = named(&h, "Basic", "First name")[0];
        let field = (in_page(&h, "Basic").get_all_by_role(egui::accesskit::Role::TextInput))
            .map(|n| n.rect())
            .find(|r| (r.center().y - first.center().y).abs() < 4.0)
            .expect("the first name's field");
        assert!(field.contains_rect(strref), "{what}: the strref inside the field");
        assert!(strref.right() > field.right() - 12.0, "{what}: at its right end");
        let after = |name: &str| {
            (in_page(&h, "Basic").get_all_by_label(name))
                .map(|n| n.rect())
                .find(|r| (r.center().y - field.center().y).abs() < 4.0)
                .unwrap_or_else(|| panic!("{what}: no {name} on the row"))
        };
        let (languages, dice) = (after("…"), after("Random Name"));
        assert!(languages.left() >= field.right(), "{what}: … after the field");
        assert!(dice.left() >= languages.right(), "{what}: Random Name after …");
        assert_eq!(languages.height(), field.height(), "{what}: … as high as the field");
    }
}

/// The page's sections flow into one column of 440 or more, or two, by
/// the window's width; Statistics' Ability Scores and Saves stand in the
/// first column of two, Armor Class and Hit Points in the second.
#[test]
fn sections_flow_into_columns_by_the_width() {
    let Some((mut h, key)) = blueprint_harness("nw_bartender", "flow_columns", ResType::UTC) else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 900.0));
    let tab = Tab::Blueprint(key);
    let width = |h: &mut Harness<'_, Moonglow>, w: f32| {
        let at = h.state().dock.find_tab(&tab).unwrap();
        if let Some(state) = h.state_mut().dock.get_window_state_mut(at.surface) {
            state.set_position(egui::pos2(20.0, 40.0)).set_size(egui::vec2(w, 820.0));
        }
        settle(h);
    };
    // (Beside the list's 150 and the page's margins 28: two of 440 and the
    // gap of 32 need 912.)
    for (w, columns) in [(980.0, 1), (1080.0, 1), (1110.0, 2), (1500.0, 2)] {
        width(&mut h, w);
        let (identity, palette) = (heading(&h, "Identity"), heading(&h, "Palette"));
        let two = palette.left() > identity.left() + 400.0;
        assert_eq!(if two { 2 } else { 1 }, columns, "at {w}: {identity:?} {palette:?}");
    }
    page(&mut h, "Statistics");
    let at = |h: &Harness<'_, Moonglow>, name: &str| {
        in_page(h, "Statistics").get_all_by_label(name).next().unwrap().rect()
    };
    let (abilities, saves) = (at(&h, "Ability Scores"), at(&h, "Saves"));
    let (ac, hp) = (at(&h, "Armor Class"), at(&h, "Hit Points"));
    assert_eq!(saves.left(), abilities.left(), "Saves under Ability Scores");
    assert!(saves.top() > abilities.top());
    assert_eq!(ac.top(), abilities.top(), "Armor Class beside them");
    assert_eq!(hp.left(), ac.left(), "Hit Points under Armor Class");
    width(&mut h, 980.0);
    let (abilities, saves) = (at(&h, "Ability Scores"), at(&h, "Saves"));
    let (ac, hp) = (at(&h, "Armor Class"), at(&h, "Hit Points"));
    assert!(
        abilities.top() < ac.top() && ac.top() < hp.top() && hp.top() < saves.top(),
        "one column: Ability Scores, Armor Class, Hit Points, Saves"
    );
}

/// At 3440 by 1440 a creature's fourteen pages are all in view at once,
/// side by side in columns of 480 or more (six in today's room, the
/// board's five beside a Right Dock as WideCreature),
/// Basic, Statistics, Appearance, Classes and Skills heading them; the
/// list marks each page in view; a page chosen there is the page at the
/// top, nothing scrolled.
#[test]
fn at_3440_every_page_of_a_creature_is_in_view_in_columns() {
    let Some((mut h, _key)) =
        blueprint_harness_in(Mode::Tabs, "nw_bartender", "wide_pages", ResType::UTC, None)
    else {
        return;
    };
    h.set_size(egui::vec2(3440.0, 1440.0));
    settle(&mut h);
    let pages = mg_ui::blueprint::pages(ResType::UTC);
    let group = |h: &Harness<'_, Moonglow>, p: &str| {
        let name = format!("{p}{}", mg_uikit::pages::PAGE_GROUP);
        h.query_all_by_label(&name).last().map(|n| n.rect())
    };
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(3440.0, 1440.0));
    let mut lefts: Vec<f32> = Vec::new();
    for p in pages {
        let r = group(&h, p).unwrap_or_else(|| panic!("{p} drawn"));
        assert!(r.height() > 20.0 && screen.contains_rect(r), "{p} in view: {r:?}");
        if !lefts.iter().any(|l| (l - r.left()).abs() < 1.0) {
            lefts.push(r.left());
        }
    }
    let basic = group(&h, "Basic").unwrap();
    let body = screen.right() - basic.left();
    let columns = lefts.len();
    assert_eq!(columns, 6, "six columns in today's room: {lefts:?}");
    assert!((body / columns as f32) >= 480.0, "each 480 or more: {body}");
    for p in ["Statistics", "Appearance", "Classes", "Skills"] {
        let r = group(&h, p).unwrap();
        assert_eq!(r.top(), basic.top(), "{p} heads a column");
    }
    // The list: the page at the top filled, every other marked in view.
    let on = Some(egui::accesskit::Toggled::True);
    assert_eq!(page_row(&h, "Basic").unwrap().accesskit_node().toggled(), on);
    let t = mg_uikit::tokens::Tokens::DARK;
    let marks: Vec<egui::Rect> = (h.output().shapes.iter())
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r)
                if r.fill == t.line_soft && r.rect.width() == mg_uikit::pages::IN_VIEW_MARK =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    for p in &pages[1..] {
        let row = page_row(&h, p).unwrap().rect();
        let marked = marks.iter().any(|m| m.left() == row.left() && row.contains(m.center()));
        assert!(marked, "{p} marked in view");
    }
    // A page chosen: the page at the top, the first still in view.
    page(&mut h, "Comments");
    assert_eq!(group(&h, "Basic").unwrap().top(), basic.top(), "nothing scrolled");
    // Beside its own Preview in the Right Dock, as WideCreature's (M4's
    // U11): the board's five columns, every page still in view.
    page(&mut h, "Basic");
    h.state_mut().palette_dock = true;
    settle(&mut h);
    let dock = right_dock_rect(&h);
    assert!((540.0..=561.0).contains(&dock.width()), "the Preview's dock: {dock:?}");
    let mut lefts: Vec<f32> = pages.iter().filter_map(|p| group(&h, p)).map(|r| r.left()).collect();
    lefts.sort_by(f32::total_cmp);
    lefts.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    assert_eq!(lefts.len(), 5, "the board's five: {lefts:?}");
    assert!(lefts.iter().all(|l| *l < dock.left()), "beside the dock: {lefts:?}");
    for p in pages {
        assert!(group(&h, p).is_some_and(|r| r.height() > 20.0), "{p} in view");
    }
}

/// A number field's arrows stand one over the other at its right end, as
/// high as the field together: sized with their field.
#[test]
fn a_number_field_s_arrows_are_sized_with_it() {
    let Some((mut h, _key)) = blueprint_harness("nw_bartender", "spin_sized", ResType::UTC) else {
        return;
    };
    settle(&mut h);
    page(&mut h, "Statistics");
    let stats = in_page(&h, "Statistics");
    let up = stats.get_by_label("Strength: One More").rect();
    let down = stats.get_by_label("Strength: One Less").rect();
    let label = stats.get_all_by_label("Strength").next().unwrap().rect();
    let field = (stats.get_all_by_role(egui::accesskit::Role::SpinButton))
        .map(|n| n.rect())
        .find(|r| (r.center().y - label.center().y).abs() < 4.0)
        .expect("Strength's field");
    assert_eq!(up.left(), down.left(), "one over the other");
    assert_eq!(up.bottom(), down.top(), "touching");
    assert!((up.top() - field.top()).abs() <= 1.0, "from the field's top: {up:?} {field:?}");
    assert!((down.bottom() - field.bottom()).abs() <= 1.0, "to its foot: {down:?}");
    assert!(up.left() >= field.right() && up.left() - field.right() <= 4.0, "at its right end");
    assert!(up.width() < field.height(), "narrower than the field is high");
    assert!(up.width() <= 16.0, "14 wide: {up:?}");
}

/// Every page of every kind fits the room it is given: its pages side by
/// side (at 3440, and in columns at their least, 480), each control inside
/// its page's column; in a window of 1180, where a page's sections stand
/// in two columns, inside the page in sight. (A caster with spells, the creature's Spells page full.)
#[test]
fn every_page_of_every_kind_fits_its_column() {
    let editors = [
        ("nw_bandit001", ResType::UTC),
        ("nw_mumcleric", ResType::UTC),
        ("nw_arhe001", ResType::UTI),
        ("plc_chest1", ResType::UTP),
        ("nw_door_ttr_01", ResType::UTD),
        ("trackstrigger", ResType::UTT),
        ("nw_verminbeet", ResType::UTE),
        ("nw_storebar01", ResType::UTM),
        ("animalcriesday", ResType::UTS),
        ("nw_waypoint001", ResType::UTW),
    ];
    let mut found = Vec::new();
    for (n, (name, t)) in editors.into_iter().enumerate() {
        let copy = format!("fit_{n}");
        let Some((mut h, key)) = blueprint_harness(name, &copy, t) else { return };
        for (screen, window) in [
            (egui::vec2(3440.0, 1440.0), egui::vec2(3420.0, 1360.0)),
            // (Four columns at their least, 480: the board's width.)
            (egui::vec2(3440.0, 1440.0), egui::vec2(2090.0, 1360.0)),
            (egui::vec2(1400.0, 900.0), egui::vec2(1180.0, 820.0)),
        ] {
            h.set_size(screen);
            let at = h.state().dock.find_tab(&Tab::Blueprint(key)).unwrap();
            if let Some(state) = h.state_mut().dock.get_window_state_mut(at.surface) {
                state.set_position(egui::pos2(10.0, 40.0)).set_size(window);
            }
            let pages = mg_ui::blueprint::pages(t);
            for page in pages {
                h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), page);
                settle(&mut h);
                let group = in_page(&h, page);
                let seen = group.rect();
                // (In columns, a page's column: from its left to the next
                // column's, which the pages' lefts give.)
                let mut lefts: Vec<f32> = (pages.iter())
                    .filter_map(|p| {
                        let name = format!("{p}{}", mg_uikit::pages::PAGE_GROUP);
                        h.query_all_by_label(&name).last().map(|g| g.rect().left())
                    })
                    .collect();
                lefts.sort_by(f32::total_cmp);
                lefts.dedup_by(|a, b| (*a - *b).abs() < 1.0);
                let right = match lefts.windows(2).next() {
                    Some(w) if window.x > 2000.0 => seen.left() + (w[1] - w[0]) - 1.0,
                    _ => seen.right(),
                };
                for node in group.query_all(egui_kittest::kittest::by()) {
                    let Some(b) = node.accesskit_node().bounding_box() else { continue };
                    let r = egui::Rect::from_min_max(
                        egui::pos2(b.x0 as f32, b.y0 as f32),
                        egui::pos2(b.x1 as f32, b.y1 as f32),
                    );
                    if !r.is_positive()
                        || r == seen
                        || r.bottom() < seen.top()
                        || r.top() > seen.bottom()
                    {
                        continue;
                    }
                    // (A list's scroll bar stands over its edge by design.)
                    if node.accesskit_node().role() == egui::accesskit::Role::ScrollBar {
                        continue;
                    }
                    if r.right() > right + 1.0 || r.left() < seen.left() - 1.0 {
                        let a = node.accesskit_node();
                        let what = a.label().or_else(|| a.value()).unwrap_or_default();
                        found.push(format!(
                            "{name} {page} at {}: {:?} {what:?} {r:?} past {right}",
                            window.x,
                            node.accesskit_node().role()
                        ));
                    }
                }
            }
        }
    }
    assert!(found.is_empty(), "past their room:\n{}", found.join("\n"));
}

/// A creature docked across 3440 by 1440, As Tabs.
fn wide_creature(copy: &str) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let (mut h, key) = blueprint_harness_in(Mode::Tabs, "nw_bartender", copy, ResType::UTC, None)?;
    h.set_size(egui::vec2(3440.0, 1440.0));
    settle(&mut h);
    Some((h, key))
}

/// At 3440, beside the creature's own Preview as a builder has it (M4's
/// U11: five columns), a list's page is given what its column has to
/// spare: Special Abilities, a list under Skills and Spells in its column,
/// more than the least a list is given (Feats was, under Spells, while the
/// Appearance page held the model: now Scripts follows it); Skills shows
/// every skill, each rank inside the page (the review's A3).
#[test]
fn at_3440_a_list_is_given_its_column_s_room() {
    let Some((mut h, _key)) = wide_creature("wide_skills") else { return };
    h.state_mut().palette_dock = true;
    settle(&mut h);
    let skills = in_page(&h, "Skills").rect();
    let abilities = in_page(&h, "Special Abilities").rect();
    assert!((skills.left() - abilities.left()).abs() < 1.0, "under Skills: {abilities:?}");
    assert!(right_dock_rect(&h).left() > skills.right(), "beside the Preview");
    let abilities = in_page(&h, "Special Abilities").rect();
    assert!(
        abilities.height() > mg_uikit::pages::PAGE_FILL + 40.0,
        "Special Abilities given its column's spare: {abilities:?}"
    );
    let skills = in_page(&h, "Skills");
    let seen = skills.rect();
    let ranks: Vec<egui::Rect> = (skills.query_all_by_role(egui::accesskit::Role::SpinButton))
        .chain(skills.query_all_by_role(egui::accesskit::Role::TextInput))
        .map(|n| n.rect())
        .collect();
    assert!(ranks.len() >= 27, "every skill's rank: {}", ranks.len());
    for r in &ranks {
        assert!(seen.contains_rect(*r), "{r:?} inside the page in sight {seen:?}");
    }
}

/// Every equipment slot's name is drawn whole, in a column of pages side
/// by side (480) and in a window at its first width (the review's A4).
#[test]
fn equipment_slot_names_are_drawn_whole() {
    let Some((mut h, key)) = blueprint_harness("nw_bartender", "slots_whole", ResType::UTC) else {
        return;
    };
    let slots = [
        "Gloves or Bracers",
        "Secondary Weapon or Shield",
        "Creature Weapon (Left)",
        "Creature Weapon (Right)",
        "Creature Weapon (Bite)",
        "Creature Hide",
    ];
    for (screen, window) in [
        (egui::vec2(3440.0, 1440.0), egui::vec2(2090.0, 1360.0)),
        (egui::vec2(1600.0, 1000.0), egui::vec2(980.0, 940.0)),
    ] {
        h.set_size(screen);
        let at = h.state().dock.find_tab(&Tab::Blueprint(key)).unwrap();
        if let Some(state) = h.state_mut().dock.get_window_state_mut(at.surface) {
            state.set_position(egui::pos2(10.0, 40.0)).set_size(window);
        }
        settle(&mut h);
        page(&mut h, "Inventory");
        let mut checked = 0;
        for slot in slots {
            // (Scrolled to it, then its name as drawn.)
            let Some(node) = in_page(&h, "Inventory").query_by_label(slot) else { continue };
            node.scroll_to_me();
            settle(&mut h);
            let r = in_page(&h, "Inventory").get_by_label(slot).rect();
            let texts = crate::shell::drawn_texts(&h, r.expand2(egui::vec2(0.0, 2.0)));
            assert!(texts.iter().any(|t| t == slot), "{}: {slot} drawn whole: {texts:?}", window.x);
            checked += 1;
        }
        assert_eq!(checked, slots.len(), "{}: every slot found", window.x);
    }
}

/// The board's remaining layouts (the review's C10): the Classes page in
/// Alignment, Classes and Associates, each label under its heading;
/// Advanced's check boxes two to a row, Challenge Rating under them;
/// Skills in two lists side by side where the page has the room.
#[test]
fn classes_advanced_and_skills_stand_as_widecreature_draws_them() {
    let Some((mut h, _key)) = blueprint_harness("nw_bartender", "wide_layouts", ResType::UTC)
    else {
        return;
    };
    settle(&mut h);
    page(&mut h, "Classes");
    let classes = in_page(&h, "Classes");
    let top = |name: &str| {
        let mut all: Vec<egui::Rect> = classes.get_all_by_label(name).map(|n| n.rect()).collect();
        all.sort_by(|a, b| a.top().total_cmp(&b.top()));
        all
    };
    let (alignment, list, associates) =
        (top("Alignment")[0], top("Classes")[0], top("Associates")[0]);
    assert!(alignment.top() < list.top() && list.top() < associates.top(), "in order");
    for label in ["Good–Evil", "Lawful–Chaotic"] {
        let r = top(label)[0];
        assert!(r.top() > alignment.bottom() && r.top() < list.top(), "{label} under Alignment");
    }
    let class = top("Class 1")[0];
    assert!(class.top() > list.bottom() && class.top() < associates.top(), "Class 1 under Classes");
    for label in ["Familiar", "Default package for autolevelup"] {
        let r = top(label)[0];
        assert!(r.top() > associates.bottom(), "{label} under Associates");
    }
    page(&mut h, "Advanced");
    let advanced = in_page(&h, "Advanced");
    let at = |name: &str| advanced.get_by_label(name).rect();
    let (lootable, disarmable) = (at("Leaves lootable corpse"), at("Disarmable"));
    assert!((lootable.center().y - disarmable.center().y).abs() < 2.0, "two to a row");
    assert!(disarmable.left() > lootable.right(), "side by side");
    let cr = advanced.get_all_by_label("Challenge Rating").next().unwrap().rect();
    assert!(cr.top() > at("Immortal").bottom(), "Challenge Rating under the check boxes");
    page(&mut h, "Skills");
    let heads: Vec<egui::Rect> =
        in_page(&h, "Skills").get_all_by_label("Skill").map(|n| n.rect()).collect();
    assert_eq!(heads.len(), 2, "two lists");
    assert!((heads[0].top() - heads[1].top()).abs() < 1.0, "side by side");
}

/// A conversation's `…` is as high as its field, as a name's (the
/// review's C12).
#[test]
fn a_conversation_s_button_is_as_high_as_its_field() {
    let Some((mut h, _key)) = blueprint_harness("nw_bartender", "conv_button", ResType::UTC) else {
        return;
    };
    settle(&mut h);
    let basic = in_page(&h, "Basic");
    let label = basic.get_all_by_label("Conversation").map(|n| n.rect()).next_back().unwrap();
    let on_row = |r: &egui::Rect| (r.center().y - label.center().y).abs() < 4.0;
    let field = (basic.get_all_by_role(egui::accesskit::Role::TextInput))
        .map(|n| n.rect())
        .find(on_row)
        .expect("the conversation's field");
    let button = basic.get_all_by_label("…").map(|n| n.rect()).find(on_row).expect("its …");
    assert_eq!(button.height(), field.height(), "as high as the field");
    assert!(button.left() >= field.right(), "after it");
}

/// What is typed in a number field of Statistics, not yet set, is kept
/// when the page's sections change their order with the column count
/// (each section's ids by its own key, the review's B5).
#[test]
fn a_draft_survives_the_sections_reordered_by_the_width() {
    let Some((mut h, key)) = blueprint_harness("nw_bartender", "stats_draft", ResType::UTC) else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 900.0));
    let tab = Tab::Blueprint(key);
    let width = |h: &mut Harness<'_, Moonglow>, w: f32| {
        let at = h.state().dock.find_tab(&tab).unwrap();
        if let Some(state) = h.state_mut().dock.get_window_state_mut(at.surface) {
            state.set_position(egui::pos2(20.0, 40.0)).set_size(egui::vec2(w, 820.0));
        }
        settle(h);
    };
    width(&mut h, 1150.0);
    page(&mut h, "Statistics");
    let field = |h: &Harness<'_, Moonglow>| {
        let stats = in_page(h, "Statistics");
        let label = stats.get_by_label("Base hit points").rect();
        (stats.query_all_by_role(egui::accesskit::Role::SpinButton))
            .chain(stats.query_all_by_role(egui::accesskit::Role::TextInput))
            .map(|n| (n.rect(), n.accesskit_node().is_focused(), n.accesskit_node().value()))
            .find(|(r, ..)| (r.center().y - label.center().y).abs() < 4.0)
            .expect("Base hit points' field")
    };
    let at = field(&h).0.center();
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("77".into()));
    h.run_steps(3);
    assert_eq!(field(&h).2.as_deref(), Some("77"), "typed");
    // Narrower: one column, the sections in another order.
    width(&mut h, 980.0);
    let (_, focused, value) = field(&h);
    assert!(focused, "the field keeps the keyboard");
    assert_eq!(value.as_deref(), Some("77"), "and what was typed");
}
