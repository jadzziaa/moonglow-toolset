//! The Item Wizard's magical items (`mg_rules::item_wizard`) for every
//! base item the wizard lists, at every item level and quality: each
//! property one the base item can have (itemprops.2da), with a subtype,
//! value and parameter among those its tables hold, the item worth no
//! more than the level and quality allow.

use mg_core::ResRef;
use mg_module::blueprints::{self, make_magical};
use mg_resman::GameInstall;
use mg_rules::item_wizard::{ItemMagic, Magical, most_worth};
use mg_rules::{GameData, ItemProperty, ItemValue};
use mg_testkit::corpus;

/// Why a base item can't have a property, if it can't.
fn invalid(gd: &GameData, base: u32, p: &ItemProperty) -> Option<String> {
    let available = gd.available_properties(base);
    let Some(t) = available.iter().find(|t| t.row == p.property) else {
        return Some("not a property the base item can have".into());
    };
    let rows = |c: Vec<mg_rules::Choice>| c.into_iter().map(|c| c.row).collect::<Vec<_>>();
    if t.subtypes.is_some() && !rows(gd.property_subtypes(t, base)).contains(&(p.subtype as usize))
    {
        return Some(format!("subtype {}", p.subtype));
    }
    if p.cost_table != t.cost_table
        || t.cost_table != 0
            && !rows(gd.property_costs(t.cost_table, base)).contains(&(p.cost_value as usize))
    {
        return Some(format!("value {} of cost table {}", p.cost_value, p.cost_table));
    }
    let sub = if t.subtypes.is_some() { p.subtype } else { 0 };
    match gd.property_param_table(t, sub) {
        Some(table) if p.param1 != table => Some(format!("parameter table {}", p.param1)),
        Some(table) if !rows(gd.property_params(table)).contains(&(p.param1_value as usize)) => {
            Some(format!("parameter {}", p.param1_value))
        }
        None if p.param1 != 255 => Some(format!("parameter table {} (none)", p.param1)),
        _ => None,
    }
}

#[test]
fn magical_items_have_valid_properties_for_every_base_item() {
    let root = corpus!();
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let base_items = gd.table("baseitems").unwrap();
    let mut failures = Vec::new();
    let (mut magical, mut items, mut props) = (0, 0, 0);
    let mut tables = std::collections::BTreeSet::new();
    for base in (0..base_items.len() as u32)
        .filter(|&r| base_items.get_int(r as usize, "StorePanel").is_some())
    {
        let Some(m) = ItemMagic::new(&gd, base) else { continue };
        magical += 1;
        tables.insert(m.table());
        let plain = blueprints::item(&gd, ResRef::from_str("magic").unwrap(), "Magic", base, 0);
        for (level, quality, seed) in
            (0..4).flat_map(|l| (0..4).flat_map(move |q| (0..3).map(move |s| (l, q, s))))
        {
            let mut g = plain.clone();
            make_magical(&gd, &mut g, Magical { level, quality, seed });
            items += 1;
            let what = format!("base item {base}, level {level}, quality {quality}, seed {seed}");
            let list: Vec<ItemProperty> = g
                .root
                .list("PropertiesList")
                .unwrap_or(&[])
                .iter()
                .map(ItemProperty::from_gff)
                .collect();
            // (The classes it is limited to last.)
            let own = list.len().saturating_sub(m.classes().len());
            if own == 0 || own > m.most(quality) {
                failures.push(format!("{what}: {own} properties"));
            }
            for p in &list {
                props += 1;
                if let Some(why) = invalid(&gd, base, p) {
                    failures.push(format!("{what}: {}: {why}", gd.property_text(p)));
                }
            }
            let cost = gd.item_cost(&ItemValue::from_gff(&g.root));
            let cap = most_worth(&gd, level, quality).unwrap();
            // (Only the first is kept when nothing fits.)
            if cost > cap && own > 1 {
                failures.push(format!("{what}: worth {cost}, more than {cap}"));
            }
            if g.root.integer("Cost") != Some(cost.into())
                || g.root.integer("Identified") != Some(0)
            {
                failures.push(format!("{what}: Cost or Identified"));
            }
            if g.root.integer("Charges") != Some(m.charges().into()) {
                failures.push(format!("{what}: charges"));
            }
        }
    }
    eprintln!("{items} magical items of {magical} base items, {props} properties: {tables:?}");
    assert!(magical >= 60, "{magical} base items made magical");
    assert_eq!(tables.len(), 12, "every table used: {tables:?}");
    let shown: Vec<_> = failures.iter().take(40).cloned().collect();
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), shown.join("\n"));
}
