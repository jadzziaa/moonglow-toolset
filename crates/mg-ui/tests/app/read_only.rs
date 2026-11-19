//! The read-only resource tab (M4's U23, the Resources board): its
//! header (where the resource is from, its kind, Copy to Module and Save
//! As…), a GFF's fields in three columns with names beside numbers as
//! `mg gff --names` gives them, its rows in sight alone, its counts at the
//! Status Bar's right; nothing in it edited, Copy to Module one Undo step.

use crate::common::*;
use mg_gff::{Field, Label, Struct, Value};

fn chicken() -> ResKey {
    ResKey::parse("nw_chicken", ResType::UTC).unwrap()
}

/// A field of the authored GFFs.
fn field(label: &str, value: Value) -> Field {
    Field { label: Label::new(label).unwrap(), value }
}

/// The game's data with a module open and a user folder whose override
/// holds `authored` (each resource's name and its GFF), the window `width`
/// by 900, `key` open read-only in front. `None` without the game.
fn opened(
    name: &str,
    width: f32,
    authored: &[(&str, mg_gff::Gff)],
    key: ResKey,
) -> Option<Harness<'static, Moonglow>> {
    opened_in(Mode::Tabs, name, width, authored, key, None)
}

/// [`opened`] with the editors opening as `mode` says (floating: the
/// tab in a window of its own over the main window), and where `tlk` is
/// given, the module's talk table (`mg_names.tlk` in the user's tlk
/// folder, the module naming it).
fn opened_in(
    mode: Mode,
    name: &str,
    width: f32,
    authored: &[(&str, mg_gff::Gff)],
    key: ResKey,
    tlk: Option<&mg_tlk::Tlk>,
) -> Option<Harness<'static, Moonglow>> {
    let root = mg_testkit::nwn_root()?;
    let dir = mg_testkit::scratch_dir(name);
    let user = user_folder(&dir, &["override", "tlk"]);
    for (file, gff) in authored {
        std::fs::write(user.join("override").join(file), gff.to_bytes().unwrap()).unwrap();
    }
    let path = sample_module(&dir);
    if let Some(tlk) = tlk {
        std::fs::write(user.join("tlk/mg_names.tlk"), tlk.to_bytes().unwrap()).unwrap();
        let mut m = mg_module::Module::open(&path).unwrap();
        let mut info = m.info().unwrap();
        info.root.write(&ifo::MOD_CUSTOM_TLK, ExoString::from("mg_names"));
        m.set_info(&info).unwrap();
        m.save().unwrap();
    }
    let mut app = game_app(&root, Some(user));
    app.settings = mode.settings();
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, width, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    settle(&mut h);
    Some(h)
}

/// A row of the GFF view by its name: the node, its value as the view
/// gives it for a screen reader ("type, value meaning").
fn row<'h>(h: &'h Harness<'_, Moonglow>, name: &'h str) -> egui_kittest::Node<'h> {
    let tab = h.state().editor_area_rect().expect("the editor area");
    (h.get_all_by_label(name))
        .find(|n| tab.contains(n.rect().center()))
        .unwrap_or_else(|| panic!("no row {name}"))
}

/// The view scrolled down by the wheel, `pages` pages.
fn scroll_view(h: &mut Harness<'_, Moonglow>, pages: usize) {
    let view = h.state().editor_area_rect().unwrap();
    for _ in 0..pages {
        h.event(egui::Event::PointerMoved(view.center()));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, -1.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        h.run();
    }
}

/// The row the header named `name` stands in.
fn header_row(h: &Harness<'_, Moonglow>, name: &str) -> egui::Rect {
    mg_uikit::header::records(&h.ctx, name).first().expect("the header drawn").row
}

fn said(n: &egui_kittest::Node<'_>) -> String {
    n.accesskit_node().value().unwrap_or_default()
}

/// What `mg gff --names` gives a field (the same code: `meaning` and
/// `meaning_name`), for the test to compare with the view's.
fn mg_name(h: &Harness<'_, Moonglow>, path: &[&str], label: &str, value: i64) -> String {
    let game = h.state().game.as_deref().unwrap();
    let m = mg_rules::rows::meaning(Some(game), "UTC ", path, label, &|_| None).expect("a meaning");
    game.meaning_name(&m, &[], value).expect("a name")
}

/// The chicken's fields in three columns: a number with its row's name
/// beside it, as `mg gff --names` names it, a localized string with its
/// talk-table text, a list folded with its count, unfolded to its items
/// and an item's fields; a number whose row or string is missing says
/// so; a faction's number is shown bare (no module's factions named, as
/// `mg gff --names` without a module).
#[test]
fn the_gff_view_names_numbers_as_mg_gff_names_does() {
    let mut gff = mg_gff::Gff::new(*b"UTC ");
    gff.root.fields.push(field("Appearance_Type", Value::Word(65000)));
    let ls = mg_core::LocString { strref: mg_core::StrRef(16_000_000), ..Default::default() };
    gff.root.fields.push(field("FirstName", Value::LocString(ls)));
    gff.root.fields.push(field("FactionID", Value::Word(2)));
    // (Values that mean none, the field's largest; a string of the
    // module's talk table.)
    gff.root.fields.push(field("Race", Value::Byte(255)));
    gff.root.fields.push(field("SoundSetFile", Value::Word(65535)));
    let custom = mg_core::LocString { strref: mg_core::StrRef::custom(5), ..Default::default() };
    gff.root.fields.push(field("LastName", Value::LocString(custom)));
    let Some(mut h) = opened("ui-u23-names", 1600.0, &[], chicken()) else { return };
    let appearance = said(&row(&h, "Appearance_Type"));
    let name = mg_name(&h, &[], "Appearance_Type", 31);
    assert_eq!(appearance, format!("word, 31  {name}"));
    let first = said(&row(&h, "FirstName"));
    assert!(first.starts_with("cexolocstring, StrRef 12416  Chicken"), "{first}");
    assert_eq!(said(&row(&h, "FactionID")), "word, 2", "a faction bare");
    assert_eq!(said(&row(&h, "FeatList")), "list, 1 item");
    row(&h, "FeatList").click();
    settle(&mut h);
    assert_eq!(said(&row(&h, "[0]")), "struct 1, 1 field");
    assert_eq!(row(&h, "FeatList").accesskit_node().data().is_expanded(), Some(true));
    row(&h, "[0]").click();
    settle(&mut h);
    let feat = said(&row(&h, "Feat"));
    let name = mg_name(&h, &["FeatList"], "Feat", 289);
    assert_eq!(feat, format!("word, 289  {name}"));
    // (A row the game's portraits.2da has with no name: none too.)
    let blank = {
        let game = h.state().game.as_deref().unwrap();
        let m = mg_rules::rows::meaning(Some(game), "UTC ", &[], "PortraitId", &|_| None).unwrap();
        let rows = game.table("portraits").unwrap().len() as i64;
        (1..rows).find(|&r| game.meaning_name(&m, &[], r).is_none()).expect("a blank row")
    };
    gff.root.fields.push(field("PortraitId", Value::Word(blank as u16)));
    // Missing: what is shown.
    let key = ResKey::parse("mg_missing", ResType::UTC).unwrap();
    let Some(h) = opened("ui-u23-missing", 1600.0, &[("mg_missing.utc", gff.clone())], key) else {
        return;
    };
    assert_eq!(said(&row(&h, "Appearance_Type")), "word, 65000  (no row 65000 in appearance.2da)");
    assert_eq!(said(&row(&h, "FirstName")), "cexolocstring, StrRef 16000000  (no string 16000000)");
    assert_eq!(said(&row(&h, "FactionID")), "word, 2");
    // None: nothing beside it, as `mg gff --names` prints nothing.
    assert_eq!(said(&row(&h, "Race")), "byte, 255", "the field's largest");
    assert_eq!(said(&row(&h, "SoundSetFile")), "word, 65535", "the field's largest");
    assert_eq!(said(&row(&h, "PortraitId")), format!("word, {blank}"), "a row with no name");
    let custom = said(&row(&h, "LastName"));
    let words = "(string 5 of the module's talk table, not read here)";
    assert!(custom.ends_with(words), "no talk table read: {custom}");
    // With the module's talk table read, its text.
    let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
    tlk.entries = (0..6).map(|i| mg_tlk::TlkEntry::text(format!("Ours {i}"))).collect();
    let key = ResKey::parse("mg_missing", ResType::UTC).unwrap();
    let authored = [("mg_missing.utc", gff.clone())];
    let Some(h) = opened_in(Mode::Tabs, "ui-u23-custom", 1600.0, &authored, key, Some(&tlk)) else {
        return;
    };
    assert!(h.state().game.as_deref().unwrap().custom_tlk().is_some(), "the module's talk table");
    let custom = said(&row(&h, "LastName"));
    assert!(custom.ends_with("  Ours 5"), "{custom}");
}

/// A GFF of 50,000 fields: the view draws the rows in sight alone, folded
/// and unfolded and scrolled, and lists its shown rows once a fold, never
/// in a frame that changes nothing; the folds stay while the tab lives,
/// another tab in front and back.
#[test]
fn a_big_gff_draws_its_rows_in_sight_and_lists_them_once_a_fold() {
    let mut gff = mg_gff::Gff::new(*b"UTC ");
    let items: Vec<Struct> = (0..24_950)
        .map(|i| Struct {
            id: 7,
            fields: vec![field("A", Value::Int(i)), field("B", Value::Int(1))],
        })
        .collect();
    gff.root.fields.push(field("Big", Value::List(items)));
    for i in 0..100 {
        gff.root.fields.push(field(&format!("F{i}"), Value::Int(i)));
    }
    let key = ResKey::parse("mg_big", ResType::UTC).unwrap();
    let Some(mut h) = opened("ui-u23-big", 1600.0, &[("mg_big.utc", gff)], key) else { return };
    let counts = |h: &Harness<'_, Moonglow>| h.state().read_only_counts();
    // (The rows a frame can show: the view's height over 22, and a row
    // at each end.)
    let most = (900.0 / 22.0) as usize + 2;
    let steady = |h: &mut Harness<'_, Moonglow>, what: &str| {
        rest(h);
        let (walks, drawn) = counts(h);
        assert!(drawn > 0 && drawn <= most, "{what}: {drawn} rows drawn");
        h.run_steps(10);
        assert_eq!(counts(h).0, walks, "{what}: listed again with nothing changed");
        walks
    };
    let folded = steady(&mut h, "folded");
    // Unfolded: 25,051 rows, those in sight drawn.
    row(&h, "Big").click();
    let unfolded = steady(&mut h, "unfolded");
    assert_eq!(unfolded, folded + 1, "listed once for the fold");
    // Scrolled far down: still those in sight.
    scroll_view(&mut h, 20);
    assert_eq!(steady(&mut h, "scrolled"), unfolded, "a scroll lists nothing");
    assert!(h.query_by_label("Big").is_none(), "scrolled past the top");
    // Another tab in front and back: still unfolded.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(chicken())));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    settle(&mut h);
    let (_, drawn) = counts(&h);
    assert!(drawn <= most, "{drawn}");
    let items = h.query_all_by_label_contains("[").count();
    assert!(
        items > 0,
        "the list's items still shown: {:?} {:?}",
        h.state().active_editor(),
        counts(&h)
    );
}

/// The header (the Resources board's): the lock and where the resource is
/// from, its kind, Copy to Module and Save As…, every one reachable on the
/// row or in More from 1600 points down to 560 at three text sizes; a
/// table's Edit… menu too. Copy to Module is off where the module has the
/// resource, with a tip that says so.
#[test]
fn the_read_only_header_holds_its_controls() {
    let Some(mut h) = opened("ui-u23-header", 1600.0, &[], chicken()) else { return };
    let header = mg_ui::RESOURCE_HEADER;
    assert!(h.get_by_label(header).query_by_label("OK").is_none(), "no OK");
    let names = ["Read-only: from the game's data", "UTC file", "Copy to Module", "Save As…"];
    let mut compact_seen = 0;
    for scale in [1.0_f32, 1.25, 1.5] {
        h.ctx.set_zoom_factor(scale);
        for width in [1600.0, 1200.0, 900.0, 700.0, 560.0] {
            h.set_size(egui::vec2(width, 900.0));
            settle(&mut h);
            header_holds(&mut h, header, &names);
            // (The lock alone where the row is narrowest: at 1600 with
            // both docks open and the text larger. Narrower windows fold
            // the docks, which leaves the tab more room.)
            let from = h.get_by_label(header).get_by_label("Read-only: from the game's data");
            let alone = from.accesskit_node().role() == egui::accesskit::Role::Image;
            assert_eq!(alone, scale > 1.0 && width == 1600.0, "the lock alone at {width} x{scale}");
            if alone {
                compact_seen += 1;
                from.hover();
                h.run_steps(60);
                // (The count, which the Status Bar shows too, in the tip.)
                let tip = "from key:nwn_base, 3078 bytes, shown as the game reads it";
                let has = |n: &egui_kittest::Node<'_>| {
                    let a = n.accesskit_node();
                    a.value().unwrap_or_default().contains(tip)
                        || a.label().unwrap_or_default().contains(tip)
                };
                let tips = h.query_all(egui_kittest::kittest::by()).filter(has).count();
                assert!(tips > 0, "the lock's tip with the count at {width} x{scale}");
                h.hover_at(egui::pos2(width / 2.0, 880.0));
                h.run_steps(5);
            }
        }
    }
    assert_eq!(compact_seen, 2, "the lock alone reached at 1600, x1.25 and x1.5");
    h.ctx.set_zoom_factor(1.0);
    h.set_size(egui::vec2(1600.0, 900.0));
    let table = ResKey::parse("classes", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(table)));
    settle(&mut h);
    for width in [1600.0, 700.0] {
        h.set_size(egui::vec2(width, 900.0));
        settle(&mut h);
        let names = ["Read-only: from the game's data", "2DA file", "Edit…", "Copy to Module"];
        header_holds(&mut h, header, &names);
        header_holds(&mut h, header, &["Save As…"]);
    }
    // In the module already: Copy to Module off, said why.
    h.set_size(egui::vec2(1600.0, 900.0));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(chicken())));
    settle(&mut h);
    h.get_by_label(header).get_by_label("Copy to Module").click();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(chicken())));
    settle(&mut h);
    let copy = h.get_by_label(header).get_by_label("Copy to Module");
    assert!(copy.accesskit_node().is_disabled(), "the module has its own");
    copy.hover();
    h.run_steps(60);
    h.get_by_label_contains("The module has its own nw_chicken.utc");
}

/// Nothing in the tab can be changed: no field, box or switch in its
/// view, and folding, unfolding and copying a value from its menu make no
/// Undo step; Copy to Module makes one, on the module, which Undo takes
/// back.
#[test]
fn nothing_in_the_tab_is_edited_and_copy_to_module_is_one_undo_step() {
    use egui::accesskit::Role;
    let Some(mut h) = opened("ui-u23-undo", 1600.0, &[], chicken()) else { return };
    let undo_of =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    let was = undo_of(&h);
    let view = h.state().editor_area_rect().unwrap();
    let below_header = header_row(&h, mg_ui::RESOURCE_HEADER).bottom();
    for role in [Role::TextInput, Role::CheckBox, Role::ComboBox, Role::SpinButton, Role::Slider] {
        let found = (h.query_all_by_role(role))
            .filter(|n| view.contains(n.rect().center()) && n.rect().top() > below_header)
            .count();
        assert_eq!(found, 0, "{role:?} in the read-only view");
    }
    row(&h, "FeatList").click();
    settle(&mut h);
    row(&h, "Tag").click_secondary();
    settle(&mut h);
    h.get_by_label("Copy Value").click();
    settle(&mut h);
    assert_eq!(undo_of(&h), was, "no Undo step from the view");
    h.get_by_label(mg_ui::RESOURCE_HEADER).get_by_label("Copy to Module").click();
    settle(&mut h);
    assert!(h.state().ws.as_ref().unwrap().module.contains(&chicken()));
    assert_eq!(undo_of(&h).as_deref(), Some("Copy nw_chicken.utc to the module"), "one step");
    h.state_mut().run(mg_ui::Action::Undo);
    settle(&mut h);
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&chicken()), "Undo takes it back");
    assert_eq!(undo_of(&h), was);
}

/// Copy to Module on a hak's copy of a table (the lower of two haks'):
/// the module gets that hak's bytes, not the copy the game reads, in one
/// Undo step. A kind with no editor (a
/// texture): it lands in the module, one Undo step, no tab for it, and
/// the saved module keeps it.
#[test]
fn copy_to_module_takes_the_copy_shown_and_any_kind() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-u23-copy-hak");
    let user = dir.join("user");
    let path = crate::resources::module_with_two_tables(&dir, &user);
    let mut app = game_app(&root, Some(user.clone()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let key = ResKey::parse("portraits", ResType::TWODA).unwrap();
    let undo_of =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    let was = undo_of(&h);
    let tab = Tab::ResourceCopy { key, layer: "hak:mg_low".into() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    let header = mg_ui::RESOURCE_HEADER;
    h.get_by_label(header).get_by_label("Edit in mg_low.hak");
    h.get_by_label(header).get_by_label("Copy to Module").click();
    settle(&mut h);
    let module = |h: &Harness<'_, Moonglow>, k: &ResKey| {
        h.state().ws.as_ref().unwrap().module.get(k).map(|d| d.to_vec())
    };
    let low = b"2DA V2.0\n\n LABEL\n0 mg_low\n".to_vec();
    assert_eq!(module(&h, &key), Some(low), "the lower hak's copy, as shown");
    assert_eq!(undo_of(&h).as_deref(), Some("Copy portraits.2da to the module"), "one step");
    h.state_mut().run(mg_ui::Action::Undo);
    settle(&mut h);
    assert!(module(&h, &key).is_none(), "Undo takes it back");
    assert_eq!(undo_of(&h), was);
    // A texture: no editor of its own.
    let tga = {
        let game = h.state().game.as_deref().unwrap();
        let r = *game.resman.list(ResType::TGA).first().expect("a texture in the game's data");
        ResKey::new(r, ResType::TGA)
    };
    assert_eq!(Tab::for_resource(tga), None, "a kind with no editor");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab]));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(tga)));
    settle(&mut h);
    let before = h.state().active_editor().cloned();
    h.get_by_label(header).get_by_label("Copy to Module").click();
    settle(&mut h);
    let bytes = h.state().game.as_deref().unwrap().resman.get(&tga).unwrap().to_vec();
    assert_eq!(module(&h, &tga).as_ref(), Some(&bytes), "in the module");
    assert_eq!(undo_of(&h), Some(format!("Copy {tga} to the module")), "one step");
    assert_eq!(h.state().active_editor().cloned(), before, "no tab opened for it");
    h.state_mut().run(mg_ui::Action::Save);
    settle(&mut h);
    let saved = mg_module::Module::open(&path).unwrap();
    assert_eq!(saved.get(&tga).map(|d| d.to_vec()), Some(bytes), "the saved module keeps it");
}

/// A module kept as files cannot hold a kind with no file extension (its
/// file would be named by its number, which no folder reads back): Copy
/// to Module is off on one there, a tip saying why; a module archive
/// takes it.
#[test]
fn copy_to_module_is_off_for_a_kind_a_module_folder_cannot_hold() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-u23-copy-kind");
    let user = user_folder(&dir, &["hak"]);
    let none = (3000..u16::MAX).map(ResType).find(|t| t.extension().is_none()).unwrap();
    let odd = ResKey::new(ResRef::from_str("mg_odd").unwrap(), none);
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    w.add(odd.resref, odd.restype, b"odd bytes".to_vec()).unwrap();
    std::fs::write(user.join("hak/mg_odd.hak"), w.to_bytes().unwrap()).unwrap();
    let archive = sample_module(&dir);
    let mut m = Module::open(&archive).unwrap();
    let mut info = m.info().unwrap();
    let mut s = ifo::MOD_HAK_LIST.new_item();
    s.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from("mg_odd"));
    info.root.items_mut(&ifo::MOD_HAK_LIST).push(s);
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let folder = dir.join("mg_folder");
    Module::open(&archive).unwrap().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    // (Why: put in the folder module and saved, it is not there again.)
    let lost = dir.join("mg_lost");
    let mut m = Module::open(&archive).unwrap();
    m.set(odd, &b"odd bytes"[..]);
    m.save_as(&ModuleLocation::Folder(lost.clone())).unwrap();
    assert_eq!(Module::open(&lost).unwrap().get(&odd), None, "a module folder loses it");
    let header = mg_ui::RESOURCE_HEADER;
    for (path, takes) in [(&folder, false), (&archive, true)] {
        let mut app = game_app(&root, Some(user.clone()));
        app.settings.no_last_area = true;
        app.open_module(path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        let tab = Tab::ResourceCopy { key: odd, layer: "hak:mg_odd".into() };
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        settle(&mut h);
        let copy = h.get_by_label(header).get_by_label("Copy to Module");
        assert_eq!(copy.accesskit_node().is_disabled(), !takes, "{}", path.display());
        if !takes {
            copy.hover();
            h.run_steps(60);
            h.get_by_label_contains("A module kept as files cannot hold this kind");
            continue;
        }
        copy.click();
        settle(&mut h);
        let module = &h.state().ws.as_ref().unwrap().module;
        assert_eq!(module.get(&odd), Some(&b"odd bytes"[..]), "the archive takes it");
    }
}

/// What was copied to the clipboard in the frame just run.
fn copied(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.output().platform_output.commands.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    })
}

/// The row `name`'s menu (by a right click, or by Shift+F10 with the row
/// holding the keyboard when `keyed`): its items' names, and what its
/// item `item` copies to the clipboard.
fn menu_copy(
    h: &mut Harness<'_, Moonglow>,
    name: &str,
    keyed: bool,
    item: &str,
) -> (Vec<String>, String) {
    if keyed {
        row(h, name).focus();
        h.step();
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    } else {
        row(h, name).click_secondary();
    }
    settle(h);
    let items: Vec<String> = (h.query_all_by_role(egui::accesskit::Role::Button))
        .filter_map(|n| n.accesskit_node().label().filter(|l| l.starts_with("Copy ")))
        .filter(|l| l != "Copy to Module")
        .collect();
    h.get_by_label(item).click();
    h.step();
    let text = copied(h).unwrap_or_else(|| panic!("{item} of {name} copied nothing"));
    settle(h);
    (items, text)
}

/// Copy Value copies the value itself (M4's U23 fix): a string with its
/// newlines, a number, a resource name, a void's bytes as hex; a
/// localized string says what it copies (its text in the editing
/// language, or its StrRef), Copy All Languages where it has more than
/// one; Copy Meaning where the row has one. Ctrl+C on the row holding the
/// keyboard copies its value; Shift+F10 opens its menu.
#[test]
fn copy_value_copies_the_value_itself() {
    use mg_core::{Gender, Language, LocString, LocStringKey, StrRef};
    let mut gff = mg_gff::Gff::new(*b"UTC ");
    gff.root.fields.push(field("Appearance_Type", Value::Word(31)));
    gff.root.fields.push(field("Comment", Value::String(b"line one\nline \"two\"".to_vec())));
    gff.root.fields.push(field("TemplateResRef", Value::ResRef(b"mg_hen".to_vec())));
    gff.root.fields.push(field("Blob", Value::Void(vec![0xde, 0xad, 0x01])));
    let both = LocString {
        strref: StrRef::NONE,
        strings: vec![
            (LocStringKey::new(Language(0), Gender::Male), b"Hen".to_vec()),
            (LocStringKey::new(Language(1), Gender::Female), b"Poule".to_vec()),
        ],
    };
    gff.root.fields.push(field("FirstName", Value::LocString(both)));
    let talk = LocString { strref: StrRef(12416), ..Default::default() };
    gff.root.fields.push(field("LastName", Value::LocString(talk)));
    let key = ResKey::parse("mg_copy", ResType::UTC).unwrap();
    let Some(mut h) = opened("ui-u23-copy", 1600.0, &[("mg_copy.utc", gff)], key) else { return };
    let meaning = mg_name(&h, &[], "Appearance_Type", 31);
    let (items, text) = menu_copy(&mut h, "Appearance_Type", false, "Copy Value");
    assert_eq!(items, ["Copy Value", "Copy Meaning", "Copy Name"]);
    assert_eq!(text, "31");
    assert_eq!(menu_copy(&mut h, "Appearance_Type", true, "Copy Meaning").1, meaning);
    // By the keys alone: Shift+F10 opens it with its first item holding
    // the keyboard, Tab goes to the next, Enter copies; Escape closes it
    // with the keyboard back on the row.
    let focused = |h: &Harness<'_, Moonglow>| h.ctx.memory(|m| m.focused());
    row(&h, "Appearance_Type").focus();
    h.step();
    let the_row = focused(&h).expect("the row has the keyboard");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    settle(&mut h);
    let first = h.get_by_label("Copy Value").accesskit_node().is_focused();
    assert!(first, "the first item has the keyboard");
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert!(h.get_by_label("Copy Meaning").accesskit_node().is_focused(), "Tab: the next");
    // (Each queued event is a pass of its own, and the output is the last
    // pass's: the press is stepped alone to read what it copied.)
    let enter = |pressed| egui::Event::Key {
        key: egui::Key::Enter,
        pressed,
        modifiers: egui::Modifiers::NONE,
        repeat: false,
        physical_key: None,
    };
    h.event(enter(true));
    h.step();
    let on_enter = copied(&h);
    h.event(enter(false));
    h.step();
    assert_eq!(on_enter.as_deref(), Some(meaning.as_str()), "Enter copies");
    settle(&mut h);
    assert!(h.query_by_label("Copy Meaning").is_none(), "and closes it");
    row(&h, "Appearance_Type").focus();
    h.step();
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    settle(&mut h);
    assert!(h.query_by_label("Copy Value").is_some(), "open again");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Copy Value").is_none(), "Escape closes it");
    assert_eq!(focused(&h), Some(the_row), "the keyboard back on the row");
    let (items, text) = menu_copy(&mut h, "Comment", false, "Copy Value");
    assert_eq!(items, ["Copy Value", "Copy Name"]);
    assert_eq!(text, "line one\nline \"two\"");
    assert_eq!(menu_copy(&mut h, "TemplateResRef", false, "Copy Value").1, "mg_hen");
    assert_eq!(menu_copy(&mut h, "Blob", false, "Copy Value").1, "dead01");
    let (items, text) = menu_copy(&mut h, "FirstName", true, "Copy English Text");
    assert_eq!(items, ["Copy English Text", "Copy All Languages", "Copy Name"]);
    assert_eq!(text, "Hen");
    let all = menu_copy(&mut h, "FirstName", false, "Copy All Languages").1;
    assert_eq!(all, "English: Hen\nFrench (female): Poule");
    let (items, text) = menu_copy(&mut h, "LastName", false, "Copy StrRef");
    assert_eq!(items, ["Copy StrRef", "Copy Meaning", "Copy Name"]);
    assert_eq!(text, "12416");
    assert_eq!(menu_copy(&mut h, "LastName", false, "Copy Meaning").1, "Chicken");
    // Ctrl+C (the backend's Copy) on the row holding the keyboard.
    row(&h, "Comment").focus();
    h.step();
    h.event(egui::Event::Copy);
    h.step();
    let all: Vec<&String> = (h.output().platform_output.commands.iter())
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(all, ["line one\nline \"two\""], "copied once");
}

/// The Status Bar beside the tab (the Resources board's): the resource,
/// the layer it is read from and its size at the right; no hints of its
/// own (nothing in it is bound to a key, and no Save); with the pointer
/// over the Resources panel, the panel's hints and its count first.
#[test]
fn the_status_bar_says_the_resource_and_the_panel_s_count() {
    let Some(mut h) = opened("ui-u23-status", 1600.0, &[], chicken()) else { return };
    let bytes = {
        let game = h.state().game.as_deref().unwrap();
        game.resman.get(&chicken()).unwrap().len()
    };
    let count = format!("nw_chicken.utc from key:nwn_base, {bytes} bytes");
    let view = h.state().editor_area_rect().unwrap();
    h.hover_at(view.center());
    settle(&mut h);
    let s = h.state().status_said();
    assert!(s.contains(&count), "{s:?}");
    for none in ["Click", "Enter", "Save", "Esc", "Tab"] {
        assert!(!s.iter().any(|l| l.contains(&format!("{none}:"))), "no {none} hint: {s:?}");
    }
    // The panel in use: its hints, its count before the tab's.
    show_resources(h.state_mut());
    settle(&mut h);
    let panel = h.state().left_dock_rect().expect("the Left Dock");
    h.hover_at(panel.center());
    settle(&mut h);
    let s = h.state().status_said();
    let panel_count = s.iter().position(|l| l.contains(" of ") && l.contains(" resources"));
    let tab_count = s.iter().position(|l| *l == count);
    assert!(panel_count.is_some() && tab_count.is_some(), "{s:?}");
    assert!(panel_count < tab_count, "the panel's first: {s:?}");
    assert!(s.iter().any(|l| l == "Double Click: Open"), "{s:?}");
}

/// A floating read-only tab (M4's U23 fix): the Status Bar says which it
/// is, where from and its size while the pointer is over its window, and
/// while its window had the last press with the pointer over no docked
/// editor (on the Status Bar); not with the pointer over the docked area
/// behind it. The lock's tip says the same, whatever the bar shows.
#[test]
fn a_floating_read_only_tab_says_where_it_is_from_and_its_size() {
    let Some(mut h) = opened_in(Mode::Floating, "ui-u23-floating", 1600.0, &[], chicken(), None)
    else {
        return;
    };
    // (The module's Properties, opened with it in a window too, out of
    // the way.)
    close_tab(h.state_mut(), &Tab::ModuleProperties);
    settle(&mut h);
    let t = Tab::Resource(chicken());
    let at = h.state().dock.find_tab(&t).expect("the tab");
    assert!(!at.surface.is_main(), "floating");
    let bytes = {
        let game = h.state().game.as_deref().unwrap();
        game.resman.get(&chicken()).unwrap().len()
    };
    let count = format!("nw_chicken.utc from key:nwn_base, {bytes} bytes");
    let says = |h: &Harness<'_, Moonglow>| h.state().status_said().contains(&count);
    // Over the window: its count.
    let tag = row(&h, "Tag").rect();
    h.hover_at(tag.center());
    settle(&mut h);
    assert!(says(&h), "over the window: {:?}", h.state().status_said());
    // Over the docked area beside it: not its.
    let header = header_row(&h, mg_ui::RESOURCE_HEADER);
    let area = h.state().editor_area_rect().unwrap();
    // (Left of the window, which opens clear of the Left Dock's edge.)
    assert!(header.left() - area.left() > 40.0, "room beside: {header:?} in {area:?}");
    let beside = egui::pos2((area.left() + header.left()) / 2.0, area.bottom() - 20.0);
    h.hover_at(beside);
    settle(&mut h);
    assert!(!says(&h), "over the docked area: {:?}", h.state().status_said());
    // A press in the window, the pointer then on the Status Bar: its.
    row(&h, "Tag").click();
    settle(&mut h);
    h.hover_at(egui::pos2(area.center().x, area.bottom() + 12.0));
    settle(&mut h);
    assert!(says(&h), "with the last press: {:?}", h.state().status_said());
    // The lock's tip says it too.
    let lock =
        h.get_by_label(mg_ui::RESOURCE_HEADER).get_by_label("Read-only: from the game's data");
    lock.hover();
    h.run_steps(60);
    let tip = format!("{count}, shown as the game reads it");
    let shown: Vec<String> = (h.query_all_by_label_contains("as the game reads it"))
        .map(|n| {
            (n.accesskit_node().label().or_else(|| n.accesskit_node().value())).unwrap_or_default()
        })
        .collect();
    assert!(shown.contains(&tip), "the lock's tip {tip:?} in {shown:?}");
}

/// The tab comes to rest with each kind it shows (a GFF folded, unfolded
/// and scrolled, a table, a script, a text, a picture as its bytes, a
/// model as text, a sound as its bytes), docked and floating, the side
/// docks open and folded; the last check that the floating tab is drawn.
#[test]
fn the_read_only_tab_comes_to_rest_with_each_kind() {
    let Some(mut h) = opened("ui-u23-rest", 1600.0, &[], chicken()) else { return };
    rest(&mut h);
    row(&h, "FeatList").click();
    rest(&mut h);
    scroll_view(&mut h, 2);
    rest(&mut h);
    for (name, t) in [
        ("classes", ResType::TWODA),
        ("nw_c2_default1", ResType::NSS),
        ("tcn01", ResType::SET),
        ("po_chicken_l", ResType::TGA),
        ("plc_a01", ResType::MDL),
        ("as_an_crow1", ResType::WAV),
    ] {
        let key = ResKey::parse(name, t).unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
        rest(&mut h);
        h.run_steps(3);
        rest(&mut h);
    }
    h.state_mut().settings.hide_tree = true;
    h.state_mut().settings.hide_log = true;
    rest(&mut h);
    // Floating.
    close_tab(h.state_mut(), &Tab::Resource(chicken()));
    rest(&mut h);
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Floating;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(chicken())));
    rest(&mut h);
    let path = h.state().dock.find_tab(&Tab::Resource(chicken())).expect("open");
    assert!(
        h.state_mut().dock.get_window_state_mut(path.surface).is_some(),
        "floating in a window of its own"
    );
    row(&h, "FeatList");
}
