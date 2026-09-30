//! A module made by Moonglow from nothing, with an area for every tileset
//! (and the Area Wizard's sizes), runs in the engine: `nwserver` loads it,
//! starts at its starting area, and reports every area's properties and
//! every tile (id, orientation, height, lights) as Moonglow wrote them.

use std::time::Duration;

use mg_core::{Gender, Language, ResRef, ResType};
use mg_module::ModuleLocation;
use mg_module::new::{AREA_SIZES, AreaSpec, add_area, new_module, tilesets};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, are, git, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::compile;

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

string I(int n) { return IntToString(n); }

void Finish()
{
    Log("MG_DONE");
    int i;
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}

void ProbeFrom(int nArea, int nTile)
{
    int a = 0;
    object oArea = GetFirstArea();
    while (GetIsObjectValid(oArea) && a < nArea) { oArea = GetNextArea(); a++; }
    while (GetIsObjectValid(oArea))
    {
        string sArea = GetResRef(oArea);
        int w = GetAreaSize(AREA_WIDTH, oArea);
        int h = GetAreaSize(AREA_HEIGHT, oArea);
        if (nTile == 0)
            Log("MG_AREA " + sArea + "|" + GetTag(oArea) + "|" + GetName(oArea) + "|" + GetTilesetResRef(oArea)
                + "|" + I(w) + "x" + I(h) + "|" + I(GetIsAreaInterior(oArea)) + I(GetIsAreaNatural(oArea))
                + I(GetIsAreaAboveGround(oArea)) + "|" + I(GetAreaNoRestFlag(oArea)) + "|" + I(GetSkyBox(oArea))
                + "|" + I(MusicBackgroundGetDayTrack(oArea)) + "," + I(MusicBackgroundGetNightTrack(oArea))
                + "," + I(MusicBackgroundGetBattleTrack(oArea))
                + "|" + I(GetAreaLightColor(AREA_LIGHT_COLOR_MOON_AMBIENT, oArea))
                + "," + I(GetAreaLightColor(AREA_LIGHT_COLOR_MOON_DIFFUSE, oArea))
                + "," + I(GetAreaLightColor(AREA_LIGHT_COLOR_SUN_AMBIENT, oArea))
                + "," + I(GetAreaLightColor(AREA_LIGHT_COLOR_SUN_DIFFUSE, oArea))
                + "|" + I(GetFogAmount(FOG_TYPE_SUN, oArea)) + "," + I(GetFogAmount(FOG_TYPE_MOON, oArea))
                + "," + I(GetFogColor(FOG_TYPE_SUN, oArea)) + "," + I(GetFogColor(FOG_TYPE_MOON, oArea)));
        int t;
        for (t = nTile; t < w * h; t++)
        {
            if (GetScriptInstructionsRemaining() < 20000)
            {
                DelayCommand(0.0, ProbeFrom(a, t));
                return;
            }
            // The tile getters take a position in metres, the light getters
            // the tile's grid coordinates.
            location l = Location(oArea, Vector(IntToFloat(t % w) * 10.0 + 5.0, IntToFloat(t / w) * 10.0 + 5.0, 0.0), 0.0);
            location g = Location(oArea, Vector(IntToFloat(t % w), IntToFloat(t / w), 0.0), 0.0);
            Log("MG_TILE " + sArea + "|" + I(t) + "|" + I(GetTileID(l)) + "|" + I(GetTileOrientation(l))
                + "|" + I(GetTileHeight(l)) + "|" + I(GetTileMainLight1Color(g)) + "," + I(GetTileMainLight2Color(g))
                + "," + I(GetTileSourceLight1Color(g)) + "," + I(GetTileSourceLight2Color(g)));
        }
        nTile = 0;
        a++;
        oArea = GetNextArea();
    }
    DelayCommand(0.0, Finish());
}

void main()
{
    location l = GetStartingLocation();
    vector v = GetPositionFromLocation(l);
    Log("MG_MODULE " + GetModuleName() + "|" + GetTag(GetModule()) + "|" + GetResRef(GetAreaFromLocation(l))
        + "|" + FloatToString(v.x, 0, 1) + "," + FloatToString(v.y, 0, 1));
    ProbeFrom(0, 0);
}
"#;

/// ARE files store colours as 0xBBGGRR; the engine's getters return 0xRRGGBB.
fn rgb(bgr: u32) -> u32 {
    (bgr & 0xff) << 16 | (bgr & 0xff00) | (bgr >> 16) & 0xff
}

/// The engine reports a tile's source light as `TILE_SOURCE_LIGHT_COLOR_*`,
/// one less than the value in the ARE file (so 0 comes back as 255).
fn source(stored: u8) -> u8 {
    stored.wrapping_sub(1)
}

#[test]
fn new_module_runs_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_new_module");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::new();
    let mut m = new_module(&game, "Moonglow New Module", &mut rng).unwrap();

    // Every tileset at 4×4, the Area Wizard's sizes on one tileset, and a
    // rectangle.
    let mut specs: Vec<AreaSpec> = Vec::new();
    for (i, t) in tilesets(&game).iter().enumerate() {
        specs.push(AreaSpec {
            name: format!("Tileset {i:02}"),
            tileset: t.resref,
            width: 4,
            height: 4,
        });
    }
    for (label, n) in AREA_SIZES {
        specs.push(AreaSpec {
            name: label.into(),
            tileset: ResRef::from_str("tic01").unwrap(),
            width: n,
            height: n,
        });
    }
    specs.push(AreaSpec {
        name: "Wide".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 7,
        height: 3,
    });
    let mut areas = Vec::new();
    for spec in &specs {
        areas.push(
            add_area(&mut m, &game, spec, &mut rng).unwrap_or_else(|e| panic!("{spec:?}: {e}")),
        );
    }

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_new.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_new", "MG_DONE", Duration::from_secs(300)).unwrap();
    assert!(run.finished, "the server did not finish");

    // What Moonglow wrote, in the probe's format.
    let first = &areas[0];
    let first_spec = &specs[0];
    let mut expected_areas = Vec::new();
    let mut expected_tiles = Vec::new();
    for area in &areas {
        let a = m.gff(&ResKey::new(*area, ResType::ARE)).unwrap().unwrap().root;
        let g = m.gff(&ResKey::new(*area, ResType::GIT)).unwrap().unwrap().root;
        let props = g.child("AreaProperties").unwrap();
        let flags = a.read(&are::FLAGS);
        let name = a.read(&are::NAME);
        expected_areas.push(format!(
            "{area}|{}|{}|{}|{}x{}|{}{}{}|{}|{}|{},{},{}|{},{},{},{}|{},{},{},{}",
            String::from_utf8_lossy(a.read(&are::TAG).as_bytes()),
            name.text(Language::ENGLISH, Gender::Male).unwrap(),
            a.read(&are::TILESET),
            a.read(&are::WIDTH),
            a.read(&are::HEIGHT),
            // GetIsAreaInterior: interior or underground.
            u32::from(flags & 0b11 != 0),
            (flags >> 2) & 1,
            1 - ((flags >> 1) & 1),
            a.read(&are::NO_REST),
            a.read(&are::SKY_BOX),
            props.read(&git::area_properties::MUSIC_DAY),
            props.read(&git::area_properties::MUSIC_NIGHT),
            props.read(&git::area_properties::MUSIC_BATTLE),
            rgb(a.read(&are::MOON_AMBIENT_COLOR)),
            rgb(a.read(&are::MOON_DIFFUSE_COLOR)),
            rgb(a.read(&are::SUN_AMBIENT_COLOR)),
            rgb(a.read(&are::SUN_DIFFUSE_COLOR)),
            a.read(&are::SUN_FOG_AMOUNT),
            a.read(&are::MOON_FOG_AMOUNT),
            rgb(a.read(&are::SUN_FOG_COLOR)),
            rgb(a.read(&are::MOON_FOG_COLOR)),
        ));
        for (i, t) in a.items(&are::TILE_LIST).iter().enumerate() {
            use are::tile_list as tl;
            expected_tiles.push(format!(
                "{area}|{i}|{}|{}|{}|{},{},{},{}",
                t.read(&tl::TILE_ID),
                t.read(&tl::TILE_ORIENTATION),
                t.read(&tl::TILE_HEIGHT),
                t.read(&tl::TILE_MAIN_LIGHT1),
                t.read(&tl::TILE_MAIN_LIGHT2),
                source(t.read(&tl::TILE_SRC_LIGHT1)),
                source(t.read(&tl::TILE_SRC_LIGHT2)),
            ));
        }
    }
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    assert_eq!(
        run.values("MG_MODULE"),
        [format!(
            "Moonglow New Module|MODULE|{first}|{:.1},{:.1}",
            first_spec.width as f32 * 5.0,
            first_spec.height as f32 * 5.0
        )]
    );
    let differ = |got: Vec<String>, want: &[String]| -> Vec<String> {
        let (got, want) = (sorted(got), sorted(want.to_vec()));
        let only_got = got.iter().filter(|l| !want.contains(l)).map(|l| format!("engine: {l}"));
        let only_want = want.iter().filter(|l| !got.contains(l)).map(|l| format!("written: {l}"));
        only_got.chain(only_want).collect()
    };
    assert_eq!(differ(run.values("MG_AREA"), &expected_areas), Vec::<String>::new());
    assert_eq!(differ(run.values("MG_TILE"), &expected_tiles), Vec::<String>::new());
    eprintln!("{} areas and {} tiles as written", expected_areas.len(), expected_tiles.len());
}
