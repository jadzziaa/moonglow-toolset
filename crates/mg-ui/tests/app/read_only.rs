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
                h.step();
                advance_clock(&mut h, 1.5);
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
    h.step();
    advance_clock(&mut h, 1.5);
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
            h.step();
            advance_clock(&mut h, 1.5);
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
    h.step();
    advance_clock(&mut h, 1.5);
    let tip = format!("{count}, shown as the game reads it");
    let shown: Vec<String> = (h.query_all_by_label_contains("as the game reads it"))
        .map(|n| {
            (n.accesskit_node().label().or_else(|| n.accesskit_node().value())).unwrap_or_default()
        })
        .collect();
    assert!(shown.contains(&tip), "the lock's tip {tip:?} in {shown:?}");
}

/// A game's script shown read-only: its lines at the script editor's pitch
/// (the board's 18 points at 13), each line's row 18 high.
#[test]
fn a_read_only_script_s_lines_are_the_editor_s_pitch_apart() {
    let key = ResKey::parse("nw_c2_default1", ResType::NSS).unwrap();
    let Some(mut h) = opened("ui-w3-read-only-pitch", 1600.0, &[], key) else { return };
    rest(&mut h);
    let tab = h.state().editor_area_rect().expect("the editor area");
    let run = egui_kittest::kittest::by().role(egui::accesskit::Role::Label);
    let said = |n: &egui_kittest::Node<'_>| n.accesskit_node().value().unwrap_or_default();
    let first = (h.query_all(run.clone()))
        .find(|n| said(n).starts_with("//::") && tab.contains(n.rect().center()))
        .expect("the script's first line")
        .rect();
    // (Its lines, the empty ones between them as the gaps: each a row.)
    let mut rows: Vec<egui::Rect> = (h.query_all(run))
        .map(|n| n.rect())
        .filter(|r| r.left() == first.left() && tab.contains(r.center()) && r.top() >= first.top())
        .collect();
    rows.sort_by(|a, b| a.top().total_cmp(&b.top()));
    assert!(rows.len() >= 20, "the script's lines in sight: {rows:?}");
    for (a, b) in rows.iter().zip(&rows[1..]) {
        assert_eq!(a.height(), 18.0, "{a:?}");
        let apart = b.top() - a.top();
        assert!(apart > 0.0 && apart % 18.0 == 0.0, "{a:?} {b:?}");
    }
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

// M5's W6, the owner's ruling 49 ("share the viewers"): a picture or a
// sound open read-only is shown with the hak editor's Preview's viewers,
// one code (`mg_ui::viewers`), the bytes a check row of the header's More.

/// The speaker the viewers' tests listen to.
type Heard = std::rc::Rc<std::cell::RefCell<mg_ui::audio::Silence>>;

/// The game's data with a module open and a user folder whose override
/// holds `files` (name and bytes), the window `width` by 900, `key` open
/// read-only in front, sounds going to the speaker returned. `None`
/// without the game.
fn viewed_in(
    name: &str,
    width: f32,
    files: &[(&str, Vec<u8>)],
    key: ResKey,
) -> Option<(Harness<'static, Moonglow>, Heard)> {
    viewed_on(name, width, files, key, None)
}

/// [`viewed_in`] drawn on the GPU's render state `rs` where given.
fn viewed_on(
    name: &str,
    width: f32,
    files: &[(&str, Vec<u8>)],
    key: ResKey,
    rs: Option<egui_wgpu::RenderState>,
) -> Option<(Harness<'static, Moonglow>, Heard)> {
    let root = mg_testkit::nwn_root()?;
    let dir = mg_testkit::scratch_dir(name);
    let user = user_folder(&dir, &["override"]);
    for (file, data) in files {
        std::fs::write(user.join("override").join(file), data).unwrap();
    }
    let path = sample_module(&dir);
    let mut app = game_app(&root, Some(user));
    app.settings = Mode::Tabs.settings();
    app.settings.no_last_area = true;
    app.open_module(&path);
    let heard: Heard = Default::default();
    app.speaker = Box::new(heard.clone());
    let mut h = match rs {
        Some(rs) => {
            app.set_render_state(rs.clone());
            gpu_harness(app, rs, width, 900.0)
        }
        None => harness(app, width, 900.0),
    };
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    settle(&mut h);
    Some((h, heard))
}

/// A picture of `w` by `h` pixels as a TGA, gray.
fn tga_of(w: u32, h: u32) -> Vec<u8> {
    let mut image = mg_image::Rgba::new(w, h);
    image.data.fill(180);
    mg_image::write_tga(&image)
}

/// A short silent WAVE (8 kHz, 16-bit, mono).
fn wav_bytes() -> Vec<u8> {
    let samples: Vec<u8> = [0i16; 800].iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + samples.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    for n in [1u16, 1] {
        wav.extend(n.to_le_bytes());
    }
    for n in [8000u32, 16000] {
        wav.extend(n.to_le_bytes());
    }
    for n in [2u16, 16] {
        wav.extend(n.to_le_bytes());
    }
    wav.extend(b"data");
    wav.extend((samples.len() as u32).to_le_bytes());
    wav.extend(samples);
    wav
}

fn w6_key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

/// The files the viewers' tests open: a TGA, a PLT, a WAV and a kind
/// with no viewer.
fn w6_files() -> Vec<(&'static str, Vec<u8>)> {
    let plt = mg_image::plt::Plt::new(4, 2, vec![[128, 4]; 8]);
    vec![
        ("mg_w6pic.tga", tga_of(8, 4)),
        ("mg_w6plt.plt", plt.write()),
        ("mg_w6snd.wav", wav_bytes()),
        ("mg_w6bin.ncs", b"NCS V1.0B\x00\x00\x00\x0d\x2a\x00\x20".to_vec()),
    ]
}

/// The read-only header's More's check row `name` clicked (More opened
/// where it is not open).
fn more_row(h: &mut Harness<'_, Moonglow>, name: &str) {
    if h.query_by_label(name).is_none() {
        h.get_by_label(mg_ui::RESOURCE_HEADER).get_by_label("More").click();
        settle(h);
    }
    h.get_by_label(name).click();
    settle(h);
}

/// The node named `name` in the editor area.
fn in_tab<'h>(h: &'h Harness<'_, Moonglow>, name: &'h str) -> Option<egui_kittest::Node<'h>> {
    let tab = h.state().editor_area_rect().expect("the editor area");
    h.query_all_by_label(name).find(|n| tab.contains(n.rect().center()))
}

/// A picture open read-only is the picture, fitted, its size under it
/// (the hak Preview's viewer), with the header it had: where it is from,
/// its kind, Copy to Module and Save As…; its bytes are a check row of
/// the header's More (Show Bytes), which shows the first bytes as
/// before, and again the picture. A PLT is a picture (in each layer's
/// first color). A kind with no viewer is its bytes, with no Show Bytes.
#[test]
fn a_picture_open_read_only_is_the_picture() {
    let pic = w6_key("mg_w6pic.tga");
    let Some((mut h, _)) = viewed_in("ui-w6-picture", 1600.0, &w6_files(), pic) else { return };
    let header = mg_ui::RESOURCE_HEADER;
    let image = in_tab(&h, "mg_w6pic.tga, 8 by 4 pixels").expect("the picture");
    assert_eq!(image.accesskit_node().role(), egui::accesskit::Role::Image);
    h.get_by_label(header).get_by_label("Copy to Module");
    h.get_by_label(header).get_by_label("Save As…");
    h.get_by_label(header).get_by_label_contains("Read-only: from the override folder");
    assert!(in_tab(&h, "00000000").is_none(), "no bytes beside it");
    // Show Bytes, in More.
    more_row(&mut h, mg_ui::viewers::SHOW_BYTES);
    let tab = h.state().editor_area_rect().unwrap();
    let hex = h
        .query_all_by_label_contains("00000000  ")
        .find(|n| tab.contains(n.rect().center()))
        .expect("its first bytes");
    let node = hex.accesskit_node();
    let said = node.value().or_else(|| node.label()).unwrap_or_default();
    assert!(said.contains("00 00 02"), "a TGA's header: {said}");
    assert!(in_tab(&h, "mg_w6pic.tga, 8 by 4 pixels").is_none(), "the picture away");
    more_row(&mut h, mg_ui::viewers::SHOW_BYTES);
    assert!(in_tab(&h, "mg_w6pic.tga, 8 by 4 pixels").is_some(), "the picture again");
    // A PLT.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(w6_key("mg_w6plt.plt"))));
    settle(&mut h);
    assert!(in_tab(&h, "mg_w6plt.plt, 4 by 2 pixels").is_some(), "a PLT's picture");
    // No viewer: its bytes, as before, and no Show Bytes.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(w6_key("mg_w6bin.ncs"))));
    settle(&mut h);
    let tab = h.state().editor_area_rect().unwrap();
    assert!(
        h.query_all_by_label_contains("00000000  ").any(|n| tab.contains(n.rect().center())),
        "its bytes"
    );
    // (No More at all where nothing goes into it.)
    if let Some(more) = h.get_by_label(header).query_by_label("More") {
        more.click();
        settle(&mut h);
    }
    assert!(h.query_by_label(mg_ui::viewers::SHOW_BYTES).is_none(), "nothing to toggle");
}

/// A sound open read-only plays with the hak Preview's Play and Stop, on
/// the Preview channel, one at a time: it stops when its tab is closed,
/// put behind another or played over by another Play (the tab then says
/// Play), and says Play again once it ends, or once anything else stops
/// the Preview channel (at once, before a frame); the window stops asking
/// for frames then. No sound device: the test's speaker hears it.
#[test]
fn a_sound_open_read_only_plays_and_stops_when_unseen() {
    use mg_ui::audio::Channel;
    let snd = w6_key("mg_w6snd.wav");
    let asks = |h: &Harness<'_, Moonglow>| {
        h.ctx.repaint_causes().iter().any(|c| c.file.ends_with("viewers.rs"))
    };
    for way in ["stop", "close", "behind", "another", "ends", "elsewhere"] {
        let name = format!("ui-w6-sound-{way}");
        let Some((mut h, heard)) = viewed_in(&name, 1600.0, &w6_files(), snd) else { return };
        let playing = |heard: &Heard| heard.borrow().channels.get(&Channel::Preview).cloned();
        in_tab(&h, "A sound, 1.6 KB").expect("its size");
        assert!(playing(&heard).is_none(), "{way}: not at first");
        in_tab(&h, "Play").expect("Play").click();
        h.run_steps(2);
        assert_eq!(playing(&heard).map(|p| p.0.to_string()), Some("mg_w6snd".into()), "{way}");
        assert!(in_tab(&h, "Stop").is_some(), "{way}: Stop while it plays");
        assert!(asks(&h), "{way}: it asks to look again while it plays");
        match way {
            "stop" => in_tab(&h, "Stop").unwrap().click(),
            "close" => close_tab(h.state_mut(), &Tab::Resource(snd)),
            "behind" => {
                h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(chicken())))
            }
            "another" => {
                let other = mg_core::ResRef::from_str("mg_w6snd").unwrap();
                assert!(h.state_mut().play_sound(Channel::Preview, other, 1.0, false));
            }
            "elsewhere" => {
                h.state_mut().stop_sound(Channel::Preview);
                assert!(!h.state().viewer_plays(), "{way}: let go of at once");
            }
            _ => mg_ui::audio::Speaker::stop(&mut *heard.borrow_mut(), Channel::Preview),
        }
        settle(&mut h);
        h.run_steps(2);
        match way {
            // (Another's Play holds the channel: the tab no longer.)
            "another" => assert!(playing(&heard).is_some(), "{way}: the other plays"),
            _ => assert!(playing(&heard).is_none(), "{way}: stopped"),
        }
        if matches!(way, "stop" | "another" | "ends" | "elsewhere") {
            assert!(in_tab(&h, "Play").is_some(), "{way}: Play again");
            assert!(in_tab(&h, "Stop").is_none(), "{way}: no Stop");
        }
        if way != "another" {
            assert!(!asks(&h), "{way}: no more asks");
        }
    }
}

/// A large picture (2048 by 2048) is decoded off the frame: while it is
/// read (held here) the frames go on, the tab saying so, and the window's
/// own work is under way; then it is shown, decoded once, not again as
/// frames are drawn; let go of with its tab and decoded anew when opened
/// again.
struct ReleaseOnDrop<'a>(&'a [ResKey]);
impl Drop for ReleaseOnDrop<'_> {
    fn drop(&mut self) {
        mg_ui::viewers::release(self.0);
    }
}

struct HealOnDrop<'a>(&'a [ResKey]);
impl Drop for HealOnDrop<'_> {
    fn drop(&mut self) {
        mg_ui::viewers::heal(self.0);
    }
}

#[test]
fn a_large_picture_is_decoded_off_the_frame_once() {
    let big = w6_key("mg_w6big.tga");
    let held_keys = [big];
    let files = [("mg_w6big.tga", tga_of(2048, 2048))];
    mg_ui::viewers::hold(&held_keys);
    let release_guard = ReleaseOnDrop(&held_keys);
    let Some((mut h, _)) = ({
        // (Opened without waiting: the hold keeps it under way.)
        let root = mg_testkit::nwn_root();
        match root {
            None => None,
            Some(root) => {
                let dir = mg_testkit::scratch_dir("ui-w6-large");
                let user = user_folder(&dir, &["override"]);
                for (file, data) in &files {
                    std::fs::write(user.join("override").join(file), data).unwrap();
                }
                let path = sample_module(&dir);
                let mut app = game_app(&root, Some(user));
                app.settings = Mode::Tabs.settings();
                app.settings.no_last_area = true;
                app.open_module(&path);
                let mut h = harness(app, 1600.0, 900.0);
                settle(&mut h);
                Some((h, ()))
            }
        }
    }) else {
        return;
    };
    let before = mg_ui::viewers::decoded_of(big);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(big)));
    for _ in 0..5 {
        h.run();
    }
    assert!(in_tab(&h, "Reading the picture…").is_some(), "said while it is read");
    assert!(h.state().under_way().is_some(), "the window's own work");
    assert_eq!(mg_ui::viewers::decoded_of(big), before, "not on the frame");
    drop(release_guard);
    settle(&mut h);
    assert!(in_tab(&h, "mg_w6big.tga, 2048 by 2048 pixels").is_some(), "shown");
    assert_eq!(mg_ui::viewers::decoded_of(big), before + 1, "decoded once");
    for _ in 0..10 {
        h.run();
    }
    assert_eq!(mg_ui::viewers::decoded_of(big), before + 1, "not again as frames go by");
    close_tab(h.state_mut(), &Tab::Resource(big));
    settle(&mut h);
    // (Let go of a while after it is drawn no more.)
    let started = std::time::Instant::now();
    while h.state().read_only_pictures() > 0 && started.elapsed().as_secs() < 10 {
        h.run();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(h.state().read_only_pictures(), 0, "let go of");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(big)));
    settle(&mut h);
    assert!(in_tab(&h, "mg_w6big.tga, 2048 by 2048 pixels").is_some(), "shown again");
    assert_eq!(mg_ui::viewers::decoded_of(big), before + 2, "decoded anew");
    decodes_ended(&mut h, "all done");
}

/// Show Bytes on over a picture, longer than a picture unseen is kept
/// (`model_view::UNSEEN_AFTER`, 2 s): the picture is kept, not read and
/// decoded anew each 2 s, and the window's own work is done (the review
/// of M5's W6: it was let go of under its bytes and decoded again for as
/// long as they showed, never drawn, the window never done).
#[test]
fn a_picture_under_its_bytes_is_kept_and_decoded_once() {
    let pic = w6_key("mg_w6kept.tga");
    let files = [("mg_w6kept.tga", tga_of(8, 4))];
    let Some((mut h, _)) = viewed_in("ui-w6-bytes-kept", 1600.0, &files, pic) else { return };
    let before = mg_ui::viewers::decoded_of(pic);
    more_row(&mut h, mg_ui::viewers::SHOW_BYTES);
    let started = std::time::Instant::now();
    while started.elapsed().as_millis() < 3500 {
        h.run();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(mg_ui::viewers::decoded_of(pic), before, "not decoded again");
    assert_eq!(h.state().read_only_pictures(), 1, "kept");
    assert_eq!(h.state().under_way(), None, "the window's work done");
    more_row(&mut h, mg_ui::viewers::SHOW_BYTES);
    assert!(in_tab(&h, "mg_w6kept.tga, 8 by 4 pixels").is_some(), "the picture again");
    assert_eq!(mg_ui::viewers::decoded_of(pic), before, "the one decoded");
}

/// A picture whose decode panics says it could not be read and leaves
/// nothing waiting: the window's own work is done (`settle` returns).
#[test]
fn a_picture_whose_decode_fails_leaves_nothing_waiting() {
    let bad = w6_key("mg_w6bad.tga");
    let bad_keys = [bad];
    let files = [("mg_w6bad.tga", tga_of(8, 4))];
    mg_ui::viewers::fail(&bad_keys);
    let heal_guard = HealOnDrop(&bad_keys);
    let opened = viewed_in("ui-w6-decode-fails", 1600.0, &files, bad);
    drop(heal_guard);
    let Some((h, _)) = opened else { return };
    assert!(in_tab(&h, "The picture could not be read.").is_some(), "said");
    assert_eq!(h.state().under_way(), None, "nothing waiting");
    assert_eq!(h.state().picture_decodes().1, 0);
}

/// Many pictures opened at once (a session's tabs restored) are decoded
/// at most four at a time, the rest waiting their turn, each decoded once
/// at most (one let go of before its turn is not decoded).
#[test]
fn many_pictures_are_decoded_a_few_at_a_time() {
    let names: Vec<String> = (0..7).map(|i| format!("mg_w6many{i}.tga")).collect();
    let keys: Vec<ResKey> = names.iter().map(|n| w6_key(n)).collect();
    let files: Vec<(&str, Vec<u8>)> = names.iter().map(|n| (n.as_str(), tga_of(8, 4))).collect();
    let Some((mut h, _)) = viewed_in("ui-w6-many", 1600.0, &files, keys[0]) else { return };
    let before: Vec<usize> = keys.iter().map(|k| mg_ui::viewers::decoded_of(*k)).collect();
    // (Each made in front once, its decode held.)
    mg_ui::viewers::hold(&keys[1..]);
    let release_guard = ReleaseOnDrop(&keys);
    for k in &keys[1..] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(*k)));
        h.run_steps(2);
        let started = std::time::Instant::now();
        while in_tab(&h, "Reading the picture…").is_none() && started.elapsed().as_secs() < 10 {
            h.run();
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    let (workers, waiting) = h.state().picture_decodes();
    drop(release_guard);
    assert_eq!((workers, waiting), (4, 6), "four at a time, six waiting or under way");
    settle(&mut h);
    decodes_ended(&mut h, "all done");
    let mut decoded = 0;
    for (i, k) in keys.iter().enumerate().skip(1) {
        let n = mg_ui::viewers::decoded_of(*k) - before[i];
        assert!(n <= 1, "{k} decoded once at most: {n}");
        decoded += n;
    }
    assert!(decoded >= 4, "the held ones decoded: {decoded}");
}

/// `key`'s tab opened in front, while its decode is held or waits: until
/// it says it is being read.
fn open_while_read(h: &mut Harness<'_, Moonglow>, key: ResKey) {
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    h.run_steps(2);
    let started = std::time::Instant::now();
    while in_tab(h, "Reading the picture…").is_none() && started.elapsed().as_secs() < 10 {
        h.run();
    }
    assert!(in_tab(h, "Reading the picture…").is_some(), "{key} being read");
}

/// Steps the window until no picture decode is left and no decoding
/// thread is counted: a thread is counted a moment after its last
/// picture's decode is, which `settle` (by the decodes) does not wait for.
#[track_caller]
fn decodes_ended(h: &mut Harness<'_, Moonglow>, what: &str) {
    let started = std::time::Instant::now();
    while h.state().picture_decodes() != (0, 0) {
        assert!(started.elapsed().as_secs() < 10, "{what}: {:?}", h.state().picture_decodes());
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// Pictures of 8 by 4 pixels named `mg_w6{stem}{i}.tga` for each `i` in
/// `0..n`: their file names and keys.
fn w6_many(stem: &str, n: usize) -> (Vec<(String, Vec<u8>)>, Vec<ResKey>) {
    let names: Vec<String> = (0..n).map(|i| format!("mg_w6{stem}{i}.tga")).collect();
    let keys = names.iter().map(|n| w6_key(n)).collect();
    (names.into_iter().map(|n| (n, tga_of(8, 4))).collect(), keys)
}

/// Five decodes that panic (each takes its decoding thread with it), then
/// a good picture: the window still decodes it (a thread that died gives
/// up its place, the review of M5, reviewer D; counted dead, four would
/// have left every later picture waiting for ever).
#[test]
fn the_window_decodes_on_after_decodes_that_panic() {
    let (mut files, keys) = w6_many("die", 5);
    files.push(("mg_w6good.tga".into(), tga_of(8, 4)));
    let files: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let good = w6_key("mg_w6good.tga");
    mg_ui::viewers::fail(&keys);
    let heal_guard = HealOnDrop(&keys);
    let opened = viewed_in("ui-w6-panics", 1600.0, &files, keys[0]);
    let Some((mut h, _)) = opened else {
        return;
    };
    for k in &keys[1..] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(*k)));
        settle(&mut h);
        assert!(in_tab(&h, "The picture could not be read.").is_some(), "{k} failed");
    }
    drop(heal_guard);
    let before = mg_ui::viewers::decoded_of(good);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(good)));
    settle(&mut h);
    assert!(in_tab(&h, "mg_w6good.tga, 8 by 4 pixels").is_some(), "decoded and shown");
    assert_eq!(mg_ui::viewers::decoded_of(good), before + 1);
    decodes_ended(&mut h, "nothing left");
}

/// A picture whose tab closes while its decode waits its turn (four held
/// ahead of it) is let go of before its turn comes, and not decoded.
#[test]
fn a_picture_let_go_of_before_its_turn_is_not_decoded() {
    let (mut files, held) = w6_many("ahead", 4);
    files.push(("mg_w6skip.tga".into(), tga_of(8, 4)));
    let files: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let skip = w6_key("mg_w6skip.tga");
    let Some((mut h, _)) = viewed_in("ui-w6-skip", 1600.0, &files, chicken()) else { return };
    let before = mg_ui::viewers::decoded_of(skip);
    mg_ui::viewers::hold(&held);
    let release_guard = ReleaseOnDrop(&held);
    for k in &held {
        open_while_read(&mut h, *k);
    }
    // (The four taken by the threads first: the picture in sight goes
    // before those waiting, and on a busy machine the threads had not
    // taken them yet when the last was opened, which then went first.)
    let started = std::time::Instant::now();
    while h.state().picture_decodes_untaken() > 0 {
        assert!(started.elapsed().as_secs() < 10, "the four never taken");
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    open_while_read(&mut h, skip);
    assert_eq!(h.state().picture_decodes(), (4, 5), "four at work, one waiting");
    for k in held.iter().chain([&skip]) {
        close_tab(h.state_mut(), &Tab::Resource(*k));
    }
    let started = std::time::Instant::now();
    while h.state().read_only_pictures() > 0 && started.elapsed().as_secs() < 10 {
        h.run();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let let_go = h.state().read_only_pictures();
    drop(release_guard);
    assert_eq!(let_go, 0, "let go of");
    settle(&mut h);
    assert_eq!(mg_ui::viewers::decoded_of(skip), before, "not decoded");
    decodes_ended(&mut h, "nothing left");
}

/// The picture in sight goes first: with every decoding thread busy and
/// two pictures of tabs now behind waiting, the one in front (opened last)
/// is decoded at the first free thread, before them.
#[test]
fn the_picture_in_sight_is_decoded_before_hidden_ones() {
    let (files, keys) = w6_many("turn", 7);
    let files: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let Some((mut h, _)) = viewed_in("ui-w6-turn", 1600.0, &files, chicken()) else { return };
    let (held, front) = (&keys[..6], keys[6]);
    let before = mg_ui::viewers::decoded_of(front);
    mg_ui::viewers::hold(held);
    let release_guard = ReleaseOnDrop(held);
    for k in &keys {
        open_while_read(&mut h, *k);
    }
    assert_eq!(h.state().picture_decodes(), (4, 7), "four at work, three waiting");
    // (One thread freed: it takes the picture in front.)
    mg_ui::viewers::release(&keys[..1]);
    let started = std::time::Instant::now();
    while mg_ui::viewers::decoded_of(front) == before && started.elapsed().as_secs() < 5 {
        h.run();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let decoded = mg_ui::viewers::decoded_of(front) - before;
    let waiting = h.state().picture_decodes().1;
    drop(release_guard);
    assert_eq!(decoded, 1, "the one in front decoded first");
    assert_eq!(waiting, 5, "the hidden ones still waiting (two) or held (three)");
    settle(&mut h);
    assert!(in_tab(&h, "mg_w6turn6.tga, 8 by 4 pixels").is_some(), "shown");
    decodes_ended(&mut h, "all done");
}

/// A model open read-only (a layer's copy, or opened so by a plugin) is
/// drawn in the model viewer, paused, its header the model views' own;
/// Show as Text in the read-only header's More shows its text as before.
#[test]
fn a_model_open_read_only_is_drawn() {
    let model = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    let Some(rs) = gpu() else { return };
    let Some((mut h, _)) = viewed_on("ui-w6-model", 1600.0, &[], model, Some(rs)) else { return };
    let tab = h.state().editor_area_rect().unwrap();
    let row = mg_uikit::header::records(&h.ctx, mg_ui::model_view::MODEL_HEADER);
    assert_eq!(row.len(), 1, "the model's header");
    assert!(tab.contains_rect(row[0].row), "in the tab");
    let source = mg_ui::model_view::Source::Resource(model);
    assert!(!h.state().model_views[&source].playing, "paused");
    more_row(&mut h, mg_ui::viewers::SHOW_TEXT);
    let tab = h.state().editor_area_rect().unwrap();
    assert!(
        h.query_all_by_label_contains("newmodel").any(|n| tab.contains(n.rect().center())),
        "its text"
    );
    // (Its text in place of the drawing: the view is drawn no more.)
    let drawn =
        |h: &Harness<'_, Moonglow>| h.state().model_views.get(&source).and_then(|v| v.drawn_in());
    let was = drawn(&h);
    h.run_steps(3);
    assert_eq!(drawn(&h), was, "not drawn");
}

// v5.2's W3 (a builder's reports 9 and 10): Find in a read-only text, and
// a game script opened by Go to Definition at its declaration's line.

/// A big game script of our own (`x_big.nss`, about 700 KB: 20,000
/// lines), two lines in it holding MARKER, and an include
/// (`x_inc.nss`) whose functions lie far down; a module whose script
/// `main_a` calls them. The game's data is a layer of ours: no install.
fn w3_fixture(mode: Mode, name: &str) -> (Harness<'static, Moonglow>, ResKey) {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = sample_module(&dir);
    let main = ResKey::parse("main_a", ResType::NSS).unwrap();
    let mut m = mg_module::Module::open(&path).unwrap();
    m.set(main, W3_MAIN.as_bytes().to_vec());
    m.save().unwrap();
    let mut mem = mg_resman::MemContainer::new();
    mem.insert(w3_key("x_big"), w3_big().into_bytes());
    mem.insert(w3_key("x_inc"), w3_inc().into_bytes());
    let mut rm = mg_resman::ResMan::new();
    rm.add(mg_resman::priority::KEY, "mem", mg_resman::LayerClass::Key, mem);
    let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
    let mut app = app_with(Vec::new());
    put_in(&mut app, mode);
    app.settings.no_last_area = true;
    app.game = Some(std::sync::Arc::new(mg_rules::GameData::new(rm, tlk)));
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    (h, main)
}

const W3_MAIN: &str = "#include \"x_inc\"\nvoid main()\n{\n    XFar();\n    XNear();\n}\n";

fn w3_key(name: &str) -> ResKey {
    ResKey::parse(name, ResType::NSS).unwrap()
}

/// `x_big.nss`: MARKER on lines 15,000 and 18,000 (from 0).
fn w3_big() -> String {
    let mut text = String::new();
    for n in 0..20_000 {
        match n {
            15_000 => text.push_str("int MARKER_ONE = 15000; // the first\n"),
            18_000 => text.push_str("int MARKER_TWO = 18000; // the second\n"),
            n => text.push_str(&format!("int LINE_{n:05} = {n}; // filler text\n")),
        }
    }
    text
}

/// `x_inc.nss`: XNear on line 3 and XFar on line 900 (from 0).
fn w3_inc() -> String {
    let mut text = String::from("// An include of the game's.\n\n// Near.\nvoid XNear() { }\n");
    while text.lines().count() < 899 {
        text.push_str("// filler\n");
    }
    text.push_str("// Far.\nvoid XFar() { }\n");
    text
}

/// The lines of the text drawn in sight in the read-only tab of `key`
/// (its view's rect, in a window or not) that hold `part`.
fn lines_shown(h: &Harness<'_, Moonglow>, key: ResKey, part: &str) -> Vec<egui::Rect> {
    let area = h.state().read_only_view_rect_for_test(key).expect("the text drawn");
    let run = egui_kittest::kittest::by().role(egui::accesskit::Role::Label);
    (h.query_all(run))
        .filter(|n| n.accesskit_node().value().unwrap_or_default().contains(part))
        .map(|n| n.rect())
        .filter(|r| area.contains(r.center()))
        .collect()
}

/// Ctrl+F in a read-only script opens its Find bar; what is typed is found
/// (all 700 KB looked through, a slice a frame) and the match chosen is
/// scrolled into sight; F3, Shift+F3, Enter and the bar's buttons go on;
/// the count says which; Escape closes it and gives the keys back.
#[test]
fn ctrl_f_finds_in_a_read_only_script() {
    for mode in MODES {
        let (mut h, _) = w3_fixture(mode, "ui-w3-find");
        let big = w3_key("x_big");
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(big)));
        settle(&mut h);
        assert!(lines_shown(&h, big, "MARKER").is_empty(), "far down, out of sight");
        assert!(h.query_by_label("Find What").is_none());
        // (A window just opened has the keys once the pointer is over it, or
        // it was clicked: as Floating Windows, the pointer over its text.)
        if !mode.tabs() {
            let view = h.state().read_only_view_rect_for_test(big).expect("drawn");
            h.hover_at(view.center());
            settle(&mut h);
        }
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
        settle(&mut h);
        assert!(h.state().read_only_find_open_for_test(big, None), "Ctrl+F opens the bar");
        let field = h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Find What");
        assert!(field.accesskit_node().is_focused(), "its field has the keys");
        h.event(egui::Event::Text("marker".into()));
        settle(&mut h);
        let state =
            |h: &Harness<'_, Moonglow>| h.state().read_only_find_for_test(big, None).unwrap();
        assert_eq!(state(&h), (2, true, Some(15_000), None), "both found, the first chosen");
        assert_eq!(lines_shown(&h, big, "MARKER_ONE").len(), 1, "scrolled into sight");
        h.get_by_label("1 of 2 matches");
        // F3: the next; Shift+F3: the previous; round from the last.
        h.key_press(egui::Key::F3);
        settle(&mut h);
        assert_eq!(state(&h).2, Some(18_000));
        assert_eq!(lines_shown(&h, big, "MARKER_TWO").len(), 1, "the second in sight");
        h.get_by_label("2 of 2 matches");
        h.key_press(egui::Key::F3);
        settle(&mut h);
        assert_eq!(state(&h).2, Some(15_000), "round to the first");
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F3);
        settle(&mut h);
        assert_eq!(state(&h).2, Some(18_000), "Shift+F3: the previous");
        // Enter in the field: the next, the field keeping the keys.
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(state(&h).2, Some(15_000), "Enter: the next");
        assert!(
            h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Find What")
                .accesskit_node()
                .is_focused(),
            "kept"
        );
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Enter);
        settle(&mut h);
        assert_eq!(state(&h).2, Some(18_000), "Shift+Enter: the previous");
        h.get_by_label("Find Previous").click();
        settle(&mut h);
        assert_eq!(state(&h).2, Some(15_000), "the bar's Find Previous");
        // Match Whole Word Only: MARKER is no word of MARKER_ONE.
        h.get_by_label("Match Whole Word Only").click();
        settle(&mut h);
        assert_eq!(state(&h).0, 0);
        h.get_by_label("No matches");
        // Escape: closed, the keys the text's again.
        h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Find What").click();
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert!(!h.state().read_only_find_open_for_test(big, None), "Escape closes it");
        assert!(h.query_by_label("Find What").is_none());
        assert!(h.state().dock.find_tab(&Tab::Resource(big)).is_some(), "the tab stays");
        // (The keys the text's: Ctrl+F opens it again.)
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
        settle(&mut h);
        assert!(h.state().read_only_find_open_for_test(big, None), "and again");
        // Another read-only text in front, then this one again: its bar and
        // what was typed kept (each tab's own).
        let inc = w3_key("x_inc");
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(inc)));
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(big)));
        settle(&mut h);
        assert!(h.state().read_only_find_open_for_test(big, None), "{mode:?}: kept");
        assert_eq!(state(&h).0, 0, "{mode:?}: what was typed kept (whole words: none)");
    }
}

/// Ctrl+A in a read-only text chooses all of it, which Copy copies.
#[test]
fn select_all_and_copy_in_a_read_only_text() {
    let (mut h, _) = w3_fixture(Mode::Tabs, "ui-w3-copy");
    let inc = w3_key("x_inc");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(inc)));
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    settle(&mut h);
    h.event(egui::Event::Copy);
    h.run_steps(1);
    assert_eq!(copied(&h).as_deref(), Some(w3_inc().as_str()), "the whole text");
}

/// The Status Bar hints Find's keys over a read-only text; the Command
/// Palette's Find opens its bar; lines in sight dragged over are copied
/// by Copy.
#[test]
fn the_palette_s_find_the_hints_and_a_part_copied() {
    let (mut h, _) = w3_fixture(Mode::Tabs, "ui-w3-palette");
    let inc = w3_key("x_inc");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(inc)));
    settle(&mut h);
    let said = h.state().status_said();
    assert!(said.iter().any(|s| s.ends_with(": Find")), "{said:?}");
    assert!(said.iter().any(|s| s.ends_with(": Find next")), "{said:?}");
    command_palette(&mut h, "find");
    quick_open_row(&h, "Find").click();
    settle(&mut h);
    assert!(h.state().read_only_find_open_for_test(inc, None), "the palette's Find");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().read_only_find_open_for_test(inc, None));
    // Two lines dragged over, then Copy.
    let first = lines_shown(&h, inc, "An include")[0];
    let second = lines_shown(&h, inc, "Near.")[0];
    drag(&mut h, first.left_center() + egui::vec2(1.0, 0.0), second.right_center());
    h.event(egui::Event::Copy);
    h.run_steps(1);
    let got = copied(&h).unwrap_or_default();
    assert!(got.contains("An include") && got.contains("Near."), "{got:?}");
}

/// Go to Definition (Ctrl+click) on a function of a game script opens the
/// script read-only at the declaration's line, marked; again on another
/// function of the same script, the same tab goes to its line. As Floating
/// Windows, a tab of the window of the script it was asked from (the
/// owner's ruling of 2026-11-23), no new window; as Tabs, a tab of the main
/// window.
#[test]
fn go_to_definition_opens_a_game_script_at_the_line() {
    for mode in MODES {
        let (mut h, main) = w3_fixture(mode, "ui-w3-definition");
        let inc = w3_key("x_inc");
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(main)));
        settle(&mut h);
        let windows = |h: &Harness<'_, Moonglow>| {
            let dock = &h.state().dock;
            dock.iter_surfaces().filter(|s| matches!(s, egui_dock::Surface::Window(..))).count()
        };
        let before = windows(&h);
        let script = h.state().dock.find_tab(&Tab::Script(main)).expect("open");
        assert_eq!(script.surface.is_main(), mode.tabs(), "{mode:?}");
        let go = |h: &mut Harness<'_, Moonglow>, name: &str| {
            let at = W3_MAIN.find(name).unwrap() + 1;
            let d = h.state_mut().declaration_at(main, at).expect("a declaration");
            assert_eq!(d.at.file, "x_inc");
            h.state_mut().go_to_declaration(&d);
            settle(h);
        };
        go(&mut h, "XFar");
        let path = h.state().dock.find_tab(&Tab::Resource(inc)).expect("x_inc opens");
        assert_eq!(windows(&h), before, "{mode:?}: no window more");
        assert_eq!(path.surface, script.surface, "{mode:?}: where the script is");
        let state = |h: &Harness<'_, Moonglow>| h.state().read_only_find_for_test(inc, None);
        assert_eq!(state(&h).unwrap().3, Some(900), "{mode:?}: XFar's line marked");
        assert_eq!(lines_shown(&h, inc, "void XFar").len(), 1, "{mode:?}: in sight");
        assert!(lines_shown(&h, inc, "void XNear").is_empty(), "{mode:?}: at the top no more");
        // Another function of it: the same tab, at its line.
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(main)));
        settle(&mut h);
        go(&mut h, "XNear");
        assert_eq!(h.state().dock.find_tab(&Tab::Resource(inc)), Some(path), "{mode:?}: reused");
        assert_eq!(windows(&h), before);
        assert_eq!(state(&h).unwrap().3, Some(3), "{mode:?}: XNear's line marked");
        assert_eq!(lines_shown(&h, inc, "void XNear").len(), 1, "{mode:?}: in sight");
    }
}

/// Go to Definition on a function of the game's own `nwscript.nss` lands
/// on its line (needs the game).
#[test]
fn go_to_definition_lands_on_an_nwscript_function_s_line() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-w3-nwscript");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main() { ActionSit(OBJECT_SELF); }\n";
    let mut m = mg_module::Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let d = h.state_mut().declaration_at(key, text.find("ActionSit").unwrap() + 2).unwrap();
    h.state_mut().go_to_declaration(&d);
    settle(&mut h);
    let nwscript = ResKey::parse("nwscript", ResType::NSS).unwrap();
    let game = h.state().game.clone().unwrap();
    let source =
        String::from_utf8_lossy(&game.resman.get(&nwscript).unwrap()).replace("\r\n", "\n");
    let at = source.find("\nvoid ActionSit(").expect("declared") + 1;
    let line = source[..at].matches('\n').count();
    assert!(line > 1000, "far down: {line}");
    let marked = h.state().read_only_find_for_test(nwscript, None).and_then(|s| s.3);
    assert_eq!(marked, Some(line));
    assert_eq!(lines_shown(&h, nwscript, "void ActionSit(").len(), 1, "in sight");
}

/// A read-only text's right-click menu: the text fields' rows, Cut and
/// Paste grayed, Copy with lines chosen (a right click in them keeps
/// them), Select All, then Find…; Shift+F10 opens it from the keys and
/// Escape closes it, not the Find bar.
#[test]
fn a_read_only_text_s_right_click_menu() {
    use crate::fields::{choose_text_row, right_click_at, text_menu_row, text_menu_rows};
    let (mut h, _) = w3_fixture(Mode::Tabs, "ui-w3-menu");
    let inc = w3_key("x_inc");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(inc)));
    settle(&mut h);
    let first = lines_shown(&h, inc, "An include")[0];
    let second = lines_shown(&h, inc, "Near.")[0];
    // Nothing chosen: Copy grayed too.
    right_click_at(&mut h, second.center());
    assert_eq!(text_menu_rows(&h), [Some(false), Some(false), Some(false), Some(true)]);
    assert_eq!(text_menu_row(&h, "Find…", "Ctrl+F").map(|r| r.0), Some(true), "then Find…");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(text_menu_row(&h, "Select All", "Ctrl+A").is_none(), "Escape closes it");
    // Two lines dragged over, a right click in them: Copy copies them.
    drag(&mut h, first.left_center() + egui::vec2(1.0, 0.0), second.right_center());
    right_click_at(&mut h, first.center());
    assert_eq!(text_menu_rows(&h), [Some(false), Some(true), Some(false), Some(true)]);
    let (got, _) = choose_text_row(&mut h, "Copy", "Ctrl+C");
    assert!(
        got.iter().any(|t| t.contains("An include") && t.contains("Near.")),
        "the lines chosen: {got:?}"
    );
    // Select All, then Copy: the whole text.
    right_click_at(&mut h, second.center());
    choose_text_row(&mut h, "Select All", "Ctrl+A");
    right_click_at(&mut h, second.center());
    let (got, _) = choose_text_row(&mut h, "Copy", "Ctrl+C");
    assert_eq!(got.last().map(String::as_str), Some(w3_inc().as_str()), "the whole text");
    // Find… opens the bar, its field with the keys.
    right_click_at(&mut h, second.center());
    choose_text_row(&mut h, "Find…", "Ctrl+F");
    assert!(h.state().read_only_find_open_for_test(inc, None), "Find… opens the bar");
    let field = h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Find What");
    assert!(field.accesskit_node().is_focused());
    // Shift+F10 with the keys the text's: the menu, its first row on lit;
    // Escape closes the menu alone.
    h.get_by_label("Match Case").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    settle(&mut h);
    assert_eq!(text_menu_rows(&h), [Some(false), Some(false), Some(false), Some(true)]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(text_menu_row(&h, "Select All", "Ctrl+A").is_none(), "the menu closed");
    assert!(h.state().read_only_find_open_for_test(inc, None), "the bar stays open");
}

/// As Floating Windows, two read-only texts side by side: the keys of
/// Find are the one's last pressed in (its window the focused one), not
/// the one the pointer rests over; Escape closes the Find bar of the
/// window the builder is in alone.
#[test]
fn two_read_only_texts_side_by_side_one_takes_the_keys() {
    let (mut h, _) = w3_fixture(Mode::Floating, "ui-w3-two-views");
    let (big, inc) = (w3_key("x_big"), w3_key("x_inc"));
    for key in [big, inc] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
        settle(&mut h);
    }
    // (Side by side, each in a window of its own; a point of each's text.)
    own_window(&mut h, &Tab::Resource(big), egui::pos2(40.0, 120.0), egui::vec2(700.0, 600.0));
    own_window(&mut h, &Tab::Resource(inc), egui::pos2(800.0, 120.0), egui::vec2(700.0, 600.0));
    settle(&mut h);
    let spot = |h: &Harness<'_, Moonglow>, key, _other| {
        h.state().read_only_view_rect_for_test(key).expect("drawn").center()
    };
    let open = |h: &Harness<'_, Moonglow>, key| h.state().read_only_find_open_for_test(key, None);
    let click = |h: &mut Harness<'_, Moonglow>, at| {
        h.hover_at(at);
        h.run_steps(1);
        h.drag_at(at);
        h.run_steps(1);
        h.drop_at(at);
        settle(h);
    };
    // Pressed in the one, the pointer resting over the other: the keys are
    // the one's, each way round (whichever is drawn first).
    let rest = |h: &mut Harness<'_, Moonglow>, (pressed, over), key| {
        let at = spot(h, pressed, over);
        click(h, at);
        h.hover_at(spot(h, over, pressed));
        settle(h);
        h.key_press_modifiers(
            if key == egui::Key::F { egui::Modifiers::COMMAND } else { egui::Modifiers::NONE },
            key,
        );
        settle(h);
    };
    rest(&mut h, (big, inc), egui::Key::F);
    assert!(open(&h, big), "the text pressed in");
    assert!(!open(&h, inc), "not the one the pointer is over");
    rest(&mut h, (inc, big), egui::Key::F);
    assert!(open(&h, inc), "the other way round: its own Ctrl+F");
    // Both bars open: Escape closes the bar of the window pressed in alone.
    rest(&mut h, (inc, big), egui::Key::Escape);
    assert!(!open(&h, inc), "Escape closes the bar of the window pressed in");
    assert!(open(&h, big), "and not the other's");
    rest(&mut h, (big, inc), egui::Key::Escape);
    assert!(!open(&h, big), "the other way round");
}

/// The pictures of v5.2's W3 (dark): `nwscript.nss` opened by Go to
/// Definition at ActionSit's line, in each mode, and its Find bar. By hand:
/// `MOONGLOW_W3_PICTURES=<folder> cargo test -p mg-ui --test app --
/// --ignored read_only::w3_pictures`.
#[test]
#[ignore]
fn w3_pictures() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let Some(out) = std::env::var_os("MOONGLOW_W3_PICTURES").map(std::path::PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    mg_testkit::gpu::hold();
    for mode in MODES {
        let dir = mg_testkit::scratch_dir(&mode.dir("ui-w3-pictures"));
        let path = sample_module(&dir);
        let key = ResKey::parse("hello", ResType::NSS).unwrap();
        let text = "void main()\n{\n    ActionSit(OBJECT_SELF);\n}\n";
        let mut m = mg_module::Module::open(&path).unwrap();
        m.set(key, text.as_bytes().to_vec());
        m.save().unwrap();
        let mut app = game_app(&root, None);
        put_in(&mut app, mode);
        app.settings.no_last_area = true;
        app.settings.light_theme = false;
        app.open_module(&path);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1600.0, 900.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        settle(&mut h);
        let d = h.state_mut().declaration_at(key, text.find("ActionSit").unwrap() + 2).unwrap();
        h.state_mut().go_to_declaration(&d);
        settle(&mut h);
        h.run_steps(3);
        let shot = |h: &mut Harness<'_, Moonglow>, name: &str| {
            let image = h.render().expect("render");
            image.save(out.join(format!("{name}-{}.png", mode.word()))).unwrap();
        };
        shot(&mut h, "w3-definition-at-the-line");
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
        settle(&mut h);
        h.event(egui::Event::Text("ActionSit".into()));
        settle(&mut h);
        h.run_steps(3);
        shot(&mut h, "w3-find-bar");
    }
}

// The owner's ruling of 2026-11-25 (the picture viewer's zoom): the wheel
// zooms about the pointer, a drag pans, a double click goes between
// fitted and actual pixels, and `+`, `-` and `0` over the picture zoom
// and fit.

/// A wide picture, larger than the tab, as a read-only tab shows it
/// fitted: the window, and the picture's key.
fn zoomable(name: &str) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let pic = w6_key("mg_w6zoom.tga");
    let files = [("mg_w6zoom.tga", tga_of(2048, 1024)), ("mg_w6zoomb.tga", tga_of(2048, 1024))];
    let (mut h, _) = viewed_in(name, 1600.0, &files, pic)?;
    let started = std::time::Instant::now();
    while mg_ui::viewers::picture_drawn(&h.ctx, pic).is_none() {
        assert!(started.elapsed() < SETTLE_TIME, "the picture never drawn");
        h.run_steps(2);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    settle(&mut h);
    Some((h, pic))
}

/// Where and at what scale the picture is drawn, once it has been still
/// for a few frames (the wheel's scroll is smoothed over some).
fn still(h: &mut Harness<'_, Moonglow>, key: ResKey) -> (egui::Rect, f32) {
    let started = std::time::Instant::now();
    let mut last = None;
    let mut same = 0;
    loop {
        h.run_steps(2);
        let now = mg_ui::viewers::picture_drawn(&h.ctx, key).expect("drawn");
        same = if last == Some(now) { same + 1 } else { 0 };
        if same >= 4 {
            return now;
        }
        last = Some(now);
        assert!(started.elapsed() < SETTLE_TIME, "never still: {now:?}");
    }
}

fn wheel_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, notches: f32) {
    h.hover_at(at);
    h.run_steps(2);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 40.0 * notches),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
}

/// The picture's pixel under `at`, given where it is drawn.
fn texel(shown: egui::Rect, scale: f32, at: egui::Pos2) -> egui::Vec2 {
    (at - shown.min) / scale
}

/// The room the picture is zoomed in: what its node covers once it is
/// larger than the tab.
fn room_of(h: &Harness<'_, Moonglow>) -> egui::Rect {
    in_tab(h, "mg_w6zoom.tga, 2048 by 1024 pixels").expect("the picture").rect()
}

#[test]
fn the_wheel_zooms_a_picture_about_the_pointer() {
    let Some((mut h, key)) = zoomable("ui-zoom-wheel") else { return };
    let (fit_rect, fit) = still(&mut h, key);
    assert!(fit < 1.0, "fitted, smaller than its pixels: {fit}");
    assert!(in_tab(&h, "Zoom 100%").is_none() && h.query_by_label_contains("Zoom ").is_none());
    // A quarter of the way in from its left, on its middle line.
    let at = egui::pos2(fit_rect.min.x + fit_rect.width() * 0.25, fit_rect.center().y);
    let under = texel(fit_rect, fit, at);
    wheel_at(&mut h, at, 3.0);
    let (shown, scale) = still(&mut h, key);
    let want = fit * 1.25f32.powf(3.0);
    assert!((scale - want).abs() < 0.02, "three notches: {scale} for {want}");
    let now = texel(shown, scale, at);
    assert!((now - under).length() < 1.5, "the pixel under the pointer stays: {under} {now}");
    let percent = format!("Zoom {}%", (scale * 100.0).round());
    assert!(h.query_by_label(&percent).is_some(), "said: {percent}");
    // Back out, to its fit and no further.
    wheel_at(&mut h, at, -30.0);
    let (_, scale) = still(&mut h, key);
    assert!((scale - fit).abs() < 1e-3, "no smaller than fitted: {scale}");
    assert!(h.query_by_label_contains("Zoom ").is_none(), "no percent while fitted");
    // In, to 16 times and no more.
    wheel_at(&mut h, at, 60.0);
    let (_, scale) = still(&mut h, key);
    assert!((scale - 16.0).abs() < 1e-3, "no more than 16 times: {scale}");
    assert!(h.query_by_label("Zoom 1600%").is_some());
}

#[test]
fn the_wheel_over_the_picture_does_not_scroll_beyond_it() {
    let Some((mut h, key)) = zoomable("ui-zoom-no-scroll") else { return };
    let (rect, _) = still(&mut h, key);
    wheel_at(&mut h, rect.center(), 2.0);
    h.run_steps(1);
    let left = h.ctx.input(|i| i.smooth_scroll_delta);
    assert_eq!(left, egui::Vec2::ZERO, "the picture took the wheel");
    // Over the header, not the picture: nothing zooms.
    let (_, before) = still(&mut h, key);
    let header = h.get_by_label(mg_ui::RESOURCE_HEADER).rect().center();
    wheel_at(&mut h, header, 3.0);
    let (_, after) = still(&mut h, key);
    assert_eq!(before, after, "the wheel elsewhere leaves it");
}

#[test]
fn dragging_pans_a_zoomed_picture_and_stays_over_its_room() {
    let Some((mut h, key)) = zoomable("ui-zoom-drag") else { return };
    let (rect, fit) = still(&mut h, key);
    // Fitted, there is nothing to pan: a drag leaves it.
    drag(&mut h, rect.center(), rect.center() + egui::vec2(-80.0, 0.0));
    assert_eq!(still(&mut h, key).0, rect, "fitted, a drag does nothing");
    wheel_at(&mut h, rect.center(), 6.0);
    let (shown, scale) = still(&mut h, key);
    assert!(scale > fit * 3.0);
    let room = room_of(&h);
    assert!(shown.contains_rect(room.shrink(0.5)), "covers its room: {shown:?} {room:?}");
    // A small drag moves it by as much.
    let from = room.center();
    drag(&mut h, from, from + egui::vec2(-60.0, 20.0));
    let (moved, _) = still(&mut h, key);
    assert!(
        ((moved.min - shown.min) - egui::vec2(-60.0, 20.0)).length() < 2.0,
        "moved with the pointer: {:?}",
        moved.min - shown.min
    );
    // A great one stops at its edge: the room stays covered.
    drag(&mut h, from, from + egui::vec2(-6000.0, 0.0));
    let (far, _) = still(&mut h, key);
    assert!((far.max.x - room.max.x).abs() < 1.0, "its right edge at the room's: {far:?}");
    drag(&mut h, from, from + egui::vec2(6000.0, 6000.0));
    let (far, _) = still(&mut h, key);
    assert!((far.min.x - room.min.x).abs() < 1.0 && (far.min.y - room.min.y).abs() < 1.0);
}

#[test]
fn a_double_click_goes_between_fitted_and_actual_pixels() {
    let Some((mut h, key)) = zoomable("ui-zoom-double") else { return };
    let (rect, fit) = still(&mut h, key);
    // On the picture's middle, a little to the right.
    let at = egui::pos2(rect.center().x + 60.0, rect.center().y + 10.0);
    let under = texel(rect, fit, at);
    double_click_now(&mut h, at);
    let (shown, scale) = still(&mut h, key);
    assert!((scale - 1.0).abs() < 1e-4, "actual pixels: {scale}");
    assert!(h.query_by_label("Zoom 100%").is_some());
    // The point double clicked is in the middle of the room.
    let room = room_of(&h);
    let centre = texel(shown, scale, room.center());
    assert!((centre - under).length() < 1.5, "centered on it: {centre} for {under}");
    // From there (or any zoom but the fit), a double click fits again.
    double_click_now(&mut h, room.center());
    let (back, scale) = still(&mut h, key);
    assert_eq!((back, scale), (rect, fit), "fitted again");
    assert!(h.query_by_label_contains("Zoom ").is_none());
    wheel_at(&mut h, at, 2.0);
    still(&mut h, key);
    double_click_now(&mut h, at);
    assert_eq!(still(&mut h, key).1, fit, "from another zoom, it fits");
}

#[test]
fn the_keys_zoom_and_fit_a_picture_under_the_pointer() {
    let Some((mut h, key)) = zoomable("ui-zoom-keys") else { return };
    let (rect, fit) = still(&mut h, key);
    h.hover_at(rect.center());
    h.run_steps(2);
    for k in [egui::Key::Plus, egui::Key::Plus] {
        h.key_press(k);
        h.run_steps(2);
    }
    let scale = still(&mut h, key).1;
    assert!((scale - fit * 1.25 * 1.25).abs() < 1e-3, "two presses of +: {scale}");
    h.key_press(egui::Key::Minus);
    h.run_steps(2);
    assert!((still(&mut h, key).1 - fit * 1.25).abs() < 1e-3, "one of -");
    h.key_press(egui::Key::Num0);
    h.run_steps(2);
    assert_eq!(still(&mut h, key).1, fit, "0 fits");
    // Not under the pointer elsewhere.
    h.hover_at(h.get_by_label(mg_ui::RESOURCE_HEADER).rect().center());
    h.run_steps(2);
    h.key_press(egui::Key::Plus);
    h.run_steps(2);
    assert_eq!(
        still(&mut h, key).1,
        fit,
        "the keys are the picture's own, while it is under the pointer"
    );
}

#[test]
fn another_picture_starts_fitted() {
    let Some((mut h, key)) = zoomable("ui-zoom-reset") else { return };
    let (rect, fit) = still(&mut h, key);
    wheel_at(&mut h, rect.center(), 5.0);
    assert!(still(&mut h, key).1 > fit);
    let other = w6_key("mg_w6zoomb.tga");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(other)));
    settle(&mut h);
    let started = std::time::Instant::now();
    while mg_ui::viewers::picture_drawn(&h.ctx, other).is_none() {
        assert!(started.elapsed() < SETTLE_TIME, "never drawn");
        h.run_steps(2);
    }
    let (_, scale) = still(&mut h, other);
    assert_eq!(scale, fit, "fitted");
    assert!(h.query_by_label_contains("Zoom ").is_none(), "no percent");
}
