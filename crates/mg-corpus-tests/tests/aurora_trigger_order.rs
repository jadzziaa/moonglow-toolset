//! Triggers' outlines as Aurora saves them, against Moonglow's rules
//! (`mg_module::instances::outline_order`, `mg_module::ground`, Verify's
//! `transition-hidden`), in the oracle's `transition-triggers/` captures
//! of 2026-11-12 (`tools/aurora/capture_transition_triggers.py`): the
//! point-order probes and every other trigger Aurora painted (`04-base`,
//! `05-concave`) and the session's last module (`23-r-both-onto-linked`).
//!
//! - Aurora's saved outline, fed in any rotation and reversed, comes out
//!   of Moonglow's ordering as Aurora saved it: counterclockwise from its
//!   point of smallest X.
//! - Each point Aurora painted is 0.025 above the ground under it, by
//!   Verify's reading of the ground (`mg_module::ground`, walkable faces
//!   alone) and by the area view's (`mg_area::walk::Ground`, every face),
//!   which agree where both have ground; so Verify says nothing of
//!   Aurora's own transitions.

use mg_core::ResType;
use mg_gff::Struct;
use mg_module::Module;
use mg_module::instances::{OUTLINE_LIFT, outline_order, signed_area, trigger_xy};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

/// The captures compared.
const CAPTURES: [&str; 3] = [
    "transition-triggers/04-base.mod",
    "transition-triggers/05-concave.mod",
    "transition-triggers/23-r-both-onto-linked.mod",
];

/// Every trigger of the module's areas: the area, its tag and its struct.
fn triggers(m: &Module) -> Vec<(String, String, Struct)> {
    let mut out = Vec::new();
    for area in m.areas().unwrap() {
        let Some(Ok(git)) = m.gff(&ResKey::new(area, ResType::GIT)) else { continue };
        for t in git.root.list("TriggerList").unwrap_or(&[]) {
            let tag = String::from_utf8_lossy(t.string("Tag").unwrap_or_default()).into_owned();
            out.push((area.to_string(), tag, t.clone()));
        }
    }
    out
}

#[test]
fn aurora_s_outlines_come_out_of_moonglow_s_order_as_saved() {
    let mut compared = 0;
    for name in CAPTURES {
        let m = Module::open(&aurora_capture!(name)).unwrap();
        for (area, tag, t) in triggers(&m) {
            let saved = trigger_xy(t.list("Geometry").unwrap_or(&[]));
            let what = format!("{name}: {area} {tag}");
            assert!(saved.len() >= 3, "{what}: {saved:?}");
            assert!(signed_area(&saved) > 0.0, "{what}: counterclockwise");
            let n = saved.len();
            for turn in 0..n {
                for reversed in [false, true] {
                    // (The clicks: Aurora's points from `turn` on, or
                    // those backwards.)
                    let mut clicks: Vec<usize> = (0..n).map(|i| (i + turn) % n).collect();
                    if reversed {
                        clicks.reverse();
                    }
                    let fed: Vec<[f32; 2]> = clicks.iter().map(|&i| saved[i]).collect();
                    let back: Vec<usize> = outline_order(&fed).iter().map(|&i| clicks[i]).collect();
                    let want: Vec<usize> = (0..n).collect();
                    assert_eq!(back, want, "{what}, from point {turn}, reversed {reversed}");
                }
            }
            compared += 1;
        }
    }
    // (04 has 20 triggers, 05 21 and 23 25: the probes, the linked ones
    // and those of the other tests.)
    assert!(compared >= 60, "{compared} triggers");
}

#[test]
fn aurora_s_painted_points_are_on_the_ground_as_verify_reads_it() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let (mut points, mut off_walkable) = (0, 0);
    for name in CAPTURES {
        let m = Module::open(&aurora_capture!(name)).unwrap();
        let mut grounds = mg_module::ground::Grounds::default();
        for area in m.areas().unwrap() {
            let Some(Ok(git)) = m.gff(&ResKey::new(area, ResType::GIT)) else { continue };
            let Some(Ok(are)) = m.gff(&ResKey::new(area, ResType::ARE)) else { continue };
            let ours = grounds.of(&game.resman, &m, area).expect("the area's ground");
            let set = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).unwrap();
            let model = mg_area::AreaModel::read(&game, &are.root, &git.root, Some(&set));
            let view = mg_area::walk::Ground::new(&game, &model);
            for t in git.root.list("TriggerList").unwrap_or(&[]) {
                let tag = String::from_utf8_lossy(t.string("Tag").unwrap_or_default()).into_owned();
                let f = |s: &Struct, l: &str| s.float(l).unwrap_or(0.0);
                let at = [f(t, "XPosition"), f(t, "YPosition"), f(t, "ZPosition")];
                for (i, p) in t.list("Geometry").unwrap_or(&[]).iter().enumerate() {
                    let (x, y) = (at[0] + f(p, "PointX"), at[1] + f(p, "PointY"));
                    let z = at[2] + f(p, "PointZ");
                    let what = format!("{name}: {area} {tag} point {i} ({x}, {y}, {z})");
                    let theirs = view.height(glam::Vec2::new(x, y), z);
                    let mine = ours.height(x, y, z);
                    // (Verify takes walkable faces alone for ground; the
                    // area view, as Aurora painting, every face: beta's
                    // TT_B3 has a corner on a non-walkable one.)
                    let Some(under) = mine else {
                        assert!(theirs.is_some(), "{what}: no ground at all");
                        off_walkable += 1;
                        continue;
                    };
                    let theirs = theirs.expect("the area view's ground");
                    assert!((under - theirs).abs() < 1e-3, "{what}: {under} {theirs}");
                    assert!((z - under - OUTLINE_LIFT).abs() < 0.01, "{what}: ground {under}");
                    points += 1;
                }
            }
        }
    }
    assert!(points >= 200 && off_walkable <= points / 20, "{points} points, {off_walkable} off");
    // And Verify finds none of Aurora's transitions hidden.
    for name in CAPTURES {
        let m = Module::open(&aurora_capture!(name)).unwrap();
        let tlk = mg_module::doctor::TalkTables::default();
        let found = mg_module::doctor::examine(&m, &game.resman, tlk);
        let hidden: Vec<_> = found.iter().filter(|f| f.check == "transition-hidden").collect();
        assert!(hidden.is_empty(), "{name}: {hidden:#?}");
    }
}

/// Aurora's own triggers left off the ground (round two's captures): its
/// paste (the copy as high in the air as the ground is, `33` on gamma's
/// hill, `37` into beta), the ground raised under unmoved ones (`38`) and
/// lowered under one (`40`). Verify says those left at another level, all
/// their points more than 1 m off (`TRANSITION_OFF_GROUND`; nearer, the
/// client's answer depends on the place), and once their fixes are made,
/// as the window or `mg verify --fix transition-hidden` makes them, finds
/// none. By its nearest point to the ground:
/// - `33`: gamma's pasted copy, 4.76 m above: said.
/// - `37`: beta's pasted copy, 5.02 m above: said.
/// - `38`: beta's copy of `37`, 5.02 above: said. Gamma's three on the
///   raised hill, 0.21 and 0.55 below and 0.48 above: not said (within
///   1 m; the client was not read on them).
/// - `40`: beta's copy, 5.02 above, and gamma's on the lowered ground,
///   1.36 above: said. Gamma's three of `38`: not said, as there.
#[test]
fn aurora_s_triggers_left_off_the_ground_are_found_and_fixed() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let hidden = |m: &Module| -> Vec<mg_module::doctor::Finding> {
        let tlk = mg_module::doctor::TalkTables::default();
        let found = mg_module::doctor::examine(m, &game.resman, tlk);
        found.into_iter().filter(|f| f.check == "transition-hidden").collect()
    };
    // Before: the six painted on gamma, on the ground.
    let painted = Module::open(&aurora_capture!("transition-triggers/26-r2-painted.mod")).unwrap();
    assert!(hidden(&painted).is_empty(), "painted: {:#?}", hidden(&painted));
    // (Each capture: how many Verify says are left at another level, and
    // the area of the one its step left there.)
    for (name, count, area) in [
        ("transition-triggers/33-r2-c-paste-hill.mod", 1, "gamma"),
        ("transition-triggers/37-r2-c-paste-beta.mod", 1, "beta"),
        ("transition-triggers/38-r2-d-raise.mod", 1, "beta"),
        ("transition-triggers/40-r2-d-lower.mod", 2, "gamma"),
    ] {
        let m = Module::open(&aurora_capture!(name)).unwrap();
        let found = hidden(&m);
        assert_eq!(found.len(), count, "{name}: {found:#?}");
        assert!(
            found.iter().all(|f| f.message.contains("left at another level, which the game may"))
        );
        let here = found.iter().filter(|f| f.at.starts_with(&format!("{area} › "))).count();
        assert!(here >= 1, "{name}: one in {area}");
        // Fixed, each as Verify's window makes it: none left.
        let fixed = mg_edit::fix::attach_on(&m, Some(&game.resman), found, &[]);
        let mut guard = mg_edit::fix::Guard::new(&m, fixed.iter().filter_map(|f| f.fix.as_ref()));
        let mut ws = mg_edit::Workspace::new(m);
        for fix in fixed.iter().map(|f| f.fix.as_ref().expect("a fix")) {
            let now = guard.check(&mut ws, fix).unwrap();
            ws.apply(now.command()).unwrap();
            guard.made(&mut ws, fix);
        }
        ws.flush().unwrap();
        assert!(hidden(&ws.module).is_empty(), "{name}: fixed");
    }
}
