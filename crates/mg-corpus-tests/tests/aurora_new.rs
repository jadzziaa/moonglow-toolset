//! New modules and areas compared with Aurora's: a module made by Aurora's
//! Module Wizard with areas from its Area Wizard for every tileset at several
//! sizes (`tools/aurora/capture_new_areas.py`, kept as the oracle capture
//! `new-module-and-areas.mod`). Moonglow's must have the same fields, types,
//! order and values, except where Aurora picks at random: the module ID, and
//! for each tile the variant, orientation and lights among those allowed.

use mg_core::{Gender, Language, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::new::{AreaSpec, add_area, new_module, tilesets};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, are};
use mg_set::Tileset;
use mg_testkit::{aurora_capture, corpus};
use mg_tiles::{Lattice, Placement, TileIndex, TilesError};

/// Field labels and types, in order.
fn shape(s: &Struct) -> Vec<(String, String)> {
    s.fields
        .iter()
        .map(|f| (f.label.to_string_lossy(), f.value.field_type().json_name().to_string()))
        .collect()
}

fn gff(m: &Module, name: &str, t: ResType) -> Gff {
    m.gff(&ResKey::parse(name, t).unwrap()).unwrap_or_else(|| panic!("{name} missing")).unwrap()
}

fn without(s: &Struct, labels: &[&str]) -> Struct {
    let mut s = s.clone();
    for l in labels {
        s.remove(l);
    }
    s
}

#[test]
fn new_modules_and_areas_match_aurora() {
    let root = corpus!();
    let capture = Module::open(&aurora_capture!("new-module-and-areas.mod")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(1);
    let aurora_info = capture.info().unwrap();
    let name = aurora_info
        .root
        .locstring("Mod_Name")
        .unwrap()
        .text(Language::ENGLISH, Gender::Male)
        .unwrap();
    let mut ours = new_module(&game, &name, &mut rng).unwrap();

    // The factions and palettes are Aurora's exactly.
    assert_eq!(gff(&ours, "repute", ResType::FAC), gff(&capture, "repute", ResType::FAC));
    for kind in mg_module::new::PALETTE_TYPES {
        let pal = format!("{kind}palcus");
        assert_eq!(gff(&ours, &pal, ResType::ITP), gff(&capture, &pal, ResType::ITP), "{pal}");
    }

    // The areas, in Aurora's order (the first becomes the starting area).
    let names: Vec<ResRef> = capture.areas().unwrap();
    let mut checked = 0;
    for area in &names {
        let theirs = gff(&capture, &area.to_string(), ResType::ARE);
        let r = &theirs.root;
        let spec = AreaSpec {
            name: r
                .locstring("Name")
                .unwrap()
                .text(Language::ENGLISH, Gender::Male)
                .unwrap()
                .into_owned(),
            tileset: r.read(&are::TILESET),
            width: r.read(&are::WIDTH) as u32,
            height: r.read(&are::HEIGHT) as u32,
        };
        let made = add_area(&mut ours, &game, &spec, &mut rng).unwrap();
        assert_eq!(made, *area, "resref for {:?}", spec.name);
        let mine = gff(&ours, &area.to_string(), ResType::ARE);
        assert_eq!(shape(&mine.root), shape(r), "{area}: fields");
        assert_eq!(without(&mine.root, &["Tile_List"]), without(r, &["Tile_List"]), "{area}");
        assert_eq!(
            gff(&ours, &area.to_string(), ResType::GIC),
            gff(&capture, &area.to_string(), ResType::GIC)
        );
        let (mut mine_git, mut their_git) = (
            gff(&ours, &area.to_string(), ResType::GIT),
            gff(&capture, &area.to_string(), ResType::GIT),
        );
        if area.to_string() == "area027" {
            // Once, Aurora gave a Lizardfolk Interior (no areag.ini entry)
            // ambient volumes 78 and 80; made again, it has 0 and 0 like the
            // other tilesets without an entry.
            for g in [&mut mine_git, &mut their_git] {
                let props = g.root.child_mut("AreaProperties").unwrap();
                props.remove("AmbientSndDayVol");
                props.remove("AmbientSndNitVol");
            }
        }
        assert_eq!(mine_git, their_git, "{area}.git");

        // Tiles: the same terrain; each of Aurora's tiles is one Moonglow
        // could have picked, with lights from the lighting scheme's options.
        let data = game.resman.get(&ResKey::new(spec.tileset, ResType::SET)).unwrap();
        let set = Tileset::parse(&data, game.language.codepage()).unwrap();
        let index = TileIndex::new(&set);
        let placements = |g: &Gff| -> Vec<Placement> {
            g.root
                .items(&are::TILE_LIST)
                .iter()
                .map(|t| Placement {
                    tile: t.read(&are::tile_list::TILE_ID) as u32,
                    orientation: t.read(&are::tile_list::TILE_ORIENTATION) as u8,
                    height: t.read(&are::tile_list::TILE_HEIGHT),
                })
                .collect()
        };
        let (w, h) = (spec.width, spec.height);
        let (aurora_lattice, seams) =
            Lattice::from_tiles(&index, w, h, &placements(&theirs)).unwrap();
        assert!(seams.is_empty(), "{area}: Aurora's tiles disagree at {seams:?}");
        let (our_lattice, _) = Lattice::from_tiles(&index, w, h, &placements(&mine)).unwrap();
        assert_eq!(our_lattice, aurora_lattice, "{area}: terrain");
        for (i, p) in placements(&theirs).iter().enumerate() {
            let cell = aurora_lattice.cell(i as u32 % w, i as u32 / w);
            assert!(
                index.fits(&cell).contains(p),
                "{area}: Aurora's tile {i} {p:?} is not a candidate"
            );
        }
        let lights = |g: &Gff| -> Vec<Struct> {
            g.root
                .items(&are::TILE_LIST)
                .iter()
                .map(|t| {
                    without(
                        t,
                        &[
                            "Tile_ID",
                            "Tile_Orientation",
                            "Tile_MainLight1",
                            "Tile_MainLight2",
                            "Tile_SrcLight1",
                            "Tile_SrcLight2",
                        ],
                    )
                })
                .collect()
        };
        let env = game.table("environment").unwrap();
        let scheme = r.read(&are::LIGHTING_SCHEME) as usize;
        let options = |prefix: &str| -> Vec<u8> {
            (1..=4).map(|i| env.get_int(scheme, &format!("{prefix}{i}")).unwrap() as u8).collect()
        };
        let (main1, main2, source) =
            (options("MAIN1_COLOR"), options("MAIN2_COLOR"), options("SECONDARY_COLOR"));
        for g in [&mine, &theirs] {
            for t in g.root.items(&are::TILE_LIST) {
                use are::tile_list as tl;
                assert!(main1.contains(&t.read(&tl::TILE_MAIN_LIGHT1)), "{area}");
                assert!(main2.contains(&t.read(&tl::TILE_MAIN_LIGHT2)), "{area}");
                assert!(source.contains(&t.read(&tl::TILE_SRC_LIGHT1)), "{area}");
                assert_eq!(t.read(&tl::TILE_SRC_LIGHT1), t.read(&tl::TILE_SRC_LIGHT2), "{area}");
            }
        }
        // Animation loops come from the tile, so compare only where the
        // same tile was picked.
        for ((a, b), (pa, pb)) in lights(&mine)
            .iter()
            .zip(lights(&theirs))
            .zip(placements(&mine).iter().zip(placements(&theirs)))
        {
            if pa.tile == pb.tile {
                assert_eq!(a, &b, "{area}: tile fields");
            }
        }
        checked += 1;
    }

    // The module info: Aurora's apart from the random ID.
    let mine = ours.info().unwrap();
    assert_eq!(shape(&mine.root), shape(&aurora_info.root));
    assert_eq!(without(&mine.root, &["Mod_ID"]), without(&aurora_info.root, &["Mod_ID"]));
    assert!(matches!(mine.root.get("Mod_ID"), Some(Value::Void(v)) if v.len() == 16));
    eprintln!("{checked} areas match Aurora's");
    assert!(checked >= 40);

    // The Area Wizard lists the tilesets in the same order as Aurora's
    // (positions used by the capture script).
    let list = tilesets(&game);
    assert_eq!(list.len(), 33);
    assert_eq!(list[3].resref.to_string(), "tic01");
    assert_eq!(list[18].resref.to_string(), "dag01");
    assert_eq!(list[24].resref.to_string(), "ttr01");

    // Aurora fails (an access violation) making a 5×5 Lizardfolk Interior:
    // no tile fits the floor patch there.
    let spec = AreaSpec {
        name: "Lizards".into(),
        tileset: ResRef::from_str("dag01").unwrap(),
        width: 5,
        height: 5,
    };
    let err = add_area(&mut ours, &game, &spec, &mut rng).unwrap_err();
    assert!(
        matches!(err, mg_module::new::NewError::Tiles { source: TilesError::NoTile { .. }, .. }),
        "{err}"
    );
}
