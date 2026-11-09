//! Quick fixes: edits a finding of Verify Module offers to make, made only
//! when the user asks (a button beside the finding in the window, `mg
//! verify --fix` on the command line), as one undoable command.
//!
//! A finding is the doctor's ([`mg_module::doctor`], which knows nothing of
//! edits: its fixes are made here, [`attach`]) or a plugin's check's (whose
//! fixes the plugin records with `ctx.fix`). A fix is worked out on the
//! module as Verify read it, so a [`Guard`] holds each fix to the resources
//! it touches being as they were then.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use mg_core::ResType;
use mg_gff::{Struct, Value};
use mg_module::Module;
use mg_module::doctor::Finding;
use mg_module::tags::{TagIndex, Tagged};
use mg_resman::ResKey;

use crate::{Command, Edit, GffPath, Step, Workspace};

/// A finding's fix: what its button says, and the edits it makes; or,
/// for one that asks the user first, the plugin's command its button runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub label: String,
    pub edits: Vec<Edit>,
    /// A fix that asks (a form, a choice) is a plugin's command, run on
    /// what the finding is about when the button is clicked, its edits
    /// one step of Undo as a command's are. None: its edits are the fix.
    pub asks: Option<Asks>,
}

/// The command a fix that asks runs: the plugin's id, the command's, and
/// what it runs on (each a resource and where in it, as a plugin's
/// selection has it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asks {
    pub plugin: String,
    pub command: String,
    pub on: Vec<(ResKey, GffPath)>,
}

impl Fix {
    /// The fix as the command that makes it (Undo names it so).
    pub fn command(&self) -> Command {
        Command::new(self.label.clone(), self.edits.clone())
    }

    /// The resources it changes, each once, in the order first changed.
    pub fn resources(&self) -> Vec<ResKey> {
        let mut out: Vec<ResKey> = Vec::new();
        for e in &self.edits {
            if !out.contains(e.key()) {
                out.push(*e.key());
            }
        }
        out
    }

    /// Whether it only sets and removes fields, and takes items out of
    /// lists: then the paths of another fix of the same resources still
    /// lead where they did, or are followed ([`Guard::check`]).
    fn sets_fields_only(&self) -> bool {
        self.edits.iter().all(|e| matches!(e, Edit::SetField { .. } | Edit::RemoveItem { .. }))
    }
}

/// A finding, and its fix if it has one.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub finding: Finding,
    pub fix: Option<Fix>,
}

impl From<Finding> for Found {
    fn from(finding: Finding) -> Found {
        Found { finding, fix: None }
    }
}

/// Verify's findings with their fixes: those `plugins` found with one (by
/// the finding, which their checks made), and the doctor's fixes where it
/// has one ([`doctor_fix`]), worked out on `module` as the findings were.
pub fn attach(module: &Module, findings: Vec<Finding>, plugins: &[Found]) -> Vec<Found> {
    let mut used = vec![false; plugins.len()];
    let mut seen = Seen::default();
    findings
        .into_iter()
        .map(|finding| {
            let theirs = plugins
                .iter()
                .enumerate()
                .find(|(i, p)| !used[*i] && p.fix.is_some() && p.finding == finding);
            let fix = match theirs {
                Some((i, p)) => {
                    used[i] = true;
                    p.fix.clone()
                }
                None => doctor_fix_with(module, &finding, &mut seen),
            };
            Found { finding, fix }
        })
        .collect()
}

/// The fix of one of the doctor's findings, where one is plain:
/// `tag-unique` retags the objects after the first (`TAG_2`, `TAG_3`…;
/// an area keeps its tag), `area-variables` moves an area's variables
/// from its ARE to its GIT, `var-runtime` takes the variable out and
/// `static-transform` takes the placeable's visual transform off (as
/// ticking Static does: the game draws it without one). Others have none
/// (`pc-faction`: which faction is the builder's to choose).
pub fn doctor_fix(module: &Module, finding: &Finding) -> Option<Fix> {
    doctor_fix_with(module, finding, &mut Seen::default())
}

/// What the fixes of one Verify's findings share: the module's tags,
/// read once, and how many findings were said of each place (two
/// variables of one name in one `VarTable` are two findings of one
/// place, each the fix of one of them).
#[derive(Default)]
struct Seen {
    tags: Option<TagIndex>,
    said: HashMap<(String, ResKey, String), usize>,
}

fn doctor_fix_with(module: &Module, f: &Finding, seen: &mut Seen) -> Option<Fix> {
    if f.source != "module" {
        return None;
    }
    let said = seen.said.entry((f.check.to_string(), f.resource, f.at.clone())).or_default();
    let nth = *said;
    *said += 1;
    match f.check.as_ref() {
        "tag-unique" => {
            let index = seen.tags.get_or_insert_with(|| TagIndex::of_module(module));
            retag_fix(index, f)
        }
        "area-variables" => move_variables_fix(module, f.resource),
        "var-runtime" => remove_variable_fix(module, f, nth),
        "static-transform" => untransform_fix(module, f, nth),
        _ => None,
    }
}

/// The label of `var-runtime`'s fix.
pub const REMOVE_VARIABLE: &str = "Remove the variable";

/// The label of `static-transform`'s fix.
pub const TAKE_TRANSFORM_OFF: &str = "Take the transform off";

/// `var-runtime`: the variable out of its `VarTable` (the `nth` of those
/// the finding's place names, as the doctor found them).
fn remove_variable_fix(module: &Module, f: &Finding, nth: usize) -> Option<Fix> {
    let gff = module.gff(&f.resource)?.ok()?;
    let found = mg_module::var_desc::variables_of(f.resource, &gff).into_iter().filter(|v| {
        let table = mg_module::var_desc::table_path(&v.path);
        let place = mg_module::rename::describe(f.resource, Some(&gff), &table);
        format!("{place} › {}", v.name) == f.at
    });
    let v = found.into_iter().nth(nth)?;
    let edits = vec![Edit::remove_item(f.resource, v.path, "VarTable", v.index)];
    Some(Fix { label: REMOVE_VARIABLE.into(), edits, asks: None })
}

/// `static-transform`: the static placeable's visual transform off, as
/// ticking Static takes it off ([`mg_module::areas::static_fields`]): the
/// `nth` of those the finding's place names (placeables of one tag are
/// said alike).
fn untransform_fix(module: &Module, f: &Finding, nth: usize) -> Option<Fix> {
    if f.resource.restype != ResType::GIT {
        return None;
    }
    let gff = module.gff(&f.resource)?.ok()?;
    let list = gff.root.list("Placeable List")?;
    let at = (0..list.len())
        .filter(|i| {
            let path = GffPath::root().item("Placeable List", *i).field("Static");
            mg_module::rename::describe(f.resource, Some(&gff), &path) == f.at
        })
        .nth(nth)?;
    let path = GffPath::root().item("Placeable List", at);
    let edits: Vec<Edit> = mg_module::areas::static_fields(&list[at])
        .into_iter()
        .filter(|(_, value)| value.is_none())
        .map(|(label, value)| Edit::SetField {
            key: f.resource,
            path: path.clone(),
            label: label.into(),
            value,
        })
        .collect();
    (!edits.is_empty()).then(|| Fix { label: TAKE_TRANSFORM_OFF.into(), edits, asks: None })
}

/// `tag-unique`: the places of the tag the finding is about (found as the
/// doctor found them: its first place is the finding's resource, and `at`
/// lists them all: the module's, or of one area where the project wants
/// the tag unique in each), each but one given a tag of its own.
fn retag_fix(index: &TagIndex, f: &Finding) -> Option<Fix> {
    let said = |places: &[Tagged]| {
        places[0].key() == f.resource
            && places.iter().map(|p| p.describe()).collect::<Vec<_>>().join(", ") == f.at
    };
    let whole = index.duplicates().into_iter().find(|(_, places)| said(places));
    let (tag, places): (&[u8], Vec<Tagged>) = match whole {
        Some((tag, places)) => (tag, places.to_vec()),
        None => index.duplicates_in_areas().into_iter().find(|(_, places)| said(places))?,
    };
    // (Tags are written as bytes: one that is not plain text is left to
    // the builder.)
    let tag = std::str::from_utf8(tag).ok().filter(|t| t.is_ascii())?.to_string();
    // The area keeps its tag, if one of them is an area; else the first.
    let keep = places.iter().position(|p| matches!(p, Tagged::Area(_))).unwrap_or(0);
    let mut chosen: Vec<String> = Vec::new();
    let mut edits = Vec::new();
    for (i, place) in places.iter().enumerate() {
        let Tagged::Object { kind, index: at, .. } = *place else { continue };
        if i == keep {
            continue;
        }
        let taken = |t: &str| index.count(t.as_bytes()) > 0 || chosen.iter().any(|c| c == t);
        let new = mg_module::tags::retag(&tag, &taken);
        edits.push(Edit::SetField {
            key: place.key(),
            path: GffPath::root().item(kind.list(), at),
            label: "Tag".into(),
            value: Some(Value::String(new.clone().into_bytes())),
        });
        chosen.push(new);
    }
    if edits.is_empty() {
        return None;
    }
    let label = match chosen.as_slice() {
        [one] => format!("Retag the other as {one}"),
        many => format!("Retag the other {} ({}…)", many.len(), many[0]),
    };
    Some(Fix { label, edits, asks: None })
}

/// An area's variables as they are once moved from its ARE to its GIT:
/// the GIT's, then those of the ARE it has none of the same name of (the
/// GIT's value is the one the game reads).
pub fn variables_moved(git: &[Struct], are: &[Struct]) -> Vec<Struct> {
    let name = |s: &Struct| s.string("Name").map(<[u8]>::to_vec);
    let mut out = git.to_vec();
    for v in are {
        if !git.iter().any(|g| name(g) == name(v)) {
            out.push(v.clone());
        }
    }
    out
}

/// `area-variables`: the area's variables into its GIT, and none left in
/// its ARE.
fn move_variables_fix(module: &Module, are: ResKey) -> Option<Fix> {
    if are.restype != ResType::ARE {
        return None;
    }
    let git = ResKey::new(are.resref, ResType::GIT);
    let list = |key: &ResKey| -> Option<Vec<Struct>> {
        let gff = module.gff(key)?.ok()?;
        Some(gff.root.list("VarTable").map(<[_]>::to_vec).unwrap_or_default())
    };
    let (in_are, in_git) = (list(&are)?, list(&git)?);
    if in_are.is_empty() {
        return None;
    }
    let set = |key: ResKey, value: Option<Value>| Edit::SetField {
        key,
        path: GffPath::root(),
        label: "VarTable".into(),
        value,
    };
    let edits =
        vec![set(git, Some(Value::List(variables_moved(&in_git, &in_are)))), set(are, None)];
    Some(Fix { label: "Move the variables to the GIT".into(), edits, asks: None })
}

/// A resource's bytes as a number to compare (`None`: the module has no
/// such resource).
fn digest(bytes: Option<&[u8]>) -> Option<u64> {
    bytes.map(|b| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        b.hash(&mut h);
        h.finish()
    })
}

/// What making a fix changed of a [`Guard`], to take back when the fix is
/// undone ([`Guard::unmade`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Made {
    /// Each resource's digest before (none: not held).
    seen: Vec<(ResKey, Option<Option<u64>>)>,
    removed: Vec<((ResKey, GffPath, String), usize)>,
}

/// A list: its resource, the path of its struct and its label, as Verify
/// read them.
type ListAt = (ResKey, GffPath, String);

/// The items one fix took out so far, by their list: their places as
/// Verify read them. A fix's edits run one after another, so an edit
/// after a removal has its places as the removal left them: these turn
/// them back.
#[derive(Debug, Default)]
struct Removed(HashMap<ListAt, Vec<usize>>);

impl Removed {
    /// The place Verify read of what is at `index` of the list now.
    fn index_then(&self, at: &ListAt, index: usize) -> usize {
        let mut earlier = self.0.get(at).cloned().unwrap_or_default();
        earlier.sort_unstable();
        earlier.into_iter().fold(index, |then, gone| then + usize::from(gone <= then))
    }

    /// The path Verify read of what is at `path` now.
    fn path_then(&self, key: &ResKey, path: &GffPath) -> GffPath {
        let mut then = GffPath::root();
        for step in &path.0 {
            let step = match step {
                Step::Field(_) => step.clone(),
                Step::Item(list, i) => {
                    let at = (*key, then.clone(), list.clone());
                    Step::Item(list.clone(), self.index_then(&at, *i))
                }
            };
            then.0.push(step);
        }
        then
    }
}

/// What a Verify's fixes were worked out on: each resource one of them
/// touches, as Verify read it. A fix is made only while each of its
/// resources is as it was then, or has changed since only by fixes of the
/// same Verify that set fields (whose paths stay good) or took items out
/// of lists (whose paths it follows: the second of two variables of one
/// `VarTable` taken out is one place up); otherwise Verify is to run
/// again. So a fix never lands on another object than the one it was
/// found on.
#[derive(Debug, Clone, Default)]
pub struct Guard {
    /// Each resource's bytes' digest as a fix may find them; a resource a
    /// fix changed otherwise than by setting fields and taking items out
    /// is gone from here.
    seen: HashMap<ResKey, Option<u64>>,
    /// The items fixes took out, by their list (its resource, the path of
    /// its struct and its label, as Verify read them): their places, as
    /// Verify read them.
    removed: HashMap<(ResKey, GffPath, String), Vec<usize>>,
}

impl Guard {
    /// The resources `fixes` touch, as they are in `module` (the module
    /// the findings were made on).
    pub fn new<'a>(module: &Module, fixes: impl IntoIterator<Item = &'a Fix>) -> Guard {
        let mut seen = HashMap::new();
        for fix in fixes {
            for key in fix.resources() {
                seen.entry(key).or_insert_with(|| digest(module.get(&key)));
            }
        }
        Guard { seen, removed: HashMap::new() }
    }

    /// Whether `fix` may be made on the module of `ws` now, and the fix
    /// to make: `fix` with its paths following the items other fixes
    /// took out; else why not.
    pub fn check(&self, ws: &mut Workspace, fix: &Fix) -> Result<Fix, String> {
        ws.flush().map_err(|e| e.to_string())?;
        for key in fix.resources() {
            let now = digest(ws.module.get(&key));
            match self.seen.get(&key) {
                Some(then) if *then == now => {}
                _ => {
                    return Err(format!(
                        "{key} has changed since Verify found this: run Verify again for a fix"
                    ));
                }
            }
        }
        let gone = |key: &ResKey| {
            format!("{key}: another fix took out what this fixes: run Verify again for a fix")
        };
        let mut edits = Vec::new();
        // (The fix's own edits run one after another: what it took out
        // before an edit moves that edit's places too.)
        let mut own = Removed::default();
        for e in &fix.edits {
            let key = *e.key();
            let path_now =
                |then: &GffPath| self.path_now(&own, &key, then).ok_or_else(|| gone(&key));
            // How many items before place `index` (as Verify read it) of a
            // list are out: other fixes', and this one's so far.
            let before = |at: &ListAt, index: usize| {
                let below = |r: Option<&Vec<usize>>| {
                    r.map_or(0, |r| r.iter().filter(|i| **i < index).count())
                };
                below(self.removed.get(at)) + below(own.0.get(at))
            };
            edits.push(match e {
                Edit::SetField { path, label, value, .. } => {
                    let then = own.path_then(&key, path);
                    // (A list set whole, which held an item taken out:
                    // set as it was, it would put the item back.)
                    if self.removed.contains_key(&(key, then.clone(), label.clone())) {
                        return Err(gone(&key));
                    }
                    let path = path_now(&then)?;
                    Edit::SetField { key, path, label: label.clone(), value: value.clone() }
                }
                Edit::RemoveItem { path, list, index, .. } => {
                    let at = (key, own.path_then(&key, path), list.clone());
                    let then = own.index_then(&at, *index);
                    if self.removed.get(&at).is_some_and(|r| r.contains(&then)) {
                        return Err(gone(&key));
                    }
                    let now = Edit::remove_item(
                        key,
                        path_now(&at.1)?,
                        list.clone(),
                        then - before(&at, then),
                    );
                    own.0.entry(at).or_default().push(then);
                    now
                }
                Edit::InsertItem { path, list, index, item, .. } => {
                    let at = (key, own.path_then(&key, path), list.clone());
                    let then = own.index_then(&at, *index);
                    let index = then - before(&at, then);
                    Edit::insert_item(key, path_now(&at.1)?, list.clone(), index, item.clone())
                }
                Edit::SetResource { .. } => e.clone(),
            });
        }
        Ok(Fix { label: fix.label.clone(), edits, asks: fix.asks.clone() })
    }

    /// Where a path Verify read leads now that fixes took items out (none:
    /// through one another fix took out): `own` holds what the fix being
    /// made took out itself so far.
    fn path_now(&self, own: &Removed, key: &ResKey, path: &GffPath) -> Option<GffPath> {
        let (mut then, mut now) = (GffPath::root(), GffPath::root());
        for step in &path.0 {
            let step_now = match step {
                Step::Field(_) => step.clone(),
                Step::Item(list, i) => {
                    let at = (*key, then.clone(), list.clone());
                    let others = self.removed.get(&at);
                    if others.is_some_and(|r| r.contains(i)) {
                        return None;
                    }
                    let below = |r: Option<&Vec<usize>>| {
                        r.map_or(0, |r| r.iter().filter(|x| *x < i).count())
                    };
                    Step::Item(list.clone(), i - below(others) - below(own.0.get(&at)))
                }
            };
            then.0.push(step.clone());
            now.0.push(step_now);
        }
        Some(now)
    }

    /// Notes that `fix` (as Verify found it: the one given to
    /// [`Guard::check`]) was made on the module of `ws`: its resources as
    /// they are now are those the other fixes may find, if it only set
    /// fields and took items out (which they follow); else they refuse
    /// them.
    pub fn made(&mut self, ws: &mut Workspace, fix: &Fix) -> Made {
        let flushed = ws.flush().is_ok();
        let follows = flushed && fix.sets_fields_only();
        let mut made = Made::default();
        for key in fix.resources() {
            let was = if follows {
                self.seen.insert(key, digest(ws.module.get(&key)))
            } else {
                self.seen.remove(&key)
            };
            made.seen.push((key, was));
        }
        if follows {
            // (Each place as Verify read it, not as the edit has it: the
            // second of a fix's two removals at one place took out the
            // item after the first's.)
            let mut own = Removed::default();
            for e in &fix.edits {
                if let Edit::RemoveItem { key, path, list, index } = e {
                    let at = (*key, own.path_then(key, path), list.clone());
                    let then = own.index_then(&at, *index);
                    own.0.entry(at.clone()).or_default().push(then);
                    self.removed.entry(at.clone()).or_default().push(then);
                    made.removed.push((at, then));
                }
            }
        }
        made
    }

    /// Takes back what [`Guard::made`] noted of a fix, once the fix is
    /// undone (its resources as Verify read them again): it may be made
    /// again, and the other fixes there follow as before it.
    pub fn unmade(&mut self, made: &Made) {
        for (key, was) in made.seen.iter().rev() {
            match was {
                Some(d) => self.seen.insert(*key, *d),
                None => self.seen.remove(key),
            };
        }
        for (at, index) in &made.removed {
            if let Some(r) = self.removed.get_mut(at) {
                if let Some(i) = r.iter().rposition(|x| x == index) {
                    r.remove(i);
                }
                if r.is_empty() {
                    self.removed.remove(at);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::Gff;
    use mg_module::doctor::Severity;

    fn key(name: &str) -> ResKey {
        ResKey::from_filename(name).unwrap()
    }

    fn finding(check: &'static str, resource: &str, at: &str) -> Finding {
        Finding {
            severity: Severity::Warning,
            check: check.into(),
            source: "module".into(),
            resource: key(resource),
            at: at.into(),
            message: String::new(),
        }
    }

    fn tagged(tag: &str) -> Struct {
        let mut s = Struct::new(9);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s
    }

    /// Two areas: a store in each tagged SHOP, and a third SHOP_2 taken.
    fn module() -> Module {
        let mut m = Module::new();
        for (area, tags) in [("inn", vec!["SHOP", "SHOP_2"]), ("mill", vec!["SHOP"])] {
            let mut git = Gff::new(*b"GIT ");
            git.root.set("StoreList", Value::List(tags.iter().map(|t| tagged(t)).collect()));
            m.set_gff(key(&format!("{area}.git")), &git).unwrap();
            let mut are = Gff::new(*b"ARE ");
            are.root.set("Tag", Value::String(area.to_uppercase().into_bytes()));
            m.set_gff(key(&format!("{area}.are")), &are).unwrap();
        }
        m
    }

    #[test]
    fn a_tag_on_two_objects_is_retagged_on_the_second() {
        let m = module();
        let at = "store 0 in inn, store 0 in mill";
        let fix = doctor_fix(&m, &finding("tag-unique", "inn.git", at)).unwrap();
        assert_eq!(fix.label, "Retag the other as SHOP_3");
        assert_eq!(
            fix.edits,
            [Edit::SetField {
                key: key("mill.git"),
                path: GffPath::root().item("StoreList", 0),
                label: "Tag".into(),
                value: Some(Value::String(b"SHOP_3".to_vec())),
            }]
        );
        // A finding the doctor did not make this way: none.
        assert_eq!(doctor_fix(&m, &finding("tag-unique", "inn.git", "store 0 in inn")), None);
        assert_eq!(doctor_fix(&m, &finding("pc-faction", "inn.git", at)), None);
    }

    /// A tag the project wants unique in each area, twice in one: the
    /// finding lists that area's places, and the fix retags there alone.
    #[test]
    fn a_tag_twice_in_one_area_is_retagged_there() {
        let mut m = module();
        let git = key("mill.git");
        let mut g = m.gff(&git).unwrap().unwrap();
        g.root.set("StoreList", Value::List(vec![tagged("SHOP"), tagged("SHOP")]));
        m.set_gff(git, &g).unwrap();
        let at = "store 0 in mill, store 1 in mill";
        let fix = doctor_fix(&m, &finding("tag-unique", "mill.git", at)).unwrap();
        assert_eq!(fix.label, "Retag the other as SHOP_3");
        assert_eq!(
            fix.edits,
            [Edit::SetField {
                key: git,
                path: GffPath::root().item("StoreList", 1),
                label: "Tag".into(),
                value: Some(Value::String(b"SHOP_3".to_vec())),
            }]
        );
    }

    #[test]
    fn an_area_s_variables_move_to_its_git() {
        let mut m = module();
        let var = |name: &str, value: i32| {
            let mut v = Struct::new(0);
            v.set("Name", Value::String(name.as_bytes().to_vec()));
            v.set("Type", Value::Dword(1));
            v.set("Value", Value::Int(value));
            v
        };
        let mut are = m.gff(&key("inn.are")).unwrap().unwrap();
        are.root.set("VarTable", Value::List(vec![var("lit", 1), var("rooms", 4)]));
        m.set_gff(key("inn.are"), &are).unwrap();
        let mut git = m.gff(&key("inn.git")).unwrap().unwrap();
        git.root.set("VarTable", Value::List(vec![var("rooms", 6)]));
        m.set_gff(key("inn.git"), &git).unwrap();

        let fix = doctor_fix(&m, &finding("area-variables", "inn.are", "VarTable")).unwrap();
        let mut ws = Workspace::new(m);
        ws.apply(fix.command()).unwrap();
        let git = ws.doc(&key("inn.git")).unwrap().root.list("VarTable").unwrap().to_vec();
        assert_eq!(git, [var("rooms", 6), var("lit", 1)]);
        assert!(ws.doc(&key("inn.are")).unwrap().root.get("VarTable").is_none());
        assert_eq!(ws.can_undo(), Some("Move the variables to the GIT"));
    }

    /// A fix that takes two items out one after the other at one place (a
    /// plugin's "remove 3, remove 3" for items 3 and 4): the places noted
    /// are those Verify read, so a fix on the second of them is refused
    /// and one on a later item follows to where that item is (it was let
    /// through onto the object two places on).
    #[test]
    fn a_fix_s_removals_one_after_another_are_followed_by_the_others() {
        let mut m = module();
        let git = key("inn.git");
        let mut g = m.gff(&git).unwrap().unwrap();
        let tags = ["P0", "P1", "P2", "P3", "P4", "P5", "P6"];
        g.root.set("Placeable List", Value::List(tags.iter().map(|t| tagged(t)).collect()));
        m.set_gff(git, &g).unwrap();
        let remove = |index| Edit::remove_item(git, GffPath::root(), "Placeable List", index);
        let fix = |label: &str, edits| Fix { label: label.into(), edits, asks: None };
        let retag = |index: usize| {
            let path = GffPath::root().item("Placeable List", index);
            let value = Some(Value::String(b"NEW".to_vec()));
            fix("Retag", vec![Edit::SetField { key: git, path, label: "Tag".into(), value }])
        };
        let two = fix("Remove two", vec![remove(3), remove(3)]);
        let (on_third, on_fourth, on_fifth) = (retag(3), retag(4), retag(5));
        let (take_fourth, take_last) = (fix("Take", vec![remove(4)]), fix("Take", vec![remove(6)]));
        // (One that takes item 1 out and then sets what was item 2.)
        let own_path = {
            let path = GffPath::root().item("Placeable List", 1);
            let value = Some(Value::String(b"TWO".to_vec()));
            let set = Edit::SetField { key: git, path, label: "Tag".into(), value };
            fix("Take and set", vec![remove(1), set])
        };
        let all = [&two, &on_third, &on_fourth, &on_fifth, &take_fourth, &take_last, &own_path];
        let mut guard = Guard::new(&m, all);
        let mut ws = Workspace::new(m);
        let now = guard.check(&mut ws, &two).unwrap();
        assert_eq!(now, two, "its own places are as it has them");
        ws.apply(now.command()).unwrap();
        let made = guard.made(&mut ws, &two);
        let tags_now = |ws: &mut Workspace| -> Vec<String> {
            let list = ws.doc(&git).unwrap().root.list("Placeable List").unwrap();
            list.iter().map(|p| String::from_utf8_lossy(p.string("Tag").unwrap()).into()).collect()
        };
        assert_eq!(tags_now(&mut ws), ["P0", "P1", "P2", "P5", "P6"]);
        for gone in [&on_third, &on_fourth, &take_fourth] {
            let err = guard.check(&mut ws, gone).unwrap_err();
            assert!(err.contains("another fix took out"), "{err}");
        }
        // The fifth is two places up, and so is the last.
        let now = guard.check(&mut ws, &on_fifth).unwrap();
        assert_eq!(now, retag(3));
        assert_eq!(guard.check(&mut ws, &take_last).unwrap().edits, [remove(4)]);
        // A fix's own later edit keeps its place as the fix has it, less
        // what the others took out before it.
        let now = guard.check(&mut ws, &own_path).unwrap();
        assert_eq!(now, own_path);
        ws.apply(now.command()).unwrap();
        guard.made(&mut ws, &own_path);
        assert_eq!(tags_now(&mut ws), ["P0", "TWO", "P5", "P6"]);
        assert_eq!(guard.check(&mut ws, &take_last).unwrap().edits, [remove(3)]);
        assert!(guard.check(&mut ws, &retag(1)).is_err(), "item 1 is out");
        // What the first noted is taken back by the places Verify read.
        guard.unmade(&made);
        assert_eq!(guard.removed.values().cloned().collect::<Vec<_>>(), [vec![1]]);
    }

    #[test]
    fn a_fix_is_made_only_on_what_verify_saw() {
        let m = module();
        let set = |area: &str, tag: &str| Fix {
            label: format!("Set {tag}"),
            edits: vec![Edit::SetField {
                key: key(&format!("{area}.git")),
                path: GffPath::root().item("StoreList", 0),
                label: "Tag".into(),
                value: Some(Value::String(tag.as_bytes().to_vec())),
            }],
            asks: None,
        };
        let (a, b, c) = (set("inn", "A"), set("inn", "B"), set("mill", "C"));
        let removal = Fix {
            label: "Remove".into(),
            edits: vec![Edit::remove_item(key("mill.git"), GffPath::root(), "StoreList", 0)],
            asks: None,
        };
        let inserted = Fix {
            label: "Insert".into(),
            edits: vec![Edit::insert_item(
                key("mill.git"),
                GffPath::root(),
                "StoreList",
                0,
                tagged("X"),
            )],
            asks: None,
        };
        let mut guard = Guard::new(&m, [&a, &b, &c, &removal, &inserted]);
        let mut ws = Workspace::new(m);
        // Fixes that set fields go together.
        assert_eq!(guard.check(&mut ws, &a).unwrap(), a);
        ws.apply(a.command()).unwrap();
        guard.made(&mut ws, &a);
        guard.check(&mut ws, &b).unwrap();
        // One that takes an item out leaves those of what it took out
        // refused, and the others' paths followed.
        guard.check(&mut ws, &removal).unwrap();
        ws.apply(removal.command()).unwrap();
        guard.made(&mut ws, &removal);
        assert!(guard.check(&mut ws, &c).unwrap_err().contains("another fix took out"));
        assert!(guard.check(&mut ws, &removal).unwrap_err().contains("another fix took out"));
        // One that adds an item leaves the others of its resource
        // refused: their paths may lead elsewhere now.
        guard.check(&mut ws, &inserted).unwrap();
        ws.apply(inserted.command()).unwrap();
        guard.made(&mut ws, &inserted);
        assert!(guard.check(&mut ws, &c).unwrap_err().contains("mill.git has changed"));
        // As is a resource changed by anything else.
        let other = set("inn", "Z");
        ws.apply(other.command()).unwrap();
        assert!(guard.check(&mut ws, &b).is_err());
    }

    /// Verify's findings of a module, with their fixes, and the module
    /// once all those of `check` are made (each followed as Verify's
    /// window and `mg verify --fix` make them).
    fn fixed(m: Module, check: &str) -> (Vec<Found>, Workspace) {
        let findings = mg_module::doctor::examine(
            &m,
            &mg_resman::ResMan::new(),
            mg_module::doctor::TalkTables::default(),
        );
        let found: Vec<Found> =
            attach(&m, findings, &[]).into_iter().filter(|f| f.finding.check == check).collect();
        let mut guard = Guard::new(&m, found.iter().filter_map(|f| f.fix.as_ref()));
        let mut ws = Workspace::new(m);
        for fix in found.iter().filter_map(|f| f.fix.as_ref()) {
            let now = guard.check(&mut ws, fix).unwrap();
            ws.apply(now.command()).unwrap();
            guard.made(&mut ws, fix);
        }
        (found, ws)
    }

    fn var(name: &str, value: i32) -> Struct {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(name.as_bytes().to_vec()));
        v.set("Type", Value::Dword(1));
        v.set("Value", Value::Int(value));
        v
    }

    fn names(vars: &[Struct]) -> Vec<String> {
        vars.iter().map(|v| String::from_utf8_lossy(v.string("Name").unwrap()).into()).collect()
    }

    /// `var-runtime`: each variable the scripts alone set, kept in a
    /// file, is taken out, two of one `VarTable` (and two of one name)
    /// alike: the second fix follows the first's places (the Domains of
    /// Dread's setup on 2.1.0).
    #[test]
    fn a_variable_of_the_scripts_is_taken_out_of_its_table() {
        let mut m = module();
        m.project_file.read_variables(
            "variables.toml",
            "[tdod_keep]\ntype = \"int\"\n\n[\"tdod_*_d20\"]\nruntime = true\n",
        );
        let git = key("inn.git");
        let mut g = m.gff(&git).unwrap().unwrap();
        let mut chair = tagged("CHAIR");
        chair.set(
            "VarTable",
            Value::List(vec![
                var("tdod_a_d20", 20),
                var("tdod_keep", 1),
                var("tdod_b_d20", 1),
                var("tdod_a_d20", 3),
                var("tdod_keep", 2),
            ]),
        );
        g.root.set("Placeable List", Value::List(vec![tagged("TABLE"), chair]));
        g.root.set("VarTable", Value::List(vec![var("tdod_c_d20", 5), var("tdod_keep", 3)]));
        m.set_gff(git, &g).unwrap();

        let (found, mut ws) = fixed(m, "var-runtime");
        assert_eq!(found.len(), 4, "{found:#?}");
        assert!(found.iter().all(|f| f.fix.as_ref().is_some_and(|f| f.label == REMOVE_VARIABLE)));
        let g = ws.doc(&git).unwrap();
        let chair = &g.root.list("Placeable List").unwrap()[1];
        assert_eq!(names(chair.list("VarTable").unwrap()), ["tdod_keep", "tdod_keep"]);
        let kept: Vec<i32> = (chair.list("VarTable").unwrap().iter())
            .map(|v| match v.get("Value") {
                Some(Value::Int(i)) => *i,
                _ => -1,
            })
            .collect();
        assert_eq!(kept, [1, 2], "the right ones kept");
        assert_eq!(names(g.root.list("VarTable").unwrap()), ["tdod_keep"]);
        assert_eq!(
            names(g.root.list("Placeable List").unwrap()[0].list("VarTable").unwrap_or(&[])),
            Vec::<String>::new()
        );
    }

    /// `static-transform`: the transform off, as ticking Static takes it
    /// off; the rest of the placeable kept (xtul's report, October 2026).
    #[test]
    fn a_static_placeable_s_transform_is_taken_off() {
        let mut m = module();
        let transformed = |tag: &str| {
            let mut p = tagged(tag);
            p.set("Static", Value::Byte(1));
            let mut scale = Struct::new(0);
            scale.set("ValueTo", Value::Float(0.5));
            let mut entry = Struct::new(6);
            entry.set("Scope", Value::Int(0));
            entry.set("ScaleX", Value::Struct(scale));
            p.set("VisTransformList", Value::List(vec![entry]));
            p
        };
        let git = key("mill.git");
        let mut g = m.gff(&git).unwrap().unwrap();
        // (Two of one tag, said alike: each fix its own.)
        g.root.set(
            "Placeable List",
            Value::List(vec![
                tagged("PLAIN"),
                transformed("MIRROR"),
                transformed("LAMP"),
                transformed("LAMP"),
            ]),
        );
        m.set_gff(git, &g).unwrap();
        let (found, mut ws) = fixed(m, "static-transform");
        let labels: Vec<_> =
            found.iter().map(|f| f.fix.as_ref().map(|f| f.label.as_str())).collect();
        assert_eq!(labels, [Some(TAKE_TRANSFORM_OFF); 3]);
        assert_eq!(found[1].finding.at, found[2].finding.at);
        let list = ws.doc(&git).unwrap().root.list("Placeable List").unwrap().to_vec();
        assert!(list.iter().all(|p| !p.contains("VisTransformList")), "{list:?}");
        assert_eq!(list[1].integer("Static"), Some(1), "still static");
        assert_eq!(list[2].string("Tag"), Some(&b"LAMP"[..]));
    }

    #[test]
    fn the_plugins_fixes_go_with_their_findings() {
        let m = module();
        let mine = finding("mine/rule", "inn.git", "here");
        let fix = Fix { label: "Do it".into(), edits: Vec::new(), asks: None };
        let plugins = [Found { finding: mine.clone(), fix: Some(fix.clone()) }];
        let found = attach(&m, vec![finding("other", "inn.git", ""), mine], &plugins);
        assert_eq!(found[0].fix, None);
        assert_eq!(found[1].fix, Some(fix));
    }
}
