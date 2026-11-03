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

/// The palette groupings proposal's ten rough rules for what a placeable
/// is (`docs/palette-groupings-proposal.md`, "Measured: kinds from names").
const KINDS: &str = r#"
[palette.kind]
types = ["utp"]
# (After the theme, before the source and the pack a model was made after:
# "Autumn: Fern 01, after HAM (AUT)" is a fern, not food.)
words_of = '^.*?:\s*(.*?)\s*(?:,\s*after\b.*?)?\s*(?:\(|\[|$)'
[[palette.kind.group]]
name = "Plants"
names = ["plant", "plants", "bush", "fern", "flower", "flowers", "ivy", "tree", "grass", "shrub", "weed", "weeds", "vine", "vines", "moss", "mushroom", "mushrooms"]
[[palette.kind.group]]
name = "Walls and fences"
names = ["wall", "walls", "fence", "fences", "railing", "palisade", "hedge"]
[[palette.kind.group]]
name = "Rocks"
names = ["rock", "rocks", "boulder", "boulders", "stone", "stones", "pebbles"]
[[palette.kind.group]]
name = "Containers"
names = ["chest", "barrel", "barrels", "crate", "crates", "box", "sack", "sacks", "basket", "urn", "coffer"]
[[palette.kind.group]]
name = "Food"
names = ["food", "bread", "cheese", "meat", "fish", "fruit", "apple", "apples", "ham", "pie", "vegetables", "sausage"]
[[palette.kind.group]]
name = "Dishes"
names = ["plate", "plates", "bowl", "bowls", "cup", "cups", "mug", "mugs", "jug", "pot", "pots", "bottle", "bottles", "goblet", "dish"]
[[palette.kind.group]]
name = "Lights"
names = ["lamp", "lantern", "candle", "candles", "torch", "brazier", "candelabra", "chandelier", "sconce"]
[[palette.kind.group]]
name = "Seats"
names = ["chair", "chairs", "stool", "bench", "throne", "seat"]
[[palette.kind.group]]
name = "Tables"
names = ["table", "tables", "desk", "counter"]
[[palette.kind.group]]
name = "Beds"
names = ["bed", "beds", "cot", "bedroll", "hammock"]
"#;

/// Groupings are worked out when the palettes or the rules change, not
/// every frame: grouping the game's standard placeables, and TDoD's
/// 14,665 entries where `MOONGLOW_HAKS` has its haks, takes milliseconds
/// (the time is printed: `docs/palette-groupings-proposal.md`).
#[test]
fn a_large_palette_is_grouped_in_milliseconds() {
    use mg_module::palette::PaletteName;
    use mg_module::palette_groups::{OTHER, group_palette};
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rules = mg_module::project_file::ProjectFile::default();
    rules.apply("moonglow.toml", KINDS, false);
    assert_eq!(rules.warnings, Vec::<String>::new());
    let grouping = &rules.palette.list[0];
    let time = |what: &str, palette: &Palette, name: &dyn Fn(&PaletteName) -> String| {
        let n = palette.blueprints().len();
        let started = std::time::Instant::now();
        let nested = group_palette(palette, BlueprintKind::Placeable, grouping, name, true);
        let nested_took = started.elapsed();
        let started = std::time::Instant::now();
        let flat = group_palette(palette, BlueprintKind::Placeable, grouping, name, false);
        let flat_took = started.elapsed();
        let counts: Vec<String> = (flat.palette.nodes.iter().enumerate())
            .map(|(i, g)| format!("{} {}", flat.group_name(grouping, i), g.blueprints.len()))
            .collect();
        eprintln!(
            "{what}: {n} entries grouped in {:.1} ms (nested), {:.1} ms (flat): {}",
            nested_took.as_secs_f64() * 1000.0,
            flat_took.as_secs_f64() * 1000.0,
            counts.join(", ")
        );
        assert_eq!(flat.groups.last(), Some(&None), "{OTHER} last");
        assert_eq!(nested.groups, flat.groups);
        // (Milliseconds; a generous bound for a loaded machine.)
        assert!(nested_took.as_secs_f64() < 2.0, "{what}: {nested_took:?}");
        flat
    };
    let data = game.resman.get_named("placeablepalstd", ResType::ITP).unwrap();
    let standard = Palette::read(&Gff::read(&data).unwrap());
    let grouped = time("the game's placeables", &standard, &|n| n.text(&game));
    assert!(grouped.groups.len() > 3, "the game's chests, tables, chairs…");
    // TDoD's, where the builder's haks are given.
    let Some(hak) = mg_testkit::builder_hak_dirs()
        .into_iter()
        .map(|d| d.join("tdod_blueprints.hak"))
        .find(|h| h.is_file())
    else {
        eprintln!("TDoD's palette skipped: MOONGLOW_HAKS has no tdod_blueprints.hak");
        return;
    };
    let bytes = std::fs::read(&hak).unwrap();
    let erf = mg_erf::Erf::read(&bytes).unwrap();
    let itp = mg_core::ResRef::from_str("placeablepalstd").unwrap();
    let data = erf.get(&itp, ResType::ITP).expect("TDoD's palette").unwrap();
    let tdod = Palette::read(&Gff::read(&data).unwrap());
    let grouped = time("TDoD's placeables", &tdod, &|n| n.text(&game));
    // The words are the name's after its theme, before its source and
    // the pack it was made after: no "Fern 01, after HAM" is food.
    let food = (grouped.palette.nodes.iter().enumerate())
        .find(|(i, _)| grouped.group_name(grouping, *i) == "Food")
        .map(|(_, n)| n);
    let names: Vec<String> =
        food.map(|f| f.blueprints.iter().map(|b| b.name.text(&game)).collect()).unwrap_or_default();
    assert!(!names.iter().any(|n| n.to_lowercase().contains("fern")), "{names:?}");
}

/// TDoD's own lists, as its library build writes them (`palette_kinds.toml`:
/// 48 groups over placeables, creatures and items; `palette_roles.toml`:
/// the items'), beside its project (read, never changed), where
/// `MOONGLOW_HAKS` has its haks: read without a fault, and grouping its
/// palettes by them takes milliseconds. A group of several types shows in
/// each palette it has blueprints of alone.
#[test]
fn tdod_s_lists_read_and_group_its_palettes() {
    use mg_module::palette::PaletteName;
    use mg_module::palette_groups::group_palette;
    let root = corpus!();
    let Some(hak) = mg_testkit::builder_hak_dirs()
        .into_iter()
        .map(|d| d.join("tdod_blueprints.hak"))
        .find(|h| h.is_file())
    else {
        eprintln!("skipped: MOONGLOW_HAKS has no tdod_blueprints.hak");
        return;
    };
    // (The project's folder: the haks are in its build/hak.)
    let hak_dir = hak.parent().unwrap().to_path_buf();
    let project = std::fs::canonicalize(&hak_dir).ok().and_then(|_| {
        let p = hak_dir.parent()?.parent()?.to_path_buf();
        p.join("palette_kinds.toml").is_file().then_some(p)
    });
    let Some(project) = project else {
        eprintln!("skipped: no palette_kinds.toml beside TDoD's project");
        return;
    };
    let dir = mg_testkit::scratch_dir("tdod-palette-lists");
    let rules = format!(
        "[palette.kind]\ntitle = \"Kind\"\n\
         types = [\"utc\", \"ute\", \"uti\", \"utp\", \"uts\", \"utm\", \"utt\", \"utw\"]\n\
         from = {:?}\n[palette.role]\ntitle = \"Role\"\ntypes = [\"uti\"]\nfrom = {:?}\n",
        project.join("palette_kinds.toml").display().to_string(),
        project.join("palette_roles.toml").display().to_string(),
    );
    std::fs::write(dir.join("moonglow.toml"), rules).unwrap();
    let started = std::time::Instant::now();
    let p = mg_module::project_file::ProjectFile::read(&dir);
    let read = started.elapsed();
    assert_eq!(p.warnings, Vec::<String>::new());
    let (kind, role) = (&p.palette.list[0], &p.palette.list[1]);
    let listed: usize = kind.groups.iter().map(|g| g.listed.len()).sum();
    eprintln!(
        "TDoD's lists: {} groups of Kind ({listed} entries), {} of Role, read in {:.1} ms",
        kind.groups.len(),
        role.groups.len(),
        read.as_secs_f64() * 1000.0
    );
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let bytes = std::fs::read(&hak).unwrap();
    let erf = mg_erf::Erf::read(&bytes).unwrap();
    for kind_of in [BlueprintKind::Placeable, BlueprintKind::Creature, BlueprintKind::Item] {
        let name = mg_core::ResRef::from_str(&format!("{}palstd", kind_of.name())).unwrap();
        // (TDoD's palette where its hak has one, else the game's.)
        let data = match erf.get(&name, ResType::ITP) {
            Some(d) => d.unwrap().into_owned(),
            None => game.resman.get(&mg_resman::ResKey::new(name, ResType::ITP)).unwrap().into(),
        };
        let palette = Palette::read(&Gff::read(&data).unwrap());
        for g in [kind, role].into_iter().filter(|g| g.offered(kind_of)) {
            let text = |n: &PaletteName| n.text(&game);
            let started = std::time::Instant::now();
            let grouped = group_palette(&palette, kind_of, g, &text, true);
            let took = started.elapsed();
            eprintln!(
                "  {} by {}: {} entries in {} groups (Other {}) in {:.1} ms",
                kind_of.name(),
                g.title,
                palette.blueprints().len(),
                grouped.groups.iter().filter(|x| x.is_some()).count(),
                if grouped.groups.last() == Some(&None) { "shown" } else { "empty" },
                took.as_secs_f64() * 1000.0
            );
            assert!(took.as_secs_f64() < 2.0);
            // Every group shown has a blueprint of this palette.
            assert!(grouped.palette.nodes.iter().all(|n| !n.children.is_empty()));
        }
    }
}
