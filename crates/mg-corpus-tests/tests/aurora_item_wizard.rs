//! The Item Wizard's magical items against Aurora's (the oracle capture
//! `item-wizard/`: 126 items Aurora's wizard made with Magical ticked, of
//! 33 base items at every item level and quality, listed with their level
//! and quality in `manifest.tsv`). Aurora's choices are random; what
//! `mg_rules::item_wizard` takes from them must hold for each: the
//! table, the first property's tier, the tiers and number of the rest,
//! the most the item is worth, the charges, the classes, and that it is
//! unidentified.

use mg_gff::Gff;
use mg_resman::GameInstall;
use mg_rules::item_wizard::{ItemMagic, most_worth};
use mg_rules::{GameData, ItemProperty};
use mg_testkit::{aurora_capture, corpus};

/// The tiers of a table's rows that are this property.
fn tiers(gd: &GameData, table: &str, p: &ItemProperty) -> Vec<i32> {
    let t = gd.table(table).unwrap();
    (0..t.len())
        .filter(|&r| {
            let int = |c: &str, none: i32| t.get_int(r, c).unwrap_or(none);
            (int("Prop", -1), int("SubProp", 65535), int("CostTable", 0), int("CostVal", 0))
                == (p.property.into(), p.subtype.into(), p.cost_table.into(), p.cost_value.into())
                && (int("ParamTable", 255), int("Param", 0))
                    == (p.param1.into(), p.param1_value.into())
        })
        .filter_map(|r| t.get_int(r, "Value"))
        .collect()
}

#[test]
fn magical_items_are_made_as_aurora_s() {
    let root = corpus!();
    let manifest = aurora_capture!("item-wizard/manifest.tsv");
    let dir = manifest.parent().unwrap();
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let manifest = std::fs::read_to_string(&manifest).unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in manifest.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        let (file, level, quality): (&str, u8, u8) =
            (f[0], f[1].parse().unwrap(), f[2].parse().unwrap());
        let g = Gff::read(&std::fs::read(dir.join(file)).unwrap()).unwrap();
        let base = g.root.integer("BaseItem").unwrap() as u32;
        let list: Vec<ItemProperty> =
            g.root.list("PropertiesList").unwrap().iter().map(ItemProperty::from_gff).collect();
        let what = format!("{file} (base item {base}, level {level}, quality {quality})");
        checked += 1;
        let Some(m) = ItemMagic::new(&gd, base) else {
            if !list.is_empty() {
                failures.push(format!("{what}: Aurora made it magical"));
            }
            continue;
        };
        if g.root.integer("Identified") != Some(0) {
            failures.push(format!("{what}: identified"));
        }
        if g.root.integer("Charges") != Some(m.charges().into()) {
            failures.push(format!("{what}: {:?} charges", g.root.integer("Charges")));
        }
        // The classes it is limited to, last.
        let own = list.len().saturating_sub(m.classes().len());
        let classes: Vec<u16> = list[own..].iter().map(|p| p.subtype).collect();
        if classes != m.classes() || list[own..].iter().any(|p| p.property != 63) {
            failures.push(format!("{what}: classes {classes:?}"));
        }
        if own == 0 || own > m.most(quality) {
            failures.push(format!("{what}: {own} properties"));
        }
        for (i, p) in list[..own].iter().enumerate() {
            let want = match i {
                0 => vec![m.tier(level, quality)],
                _ => m.later_tiers(level, quality),
            };
            let have = tiers(&gd, m.table(), p);
            if !have.iter().any(|t| want.contains(t)) {
                failures.push(format!(
                    "{what}: property {i} ({}) of tiers {have:?}, not {want:?}",
                    gd.property_text(p)
                ));
            }
        }
        // (The first property is kept whatever it is worth.)
        let cost = g.root.integer("Cost").unwrap();
        let cap = most_worth(&gd, level, quality).unwrap();
        if cost > i64::from(cap) && own > 1 {
            failures.push(format!("{what}: worth {cost}, more than {cap}"));
        }
    }
    assert!(checked >= 120, "{checked} items");
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
