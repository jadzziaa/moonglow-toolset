//! The custom palettes (`<type>palcus.itp`) compared with Aurora's, after
//! the same operations (El Grillo's and Jasperre's report: Moonglow deleted
//! the entries of their haks' blueprints, turned STRREFs into names and
//! turned every commit's palettes upside down).
//!
//! The oracle captures in `palettes/` were made with
//! `tools/aurora/capture_palettes.py` (it says how): a module whose item
//! palette has entries out of order, talk-table names (`STRREF`), a hak's
//! blueprints (`mgpal.hak`), the override folder's (`ov_item.uti`), one
//! there is no more, a stale name, a field of the builder's own and an
//! entry in the wrong category (`before.mod`); Aurora's plain save of it
//! (`saveas.mod`); then, one after the other, Edit and OK on the stale one
//! (`edit-ok.mod`), on the two talk-table ones (`edit-strref.mod`,
//! `edit-strname.mod`) and on the misplaced one (`edit-moved.mod`), Edit
//! Copy (`edit-copy.mod`), Refresh Palette (`refresh.mod`) and Delete
//! (`delete.mod`); and the module without its item palette
//! (`nopal-before.mod`), saved (`nopal-saveas.mod`) and built with Build's
//! Palettes alone (`nopal-build.mod`).

use std::collections::BTreeMap;

use mg_core::ResType;
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::palette::{
    BlueprintKind, rebuild_custom_palette, update_custom_palette, update_custom_palettes,
};
use mg_resman::{ErfContainer, GameInstall, LayerClass, MemContainer, ResKey, priority};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

use crate::aurora_common::{PLAIN, lines};

/// How the GFFs are compared, as lines.
const SHOWN: crate::aurora_common::Shown<'static> = PLAIN;

/// The game, with the probe's hak and the override folder's blueprint.
fn game(root: &std::path::Path) -> Option<GameData> {
    let mut game = GameData::open(&GameInstall::new(root, None, "en")).ok()?;
    let hak = ErfContainer::open(&mg_testkit::aurora_capture("palettes/mgpal.hak")?).ok()?;
    game.resman.add(priority::HAK_USER, "mgpal", LayerClass::Erf, hak);
    let mut over = MemContainer::new();
    let ov = std::fs::read(mg_testkit::aurora_capture("palettes/ov_item.uti")?).ok()?;
    over.insert(ResKey::parse("ov_item", ResType::UTI).unwrap(), ov);
    game.resman.add(priority::OVERRIDE, "override", LayerClass::Directory, over);
    Some(game)
}

fn capture(name: &str) -> Option<Module> {
    Module::open(&mg_testkit::aurora_capture(&format!("palettes/{name}.mod"))?).ok()
}

fn palette(m: &Module, kind: BlueprintKind) -> Gff {
    m.gff(&kind.custom_key()).unwrap().unwrap()
}

fn differences(what: &str, ours: &Gff, aurora: &Gff) -> Option<String> {
    let (a, b) = (lines(&ours.root, SHOWN), lines(&aurora.root, SHOWN));
    (a != b).then(|| {
        let first = a.iter().zip(&b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
        let from = first.saturating_sub(3);
        format!(
            "{what}: {} vs {} lines, from line {first}:\n  ours:   {:?}\n  aurora: {:?}",
            a.len(),
            b.len(),
            &a[from..(first + 4).min(a.len())],
            &b[from..(first + 4).min(b.len())]
        )
    })
}

/// The module `ours` with the item blueprints `next` has (Aurora's, after
/// an operation): those it changed, added and deleted.
fn with_blueprints_of(ours: &Module, next: &Module) -> Module {
    let mut now = ours.clone();
    let gone: Vec<ResKey> =
        ours.keys_of(ResType::UTI).filter(|k| !next.contains(k)).copied().collect();
    for k in gone {
        now.remove(&k);
    }
    for k in next.keys_of(ResType::UTI) {
        now.set(*k, next.get(k).unwrap().to_vec());
    }
    now
}

/// Opened and saved with nothing changed, the palette is as it was, byte
/// for byte. Aurora's plain save keeps every entry too (the hak's, the
/// override folder's, the one there is no more, the talk-table names, the
/// stale name, the misplaced one), but puts each category in order by
/// name and drops the field it doesn't know: Moonglow does neither, so
/// that a commit has no change where the builder made none.
#[test]
fn a_plain_save_keeps_every_entry() {
    let root = corpus!();
    let before = Module::open(&aurora_capture!("palettes/before.mod")).unwrap();
    let (Some(game), Some(aurora)) = (game(&root), capture("saveas")) else {
        panic!("the palette captures are incomplete");
    };
    let mut saved = before.clone();
    assert_eq!(update_custom_palettes(&mut saved, &before, &game), Ok(0));
    let key = BlueprintKind::Item.custom_key();
    assert_eq!(saved.get(&key), before.get(&key), "byte for byte");
    // The same entries as Aurora's, each as it is (but the field Aurora
    // drops), in the order the module has them.
    let entries = |g: &Gff| -> BTreeMap<String, Vec<String>> {
        let mut out = BTreeMap::new();
        fn walk(list: &[Struct], out: &mut BTreeMap<String, Vec<String>>) {
            for n in list {
                if let Some(r) = n.resref("RESREF") {
                    let mut fields: Vec<String> = n
                        .fields
                        .iter()
                        .filter(|f| f.label.as_bytes() != b"XTRA")
                        .map(|f| format!("{} {:?}", f.label.to_string_lossy(), f.value))
                        .collect();
                    fields.sort();
                    out.insert(r.to_string(), fields);
                }
                walk(n.list("LIST").unwrap_or(&[]), out);
            }
        }
        walk(g.root.list("MAIN").unwrap(), &mut out);
        out
    };
    let ours = palette(&saved, BlueprintKind::Item);
    assert_eq!(entries(&ours), entries(&palette(&aurora, BlueprintKind::Item)));
    for resref in ["hk_strref", "hk_text", "hk_moved", "gh_missing", "ov_item", "md_strref"] {
        assert!(entries(&ours).contains_key(resref), "{resref} kept");
    }
}

/// Each operation, one after the other from Aurora's saved palette, changes
/// the palette as Aurora's did: an edited blueprint whose entry is stale or
/// misplaced gets its entry made right at its place by name; one whose
/// entry still says what it does (a talk-table name too) leaves it alone;
/// a copy gets an entry after the one of the same name; a deleted one
/// takes its entry alone. One difference: a new entry for a talk-table
/// name is its STRREF (as Aurora's Build and Refresh Palette write it),
/// where Aurora's Edit Copy writes the text.
#[test]
fn each_operation_changes_its_entry_as_aurora_does() {
    let root = corpus!();
    let _ = aurora_capture!("palettes/saveas.mod");
    let Some(game) = game(&root) else { panic!("the palette captures are incomplete") };
    let kind = BlueprintKind::Item;
    let mut ours = capture("saveas").unwrap();
    let mut failures = Vec::new();
    for step in ["edit-ok", "edit-strref", "edit-strname", "edit-moved", "edit-copy"] {
        let aurora = capture(step).unwrap_or_else(|| panic!("no capture {step}"));
        let mut now = with_blueprints_of(&ours, &aurora);
        let made = update_custom_palette(&now, &ours, &game, kind).unwrap();
        if let Some(g) = &made {
            now.set_gff(kind.custom_key(), g).unwrap();
        }
        let (mut mine, theirs) = (palette(&now, kind), palette(&aurora, kind));
        if step == "edit-copy" {
            // The copy's entry: its STRREF, Aurora's the text.
            let copy = entry_mut(&mut mine, "md_strref001");
            assert_eq!(copy.integer("STRREF"), Some(13411));
            let text = game.string(mg_core::StrRef(13411)).unwrap();
            copy.remove("STRREF");
            copy.fields.insert(0, field("NAME", Value::String(text.into_bytes())));
        }
        let changes = !matches!(step, "edit-strref" | "edit-strname");
        assert_eq!(made.is_some(), changes, "{step}: whether the palette changed");
        failures.extend(differences(step, &mine, &theirs));
        ours = now;
    }

    // Refresh Palette (and Build's Palettes): made anew, as Aurora's.
    let refreshed = capture("refresh").unwrap();
    let built = rebuild_custom_palette(&ours, &game, kind).unwrap();
    failures.extend(differences("refresh", &built, &palette(&refreshed, kind)));

    // Delete, after the refresh.
    let deleted = capture("delete").unwrap();
    let now = with_blueprints_of(&refreshed, &deleted);
    let made = update_custom_palette(&now, &refreshed, &game, kind).unwrap().expect("one gone");
    failures.extend(differences("delete", &made, &palette(&deleted, kind)));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Build's Palettes on a module without its item palette makes every
/// palette as Aurora's Build did: the module's and the hak's blueprints, a
/// talk-table name as its STRREF.
#[test]
fn build_makes_every_palette_as_aurora_s_build() {
    let root = corpus!();
    let before = Module::open(&aurora_capture!("palettes/nopal-before.mod")).unwrap();
    let (Some(game), Some(aurora)) = (game(&root), capture("nopal-build")) else {
        panic!("the palette captures are incomplete");
    };
    let mut failures = Vec::new();
    for kind in BlueprintKind::ALL {
        let built = rebuild_custom_palette(&before, &game, kind).unwrap();
        failures.extend(differences(kind.name(), &built, &palette(&aurora, kind)));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Add to Palette on the longsword lying in the probe's area (a module
/// whose palette lists the hak's blueprints, the override folder's and
/// one there is no more): the new blueprint gets its entry (its talk-table
/// name as its STRREF), and every other entry is as it was.
#[test]
fn add_to_palette_keeps_the_other_entries() {
    let root = corpus!();
    let before = Module::open(&aurora_capture!("palettes/before.mod")).unwrap();
    let Some(game) = game(&root) else { panic!("the palette captures are incomplete") };
    let area = before.gff(&ResKey::parse("field", ResType::GIT).unwrap()).unwrap().unwrap();
    let sword = area.root.list("List").unwrap()[0].clone();
    let read = |k: ResKey| match before.gff(&k) {
        Some(g) => g.ok().map(|g| g.root),
        None => Gff::read(&game.resman.get(&k).ok()?).ok().map(|g| g.root),
    };
    let taken = |k: &ResKey| before.contains(k) || game.resman.contains(k);
    let added = mg_module::palette_add::add_to_palette(
        &game,
        ResType::UTI,
        &sword,
        &read,
        &taken,
        &Default::default(),
    )
    .unwrap();
    let mut now = before.clone();
    for (key, g) in &added.blueprints {
        now.set_gff(*key, g).unwrap();
    }
    let new = added.blueprints[0].0.resref;
    assert_eq!(update_custom_palettes(&mut now, &before, &game), Ok(1));
    let (was, made) = (palette(&before, BlueprintKind::Item), palette(&now, BlueprintKind::Item));
    // The new entry, then the palette as it was but for it.
    let mut taken_out = made.clone();
    let entry = remove_entry(&mut taken_out, &new.to_string()).expect("the new entry");
    assert_eq!(entry.integer("STRREF"), Some(166), "Longsword, from the talk table");
    assert!(differences("the other entries", &taken_out, &was).is_none());
}

fn remove_entry(g: &mut Gff, resref: &str) -> Option<Struct> {
    fn walk(list: &mut Vec<Struct>, resref: &str) -> Option<Struct> {
        if let Some(i) =
            list.iter().position(|n| n.resref("RESREF").is_some_and(|r| r.to_string() == resref))
        {
            return Some(list.remove(i));
        }
        list.iter_mut().find_map(|n| walk(n.list_mut("LIST")?, resref))
    }
    walk(g.root.list_mut("MAIN")?, resref)
}

fn field(label: &str, value: Value) -> mg_gff::Field {
    let mut s = Struct::new(0);
    s.set(label, value);
    s.fields.remove(0)
}

fn entry_mut<'a>(g: &'a mut Gff, resref: &str) -> &'a mut Struct {
    fn walk<'a>(list: &'a mut [Struct], resref: &str) -> Option<&'a mut Struct> {
        for n in list {
            if n.resref("RESREF").is_some_and(|r| r.to_string() == resref) {
                return Some(n);
            }
            if let Some(children) = n.list_mut("LIST")
                && let Some(found) = walk(children, resref)
            {
                return Some(found);
            }
        }
        None
    }
    walk(g.root.list_mut("MAIN").unwrap(), resref).unwrap()
}
