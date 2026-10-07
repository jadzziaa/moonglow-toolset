//! Blueprint previews from the base game: their structure, and each drawn
//! offscreen (`target/test-output/previews/` to look at).

use std::sync::Arc;

use glam::Mat4;
use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_mdl::Model;
use mg_preview::compose::Composed;
use mg_preview::{CreatureLook, Preview, creature, creature_look, door, item, placeable};
use mg_render::{AreaLight, Camera, Gpu, Renderer, Scene};
use mg_resman::GameInstall;
use mg_rules::GameData;
use mg_rules::items::{ArmorChannel, set_armor_part_color};

fn game() -> Option<GameData> {
    let root = mg_testkit::nwn_root()?;
    GameData::open(&GameInstall::new(&root, None, "en")).ok()
}

fn blueprint(game: &GameData, name: &str, t: ResType) -> Gff {
    Gff::read(&game.resman.get_named(name, t).unwrap_or_else(|_| panic!("{name}"))).unwrap()
}

fn items(game: &GameData) -> impl Fn(ResRef) -> Option<Gff> + '_ {
    move |r| {
        let data = game.resman.get_named(r.as_str()?, ResType::UTI).ok()?;
        Gff::read(&data).ok()
    }
}

fn models(p: &Preview) -> Vec<&str> {
    std::iter::once(p.base.model.as_str()).chain(p.parts.iter().map(|x| x.model.as_str())).collect()
}

#[test]
fn creatures_by_appearance() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    // A human woman: her skeleton, a bare body (every part but the belt and
    // shoulders, the right foot included) and the head asked for.
    let look = CreatureLook { gender: 1, head: 3, colors: [5, 6, 7, 8], ..CreatureLook::new(6) };
    let p = creature_look(&game, &look).unwrap();
    let names = models(&p);
    eprintln!("appearance 6, female: {names:?}");
    assert_eq!(p.base.model, "pfh0");
    for part in ["footr", "footl", "chest", "pelvis", "handr", "neck"] {
        assert!(names.contains(&format!("pfh0_{part}001").as_str()), "{part}");
    }
    assert!(!names.iter().any(|n| n.contains("belt") || n.contains("sho")));
    let head = p.parts.iter().find(|x| x.model == "pfh0_head003").unwrap();
    assert_eq!(head.attach.as_deref(), Some("head_g"));
    assert_eq!(head.colors.unwrap()[..2], [5, 6]);
    // Wings when asked.
    let winged = CreatureLook { wings: 1, ..CreatureLook::new(0) };
    let p = creature_look(&game, &winged).unwrap();
    assert!(p.parts.iter().any(|x| x.attach.as_deref() == Some("wings")));
    // A single-model creature: its model alone.
    let badger = creature_look(&game, &CreatureLook::new(8)).unwrap();
    assert_eq!(models(&badger), ["c_badger"]);
    assert!(creature_look(&game, &CreatureLook::new(60_000)).is_err());
}

#[test]
fn blueprints_assemble() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    // A gnome in a robed armour with a cloak and a crossbow.
    let utc = blueprint(&game, "nw_hen_bod_05", ResType::UTC);
    let p = creature(&game, &utc.root, &items(&game)).unwrap();
    eprintln!("nw_hen_bod_05: {:?}", models(&p));
    assert_eq!(p.base.model, "pmg0");
    assert_eq!(p.idle.as_deref(), Some("pause1"));
    // Robe 4 hides the chest, legs and pelvis; feet, hands and head stay.
    assert!(p.parts.iter().any(|x| x.model == "pmg0_robe004" && x.animated));
    assert!(!p.parts.iter().any(|x| x.model.starts_with("pmg0_chest")));
    assert!(p.parts.iter().any(|x| x.model.starts_with("pmg0_head")));
    assert!(p.parts.iter().any(|x| x.model.starts_with("pmg0_cloak_") && x.animated));
    assert!(p.parts.iter().any(|x| x.attach.as_deref() == Some("rhand")));
    // Its own skin colour on the body parts.
    let head = p.parts.iter().find(|x| x.model.starts_with("pmg0_head")).unwrap();
    let skin = utc.root.integer("Color_Skin").unwrap() as u8;
    assert_eq!(head.colors.unwrap()[0], skin);

    // Items: a longsword's three parts, a helmet with colours, armour worn.
    let sword = item(&game, &blueprint(&game, "nw_wswls001", ResType::UTI).root).unwrap();
    assert_eq!(models(&sword), ["wswls_b_061", "wswls_m_011", "wswls_t_011"]);
    let helm = item(&game, &blueprint(&game, "nw_arhe001", ResType::UTI).root).unwrap();
    assert_eq!(helm.base.model, "helm_001");
    assert!(helm.base.colors.is_some());
    let armor = item(&game, &blueprint(&game, "nw_aarcl001", ResType::UTI).root).unwrap();
    assert_eq!(armor.base.model, "pmh0");
    assert!(armor.parts.iter().any(|x| x.model == "pmh0_chest016"));
    // A part's own colors (EE) are on that part alone: the torso's cloth 1
    // (PLT layer 4) and the right foot's metal 2 (layer 3).
    let mut uti = blueprint(&game, "nw_aarcl001", ResType::UTI).root;
    let cloth = uti.integer("Cloth1Color").unwrap() as u8;
    set_armor_part_color(&mut uti, 7, ArmorChannel::Cloth1, Some(33));
    set_armor_part_color(&mut uti, 0, ArmorChannel::Metal2, Some(171));
    let armor = item(&game, &uti).unwrap();
    let colors = |model: &str| {
        armor.parts.iter().find(|x| x.model.starts_with(model)).unwrap().colors.unwrap()
    };
    assert_eq!(colors("pmh0_chest")[4], 33);
    assert_eq!(colors("pmh0_footr")[3], 171);
    assert_eq!(colors("pmh0_footl")[3], uti.integer("Metal2Color").unwrap() as u8);
    assert_eq!(colors("pmh0_pelvis")[4], cloth);

    // A placeable and doors.
    let armoire = placeable(&game, &blueprint(&game, "plc_armoire", ResType::UTP).root).unwrap();
    assert_eq!(armoire.base.model, "plc_a01");
    assert_eq!(armoire.idle.as_deref(), Some("default"));
    // A door blueprint is generic (Appearance 0: genericdoors row 0); placed
    // in a tileset's door slot it takes a doortypes row.
    let mut utd = blueprint(&game, "nw_door_ttr_01", ResType::UTD).root;
    assert_eq!(door(&game, &utd).unwrap().base.model, "t_door01");
    utd.set("Appearance", mg_gff::Value::Dword(1));
    let door_types = game.table("doortypes").unwrap();
    let d = door(&game, &utd).unwrap();
    assert_eq!(d.base.model, door_types.get(1, "Model").unwrap().to_ascii_lowercase());
}

/// A part whose model names a texture that doesn't exist takes the texture
/// of its own name, as the game does: `pfh0_belt063` names `beltmerged`.
#[test]
fn a_part_naming_a_missing_texture_takes_its_own() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut look = CreatureLook::new(6);
    look.gender = 1;
    let mut utc = look.to_utc();
    utc.set("BodyPart_Belt", mg_gff::Value::Byte(63));
    let p = creature(&game, &utc, &|_| None).unwrap();
    let belt = p.parts.iter().find(|x| x.model == "pfh0_belt063").expect("the belt");
    assert_eq!(belt.textures.get("beltmerged").map(String::as_str), Some("pfh0_belt063"));
    // No part of the base game's human bodies is left without a texture
    // that one of its own name (or its fallbacks') would give it.
    let has = |name: &str| {
        [ResType::PLT, ResType::DDS, ResType::TGA]
            .iter()
            .any(|&t| game.resman.get_named(name, t).is_ok())
    };
    for part in &p.parts {
        for (old, new) in &part.textures {
            assert!(!has(old) && has(new), "{}: {old} -> {new}", part.model);
        }
    }
}

/// A part hangs from a node of the base model: where the base has no such
/// node (a phenotype with another body, as the Vault's bariaur, which has
/// no pelvis or legs), the part isn't drawn, rather than drawn at the feet.
#[test]
fn a_part_whose_node_the_base_lacks_is_not_drawn() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let rm = &game.resman;
    let load = |name: &str| -> Option<Arc<Model>> {
        let data = rm.get_named(name, ResType::MDL).ok()?;
        Model::read(&data).ok().map(Arc::new)
    };
    let mut p = creature(&game, &CreatureLook::new(6).to_utc(), &|_| None).unwrap();
    let whole = Composed::new(&gpu, &p, &load).unwrap().part_count();
    let pelvis = p.parts.iter_mut().find(|x| x.model.contains("pelvis")).expect("a pelvis");
    pelvis.attach = Some("no_such_node".into());
    let c = Composed::new(&gpu, &p, &load).unwrap();
    assert_eq!(c.part_count(), whole - 1, "the pelvis is left out");
    assert!(c.missing.is_empty(), "and isn't a missing model: {:?}", c.missing);
}

#[test]
fn previews_render() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let rm = &game.resman;
    let load = |name: &str| -> Option<Arc<Model>> {
        let data = rm.get_named(name, ResType::MDL).ok()?;
        Model::read(&data).ok().map(Arc::new)
    };
    let dir = mg_testkit::scratch_dir("previews");
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let item_of = items(&game);
    // Creatures and bodies face +Y: seen from the front; held items from
    // the side.
    // An armor whose torso has colors of its own (EE).
    let mut parts = blueprint(&game, "nw_aarcl001", ResType::UTI).root;
    for (channel, color) in [
        (ArmorChannel::Cloth1, 88),
        (ArmorChannel::Cloth2, 88),
        (ArmorChannel::Leather1, 46),
        (ArmorChannel::Leather2, 46),
    ] {
        set_armor_part_color(&mut parts, 7, channel, Some(color));
    }
    let mut drawings = std::collections::HashMap::new();
    let cases: Vec<(&str, Preview)> = vec![
        (
            "nw_hen_bod_05",
            creature(&game, &blueprint(&game, "nw_hen_bod_05", ResType::UTC).root, &item_of)
                .unwrap(),
        ),
        (
            "nw_halfdra002",
            creature(&game, &blueprint(&game, "nw_halfdra002", ResType::UTC).root, &item_of)
                .unwrap(),
        ),
        ("nw_aarcl001", item(&game, &blueprint(&game, "nw_aarcl001", ResType::UTI).root).unwrap()),
        ("part_colors", item(&game, &parts).unwrap()),
        ("nw_wswls001", item(&game, &blueprint(&game, "nw_wswls001", ResType::UTI).root).unwrap()),
        ("nw_arhe001", item(&game, &blueprint(&game, "nw_arhe001", ResType::UTI).root).unwrap()),
        (
            "plc_armoire",
            placeable(&game, &blueprint(&game, "plc_armoire", ResType::UTP).root).unwrap(),
        ),
    ];
    for (name, preview) in cases {
        let c = Composed::new(&gpu, &preview, &load).expect(name);
        assert!(c.missing.is_empty(), "{name}: missing {:?}", c.missing);
        let (min, max) = c.bounds();
        let centre = (min + max) * 0.5;
        let radius = ((max - min).length() * 0.5).max(0.1);
        let yaw = if preview.base.model.starts_with('p') { 90f32 } else { 20.0 };
        let camera = Camera::orbit(
            centre,
            radius / 20f32.to_radians().sin() * 1.1,
            yaw.to_radians(),
            15f32.to_radians(),
        );
        let scene = Scene {
            instances: c.instances(c.idle.as_deref(), 0.5, Mat4::IDENTITY),
            lights: c.point_lights(Mat4::IDENTITY),
            area: AreaLight::default(),
            background: [0.2, 0.25, 0.3],
            ..Default::default()
        };
        let img = r.render_image(&gpu, rm, &scene, &camera, 256, 256);
        let file = std::fs::File::create(dir.join(format!("{name}.png"))).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header().unwrap().write_image_data(&img.data).unwrap();
        let bg = [51u8, 64, 77];
        let drawn = img
            .data
            .chunks(4)
            .filter(|p| p[..3].iter().zip(bg).any(|(a, b)| a.abs_diff(b) > 3))
            .count();
        eprintln!("{name}: {} instances, {drawn} pixels drawn", scene.instances.len());
        assert!(drawn > 2000, "{name}: little drawn");
        drawings.insert(name, img.data);
    }
    // The torso's own colors show: its pixels differ, the rest are the same.
    let (plain, own) = (&drawings["nw_aarcl001"], &drawings["part_colors"]);
    let differ = plain.chunks(4).zip(own.chunks(4)).filter(|(a, b)| a != b).count();
    eprintln!("part colors: {differ} pixels differ");
    assert!(differ > 300 && differ < 256 * 256 / 8, "{differ} pixels differ");
}

/// Under armor, a body part shows the creature's own where the armor's
/// is bare skin (part 1), and the armor's where it covers it (GitHub issue
/// 5: a jackalwere's legs and paws were a human's under its armor).
#[test]
fn a_creature_s_own_parts_show_where_its_armor_is_bare() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut utc = CreatureLook::new(6).to_utc();
    utc.set("BodyPart_RBicep", mg_gff::Value::Byte(2));
    let mut worn = mg_gff::Struct::new(2);
    worn.set("EquippedRes", mg_gff::Value::resref("worn".parse().unwrap()));
    utc.set("Equip_ItemList", mg_gff::Value::List(vec![worn]));
    let armor = |_: ResRef| -> Option<Gff> {
        let mut g = Gff::new(*b"UTI ");
        g.root.set("BaseItem", mg_gff::Value::Int(16));
        for (field, _) in mg_rules::items::ARMOR_PARTS {
            let n = if field == "ArmorPart_Torso" { 5 } else { 1 };
            g.root.set(field, mg_gff::Value::Byte(n));
        }
        g.root.set("ArmorPart_Robe", mg_gff::Value::Byte(0));
        Some(g)
    };
    let preview = creature(&game, &utc, &armor).unwrap();
    let models = models(&preview);
    assert!(models.contains(&"pmh0_bicepr002"), "the creature's own arm: {models:?}");
    assert!(models.contains(&"pmh0_bicepl001"), "{models:?}");
    assert!(models.contains(&"pmh0_chest005"), "the armor's chest: {models:?}");
}

/// A cloak moves with the bones of the body wearing it: on an elf (whose
/// skeleton plays the human animations at 0.894 of their size, while her
/// cloak's model has no scale of its own) its bones are where hers are,
/// not at a human's height, over her face.
#[test]
fn a_cloak_sits_on_a_smaller_body_s_shoulders() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let rm = &game.resman;
    let load = |name: &str| -> Option<Arc<Model>> {
        Model::read(&rm.get_named(name, ResType::MDL).ok()?).ok().map(Arc::new)
    };
    // An elf woman in the Great Cloak (cloakmodel.2da row 3).
    let mut look = CreatureLook::new(1);
    look.gender = 1;
    let mut utc = look.to_utc();
    let mut worn = mg_gff::Struct::new(0x40);
    worn.set("ModelPart1", mg_gff::Value::Byte(3));
    utc.set("Equip_ItemList", mg_gff::Value::List(vec![worn]));
    let preview = creature(&game, &utc, &|_| None).unwrap();
    let c = Composed::new(&gpu, &preview, &load).unwrap();
    let drawn = c.instances(c.idle.as_deref(), 0.5, Mat4::IDENTITY);
    let named = |name: &str| drawn.iter().find(|i| i.model.model.name.eq_ignore_ascii_case(name));
    let (body, cloak) = (named("pfe0").unwrap(), named("pfe0_cloak_002").expect("the cloak"));
    let at = |i: &mg_render::scene::Instance, node: &str| {
        let n = i.model.model.nodes.iter().position(|n| n.name.eq_ignore_ascii_case(node))?;
        Some(i.pose.as_ref()?[n].w_axis.truncate())
    };
    for node in ["rootdummy", "torso_g", "neck_g"] {
        let (Some(hers), Some(its)) = (at(body, node), at(cloak, node)) else { continue };
        assert!(hers.distance(its) < 0.01, "{node}: the body's at {hers}, the cloak's at {its}");
    }
    assert!(at(cloak, "rootdummy").is_some(), "the cloak has the body's bones");
}

/// A look at creatures of custom content (GitHub issue 5): `MG_PROBE_USER`
/// a user directory with the haks, `MG_PROBE_HAKS` their names (top
/// first, comma-separated), `MG_PROBE_UTC` the blueprints' files.
#[test]
#[ignore = "a look at custom content"]
fn probe_custom_creatures() {
    let (Some(root), Ok(user)) = (mg_testkit::nwn_root(), std::env::var("MG_PROBE_USER")) else {
        return;
    };
    let install = GameInstall::new(&root, Some(std::path::PathBuf::from(user)), "en");
    let mut game = GameData::open(&install).unwrap();
    let haks = std::env::var("MG_PROBE_HAKS").unwrap_or_default();
    let haks: Vec<&str> = haks.split(',').filter(|h| !h.is_empty()).collect();
    game.resman.add_haks(&install, &haks).unwrap();
    mg_testkit::gpu::hold();
    let gpu = Gpu::headless().unwrap();
    let dir = mg_testkit::scratch_dir("probe-creatures");
    let files = std::env::var("MG_PROBE_UTC").unwrap_or_default();
    let folder =
        std::path::Path::new(files.split(',').next().unwrap()).parent().unwrap().to_owned();
    let rm = &game.resman;
    let load = |name: &str| -> Option<Arc<Model>> {
        Model::read(&rm.get_named(name, ResType::MDL).ok()?).ok().map(Arc::new)
    };
    let item_of = |r: ResRef| -> Option<Gff> {
        let local = folder.join(format!("{r}.uti"));
        let data = std::fs::read(local)
            .ok()
            .or_else(|| rm.get_named(r.as_str()?, ResType::UTI).ok().map(|d| d.into_owned()))?;
        Gff::read(&data).ok()
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    for file in files.split(',') {
        // (A number: an appearance row, as a creature of it.)
        let preview = match file.parse::<u16>() {
            Ok(row) => {
                // (`MG_PROBE_GENDER`, and `MG_PROBE_CLOAK` a cloakmodel.2da row worn.)
                let env = |n: &str| std::env::var(n).ok().and_then(|v| v.parse::<u8>().ok());
                let mut look = CreatureLook::new(row);
                look.gender = env("MG_PROBE_GENDER").unwrap_or(0);
                let mut utc = look.to_utc();
                if let Some(cloak) = env("MG_PROBE_CLOAK") {
                    let mut worn = mg_gff::Struct::new(0x40);
                    worn.set("ModelPart1", mg_gff::Value::Byte(cloak));
                    utc.set("Equip_ItemList", mg_gff::Value::List(vec![worn]));
                }
                creature(&game, &utc, &item_of).unwrap()
            }
            Err(_) => {
                let utc = Gff::read(&std::fs::read(file).unwrap()).unwrap();
                creature(&game, &utc.root, &item_of).unwrap()
            }
        };
        eprintln!("  idle {:?}", preview.idle);
        eprintln!("{file}: {:?}", models(&preview));
        let c = Composed::new(&gpu, &preview, &load).unwrap();
        eprintln!("  missing {:?}", c.missing);
        let (min, max) = c.bounds();
        let centre = (min + max) * 0.5;
        let radius = ((max - min).length() * 0.5).max(0.1);
        let camera = Camera::orbit(
            centre,
            radius / 20f32.to_radians().sin() * 1.1,
            90f32.to_radians(),
            15f32.to_radians(),
        );
        let scene = Scene {
            instances: c.instances(c.idle.as_deref(), 0.5, Mat4::IDENTITY),
            lights: c.point_lights(Mat4::IDENTITY),
            area: AreaLight::default(),
            background: [0.2, 0.25, 0.3],
            ..Default::default()
        };
        let img = r.render_image(&gpu, rm, &scene, &camera, 384, 384);
        let name = std::path::Path::new(file).file_stem().unwrap().to_string_lossy().into_owned();
        let out = std::fs::File::create(dir.join(format!("{name}.png"))).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(out), img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&img.data).unwrap();
    }
}

/// Armour and a cloak lying in an area are a bag, as the game drops what
/// is only ever worn (a builder's area showed a man standing where a
/// tunic lay, and a cloak hanging from nothing); a sword lies there as
/// itself.
#[test]
fn what_is_only_worn_lies_in_an_area_as_aurora_shows_it() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    let placed = |name: &str| {
        let uti = blueprint(&game, name, ResType::UTI);
        mg_preview::item_placed(&game, &uti.root).unwrap()
    };
    // Armor: the game's model of armor dropped, by its weight (cloth,
    // leather, chain, plate), no body.
    let dropped =
        |name: &str| models(&placed(name)).into_iter().map(str::to_string).collect::<Vec<_>>();
    assert_eq!(dropped("nw_cloth001"), ["gi_armor01"]);
    assert_eq!(dropped("nw_aarcl001"), ["gi_armor04"], "leather");
    assert_eq!(dropped("nw_aarcl004"), ["gi_armor03"], "chain");
    assert_eq!(dropped("nw_aarcl007"), ["gi_armor02"], "full plate");
    // A cloak: the bag, standing.
    let cloak = placed("nw_aarcl013");
    assert_eq!(models(&cloak), ["it_bag"]);
    // As it is worn: a body's parts.
    let worn = item(&game, &blueprint(&game, "nw_aarcl001", ResType::UTI).root).unwrap();
    assert!(models(&worn).len() > 5);
    // A sword is its own model, there too.
    let sword = blueprint(&game, "nw_wswls001", ResType::UTI);
    let lying = mg_preview::item_placed(&game, &sword.root).unwrap();
    assert_eq!(models(&lying), models(&item(&game, &sword.root).unwrap()));
    // How each is turned where it lies: baseitems.2da's RotateOnGround.
    let turn =
        |name: &str| mg_preview::ground_turn(&game, &blueprint(&game, name, ResType::UTI).root);
    assert_eq!(turn("nw_wswls001"), 1, "a sword on its flat");
    assert_eq!(turn("nw_it_mpotion001"), 2, "a potion stood up");
    assert_eq!(turn("nw_aarcl001"), 0, "armor as its model is");
    assert_eq!(turn("nw_aarcl013"), 0, "the bag");
}
