//! The Store Setup Wizard against Aurora's: two runs on a bandit standing
//! at the centre of an area (`examples/popup_probe_module.rs`), captured in
//! `store-setup/defaults.mod` (the defaults, the custom store `mgp_store`,
//! the hostile bandit given the Merchant faction) and `second.mod` (no
//! appraise checks, the standard store `nw_storethief001`). Moonglow's
//! conversation, script, store instance and names must be Aurora's.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::factions::Factions;
use mg_module::instances::{Placing, instance};
use mg_module::store_setup::{
    GREETING, NO, YES, conversation, hostile, merchant, next_name, script, store_placement,
};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

/// Field labels, types and values, one line each, fields in label order
/// (so the order fields are written in does not count).
fn lines(s: &Struct, indent: &str, out: &mut Vec<String>) {
    let mut fields: Vec<_> = s.fields.iter().collect();
    fields.sort_by_key(|f| f.label.to_string_lossy());
    for f in fields {
        match &f.value {
            Value::List(items) => {
                out.push(format!("{indent}{} list[{}]", f.label.to_string_lossy(), items.len()));
                for (i, item) in items.iter().enumerate() {
                    out.push(format!("{indent}  [{i}] struct {}", item.id));
                    lines(item, &format!("{indent}    "), out);
                }
            }
            Value::Float(v) => {
                let v = if v.abs() < 1e-6 { 0.0 } else { *v };
                out.push(format!("{indent}{} Float({v:.5})", f.label.to_string_lossy()))
            }
            v => out.push(format!("{indent}{} {:?}", f.label.to_string_lossy(), v)),
        }
    }
}

fn text(s: &Struct) -> Vec<String> {
    let mut out = Vec::new();
    lines(s, "", &mut out);
    out
}

fn gff(m: &Module, name: &str, t: ResType) -> Gff {
    m.gff(&ResKey::parse(name, t).unwrap()).unwrap().unwrap()
}

#[test]
fn store_setup_matches_aurora() {
    let root = corpus!();
    let first = Module::open(&aurora_capture!("store-setup/defaults.mod")).unwrap();
    let second = Module::open(&aurora_capture!("store-setup/second.mod")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();

    // The conversation and script, byte for byte where Aurora wrote text.
    let open = ResRef::from_str("openstore001").unwrap();
    let ours = conversation(GREETING, YES, NO, open);
    assert_eq!(text(&ours.root), text(&gff(&first, "store001", ResType::DLG).root));
    let nss =
        |m: &Module, n: &str| m.get(&ResKey::parse(n, ResType::NSS).unwrap()).unwrap().to_vec();
    assert_eq!(script("MGP_STORE", true).into_bytes(), nss(&first, "openstore001"));
    assert_eq!(script("NW_STORETHIEF001", false).into_bytes(), nss(&second, "openstore002"));
    // The next run's names.
    assert_eq!(next_name(&first, "store", ResType::DLG).to_string(), "store002");
    assert_eq!(next_name(&first, "openstore", ResType::NSS).to_string(), "openstore002");

    // The shopkeeper: the conversation, and the Merchant faction for the
    // hostile bandit.
    let git = gff(&first, "field", ResType::GIT);
    let creature = &git.root.list("Creature List").unwrap()[0];
    assert_eq!(creature.resref("Conversation"), Some(ResRef::from_str("store001").unwrap()));
    let factions = Factions::read(&gff(&first, "repute", ResType::FAC));
    assert!(hostile(Some(&factions), 1), "Hostile");
    assert!(!hostile(Some(&factions), merchant(Some(&factions))));
    assert_eq!(creature.integer("FactionID"), Some(i64::from(merchant(Some(&factions)))));

    // The stores, placed where the shopkeeper stands.
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let placing = Placing { game: &game, item: &item };
    let git2 = gff(&second, "field", ResType::GIT);
    let stores = git2.root.list("StoreList").unwrap();
    for (placed, blueprint) in stores.iter().zip(["mgp_store", "nw_storethief001"]) {
        let bp = match second.gff(&ResKey::parse(blueprint, ResType::UTM).unwrap()) {
            Some(Ok(g)) => g,
            _ => Gff::read(
                &game.resman.get(&ResKey::parse(blueprint, ResType::UTM).unwrap()).unwrap(),
            )
            .unwrap(),
        };
        let at = store_placement(&git2.root.list("Creature List").unwrap()[0]);
        let ours = instance(&placing, ResType::UTM, &bp.root, at, &[]).unwrap();
        let (a, b) = (text(&ours), text(placed));
        let first = a.iter().zip(&b).position(|(x, y)| x != y);
        if let Some(i) = first.or((a.len() != b.len()).then(|| a.len().min(b.len()))) {
            let from = i.saturating_sub(8);
            panic!(
                "{blueprint}: differs at line {i}:\nMoonglow {:#?}\nAurora {:#?}",
                &a[from..(i + 3).min(a.len())],
                &b[from..(i + 3).min(b.len())]
            );
        }
    }
}
