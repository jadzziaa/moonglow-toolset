//! Palettes: every standard palette reads; a shipped module's custom
//! palettes are what rebuilding them from its blueprints gives.

use mg_core::ResType;
use mg_gff::Gff;
use mg_module::Module;
use mg_module::palette::{BlueprintKind, Palette, rebuild_custom_palette};
use mg_resman::GameInstall;
use mg_rules::GameData;
use mg_testkit::corpus;

#[test]
fn standard_palettes_read() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    for kind in BlueprintKind::ALL {
        let data = game.resman.get_named(&format!("{}palstd", kind.name()), ResType::ITP).unwrap();
        let p = Palette::read(&Gff::read(&data).unwrap());
        let n = p.blueprints().len();
        // Every listed blueprint exists.
        let missing: Vec<String> = p
            .blueprints()
            .iter()
            .filter(|(_, b)| {
                game.resman.get(&mg_resman::ResKey::new(b.resref, kind.restype())).is_err()
            })
            .map(|(_, b)| b.resref.to_string())
            .collect();
        eprintln!(
            "{}: {n} blueprints, {} categories, missing {missing:?}",
            kind.name(),
            p.categories(&game).len()
        );
        assert!(n > 0);
        assert!(missing.len() * 100 <= n, "{}: {} missing", kind.name(), missing.len());
    }
}

#[test]
fn custom_palettes_rebuild_like_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let path = root.join("data/nwm/Chapter1.nwm");
    let Ok(module) = Module::open(&path) else {
        eprintln!("skipped: no {}", path.display());
        return;
    };
    let mut report = Vec::new();
    let mut order_differs = 0;
    for kind in BlueprintKind::ALL {
        let shipped = Palette::read(&module.gff(&kind.custom_key()).unwrap().unwrap());
        let ours = Palette::read(&rebuild_custom_palette(&module, &game, kind).unwrap());
        let entries = |p: &Palette| -> Vec<(Option<u8>, String, String)> {
            let mut v: Vec<_> = p
                .blueprints()
                .iter()
                .map(|(c, b)| (*c, b.resref.to_string(), b.name.text(&game)))
                .collect();
            v.sort();
            v
        };
        let (a, b) = (entries(&shipped), entries(&ours));
        let only_shipped: Vec<_> = a.iter().filter(|x| !b.contains(x)).collect();
        let only_ours: Vec<_> = b.iter().filter(|x| !a.contains(x)).collect();
        eprintln!(
            "{}: shipped {}, ours {}, only shipped {:?}, only ours {:?}",
            kind.name(),
            a.len(),
            b.len(),
            only_shipped.iter().take(5).collect::<Vec<_>>(),
            only_ours.iter().take(5).collect::<Vec<_>>()
        );
        report.push((kind, only_shipped.len(), only_ours.len(), a.len()));
        // The same order, category by category.
        let order = |p: &Palette| -> Vec<String> {
            p.blueprints().iter().map(|(_, b)| b.resref.to_string()).collect()
        };
        let (so, oo) = (order(&shipped), order(&ours));
        if so != oo {
            let first = so.iter().zip(&oo).position(|(x, y)| x != y);
            eprintln!(
                "{}: order differs at {first:?}: shipped {:?}, ours {:?}",
                kind.name(),
                first.map(|i| &so[i.saturating_sub(1)..(i + 2).min(so.len())]),
                first.map(|i| &oo[i.saturating_sub(1)..(i + 2).min(oo.len())])
            );
        }
        order_differs += usize::from(so != oo);
    }
    assert_eq!(order_differs, 0, "blueprints in another order");
    for (kind, s, o, n) in report {
        assert!(s + o == 0, "{}: {s} only shipped, {o} only ours of {n}", kind.name());
    }
}
