//! The area viewer's commands compared with Aurora's: Add to Palette, and
//! Create Waypoint then Create Set.
//!
//! Add to Palette (the oracle capture
//! `placement-doors.mod`: a chest holding a potion and a longsword, placed
//! from `mgp_utp`, added to the palette): the new chest blueprint
//! (`mgp_utp001`), the item blueprints it now holds (`it_mpotion025`,
//! `wswls003`: `nw_` dropped, the first free numbers among the game's), and
//! the placed chest naming them, field for field.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::instances::{Placement, Placing, instance};
use mg_module::palette_add::add_to_palette;
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

fn lines(s: &Struct, indent: &str, out: &mut Vec<String>) {
    for f in &s.fields {
        match &f.value {
            Value::List(items) => {
                out.push(format!("{indent}{} list[{}]", f.label.to_string_lossy(), items.len()));
                for (i, item) in items.iter().enumerate() {
                    out.push(format!("{indent}  [{i}] struct {}", item.id));
                    lines(item, &format!("{indent}    "), out);
                }
            }
            Value::Float(v) => {
                let v = if v.abs() < 1e-6 { 0.0 } else { *v };
                out.push(format!("{indent}{} Float({v:.5})", f.label.to_string_lossy()))
            }
            v => out.push(format!("{indent}{} {:?}", f.label.to_string_lossy(), v)),
        }
    }
}

fn differences(what: &str, ours: &Struct, aurora: &Struct) -> Option<String> {
    let (mut a, mut b) = (Vec::new(), Vec::new());
    lines(ours, "", &mut a);
    lines(aurora, "", &mut b);
    (a != b).then(|| {
        let diff: Vec<String> = a
            .iter()
            .zip(&b)
            .filter(|(x, y)| x != y)
            .map(|(x, y)| format!("  ours   {x}\n  aurora {y}"))
            .take(6)
            .collect();
        format!("{what}: {} vs {} lines\n{}", a.len(), b.len(), diff.join("\n"))
    })
}

#[test]
fn add_to_palette_matches_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = Module::open(&aurora_capture!("placement-doors.mod")).unwrap();
    let made = ["mgp_utp001.utp", "it_mpotion025.uti", "wswls003.uti"];
    let made_keys: Vec<ResKey> = made
        .iter()
        .map(|n| {
            let (name, ext) = n.split_once('.').unwrap();
            ResKey::new(ResRef::from_str(name).unwrap(), ResType::from_extension(ext).unwrap())
        })
        .collect();
    let read = |k: ResKey| -> Option<Struct> {
        match m.gff(&k) {
            Some(g) => g.ok().map(|g| g.root),
            None => Gff::read(&game.resman.get(&k).ok()?).ok().map(|g| g.root),
        }
    };
    // Taken then: the game's and the module's resources, but what it made.
    let taken =
        |k: &ResKey| !made_keys.contains(k) && (m.contains(k) || game.resman.get(k).is_ok());
    let item = |r: ResRef| read(ResKey::new(r, ResType::UTI));
    let placing = Placing { game: &game, item: &item };
    let git = m.gff(&ResKey::new(m.areas().unwrap()[0], ResType::GIT)).unwrap().unwrap();
    let chest = git
        .root
        .list("Placeable List")
        .unwrap()
        .iter()
        .find(|p| p.resref("TemplateResRef").unwrap().to_string() == "mgp_utp001")
        .unwrap()
        .clone();
    // The chest as placed before (its bearing and visual transform came
    // after, from Adjust Location).
    let f = |l: &str| chest.float(l).unwrap();
    let at = Placement { position: [f("X"), f("Y"), f("Z")], rotation: 0.0 };
    let bp = read(ResKey::new(ResRef::from_str("mgp_utp").unwrap(), ResType::UTP)).unwrap();
    let placed = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
    let added = add_to_palette(&game, ResType::UTP, &placed, &read, &taken).unwrap();
    let names: Vec<String> = added.blueprints.iter().map(|(k, _)| k.to_string()).collect();
    assert_eq!(names, made);
    let mut failures = Vec::new();
    for ((key, ours), name) in added.blueprints.iter().zip(made) {
        let aurora = read(*key).unwrap_or_else(|| panic!("{name} not in the capture"));
        failures.extend(differences(name, &ours.root, &aurora));
    }
    // The chest names its new blueprint, and its items theirs.
    let mut now = chest.clone();
    now.set("Bearing", Value::Float(0.0));
    now.remove("VisTransformList");
    failures.extend(differences("the placed chest", &added.instance, &now));
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Create Waypoint on the bandit (where the context menu was opened), then
/// Create Set named "Patrol": a waypoint from no blueprint, facing north,
/// tagged `WP_MGP_UTC_01` and then `Patrol_01`.
#[test]
fn create_waypoint_and_set_match_aurora() {
    let _ = corpus!();
    let m = Module::open(&aurora_capture!("placement-doors.mod")).unwrap();
    let git = m.gff(&ResKey::new(m.areas().unwrap()[0], ResType::GIT)).unwrap().unwrap();
    let captured = git.root.list("WaypointList").unwrap()[0].clone();
    let f = |l: &str| captured.float(l).unwrap();
    let position = [f("XPosition"), f("YPosition"), f("ZPosition")];
    let creature_tag = mg_module::instances::set_tag("WP_MGP_UTC", &[]);
    assert_eq!(creature_tag, "WP_MGP_UTC_01");
    let made = mg_module::instances::walk_waypoint(&creature_tag, position);
    // Create Set renames it.
    let mut set = made.clone();
    set.set("Tag", Value::String(mg_module::instances::set_tag("Patrol", &[]).into_bytes()));
    assert!(
        differences("the waypoint", &set, &captured).is_none(),
        "{:?}",
        differences("the waypoint", &set, &captured)
    );
    // Numbers continue past those in use.
    let tags = vec!["Patrol_01".to_string(), "PATROL_02".to_string()];
    assert_eq!(mg_module::instances::set_tag("Patrol", &tags), "Patrol_03");
}

/// Adjust Location on the chest in Aurora (bearing 45, scale 2, Z rotation
/// 30, Z translation 0.5): the same bearing and visual transform. (Aurora
/// rewrites the whole object, so its `VisTransformList` comes before
/// `ItemList`; an edit adds it at the end. The engine does not mind.)
#[test]
fn adjust_location_matches_aurora() {
    use glam::Vec3;
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = Module::open(&aurora_capture!("placement-doors.mod")).unwrap();
    let git_key = ResKey::new(m.areas().unwrap()[0], ResType::GIT);
    let git = m.gff(&git_key).unwrap().unwrap();
    let list = git.root.list("Placeable List").unwrap();
    let index = list
        .iter()
        .position(|p| p.resref("TemplateResRef").unwrap().to_string() == "mgp_utp001")
        .unwrap();
    let aurora = list[index].clone();
    let mut before = aurora.clone();
    before.set("Bearing", Value::Float(0.0));
    before.remove("VisTransformList");
    let o = mg_area::AreaObject::read(&game, mg_area::ObjectKind::Placeable, index, &before);
    let visual = mg_area::VisualTransform {
        scale: Vec3::splat(2.0),
        rotate: Vec3::new(0.0, 0.0, 30.0),
        translate: Vec3::new(0.0, 0.0, 0.5),
    };
    let mut edits = mg_area::edit::move_edits(git_key, &o, &before, o.position, 45f32.to_radians());
    edits.extend(mg_area::edit::visual_transform_edits(git_key, &o, &before, visual));
    let mut after = before.clone();
    for e in edits {
        let mg_edit::Edit::SetField { label, value, .. } = e else { panic!("{e:?}") };
        match value {
            Some(v) => after.set(&label, v),
            None => {
                after.remove(&label);
            }
        }
    }
    assert!((after.float("Bearing").unwrap() - aurora.float("Bearing").unwrap()).abs() < 1e-6);
    assert_eq!(after.get("VisTransformList"), aurora.get("VisTransformList"));
    assert_eq!(mg_area::VisualTransform::read(&after), Some(visual));
}
