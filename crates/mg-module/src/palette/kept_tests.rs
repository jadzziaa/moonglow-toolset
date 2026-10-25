//! The custom palettes kept as Aurora keeps them: a save changes the
//! entries of the blueprints the builder changed and nothing else; Build
//! makes the palettes anew (El Grillo's and Jasperre's report: entries
//! deleted, STRREFs turned into names, every commit's palettes turned
//! upside down). The palette here is the Aurora probe's
//! (`tools/aurora/capture_palettes.py`): entries out of order, talk-table
//! names, a hak's blueprints, the override folder's, a blueprint there is
//! no more, a stale name, a field of the builder's own, an entry in the
//! wrong category.

use mg_core::{Gender, Language, LocString, ResRef, ResType, StrRef};
use mg_gff::{Gff, Struct, Value};
use mg_resman::{LayerClass, MemContainer, ResKey, ResMan, priority};
use mg_rules::GameData;

use super::*;
use crate::Module;

const LONGSWORD: u32 = 1;
const DROW_PLATE: u32 = 2;
const POTION: u32 = 3;
/// Category ids: the probe's, the game's longswords and claws.
const LONGSWORDS: u8 = 36;
const CLAWS: u8 = 13;

fn key(name: &str) -> ResKey {
    ResKey::parse(name, ResType::UTI).unwrap()
}

fn r(name: &str) -> ResRef {
    ResRef::from_str(name).unwrap()
}

/// An item blueprint named `name` (a talk-table string or text) in
/// `category`.
fn item(resref: &str, name: Result<u32, &str>, category: u8) -> Vec<u8> {
    let mut g = Gff::new(*b"UTI ");
    let name = match name {
        Ok(s) => LocString::from_strref(StrRef(s)),
        Err(t) => LocString::from_text(Language::ENGLISH, Gender::Male, t.as_bytes().to_vec()),
    };
    g.root.set("LocalizedName", Value::LocString(name));
    g.root.set("Tag", Value::String(resref.to_uppercase().into_bytes()));
    g.root.set("TemplateResRef", Value::resref(r(resref)));
    g.root.set("Comment", Value::String(Vec::new()));
    g.root.set("PaletteID", Value::Byte(category));
    g.to_bytes().unwrap()
}

fn node(fields: Vec<(&str, Value)>) -> Struct {
    let mut s = Struct::new(0);
    for (label, value) in fields {
        s.set(label, value);
    }
    s
}

fn named(resref: &str, name: &str) -> Struct {
    node(vec![("NAME", Value::String(name.into())), ("RESREF", Value::resref(r(resref)))])
}

fn strref(resref: &str, s: u32) -> Struct {
    node(vec![("STRREF", Value::Dword(s)), ("RESREF", Value::resref(r(resref)))])
}

/// The game: its talk table, its item skeleton (two categories), a hak
/// with three item blueprints and the override folder with one.
fn game() -> GameData {
    let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
    for text in
        ["", "Longsword", "Drow Full Plate +1", "Potion of Cure Light Wounds", "Longswords", "Claw"]
    {
        tlk.entries.push(mg_tlk::TlkEntry::text(text));
    }
    let mut skeleton = Gff::new(*b"ITP ");
    let category =
        |s: u32, id: u8| node(vec![("STRREF", Value::Dword(s)), ("ID", Value::Byte(id))]);
    skeleton.root.set("MAIN", Value::List(vec![category(4, LONGSWORDS), category(5, CLAWS)]));
    let mut base = MemContainer::new();
    // (The other types' skeletons: no categories.)
    for kind in BlueprintKind::ALL {
        let mut empty = Gff::new(*b"ITP ");
        empty.root.set("MAIN", Value::List(Vec::new()));
        base.insert(kind.skeleton_key(), empty.to_bytes().unwrap());
    }
    base.insert(ResKey::parse("itempal", ResType::ITP).unwrap(), skeleton.to_bytes().unwrap());
    let mut hak = MemContainer::new();
    hak.insert(key("hk_strref"), item("hk_strref", Ok(DROW_PLATE), LONGSWORDS));
    hak.insert(key("hk_text"), item("hk_text", Err("Hak Sword"), LONGSWORDS));
    hak.insert(key("hk_unlisted"), item("hk_unlisted", Err("Unlisted Hak Sword"), LONGSWORDS));
    let mut over = MemContainer::new();
    over.insert(key("ov_item"), item("ov_item", Err("Override Sword"), LONGSWORDS));
    let mut resman = ResMan::new();
    resman.add(priority::KEY, "base", LayerClass::Key, base);
    resman.add(priority::HAK_USER, "mgpal", LayerClass::Erf, hak);
    resman.add(priority::OVERRIDE, "override", LayerClass::Directory, over);
    GameData::new(resman, tlk)
}

/// The probe module: its blueprints and its custom palette, as the
/// builders' palettes have them.
fn module() -> Module {
    let mut m = Module::new();
    // (The other types' palettes: empty.)
    for kind in BlueprintKind::ALL {
        let mut empty = Gff::new(*b"ITP ");
        empty.root.set("MAIN", Value::List(Vec::new()));
        m.set_gff(kind.custom_key(), &empty).unwrap();
    }
    m.set(key("md_text"), item("md_text", Err("Module Sword"), LONGSWORDS));
    m.set(key("md_strref"), item("md_strref", Ok(POTION), LONGSWORDS));
    m.set(key("md_strname"), item("md_strname", Ok(LONGSWORD), LONGSWORDS));
    m.set(key("md_unlisted"), item("md_unlisted", Err("Unlisted Module Sword"), LONGSWORDS));
    m.set(key("md_moved"), item("md_moved", Err("Moved Module Sword"), LONGSWORDS));
    let odd = node(vec![
        ("XTRA", Value::Dword(7)),
        ("NAME", Value::String(b"Hak Sword".to_vec())),
        ("RESREF", Value::resref(r("hk_text"))),
    ]);
    let longswords = vec![
        named("md_text", "Stale Sword Name"),
        strref("hk_strref", DROW_PLATE),
        named("gh_missing", "Ghost Sword"),
        strref("md_strref", POTION),
        odd,
        named("md_strname", "Longsword"),
        named("ov_item", "Override Sword"),
    ];
    let claws = vec![named("md_moved", "Moved Module Sword")];
    let category = |s: u32, id: u8, list: Vec<Struct>| {
        node(vec![
            ("STRREF", Value::Dword(s)),
            ("ID", Value::Byte(id)),
            ("LIST", Value::List(list)),
        ])
    };
    let mut palette = Gff::new(*b"ITP ");
    palette.root.set(
        "MAIN",
        Value::List(vec![category(5, CLAWS, claws), category(4, LONGSWORDS, longswords)]),
    );
    m.set_gff(BlueprintKind::Item.custom_key(), &palette).unwrap();
    m
}

/// A palette's entries: each category's id with its entries' ResRefs, in
/// order.
fn listing(g: &Gff) -> Vec<(u8, Vec<String>)> {
    g.root
        .list("MAIN")
        .unwrap()
        .iter()
        .map(|c| {
            let entries = c.list("LIST").unwrap_or(&[]);
            let resrefs = entries.iter().map(|e| e.resref("RESREF").unwrap().to_string());
            (c.integer("ID").unwrap() as u8, resrefs.collect())
        })
        .collect()
}

fn entry<'a>(g: &'a Gff, resref: &str) -> &'a Struct {
    let mut found = Vec::new();
    entries_of(g.root.list("MAIN").unwrap(), r(resref), &mut Vec::new(), &mut found);
    node_at(g.root.list("MAIN").unwrap(), &found[0]).unwrap()
}

/// The blueprint edited (its comment), its name and category as they were.
fn edit(m: &mut Module, resref: &str) {
    let mut g = Gff::read(m.get(&key(resref)).unwrap()).unwrap();
    g.root.set("Comment", Value::String(b"edited".to_vec()));
    m.set_gff(key(resref), &g).unwrap();
}

fn palette_of(m: &Module) -> Gff {
    m.gff(&BlueprintKind::Item.custom_key()).unwrap().unwrap()
}

/// `module` changed by `change`, its palette brought up to date (as a save
/// does): the palette, or `None` if it was left as it was.
fn after(change: impl FnOnce(&mut Module)) -> Option<Gff> {
    let (game, before) = (game(), module());
    let mut now = before.clone();
    change(&mut now);
    update_custom_palette(&now, &before, &game, BlueprintKind::Item).unwrap()
}

#[test]
fn a_palette_whose_blueprints_did_not_change_is_left_as_it_is() {
    let (game, before) = (game(), module());
    let mut now = before.clone();
    // Something else changed: an area.
    now.set(ResKey::parse("area1", ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
    let bytes = now.get(&BlueprintKind::Item.custom_key()).unwrap().to_vec();
    assert_eq!(update_custom_palettes(&mut now, &before, &game), Ok(0));
    assert_eq!(now.get(&BlueprintKind::Item.custom_key()).unwrap(), &bytes[..], "byte for byte");
    // A blueprint written again as it was is no change either.
    let same = now.get(&key("md_text")).unwrap().to_vec();
    now.set(key("md_text"), same);
    assert_eq!(update_custom_palettes(&mut now, &before, &game), Ok(0));
    // An edit that leaves the entry as it was: a talk-table name stays a
    // STRREF (and a name written out for a talk-table one stays so), as
    // Aurora leaves them.
    assert_eq!(after(|m| edit(m, "md_strref")), None);
    assert_eq!(after(|m| edit(m, "md_strname")), None);
    // Nor are the entries of the hak's blueprints, the override folder's
    // or the one there is no more.
    assert_eq!(
        after(|m| edit(m, "md_unlisted")).map(|g| entry(&g, "gh_missing").clone()),
        Some(named("gh_missing", "Ghost Sword"))
    );
}

#[test]
fn an_edited_blueprint_changes_its_entry_alone() {
    let before = palette_of(&module());
    // Named anew (Aurora: Edit, OK): its entry has its name, at its place
    // by name; every other entry is as it was, the builder's field too.
    let renamed = after(|m| edit(m, "md_text")).expect("the stale name made right");
    assert_eq!(
        listing(&renamed),
        [
            (CLAWS, vec!["md_moved".into()]),
            (
                LONGSWORDS,
                [
                    "hk_strref",
                    "gh_missing",
                    "md_strref",
                    "hk_text",
                    "md_strname",
                    "md_text",
                    "ov_item"
                ]
                .map(String::from)
                .to_vec()
            ),
        ]
    );
    assert_eq!(entry(&renamed, "md_text").string("NAME"), Some(&b"Module Sword"[..]));
    for other in ["hk_strref", "gh_missing", "md_strref", "hk_text", "md_strname", "ov_item"] {
        assert_eq!(entry(&renamed, other), entry(&before, other), "{other}");
    }
    assert_eq!(entry(&renamed, "hk_text").integer("XTRA"), Some(7));

    // Moved to another category (Aurora: Edit, OK on one whose entry is
    // in the wrong category): out of that one, into its own by name.
    let moved = after(|m| edit(m, "md_moved")).unwrap();
    let listed = listing(&moved);
    assert_eq!(listed[0], (CLAWS, vec![]));
    let at = |name: &str| listed[1].1.iter().position(|x| x == name).unwrap();
    assert_eq!(at("md_moved"), at("md_strname") + 1, "Longsword < Moved Module Sword < Override");

    // Hidden (category 255): its entry goes.
    let hidden = after(|m| m.set(key("md_strref"), item("md_strref", Ok(POTION), 255))).unwrap();
    assert!(!listing(&hidden)[1].1.contains(&"md_strref".to_string()));
    assert_eq!(listing(&hidden)[1].1.len(), 6);
}

#[test]
fn a_new_blueprint_gets_an_entry_at_its_place_by_name() {
    // A talk-table name: its STRREF, as Aurora's Build writes it (Aurora's
    // Edit Copy writes the text; a STRREF is what both keep), after the
    // last entry whose name does not come after it (the category is out
    // of order: the others stay as they are).
    let added =
        after(|m| m.set(key("md_strref001"), item("md_strref001", Ok(POTION), 36))).unwrap();
    let listed = listing(&added);
    assert_eq!(listed[1].1[..7], listing(&palette_of(&module()))[1].1[..]);
    assert_eq!(listed[1].1[7], "md_strref001", "after Override Sword");
    let new = entry(&added, "md_strref001");
    assert_eq!(new.integer("STRREF"), Some(i64::from(POTION)));
    assert!(new.get("NAME").is_none());
    assert_eq!(
        new.fields.iter().map(|f| f.label.to_string_lossy()).collect::<Vec<_>>(),
        ["STRREF", "RESREF"]
    );
    // A name of its own: NAME, in the bytes the blueprint has.
    let added = after(|m| m.set(key("md_new"), item("md_new", Err("Axe of Doom"), 36))).unwrap();
    let new = entry(&added, "md_new");
    assert_eq!(new.string("NAME"), Some(&b"Axe of Doom"[..]));
    assert_eq!(listing(&added)[1].1[0], "md_new", "Axe < Drow");
}

#[test]
fn a_deleted_blueprint_takes_its_entry_alone() {
    let gone = after(|m| {
        m.remove(&key("md_strname"));
    })
    .unwrap();
    assert_eq!(
        listing(&gone)[1].1,
        ["md_text", "hk_strref", "gh_missing", "md_strref", "hk_text", "ov_item"]
    );
    // One a hak still has keeps its entry (the hak's blueprint's now).
    let before = module();
    let mut with_copy = before.clone();
    with_copy.set(key("hk_text"), item("hk_text", Err("Hak Sword"), LONGSWORDS));
    let mut now = with_copy.clone();
    now.remove(&key("hk_text"));
    assert_eq!(update_custom_palette(&now, &with_copy, &game(), BlueprintKind::Item), Ok(None));
    // Renamed (deleted and added as another, same name and category): the
    // new ResRef in the old entry's place, its STRREF kept.
    let renamed = after(|m| {
        m.remove(&key("md_strref"));
        m.set(key("md_potion"), item("md_potion", Ok(POTION), LONGSWORDS));
    })
    .unwrap();
    assert_eq!(listing(&renamed)[1].1[3], "md_potion");
    assert_eq!(entry(&renamed, "md_potion").integer("STRREF"), Some(i64::from(POTION)));
}

/// `mg rename` (and the module tree's Rename) renames the blueprint's
/// entry where it is; the save after it changes nothing more.
#[test]
fn a_renamed_blueprint_keeps_its_entry() {
    let (game, before) = (game(), module());
    let mut now = before.clone();
    crate::rename::rename(&mut now, key("md_strref"), r("md_potion"), false).unwrap();
    let renamed = palette_of(&now);
    assert_eq!(listing(&renamed)[1].1[3], "md_potion");
    assert_eq!(entry(&renamed, "md_potion"), &strref("md_potion", POTION));
    assert_eq!(update_custom_palette(&now, &before, &game, BlueprintKind::Item), Ok(None));
}

/// The module's categories edited: the categories are made again from its
/// skeleton, each with the entries it had, as they were.
#[test]
fn new_categories_keep_their_entries() {
    let mut skeleton = Gff::new(*b"ITP ");
    let category = |name: &[u8], id: u8| {
        node(vec![("NAME", Value::String(name.to_vec())), ("ID", Value::Byte(id))])
    };
    skeleton.root.set(
        "MAIN",
        Value::List(vec![
            category(b"Blades", LONGSWORDS),
            category(b"Ruins", 70),
            category(b"Claws", CLAWS),
        ]),
    );
    let before = palette_of(&module());
    let made =
        after(|m| m.set_gff(BlueprintKind::Item.skeleton_key(), &skeleton).unwrap()).unwrap();
    let listed = listing(&made);
    assert_eq!(listed.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [LONGSWORDS, 70, CLAWS]);
    assert_eq!(listed[0].1, listing(&before)[1].1, "the entries as they were, in their order");
    assert_eq!(entry(&made, "hk_text"), entry(&before, "hk_text"));
    assert_eq!(made.root.list("MAIN").unwrap()[0].string("NAME"), Some(&b"Blades"[..]));
}

/// Build's Palettes (and Refresh Palette) makes the palette anew, as
/// Aurora's does: the module's and the hak's blueprints, a talk-table name
/// as its STRREF, by name then ResRef; the override folder's blueprint and
/// the one there is no more are not listed.
#[test]
fn build_makes_the_palette_anew_from_the_module_and_its_haks() {
    let (game, m) = (game(), module());
    let built = rebuild_custom_palette(&m, &game, BlueprintKind::Item).unwrap();
    let listed = listing(&built);
    assert_eq!(
        listed,
        [
            (CLAWS, vec![]),
            (
                LONGSWORDS,
                [
                    "hk_strref",   // Drow Full Plate +1
                    "hk_text",     // Hak Sword
                    "md_strname",  // Longsword
                    "md_text",     // Module Sword
                    "md_moved",    // Moved Module Sword
                    "md_strref",   // Potion of Cure Light Wounds
                    "hk_unlisted", // Unlisted Hak Sword
                    "md_unlisted", // Unlisted Module Sword
                ]
                .map(String::from)
                .to_vec()
            ),
        ]
    );
    assert_eq!(entry(&built, "hk_strref"), &strref("hk_strref", DROW_PLATE));
    assert_eq!(entry(&built, "md_strname"), &strref("md_strname", LONGSWORD));
    assert_eq!(entry(&built, "hk_text"), &named("hk_text", "Hak Sword"));
    // A module without the palette gets it made so at a save.
    let mut bare = m.clone();
    bare.remove(&BlueprintKind::Item.custom_key());
    let made = update_custom_palette(&bare, &bare, &game, BlueprintKind::Item).unwrap();
    assert_eq!(made, Some(built));
}

/// The team commits nasher's JSON: a save that changes nothing leaves the
/// palette's file as it is, and one blueprint changed changes its entry
/// alone in it.
#[test]
fn a_nasher_project_s_palette_file_changes_only_where_a_blueprint_did() {
    let root = mg_testkit::scratch_dir("mg-module-palette-nasher");
    let game = game();
    let mut m = module();
    m.save_as(&crate::ModuleLocation::Project { root: root.clone(), target: "default".into() })
        .unwrap();
    fn find(dir: &std::path::Path) -> Option<std::path::PathBuf> {
        for e in std::fs::read_dir(dir).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                if let Some(f) = find(&p) {
                    return Some(f);
                }
            } else if p.to_string_lossy().ends_with("itempalcus.itp.json") {
                return Some(p);
            }
        }
        None
    }
    let file = find(&root).expect("the palette's file");
    let written = std::fs::read(&file).unwrap();

    // Opened and saved with nothing changed: the file as it was.
    let mut m = Module::open(&root).unwrap();
    let opened = m.clone();
    assert_eq!(update_custom_palettes(&mut m, &opened, &game), Ok(0));
    m.save().unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), written);

    // One blueprint renamed: its entry alone changes.
    m.set(key("md_text"), item("md_text", Err("Module Sword"), LONGSWORDS));
    assert_eq!(update_custom_palettes(&mut m, &opened, &game), Ok(1));
    m.save().unwrap();
    let text = String::from_utf8(std::fs::read(&file).unwrap()).unwrap();
    let was = String::from_utf8(written).unwrap();
    assert_ne!(text, was);
    assert!(!text.contains("Stale Sword Name") && text.contains("Module Sword"));
    assert_eq!(text.matches("STRREF").count(), was.matches("STRREF").count(), "STRREFs kept");
    assert!(text.contains("XTRA") && text.contains("gh_missing") && text.contains("ov_item"));
    // Read back, it is the palette the save made.
    let back = Module::open(&root).unwrap();
    assert_eq!(listing(&palette_of(&back)), listing(&palette_of(&m)));
}
