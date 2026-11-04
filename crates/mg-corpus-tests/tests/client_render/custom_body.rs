//! A builder's custom body in the client, beside Moonglow (a builder's
//! report, 2026-11-04: "custom phenotype appearances aren't working as they
//! should"). The builder's files are not the project's: they are read
//! where they are on this machine (`mg_testkit::builder_files`), and the
//! reading skips without them.
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render custom_body -- --ignored --nocapture`.

use super::batch2::put;
use super::readings::{INVISIBLE, Reading, enter, logged, moonglow_beside_as, print_log};
use super::*;

/// The builder's files (`dafena-males/x`, a hak's contents, loose) in the
/// scratch user directory's override: an FFXIV-style body whose skeleton
/// (`pmq0`, race Q; `pmv0`, race V) has 195 bones with FFXIV's names
/// (capart.2da hangs the chest from `j_sebo_a`, the neck from `j_kubi`,
/// the head, as the right foot, from `j_kao`), its animations in a chain
/// of supermodels (`pause1` in the second), its body parts skins compiled
/// against the skeleton, and its materials custom shaders (`fs_plt_tinter`,
/// `fs_xiv_hair`) that tint from tint maps and a palette in `texture7` to
/// `texture10`. Commoners (male, phenotype 0) made appearance 15100 (west,
/// a race Q human) and 15101 (east, a race V Miqo'te; `MG_APPEARANCES`
/// for others), wearing nothing, no scripts run, north of the player and
/// facing it; from nearer (`client_near.png`, `SetCameraFacing(90, 4.5,
/// 80)`), then from the reference camera beside Moonglow's area view.
///
/// The files are the males' alone, and the builder's rows 1 and 6 (the
/// elf and the human) are races V and Q: the scratch copy of
/// appearance.2da gives those two rows the game's races again (`E`, `H`),
/// or the player (the client's default character, an elf woman, `pfv0`,
/// not among the files) is a model the client can't load, and it stays on
/// its loading screen. `MG_WAIT` the seconds the client is given before
/// its picture (30: it reads the textures for twenty).
///
/// Seen (2026-11-04): see `docs/findings.md`, "A builder's FFXIV body".
#[test]
#[ignore]
fn reading_custom_skeleton_body() {
    let Some(files) = mg_testkit::builder_files("dafena-males/x") else {
        eprintln!("skipped: the builder's files are not on this machine");
        return;
    };
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("custom_body", "tic01", light, &[]);
    // (Hard links where they can be made: 1.4 GB.)
    let over = r.dir.join("user/override");
    for f in std::fs::read_dir(&files).unwrap().flatten() {
        let to = over.join(f.file_name());
        if std::fs::hard_link(f.path(), &to).is_err() {
            std::fs::copy(f.path(), &to).unwrap();
        }
    }
    // (A file of its own, not the hard link: the builder's is left as it
    // is.)
    let table = std::fs::read_to_string(files.join("appearance.2da")).unwrap();
    let table: Vec<String> = table
        .lines()
        .map(|l| {
            let model = "Character_model           ";
            match l.split_whitespace().next() {
                Some("1") => l.replacen(&format!("{model}V"), &format!("{model}E"), 1),
                Some("6") => l.replacen(&format!("{model}Q"), &format!("{model}H"), 1),
                _ => l.to_string(),
            }
        })
        .collect();
    let at = over.join("appearance.2da");
    std::fs::remove_file(&at).unwrap();
    std::fs::write(&at, table.join("\r\n") + "\r\n").unwrap();
    r.game = GameData::open(&GameInstall::new(
        mg_testkit::nwn_root().unwrap(),
        Some(r.dir.join("user")),
        "en",
    ))
    .unwrap();
    let rows: Vec<u16> = std::env::var("MG_APPEARANCES").map_or_else(
        |_| vec![15100, 15101],
        |r| r.split(',').filter_map(|v| v.trim().parse().ok()).collect(),
    );
    for (appearance, x) in rows.iter().copied().zip([19.0f32, 21.0, 20.0]) {
        put(
            &mut r,
            ResType::UTC,
            "nw_commale",
            "Creature List",
            [x, 24.0, 0.0],
            270.0,
            &[
                ("Appearance_Type", Value::Word(appearance)),
                ("Phenotype", Value::Int(0)),
                ("Gender", Value::Byte(0)),
            ],
        );
    }
    if let Some(list) = r.git.root.list_mut("Creature List") {
        for c in list.iter_mut() {
            // (The commoner's blueprint has no body part fields, and a
            // creature without them shows no parts: a bare body's, head
            // and parts 1, as the preview's `CreatureLook` makes it.)
            let row = c.integer("Appearance_Type").unwrap_or(6) as u16;
            for f in mg_preview::CreatureLook::new(row).to_utc().fields {
                c.set(&f.label.to_string_lossy(), f.value);
            }
            c.set("Equip_ItemList", Value::List(Vec::new()));
            let scripts: Vec<String> = c
                .fields
                .iter()
                .map(|f| f.label.to_string_lossy())
                .filter(|l| l.starts_with("Script"))
                .collect();
            for label in scripts {
                c.set(&label, Value::resref(ResRef::EMPTY));
            }
        }
    }
    let wait = std::env::var("MG_WAIT").unwrap_or_else(|_| "30.0".into());
    let body = format!(
        "{INVISIBLE}\n    Log(\"MG_PLAYER appearance \" + IntToString(GetAppearanceType(pc)));"
    );
    let entry = |camera| {
        enter(&logged(&body), camera).replace(
            "DelayCommand(4.0, Log(\"MG_READY\"))",
            &format!("DelayCommand({wait}, Log(\"MG_READY\"))"),
        )
    };
    let dir = r.dir.clone();
    for (camera, near) in [((90.0, 4.5, 80.0), true), ((90.0, 12.0, 45.0), false)] {
        r.save(&entry(camera), SETTINGS);
        let Some(client) = client_screenshot(&dir, "MgScene") else {
            panic!("the client did not reach the scene (see {})", dir.display());
        };
        print_log(&dir);
        if near {
            std::fs::rename(dir.join("client.png"), dir.join("client_near.png")).unwrap();
            continue;
        }
        let game = GameData::open(&GameInstall::new(
            mg_testkit::nwn_root().unwrap(),
            Some(dir.join("user")),
            "en",
        ))
        .unwrap();
        moonglow_beside_as(&dir, &game, &r.are, &r.git, &client, &|_| {}, "moonglow");
    }
}
