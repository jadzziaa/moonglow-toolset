//! Objects placed in an area, compared with Aurora's: the oracle captures
//! `placement-probe.mod` (a bandit with equipment and a potion, a
//! longsword, a chest and a store holding items, a sound and a waypoint,
//! each placed from a copy of a base-game blueprint) and
//! `placement-defaults.mod` (minimal blueprints, whose instances show
//! Aurora's defaults, and a trigger and encounters drawn as polygons) and
//! `placement-doors.mod` (a generic door and a minimal one on a castle
//! tile's door hooks, a minimal placeable; the chest there was added to
//! the palette and given a visual transform, and the waypoint made by
//! Create Waypoint: those are compared elsewhere).
//! Moonglow's instance of each blueprint, at the place Aurora put it, must
//! have the same fields, types, order and values.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct};
use mg_module::Module;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

use crate::aurora_common::{PLAIN, Shown, lines};

/// How the GFFs are compared, as lines.
const SHOWN: Shown<'static> = Shown { decimals: Some(5), ..PLAIN };

/// Where Aurora put an instance, and its outline.
fn placement(t: ResType, s: &Struct) -> (Placement, Vec<[f32; 3]>) {
    let f = |l: &str| s.float(l).unwrap_or(0.0);
    let (position, rotation) = match t {
        ResType::UTD | ResType::UTP => ([f("X"), f("Y"), f("Z")], f("Bearing")),
        _ => {
            let facing = f("YOrientation").atan2(f("XOrientation"));
            let p = [f("XPosition"), f("YPosition"), f("ZPosition")];
            (p, facing - std::f32::consts::FRAC_PI_2)
        }
    };
    let labels = if t == ResType::UTT { ["PointX", "PointY", "PointZ"] } else { ["X", "Y", "Z"] };
    let outline = s
        .list("Geometry")
        .unwrap_or(&[])
        .iter()
        .map(|p| labels.map(|l| p.float(l).unwrap_or(0.0)))
        .collect();
    (Placement { position, rotation }, outline)
}

#[test]
fn placed_objects_match_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut failures = Vec::new();
    let mut compared = 0;
    for capture in ["placement-probe.mod", "placement-defaults.mod", "placement-doors.mod"] {
        let m = Module::open(&aurora_capture!(capture)).unwrap();
        let gff = |r: ResRef, t: ResType| -> Option<Gff> {
            match m.gff(&ResKey::new(r, t)) {
                Some(g) => g.ok(),
                None => Gff::read(&game.resman.get(&ResKey::new(r, t)).ok()?).ok(),
            }
        };
        let item = |r: ResRef| gff(r, ResType::UTI).map(|g| g.root);
        let placing = Placing { game: &game, item: &item };
        let area = m.areas().unwrap()[0];
        let git = m.gff(&ResKey::new(area, ResType::GIT)).unwrap().unwrap();
        for t in [
            ResType::UTC,
            ResType::UTI,
            ResType::UTP,
            ResType::UTM,
            ResType::UTS,
            ResType::UTW,
            ResType::UTT,
            ResType::UTE,
            ResType::UTD,
        ] {
            let (list, _) = git_list(t).unwrap();
            for placed in git.root.list(list).unwrap_or(&[]) {
                let field = mg_rules::ObjectKind::resref_field_of(t);
                let r = placed.resref(field).unwrap();
                // Edited after placing (Add to Palette, Adjust Location,
                // Create Waypoint): compared elsewhere.
                if capture == "placement-doors.mod"
                    && !r.to_string().starts_with("mgq_")
                    && t != ResType::UTD
                {
                    continue;
                }
                let bp = gff(r, t).unwrap_or_else(|| panic!("{r}: no blueprint"));
                let (at, outline) = placement(t, placed);
                let ours = instance(&placing, t, &bp.root, at, &outline).unwrap();
                compared += 1;
                let (a, mut b) = (lines(&ours, SHOWN), lines(placed, SHOWN));
                // Aurora's quirks with blueprints lacking Comment (not copied).
                if r.to_string() == "mgq_utc" {
                    b.retain(|l| !l.trim_start().starts_with("Comment "));
                }
                if a != b {
                    let diff: Vec<String> = a
                        .iter()
                        .zip(&b)
                        .filter(|(x, y)| x != y)
                        .map(|(x, y)| format!("  ours   {x}\n  aurora {y}"))
                        .take(6)
                        .collect();
                    failures.push(format!(
                        "{capture} {r}.{}: {} vs {} lines\n{}",
                        t.extension().unwrap_or_default(),
                        a.len(),
                        b.len(),
                        diff.join("\n")
                    ));
                }
            }
        }
    }
    assert!(compared >= 14, "only {compared} instances compared");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
