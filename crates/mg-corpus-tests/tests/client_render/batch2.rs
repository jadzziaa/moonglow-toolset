//! The deferred sprint's second client batch (2026-10-29): readings of
//! the client that answer the deferred list's questions, beside the first
//! batch's (`readings.rs`, whose scaffolding they use). Each writes the
//! client's pictures to its scratch directory
//! (`target/test-output/client_reading_<name>/`) and prints where; what
//! was seen is in its comment.
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=:2 cargo test -p mg-corpus-tests --release --test client_render batch2 -- --ignored --nocapture`.

use super::readings::{INVISIBLE, Reading, enter, print_log};
use super::*;
use std::process::Command;

/// `source` (bytes, in the codepage they are in) compiled by the
/// official compiler.
fn compile_bytes(dir: &Path, name: &str, source: &[u8]) -> Vec<u8> {
    let compiler = mg_testkit::nwn_tool("nwn_script_comp").expect("nwn_script_comp");
    let (nss, ncs) = (dir.join(format!("{name}.nss")), dir.join(format!("{name}.ncs")));
    std::fs::write(&nss, source).unwrap();
    let out = Command::new(&compiler)
        .args(["-o", ncs.to_str().unwrap(), nss.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));
    std::fs::read(&ncs).unwrap()
}

/// A script's strings in Windows-1250 in a Polish client (GitHub issue 27,
/// the deferred list's "A script's codepage"): `SpeakString`,
/// `FloatingTextStringOnCreature` and `SendMessageToPC` of "Zażółć gęślą
/// jaźń" written in Windows-1250 bytes in the source, the client's
/// language Polish (`game.language.override = "pl"`, the install's
/// `lang/pl`).
#[test]
#[ignore]
fn reading_polish_script_strings() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("polish_strings", "tic01", light, &[]);
    let polish = mg_core::Language::POLISH.codepage().encode_lossy("Zażółć gęślą jaźń ŁŚŹŻ");
    let body = format!(
        "{INVISIBLE}
    DelayCommand(1.0, AssignCommand(pc, SpeakString(\"MG1 @\")));
    DelayCommand(1.5, FloatingTextStringOnCreature(\"MG2 @\", pc, FALSE));
    DelayCommand(2.0, SendMessageToPC(pc, \"MG3 @\"));"
    );
    let source = enter(&body, (90.0, 8.0, 60.0));
    let mut bytes = Vec::new();
    for (i, part) in source.split('@').enumerate() {
        if i > 0 {
            bytes.extend_from_slice(&polish);
        }
        bytes.extend_from_slice(part.as_bytes());
    }
    let ncs = compile_bytes(&r.dir, "mg_enter", &bytes);
    let settings = format!("{SETTINGS}[game]\n\t[game.language]\n\t\toverride = \"pl\"\n");
    r.save(&source, &settings);
    r.m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), ncs);
    r.m.save_as(&ModuleLocation::Archive(r.dir.join("user/modules/MgScene.mod"))).unwrap();
    let Some(_) = client_screenshot(&r.dir, "MgScene") else {
        panic!("the client did not reach the scene (see {})", r.dir.display());
    };
    print_log(&r.dir);
}

/// A module's feminine talk table in single player (the deferred list's
/// NWSync and language entries): the module's own talk table `mg_rtlk`
/// and `mg_rtlkf` beside it, both in the module and (in a second
/// scratch directory) in the user directory's `tlk`, with other text at
/// each line; the player of `+TestNewModule` is a woman (the first
/// pregenerated character, made one again by `SetGender`). The client's
/// `FloatingTextStrRefOnCreature` and the server's `GetStringByStrRef`
/// (with the gender asked) of line 16777216, and of a game line whose
/// feminine text differs.
#[test]
#[ignore]
fn reading_feminine_talk_table() {
    for place in ["module", "tlk"] {
        let light =
            Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
        let mut r = Reading::new(&format!("feminine_{place}"), "tic01", light, &[]);
        let table = |text: &str| {
            let mut t = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
            t.entries.push(mg_tlk::TlkEntry::text(format!("MG {text} ZERO").as_str()));
            t.entries.push(mg_tlk::TlkEntry::text(format!("MG {text} ONE").as_str()));
            t.to_bytes().unwrap()
        };
        let (m, f) = (table("MASCULINE"), table("FEMININE"));
        if place == "module" {
            r.m.set(ResKey::parse("mg_rtlk", ResType::TLK).unwrap(), m);
            r.m.set(ResKey::parse("mg_rtlkf", ResType::TLK).unwrap(), f);
        } else {
            let tlk = r.dir.join("user/tlk");
            std::fs::create_dir_all(&tlk).unwrap();
            std::fs::write(tlk.join("mg_rtlk.tlk"), m).unwrap();
            std::fs::write(tlk.join("mg_rtlkf.tlk"), f).unwrap();
        }
        let mut info = r.m.info().unwrap();
        info.root.set("Mod_CustomTlk", Value::String(b"mg_rtlk".to_vec()));
        r.m.set_info(&info).unwrap();
        let body = format!(
            "{INVISIBLE}
    SetGender(pc, GENDER_FEMALE);
    DelayCommand(1.0, FloatingTextStrRefOnCreature(16777216, pc, FALSE));
    DelayCommand(1.5, SendMessageToPC(pc, \"MG f: \" + GetStringByStrRef(16777217, GENDER_FEMALE)));
    DelayCommand(2.0, SendMessageToPC(pc, \"MG m: \" + GetStringByStrRef(16777217, GENDER_MALE)));
    Log(\"MG_SERVER f=\" + GetStringByStrRef(16777216, GENDER_FEMALE) + \" m=\" + GetStringByStrRef(16777216, GENDER_MALE) + \" gender=\" + IntToString(GetGender(pc)));"
        );
        let (dir, _) = r.shoot(&enter(&body, (90.0, 8.0, 60.0)), SETTINGS);
        print_log(&dir);
    }
}

/// The blueprint `resref` (`ty`) placed at `at` facing `facing`
/// (degrees) in the reading's GIT list `list`, with `fields` set.
fn put(
    r: &mut Reading,
    ty: ResType,
    resref: &str,
    list: &str,
    at: [f32; 3],
    facing: f32,
    fields: &[(&str, Value)],
) {
    use mg_module::instances::{Placement, Placing, instance};
    let bp = Gff::read(&r.game.resman.get_named(resref, ty).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let at = Placement { position: at, rotation: facing.to_radians() };
    let mut p = instance(&placing, ty, &bp.root, at, &[]).unwrap();
    for (label, v) in fields {
        p.set(label, v.clone());
    }
    let mut items = r.git.root.list(list).map(<[_]>::to_vec).unwrap_or_default();
    items.push(p);
    r.git.root.set(list, Value::List(items));
}

/// The sun's shadows of what the shadows entry asks about, in the client
/// beside Moonglow's area view (ttr01 by day, Sun Shadows on at 100,
/// the client's environment shadows on, the sun towards (4000, 4500,
/// 7000)): north of the player, west to east, a door (`nw_door_ttr_01`)
/// standing free, a longsword lying on the ground (made by the entry
/// script), and three armoires raised 0, 1.5 and 3 m above the ground;
/// then the same from a low camera (`MG_LOW=1`: pitch 85, distance 5),
/// below the raised armoires' tops.
#[test]
#[ignore]
fn reading_shadows_of_doors_items_and_heights() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let low = std::env::var_os("MG_LOW").is_some();
    let name = if low { "shadow_heights_low" } else { "shadow_heights" };
    let mut r = Reading::new(name, "ttr01", light, &[]);
    for (label, v) in [("SunShadows", 1u8), ("MoonShadows", 0), ("ShadowOpacity", 100)] {
        r.are.root.set(label, Value::Byte(v));
    }
    put(&mut r, ResType::UTD, "nw_door_ttr_01", "Door List", [15.5, 24.0, 0.0], 0.0, &[]);
    for (i, z) in [0.0f32, 1.5, 3.0].into_iter().enumerate() {
        let x = 21.0 + 2.5 * i as f32;
        put(&mut r, ResType::UTP, "plc_armoire", "Placeable List", [x, 24.0, z], 180.0, &[]);
    }
    let body = format!(
        "{INVISIBLE}
    CreateObject(OBJECT_TYPE_ITEM, \"nw_wswls001\", Location(area, Vector(18.5, 23.5, 0.0), 30.0));"
    );
    let settings = super::shadows::settings();
    if low {
        let (dir, _) = r.shoot(&enter(&body, (90.0, 5.0, 85.0)), &settings);
        print_log(&dir);
    } else {
        let (dir, _) = r.shoot_beside(&body, &settings, &|v| v.shadows = true);
        print_log(&dir);
    }
}

/// Where the engine places its grass: white opaque tufts (an area
/// override of ttr01's grass, its texture white) on the flat field at two
/// densities (`MG_DENSITIES`, default 0.3 and 1.5 a square meter), from
/// the reference camera, each beside Moonglow's area view; the tufts'
/// pixels counted by band of the picture.
#[test]
#[ignore]
fn reading_grass_placement() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let densities: Vec<f32> = std::env::var("MG_DENSITIES")
        .ok()
        .map(|v| v.split(',').filter_map(|d| d.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![0.3, 1.5]);
    for density in densities {
        let overrides = [("mgwhite.tga", white_tga())];
        let mut r = Reading::new(&format!("grass_placement_{density}"), "ttr01", light, &overrides);
        let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else { panic!() };
        let mut ovr = mg_gff::Struct::new(101);
        let white = grass_override(3, "mgwhite", density, 1.2, [1.0, 1.0, 1.0]);
        ovr.set("GrassOvrList", Value::List(vec![white]));
        props.set("GrassOverrides", Value::Struct(ovr));
        let (dir, client) = r.shoot_beside(INVISIBLE, &super::readings::with_grass(), &|_| {});
        let ours = common::read_png(&dir.join("moonglow_on.png"));
        // The white tufts' share of each band of a tenth of the picture,
        // away from the interface (rows 0.1 to 0.8).
        let white = |img: &Rgba, y0: f32, y1: f32| {
            let (w, h) = (img.width as usize, img.height as usize);
            let mut n = 0usize;
            for y in (y0 * h as f32) as usize..(y1 * h as f32) as usize {
                for x in 0..w {
                    let p = &img.data[(y * w + x) * 4..(y * w + x) * 4 + 3];
                    n += usize::from(p.iter().all(|c| *c > 150));
                }
            }
            n as f32 / ((y1 - y0) * h as f32 * w as f32)
        };
        for band in 1..8 {
            let (y0, y1) = (band as f32 / 10.0, (band + 1) as f32 / 10.0);
            eprintln!(
                "density {density}: rows {y0:.1}-{y1:.1}: white client {:.4}, Moonglow {:.4}",
                white(&client, y0, y1),
                white(&ours, y0, y1)
            );
        }
    }
}

/// A `VisTransformList` entry of `scope`, scaled by `scale` and turned
/// by `turn` (`RotateX`, degrees), as Aurora's Adjust Location writes
/// one (each axis a struct of `ValueTo`).
fn transform_entry(scope: i32, scale: f32, turn: f32) -> mg_gff::Struct {
    let axis = |v: f32| {
        let mut s = mg_gff::Struct::new(0);
        s.set("TimerType", Value::Int(0));
        s.set("ValueTo", Value::Float(v));
        s.set("LerpType", Value::Int(0));
        Value::Struct(s)
    };
    let mut e = mg_gff::Struct::new(6);
    e.set("Scope", Value::Int(scope));
    e.set("AnimationSpeed", axis(1.0));
    for (prefix, v) in
        [("Scale", [scale; 3]), ("Rotate", [turn, 0.0, 0.0]), ("Translate", [0.0; 3])]
    {
        for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
            e.set(&format!("{prefix}{a}"), axis(v[i]));
        }
    }
    e
}

/// Which visual transform the client draws a placeable with when its
/// `VisTransformList` has no entry of scope 0 (Moonglow: the list's
/// first, `mg_module::placement::visual_transform`): five armoires north
/// of the player, west to east: a scope 0 entry (scale 1.6, turned 45),
/// the control; one of scope 1 alone (the same); one of scope 4 alone;
/// scope 1 (scale 1.6) then scope 0 (scale 0.6); no list. In the client
/// beside Moonglow's area view.
#[test]
#[ignore]
fn reading_visual_transform_without_scope_0() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("vt_scopes", "tic01", light, &[]);
    let lists: [Vec<mg_gff::Struct>; 5] = [
        vec![transform_entry(0, 1.6, 45.0)],
        vec![transform_entry(1, 1.6, 45.0)],
        vec![transform_entry(4, 1.6, 45.0)],
        vec![transform_entry(1, 1.6, 45.0), transform_entry(0, 0.6, 0.0)],
        vec![],
    ];
    for (i, list) in lists.into_iter().enumerate() {
        let x = 14.5 + 2.75 * i as f32;
        let fields: Vec<(&str, Value)> =
            if list.is_empty() { vec![] } else { vec![("VisTransformList", Value::List(list))] };
        put(&mut r, ResType::UTP, "plc_armoire", "Placeable List", [x, 24.5, 0.0], 180.0, &fields);
    }
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    print_log(&dir);
}

/// Emitters from above (the deferred list's "Emitters start as they look
/// once going"): a wide fog (`new:Mist`, placeables.2da 791,
/// `tnp_gmist`) west and Magic Sparks (White and Blue, 158 and 159)
/// east, north of the player on ttr01, from the reference camera beside
/// Moonglow's area view (Animations on, its emitters run ahead), then
/// (`MG_TOP=1`) the client's alone from straight above (pitch 1,
/// distance 20).
#[test]
#[ignore]
fn reading_emitters_from_above() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let top = std::env::var_os("MG_TOP").is_some();
    let mut r = Reading::new(if top { "emitters_top" } else { "emitters" }, "ttr01", light, &[]);
    r.place(791, [16.5, 25.0, 0.0], 0.0, &[]);
    r.place(158, [22.5, 24.0, 0.0], 0.0, &[]);
    r.place(159, [24.5, 25.5, 0.0], 0.0, &[]);
    if top {
        let (dir, _) = r.shoot(&enter(INVISIBLE, (90.0, 20.0, 1.0)), SETTINGS);
        print_log(&dir);
    } else {
        let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|v| v.animate = true);
        print_log(&dir);
    }
}
