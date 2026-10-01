//! Writes a module for Aurora to place objects in, to capture what it
//! writes into the area's GIT: one flat 4 by 4 rural area and a custom
//! blueprint of every type (`mgp_*`, copies of base-game ones), a creature
//! with equipment and an inventory item, a chest and a store holding items;
//! and minimal blueprints (`mgq_*`: a name, a tag, the palette category and
//! what a model needs), whose instances show Aurora's default for every
//! other field:
//! `cargo run -p mg-corpus-tests --example placement_probe_module OUT.mod [doors]`.
//! With `doors`, the area is a castle interior whose tile (1, 1) has three
//! door hooks (tic01 tile 7: at 20, 15; 10, 15; 15, 20), and the door
//! blueprints are a generic door (nw_door_normal) and a minimal one.
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// An inventory entry: the item and where it sits in the grid.
fn held(item: &str, x: u16, y: u16) -> Struct {
    let mut s = Struct::new(0);
    s.set("InventoryRes", Value::resref(r(item)));
    s.set("Repos_PosX", Value::Word(x));
    s.set("Repos_Posy", Value::Word(y));
    s
}

fn main() {
    let out = std::env::args().nth(1).expect("output module");
    let doors = std::env::args().nth(2).is_some_and(|a| a == "doors");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(&game, "Placement Probe", &mut rng).unwrap();
    let tileset = r(if doors { "tic01" } else { "ttr01" });
    let spec = AreaSpec { name: "Field".into(), tileset, width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    if doors {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        let tile = &mut are.root.list_mut("Tile_List").unwrap()[5];
        tile.set("Tile_ID", Value::Int(7));
        tile.set("Tile_Orientation", Value::Int(0));
        tile.set("Tile_Height", Value::Int(0));
        m.set_gff(key, &are).unwrap();
    }
    let copy = |from: &str, t: ResType, name: &str| -> Gff {
        let data = game.resman.get(&ResKey::new(r(from), t)).unwrap();
        let mut g = Gff::read(&data).unwrap();
        let field = if t == ResType::UTM { "ResRef" } else { "TemplateResRef" };
        g.root.set(field, Value::resref(r(name)));
        g.root.set("Tag", Value::String(name.to_uppercase().into_bytes()));
        g
    };
    let mut utc = copy("nw_bandit001", ResType::UTC, "mgp_utc");
    utc.root.set("ItemList", Value::List(vec![held("nw_it_mpotion001", 0, 0)]));
    let mut utp = copy("plc_chest1", ResType::UTP, "mgp_utp");
    utp.root.set("HasInventory", Value::Byte(1));
    utp.root.set(
        "ItemList",
        Value::List(vec![held("nw_it_mpotion001", 0, 0), held("nw_wswls001", 1, 0)]),
    );
    let mut utm = copy("nw_storebar01", ResType::UTM, "mgp_utm");
    let mut potions = Struct::new(2);
    potions.set("ItemList", Value::List(vec![held("nw_it_mpotion001", 0, 0)]));
    let mut weapons = Struct::new(4);
    weapons.set("ItemList", Value::List(vec![held("nw_wswls001", 0, 0)]));
    let pages = [Struct::new(0), Struct::new(4), potions, Struct::new(3), Struct::new(1)];
    let mut pages = pages.to_vec();
    pages[1] = weapons;
    utm.root.set("StoreList", Value::List(pages));
    for (name, t, g) in [
        ("mgp_utc", ResType::UTC, utc),
        ("mgp_utp", ResType::UTP, utp),
        ("mgp_utm", ResType::UTM, utm),
        ("mgp_uti", ResType::UTI, copy("nw_wswls001", ResType::UTI, "mgp_uti")),
        (
            "mgp_utd",
            ResType::UTD,
            copy(if doors { "nw_door_normal" } else { "nw_door_ttr_01" }, ResType::UTD, "mgp_utd"),
        ),
        ("mgp_utt", ResType::UTT, copy("newgeneric", ResType::UTT, "mgp_utt")),
        ("mgp_ute", ResType::UTE, copy("nw_verminbeet", ResType::UTE, "mgp_ute")),
        ("mgp_uts", ResType::UTS, copy("animalcriesday", ResType::UTS, "mgp_uts")),
        ("mgp_utw", ResType::UTW, copy("nw_waypoint001", ResType::UTW, "mgp_utw")),
    ] {
        m.set_gff(ResKey::new(r(name), t), &g).unwrap();
    }
    // Minimal blueprints, in the same palette categories.
    for (from, t, name_field, extra) in [
        ("nw_bandit001", ResType::UTC, "FirstName", Some(("Appearance_Type", Value::Word(6)))),
        ("nw_wswls001", ResType::UTI, "LocalizedName", Some(("BaseItem", Value::Int(1)))),
        ("plc_chest1", ResType::UTP, "LocName", Some(("Appearance", Value::Dword(8)))),
        ("nw_storebar01", ResType::UTM, "LocName", None),
        ("animalcriesday", ResType::UTS, "LocName", None),
        ("nw_waypoint001", ResType::UTW, "LocalizedName", None),
        ("newgeneric", ResType::UTT, "LocalizedName", None),
        ("nw_verminbeet", ResType::UTE, "LocalizedName", None),
        ("nw_door_normal", ResType::UTD, "LocName", Some(("GenericType_New", Value::Dword(0)))),
    ] {
        let data = game.resman.get(&ResKey::new(r(from), t)).unwrap();
        let base = Gff::read(&data).unwrap();
        let ext = t.extension().unwrap_or_default();
        let name = format!("mgq_{ext}");
        let mut g = Gff::new(base.file_type);
        let (resref_field, palette_field) =
            if t == ResType::UTM { ("ResRef", "ID") } else { ("TemplateResRef", "PaletteID") };
        g.root.set(resref_field, Value::resref(r(&name)));
        g.root.set("Tag", Value::String(name.to_uppercase().into_bytes()));
        let text = format!("Minimal {ext}");
        g.root.set(
            name_field,
            Value::LocString(mg_core::LocString::from_text(
                mg_core::Language::ENGLISH,
                mg_core::Gender::Male,
                &*text,
            )),
        );
        g.root.set(palette_field, base.root.get(palette_field).cloned().unwrap());
        if let Some((label, v)) = extra {
            g.root.set(label, v);
        }
        m.set_gff(ResKey::new(r(&name), t), &g).unwrap();
    }
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
