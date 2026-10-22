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

use crate::{Command, Edit, GffPath, Workspace};

/// A finding's fix: what its button says, and the edits it makes.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub label: String,
    pub edits: Vec<Edit>,
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

    /// Whether it only sets and removes fields: then the paths of another
    /// fix of the same resources still lead where they did.
    fn sets_fields_only(&self) -> bool {
        self.edits.iter().all(|e| matches!(e, Edit::SetField { .. }))
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
    let mut tags: Option<TagIndex> = None;
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
                None => doctor_fix_with(module, &finding, &mut tags),
            };
            Found { finding, fix }
        })
        .collect()
}

/// The fix of one of the doctor's findings, where one is plain:
/// `tag-unique` retags the objects after the first (`TAG_2`, `TAG_3`…;
/// an area keeps its tag), `area-variables` moves an area's variables
/// from its ARE to its GIT. Others have none (`pc-faction`: which faction
/// is the builder's to choose).
pub fn doctor_fix(module: &Module, finding: &Finding) -> Option<Fix> {
    doctor_fix_with(module, finding, &mut None)
}

fn doctor_fix_with(module: &Module, f: &Finding, tags: &mut Option<TagIndex>) -> Option<Fix> {
    if f.source != "module" {
        return None;
    }
    match f.check.as_ref() {
        "tag-unique" => {
            let index = tags.get_or_insert_with(|| TagIndex::of_module(module));
            retag_fix(index, f)
        }
        "area-variables" => move_variables_fix(module, f.resource),
        _ => None,
    }
}

/// `tag-unique`: the places of the tag the finding is about (found as the
/// doctor found them: its first place is the finding's resource, and `at`
/// lists them all), each but one given a tag of its own.
fn retag_fix(index: &TagIndex, f: &Finding) -> Option<Fix> {
    let (tag, places) = index.duplicates().into_iter().find(|(_, places)| {
        places[0].key() == f.resource
            && places.iter().map(|p| p.describe()).collect::<Vec<_>>().join(", ") == f.at
    })?;
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
    Some(Fix { label, edits })
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
    Some(Fix { label: "Move the variables to the GIT".into(), edits })
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

/// What a Verify's fixes were worked out on: each resource one of them
/// touches, as Verify read it. A fix is made only while each of its
/// resources is as it was then, or has changed since only by fixes of the
/// same Verify that set fields (whose paths stay good); otherwise Verify
/// is to run again. So a fix never lands on another object than the one
/// it was found on.
#[derive(Debug, Clone, Default)]
pub struct Guard {
    /// Each resource's bytes' digest as a fix may find them; a resource a
    /// fix changed otherwise than by setting fields is gone from here.
    seen: HashMap<ResKey, Option<u64>>,
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
        Guard { seen }
    }

    /// Whether `fix` may be made on the module of `ws` now; else why not.
    pub fn check(&self, ws: &mut Workspace, fix: &Fix) -> Result<(), String> {
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
        Ok(())
    }

    /// Notes that `fix` was made on the module of `ws`: its resources as
    /// they are now are those the other fixes may find, if it only set
    /// fields; else they refuse them.
    pub fn made(&mut self, ws: &mut Workspace, fix: &Fix) {
        let flushed = ws.flush().is_ok();
        for key in fix.resources() {
            if flushed && fix.sets_fields_only() {
                self.seen.insert(key, digest(ws.module.get(&key)));
            } else {
                self.seen.remove(&key);
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
        };
        let (a, b, c) = (set("inn", "A"), set("inn", "B"), set("mill", "C"));
        let removal = Fix {
            label: "Remove".into(),
            edits: vec![Edit::remove_item(key("mill.git"), GffPath::root(), "StoreList", 0)],
        };
        let mut guard = Guard::new(&m, [&a, &b, &c, &removal]);
        let mut ws = Workspace::new(m);
        // Fixes that set fields go together.
        guard.check(&mut ws, &a).unwrap();
        ws.apply(a.command()).unwrap();
        guard.made(&mut ws, &a);
        guard.check(&mut ws, &b).unwrap();
        // One that removes an item leaves the others of its resource
        // refused: their paths may lead elsewhere now.
        guard.check(&mut ws, &removal).unwrap();
        ws.apply(removal.command()).unwrap();
        guard.made(&mut ws, &removal);
        assert!(guard.check(&mut ws, &c).unwrap_err().contains("mill.git has changed"));
        // As is a resource changed by anything else.
        let other = set("inn", "Z");
        ws.apply(other.command()).unwrap();
        assert!(guard.check(&mut ws, &b).is_err());
    }

    #[test]
    fn the_plugins_fixes_go_with_their_findings() {
        let m = module();
        let mine = finding("mine/rule", "inn.git", "here");
        let fix = Fix { label: "Do it".into(), edits: Vec::new() };
        let plugins = [Found { finding: mine.clone(), fix: Some(fix.clone()) }];
        let found = attach(&m, vec![finding("other", "inn.git", ""), mine], &plugins);
        assert_eq!(found[0].fix, None);
        assert_eq!(found[1].fix, Some(fix));
    }
}
