//! Areas as a whole: their properties, several edited together, copies,
//! renames and variables.

use crate::common::*;

#[test]
fn minimap_exported_and_object_walkmeshes_shown() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-minimap");
    let path = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &path).unwrap();
    let png = dir.join("map.png");
    let dialogs = NoDialogs { save: vec![png.clone()], ..Default::default() };
    let mut app = game_app_with(&root, None, dialogs);
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.get_by_label("CHESS").click_secondary();
    h.run();
    h.get_by_label("Export Minimap…").click();
    h.run();
    let data = std::fs::read(&png).expect("the minimap is saved");
    assert_eq!(&data[1..4], b"PNG");

    // The area view draws placeables' and doors' walkmeshes when asked.
    let area = ResRef::from_str("chess").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    h.run();
    // (One button for the walkmeshes: a right click, or holding it,
    // opens the two to choose from.)
    h.get_by_label("👣 Walkmesh…").click_secondary();
    h.run();
    h.get_by_label("Object Walkmeshes").click();
    h.run();
    assert!(h.state().area_views[&area].object_walkmesh);
    assert!(!h.state().area_views[&area].walkmesh);
    // A click on the button hides what is shown.
    h.get_by_label("Ground Walkmesh");
    h.key_press(egui::Key::Escape);
    h.run();
    h.get_by_label("👣 Walkmesh…").click();
    h.run();
    assert!(!h.state().area_views[&area].object_walkmesh);
}

/// Area Properties › Visual: another area's lighting given to this one
/// (Copy From Area), and lighting kept under a name and given to another
/// area (Saved Lighting): dansen9191's ask.
#[test]
fn an_area_s_lighting_is_copied_and_kept_for_another() {
    use mg_module::new::{AreaSpec, add_area};
    let Some(mut h) = game_harness("lighting") else { return };
    let dir = mg_testkit::scratch_dir("ui-lighting-kept");
    h.state_mut().lighting_dir = Some(dir.clone());
    let first = ResRef::from_str("start").unwrap();
    let second = {
        let app = h.state_mut();
        let game = app.game.clone().unwrap();
        let spec = AreaSpec {
            name: "Second".into(),
            tileset: ResRef::from_str("ttr01").unwrap(),
            width: 4,
            height: 4,
        };
        let ws = app.ws.as_mut().unwrap();
        add_area(&mut ws.module, &game, &spec, &mut fastrand::Rng::with_seed(3)).unwrap()
    };
    let field = |h: &mut Harness<'_, Moonglow>, area: ResRef, label: &str| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().root.get(label).cloned()
    };
    // The first area's colors made its own.
    let set = |label: &str, v: mg_gff::Value| mg_edit::Edit::SetField {
        key: ResKey::new(first, ResType::ARE),
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(v),
    };
    let edits = vec![
        set("SunAmbientColor", mg_gff::Value::Dword(0x12_3456)),
        set("MoonFogColor", mg_gff::Value::Dword(0x00_0a0b)),
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Own", edits)));
    h.run();
    assert_ne!(field(&mut h, second, "SunAmbientColor"), Some(mg_gff::Value::Dword(0x12_3456)));
    // In the second's Visual page: Copy From Area › the first.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(second)));
    h.run_steps(3);
    h.get_by_label("Visual").click();
    h.run_steps(2);
    h.get_by_label("Copy From Area").click();
    h.run_steps(2);
    h.get_all_by_label("start").last().unwrap().click();
    h.run_steps(3);
    assert_eq!(field(&mut h, second, "SunAmbientColor"), Some(mg_gff::Value::Dword(0x12_3456)));
    assert_eq!(field(&mut h, second, "MoonFogColor"), Some(mg_gff::Value::Dword(0x00_0a0b)));
    assert!(h.state().ws.as_ref().unwrap().can_undo().is_some_and(|u| u.starts_with("Visual")));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert_ne!(field(&mut h, second, "SunAmbientColor"), Some(mg_gff::Value::Dword(0x12_3456)));
    // The first's lighting kept as "Dusk", from its own page.
    close_tab(h.state_mut(), &Tab::AreaProperties(second));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(first)));
    h.run_steps(3);
    h.get_by_label("Visual").click();
    h.run_steps(2);
    h.get_by_label("Saved Lighting").click();
    h.run_steps(2);
    h.get_by_label("None saved yet");
    h.get_by_label("Save This Lighting…").click();
    h.run_steps(3);
    h.get_all_by_role(egui::accesskit::Role::TextInput).last().unwrap().type_text("Dusk");
    h.run_steps(2);
    h.get_by_label("Save").click();
    h.run_steps(2);
    assert_eq!(mg_ui::lighting::list(Some(&dir)), ["Dusk"]);
    // Given to the second: its colors are the first's.
    close_tab(h.state_mut(), &Tab::AreaProperties(first));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(second)));
    h.run_steps(3);
    h.get_by_label("Visual").click();
    h.run_steps(2);
    h.get_by_label("Saved Lighting").click();
    h.run_steps(2);
    h.get_by_label("Dusk").click();
    h.run_steps(3);
    assert_eq!(field(&mut h, second, "SunAmbientColor"), Some(mg_gff::Value::Dword(0x12_3456)));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Lighting Dusk"));
}

#[test]
fn area_properties_edit_the_area() {
    let Some((mut h, area)) = area_harness("area-props") else { return };
    let are = |h: &mut Harness<'_, Moonglow>| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().root.clone()
    };
    h.get_by_label("ℹ Area Properties").click();
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&Tab::AreaProperties(area)).is_some());

    // Advanced: the tag.
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    let tag = String::from_utf8_lossy(are(&mut h).string("Tag").unwrap()).into_owned();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&tag).find(is_input).expect("the tag field").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&tag).find(is_input).unwrap().type_text("FIELD_TAG");
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(2);
    assert_eq!(are(&mut h).string("Tag"), Some(&b"FIELD_TAG"[..]));
    // A shader flag past the three, the three kept.
    let flags = are(&mut h).integer("Flags").unwrap();
    h.get_by_label("8").click();
    h.run_steps(2);
    assert_eq!(are(&mut h).integer("Flags"), Some(flags | 8));

    // Visual: a lighting scheme sets the lighting and the tiles' lights in
    // one command.
    h.get_by_label("Visual").click();
    h.run_steps(2);
    let game_row = 2; // environment.2da ExteriorDark
    let name = {
        let game = h.state().game.as_deref().unwrap();
        let t = game.table("environment").unwrap();
        let strref = t.get(game_row, "STRREF").unwrap().trim().parse::<u32>().unwrap();
        game.string(mg_core::StrRef(strref)).unwrap()
    };
    h.get_by_label(&name).click();
    h.run_steps(3);
    let a = are(&mut h);
    assert_eq!(a.integer("LightingScheme"), Some(game_row as i64));
    assert_eq!(a.integer("SunAmbientColor"), Some(0x32_3A_3C), "ExteriorDark's 60, 58, 50");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Lighting scheme"));
    // Always dark.
    h.get_by_label("Always Dark").click();
    h.run_steps(2);
    let a = are(&mut h);
    assert_eq!((a.integer("DayNightCycle"), a.integer("IsNight")), (Some(0), Some(1)));
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-props").join("visual.png")).unwrap();
}

/// An area copied with its tag and its objects': the area's own tag and
/// those the project wants unique are listed, the others counted; Retag
/// gives the copy's their own.
#[test]
fn an_area_copied_with_its_tags_offers_retag() {
    let Some((mut h, area)) = area_harness("copy-area-tags") else { return };
    // (Options › General › Show duplicate tags warning: off by default.)
    h.state_mut().settings.warn_duplicate_tags = true;
    h.state_mut().ws.as_mut().unwrap().module.project_file.tags.unique =
        mg_module::project_file::Patterns::new(&["NW_*"]).unwrap();
    let key = ResKey::new(area, ResType::ARE);
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(key));
    h.run_steps(3);
    h.state_mut().copy_as.as_mut().unwrap().resref = "field_two".into();
    h.run_steps(2);
    h.get_by_label("Create Copy").click();
    h.run_steps(3);
    let clash = h.state().tag_clash.clone().expect("the Duplicate Tags window");
    let tag = |h: &mut Harness<'_, Moonglow>, k: ResKey, path: mg_edit::GffPath| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let doc = ws.doc(&k).unwrap();
        String::from_utf8_lossy(path.get(&doc.root).unwrap().string("Tag").unwrap()).into_owned()
    };
    let area_tag = tag(&mut h, key, mg_edit::GffPath::root());
    let waypoint_tag = tag(
        &mut h,
        ResKey::new(area, ResType::GIT),
        mg_edit::GffPath::root().item("WaypointList", 0),
    );
    let listed: Vec<&str> = clash.clashes.iter().map(|c| c.tag.as_str()).collect();
    assert!(
        listed.contains(&area_tag.as_str()) && listed.contains(&waypoint_tag.as_str()),
        "{listed:?}"
    );
    assert_eq!(clash.objects.len(), 3, "the area and its two waypoints");
    h.get_by_label("Retag").click();
    h.run_steps(2);
    let new = ResRef::from_str("field_two").unwrap();
    let copy_tag = tag(&mut h, ResKey::new(new, ResType::ARE), mg_edit::GffPath::root());
    assert_eq!(copy_tag, format!("{area_tag}_2"));
    let git = ResKey::new(new, ResType::GIT);
    let w: Vec<String> = (0..2)
        .map(|i| tag(&mut h, git, mg_edit::GffPath::root().item("WaypointList", i)))
        .collect();
    assert_eq!(w, [format!("{waypoint_tag}_2"), format!("{waypoint_tag}_3")]);
}

/// An area copied whose tags the project wants unique in each area
/// (`unique_in_area`): the copy's own tag is in another area, as meant,
/// and not listed; its two waypoints of one tag are, in the copy.
#[test]
fn an_area_copied_counts_tags_unique_in_each_area_in_the_copy() {
    let Some((mut h, area)) = area_harness("copy-area-tags-in-area") else { return };
    let key = ResKey::new(area, ResType::ARE);
    let tag = |h: &mut Harness<'_, Moonglow>, k: ResKey, path: mg_edit::GffPath| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let doc = ws.doc(&k).unwrap();
        String::from_utf8_lossy(path.get(&doc.root).unwrap().string("Tag").unwrap()).into_owned()
    };
    let area_tag = tag(&mut h, key, mg_edit::GffPath::root());
    let waypoint_tag = tag(
        &mut h,
        ResKey::new(area, ResType::GIT),
        mg_edit::GffPath::root().item("WaypointList", 0),
    );
    h.state_mut().ws.as_mut().unwrap().module.project_file.tags.unique_in_area =
        mg_module::project_file::Patterns::new(&[area_tag.as_str(), "NW_*"]).unwrap();
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(key));
    h.run_steps(3);
    h.state_mut().copy_as.as_mut().unwrap().resref = "field_two".into();
    h.run_steps(2);
    h.get_by_label("Create Copy").click();
    h.run_steps(3);
    let clash = h.state().tag_clash.clone().expect("the Duplicate Tags window");
    let said: Vec<String> = clash.clashes.iter().map(|c| c.sentence()).collect();
    assert_eq!(
        said,
        [format!(
            "{waypoint_tag} is now on two objects in field_two (the project wants NW_* unique in \
             each area)"
        )]
    );
    assert_eq!(clash.objects.len(), 2, "the copy's two waypoints, not its area");
}

#[test]
fn the_area_wizard_opens_at_the_top_of_its_tilesets() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-area-wizard-top");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.open_palette = false;
    let first = mg_module::new::tilesets(app.game.as_deref().unwrap())[0].name.clone();
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    let shown = |h: &Harness<'_, Moonglow>| {
        let top = h.get_by_label("Tileset").rect().bottom();
        let bottom = h.get_by_label("Size").rect().top();
        let r = h.get_by_label(&first).rect();
        r.top() >= top && r.bottom() <= bottom
    };
    h.state_mut().actions.push(mg_ui::Action::AreaWizard);
    h.run_steps(3);
    assert!(shown(&h), "{first} is at the top of the list");
    // Scrolled to the end, closed and opened again: at the top again.
    let over = h.get_by_label(&first).rect().center();
    h.hover_at(over);
    for _ in 0..20 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -400.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
    h.run_steps(5);
    assert!(!shown(&h), "the list scrolled");
    h.state_mut().wizard = None;
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::AreaWizard);
    h.run_steps(3);
    assert!(shown(&h), "{first} is at the top of the list again");
}

#[test]
fn an_area_is_renamed_from_its_tab() {
    let Some((mut h, area)) = area_harness("rename-tab") else { return };
    h.run_steps(3);
    // Right click on the area's tab: Rename….
    h.get_by_label(&area.to_string()).click_secondary();
    h.run_steps(2);
    h.get_by_label("Rename…").click();
    h.run_steps(2);
    // (The dialog's own typing is another test's.)
    h.state_mut().rename.as_mut().expect("the Rename dialog").to = "meadow".into();
    h.run();
    h.get_by_label("Rename").click();
    h.run_steps(3);
    let meadow = ResRef::from_str("meadow").unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    for t in [ResType::ARE, ResType::GIT] {
        assert!(ws.module.contains(&ResKey::new(meadow, t)), "{t:?} renamed");
        assert!(!ws.module.contains(&ResKey::new(area, t)));
    }
    // The module's area list names it, and its tab and view follow.
    let ifo = ws.module.info().unwrap();
    let areas: Vec<_> = ifo
        .root
        .list("Mod_Area_list")
        .unwrap()
        .iter()
        .filter_map(|a| a.resref("Area_Name"))
        .collect();
    assert_eq!(areas, [meadow]);
    assert!(h.state().dock.find_tab(&Tab::Area(meadow)).is_some());
    assert!(h.state().area_views.contains_key(&meadow));
}

/// Edit › Edit Areas Together…: areas ticked in the chooser (narrowed by
/// its filters) open in one Area Properties, where a change is set on each:
/// a field, a flag (each area keeping its other flags), and the variables
/// added or changed (each keeping its own).
#[test]
fn several_areas_are_edited_together() {
    let dir = mg_testkit::scratch_dir("ui-areas-together");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let key = |name: &str, t| ResKey::parse(name, t).unwrap();
    let int_var = |name: &str, value: i32| {
        let mut s = mg_gff::Struct::new(0);
        s.set("Name", mg_gff::Value::String(name.as_bytes().to_vec()));
        s.set("Type", mg_gff::Value::Dword(1));
        s.set("Value", mg_gff::Value::Int(value));
        s
    };
    // Two caves (underground) and an inn; the caves' flags and variables
    // differ.
    for (name, title, flags, vars) in [
        ("cave1", "Wolf Cave", 0x7u32, vec![int_var("nDepth", 1)]),
        ("cave2", "Bear Cave", 0x3 | 0x100, vec![int_var("nBears", 4)]),
        ("inn", "Apple Inn", 0x1, Vec::new()),
    ] {
        let mut are = Gff::new(*b"ARE ");
        let title = LocString::from_text(Language::ENGLISH, Gender::Male, title);
        are.root.set("Name", mg_gff::Value::LocString(title));
        are.root.set("Flags", mg_gff::Value::Dword(flags));
        are.root.set("DayNightCycle", mg_gff::Value::Byte(1));
        are.root.set("IsNight", mg_gff::Value::Byte(0));
        are.root.set("VarTable", mg_gff::Value::List(vars));
        m.set(key(name, ResType::ARE), are.to_bytes().unwrap());
        let mut git = Gff::new(*b"GIT ");
        git.root.set("AreaProperties", mg_gff::Value::Struct(mg_gff::Struct::new(100)));
        m.set(key(name, ResType::GIT), git.to_bytes().unwrap());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let are = |h: &mut Harness<'_, Moonglow>, name: &str| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::parse(name, ResType::ARE).unwrap()).unwrap().root.clone()
    };

    // The chooser: the underground areas, ticked.
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Edit Areas Together…").click();
    h.run();
    h.get_by_label_contains("Apple Inn");
    h.get_by_value("Above or under ground").click();
    h.run();
    h.get_all_by_label("Underground").last().unwrap().click();
    h.run();
    assert!(h.query_by_label_contains("Apple Inn").is_none(), "the inn is above ground");
    assert!(h.query_by_label("Edit 0 Together").is_some());
    h.get_by_label("Tick Shown").click();
    h.run();
    h.get_by_label("Edit 2 Together").click();
    h.run_steps(3);
    let caves: Vec<ResRef> = ["cave1", "cave2"].map(|n| ResRef::from_str(n).unwrap()).to_vec();
    assert!(h.state().dock.find_tab(&Tab::AreasProperties(caves)).is_some());
    assert!(h.query_by_label("Edit Areas Together").is_none(), "the chooser closed");

    // Visual: always dark, on both caves and not on the inn.
    h.get_by_label("Always Dark").click();
    h.run_steps(2);
    for (name, night) in [("cave1", 1), ("cave2", 1), ("inn", 0)] {
        let a = are(&mut h, name);
        assert_eq!(
            (a.integer("IsNight"), a.integer("DayNightCycle")),
            (Some(night), Some(1 - night))
        );
    }
    // Advanced: no tag or ResRef for several; a flag is cleared on each,
    // which keeps its other flags.
    h.get_all_by_label("Advanced").last().unwrap().click();
    h.run_steps(2);
    assert!(h.query_by_label("ResRef").is_none());
    h.get_by_label("Names, tags and comments are edited one area at a time.");
    h.get_by_label("Above ground").click();
    h.run_steps(2);
    let flags = |h: &mut Harness<'_, Moonglow>, name: &str| are(h, name).integer("Flags");
    assert_eq!(flags(&mut h, "cave1"), Some(0x5));
    assert_eq!(flags(&mut h, "cave2"), Some(0x1 | 0x100));
    assert_eq!(flags(&mut h, "inn"), Some(0x1));
    // Variables: one added is added to each, which keeps its own. (The
    // caves' are in their ARE, as Moonglow wrote them before 1.34.3:
    // opening the window moves them to the GIT, where the game reads
    // them, one step of its own.)
    h.get_by_label("Variables (1)…").click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    {
        let row = h.state_mut().var_edit.as_mut().unwrap().rows.last_mut().unwrap();
        row.name = "nMusic".into();
        row.value = "42".into();
    }
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run_steps(2);
    let names = |h: &mut Harness<'_, Moonglow>, name: &str| -> Vec<String> {
        let key = ResKey::new(ResRef::from_str(name).unwrap(), ResType::GIT);
        let a = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().root.clone();
        let left = are(h, name).list("VarTable").map_or(0, <[_]>::len);
        assert_eq!(left, 0, "{name}: none left in the ARE");
        let vars = a.list("VarTable").unwrap_or(&[]);
        vars.iter()
            .map(|v| String::from_utf8_lossy(v.string("Name").unwrap()).into_owned())
            .collect()
    };
    assert_eq!(names(&mut h, "cave1"), ["nDepth", "nMusic"]);
    assert_eq!(names(&mut h, "cave2"), ["nBears", "nMusic"]);
    assert!(names(&mut h, "inn").is_empty());
    // One undo takes the variable from both.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert_eq!(names(&mut h, "cave2"), ["nBears"]);
    assert_eq!(names(&mut h, "cave1"), ["nDepth"]);
}

/// Give Every Area This Page's Visual Settings, on areas edited together:
/// an area of another lighting scheme gets the first's, and its tiles'
/// lights from that scheme's colors, as choosing the scheme gives them;
/// one of the same scheme keeps its tiles' own. One step to undo.
#[test]
fn every_area_is_given_the_scheme_s_tile_lights() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-areas-scheme");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let key = |name: &str, t| ResKey::parse(name, t).unwrap();
    // ExteriorEvil (environment.2da 5): source lights 12 or 8.
    let evil: u8 = 5;
    for (name, title, scheme) in
        [("cave1", "Wolf Cave", evil), ("cave2", "Bear Cave", 0), ("cave3", "Bat Cave", evil)]
    {
        let mut are = Gff::new(*b"ARE ");
        let title = LocString::from_text(Language::ENGLISH, Gender::Male, title);
        are.root.set("Name", mg_gff::Value::LocString(title));
        are.root.set("LightingScheme", mg_gff::Value::Byte(scheme));
        let tiles = (0..4)
            .map(|_| {
                let mut t = mg_gff::Struct::new(1);
                for l in mg_module::new::TILE_LIGHT_FIELDS {
                    t.set(l, mg_gff::Value::Byte(31));
                }
                t
            })
            .collect();
        are.root.set("Tile_List", mg_gff::Value::List(tiles));
        m.set(key(name, ResType::ARE), are.to_bytes().unwrap());
        let mut git = Gff::new(*b"GIT ");
        git.root.set("AreaProperties", mg_gff::Value::Struct(mg_gff::Struct::new(100)));
        m.set(key(name, ResType::GIT), git.to_bytes().unwrap());
    }
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let caves: Vec<ResRef> =
        ["cave1", "cave2", "cave3"].map(|n| ResRef::from_str(n).unwrap()).to_vec();
    app.actions.push(mg_ui::Action::OpenTab(Tab::AreasProperties(caves)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let sources = |h: &mut Harness<'_, Moonglow>, name: &str| -> (Option<i64>, Vec<i64>) {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let a = ws.doc(&ResKey::parse(name, ResType::ARE).unwrap()).unwrap().root.clone();
        let tiles = a.list("Tile_List").unwrap();
        (
            a.integer("LightingScheme"),
            tiles.iter().map(|t| t.integer("Tile_SrcLight1").unwrap()).collect(),
        )
    };
    let before = sources(&mut h, "cave2");
    h.get_by_label("Visual").click();
    h.run_steps(2);
    h.get_by_label("Give Every Area This Page's Visual Settings").click();
    h.run_steps(3);
    let (scheme, lights) = sources(&mut h, "cave2");
    assert_eq!(scheme, Some(i64::from(evil)));
    assert!(lights.iter().all(|l| [12, 8].contains(l)), "the scheme's source lights: {lights:?}");
    assert_eq!(
        sources(&mut h, "cave3"),
        (Some(i64::from(evil)), vec![31; 4]),
        "the same scheme: its own"
    );
    // (Rendering opens a GPU device.)
    mg_testkit::gpu::hold();
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-areas-scheme").join("visual.png")).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert_eq!(sources(&mut h, "cave2"), before, "one undo");
}

/// Copy… in the module tree copies an area with what is placed in it,
/// under the ResRef and Tag given, and the module lists it.
#[test]
fn an_area_is_copied_with_its_objects() {
    let Some((mut h, area)) = area_harness("copy-area") else { return };
    let key = ResKey::new(area, ResType::ARE);
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(key));
    h.run_steps(3);
    let draft = h.state_mut().copy_as.as_mut().expect("the Copy window");
    draft.resref = "start_two".into();
    draft.tag = Some("StartTwo".into());
    h.run_steps(2);
    h.get_by_label("Create Copy").click();
    h.run_steps(3);
    let new = ResRef::from_str("start_two").unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let are = ws.doc(&ResKey::new(new, ResType::ARE)).unwrap().root.clone();
    assert_eq!(are.resref("ResRef"), Some(new));
    assert_eq!(are.string("Tag"), Some(&b"StartTwo"[..]));
    let placed = |ws: &mut mg_edit::Workspace, a: ResRef| {
        let git = ws.doc(&ResKey::new(a, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, <[mg_gff::Struct]>::len)
    };
    assert_eq!(placed(ws, new), placed(ws, area));
    assert!(placed(ws, new) > 0);
    let ifo = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
    let listed: Vec<ResRef> = ifo
        .root
        .list("Mod_Area_list")
        .unwrap()
        .iter()
        .filter_map(|a| a.resref("Area_Name"))
        .collect();
    assert_eq!(listed, [area, new]);
    // A name the module has is refused.
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(key));
    h.run_steps(3);
    h.state_mut().copy_as.as_mut().unwrap().resref = "start_two".into();
    h.run_steps(2);
    assert!(h.query_by_label_contains("has a start_two already").is_some());
    // One undo takes the copy away.
    h.state_mut().copy_as = None;
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    let ws = h.state().ws.as_ref().unwrap();
    assert!(!ws.module.contains(&ResKey::new(new, ResType::ARE)));
    assert!(!ws.module.contains(&ResKey::new(new, ResType::GIT)));
}

/// An object selected in the area's view is shown in the module tree
/// where its area is opened out there: its kind's list opened, its row
/// marked. An area left closed in the tree is opened out to it, as in
/// Aurora, or with the option off stays closed.
/// An area's variables are its GIT's, where the game reads them
/// (`engine_area_variables`): the Advanced page's Variables… edits those,
/// and moves there what an earlier Moonglow wrote into the ARE (a
/// builder's report: an area's variable read 0 in its OnEnter).
#[test]
fn an_area_s_variables_are_kept_in_its_git() {
    use mg_gff::{Struct, Value};
    let Some((mut h, area)) = area_harness("area-variables") else { return };
    let (are, git) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let var = |name: &str, value: i32| {
        let mut s = Struct::new(0);
        s.set("Name", Value::String(name.as_bytes().to_vec()));
        s.set("Type", Value::Dword(1));
        s.set("Value", Value::Int(value));
        s
    };
    let set = mg_edit::Edit::SetField {
        key: are,
        path: mg_edit::GffPath::root(),
        label: "VarTable".into(),
        value: Some(Value::List(vec![var("iNoMap", 1)])),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Old", vec![set])));
    let vars = |h: &mut Harness<'_, Moonglow>, key: ResKey| -> Option<Vec<Struct>> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&key).unwrap().root.list("VarTable").map(<[_]>::to_vec)
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    h.run_steps(3);
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    h.get_by_label("Variables (1)…").click();
    h.run_steps(3);
    assert_eq!(vars(&mut h, git), Some(vec![var("iNoMap", 1)]), "moved to the GIT");
    assert_eq!(vars(&mut h, are), None, "and gone from the ARE");
    assert_eq!(h.state().var_edit.as_ref().unwrap().target.key, git);
    // A variable added: in the GIT.
    h.state_mut()
        .var_edit
        .as_mut()
        .unwrap()
        .rows
        .push(mg_ui::widgets::VarRow::new("iMore", 1, "2"));
    h.run_steps(2);
    h.get_all_by_label("OK").last().unwrap().click();
    h.run_steps(3);
    assert!(h.state().var_edit.is_none(), "the window closed");
    assert_eq!(vars(&mut h, git), Some(vec![var("iNoMap", 1), var("iMore", 2)]));
    assert_eq!(vars(&mut h, are), None);
}

/// Saving a module folder without changing anything must leave every
/// file untouched, even after opening an area, its properties, the
/// blueprint and instance editors, and reading every area as Update
/// Instances does. Previously, every document that had been read was
/// re-serialized on save, so git reported each one as changed.
#[test]
fn a_module_folder_saved_with_nothing_changed_is_as_it_was() {
    let root = mg_testkit::corpus!();
    let folder = mg_testkit::scratch_dir("ui-folder-untouched").join("chess");
    let _ = std::fs::remove_dir_all(&folder);
    // BioWare's own module, so the files have Aurora's byte layout.
    let chess = Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    chess.clone().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let files = || -> std::collections::BTreeMap<String, Vec<u8>> {
        (std::fs::read_dir(&folder).unwrap().flatten())
            .map(|e| {
                (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap())
            })
            .collect()
    };
    let before = files();
    let mut app = game_app(&root, None);
    app.open_module(&folder);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let area = ResRef::from_str("chess").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    h.run();
    // Open every blueprint's editor, and the editor for the first placed
    // object of each kind.
    let blueprints: Vec<ResKey> = (chess.keys())
        .filter(|k| mg_rules::ObjectKind::from_restype(k.restype).is_some())
        .copied()
        .collect();
    let git = chess.gff(&ResKey::new(area, ResType::GIT)).unwrap().unwrap();
    let placed = mg_rules::ObjectKind::ALL
        .map(mg_rules::ObjectKind::list)
        .into_iter()
        .filter(|list| git.root.list(list).is_some_and(|l| !l.is_empty()))
        .map(|list| Tab::Instance { area, path: mg_edit::GffPath::root().item(list, 0) });
    let tabs: Vec<Tab> = blueprints.iter().map(|k| Tab::Blueprint(*k)).chain(placed).collect();
    assert!(tabs.len() > 3, "{tabs:?}");
    for tab in tabs {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run();
    }
    let ws = h.state_mut().ws.as_mut().unwrap();
    for area in chess.areas().unwrap() {
        for t in [ResType::ARE, ResType::GIT, ResType::GIC] {
            let _ = ws.doc(&ResKey::new(area, t));
        }
    }
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let after = files();
    let changed: std::collections::BTreeSet<&String> = (before.keys().chain(after.keys()))
        .filter(|name| before.get(*name) != after.get(*name))
        .collect();
    assert!(changed.is_empty(), "files were rewritten although nothing changed: {changed:?}");
}
