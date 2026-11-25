//! The deferred list's "read in the client, not yet" entries and the
//! renderer's open comparisons (the eighth client batch, part D,
//! 2026-11-25): readings of the client that only read; nothing of the
//! renderer was changed for them. Each writes the client's pictures (and
//! Moonglow's beside them) to its scratch directory and prints what it
//! measured; what was seen is in its comment, in the same words as the
//! deferred entry it answers.
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render batch8d -- --ignored --nocapture --test-threads 1`.

use std::borrow::Cow;

use super::part_textures::{OWN_PLT, RED, Row, flat_tga, overrides, place, x_of};
use super::programs::{Quad, marked, marks_at, model, standing};
use super::readings::{INVISIBLE, Reading, enter, placeables_with, print_log};
use super::*;

/// A TGA of one color and alpha (4 × 4).
fn solid(c: [u8; 4]) -> Vec<u8> {
    mg_image::write_tga(&Rgba { width: 4, height: 4, data: c.repeat(16) })
}

/// The pixel at (x, y), clamped to the picture.
fn pixel(img: &Rgba, x: i32, y: i32) -> [u8; 4] {
    let x = x.clamp(0, img.width as i32 - 1) as usize;
    let y = y.clamp(0, img.height as i32 - 1) as usize;
    let i = (y * img.width as usize + x) * 4;
    [img.data[i], img.data[i + 1], img.data[i + 2], img.data[i + 3]]
}

/// The mean color of a (2r+1)-pixel square around (x, y).
fn window(img: &Rgba, x: f32, y: f32, r: i32) -> [f32; 3] {
    let mut sum = [0f32; 3];
    let mut n = 0f32;
    for dy in -r..=r {
        for dx in -r..=r {
            let p = pixel(img, x as i32 + dx, y as i32 + dy);
            for k in 0..3 {
                sum[k] += f32::from(p[k]);
            }
            n += 1.0;
        }
    }
    sum.map(|v| v / n)
}

/// Where the world point `p` is in a picture of `w` × `h` from `camera`.
fn project(camera: &Camera, p: Vec3, w: u32, h: u32) -> (f32, f32) {
    let vp = camera.projection(w as f32 / h as f32) * camera.view();
    let c = vp * p.extend(1.0);
    let n = c.truncate() / c.w;
    ((n.x * 0.5 + 0.5) * w as f32, (0.5 - n.y * 0.5) * h as f32)
}

/// The reference scenes' camera (the player at (20, 20)).
fn reference_camera() -> Camera {
    fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH)
}

/// A run of the client's marked programs: the vertex and fragment shader
/// lists and the override files that mark them.
type Shaders = (Vec<String>, Vec<String>, Vec<(String, Vec<u8>)>);

fn marked_shaders(base: &GameData) -> Shaders {
    let (vs, fs) = super::programs::programs(base);
    let mut files = Vec::new();
    for (list, vertex) in [(&vs, true), (&fs, false)] {
        for (i, name) in list.iter().enumerate() {
            let src = base.resman.get_named(name, ResType::SHD).unwrap();
            let text = marked(&String::from_utf8_lossy(&src), vertex, i + 1);
            files.push((format!("{name}.shd"), text.into_bytes()));
        }
    }
    (vs, fs, files)
}

fn as_refs(files: &[(String, Vec<u8>)]) -> Vec<(&str, Vec<u8>)> {
    files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect()
}

// ---------------------------------------------------------------------------
// M10: a body part of a two-digit phenotype, its own textures.

/// The rows of [`batch8d_phenotype_15_parts`]: a chest and a head box of
/// each (`part_textures.rs`' rows), a control of phenotype 0 first.
const PHENOTYPE_ROWS: [Row; 6] = [
    Row {
        label: "0: phenotype 0 control: mesh naming a TGA of no part's, own PLT",
        skeleton: "pmh0",
        n: 301,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "1: phenotype 15, mesh naming a TGA of no part's, own PLT (pmh15_chest302.plt)",
        skeleton: "pmh15",
        n: 302,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "2: phenotype 15, mesh naming a TGA, the PLT phenotype 0's alone (pmh0_chest303.plt)",
        skeleton: "pmh15",
        n: 303,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: true,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "3: phenotype 15, mesh naming a TGA, no PLT at all",
        skeleton: "pmh15",
        n: 304,
        meshes: &["mg_red"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "4: phenotype 15, mesh with bitmap NULL, own PLT",
        skeleton: "pmh15",
        n: 305,
        meshes: &["NULL"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "5: phenotype 15, mesh naming the part's own name, own PLT",
        skeleton: "pmh15",
        n: 306,
        meshes: &["own"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
];

/// What color a window's mean is: the PLT's orange (Cloth 1), the TGA's
/// red, or neither.
fn named_color(c: [f32; 3]) -> &'static str {
    let m = c[0].max(c[1]).max(c[2]).max(1.0);
    let (r, g, b) = (c[0] / m, c[1] / m, c[2] / m);
    if r > 0.8 && g < 0.2 && b < 0.2 {
        "red (the TGA)"
    } else if r > 0.8 && (0.25..0.75).contains(&g) && b < 0.25 {
        "orange (the PLT)"
    } else if m < 40.0 {
        "dark"
    } else if (c[0] - c[1]).abs() < 12.0 && (c[1] - c[2]).abs() < 12.0 {
        "gray (untextured)"
    } else {
        "other"
    }
}

/// M10 of the review of November 2026: a body part of a server's own
/// phenotype (15: `pmh15_chest302`), its own textures. The creatures
/// (human men of phenotype 15, phenotype.2da given a row 15 whose
/// `DefaultPhenoType` is 0, the skeleton `pmh15` a copy of `pmh0` under
/// that name) wear a chest box [`PHENOTYPE_ROWS`] describes: the mesh names
/// the red TGA `mg_red` (or nothing, or its own name) and a flat PLT
/// (orange) is beside it under the part's own name, under the human's
/// (`pmh0_chest303`, found by the fallback) or nowhere. Row 0, phenotype 0,
/// is the control (the client draws such a part in its own PLT whatever
/// the mesh names: `part_textures.rs`). The mean color over each chest is
/// printed for the client and for Moonglow's area view beside it.
///
/// Seen (2026-11-25, 89.8193.37): the client draws all six as Moonglow
/// does: row 1 (own PLT) and row 2 (the PLT of phenotype 0's part, by the
/// fallback) orange, row 3 (no PLT) the red TGA, rows 4 and 5 (a NULL
/// bitmap, the part's own name) orange; the two-digit phenotype is read as
/// `skeleton` writes it.
#[test]
#[ignore]
fn batch8d_phenotype_15_parts() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let kept = mg_testkit::test_output().join("batch8d_phenotype_15");
    std::fs::create_dir_all(&kept).unwrap();
    let rows: Vec<&Row> = PHENOTYPE_ROWS.iter().collect();
    let mut files = overrides(&rows);
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    // The skeleton: the human man's under phenotype 15's name.
    let skeleton = base.resman.get_named("pmh0", ResType::MDL).unwrap();
    let text = String::from_utf8_lossy(&skeleton).replace("pmh0", "pmh15");
    assert!(text.contains("newmodel pmh15"), "pmh0 is not an ASCII model");
    files.push(("pmh15.mdl".into(), text.into_bytes()));
    // phenotype.2da: rows 9 to 14 empty, 15 the server's own.
    let table = base.resman.get_named("phenotype", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).trim_end().to_string();
    for n in 9..=14 {
        table += &format!("\n{n} **** **** 0");
    }
    table += "\n15 Mg_Fifteen **** 0\n";
    files.push(("phenotype.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch8d_phenotype_15", "tic01", light, &as_refs(&files));
    let items = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        place(&mut r, row, x_of(i, rows.len()), &items);
        eprintln!("{} (x {:.2})", row.label, x_of(i, rows.len()));
    }
    let dir = r.dir.clone();
    let (are, git) = (r.are.clone(), r.git.clone());
    let (_, client) = r.shoot(&enter(INVISIBLE, (90.0, 5.0, 75.0)), SETTINGS);
    print_log(&dir);
    save_png(&client, &kept.join("client.png"));
    super::part_textures::moonglow(
        &dir,
        &are,
        &git,
        &[],
        "p15",
        "moonglow",
        &kept,
        &client,
        "4.6,74,1.0,48",
    );
    // The chest's middle: the camera of `moonglow` is the fitted one of
    // the group's, whose error is a few pixels; each chest's window is 9
    // pixels across.
    let camera = fitted_camera(Vec3::new(20.0, 20.0, 0.0), 48.0, 1.0, 4.6, 74.0);
    for (side, file) in [
        ("client", "client.png"),
        ("moonglow (own shader)", "p15_moonglow.png"),
        ("moonglow (game's)", "p15_moonglow_on.png"),
    ] {
        let img = if file == "client.png" {
            client.clone()
        } else if kept.join(file).is_file() {
            common::read_png(&kept.join(file))
        } else {
            continue;
        };
        for i in 0..rows.len() {
            // (A chest box hangs about 1.1 m up on the human man.)
            let (x, y) =
                project(&camera, Vec3::new(x_of(i, rows.len()), 22.2, 1.2), img.width, img.height);
            let c = window(&img, x, y, 4);
            eprintln!("{side:22} row {i}: {:5.0?} {}", c, named_color(c));
        }
    }
    eprintln!("{}", kept.display());
}

// ---------------------------------------------------------------------------
// M13: a creature holding an item that names no model.

/// M13 of the review: a creature whose right-hand item names no model.
/// baseitems.2da (a copy in the scratch override) gets three rows of the
/// longsword's, 113 with `ItemClass` and `DefaultModel` empty, 114 with the
/// class `WSwLs` and the part 99 (`wswls_099`: no such model) and
/// `DefaultModel` empty, 115 with the class `MgNone` (no model of the
/// name) and `DefaultModel` empty; the creatures (human men, west to east:
/// the game's longsword as the control, then the items of 113, 114 and 115)
/// hold each in the right hand. The client's picture and Moonglow's beside
/// it, a close one of the hands.
///
/// Seen (2026-11-25, 89.8193.37): the control longsword is drawn in the
/// hand; with any of the three no-model items the client never leaves its
/// loading screen (45 s watched; its engine log: `CreateItem: Model
/// _B_000 nor the default model could be loaded`, `Message type
/// mismatch`, `Unknown Update sub-message`): it neither draws the creature
/// nor the item. Moonglow draws the creature without the item.
#[test]
#[ignore]
fn batch8d_item_without_model() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let kept = mg_testkit::test_output().join("batch8d_item_without_model");
    std::fs::create_dir_all(&kept).unwrap();
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let table = base.resman.get_named("baseitems", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).trim_end().to_string();
    let header: Vec<String> =
        table.lines().nth(2).unwrap().split_whitespace().map(str::to_owned).collect();
    let column = |name: &str| header.iter().position(|c| c == name).unwrap() + 1;
    let longsword: Vec<String> = table
        .lines()
        .find(|l| l.split_whitespace().next() == Some("1"))
        .unwrap()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let last: usize =
        table.lines().last().unwrap().split_whitespace().next().unwrap().parse().unwrap();
    let new_rows = [("MgNoModel", "****"), ("WSwLs", "****"), ("MgNone", "****")];
    for (k, (class, default)) in new_rows.iter().enumerate() {
        let mut row = longsword.clone();
        row[0] = (last + 1 + k).to_string();
        row[column("label")] = format!("mg_item{k}");
        row[column("ItemClass")] = if k == 0 { "****".into() } else { (*class).to_owned() };
        row[column("DefaultModel")] = (*default).to_owned();
        table += &format!("\n{}", row.join(" "));
    }
    table += "\n";
    let files = vec![("baseitems.2da".to_string(), table.into_bytes())];
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch8d_item_without_model", "tic01", light, &as_refs(&files));
    // The items: the longsword's blueprint with another base item and part.
    let blueprint =
        Gff::read(&r.game.resman.get_named("nw_wswls001", ResType::UTI).unwrap()).unwrap();
    // (A row of the game's own that has a part number: 1, the longsword.)
    let variants: [(&str, i64, u8); 4] = [
        ("mg_item_ctl", 1, 1),
        ("mg_item_113", (last + 1) as i64, 0),
        ("mg_item_114", (last + 2) as i64, 99),
        ("mg_item_115", (last + 3) as i64, 1),
    ];
    let mut items = HashMap::new();
    for (name, base_item, part) in variants {
        let mut g = blueprint.clone();
        g.root.set("BaseItem", Value::Int(base_item as i32));
        g.root.set("ModelPart1", Value::Byte(part));
        g.root.set("PropertiesList", Value::List(Vec::new()));
        g.root.set("TemplateResRef", Value::resref(ResRef::from_str(name).unwrap()));
        let key = ResKey::new(ResRef::from_str(name).unwrap(), ResType::UTI);
        r.m.set_gff(key, &g).unwrap();
        items.insert(name.to_string(), g);
    }
    let bp = Gff::read(&r.game.resman.get_named("nw_commale", ResType::UTC).unwrap()).unwrap();
    let item = |r: ResRef| items.get(r.as_str()?).map(|g| g.root.clone());
    let placing = Placing { game: &r.game, item: &item };
    let mut list = Vec::new();
    // (`MG_ITEMS=0,2`: only these creatures, to tell which stops the client.)
    let wanted: Vec<usize> = std::env::var("MG_ITEMS")
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|_| (0..variants.len()).collect());
    let chosen: Vec<_> = variants.iter().enumerate().filter(|(i, _)| wanted.contains(i)).collect();
    for (n, (_, (name, ..))) in chosen.iter().enumerate() {
        let mut c = bp.root.clone();
        // (A human of parts, as `part_textures.rs` makes them: the
        // commoner's own appearance is a fixed model.)
        let look = mg_preview::CreatureLook::new(6);
        for f in look.to_utc().fields {
            c.set(&f.label.to_string_lossy(), f.value);
        }
        c.set("Race", Value::Byte(6));
        let mut e = mg_gff::Struct::new(0x10);
        e.set("EquippedRes", Value::resref(ResRef::from_str(name).unwrap()));
        c.set("Equip_ItemList", Value::List(vec![e]));
        // (The game takes off what a creature may not wear: the weapon
        // proficiencies.)
        let feats = [32u16, 44, 45, 46]
            .map(|f| {
                let mut e = mg_gff::Struct::new(1);
                e.set("Feat", Value::Word(f));
                e
            })
            .to_vec();
        c.set("FeatList", Value::List(feats));
        c.set("ItemList", Value::List(Vec::new()));
        let scripts: Vec<String> = c
            .fields
            .iter()
            .map(|f| f.label.to_string_lossy())
            .filter(|l| l.starts_with("Script"))
            .collect();
        for label in scripts {
            c.set(&label, Value::resref(ResRef::EMPTY));
        }
        let at = Placement {
            position: [x_of(n, chosen.len()), 22.2, 0.0],
            rotation: std::f32::consts::PI,
        };
        list.push(instance(&placing, ResType::UTC, &c, at, &[]).unwrap());
        eprintln!("creature {n}: holds {name} (x {:.2})", x_of(n, chosen.len()));
    }
    r.git.root.set("Creature List", Value::List(list));
    let dir = r.dir.clone();
    let (are, git) = (r.are.clone(), r.git.clone());
    // (Four pictures, fifteen seconds apart: whether the client ever
    // leaves its loading screen.)
    r.save(&enter(INVISIBLE, (90.0, 4.0, 50.0)), SETTINGS);
    let Some(shots) = common::client_screenshots(&dir, "MgScene", 4, Duration::from_secs(15))
    else {
        panic!("the client did not reach the scene (see {})", dir.display());
    };
    print_log(&dir);
    for (i, (seconds, shot)) in shots.iter().enumerate() {
        eprintln!("picture {i}: {seconds:.1} s after the first");
        save_png(shot, &kept.join(format!("client_{i}.png")));
    }
    let client = shots.last().unwrap().1.clone();
    save_png(&client, &kept.join("client.png"));
    super::part_textures::moonglow(
        &dir,
        &are,
        &git,
        &[],
        "item",
        "moonglow",
        &kept,
        &client,
        "3.68,49,1.0,48",
    );
    eprintln!("{}", kept.display());
}

// ---------------------------------------------------------------------------
// A bucket of several models' meshes.

/// The sets of [`batch8d_bucket_across_models`]: each is two models of one
/// card each, both naming the set's one texture: whether the first of the
/// two (`a`) is the see-through one, and whether the area lists `b` before
/// `a`.
const BUCKET_SETS: [(&str, bool, bool); 4] = [
    ("a opaque, b see-through, listed a b", false, false),
    ("a opaque, b see-through, listed b a", false, true),
    ("a see-through, b opaque, listed a b", true, false),
    ("a see-through, b opaque, listed b a", true, true),
];

/// One run of [`batch8d_bucket_across_models`]: the shaders marked, or not
/// (the cards as the client draws them).
fn bucket_across_models_run(unmarked: bool) {
    let base = super::batch5::base_game();
    let (vs, fs, shaders) = marked_shaders(&base);
    let mut overrides: Vec<(String, Vec<u8>)> = if unmarked { Vec::new() } else { shaders };
    let colors = [[230, 30, 30, 255], [30, 200, 30, 255], [40, 60, 255, 255], [230, 220, 30, 255]];
    let mut models = Vec::new();
    for (s, (_, a_see_through, _)) in BUCKET_SETS.iter().enumerate() {
        overrides.push((format!("mgbx{s}t.tga"), solid(colors[s])));
        for (k, name) in ["a", "b"].iter().enumerate() {
            let see_through = (k == 0) == *a_see_through;
            let m = format!("mgbx{s}{name}");
            let quad = Quad {
                name: "card".into(),
                at: [0.0, 0.0, 0.1],
                corners: standing(0.8),
                bitmap: Cow::Owned(format!("mgbx{s}t")),
                extra: if see_through { "  alpha 0.5\n" } else { "" },
                tverts: true,
                colors: false,
            };
            overrides.push((format!("{m}.mdl"), model(&m, &[quad]).into_bytes()));
            models.push(m);
        }
    }
    let rows: Vec<(&str, u8)> = models.iter().map(|m| (m.as_str(), 1)).collect();
    let (table, first) = placeables_with(&base, &rows);
    overrides.push(("placeables.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let name = if unmarked { "batch8d_bucket_models_unmarked" } else { "batch8d_bucket_models" };
    let mut r = Reading::new(name, "tcn01", light, &as_refs(&overrides));
    // (West to east: a at 17.4, b at 18.8; set s at y 22 + 2 s.)
    let at = |s: usize, k: usize| [17.4 + 1.4 * k as f32, 22.0 + 2.0 * s as f32];
    for (s, (_, _, b_first)) in BUCKET_SETS.iter().enumerate() {
        let order = if *b_first { [1, 0] } else { [0, 1] };
        for k in order {
            let [x, y] = at(s, k);
            r.place(first + (2 * s + k) as u32, [x, y, 0.0], 0.0, &[("Static", Value::Byte(1))]);
        }
    }
    let dir = r.dir.clone();
    let client = if unmarked {
        let (_, c) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
        c
    } else {
        r.shoot(&enter(INVISIBLE, (90.0, 12.0, 45.0)), SETTINGS).1
    };
    save_png(
        &client,
        &dir.join(if unmarked { "client_unmarked.png" } else { "client_marked.png" }),
    );
    let camera = reference_camera();
    for (s, (what, ..)) in BUCKET_SETS.iter().enumerate() {
        for (k, card) in ["a", "b"].iter().enumerate() {
            let [x, y] = at(s, k);
            let (sx, sy) =
                project(&camera, Vec3::new(x + 0.4, y, 0.5), client.width, client.height);
            if unmarked {
                let c = window(&client, sx, sy, 2);
                eprintln!("set {s} ({what}) card {card}: client color {c:5.0?}");
                continue;
            }
            let marks = marks_at(&client, sx, sy, 4);
            let seen = marks.first().map_or("(no mark)".to_string(), |(m, share)| {
                let v = vs.get(usize::from(m.vs).wrapping_sub(1)).map_or("?", String::as_str);
                let f = fs.get(usize::from(m.fs).wrapping_sub(1)).map_or("?", String::as_str);
                format!("{v}/{f} NO_DISCARD {} ({:.0}%)", u8::from(m.no_discard), share * 100.0)
            });
            eprintln!("set {s} ({what}) card {card}: {seen}");
        }
    }
    eprintln!("{}", dir.display());
}

/// A bucket of several models' meshes (the deferred list's M4: "two static
/// placeables of one texture"). Four sets, each two static placeables
/// (models `a` and `b`, one card each, both naming the set's texture; one
/// opaque and one `alpha 0.5`), the models and the area's order varied: a
/// card's `NO_DISCARD` marked (`program_choice_in_the_client`'s marks) in
/// one run, the cards as the client draws them in a second
/// (`client_unmarked.png`, with Moonglow's beside it). `batch7_bucket_order`
/// read that the first mesh of a pair in one model's order gives both their
/// `NO_DISCARD` and opacity: whether the second placeable's card takes the
/// first's is what this reads.
///
/// Seen (2026-11-25, 89.8193.37): every card keeps its own `NO_DISCARD` and
/// opacity (opaque 1, `alpha 0.5` 0) in all four sets, whichever model or
/// area order: the client keeps each model's meshes apart, as Moonglow does.
#[test]
#[ignore]
fn batch8d_bucket_across_models() {
    let _ = oracle_tool!("nwn_script_comp");
    for unmarked in [false, true] {
        bucket_across_models_run(unmarked);
    }
}

// ---------------------------------------------------------------------------
// Two models' see-through meshes.

/// The configurations of [`batch8d_see_through_two_models`]: which model
/// is nearer to the eye (`R` or `B`) and which the area lists first.
const SEE_THROUGH_CONFIGS: [(char, char); 4] = [('R', 'R'), ('B', 'R'), ('R', 'B'), ('B', 'B')];

/// A pair of [`batch8d_see_through_two_models`]: its label, set, whether
/// static, the nearer model, the one listed first and the nearer's place.
type Pair = (String, &'static str, u8, char, char, [f32; 2]);

/// The models of [`batch8d_see_through_two_models`]: each set's red and
/// blue model (a card of one texture each). The `base` ones are placed
/// both as non-static and as static placeables, so that the two share their
/// textures; the `own` ones only as static, with textures of their own.
const SEE_THROUGH_SETS: [(&str, [&str; 2], [&str; 2]); 2] = [
    ("base", ["mgst_rm", "mgst_bm"], ["mgst_r", "mgst_b"]),
    ("own", ["mgsu_rm", "mgsu_bm"], ["mgsu_r", "mgsu_b"]),
];

/// How two models' meshes of `alpha 0.5` (each one card, red and blue,
/// 1.6 m square, standing facing the eye) are drawn over one another where
/// they overlap on the screen: whichever is drawn second blends over the
/// other, the first writing its depth. Twelve pairs, four configurations
/// ([`SEE_THROUGH_CONFIGS`]: the nearer model, the one listed first in the
/// area's placeable list) of non-static placeables in the south row and of
/// static ones in the row north of it. The nearer card is 0.8 m nearer the
/// eye and 0.8 m west of the farther. Where they overlap the color is
/// either the nearer's alone (the farther was drawn after, and hidden by
/// the nearer's depth: the nearer first) or has the farther's in it (the
/// farther first). Four more static pairs, the same configurations
/// with models and textures of their own ([`SEE_THROUGH_SETS`]' `own`),
/// which no non-static placeable shares.
///
/// Which of the two is drawn first, for each configuration, tells how the
/// client orders them: always the farther (an order by distance, farthest
/// first), always the nearer, the one listed first, the red model, the
/// blue model.
///
/// Seen (2026-11-25, 89.8193.37): non-static placeables, and static ones
/// whose textures are their own, are drawn farthest first by their
/// distance in all four configurations (as Moonglow); static ones sharing
/// their textures with non-static placeables were not (configurations 1
/// and 3: the blue first whichever was near; cause not found).
#[test]
#[ignore]
fn batch8d_see_through_two_models() {
    let _ = oracle_tool!("nwn_script_comp");
    let base = super::batch5::base_game();
    let mut overrides: Vec<(String, Vec<u8>)> = Vec::new();
    // The models, in the order of their rows: each set's, the base's
    // twice (non-static, static).
    let mut rows: Vec<(String, u8)> = Vec::new();
    for (set, models, textures) in SEE_THROUGH_SETS {
        for (k, color) in [[255, 0, 0, 255], [0, 0, 255, 255]].into_iter().enumerate() {
            overrides.push((format!("{}.tga", textures[k]), solid(color)));
            let quad = Quad {
                name: "card".into(),
                at: [0.0, 0.0, 0.0],
                corners: [[-0.8, 0.0, 0.0], [0.8, 0.0, 0.0], [0.8, 0.0, 1.6], [-0.8, 0.0, 1.6]],
                bitmap: Cow::Owned(textures[k].to_string()),
                extra: "  alpha 0.5\n",
                tverts: true,
                colors: false,
            };
            overrides.push((format!("{}.mdl", models[k]), model(models[k], &[quad]).into_bytes()));
        }
        for statics in if set == "base" { vec![0u8, 1] } else { vec![1] } {
            for model in models {
                rows.push((model.to_string(), statics));
            }
        }
    }
    let list: Vec<(&str, u8)> = rows.iter().map(|(m, s)| (m.as_str(), *s)).collect();
    let (table, first) = placeables_with(&base, &list);
    overrides.push(("placeables.2da".into(), table.into_bytes()));
    // The appearance of a set's model (`k` 0 red, 1 blue) as non-static or
    // static.
    let appearance = |set: &str, k: usize, statics: u8| -> u32 {
        let (mut at, mut found) = (0, None);
        for (s, ..) in SEE_THROUGH_SETS {
            let kinds: Vec<u8> = if s == "base" { vec![0, 1] } else { vec![1] };
            for kind in kinds {
                if s == set && kind == statics {
                    found = Some(at + k);
                }
                at += 2;
            }
        }
        first + found.unwrap() as u32
    };
    let light = Lighting { ambient: 0xA0A0A0, diffuse: 0x606060, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch8d_see_through_two", "tcn01", light, &as_refs(&overrides));
    let columns = [14.8f32, 18.4, 22.0, 25.6];
    // Each pair: the label, the set, whether static, the nearer model,
    // the one listed first, the nearer's place (the farther at +0.8, +0.8).
    let mut pairs: Vec<Pair> = Vec::new();
    for (c, (near, listed)) in SEE_THROUGH_CONFIGS.iter().enumerate() {
        for (set, y, statics) in [("base", 23.0f32, 0u8), ("base", 27.0, 1), ("own", 31.0, 1)] {
            pairs.push((format!("config {c}"), set, statics, *near, *listed, [columns[c], y]));
        }
    }
    for (_, set, statics, near, listed, at) in &pairs {
        let (near_at, far_at) = (*at, [at[0] + 0.8, at[1] + 0.8]);
        let order = if listed == &'R' { ['R', 'B'] } else { ['B', 'R'] };
        for model in order {
            let [px, py] = if model == *near { near_at } else { far_at };
            let k = usize::from(model == 'B');
            r.place(
                appearance(set, k, *statics),
                [px, py, 0.0],
                0.0,
                &[("Static", Value::Byte(*statics))],
            );
        }
    }
    let dir = r.dir.clone();
    let (_, client) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    save_png(&client, &dir.join("client_picture.png"));
    let pictures: Vec<(&str, Rgba)> = [
        ("client", client.clone()),
        ("moonglow (own shader)", common::read_png(&dir.join("moonglow.png"))),
        ("moonglow (game's)", common::read_png(&dir.join("moonglow_on.png"))),
    ]
    .into();
    let camera = reference_camera();
    for (label, set, statics, near, listed, at) in &pairs {
        let (near_at, far_at) = (*at, [at[0] + 0.8, at[1] + 0.8]);
        let rect = |at: [f32; 2], img: &Rgba| {
            let pts = [(-0.8, 0.0), (0.8, 0.0), (0.8, 1.6), (-0.8, 1.6)].map(|(dx, z)| {
                project(&camera, Vec3::new(at[0] + dx, at[1], z), img.width, img.height)
            });
            let xs = pts.map(|p| p.0);
            let ys = pts.map(|p| p.1);
            [
                xs.iter().copied().fold(f32::MAX, f32::min),
                ys.iter().copied().fold(f32::MAX, f32::min),
                xs.iter().copied().fold(f32::MIN, f32::max),
                ys.iter().copied().fold(f32::MIN, f32::max),
            ]
        };
        for (side, img) in &pictures {
            let (n, f) = (rect(near_at, img), rect(far_at, img));
            let overlap = [n[0].max(f[0]), n[1].max(f[1]), n[2].min(f[2]), n[3].min(f[3])];
            let mid = ((overlap[0] + overlap[2]) / 2.0, (overlap[1] + overlap[3]) / 2.0);
            let near_only = (n[0] + (n[2] - n[0]) * 0.12, n[3] - (n[3] - n[1]) * 0.12);
            let far_only = (f[2] - (f[2] - f[0]) * 0.12, f[1] + (f[3] - f[1]) * 0.12);
            let [o, a, b] = [mid, near_only, far_only].map(|(x, y)| window(img, x, y, 2));
            let d = (0..3).map(|k| (o[k] - a[k]).abs()).fold(0.0, f32::max);
            let verdict = if d < 10.0 { "nearer first (farther hidden)" } else { "farther first" };
            eprintln!(
                "{side:22} {} {label} {set} (nearer {near}, listed first {listed}): overlap \
                 {o:4.0?} nearer alone {a:4.0?} farther alone {b:4.0?}: {d:4.1} apart: {verdict}",
                if *statics == 1 { "static    " } else { "non-static" }
            );
        }
    }
    eprintln!("{}", dir.display());
}

// ---------------------------------------------------------------------------
// Particles' size.

/// The scene of [`batch8d_particle_size`]: a placeable of one model, a red
/// card 0.7 m square west of its middle and a blue particle of size 0.7 an
/// equal distance east (the same depth from the eye, at the middle's
/// height 1.5 m), the placeable at two distances.
fn particle_size_model() -> String {
    let card = Quad {
        name: "card".into(),
        at: [-1.35, 0.0, 1.15],
        corners: standing(0.7),
        bitmap: Cow::Borrowed("mgpsz_red"),
        extra: "",
        tverts: true,
        colors: false,
    };
    let text = model("mg_psz", &[card]);
    let emitter = super::soft_particles::one(
        "e0",
        [1.0, 0.0, 1.5],
        (0.7, 0.7),
        "Normal",
        "Normal",
        [0.2, 0.2, 1.0],
    )
    .replace("parent mg_soft", "parent mg_psz");
    text.replace("endmodelgeom", &format!("{emitter}endmodelgeom"))
}

/// The width in pixels of the solid-colored shape through the pixel
/// nearest (x, y) with the greatest `strength` in a 25-pixel square,
/// averaged over five rows: in each row the distance between the two
/// places the strength crosses half its plateau (the median of the seven
/// pixels about the strongest), by linear interpolation between pixels.
fn run_width(
    img: &Rgba,
    (x, y): (f32, f32),
    strength: impl Fn([u8; 4]) -> f32,
) -> (f32, (i32, i32)) {
    // (The strongest pixel's near neighbors are not the shape's edge: of
    // those over four fifths of it, the one nearest the projected middle.)
    let mut best = 0f32;
    for dy in -12..=12 {
        for dx in -12..=12 {
            best = best.max(strength(pixel(img, x as i32 + dx, y as i32 + dy)));
        }
    }
    let mut at = (x as i32, y as i32, f32::MAX);
    for dy in -12..=12i32 {
        for dx in -12..=12i32 {
            let from = (dx * dx + dy * dy) as f32;
            if strength(pixel(img, x as i32 + dx, y as i32 + dy)) >= 0.8 * best && from < at.2 {
                at = (x as i32 + dx, y as i32 + dy, from);
            }
        }
    }
    let (cx, cy) = (at.0, at.1);
    let mut total = 0f32;
    let rows = [-4, -2, 0, 2, 4];
    for dy in rows {
        let s = |x: i32| strength(pixel(img, x, cy + dy));
        let mut near: Vec<f32> = (-3..=3).map(|d| s(cx + d)).collect();
        near.sort_by(f32::total_cmp);
        let plateau = near[3];
        let mut edges = [0f32; 2];
        for (k, step) in [-1i32, 1].into_iter().enumerate() {
            let half = plateau / 2.0;
            // (Back to the last pixel over half, then to the crossing.)
            let mut x = cx;
            while s(x + step) > half && (x - cx).abs() < 80 {
                x += step;
            }
            let (above, below) = (s(x), s(x + step));
            let t = if above > below { (above - half) / (above - below) } else { 0.0 };
            edges[k] = x as f32 + step as f32 * t;
        }
        total += edges[1] - edges[0];
    }
    (total / rows.len() as f32, (cx, cy))
}

/// Particles' size (the deferred list: "28 pixels a side against the
/// client's 30 in `surfaces_in_the_client`: the fitted camera's error, or
/// the size?"): a particle of size 0.7 beside a card of 0.7 m, at the same
/// depth, the middle of each projected by the fitted camera and the width
/// of each read at the pixels (a ratio the camera's error cancels from),
/// at two distances from the eye (the placeable at y 21 and y 25). The
/// client's picture, and Moonglow's of the same scene (soft particles on).
///
/// Seen (2026-11-25, 89.8193.37): the particle is as wide as the card
/// (ratio 1.004 and 0.996 in the client, 1.004 and 0.985 in Moonglow); the
/// whole picture is about 3 to 5 percent smaller in Moonglow (card 30.8
/// against 31.9 pixels, 26.6 against 28.1): the fitted camera's error, not
/// the particles' size.
#[test]
#[ignore]
fn batch8d_particle_size() {
    let base = super::batch5::base_game();
    let files: Vec<(String, Vec<u8>)> = vec![
        ("mg_psz.mdl".into(), particle_size_model().into_bytes()),
        ("mgpsz_red.tga".into(), flat_tga(8, 8, RED)),
        ("mgwhite.tga".into(), super::white_tga()),
    ];
    let mut files = files;
    let (table, first) = placeables_with(&base, &[("mg_psz", 0)]);
    files.push(("placeables.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch8d_particle_size", "tic01", light, &as_refs(&files));
    let distances = [21.0f32, 25.0];
    for y in distances {
        r.place(first, [20.0, y, 0.0], 0.0, &[]);
    }
    let dir = r.dir.clone();
    let (are, git) = (r.are.clone(), r.git.clone());
    let (_, client) = r.shoot(&enter(INVISIBLE, (90.0, 12.0, 45.0)), SETTINGS);
    save_png(&client, &dir.join("client_picture.png"));
    let game = GameData::open(&GameInstall::new(
        mg_testkit::nwn_root().unwrap(),
        Some(dir.join("user")),
        "en",
    ))
    .unwrap();
    let ours = moonglow_particles(&game, &are, &git, (client.width, client.height), |_| {});
    if let Some(img) = &ours {
        save_png(img, &dir.join("moonglow_picture.png"));
    }
    let camera = reference_camera();
    let red = |p: [u8; 4]| (f32::from(p[0]) - f32::from(p[1])).max(0.0);
    let blue = |p: [u8; 4]| (f32::from(p[2]) - f32::from(p[0])).max(0.0);
    let mut widths: Vec<(&str, f32, f32)> = Vec::new();
    let pictures: Vec<(&str, &Rgba)> = std::iter::once(("client", &client))
        .chain(ours.as_ref().map(|i| ("moonglow", i)))
        .collect();
    for (side, img) in pictures {
        for y in distances {
            let c = project(&camera, Vec3::new(19.0, y, 1.5), img.width, img.height);
            let p = project(&camera, Vec3::new(21.0, y, 1.5), img.width, img.height);
            let (card, at_card) = run_width(img, c, red);
            let (particle, at_particle) = run_width(img, p, blue);
            eprintln!(
                "{side:9} y {y}: card {card:5.2} px at {at_card:?} (projected {c:.0?}), particle \
                 {particle:5.2} px at {at_particle:?} (projected {p:.0?}); particle / card {:.3}, \
                 pixels per metre {:.1}",
                particle / card,
                card / 0.7
            );
            widths.push((side, card, particle));
        }
    }
    for pair in widths.chunks(2) {
        eprintln!(
            "{}: the nearer over the farther: card {:.3}, particle {:.3}",
            pair[0].0,
            pair[0].1 / pair[1].1,
            pair[0].2 / pair[1].2
        );
    }
    eprintln!("{}", dir.display());
}

/// Moonglow's area view of the scene as the soft-particle readings draw
/// it (the game's shaders, the fade on), `None` without a GPU. `view`
/// changes the area view's settings.
fn moonglow_particles(
    game: &GameData,
    are: &Gff,
    git: &Gff,
    (w, h): (u32, u32),
    view: impl Fn(&mut mg_area::View),
) -> Option<Rgba> {
    crate::common::gpu_after_display();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("Moonglow's side skipped: no GPU");
        return None;
    };
    let tileset = mg_area::tileset(game, ResRef::from_str("tic01").unwrap()).ok();
    let model = mg_area::AreaModel::read(game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, game, &model);
    let camera = reference_camera();
    let mut v = mg_area::View { fog: false, time: 6.0, ..mg_area::View::of(&model) };
    view(&mut v);
    let mut scene = area_scene.scene(&model, &v);
    let mut sims = HashMap::new();
    scene.particles = area_scene.particles(&model, &v, &mut sims, 0.0, camera.view());
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.game_shaders = true;
    r.soft_particles = true;
    Some(r.render_image(&gpu, &game.resman, &scene, &camera, w, h))
}

// ---------------------------------------------------------------------------
// Soft particles' foot.

/// The game's `fsparticle.shd` painting, over each particle, the fade it
/// computes: red the fade's value scaled to four fifths, green 1, blue 0.2,
/// alpha 0.5 (so that the pixel is `0.5 c + 0.5 floor` where the blend is
/// normal, `floor + 0.5 c` where it adds).
fn fade_shader(game: &GameData) -> String {
    let src = game.resman.get_named("fsparticle", ResType::SHD).expect("fsparticle.shd");
    let src = String::from_utf8_lossy(&src).replace("\r\n", "\n");
    let end = "    gl_FragColor = FragmentColor;\n}";
    let at = src.rfind(end).expect("fsparticle's end");
    let mut out = src.clone();
    out.insert_str(
        at + end.len() - 1,
        "#if SHADER_QUALITY_MODE > 0 && !defined MOBILE\n    float mgF = 1.0;\n    \
         if (particleSoftness > 0.0) mgF = sqr(clamp((GetDepth(gl_FragCoord.xy * vPixelSize) \
         + GetFragmentViewZ()) * particleSoftness, 0.0, 1.0));\n    \
         gl_FragColor = vec4(mgF * 0.8, 1.0, 0.2, 0.5);\n#endif\n",
    );
    out
}

/// Rows of a floor particle's foot kept (their middles 0.5, 1.5... rows
/// above the particle's lower edge).
const FOOT_LENGTH: usize = 14;

/// A floor particle's foot in a picture: its fade's value, the mean over
/// the columns of the particle's middle, at [`FOOT_LENGTH`] distances above
/// the lower edge (in rows, the edge found to a part of a row by the share
/// of its last pixel the particle covers, read off the green channel the
/// patched shader keeps at 1 whatever the fade), and the interior's and
/// the floor's red. The value is `(red - base) / (interior - base)`, the
/// base the red where the fade is 0: nothing where the pixel is the
/// painted color (a normal or a punch-through particle), the floor's where
/// it is added to it. `None` where no column shows the particle.
fn foot_of(
    img: &Rgba,
    (x0, x1, y): (u32, u32, u32),
    additive: bool,
) -> Option<([f32; FOOT_LENGTH], f32, f32, f32)> {
    let mut curve = [0f32; FOOT_LENGTH];
    let (mut interior, mut floor, mut edge_sum, mut n) = (0f32, 0f32, 0f32, 0f32);
    for x in x0..=x1 {
        let at = |yy: i32| pixel(img, x as i32, yy);
        // (The floor's green just below the edge: a floor's pattern changes
        // over 30 rows. A painted pixel is 127 (added) to 150 (not) over it.)
        let floor_g =
            (y as i32 + 6..=y as i32 + 14).map(|yy| f32::from(at(yy)[1])).sum::<f32>() / 9.0;
        let threshold = if additive { 60.0 } else { 40.0 };
        let Some(last) = (y as i32 - 40..y as i32 + 6)
            .rev()
            .find(|&yy| f32::from(at(yy)[1]) - floor_g > threshold)
        else {
            continue;
        };
        let top_g = f32::from(at(last - 14)[1]);
        let cover =
            ((f32::from(at(last)[1]) - floor_g) / (top_g - floor_g).max(1.0)).clamp(0.0, 1.0);
        let edge = last as f32 + cover;
        let floor_r = f32::from(at(last + 8)[0]);
        let base = if additive { floor_r } else { 0.0 };
        let top = (8..=14).map(|k| f32::from(at(last - k)[0])).sum::<f32>() / 7.0;
        for (k, v) in curve.iter_mut().enumerate() {
            // (The pixel row whose middle is nearest k + 0.5 above the edge,
            // each weighted to it: linear between two rows.)
            let row = edge - (k as f32 + 0.5) - 0.5;
            let (lo, t) = (row.floor() as i32, row - row.floor());
            let r = f32::from(at(lo)[0]) * (1.0 - t) + f32::from(at(lo + 1)[0]) * t;
            *v += (r - base) / (top - base).max(1.0);
        }
        interior += top;
        floor += floor_r;
        edge_sum += edge;
        n += 1.0;
    }
    (n > 0.0).then(|| (curve.map(|v| v / n), interior / n, floor / n, edge_sum / n))
}

/// Soft particles' foot (the deferred list: "a Lighten particle's foot
/// fades a little more than the client's, 6.2 of 255 away over its last
/// rows": the fade's formula, or the reading's setup?). The three floor
/// particles of `soft_particles_in_the_client` (Normal, Punch-Through and
/// Lighten, 1.2 m, half under the floor) drawn by a copy of the game's
/// `fsparticle` that paints the fade it computes ([`fade_shader`]), in the
/// client and in Moonglow (which draws the game's own program: the same
/// formula, from its own depth). Each foot's fade ([`foot_of`]) from the
/// edge up, in both, and where it reaches a half and 0.9.
///
/// Seen (2026-11-25, 89.8193.37): Normal and Lighten feet have the same
/// shape in each (0.5 reached at 2.64 and 2.66 rows in the client, 2.10 and
/// 2.09 in Moonglow); Moonglow's ramp is about 0.55 rows earlier in both
/// (half a pixel row: its fade is less, not more). Not the formula (the
/// game's own program runs in both) and not Lighten's; the Punch-Through
/// one reads fade 0 in the client throughout.
#[test]
#[ignore]
fn batch8d_soft_foot() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("client_batch8d_soft_foot");
    let (_, are, git) = super::soft_particles::soft_module(&root, &dir, false, SETTINGS);
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    std::fs::write(dir.join("user/override/fsparticle.shd"), fade_shader(&base)).unwrap();
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let camera = reference_camera();
    let mut bands = Vec::new();
    // (The client's picture; a Lighten particle lives 0.9 s of each second,
    // so a picture without it is taken again, up to four times.)
    let mut client = None;
    for attempt in 1..=4 {
        let Some(shot) = client_screenshot(&dir, "MgScene") else {
            panic!("the client did not reach the scene (see {})", dir.display());
        };
        bands.clear();
        for (what, at, blend, _) in super::soft_particles::FLOOR_EMITTERS {
            let centre = Vec3::from(super::soft_particles::SOFT_AT) + Vec3::from(at);
            let size = super::soft_particles::FLOOR_SIZE;
            let (x0, cy) =
                project(&camera, centre - Vec3::new(size * 0.2, 0.0, 0.0), shot.width, shot.height);
            let (x1, _) =
                project(&camera, centre + Vec3::new(size * 0.2, 0.0, 0.0), shot.width, shot.height);
            bands.push((what, blend, (x0 as u32, x1 as u32, cy as u32)));
        }
        let seen = bands
            .iter()
            .all(|(_, blend, band)| foot_of(&shot, *band, *blend == "Lighten").is_some());
        eprintln!("attempt {attempt}: all three particles in the client's picture: {seen}");
        client = Some(shot);
        if seen {
            break;
        }
    }
    let client = client.unwrap();
    save_png(&client, &dir.join("client.png"));
    let Some(ours) = moonglow_particles(&game, &are, &git, (client.width, client.height), |_| {})
    else {
        return;
    };
    save_png(&ours, &dir.join("moonglow.png"));
    // Where a fade curve is over `level`, in rows from the edge (linear
    // between the distances).
    let reaches = |c: &[f32; FOOT_LENGTH], level: f32| {
        (1..FOOT_LENGTH).find(|&k| c[k] >= level).map(|k| {
            let (a, b) = (c[k - 1], c[k]);
            (k as f32 - 0.5) + (level - a) / (b - a).max(1e-3)
        })
    };
    let mut report = String::new();
    for (what, blend, band) in &bands {
        let additive = *blend == "Lighten";
        report += &format!("{what} ({blend}):\n");
        let mut curves = Vec::new();
        for (side, img) in [("client", &client), ("moonglow", &ours)] {
            let Some((curve, interior, floor, edge)) = foot_of(img, *band, additive) else {
                report += &format!("  {side:9} no particle in the picture\n");
                continue;
            };
            report += &format!(
                "  {side:9} edge row {edge:.2}, interior red {interior:.1}, floor red {floor:.1}\n    \
                 fade up from the edge: {curve:.2?}\n    reaches 0.5 at {:.2?} rows, 0.9 at {:.2?}\n",
                reaches(&curve, 0.5),
                reaches(&curve, 0.9)
            );
            curves.push(curve);
        }
        if let [c, m] = &curves[..] {
            let mean = |f: &dyn Fn(f32, f32) -> f32| {
                c.iter().zip(m).skip(1).map(|(a, b)| f(*a, *b)).sum::<f32>()
                    / (FOOT_LENGTH - 1) as f32
            };
            report += &format!(
                "  the client's fade less Moonglow's: {:+.3} on average, {:.3} in absolute value \
                 (of 1; rows 1 to {})\n",
                mean(&|a, b| a - b),
                mean(&|a, b| (a - b).abs()),
                FOOT_LENGTH - 1
            );
        }
    }
    std::fs::write(dir.join("report.txt"), &report).unwrap();
    eprintln!("{report}{}", dir.display());
}
