//! Which emitters of a creature's parts the game client runs (a builder's
//! report on 4.0: "With CEP you can attach visual effects as tail to a
//! creature. These seem to be not rendered in Moonglow"): CEP's
//! `tailmodel.2da` rows 616 and up and `wingmodel.2da` rows 109 and up
//! name effect models, nearly all the game's own, of emitters and lights
//! and no mesh.
//!
//! Commoners (`nw_commale` made a human, appearance 6, head 1) north of
//! the player, west to east, 1 m apart, their backs to the camera, close
//! (`SetCameraFacing(90, 6, 75)`; one alone, 4 m off),
//! beside Moonglow's area view of the same creatures (`moonglow.png`,
//! `moonglow_on.png` in `target/test-output/client_reading_part_emitters_<group>/`;
//! `MG_CLIENT_PICTURES=target/test-output` draws Moonglow's side again
//! beside the client's pictures kept there, without the client). The
//! scratch override has a `tailmodel.2da` and a `wingmodel.2da` of the
//! rows the creatures name ([`TAILS`], [`WINGS`]).
//!
//! Groups `a` to `h`, one creature each (a dark room, so that the models'
//! lights show):
//! a. tail `wwhip_fxfire` (CEP's 619: two emitters, no animations);
//! b. tail `vdr_smoke` (753: birthrate keyed only in `duration`);
//! c. tail `vco_mehanfire01` (629: 18 emitters and 3 lights, keyed only in
//!    `conjure01` and `fade`);
//! d. tail `vpm_circfire` (668: keyed in `impact`, `duration`, `cessation`);
//! e. tail `fx_lavaspurt` (620: keyed in `default`);
//! f. wings `wwhip_fxfire`;
//! g. f wearing a cloak (`nw_maarcl055`: cloakmodel.2da hides wings);
//! h. a torch in the left hand (`nw_it_torch001`, `it_torch_000`: its
//!    emitters' birthrate 0 at rest, keyed up in its `equip` animation;
//!    the arm raised by `torchl`, as the client holds it, since 2026-11-25).
//!
//! Group `probe` (lit): our own emitter of 12 cm white squares rising 1 m/s
//! for a second ([`PROBE`], `mg_tailfx`), as
//! i. a tail; j. a tail on appearance [`SCALED`] (the human's row copied,
//!    `WING_TAIL_SCALE` 3); k. a tail with the tail's visual transform
//!    (scope 253) scaled 2 and moved 0.5 m up; l. wings;
//! m. a tail of one emitter whose birthrate is 0 at rest and 8 in its own
//!    `pause1` (`mg_tailkey`), which the commoner stands in;
//! n. a helmet (`nw_arhe001`) whose model (`helm_001`, the override's) is a
//!    small box with the probe's emitter on top.
//!
//! Group `items` (lit): helmets whose emitter is keyed in their `equip`
//! as the torch's is, `helm_002` (`nw_arhe002`) from birthrate 0 at rest
//! to 8 at its end, `helm_009` (`nw_arhe003`) from 8 to 0; and the probe as
//! a placeable, as it is and scaled 2 by its visual transform. Groups
//! `torch_up` and `torch_down`: the torch's model (`it_torch_000`) the
//! override's, a stick with the probe's emitter, keyed in its `equip`
//! from 0 to 8 and from 8 to 0.
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render part_emitters -- --ignored --nocapture --test-threads 1`
//! (`MG_GROUPS=a,probe` for some).
//!
//! Seen (2026-11-10, EE 89.8193.37):
//! - A tail's and wings' emitters run, keyed by the part's own animation
//!   of the name the creature plays (m: `pause1` keyed 8 over 0 at rest
//!   emits), else its own `default` (e: `fx_lavaspurt`, 0 at rest, spurts
//!   high), else as the model has them at rest (a, f: `wwhip_fxfire`
//!   burns; b: `vdr_smoke` smokes, its `duration` keys no part; d:
//!   `vpm_circfire` shows one flare of its four, 4 m off at rest, the
//!   others out of sight). c: `vco_mehanfire01`'s emitters are trails
//!   (spawntype 1, made as they move) and show nothing on a creature
//!   standing; its three lights light it and the floor orange.
//! - A part's scale sizes its particles, not their speed: at
//!   `WING_TAIL_SCALE` 3 (j) the squares are three times as big and rise
//!   as far as i's in their second; the tail's visual transform (k)
//!   scaled 2 doubles them, moved 0.5 m up starts them 0.5 m higher. The
//!   placeable scaled 2 (items) throws squares twice as big, rising as
//!   far: the same for any object.
//! - Wings hidden by a cloak (g) show nothing.
//! - A helmet's emitter runs as its model has it (n emits; `helm_002`,
//!   0 at rest, shows nothing; `helm_009`, 8 at rest, emits): its `equip`
//!   plays no part. A held item's stands as its `equip` ends: the game's
//!   torch (h) burns (0 at rest), `torch_up` emits and `torch_down` does
//!   not.
//! - The torch is held up, its arm raised, and lights the room (h); the
//!   creature's stance and the torch's light are not this reading's
//!   (`docs/deferred.md`).
//!
//! Moonglow drew none of the parts' particles before (only a model's base
//! was simulated); as above since, in the area view, the preview and the
//! pictures (`mg_preview::compose::Composed::emitting`).

use super::batch4::{put_creature, shoot_close, transform};
use super::readings::{INVISIBLE, Reading, print_log};
use super::*;

/// The probe's emitter, on `parent`: 12 cm white squares rising 1 m/s
/// for a second, eight a second, unless `extra` says otherwise.
fn probe_node(parent: &str, name: &str, at: [f32; 3], extra: &str) -> String {
    super::batch4::emitter(parent, name, at, &format!("birthrate 8\n  lifeExp 1\n  {extra}"))
}

/// A model of a root dummy, `nodes` and the animations `anims` (ASCII,
/// each a whole `newanim ... doneanim` block).
fn model_with(name: &str, nodes: &[String], anims: &[String]) -> String {
    format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\n\
         endnode\n{}endmodelgeom {name}\n{}donemodel {name}\n",
        nodes.concat(),
        anims.concat()
    )
}

/// The probe tail: [`probe_node`] at the root.
const PROBE: &str = "mg_tailfx";
/// The keyed probe: birthrate 0 at rest, 8 in its `pause1`.
const KEYED: &str = "mg_tailkey";

fn probe_models() -> Vec<(String, Vec<u8>)> {
    let probe = model_with(PROBE, &[probe_node(PROBE, "fx", [0.0; 3], "")], &[]);
    let keyed_anim = format!(
        "newanim pause1 {KEYED}\n  length 1\n  transtime 0.25\n  animroot {KEYED}\n\
         node dummy {KEYED}\n  parent NULL\nendnode\nnode emitter fx\n  parent {KEYED}\n\
         birthratekey 2\n    0 8\n    1 8\nendnode\ndoneanim pause1 {KEYED}\n"
    );
    let keyed =
        model_with(KEYED, &[probe_node(KEYED, "fx", [0.0; 3], "birthrate 0")], &[keyed_anim]);
    // (A helmet: a box over the head, the probe's emitter on top.)
    let helm = "helm_001";
    let mut boxed = super::part_textures::box_node(
        helm,
        "cap",
        "mgwhite",
        [-0.12, -0.12, 0.05],
        [0.12, 0.12, 0.25],
    );
    boxed += &probe_node(helm, "fx", [0.0, 0.0, 0.3], "");
    let helmet = model_with(helm, &[boxed], &[]);
    let mut files = vec![
        (format!("{PROBE}.mdl"), probe.into_bytes()),
        (format!("{KEYED}.mdl"), keyed.into_bytes()),
        (format!("{helm}.mdl"), helmet.into_bytes()),
        ("mgwhite.tga".into(), white_tga()),
    ];
    // Helmets whose emitter is keyed in their `equip` (as the torch's):
    // `helm_002` from 0 at rest to 8 at its end, `helm_009` from 8 at
    // rest to 0 at its end (`nw_arhe002` and `nw_arhe003` wear them).
    for (helm, rest, end) in [("helm_002", 0, 8), ("helm_009", 8, 0)] {
        let mut boxed = super::part_textures::box_node(
            helm,
            "cap",
            "mgwhite",
            [-0.12, -0.12, 0.05],
            [0.12, 0.12, 0.25],
        );
        boxed += &probe_node(helm, "fx", [0.0, 0.0, 0.3], &format!("birthrate {rest}"));
        let equip = format!(
            "newanim equip {helm}\n  length 1\n  transtime 0.25\n  animroot {helm}\n\
             node dummy {helm}\n  parent NULL\nendnode\nnode emitter fx\n  parent {helm}\n\
             birthratekey 2\n    0 {rest}\n    1 {end}\nendnode\ndoneanim equip {helm}\n"
        );
        files.push((format!("{helm}.mdl"), model_with(helm, &[boxed], &[equip]).into_bytes()));
    }
    files
}

/// tailmodel.2da's rows, from 1.
const TAILS: [&str; 7] =
    ["wwhip_fxfire", "vdr_smoke", "vco_mehanfire01", "vpm_circfire", "fx_lavaspurt", PROBE, KEYED];
/// wingmodel.2da's rows, from 1.
const WINGS: [&str; 2] = ["wwhip_fxfire", PROBE];

/// A table of `models` (rows from 1) as tailmodel.2da and wingmodel.2da
/// are made.
fn part_table(models: &[&str]) -> String {
    let mut t = String::from("2DA V2.0\n\n LABEL MODEL ENVMAP\n0 None **** ****\n");
    for (i, m) in models.iter().enumerate() {
        t += &format!("{} Row{} {m} ****\n", i + 1, i + 1);
    }
    t
}

/// The appearance whose `WING_TAIL_SCALE` is 3: the human's row (6)
/// copied to the end of the game's appearance.2da (the table and its
/// row returned).
fn scaled_appearance(game: &GameData) -> (String, u32) {
    let table = game.resman.get_named("appearance", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).into_owned();
    let header: Vec<String> =
        table.lines().nth(2).unwrap().split_whitespace().map(str::to_owned).collect();
    let column = header.iter().position(|c| c == "WING_TAIL_SCALE").unwrap() + 1;
    let rows = table.lines().skip(3).filter(|l| !l.trim().is_empty()).count();
    let human = table
        .lines()
        .skip(3)
        .find(|l| l.split_whitespace().next() == Some("6"))
        .unwrap()
        .to_string();
    // (Labels with spaces are quoted: the human's has none.)
    let mut row: Vec<String> = human.split_whitespace().map(str::to_owned).collect();
    row[0] = rows.to_string();
    row[column] = "3".into();
    if !table.ends_with('\n') {
        table.push('\n');
    }
    table += &(row.join(" ") + "\n");
    (table, rows as u32)
}

/// What a creature of a group is.
struct Creature {
    tail: u32,
    wings: u32,
    appearance: Option<u32>,
    equip: &'static [(u32, &'static str)],
    scope: bool,
}

const fn tail(n: u32) -> Creature {
    Creature { tail: n, wings: 0, appearance: None, equip: &[], scope: false }
}

const fn wings(n: u32) -> Creature {
    Creature { tail: 0, wings: n, appearance: None, equip: &[], scope: false }
}

/// The group with the probe placeables beside its creatures.
const ITEMS: &str = "items";

/// The scaled appearance's stand-in in [`Creature::appearance`].
const SCALED: u32 = u32::MAX;

/// Each group: its name, its lighting and its creatures, west to east.
/// The game's models one creature at a time (a to h), close; the probes
/// together.
fn groups() -> Vec<(&'static str, Lighting, Vec<Creature>)> {
    let dark = Lighting { ambient: 0x202020, diffuse: 0x303030, main_light: 0, only_tile: None };
    let lit = Lighting { ambient: 0x606060, diffuse: 0x909090, main_light: 0, only_tile: None };
    let game = [
        tail(1),
        tail(2),
        tail(3),
        tail(4),
        tail(5),
        wings(1),
        Creature { equip: &[(0x40, "nw_maarcl055")], ..wings(1) },
        Creature { equip: &[(0x20, "nw_it_torch001")], ..tail(0) },
    ];
    let mut out: Vec<_> = ["a", "b", "c", "d", "e", "f", "g", "h"]
        .into_iter()
        .zip(game)
        .map(|(g, c)| (g, dark, vec![c]))
        .collect();
    out.push((
        "probe",
        lit,
        vec![
            tail(6),
            Creature { appearance: Some(SCALED), ..tail(6) },
            Creature { scope: true, ..tail(6) },
            wings(2),
            tail(7),
            Creature { equip: &[(0x1, "nw_arhe001")], ..tail(0) },
        ],
    ));
    // (And the probe as a placeable, as it is and scaled 2: [`ITEMS`].)
    out.push((
        ITEMS,
        lit,
        vec![
            Creature { equip: &[(0x1, "nw_arhe002")], ..tail(0) },
            Creature { equip: &[(0x1, "nw_arhe003")], ..tail(0) },
        ],
    ));
    // (A torch whose model is the override's: [`torch_model`].)
    for group in [TORCH_UP, TORCH_DOWN] {
        out.push((group, lit, vec![Creature { equip: &[(0x20, "nw_it_torch001")], ..tail(0) }]));
    }
    out
}

/// The groups whose torch's model (`it_torch_000`) is the override's: a
/// stick with the probe's emitter on top, keyed in its `equip` as the
/// game's torch is, from 0 at rest to 8 at its end ([`TORCH_UP`]) or from
/// 8 to 0 ([`TORCH_DOWN`]).
const TORCH_UP: &str = "torch_up";
const TORCH_DOWN: &str = "torch_down";

fn torch_model(group: &str) -> Option<(String, Vec<u8>)> {
    let (rest, end) = match group {
        TORCH_UP => (0, 8),
        TORCH_DOWN => (8, 0),
        _ => return None,
    };
    let m = "it_torch_000";
    let mut nodes = super::part_textures::box_node(
        m,
        "stick",
        "mgwhite",
        [-0.02, -0.02, -0.1],
        [0.02, 0.02, 0.4],
    );
    nodes += &probe_node(m, "fx", [0.0, 0.0, 0.45], &format!("birthrate {rest}"));
    let equip = format!(
        "newanim equip {m}\n  length 0.1\n  transtime 0.25\n  animroot {m}\n\
         node dummy {m}\n  parent NULL\nendnode\nnode emitter fx\n  parent {m}\n\
         birthratekey 2\n    0 {rest}\n    0.1 {end}\nendnode\ndoneanim equip {m}\n"
    );
    Some((format!("{m}.mdl"), model_with(m, &[nodes], &[equip]).into_bytes()))
}

/// What was seen: the module's header.
#[test]
#[ignore]
fn part_emitters_look() {
    let _ = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let only = std::env::var("MG_GROUPS").ok();
    let (appearance, scaled) = scaled_appearance(&base);
    let (placeables, probe) = super::readings::placeables_with(&base, &[(PROBE, 0)]);
    for (group, light, creatures) in groups() {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|g| g == group)) {
            continue;
        }
        let mut files = probe_models();
        files.push(("tailmodel.2da".into(), part_table(&TAILS).into_bytes()));
        files.push(("wingmodel.2da".into(), part_table(&WINGS).into_bytes()));
        files.push(("appearance.2da".into(), appearance.clone().into_bytes()));
        files.push(("placeables.2da".into(), placeables.clone().into_bytes()));
        files.extend(torch_model(group));
        let refs: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
        let mut r = Reading::new(&format!("part_emitters_{group}"), "tic01", light, &refs);
        let k = creatures.len() as f32;
        for (i, c) in creatures.iter().enumerate() {
            let x = 20.0 + (i as f32 - (k - 1.0) / 2.0);
            let appearance = match c.appearance {
                Some(SCALED) => scaled,
                Some(a) => a,
                None => 6,
            };
            let mut fields = vec![
                ("Appearance_Type", Value::Word(appearance as u16)),
                ("Appearance_Head", Value::Byte(1)),
                ("Tail_New", Value::Dword(c.tail)),
                ("Wings_New", Value::Dword(c.wings)),
            ];
            if c.scope {
                let e = transform(253, 2.0, [0.0; 3], [0.0, 0.0, 0.5]);
                fields.push(("VisTransformList", Value::List(vec![e])));
            }
            put_creature(&mut r, "nw_commale", [x, 21.6, 0.0], 0.0, &fields, c.equip);
        }
        if group == ITEMS {
            r.place(probe, [21.5, 21.6, 0.5], 0.0, &[]);
            let e = super::batch2::transform_entry(0, 2.0, 0.0);
            r.place(probe, [22.5, 21.6, 0.5], 0.0, &[("VisTransformList", Value::List(vec![e]))]);
        }
        let distance = if creatures.len() == 1 { 4.0 } else { 6.0 };
        let (dir, _) = shoot_close(r, INVISIBLE, SETTINGS, (90.0, distance, 75.0), &|v| {
            // (A second in: `fx_lavaspurt`'s `default` spurts then, at
            // its start it throws nothing.)
            v.animate = true;
            v.time = 1.0;
        });
        print_log(&dir);
    }
}

/// The torch's light as the client draws it (`docs/findings.md`, "A
/// torch-bearer's raised arm, and an item's light on its wearer"): in a dark room a commoner holding
/// `nw_it_torch001` (facing the camera) and boxes of 0.5 m around him
/// ([`LIGHT_BOXES`]: flat tops of one color, which the light scales) from
/// high above (`MG_LIGHT_CAMERA`, `facing,distance,pitch`: pitch 10 is
/// nearly straight down). Three pictures, each `light_<run>.png` in
/// `target/test-output/part_emitters/`: `locate` (the room lit well, no
/// torch, the boxes magenta: where each is), `torch` and `bare` (the same
/// dark room with the torch and without, the boxes white), each with
/// Moonglow's area view beside it (`light_<run>_moonglow.png` and
/// `light_<run>_moonglow_on.png`, [`shoot_close`]). The boxes' tops'
/// brightness in `torch` against `bare` is the torch's light at their
/// distance; `MG_RUNS=torch,bare` for some. Two more runs tell where the
/// light is put: `effect` (nothing held: `VFX_DUR_LIGHT_YELLOW_20`, the
/// visual effect the torch's Light property applies, put on him by the
/// entry script; Moonglow's side has no light for it), and
/// `effect_turned`, `torch_turned` and `bare_turned` as `effect`,
/// `torch` and `bare` with him turned a quarter (facing 90);
/// `effect_tall` and `effect_wide` as `effect` with him made a copy of
/// the human's appearance.2da row but for `HEIGHT` 4 (the human's 2),
/// and but for `PERSPACE` 1 and `CREPERSPACE` 1.5 (0.3 and 0.5); and
/// `effect_badger` and `effect_ogre` with him made a badger (appearance 8)
/// and an ogre (127). And what else lights: `cloak` (no torch: his
/// cloak `mg_litcloak`, `nw_maarcl055` given the torch's Light
/// property), `ring` (a ring, `mg_litring`, `nw_it_mring001` given it,
/// in his left ring slot, which is not drawn), `torch_ogre` (the torch
/// held by him made an ogre) and `green` (`mg_greentorch`, the torch
/// given a green and dim Light property in place of its own). The
/// pictures are measured by hand.
///
/// Seen (2026-11-25, EE 89.8193.37): the light is the creature's
/// effect, alike for the torch, the cloak, the ring and the effect, at
/// its model's `headconjure` plus (0, 0.75, 0.6) in the model's frame;
/// it turns with him and follows the badger's and ogre's heads, and the
/// appearance's sizes change nothing. Moonglow's side matches the
/// client's within 0.03 in the log of the light added (root mean square
/// over the boxes), the green torch's color too.
#[test]
#[ignore]
fn part_emitters_torch_light() {
    let _ = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let (placeables, boxed) = super::readings::placeables_with(&base, &[("mg_lbox", 0)]);
    let kept = mg_testkit::test_output().join("part_emitters");
    std::fs::create_dir_all(&kept).unwrap();
    let only = std::env::var("MG_RUNS").ok();
    let camera = std::env::var("MG_LIGHT_CAMERA").unwrap_or_else(|_| "90,30,10".into());
    let c: Vec<f32> = camera.split(',').map(|v| v.parse().unwrap()).collect();
    let dark = Lighting { ambient: 0x202020, diffuse: 0x303030, main_light: 0, only_tile: None };
    let lit = Lighting { ambient: 0x606060, diffuse: 0x909090, main_light: 0, only_tile: None };
    // (The human's row copied, a column or two changed: rows `tall` and
    // `wide`.)
    let table = base.resman.get_named("appearance", ResType::TWODA).unwrap();
    let mut appearance = String::from_utf8_lossy(&table).into_owned();
    let header: Vec<String> =
        appearance.lines().nth(2).unwrap().split_whitespace().map(str::to_owned).collect();
    let column = |name: &str| header.iter().position(|c| c == name).unwrap() + 1;
    let human = appearance
        .lines()
        .skip(3)
        .find(|l| l.split_whitespace().next() == Some("6"))
        .unwrap()
        .to_string();
    let rows = appearance.lines().skip(3).filter(|l| !l.trim().is_empty()).count();
    if !appearance.ends_with('\n') {
        appearance.push('\n');
    }
    let (tall, wide) = (rows, rows + 1);
    for (n, changes) in
        [(tall, &[("HEIGHT", "4")][..]), (wide, &[("PERSPACE", "1"), ("CREPERSPACE", "1.5")][..])]
    {
        let mut row: Vec<String> = human.split_whitespace().map(str::to_owned).collect();
        row[0] = n.to_string();
        for (c, v) in changes {
            row[column(c)] = (*v).into();
        }
        appearance += &(row.join(" ") + "\n");
    }
    // Items given a Light property (`brightness`, `color`) in place of
    // their own properties, under a new name.
    let lit_item = |from: &str, name: &str, brightness: u16, color: u8| {
        let data = base.resman.get_named(from, ResType::UTI).unwrap();
        let mut uti = Gff::read(&data).unwrap();
        let light = mg_preview::ItemLook {
            light: Some(mg_preview::LightProperty { brightness, color }),
            ..Default::default()
        };
        let properties = light.to_uti(0).list("PropertiesList").unwrap().to_vec();
        uti.root.set("PropertiesList", Value::List(properties));
        uti.root.set("TemplateResRef", Value::resref(ResRef::from_str(name).unwrap()));
        (format!("{name}.uti"), uti.to_bytes().unwrap())
    };
    let items = [
        lit_item("nw_maarcl055", "mg_litcloak", 4, 1),
        lit_item("nw_it_mring001", "mg_litring", 4, 1),
        lit_item("nw_it_torch001", "mg_greentorch", 1, 4),
    ];
    let runs = [
        ("locate", lit, &[][..]),
        ("torch", dark, &[(0x20, "nw_it_torch001")][..]),
        ("bare", dark, &[][..]),
        ("effect", dark, &[][..]),
        ("effect_turned", dark, &[][..]),
        ("torch_turned", dark, &[(0x20, "nw_it_torch001")][..]),
        ("effect_tall", dark, &[][..]),
        ("effect_wide", dark, &[][..]),
        ("effect_badger", dark, &[][..]),
        ("effect_ogre", dark, &[][..]),
        ("bare_turned", dark, &[][..]),
        ("cloak", dark, &[(0x40, "mg_litcloak")][..]),
        ("ring", dark, &[(0x80, "mg_litring")][..]),
        ("torch_ogre", dark, &[(0x20, "nw_it_torch001")][..]),
        ("green", dark, &[(0x20, "mg_greentorch")][..]),
    ];
    for (run, light, equip) in runs {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|g| g == run)) {
            continue;
        }
        let color: [u8; 3] = if run == "locate" { [255, 0, 255] } else { [255, 255, 255] };
        let mut tga = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 24, 0];
        tga.extend([color[2], color[1], color[0]].repeat(64));
        let slab = super::part_textures::box_node(
            "mg_lbox",
            "top",
            "mg_lboxtex",
            [-0.25, -0.25, 0.0],
            [0.25, 0.25, 0.3],
        );
        let mut files: Vec<(&str, Vec<u8>)> = vec![
            ("mg_lboxtex.tga", tga),
            ("mg_lbox.mdl", model_with("mg_lbox", &[slab], &[]).into_bytes()),
            ("placeables.2da", placeables.clone().into_bytes()),
            ("appearance.2da", appearance.clone().into_bytes()),
        ];
        files.extend(items.iter().map(|(n, d)| (n.as_str(), d.clone())));
        let mut r = Reading::new(&format!("part_emitters_light_{run}"), "tic01", light, &files);
        let row = match run {
            "effect_tall" => tall,
            "effect_wide" => wide,
            "effect_badger" => 8,
            "effect_ogre" | "torch_ogre" => 127,
            _ => 6,
        };
        let fields =
            [("Appearance_Head", Value::Byte(1)), ("Appearance_Type", Value::Word(row as u16))];
        let facing = if run.ends_with("_turned") { 90.0 } else { 180.0 };
        put_creature(&mut r, "nw_commale", [LIGHT_AT[0], LIGHT_AT[1], 0.0], facing, &fields, equip);
        for (dx, dy) in LIGHT_BOXES {
            r.place(boxed, [LIGHT_AT[0] + dx, LIGHT_AT[1] + dy, 0.0], 0.0, &[]);
        }
        // (160: `VFX_DUR_LIGHT_YELLOW_20`.)
        let effect = "ApplyEffectToObject(DURATION_TYPE_PERMANENT, EffectVisualEffect(160), \
                      GetObjectByTag(\"NW_COMMALE\"));";
        let body = if run.starts_with("effect") {
            format!("{INVISIBLE}\n    {effect}")
        } else {
            INVISIBLE.into()
        };
        let (dir, shot) = shoot_close(r, &body, SETTINGS, (c[0], c[1], c[2]), &|_| {});
        print_log(&dir);
        save_png(&shot, &kept.join(format!("light_{run}.png")));
        for side in ["moonglow", "moonglow_on"] {
            let _ = std::fs::copy(
                dir.join(format!("{side}.png")),
                kept.join(format!("light_{run}_{side}.png")),
            );
        }
    }
    eprintln!("pictures: {}", kept.display());
}

/// Where the torch-bearer of [`part_emitters_torch_light`] stands.
const LIGHT_AT: [f32; 2] = [20.0, 22.2];

/// The boxes' offsets from him (east and north, meters): east and west
/// along his row at their distances, and north and south of him.
const LIGHT_BOXES: [(f32, f32); 26] = [
    (1.0, 0.0),
    (2.0, 0.0),
    (3.0, 0.0),
    (4.0, 0.0),
    (5.0, 0.0),
    (6.0, 0.0),
    (8.0, 0.0),
    (-1.5, 0.0),
    (-2.5, 0.0),
    (-3.5, 0.0),
    (-4.5, 0.0),
    (-5.5, 0.0),
    (-7.0, 0.0),
    (0.0, 2.0),
    (0.0, 4.0),
    (0.0, 6.0),
    (0.0, 8.0),
    (0.0, 10.0),
    (0.0, 12.0),
    (0.0, -2.0),
    (0.0, -4.0),
    (0.0, -6.0),
    (0.0, -8.0),
    (0.0, -10.0),
    (0.0, -12.0),
    (0.0, -14.0),
];
