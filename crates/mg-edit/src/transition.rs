//! Setup Area Transition's links (Aurora's `TdlgAreaTransition`), as one
//! [`Command`]: what links the current door or trigger and the one it
//! leads to write, on either end or both, and the waypoints a trigger is
//! given to be arrived at. Measured in Aurora on 2026-11-12 (the
//! `transition-triggers` captures):
//!
//! - whatever links to a trigger links to that trigger's waypoint,
//!   `WP_` + the trigger's tag (`LinkedToFlags` 2), made in the trigger's
//!   area at the center of the bounding box of its outline;
//! - whatever links to a door links to the door's tag (1), and a door is
//!   given no waypoint;
//! - a link to a waypoint names the waypoint's tag (2), from the current
//!   end alone;
//! - nothing else of either object changes.
//!
//! Where Moonglow departs from Aurora (each in the manual's Differences
//! from Aurora): a waypoint of that tag already standing inside the
//! trigger's box is used again, where Aurora makes another on every run;
//! Target To Current writes the target's half, which Aurora loses; and
//! the whole is one step, the current's half with it.

use mg_core::{ResRef, ResType};
use mg_gff::{FieldType, Struct, Value};
use mg_resman::ResKey;
use mg_rules::ObjectKind;

use crate::{Command, Edit, EditError, GffPath, Workspace};

/// A placed object: its area, its kind and its place in its GIT list.
/// What a transition is set up between (two triggers of one tag are two).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Placed {
    pub area: ResRef,
    pub kind: ObjectKind,
    pub index: usize,
}

impl Placed {
    pub fn new(area: ResRef, kind: ObjectKind, index: usize) -> Placed {
        Placed { area, kind, index }
    }

    /// Its area's GIT.
    pub fn git(&self) -> ResKey {
        ResKey::new(self.area, ResType::GIT)
    }

    /// Where it is in its GIT.
    pub fn path(&self) -> GffPath {
        GffPath::root().item(self.kind.list(), self.index)
    }

    /// The placed object a GIT's path names (its first step: `TriggerList[2]`).
    pub fn of(key: &ResKey, path: &GffPath) -> Option<Placed> {
        if key.restype != ResType::GIT || path.0.len() != 1 {
            return None;
        }
        let (list, index) = path.first()?.item()?;
        Some(Placed { area: key.resref, kind: ObjectKind::from_list(list)?, index })
    }
}

/// Aurora's Connection Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Connection {
    /// Each end leads to the other (Aurora's default).
    #[default]
    BothWays,
    /// The current object leads to the target; the target is unchanged.
    CurrentToTarget,
    /// The target leads to the current object; the current is unchanged.
    TargetToCurrent,
}

impl Connection {
    pub const ALL: [Connection; 3] =
        [Connection::BothWays, Connection::CurrentToTarget, Connection::TargetToCurrent];

    /// Its name, as Aurora's radio button has it.
    pub fn label(self) -> &'static str {
        match self {
            Connection::BothWays => "Both Ways",
            Connection::CurrentToTarget => "Current To Target",
            Connection::TargetToCurrent => "Target To Current",
        }
    }

    /// Whether the current object is given a link.
    pub fn links_current(self) -> bool {
        self != Connection::TargetToCurrent
    }

    /// Whether the target is given a link.
    pub fn links_target(self) -> bool {
        self != Connection::CurrentToTarget
    }
}

/// The object the transition is set up from.
#[derive(Debug, Clone, PartialEq)]
pub enum Current {
    /// One placed door or trigger.
    Placed(Placed),
    /// A blueprint, or several objects edited together (each its document
    /// and path): no one placed object for the target to lead back to, so
    /// Current To Target alone. Each is given the link.
    Fields(Vec<(ResKey, GffPath)>),
}

/// Why a connection can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// A waypoint is led to, and leads nowhere: Current To Target alone.
    WaypointTarget,
    /// A blueprint, or several objects: no one placed object to lead back to.
    NotOnePlaced,
    /// The current object has no tag for the target to name.
    NoCurrentTag,
    /// The target has no tag to name.
    NoTargetTag,
    /// The target is the current object itself.
    Itself,
    /// No such object (the list changed since it was picked).
    Missing,
}

/// Whether `connection` from `current` (with tag `current_tag`) to an
/// object of kind `to` can be made, else why not.
pub fn refusal(
    current: &Current,
    current_tag: &[u8],
    to: ObjectKind,
    connection: Connection,
) -> Option<Refused> {
    if connection == Connection::CurrentToTarget {
        return None;
    }
    if to == ObjectKind::Waypoint {
        return Some(Refused::WaypointTarget);
    }
    if !matches!(current, Current::Placed(_)) {
        return Some(Refused::NotOnePlaced);
    }
    current_tag.is_empty().then_some(Refused::NoCurrentTag)
}

/// The tag of a trigger's waypoint: `WP_` and the trigger's tag, as it is.
pub fn waypoint_tag(trigger_tag: &[u8]) -> Vec<u8> {
    [b"WP_".as_slice(), trigger_tag].concat()
}

/// What a link to `object` (of kind `kind`) names: its tag for a door or
/// a waypoint, its waypoint's for a trigger; and `LinkedToFlags`.
pub fn link_to(kind: ObjectKind, object: &Struct) -> (Vec<u8>, i64) {
    let tag = object.string("Tag").unwrap_or_default();
    match kind {
        ObjectKind::Door => (tag.to_vec(), 1),
        ObjectKind::Trigger => (waypoint_tag(tag), 2),
        _ => (tag.to_vec(), 2),
    }
}

/// A trigger's outline as absolute points (its position plus each point).
fn outline(trigger: &Struct) -> Vec<[f32; 3]> {
    let at = |l| trigger.float(l).unwrap_or(0.0);
    let (x, y, z) = (at("XPosition"), at("YPosition"), at("ZPosition"));
    let points = trigger.list("Geometry").unwrap_or(&[]);
    points
        .iter()
        .map(|p| {
            let c = |l| p.float(l).unwrap_or(0.0);
            [x + c("PointX"), y + c("PointY"), z + c("PointZ")]
        })
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .collect()
}

/// The bounding box of a trigger's outline, absolute: its least and
/// greatest corners. A trigger without points: its position.
pub fn bounds(trigger: &Struct) -> ([f32; 3], [f32; 3]) {
    let points = outline(trigger);
    if points.is_empty() {
        let at = |l| trigger.float(l).filter(|v| v.is_finite()).unwrap_or(0.0);
        let p = [at("XPosition"), at("YPosition"), at("ZPosition")];
        return (p, p);
    }
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in &points {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

/// Where Aurora puts a trigger's waypoint: the center of the bounding box
/// of its outline in X and Y (all of the 19 it made, measured), and the
/// middle of its points' heights (18 of them; one, of a run that went
/// otherwise alike, higher, not explained). The height is the trigger's
/// stored `PointZ`, not the ground's: on ground raised under a trigger
/// after it was drawn, Aurora's waypoint stands at the old height (the
/// `transition-triggers` capture `44`: Z 5.025 where the ground is 5.56
/// to 6.36).
pub fn waypoint_position(trigger: &Struct) -> [f32; 3] {
    let (lo, hi) = bounds(trigger);
    [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0]
}

/// How near a waypoint may stand outside a trigger's box and still be
/// taken as inside, meters (the box's own floats, rounded).
const INSIDE: f32 = 1e-3;

/// Whether `p` stands inside the trigger's bounding box, seen from above.
pub fn inside(trigger: &Struct, p: [f32; 2]) -> bool {
    let (lo, hi) = bounds(trigger);
    (0..2).all(|i| p[i] >= lo[i] - INSIDE && p[i] <= hi[i] + INSIDE)
}

/// The waypoint of `waypoints` (an area's `WaypointList`) a trigger's
/// link may use again: tagged as its waypoint (`WP_` and its tag, byte
/// for byte: the game's tags are) and standing inside its box. Aurora
/// makes another each time; one stands there from an earlier run.
pub fn reusable(trigger: &Struct, waypoints: &[Struct]) -> Option<usize> {
    let tag = waypoint_tag(trigger.string("Tag").unwrap_or_default());
    waypoints.iter().position(|w| {
        let at = |l| w.float(l).unwrap_or(f32::NAN);
        w.string("Tag") == Some(tag.as_slice())
            && inside(trigger, [at("XPosition"), at("YPosition")])
    })
}

/// Aurora's waypoint for a trigger (`WP_` and its tag, at
/// [`waypoint_position`], facing north): its struct exactly, as
/// [`mg_module::instances::walk_waypoint`] makes it.
pub fn waypoint_for(trigger: &Struct) -> Struct {
    let tag = waypoint_tag(trigger.string("Tag").unwrap_or_default());
    mg_module::instances::walk_waypoint(&tag, waypoint_position(trigger))
}

/// A link a command replaces: the object, what it led to and what it is to.
#[derive(Debug, Clone, PartialEq)]
pub struct Replaced {
    /// The object (none: one of several edited together, or a blueprint).
    pub object: Option<Placed>,
    pub tag: Vec<u8>,
    pub was: Vec<u8>,
    pub now: Vec<u8>,
}

/// What OK does, worked out before it is pressed: the command, and what
/// the window says of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub command: Command,
    /// The link the current object is given (`LinkedTo`, `LinkedToFlags`).
    pub current_link: Option<(Vec<u8>, i64)>,
    /// The link the target is given.
    pub target_link: Option<(Vec<u8>, i64)>,
    /// The waypoints it places, in the order it places them, with their tags.
    pub made: Vec<(Placed, Vec<u8>)>,
    /// The waypoints that already stand where one would be made.
    pub reused: Vec<(Placed, Vec<u8>)>,
    /// The areas whose GIT it changes.
    pub writes: Vec<ResRef>,
    /// The links it replaces.
    pub replaced: Vec<Replaced>,
}

/// What [`plan`] can't do.
#[derive(Debug)]
pub enum PlanError {
    Refused(Refused),
    Edit(EditError),
}

impl From<EditError> for PlanError {
    fn from(e: EditError) -> Self {
        PlanError::Edit(e)
    }
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::Refused(r) => write!(f, "{r:?}"),
            PlanError::Edit(e) => write!(f, "{e}"),
        }
    }
}

/// The struct at `path` of document `key`, as the workspace has it now.
fn object(ws: &mut Workspace, key: &ResKey, path: &GffPath) -> Option<Struct> {
    path.get(&ws.doc(key).ok()?.root).cloned()
}

/// A value of an integer field, of the type the field has (else Byte).
fn flags_value(existing: Option<&Value>, v: i64) -> Value {
    match existing.map_or(FieldType::Byte, Value::field_type) {
        FieldType::Char => Value::Char(v as i8),
        FieldType::Word => Value::Word(v as u16),
        FieldType::Short => Value::Short(v as i16),
        FieldType::Dword => Value::Dword(v as u32),
        FieldType::Dword64 => Value::Dword64(v as u64),
        FieldType::Int => Value::Int(v as i32),
        FieldType::Int64 => Value::Int64(v),
        _ => Value::Byte(v as u8),
    }
}

/// The two edits that give the object at `key`/`path` (`now` as it is)
/// the link `to`.
fn link_edits(key: ResKey, path: &GffPath, now: &Struct, to: &(Vec<u8>, i64)) -> [Edit; 2] {
    let set = |label: &str, value| Edit::SetField {
        key,
        path: path.clone(),
        label: label.into(),
        value: Some(value),
    };
    [
        set("LinkedTo", Value::String(to.0.clone())),
        set("LinkedToFlags", flags_value(now.get("LinkedToFlags"), to.1)),
    ]
}

/// The command (labeled "Area Transition") that sets up `connection`
/// between `current` and `to`, as the workspace has them now, and what it
/// does.
pub fn plan(
    ws: &mut Workspace,
    current: &Current,
    to: Placed,
    connection: Connection,
) -> Result<Plan, PlanError> {
    let refused = |r| PlanError::Refused(r);
    let target = object(ws, &to.git(), &to.path()).ok_or(refused(Refused::Missing))?;
    let (cur, cur_placed) = match current {
        Current::Placed(p) => {
            if *p == to {
                return Err(refused(Refused::Itself));
            }
            (object(ws, &p.git(), &p.path()).ok_or(refused(Refused::Missing))?, Some(*p))
        }
        Current::Fields(targets) => {
            if targets.iter().any(|(k, p)| *k == to.git() && *p == to.path()) {
                return Err(refused(Refused::Itself));
            }
            let (k, p) = targets.first().ok_or(refused(Refused::Missing))?;
            (object(ws, k, p).ok_or(refused(Refused::Missing))?, None)
        }
    };
    let cur_tag = cur.string("Tag").unwrap_or_default().to_vec();
    if let Some(r) = refusal(current, &cur_tag, to.kind, connection) {
        return Err(refused(r));
    }
    if target.string("Tag").unwrap_or_default().is_empty() {
        return Err(refused(Refused::NoTargetTag));
    }
    let mut edits = Vec::new();
    let mut plan = Plan {
        command: Command::new("Area Transition", Vec::new()),
        current_link: None,
        target_link: None,
        made: Vec::new(),
        reused: Vec::new(),
        writes: Vec::new(),
        replaced: Vec::new(),
    };
    let writes = |plan: &mut Plan, area: ResRef| {
        if !plan.writes.contains(&area) {
            plan.writes.push(area);
        }
    };
    let replaced = |object: Option<Placed>, s: &Struct, now: &[u8]| {
        let was = s.string("LinkedTo").unwrap_or_default();
        (!was.is_empty() && was != now).then(|| Replaced {
            object,
            tag: s.string("Tag").unwrap_or_default().to_vec(),
            was: was.to_vec(),
            now: now.to_vec(),
        })
    };
    // The current object's half, on it (or on each edited together).
    if connection.links_current() {
        let link = link_to(to.kind, &target);
        match current {
            Current::Placed(p) => {
                edits.extend(link_edits(p.git(), &p.path(), &cur, &link));
                plan.replaced.extend(replaced(Some(*p), &cur, &link.0));
                writes(&mut plan, p.area);
            }
            Current::Fields(targets) => {
                for (k, p) in targets {
                    let Some(s) = object(ws, k, p) else { continue };
                    edits.extend(link_edits(*k, p, &s, &link));
                    plan.replaced.extend(replaced(None, &s, &link.0));
                    if k.restype == ResType::GIT {
                        writes(&mut plan, k.resref);
                    }
                }
            }
        }
        plan.current_link = Some(link);
    }
    // The target's half, back to the current one.
    let cur_placed_kind = cur_placed.map(|p| p.kind);
    if connection.links_target()
        && let Some(p) = cur_placed
    {
        let link = link_to(p.kind, &cur);
        edits.extend(link_edits(to.git(), &to.path(), &target, &link));
        plan.replaced.extend(replaced(Some(to), &target, &link.0));
        writes(&mut plan, to.area);
        plan.target_link = Some(link);
    }
    // The waypoints: the target's first, then the current's (Aurora's
    // order, the same area's taking the places one after the other).
    let mut wanted: Vec<(Placed, &Struct)> = Vec::new();
    if to.kind == ObjectKind::Trigger && connection.links_current() {
        wanted.push((to, &target));
    }
    if cur_placed_kind == Some(ObjectKind::Trigger)
        && connection.links_target()
        && let Some(p) = cur_placed
    {
        wanted.push((p, &cur));
    }
    // (Those placed by this command, by area, for a second to find.)
    let mut placed: Vec<(ResRef, Struct)> = Vec::new();
    for (trigger_at, trigger) in wanted {
        let git = trigger_at.git();
        let list: Vec<Struct> =
            (ws.doc(&git)?.root.list(ObjectKind::Waypoint.list())).unwrap_or(&[]).to_vec();
        let tag = waypoint_tag(trigger.string("Tag").unwrap_or_default());
        if let Some(i) = reusable(trigger, &list) {
            plan.reused.push((Placed::new(trigger_at.area, ObjectKind::Waypoint, i), tag));
            continue;
        }
        let mine: Vec<Struct> = (placed.iter())
            .filter(|(a, _)| *a == trigger_at.area)
            .map(|(_, s)| s.clone())
            .collect();
        if let Some(i) = reusable(trigger, &mine) {
            plan.reused
                .push((Placed::new(trigger_at.area, ObjectKind::Waypoint, list.len() + i), tag));
            continue;
        }
        let index = list.len() + mine.len();
        let item = waypoint_for(trigger);
        edits.push(Edit::insert_item(
            git,
            GffPath::root(),
            ObjectKind::Waypoint.list(),
            index,
            item.clone(),
        ));
        placed.push((trigger_at.area, item));
        plan.made.push((Placed::new(trigger_at.area, ObjectKind::Waypoint, index), tag));
        writes(&mut plan, trigger_at.area);
    }
    plan.command.edits = edits;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::Gff;
    use mg_module::Module;

    fn text(s: &str) -> Value {
        Value::String(s.as_bytes().to_vec())
    }

    fn area(name: &str) -> ResRef {
        ResRef::from_str(name).unwrap()
    }

    /// A trigger as Aurora painted TT_A1: tag, `LinkedTo` "" and
    /// `LinkedToFlags` Byte 0, at `at` with `points` relative.
    fn trigger(tag: &str, at: [f32; 3], points: &[[f32; 3]]) -> Struct {
        let mut s = Struct::new(1);
        s.set("Tag", text(tag));
        s.set("TemplateResRef", Value::resref(ResRef::from_str("newtransition").unwrap()));
        s.set("Cursor", Value::Byte(1));
        s.set("LinkedTo", text(""));
        s.set("LinkedToFlags", Value::Byte(0));
        s.set("Type", Value::Int(1));
        s.set("XPosition", Value::Float(at[0]));
        s.set("YPosition", Value::Float(at[1]));
        s.set("ZPosition", Value::Float(at[2]));
        let geometry = points
            .iter()
            .map(|p| {
                let mut g = Struct::new(3);
                g.set("PointX", Value::Float(p[0]));
                g.set("PointY", Value::Float(p[1]));
                g.set("PointZ", Value::Float(p[2]));
                g
            })
            .collect();
        s.set("Geometry", Value::List(geometry));
        s
    }

    /// TT_A1's quad as Aurora saved it.
    const QUAD: [[f32; 3]; 4] = [
        [0.0, 0.0, 5.025],
        [4.995696, 9.536743e-7, 5.025],
        [5.9854116, 3.0162716, 5.025],
        [0.9897127, 4.0059843, 5.025],
    ];

    fn door(tag: &str) -> Struct {
        let mut s = Struct::new(8);
        s.set("Tag", text(tag));
        s.set("LinkedTo", text(""));
        s.set("LinkedToFlags", Value::Byte(0));
        s
    }

    fn waypoint(tag: &str, at: [f32; 3]) -> Struct {
        mg_module::instances::walk_waypoint(tag.as_bytes(), at)
    }

    /// A module of areas, each its triggers, doors and waypoints.
    /// An area of a test module: its name, triggers, doors and waypoints.
    type Area<'a> = (&'a str, Vec<Struct>, Vec<Struct>, Vec<Struct>);

    fn module(areas: &[Area<'_>]) -> Workspace {
        let mut m = Module::default();
        for (name, triggers, doors, waypoints) in areas {
            let mut g = Gff::new(*b"GIT ");
            g.root.set("Door List", Value::List(doors.clone()));
            g.root.set("TriggerList", Value::List(triggers.clone()));
            g.root.set("WaypointList", Value::List(waypoints.clone()));
            m.set_gff(ResKey::new(area(name), ResType::GIT), &g).unwrap();
        }
        Workspace::new(m)
    }

    fn at(name: &str, kind: ObjectKind, index: usize) -> Placed {
        Placed::new(area(name), kind, index)
    }

    fn get(ws: &mut Workspace, p: Placed) -> Struct {
        object(ws, &p.git(), &p.path()).unwrap()
    }

    fn link(s: &Struct) -> (String, i64) {
        let t = String::from_utf8_lossy(s.string("LinkedTo").unwrap_or_default()).into_owned();
        (t, s.integer("LinkedToFlags").unwrap_or(-1))
    }

    fn waypoints(ws: &mut Workspace, name: &str) -> Vec<Struct> {
        let git = ResKey::new(area(name), ResType::GIT);
        ws.doc(&git).unwrap().root.list("WaypointList").unwrap_or(&[]).to_vec()
    }

    fn tags(list: &[Struct]) -> Vec<String> {
        list.iter()
            .map(|w| String::from_utf8_lossy(w.string("Tag").unwrap_or_default()).into_owned())
            .collect()
    }

    fn run(ws: &mut Workspace, current: &Current, to: Placed, c: Connection) -> Plan {
        let plan = plan(ws, current, to, c).unwrap();
        ws.apply(plan.command.clone()).unwrap();
        plan
    }

    /// TT_A1's waypoint where Aurora put it: (14.98074, 13.96746, 5.025).
    #[test]
    fn the_waypoint_stands_at_the_center_of_the_trigger_s_box() {
        let t = trigger("TT_A1", [11.988032, 11.964466, 0.0], &QUAD);
        let (lo, hi) = bounds(&t);
        assert!((hi[0] - lo[0] - 5.98541).abs() < 1e-4 && (hi[1] - lo[1] - 4.00598).abs() < 1e-4);
        let p = waypoint_position(&t);
        for (got, want) in p.iter().zip([14.980738, 13.967459, 5.025]) {
            assert!((got - want).abs() < 1e-4, "{p:?}");
        }
        // Aurora's struct, its fields in Aurora's order.
        let w = waypoint_for(&t);
        assert_eq!(w.id, 5);
        let labels: Vec<String> = w.fields.iter().map(|f| f.label.to_string_lossy()).collect();
        let aurora = [
            "Appearance",
            "LinkedTo",
            "TemplateResRef",
            "Tag",
            "LocalizedName",
            "Description",
            "HasMapNote",
            "MapNote",
            "MapNoteEnabled",
            "XPosition",
            "YPosition",
            "ZPosition",
            "XOrientation",
            "YOrientation",
        ];
        assert_eq!(labels, aurora);
        assert_eq!(w.string("Tag"), Some(b"WP_TT_A1".as_slice()));
        assert_eq!(w.get("Appearance"), Some(&Value::Byte(1)));
        assert_eq!(w.float("XOrientation"), Some(0.0));
        assert_eq!(w.float("YOrientation"), Some(1.0));
    }

    /// On a slope: the middle of the points' heights (TT_A4's: 5.025 and
    /// 5.9688, 5.49691), plus the trigger's own height; a trigger with no
    /// points stands its waypoint where it stands.
    #[test]
    fn the_waypoint_s_height_is_the_middle_of_the_outline_s() {
        let slope = [[0.0, 0.0, 5.025], [5.0, 0.0, 5.025], [6.0, 3.0, 5.9688], [1.0, 4.0, 5.3]];
        let t = trigger("TT_A4", [21.979, 1.973, 0.0], &slope);
        assert!((waypoint_position(&t)[2] - 5.49690).abs() < 1e-4);
        let raised = trigger("R", [0.0, 0.0, -5.0], &slope);
        assert!((waypoint_position(&raised)[2] - 0.49690).abs() < 1e-4);
        let none = trigger("N", [3.0, 4.0, 1.0], &[]);
        assert_eq!(waypoint_position(&none), [3.0, 4.0, 1.0]);
        assert!(inside(&none, [3.0, 4.0]) && !inside(&none, [3.1, 4.0]));
        // (Odd floats are left out, not a panic.)
        let odd = trigger("O", [1.0, 1.0, 0.0], &[[f32::NAN, 0.0, 0.0], [2.0, 2.0, 0.0]]);
        assert_eq!(waypoint_position(&odd), [3.0, 3.0, 0.0]);
    }

    /// The tag is `WP_` and the trigger's, its case kept.
    #[test]
    fn the_waypoint_s_tag_is_the_trigger_s_after_wp() {
        assert_eq!(waypoint_tag(b"AreaTransition"), b"WP_AreaTransition");
        assert_eq!(waypoint_tag(b"tt_shared"), b"WP_tt_shared");
        assert_eq!(waypoint_tag(b""), b"WP_");
    }

    /// A waypoint of its tag inside its box is used again; outside, or of
    /// another tag (its case too), it is not.
    #[test]
    fn a_waypoint_inside_the_box_is_used_again() {
        let t = trigger("TT_A1", [10.0, 10.0, 0.0], &QUAD);
        let inside_box = waypoint("WP_TT_A1", [12.0, 12.0, 5.0]);
        let outside = waypoint("WP_TT_A1", [30.0, 12.0, 5.0]);
        let other = waypoint("wp_tt_a1", [12.0, 12.0, 5.0]);
        assert_eq!(reusable(&t, &[outside.clone(), other.clone(), inside_box.clone()]), Some(2));
        assert_eq!(reusable(&t, &[outside, other]), None);
    }

    /// Both Ways, trigger to trigger in two areas (Aurora's 06 to 07):
    /// each links to the other's waypoint (2), the target's waypoint made
    /// first; nothing else changes.
    #[test]
    fn both_ways_trigger_to_trigger() {
        let a1 = trigger("TT_A1", [11.988032, 11.964466, 0.0], &QUAD);
        let b2 = trigger("TT_B2", [1.0, 30.0, 0.0], &QUAD);
        let mut ws = module(&[
            ("alpha", vec![a1.clone()], vec![], vec![waypoint("NW", [1.0, 1.0, 0.0])]),
            ("beta", vec![b2.clone()], vec![], vec![]),
        ]);
        let (cur, to) = (at("alpha", ObjectKind::Trigger, 0), at("beta", ObjectKind::Trigger, 0));
        let plan = run(&mut ws, &Current::Placed(cur), to, Connection::BothWays);
        assert_eq!(link(&get(&mut ws, cur)), ("WP_TT_B2".into(), 2));
        assert_eq!(link(&get(&mut ws, to)), ("WP_TT_A1".into(), 2));
        assert_eq!(
            plan.made,
            vec![
                (at("beta", ObjectKind::Waypoint, 0), b"WP_TT_B2".to_vec()),
                (at("alpha", ObjectKind::Waypoint, 1), b"WP_TT_A1".to_vec())
            ]
        );
        assert_eq!(plan.writes, vec![area("alpha"), area("beta")]);
        assert!(plan.replaced.is_empty());
        assert_eq!(tags(&waypoints(&mut ws, "alpha")), ["NW", "WP_TT_A1"]);
        assert_eq!(waypoints(&mut ws, "alpha")[1], waypoint_for(&a1));
        // Only LinkedTo and LinkedToFlags changed, in place and Byte.
        let now = get(&mut ws, cur);
        assert_eq!(now.fields.len(), a1.fields.len());
        for (a, b) in a1.fields.iter().zip(&now.fields) {
            assert_eq!(a.label, b.label);
            let l = a.label.to_string_lossy();
            if l != "LinkedTo" && l != "LinkedToFlags" {
                assert_eq!(a.value, b.value, "{l}");
            }
        }
        assert_eq!(now.get("LinkedToFlags"), Some(&Value::Byte(2)));
        // One step to undo, every object with it.
        ws.undo().unwrap();
        assert_eq!(get(&mut ws, cur), a1);
        assert_eq!(get(&mut ws, to), b2);
        assert_eq!(tags(&waypoints(&mut ws, "alpha")), ["NW"]);
        assert!(waypoints(&mut ws, "beta").is_empty());
        ws.redo().unwrap();
        assert_eq!(link(&get(&mut ws, to)), ("WP_TT_A1".into(), 2));
        assert_eq!(waypoints(&mut ws, "beta").len(), 1);
    }

    /// In one area (Aurora's 09 to 10): the two waypoints take the next
    /// two places, the target's first.
    #[test]
    fn both_ways_in_one_area_places_the_target_s_first() {
        let mut ws = module(&[(
            "alpha",
            vec![trigger("TT_A4", [0.0; 3], &QUAD), trigger("TT_A5", [20.0, 0.0, 0.0], &QUAD)],
            vec![],
            vec![waypoint("X", [0.0; 3])],
        )]);
        let cur = at("alpha", ObjectKind::Trigger, 0);
        let to = at("alpha", ObjectKind::Trigger, 1);
        let plan = run(&mut ws, &Current::Placed(cur), to, Connection::BothWays);
        assert_eq!(tags(&waypoints(&mut ws, "alpha")), ["X", "WP_TT_A5", "WP_TT_A4"]);
        assert_eq!(plan.made[0].0.index, 1);
        assert_eq!(plan.made[1].0.index, 2);
        assert_eq!(plan.writes, vec![area("alpha")]);
    }

    /// Trigger to door, Both Ways (11 to 12): the trigger names the door
    /// (1), the door the trigger's waypoint (2), made beside the trigger.
    /// Door to trigger (10 to 11): the door names the trigger's waypoint,
    /// the trigger the door. Door to door: each names the other, no
    /// waypoint.
    #[test]
    fn doors_are_named_by_their_tags_and_given_no_waypoint() {
        let mut ws = module(&[
            ("alpha", vec![trigger("TT_A6", [0.0; 3], &QUAD)], vec![door("TT_AD1")], vec![]),
            ("beta", vec![trigger("TT_B5", [0.0; 3], &QUAD)], vec![door("TT_BD1")], vec![]),
        ]);
        let a6 = at("alpha", ObjectKind::Trigger, 0);
        let bd1 = at("beta", ObjectKind::Door, 0);
        let plan = run(&mut ws, &Current::Placed(a6), bd1, Connection::BothWays);
        assert_eq!(link(&get(&mut ws, a6)), ("TT_BD1".into(), 1));
        assert_eq!(link(&get(&mut ws, bd1)), ("WP_TT_A6".into(), 2));
        assert_eq!(tags(&waypoints(&mut ws, "alpha")), ["WP_TT_A6"]);
        assert_eq!(plan.made.len(), 1);
        let ad1 = at("alpha", ObjectKind::Door, 0);
        let b5 = at("beta", ObjectKind::Trigger, 0);
        run(&mut ws, &Current::Placed(ad1), b5, Connection::BothWays);
        assert_eq!(link(&get(&mut ws, ad1)), ("WP_TT_B5".into(), 2));
        assert_eq!(link(&get(&mut ws, b5)), ("TT_AD1".into(), 1));
        assert_eq!(tags(&waypoints(&mut ws, "beta")), ["WP_TT_B5"]);
        let plan = run(&mut ws, &Current::Placed(ad1), bd1, Connection::BothWays);
        assert_eq!(link(&get(&mut ws, ad1)), ("TT_BD1".into(), 1));
        assert_eq!(link(&get(&mut ws, bd1)), ("TT_AD1".into(), 1));
        assert!(plan.made.is_empty() && plan.reused.is_empty());
    }

    /// Current To Target: the current end alone, the target's waypoint
    /// made if it is a trigger. Target To Current: the target end alone,
    /// the current's waypoint made if it is a trigger.
    #[test]
    fn one_way_writes_one_end() {
        let mk = || {
            module(&[
                ("alpha", vec![trigger("TT_A2", [0.0; 3], &QUAD)], vec![door("AD")], vec![]),
                ("beta", vec![trigger("TT_B3", [0.0; 3], &QUAD)], vec![door("BD")], vec![]),
            ])
        };
        let (a2, b3) = (at("alpha", ObjectKind::Trigger, 0), at("beta", ObjectKind::Trigger, 0));
        let (ad, bd) = (at("alpha", ObjectKind::Door, 0), at("beta", ObjectKind::Door, 0));
        let mut ws = mk();
        let plan = run(&mut ws, &Current::Placed(a2), b3, Connection::CurrentToTarget);
        assert_eq!(link(&get(&mut ws, a2)), ("WP_TT_B3".into(), 2));
        assert_eq!(link(&get(&mut ws, b3)), ("".into(), 0));
        assert_eq!(tags(&waypoints(&mut ws, "beta")), ["WP_TT_B3"]);
        assert!(waypoints(&mut ws, "alpha").is_empty());
        assert_eq!(plan.target_link, None);
        let mut ws = mk();
        let plan = run(&mut ws, &Current::Placed(a2), b3, Connection::TargetToCurrent);
        assert_eq!(link(&get(&mut ws, a2)), ("".into(), 0));
        assert_eq!(link(&get(&mut ws, b3)), ("WP_TT_A2".into(), 2));
        assert_eq!(tags(&waypoints(&mut ws, "alpha")), ["WP_TT_A2"]);
        assert!(waypoints(&mut ws, "beta").is_empty());
        assert_eq!(plan.current_link, None);
        // (Doors, measured in Aurora too: the `transition-triggers`
        // captures 44 to 48, `aurora_transitions.rs`.)
        let mut ws = mk();
        let plan = run(&mut ws, &Current::Placed(ad), b3, Connection::TargetToCurrent);
        assert_eq!(link(&get(&mut ws, b3)), ("AD".into(), 1));
        assert!(plan.made.is_empty());
        let mut ws = mk();
        run(&mut ws, &Current::Placed(a2), bd, Connection::CurrentToTarget);
        assert_eq!(link(&get(&mut ws, a2)), ("BD".into(), 1));
        assert_eq!(link(&get(&mut ws, bd)), ("".into(), 0));
        assert!(waypoints(&mut ws, "alpha").is_empty() && waypoints(&mut ws, "beta").is_empty());
        let mut ws = mk();
        run(&mut ws, &Current::Placed(ad), bd, Connection::TargetToCurrent);
        assert_eq!(link(&get(&mut ws, bd)), ("AD".into(), 1));
        assert_eq!(link(&get(&mut ws, ad)), ("".into(), 0));
    }

    /// A waypoint is led to (2, its tag) from the current end alone, and
    /// nothing is made.
    #[test]
    fn a_waypoint_is_led_to_from_the_current_end_alone() {
        let mut ws = module(&[
            ("alpha", vec![trigger("TT_A7", [0.0; 3], &QUAD)], vec![], vec![]),
            ("beta", vec![], vec![], vec![waypoint("NW_WAYPOINT001", [5.0; 3])]),
        ]);
        let (a7, wp) = (at("alpha", ObjectKind::Trigger, 0), at("beta", ObjectKind::Waypoint, 0));
        for c in [Connection::BothWays, Connection::TargetToCurrent] {
            let e = plan(&mut ws, &Current::Placed(a7), wp, c);
            assert!(matches!(e, Err(PlanError::Refused(Refused::WaypointTarget))), "{c:?}");
        }
        let plan = run(&mut ws, &Current::Placed(a7), wp, Connection::CurrentToTarget);
        assert_eq!(link(&get(&mut ws, a7)), ("NW_WAYPOINT001".into(), 2));
        assert_eq!(get(&mut ws, wp), waypoint("NW_WAYPOINT001", [5.0; 3]), "unchanged");
        assert!(plan.made.is_empty());
        assert_eq!(plan.writes, vec![area("alpha")]);
    }

    /// A blueprint or several objects: Current To Target alone, each given
    /// the link; the target's waypoint made if it is a trigger. And a
    /// current object without a tag can't be led back to.
    #[test]
    fn without_one_placed_current_only_current_to_target() {
        let mut ws = module(&[(
            "alpha",
            vec![
                trigger("T1", [0.0; 3], &QUAD),
                trigger("T2", [10.0, 0.0, 0.0], &QUAD),
                trigger("", [20.0, 0.0, 0.0], &QUAD),
                trigger("DEST", [30.0, 0.0, 0.0], &QUAD),
            ],
            vec![],
            vec![],
        )]);
        let git = ResKey::new(area("alpha"), ResType::GIT);
        let path = |i| GffPath::root().item("TriggerList", i);
        let several = Current::Fields(vec![(git, path(0)), (git, path(1))]);
        let to = at("alpha", ObjectKind::Trigger, 3);
        let e = plan(&mut ws, &several, to, Connection::BothWays);
        assert!(matches!(e, Err(PlanError::Refused(Refused::NotOnePlaced))));
        // (Nor to one of them.)
        let e = plan(
            &mut ws,
            &several,
            at("alpha", ObjectKind::Trigger, 1),
            Connection::CurrentToTarget,
        );
        assert!(matches!(e, Err(PlanError::Refused(Refused::Itself))));
        let plan_ = run(&mut ws, &several, to, Connection::CurrentToTarget);
        for i in 0..2 {
            assert_eq!(link(&get(&mut ws, at("alpha", ObjectKind::Trigger, i))).0, "WP_DEST");
        }
        assert_eq!(tags(&waypoints(&mut ws, "alpha")), ["WP_DEST"]);
        assert_eq!(plan_.made.len(), 1);
        let untagged = Current::Placed(at("alpha", ObjectKind::Trigger, 2));
        let e = plan(&mut ws, &untagged, to, Connection::BothWays);
        assert!(matches!(e, Err(PlanError::Refused(Refused::NoCurrentTag))));
        let e = plan(&mut ws, &untagged, to, Connection::TargetToCurrent);
        assert!(matches!(e, Err(PlanError::Refused(Refused::NoCurrentTag))));
        assert!(plan(&mut ws, &untagged, to, Connection::CurrentToTarget).is_ok());
        // Nor the target without one, nor itself, nor one not there.
        let e = plan(
            &mut ws,
            &Current::Placed(to),
            at("alpha", ObjectKind::Trigger, 2),
            Connection::CurrentToTarget,
        );
        assert!(matches!(e, Err(PlanError::Refused(Refused::NoTargetTag))));
        let e = plan(&mut ws, &Current::Placed(to), to, Connection::CurrentToTarget);
        assert!(matches!(e, Err(PlanError::Refused(Refused::Itself))));
        let e = plan(
            &mut ws,
            &Current::Placed(to),
            at("alpha", ObjectKind::Trigger, 9),
            Connection::CurrentToTarget,
        );
        assert!(matches!(e, Err(PlanError::Refused(Refused::Missing))));
        // A blueprint's LinkedToFlags of another type keeps its type.
        let mut m = Module::default();
        let mut utt = Gff::new(*b"UTT ");
        utt.root.set("Tag", text("BP"));
        utt.root.set("LinkedToFlags", Value::Int(0));
        let key = ResKey::new(area("bp"), ResType::UTT);
        m.set_gff(key, &utt).unwrap();
        let mut g = Gff::new(*b"GIT ");
        g.root.set("Door List", Value::List(vec![door("D")]));
        m.set_gff(git, &g).unwrap();
        let mut ws = Workspace::new(m);
        let bp = Current::Fields(vec![(key, GffPath::root())]);
        run(&mut ws, &bp, at("alpha", ObjectKind::Door, 0), Connection::CurrentToTarget);
        let root = ws.doc(&key).unwrap().root.clone();
        assert_eq!(root.get("LinkedToFlags"), Some(&Value::Int(1)));
        assert_eq!(root.string("LinkedTo"), Some(b"D".as_slice()));
    }

    /// A second run uses again the waypoint the first made; two triggers
    /// of one tag in one area are given one each; a link replaced is said.
    #[test]
    fn a_second_run_uses_the_waypoint_again_and_says_what_it_replaces() {
        let mut ws = module(&[
            ("alpha", vec![trigger("TT_A1", [0.0; 3], &QUAD)], vec![], vec![]),
            (
                "beta",
                vec![
                    trigger("TT_SHARED", [8.0, 8.0, 0.0], &QUAD),
                    trigger("TT_SHARED", [18.0, 18.0, 0.0], &QUAD),
                ],
                vec![],
                vec![],
            ),
        ]);
        let a1 = Current::Placed(at("alpha", ObjectKind::Trigger, 0));
        let (s0, s1) = (at("beta", ObjectKind::Trigger, 0), at("beta", ObjectKind::Trigger, 1));
        run(&mut ws, &a1, s0, Connection::BothWays);
        // Again: nothing made, nothing replaced.
        let again = run(&mut ws, &a1, s0, Connection::BothWays);
        assert!(again.made.is_empty());
        assert_eq!(again.reused.len(), 2);
        assert!(again.replaced.is_empty());
        assert!(again.writes.contains(&area("beta")));
        // To the other of the same tag: a waypoint of its own, inside its box.
        let other = run(&mut ws, &a1, s1, Connection::BothWays);
        let beta = waypoints(&mut ws, "beta");
        assert_eq!(tags(&beta), ["WP_TT_SHARED", "WP_TT_SHARED"]);
        assert!(inside(
            &get(&mut ws, s1),
            [beta[1].float("XPosition").unwrap(), beta[1].float("YPosition").unwrap()]
        ));
        assert_eq!(
            other.made,
            vec![(at("beta", ObjectKind::Waypoint, 1), b"WP_TT_SHARED".to_vec())]
        );
        // A1 led to WP_TT_SHARED before too: the same name, not replaced.
        assert!(other.replaced.is_empty());
        // The old partner is left as it was.
        assert_eq!(link(&get(&mut ws, s0)), ("WP_TT_A1".into(), 2));
        // Led elsewhere: the link replaced is said.
        let mut ws2 = module(&[(
            "alpha",
            vec![trigger("TT_A1", [0.0; 3], &QUAD)],
            vec![door("D1"), door("D2")],
            vec![],
        )]);
        let a1 = Current::Placed(at("alpha", ObjectKind::Trigger, 0));
        run(&mut ws2, &a1, at("alpha", ObjectKind::Door, 0), Connection::BothWays);
        let to2 = at("alpha", ObjectKind::Door, 1);
        let p = plan(&mut ws2, &a1, to2, Connection::BothWays).unwrap();
        assert_eq!(
            p.replaced,
            vec![Replaced {
                object: Some(at("alpha", ObjectKind::Trigger, 0)),
                tag: b"TT_A1".to_vec(),
                was: b"D1".to_vec(),
                now: b"D2".to_vec()
            }]
        );
        // Its waypoint, made by the first run, is used again.
        assert!(p.made.is_empty() && p.reused.len() == 1);
    }

    /// The placed object a Properties window's path names.
    #[test]
    fn a_placed_object_is_its_list_and_place() {
        let git = ResKey::new(area("alpha"), ResType::GIT);
        let path = GffPath::root().item("Door List", 3);
        assert_eq!(Placed::of(&git, &path), Some(at("alpha", ObjectKind::Door, 3)));
        assert_eq!(Placed::of(&git, &GffPath::root()), None);
        assert_eq!(Placed::of(&ResKey::new(area("x"), ResType::UTT), &path), None);
        assert_eq!(at("alpha", ObjectKind::Door, 3).path(), path);
    }
}
