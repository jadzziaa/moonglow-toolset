//! Setup Area Transition with triggers, compared with Aurora's
//! (`tools/aurora/capture_transition_triggers.py`, the oracle's
//! `transition-triggers/` captures of 2026-11-12): the module Aurora
//! saved before and after each linking step. Moonglow's plan
//! (`mg_edit::transition`) is applied to the "before" and every trigger,
//! door and waypoint of both areas is compared with Aurora's "after",
//! field by field, in order, with each field's type.
//!
//! Where Moonglow and Aurora differ, and the comparison steps over it:
//! - Aurora's first waypoint of a run has `XOrientation` 1.5414283e-43,
//!   the bits of the integer 110 (garbage in its struct; the second has
//!   0.0): Moonglow writes 0.0, and floats are compared to 4 decimals
//!   with anything under 1e-6 as 0.
//! - Aurora adds a `.gic` comment entry for each object; Moonglow adds
//!   none for placed objects (its convention everywhere), so the GICs
//!   aren't compared.
//! - Aurora's Target To Current loses the target's half (it is written
//!   only when that area is saved later for another reason): in `09` the
//!   target is as before, and Aurora's `11` has it, flushed by the next
//!   step. Moonglow writes it at once: compared with `11`'s, and skipped
//!   where a step's "before" lacks it. The same for doors (`45` flushed
//!   in `46`, `47` in `48`).
//! - One waypoint Aurora made off the rule (`17`'s Z 5.38972 against the
//!   points' middle 5.1621, not explained) is not among these steps.

use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_edit::transition::{Connection, Current, Placed, plan};
use mg_gff::Struct;
use mg_module::Module;
use mg_resman::ResKey;
use mg_rules::ObjectKind;
use mg_testkit::aurora_capture;

use crate::aurora_common::{PLAIN, Shown, lines};

/// Floats to 4 decimals, Aurora's denormal noise as 0.
const SHOWN: Shown<'static> = Shown { decimals: Some(4), ..PLAIN };

/// The kinds of object compared, in every area.
const KINDS: [ObjectKind; 3] = [ObjectKind::Trigger, ObjectKind::Door, ObjectKind::Waypoint];

/// A capture, or `None` (said; a failure with `MOONGLOW_REQUIRE_CORPUS=1`).
fn capture(name: &str) -> Option<Module> {
    let name = format!("transition-triggers/{name}.mod");
    let Some(path) = mg_testkit::aurora_capture(&name) else {
        assert!(!mg_testkit::corpus_required(), "Aurora capture {name} not found");
        eprintln!("skipped: Aurora capture {name} not found (tools/aurora/)");
        return None;
    };
    Some(Module::open(&path).unwrap())
}

fn tag(s: &Struct) -> String {
    String::from_utf8_lossy(s.string("Tag").unwrap_or_default()).into_owned()
}

fn list(m: &mut Workspace, area: ResRef, kind: ObjectKind) -> Vec<Struct> {
    let git = ResKey::new(area, ResType::GIT);
    m.doc(&git).unwrap().root.list(kind.list()).unwrap_or(&[]).to_vec()
}

/// An object of the captures: by its kind and tag where the tag is its
/// alone, else (gamma's triggers, all `AreaTransition`) by its kind, area
/// and where it stands (`XPosition`, `YPosition`, within 0.1 m).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Who<'a> {
    Tag(ObjectKind, &'a str),
    At(ObjectKind, &'a str, [f32; 2]),
}

impl Who<'_> {
    fn kind(&self) -> ObjectKind {
        match self {
            Who::Tag(k, _) | Who::At(k, ..) => *k,
        }
    }

    /// Whether `s`, of `kind` in `area`, is it.
    fn is(&self, area: ResRef, kind: ObjectKind, s: &Struct) -> bool {
        match *self {
            Who::Tag(k, t) => k == kind && tag(s) == t,
            Who::At(k, a, [x, y]) => {
                let near = |l, v: f32| s.float(l).is_some_and(|f| (f - v).abs() < 0.1);
                k == kind && area.to_string() == a && near("XPosition", x) && near("YPosition", y)
            }
        }
    }
}

/// The object `who` in the module's areas (one).
fn find(ws: &mut Workspace, who: Who<'_>) -> Placed {
    let kind = who.kind();
    let mut found = Vec::new();
    for area in ws.module.areas().unwrap() {
        for (i, s) in list(ws, area, kind).iter().enumerate() {
            if who.is(area, kind, s) {
                found.push(Placed::new(area, kind, i));
            }
        }
    }
    assert_eq!(found.len(), 1, "one {who:?}: {found:?}");
    found[0]
}

/// The struct of `who` in the module.
fn get(ws: &mut Workspace, who: Who<'_>) -> Struct {
    let p = find(ws, who);
    list(ws, p.area, p.kind)[p.index].clone()
}

/// One step: `before` with Moonglow's plan from `from` to `to` against
/// Aurora's `after`; `skip`: objects not compared, with the reason in the
/// caller's comment.
fn step(
    before: &str,
    after: &str,
    from: Who<'_>,
    to: Who<'_>,
    connection: Connection,
    skip: &[Who<'_>],
) -> Option<Workspace> {
    let mut ws = Workspace::new(capture(before)?);
    let mut aurora = Workspace::new(capture(after)?);
    let current = find(&mut ws, from);
    let target = find(&mut ws, to);
    let p = plan(&mut ws, &Current::Placed(current), target, connection)
        .unwrap_or_else(|e| panic!("{before}: {e}"));
    ws.apply(p.command).unwrap();
    let mut failures = Vec::new();
    for area in aurora.module.areas().unwrap() {
        for kind in KINDS {
            let (ours, theirs) = (list(&mut ws, area, kind), list(&mut aurora, area, kind));
            let shown = |l: &[Struct]| -> Vec<String> { l.iter().map(tag).collect() };
            if shown(&ours) != shown(&theirs) {
                failures.push(format!(
                    "{after} {area} {kind:?}: {:?} against Aurora's {:?}",
                    shown(&ours),
                    shown(&theirs)
                ));
                continue;
            }
            for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
                if skip.iter().any(|w| w.is(area, kind, a)) {
                    continue;
                }
                let (la, lb) = (lines(a, SHOWN), lines(b, SHOWN));
                if la != lb {
                    let diff: Vec<String> = (la.iter().zip(&lb))
                        .filter(|(x, y)| x != y)
                        .map(|(x, y)| format!("    ours {x}\n  aurora {y}"))
                        .collect();
                    failures.push(format!(
                        "{after} {area} {kind:?}[{i}] {} ({} fields against {}):\n{}",
                        tag(a),
                        la.len(),
                        lb.len(),
                        diff.join("\n")
                    ));
                }
                assert_eq!(a.id, b.id, "struct id");
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Some(ws)
}

/// Both Ways, trigger to trigger in two areas, both unlinked (the
/// builder's report): TT_A1 in alpha and TT_B2 in beta.
#[test]
fn both_ways_trigger_to_trigger_in_two_areas() {
    aurora_capture!("transition-triggers/07-s2-both.mod");
    let tt = ObjectKind::Trigger;
    step(
        "06-typechange",
        "07-s2-both",
        Who::Tag(tt, "TT_A1"),
        Who::Tag(tt, "TT_B2"),
        Connection::BothWays,
        &[],
    );
}

/// Current To Target: TT_A2 to TT_B3, TT_B3 unchanged.
#[test]
fn current_to_target_trigger_to_trigger() {
    aurora_capture!("transition-triggers/08-s3a-cur.mod");
    let tt = ObjectKind::Trigger;
    step(
        "07-s2-both",
        "08-s3a-cur",
        Who::Tag(tt, "TT_A2"),
        Who::Tag(tt, "TT_B3"),
        Connection::CurrentToTarget,
        &[],
    );
}

/// Target To Current: TT_B4 to TT_A3, the waypoint beside TT_A3. Aurora's
/// `09` lost TT_B4's half (skipped there); its `11` has it, flushed by
/// the next step that saved beta, and Moonglow's TT_B4 is that one.
#[test]
fn target_to_current_trigger_to_trigger() {
    aurora_capture!("transition-triggers/09-s3b-tgt.mod");
    let tt = ObjectKind::Trigger;
    let b4 = Who::Tag(tt, "TT_B4");
    let skip = [b4];
    let Some(mut ws) = step(
        "08-s3a-cur",
        "09-s3b-tgt",
        Who::Tag(tt, "TT_A3"),
        b4,
        Connection::TargetToCurrent,
        &skip,
    ) else {
        return;
    };
    let Some(flushed) = capture("11-s5a-door-to-trig") else { return };
    let mut flushed = Workspace::new(flushed);
    let (ours, theirs) = (get(&mut ws, b4), get(&mut flushed, b4));
    assert_eq!(lines(&ours, SHOWN), lines(&theirs, SHOWN), "TT_B4 as Aurora wrote it later");
    assert_eq!(ours.string("LinkedTo"), Some(b"WP_TT_A3".as_slice()));
}

/// Both Ways in one area: TT_A4 and TT_A5, the two waypoints one after
/// the other, the target's first.
#[test]
fn both_ways_in_one_area() {
    aurora_capture!("transition-triggers/10-s4-same.mod");
    let tt = ObjectKind::Trigger;
    step(
        "09-s3b-tgt",
        "10-s4-same",
        Who::Tag(tt, "TT_A4"),
        Who::Tag(tt, "TT_A5"),
        Connection::BothWays,
        &[],
    );
}

/// Door to trigger, Both Ways: TT_AD1 to TT_B5. Aurora's `11` also has
/// TT_B4's half of step 3, flushed by this save of beta (skipped).
#[test]
fn both_ways_door_to_trigger() {
    aurora_capture!("transition-triggers/11-s5a-door-to-trig.mod");
    let (tt, dd) = (ObjectKind::Trigger, ObjectKind::Door);
    let skip = [Who::Tag(tt, "TT_B4")];
    step(
        "10-s4-same",
        "11-s5a-door-to-trig",
        Who::Tag(dd, "TT_AD1"),
        Who::Tag(tt, "TT_B5"),
        Connection::BothWays,
        &skip,
    );
}

/// Trigger to door, Both Ways: TT_A6 to TT_BD1.
#[test]
fn both_ways_trigger_to_door() {
    aurora_capture!("transition-triggers/12-s5b-trig-to-door.mod");
    let (tt, dd) = (ObjectKind::Trigger, ObjectKind::Door);
    step(
        "11-s5a-door-to-trig",
        "12-s5b-trig-to-door",
        Who::Tag(tt, "TT_A6"),
        Who::Tag(dd, "TT_BD1"),
        Connection::BothWays,
        &[],
    );
}

/// Trigger to waypoint: TT_A7 to beta's NW_WAYPOINT001 (Current To Target,
/// the only one Aurora offers), nothing made.
#[test]
fn current_to_target_trigger_to_waypoint() {
    aurora_capture!("transition-triggers/13-s6-trig-to-wp.mod");
    let (tt, wp) = (ObjectKind::Trigger, ObjectKind::Waypoint);
    step(
        "12-s5b-trig-to-door",
        "13-s6-trig-to-wp",
        Who::Tag(tt, "TT_A7"),
        Who::Tag(wp, "NW_WAYPOINT001"),
        Connection::CurrentToTarget,
        &[],
    );
}

// The door cases with the one-way connections, measured in Aurora's
// second round (notes section 14, captures 44 to 48): the doors are
// alpha's and beta's (tool-written, opened in Aurora), the triggers
// gamma's, all tagged `AreaTransition` (told apart by where they stand).

/// gamma's triggers, by where they stand in `42` to `48` (the one painted
/// at (6, 30) was moved by `42`'s Shift+drag).
const AT_12_12: Who<'static> = Who::At(ObjectKind::Trigger, "gamma", [11.99, 11.96]);
const AT_12_6: Who<'static> = Who::At(ObjectKind::Trigger, "gamma", [11.99, 5.98]);
const AT_6_12: Who<'static> = Who::At(ObjectKind::Trigger, "gamma", [6.0, 11.96]);
const AT_6_30: Who<'static> = Who::At(ObjectKind::Trigger, "gamma", [8.41, 29.97]);

/// Door to trigger, Current To Target (D1): TT_AD2 to gamma's (12, 12)
/// trigger; the door names its new waypoint, made beside it, the trigger
/// unchanged (measured, `42` to `44`).
#[test]
fn current_to_target_door_to_trigger() {
    aurora_capture!("transition-triggers/44-r2-14-d1-door-trig-cur.mod");
    let dd = ObjectKind::Door;
    step(
        "42-r2-f-shiftdrag",
        "44-r2-14-d1-door-trig-cur",
        Who::Tag(dd, "TT_AD2"),
        AT_12_12,
        Connection::CurrentToTarget,
        &[],
    );
}

/// Door to trigger, Target To Current (D2): gamma's (12, 6) trigger
/// leads to door TT_BD2 (1), no waypoint, the door unchanged (measured,
/// `44` to `45`). Aurora's `45` lost the trigger's half (skipped there);
/// `46` has it, flushed by the next save of gamma.
#[test]
fn target_to_current_door_to_trigger() {
    aurora_capture!("transition-triggers/45-r2-14-d2-door-trig-tgt.mod");
    let dd = ObjectKind::Door;
    let Some(mut ws) = step(
        "44-r2-14-d1-door-trig-cur",
        "45-r2-14-d2-door-trig-tgt",
        Who::Tag(dd, "TT_BD2"),
        AT_12_6,
        Connection::TargetToCurrent,
        &[AT_12_6],
    ) else {
        return;
    };
    let Some(flushed) = capture("46-r2-14-t1-trig-door-cur") else { return };
    let mut flushed = Workspace::new(flushed);
    let (ours, theirs) = (get(&mut ws, AT_12_6), get(&mut flushed, AT_12_6));
    assert_eq!(lines(&ours, SHOWN), lines(&theirs, SHOWN), "as Aurora wrote it later");
    assert_eq!(ours.string("LinkedTo"), Some(b"TT_BD2".as_slice()));
}

/// Trigger to door, Current To Target (T1): gamma's (6, 12) trigger to
/// TT_AD1 (1), the door unchanged (it keeps WP_TT_B5), no waypoint
/// (measured, `45` to `46`). Aurora's `46` also has D2's half on the
/// (12, 6) trigger, flushed by this save of gamma (skipped).
#[test]
fn current_to_target_trigger_to_door() {
    aurora_capture!("transition-triggers/46-r2-14-t1-trig-door-cur.mod");
    let dd = ObjectKind::Door;
    step(
        "45-r2-14-d2-door-trig-tgt",
        "46-r2-14-t1-trig-door-cur",
        AT_6_12,
        Who::Tag(dd, "TT_AD1"),
        Connection::CurrentToTarget,
        &[AT_12_6],
    );
}

/// Trigger to door, Target To Current (T2): door TT_BD1 (linked before,
/// its link replaced) leads to the waypoint made beside gamma's (6, 30)
/// trigger, the trigger unchanged (measured, `46` to `47`). Aurora's `47`
/// lost the door's half (skipped there); `48` has it, flushed when a
/// later link saved beta.
#[test]
fn target_to_current_trigger_to_door() {
    aurora_capture!("transition-triggers/47-r2-14-t2-trig-door-tgt.mod");
    let dd = ObjectKind::Door;
    let bd1 = Who::Tag(dd, "TT_BD1");
    let Some(mut ws) = step(
        "46-r2-14-t1-trig-door-cur",
        "47-r2-14-t2-trig-door-tgt",
        AT_6_30,
        bd1,
        Connection::TargetToCurrent,
        &[bd1],
    ) else {
        return;
    };
    let Some(flushed) = capture("48-r2-14-flush") else { return };
    let mut flushed = Workspace::new(flushed);
    let (ours, theirs) = (get(&mut ws, bd1), get(&mut flushed, bd1));
    assert_eq!(lines(&ours, SHOWN), lines(&theirs, SHOWN), "as Aurora wrote it later");
    assert_eq!(ours.string("LinkedTo"), Some(b"WP_AreaTransition".as_slice()));
}
