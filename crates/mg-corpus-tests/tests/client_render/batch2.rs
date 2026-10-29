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
