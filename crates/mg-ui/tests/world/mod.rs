//! A synthetic persistent world, at the sizes builders report Aurora
//! failing at (`docs/research/community_pain_points.md`, item 8): Tyrants of
//! the Moonsea grown to 300 areas, 8,000 item, 2,000 creature and 3,000
//! placeable blueprints, 4,000 scripts and 1,500 conversations, a store
//! holding 1,000 items, and 50 haks of CEP's scale (150,000 resources,
//! 4,000 more `placeables.2da` rows and 2,000 more `appearance.2da` rows, a
//! standard placeable palette with 10,000 more blueprints), the last hak
//! over 2 GiB (a sparse file).
//!
//! Built from the user's install into `target/test-output/pw-world` (game
//! data, never committed) and kept there: building takes a while, so it's
//! built again only when [`VERSION`] changes.

use std::path::{Path, PathBuf};

use mg_2da::TwoDa;
use mg_core::{Codepage, ResRef, ResType};
use mg_erf::ErfWriter;
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_resman::{GameInstall, ResKey, ResMan};

/// Bumped when what is built changes.
const VERSION: &str = "3";

pub(crate) const AREAS: usize = 300;
pub(crate) const ITEMS: usize = 8_000;
pub(crate) const CREATURES: usize = 2_000;
pub(crate) const PLACEABLES: usize = 3_000;
pub(crate) const SCRIPTS: usize = 4_000;
pub(crate) const DIALOGS: usize = 1_500;
pub(crate) const STORE_ITEMS: usize = 1_000;
pub(crate) const HAKS: usize = 50;
/// Models and textures in each hak.
pub(crate) const HAK_MODELS: usize = 1_000;
pub(crate) const HAK_TEXTURES: usize = 2_000;
/// Placeable blueprints in the haks' standard palette (in haks 1 to 10).
pub(crate) const HAK_PLACEABLES: usize = 10_000;
/// Rows the first hak adds to the game's `placeables.2da` (16,500 rows in
/// EE 37) and `appearance.2da` (15,100).
pub(crate) const NEW_PLACEABLE_ROWS: usize = 4_000;
pub(crate) const NEW_APPEARANCE_ROWS: usize = 2_000;

const CAMPAIGN: &str = "Neverwinter Nights - Tyrants of the Moonsea.nwm";

/// The world: a user directory (its `hak/`) and the module.
pub(crate) struct World {
    pub(crate) user: PathBuf,
    pub(crate) module: PathBuf,
}

/// The world, built if it isn't yet; `None` without Tyrants of the Moonsea.
pub(crate) fn world(root: &Path) -> Option<World> {
    let campaign = root.join("data/nwm").join(CAMPAIGN);
    if !campaign.is_file() {
        eprintln!("skipped: {CAMPAIGN} isn't installed");
        return None;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-output/pw-world");
    let user = base.join("user");
    let world = World { module: user.join("modules/pw_world.mod"), user };
    let stamp = base.join("built");
    if std::fs::read_to_string(&stamp).ok().as_deref() == Some(VERSION) {
        return Some(world);
    }
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(world.user.join("hak")).unwrap();
    std::fs::create_dir_all(world.user.join("modules")).unwrap();
    let t = std::time::Instant::now();
    build(root, &campaign, &world);
    std::fs::write(&stamp, VERSION).unwrap();
    println!("built the world in {:.1?}", t.elapsed());
    Some(world)
}

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::parse(name, t).unwrap()
}

/// Copies of `sources` (cycled) under new names `prefix{n}`, their
/// `TemplateResRef` set to the new name.
fn blueprints(
    rm: &ResMan,
    t: ResType,
    prefix: &str,
    count: usize,
    mut edit: impl FnMut(usize, &mut Struct),
) -> Vec<(ResKey, Vec<u8>)> {
    let sources: Vec<Gff> = rm
        .list(t)
        .into_iter()
        .filter_map(|r| Gff::read(&rm.get(&ResKey::new(r, t)).ok()?).ok())
        .collect();
    assert!(!sources.is_empty(), "the game has {t} blueprints");
    (0..count)
        .map(|n| {
            let mut g = sources[n % sources.len()].clone();
            let name = format!("{prefix}{n:05}");
            g.root.set("TemplateResRef", Value::resref(ResRef::from_str(&name).unwrap()));
            edit(n, &mut g.root);
            (key(&name, t), g.to_bytes().unwrap())
        })
        .collect()
}

/// An 8×8 uncompressed TGA, its color from `n`.
fn texture(n: usize) -> Vec<u8> {
    let mut v = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 32, 8];
    for _ in 0..64 {
        v.extend_from_slice(&[(n % 251) as u8, (n / 251 % 251) as u8, 128, 255]);
    }
    v
}

/// A small ASCII placeable model: a quad with texture `tex`.
fn model(name: &str, tex: &str) -> Vec<u8> {
    format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
         node trimesh body\n  parent {name}\n  bitmap {tex}\n  diffuse 1 1 1\n  verts 4\n\
         0 0 0\n1 0 0\n1 1 0\n0 1 0\n  tverts 4\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n  faces 2\n\
         0 1 2 1 0 1 2 1\n0 2 3 1 0 2 3 1\nendnode\nendmodelgeom {name}\ndonemodel {name}\n"
    )
    .into_bytes()
}

fn table(rm: &ResMan, name: &str) -> TwoDa {
    TwoDa::parse(&rm.get(&key(name, ResType::TWODA)).unwrap(), Codepage::default()).unwrap()
}

/// `table` grown to `rows` rows by copies of its rows that have `column`,
/// with `edit` on each new row (its number).
fn grow(
    t: &mut TwoDa,
    rows: usize,
    column: &str,
    mut edit: impl FnMut(usize, &mut [Option<String>]),
) {
    let c = t.column(column).unwrap();
    let models: Vec<Vec<Option<String>>> =
        t.rows.iter().filter(|r| r.get(c).is_some_and(Option::is_some)).cloned().collect();
    let mut n = 0;
    while t.rows.len() < rows {
        let mut row = models[n % models.len()].clone();
        edit(t.rows.len(), &mut row);
        t.rows.push(row);
        n += 1;
    }
}

/// Adds `leaves` (resref, name) to a standard palette's categories, a few
/// to each in turn.
fn add_to_palette(palette: &mut Gff, leaves: &[(ResRef, String)]) {
    fn count(list: &[Struct]) -> usize {
        list.iter()
            .map(|n| usize::from(n.integer("ID").is_some()) + count(n.list("LIST").unwrap_or(&[])))
            .sum()
    }
    fn fill(list: &mut [Struct], at: &mut usize, of: usize, leaves: &[(ResRef, String)]) {
        for node in list {
            if node.integer("ID").is_some() {
                let mine = leaves.iter().skip(*at).step_by(of).map(|(resref, name)| {
                    let mut leaf = Struct::new(0);
                    leaf.set("NAME", Value::String(name.clone().into_bytes()));
                    leaf.set("RESREF", Value::resref(*resref));
                    leaf
                });
                let mut children = node.list("LIST").unwrap_or(&[]).to_vec();
                children.extend(mine);
                node.set("LIST", Value::List(children));
                *at += 1;
            }
            if let Some(children) = node.list_mut("LIST") {
                fill(children, at, of, leaves);
            }
        }
    }
    let main = palette.root.list_mut("MAIN").unwrap();
    let of = count(main);
    assert!(of > 0);
    fill(main, &mut 0, of, leaves);
}

/// The first rows the haks add to `placeables.2da` and `appearance.2da`
/// (labeled `PW_Placeable_{row}`, `PW_Creature_{row}`): the lengths of the
/// game's own (`rm` without the haks).
pub(crate) fn first_new_rows(rm: &ResMan) -> (usize, usize) {
    (table(rm, "placeables").rows.len(), table(rm, "appearance").rows.len())
}

fn hak_name(i: usize) -> String {
    format!("pw_hak{i:02}")
}

fn build(root: &Path, campaign: &Path, world: &World) {
    let gi = GameInstall::new(root, None, "en");
    let rm = ResMan::for_game(&gi).unwrap();
    let mut m = Module::open(campaign).unwrap();

    // Areas: the campaign's, copied under new names up to AREAS.
    let mut info = m.info().unwrap();
    let areas = m.areas().unwrap();
    let mut list = info.root.list("Mod_Area_list").unwrap().to_vec();
    for n in areas.len()..AREAS {
        let src = areas[n % areas.len()];
        let name = format!("pw_area{n:03}");
        let r = ResRef::from_str(&name).unwrap();
        for t in [ResType::ARE, ResType::GIT, ResType::GIC] {
            let Some(data) = m.get(&ResKey::new(src, t)).map(<[u8]>::to_vec) else { continue };
            let data = if t == ResType::ARE {
                let mut g = Gff::read(&data).unwrap();
                g.root.set("ResRef", Value::resref(r));
                g.to_bytes().unwrap()
            } else {
                data
            };
            m.set(ResKey::new(r, t), data);
        }
        let mut item = list[0].clone();
        item.set("Area_Name", Value::resref(r));
        list.push(item);
    }
    info.root.set("Mod_Area_list", Value::List(list));

    // Blueprints; the placeables and creatures on the haks' new 2DA rows.
    let (placeable_row, appearance_row) = first_new_rows(&rm);
    let creatures = blueprints(&rm, ResType::UTC, "pw_c", CREATURES, |n, s| {
        let row = appearance_row + n % NEW_APPEARANCE_ROWS;
        s.set("Appearance_Type", Value::Word(row as u16));
    });
    let placeables = blueprints(&rm, ResType::UTP, "pw_p", PLACEABLES, |n, s| {
        let row = placeable_row + n % NEW_PLACEABLE_ROWS;
        s.set("Appearance", Value::Dword(row as u32));
    });
    for (k, data) in blueprints(&rm, ResType::UTI, "pw_i", ITEMS, |_, _| {})
        .into_iter()
        .chain(creatures)
        .chain(placeables)
    {
        m.set(k, data);
    }

    // A store holding STORE_ITEMS of them, on the pages their base items
    // name.
    let baseitems = table(&rm, "baseitems");
    let store_src = rm.list(ResType::UTM)[0];
    let mut store = Gff::read(&rm.get(&ResKey::new(store_src, ResType::UTM)).unwrap()).unwrap();
    let mut pages: Vec<Vec<Struct>> = vec![Vec::new(); 5];
    for n in 0..STORE_ITEMS {
        let k = key(&format!("pw_i{n:05}"), ResType::UTI);
        let uti = Gff::read(m.get(&k).unwrap()).unwrap();
        let base = uti.root.integer("BaseItem").unwrap_or(0) as usize;
        let page = baseitems.get_int(base, "StorePanel").unwrap_or(4).clamp(0, 4) as usize;
        let mut e = Struct::new(pages[page].len() as u32);
        e.set("InventoryRes", Value::resref(k.resref));
        e.set("Repos_PosX", Value::Word((pages[page].len() % 10) as u16));
        e.set("Repos_Posy", Value::Word((pages[page].len() / 10) as u16));
        pages[page].push(e);
    }
    let store_list = pages
        .into_iter()
        .enumerate()
        .map(|(i, items)| {
            let mut p = Struct::new(i as u32);
            p.set("ItemList", Value::List(items));
            p
        })
        .collect();
    store.root.set("StoreList", Value::List(store_list));
    store.root.set("ResRef", Value::resref(ResRef::from_str("pw_store").unwrap()));
    m.set_gff(key("pw_store", ResType::UTM), &store).unwrap();

    // Scripts and conversations: copies of the campaign's.
    let mains: Vec<ResKey> = m
        .keys_of(ResType::NSS)
        .copied()
        .filter(|k| {
            let text = String::from_utf8_lossy(m.get(k).unwrap()).to_lowercase();
            (text.contains("void main") || text.contains("int startingconditional"))
                && m.contains(&ResKey::new(k.resref, ResType::NCS))
        })
        .collect();
    let have = m.keys_of(ResType::NSS).count();
    for n in 0..SCRIPTS.saturating_sub(have) {
        let src = mains[n % mains.len()];
        let r = ResRef::from_str(&format!("pw_s{n:05}")).unwrap();
        for t in [ResType::NSS, ResType::NCS] {
            let data = m.get(&ResKey::new(src.resref, t)).unwrap().to_vec();
            m.set(ResKey::new(r, t), data);
        }
    }
    let dialogs: Vec<ResKey> = m.keys_of(ResType::DLG).copied().collect();
    for n in 0..DIALOGS.saturating_sub(dialogs.len()) {
        let data = m.get(&dialogs[n % dialogs.len()]).unwrap().to_vec();
        m.set(key(&format!("pw_d{n:05}"), ResType::DLG), data);
    }

    // The haks, highest priority first, over the campaign's own.
    let mut haks: Vec<Struct> = (0..HAKS)
        .map(|i| {
            let mut s = Struct::new(8);
            s.set("Mod_Hak", Value::String(hak_name(i).into_bytes()));
            s
        })
        .collect();
    haks.extend(info.root.list("Mod_HakList").unwrap_or(&[]).iter().cloned());
    info.root.set("Mod_HakList", Value::List(haks));
    m.set_info(&info).unwrap();
    write_haks(&rm, &world.user.join("hak"));

    let game =
        mg_rules::GameData::open(&GameInstall::new(root, Some(world.user.clone()), "en")).unwrap();
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    std::fs::write(&world.module, m.to_archive_bytes().unwrap()).unwrap();
}

/// The haks: the first holds the grown 2DAs and the standard placeable
/// palette; each has HAK_MODELS models and HAK_TEXTURES textures; haks 1 to
/// 10 hold the palette's placeables; the last is over 2 GiB.
fn write_haks(rm: &ResMan, dir: &Path) {
    let placeables_2da = table(rm, "placeables");
    let first_new_row = placeables_2da.rows.len();
    let models = HAKS * HAK_MODELS;
    let mut placeables_2da = placeables_2da;
    let col = |t: &TwoDa, c: &str| t.column(c).unwrap();
    let (label, strref, model_name) = (
        col(&placeables_2da, "Label"),
        col(&placeables_2da, "StrRef"),
        col(&placeables_2da, "ModelName"),
    );
    // Named by their labels.
    grow(&mut placeables_2da, first_new_row + NEW_PLACEABLE_ROWS, "ModelName", |row, cells| {
        cells[label] = Some(format!("PW_Placeable_{row}"));
        cells[strref] = None;
        cells[model_name] = Some(format!("pw_m{:05}", row % models));
    });
    let mut appearance = table(rm, "appearance");
    let (label, strref) = (col(&appearance, "LABEL"), col(&appearance, "STRING_REF"));
    let rows = appearance.rows.len() + NEW_APPEARANCE_ROWS;
    grow(&mut appearance, rows, "RACE", |row, cells| {
        cells[label] = Some(format!("PW_Creature_{row}"));
        cells[strref] = None;
    });
    // The palette's placeables, on the new rows.
    let hak_placeables = blueprints(rm, ResType::UTP, "pw_hp", HAK_PLACEABLES, |n, s| {
        let row = first_new_row + n % NEW_PLACEABLE_ROWS;
        s.set("Appearance", Value::Dword(row as u32));
    });
    let mut palette = Gff::read(&rm.get(&key("placeablepalstd", ResType::ITP)).unwrap()).unwrap();
    let leaves: Vec<(ResRef, String)> = (0..HAK_PLACEABLES)
        .map(|n| (ResRef::from_str(&format!("pw_hp{n:05}")).unwrap(), format!("PW Placeable {n}")))
        .collect();
    add_to_palette(&mut palette, &leaves);

    for i in 0..HAKS {
        let mut entries: Vec<(ResKey, Vec<u8>)> = Vec::new();
        if i == 0 {
            entries.push((
                key("placeables", ResType::TWODA),
                placeables_2da.to_bytes(Codepage::default()).unwrap(),
            ));
            entries.push((
                key("appearance", ResType::TWODA),
                appearance.to_bytes(Codepage::default()).unwrap(),
            ));
            entries.push((key("placeablepalstd", ResType::ITP), palette.to_bytes().unwrap()));
        }
        if (1..=10).contains(&i) {
            let per = HAK_PLACEABLES / 10;
            entries.extend(hak_placeables[(i - 1) * per..i * per].iter().cloned());
        }
        for n in 0..HAK_MODELS {
            let m = i * HAK_MODELS + n;
            let tex = format!("pw_t{:06}", i * HAK_TEXTURES + n);
            entries.push((
                key(&format!("pw_m{m:05}"), ResType::MDL),
                model(&format!("pw_m{m:05}"), &tex),
            ));
        }
        for n in 0..HAK_TEXTURES {
            let t = i * HAK_TEXTURES + n;
            entries.push((key(&format!("pw_t{t:06}"), ResType::TGA), texture(t)));
        }
        let path = dir.join(format!("{}.hak", hak_name(i)));
        if i == HAKS - 1 {
            // Over 2 GiB: a hole, then ten textures the game can't read.
            let names: Vec<String> = entries.iter().map(|(k, _)| k.resref.to_string()).collect();
            let before: Vec<mg_testkit::erf::Entry<'_>> = entries
                .iter()
                .zip(&names)
                .map(|((k, d), n)| (n.as_str(), k.restype.0, &d[..]))
                .collect();
            let far: Vec<(String, Vec<u8>)> =
                (0..10).map(|n| (format!("pw_far{n}"), texture(n))).collect();
            let after: Vec<mg_testkit::erf::Entry<'_>> =
                far.iter().map(|(n, d)| (n.as_str(), ResType::TGA.0, &d[..])).collect();
            mg_testkit::erf::write_padded(
                &path,
                b"HAK ",
                &before,
                ("pw_padding", 10, 1 << 31),
                &after,
            )
            .unwrap();
        } else {
            let mut w = ErfWriter::new(*b"HAK ");
            for (k, d) in &entries {
                w.add(k.resref, k.restype, &d[..]).unwrap();
            }
            let mut f = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
            w.write_to(&mut f).unwrap();
        }
    }
}
