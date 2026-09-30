//! The item property model against the game's items: every property of
//! every base-game item has a type and a text, and those of items made in
//! the toolset (not creature weapons and hides, which carry hand-made
//! values) have a subtype, value and parameter among those the editor
//! offers for that item.

use mg_core::ResType;
use mg_gff::Gff;
use mg_resman::{GameInstall, ResKey};
use mg_rules::{GameData, ItemProperty};
use mg_testkit::corpus;

#[test]
fn shipped_item_properties_are_offered() {
    let root = corpus!();
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let base_items = gd.table("baseitems").unwrap();
    let creature_item = |base: u32| {
        let slots = base_items.get_int(base as usize, "EquipableSlots").unwrap_or(0);
        slots != 0 && slots & !0x3_C000 == 0
    };
    let mut failures = Vec::new();
    let (mut props, mut items) = (0, 0);
    for r in gd.resman.list(ResType::UTI) {
        let Ok(data) = gd.resman.get(&ResKey::new(r, ResType::UTI)) else { continue };
        let Ok(g) = Gff::read(&data) else { continue };
        let base = g.root.integer("BaseItem").unwrap_or(0) as u32;
        items += 1;
        let available = gd.available_properties(base);
        for s in g.root.list("PropertiesList").unwrap_or(&[]) {
            props += 1;
            let p = ItemProperty::from_gff(s);
            let Some(t) = gd.property_type(p.property) else {
                failures.push(format!("{r}: no property type {}", p.property));
                continue;
            };
            let text = gd.property_text(&p);
            if text.contains('(') && !text.contains(')') || text.is_empty() {
                failures.push(format!("{r}: text {text:?}"));
            }
            // Some hand-made items carry properties their base item does not
            // offer; the rest must be offered.
            if creature_item(base) || !available.iter().any(|a| a.row == p.property) {
                continue;
            }
            if t.subtypes.is_some()
                && !gd.property_subtypes(&t, base).iter().any(|c| c.row == p.subtype as usize)
            {
                failures.push(format!("{r}: {text}: subtype {} not offered", p.subtype));
            }
            if p.cost_table != 0
                && !gd
                    .property_costs(p.cost_table, base)
                    .iter()
                    .any(|c| c.row == p.cost_value as usize)
            {
                failures.push(format!("{r}: {text}: value {} not offered", p.cost_value));
            }
            if p.param1 != 255
                && !gd.property_params(p.param1).iter().any(|c| c.row == p.param1_value as usize)
            {
                failures.push(format!("{r}: {text}: parameter {} not offered", p.param1_value));
            }
        }
    }
    eprintln!("{props} properties on {items} items");
    let shown: Vec<_> = failures.iter().take(40).cloned().collect();
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), shown.join("\n"));
}
