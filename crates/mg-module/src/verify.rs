//! Module verification (Aurora's Build › Verify passes that need no
//! compiler): references to resources that exist nowhere, and module
//! resources nothing references.

use std::collections::{BTreeSet, HashSet};

use mg_core::ResType;
use mg_resman::{ResKey, ResMan, priority};
use mg_rules::ObjectKind;

use crate::Module;
use crate::refs::{Location, RefKind, Reference, references};

/// Which object type a finding belongs to, following Aurora's verify
/// filters (Creatures, Doors, Placeables, Items, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Module,
    Areas,
    Creatures,
    Doors,
    Encounters,
    Items,
    Placeables,
    Sounds,
    Stores,
    Triggers,
    Waypoints,
    Conversations,
    Scripts,
    Other,
}

impl Category {
    pub const ALL: [Category; 14] = [
        Category::Module,
        Category::Areas,
        Category::Creatures,
        Category::Doors,
        Category::Encounters,
        Category::Items,
        Category::Placeables,
        Category::Sounds,
        Category::Stores,
        Category::Triggers,
        Category::Waypoints,
        Category::Conversations,
        Category::Scripts,
        Category::Other,
    ];

    /// Its id, as Verify leaves it out (`mg verify --skip
    /// references:creatures`, Options › Verify): its name in lower case.
    pub fn id(self) -> String {
        format!("{self:?}").to_ascii_lowercase()
    }

    pub fn from_id(id: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.id().eq_ignore_ascii_case(id.trim()))
    }

    /// The category of a reference, from its source and path (area
    /// instances belong to their object type).
    pub fn of(r: &Reference) -> Category {
        let list = r.path.gff().and_then(mg_gff::GffPath::first).map_or("", mg_gff::Step::label);
        match r.from.restype {
            ResType::GIT => ObjectKind::from_list(list).map_or(Category::Areas, Category::from),
            t => Category::of_type(t),
        }
    }

    pub fn of_type(t: ResType) -> Category {
        match t {
            ResType::IFO => Category::Module,
            ResType::ARE | ResType::GIT | ResType::GIC => Category::Areas,
            ResType::DLG => Category::Conversations,
            ResType::NSS | ResType::NCS => Category::Scripts,
            t => ObjectKind::from_restype(t).map_or(Category::Other, Category::from),
        }
    }
}

impl From<ObjectKind> for Category {
    /// A type of placed object's or blueprint's category.
    fn from(kind: ObjectKind) -> Category {
        match kind {
            ObjectKind::Creature => Category::Creatures,
            ObjectKind::Door => Category::Doors,
            ObjectKind::Encounter => Category::Encounters,
            ObjectKind::Item => Category::Items,
            ObjectKind::Placeable => Category::Placeables,
            ObjectKind::Sound => Category::Sounds,
            ObjectKind::Store => Category::Stores,
            ObjectKind::Trigger => Category::Triggers,
            ObjectKind::Waypoint => Category::Waypoints,
        }
    }
}

/// A reference nothing satisfies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    pub reference: Reference,
    pub category: Category,
    /// For scripts: the source exists but not the compiled script, which the
    /// game needs.
    pub uncompiled: bool,
    /// It is there, but only in the player's own content ([`Local`]: the
    /// haks the user's `userpatch.ini` names, the user's `override/`, a
    /// Workshop item): a player without the same has nothing in its place.
    pub only_local: Option<Local>,
}

/// Content of the user's own, which a module's players have only if they
/// put the same there themselves: what a module may not count on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Local {
    /// A hak the user's `userpatch.ini` names.
    Userpatch,
    /// The user's `override/` folder.
    Override,
    /// A Steam Workshop item the user subscribed to.
    Workshop,
}

impl Local {
    /// The player's own content a resource manager layer holds, by its
    /// priority. (The user's `development/` folder is not counted: it is
    /// where `mg watch` and the builder put the module's own work to try.)
    pub fn of_priority(p: u32) -> Option<Local> {
        match p {
            priority::USERPATCH => Some(Local::Userpatch),
            priority::OVERRIDE => Some(Local::Override),
            priority::WORKSHOP => Some(Local::Workshop),
            _ => None,
        }
    }

    /// Its name in JSON: `userpatch`, `override`, `workshop`.
    pub fn id(self) -> &'static str {
        match self {
            Local::Userpatch => "userpatch",
            Local::Override => "override",
            Local::Workshop => "workshop",
        }
    }

    /// Where it is, for people: "userpatch.ini's haks", "your override
    /// folder", "a Steam Workshop item".
    pub fn place(self) -> &'static str {
        match self {
            Local::Userpatch => "userpatch.ini's haks",
            Local::Override => "your override folder",
            Local::Workshop => "a Steam Workshop item",
        }
    }
}

/// Whether only the player's own content has `key` (not the module, its
/// haks or the game): the highest of it that has it. `None` when the
/// module or another layer has it too, or nothing has.
pub fn only_local(module: &Module, resman: &ResMan, key: &ResKey) -> Option<Local> {
    if module.contains(key) {
        return None;
    }
    let found = Local::of_priority(resman.layers()[resman.find(key)?].priority)?;
    let elsewhere = resman
        .layers()
        .iter()
        .any(|l| Local::of_priority(l.priority).is_none() && l.container.contains(key));
    (!elsewhere).then_some(found)
}

impl Missing {
    /// Whether it breaks the module as it plays: references to scripts,
    /// conversations, areas and the like from areas, placed objects, the
    /// module, conversations and scripts. A blueprint's
    /// matter only once something places it, and an object placed from a
    /// blueprint that is gone keeps working (it holds all it needs), so those
    /// are warnings.
    /// Whether it is an object placed in an area from a blueprint the
    /// module no longer has (nor its haks or the game): common, even in
    /// BioWare's modules, and harmless as it plays; only placing more and
    /// Update Instances need the blueprint.
    pub fn placed_from_gone_blueprint(&self) -> bool {
        let r = &self.reference;
        r.from.restype == ResType::GIT
            && r.path.field() == Some("TemplateResRef")
            && matches!(r.kind, RefKind::Blueprint(_))
    }

    /// What is wrong with it, after its name: "missing", "not compiled",
    /// or found only in the player's own content ("only in userpatch.ini's
    /// haks …", "only in your override folder …").
    pub fn what(&self) -> &'static str {
        match self.only_local {
            Some(Local::Userpatch) => {
                "only in userpatch.ini's haks (players without them won't have it)"
            }
            Some(Local::Override) => {
                "only in your override folder (players without the same won't have it)"
            }
            Some(Local::Workshop) => {
                "only in a Steam Workshop item (players not subscribed won't have it)"
            }
            None if self.uncompiled => "not compiled",
            None => "missing",
        }
    }

    pub fn is_error(&self) -> bool {
        if self.only_local.is_some() {
            return false;
        }
        let r = &self.reference;
        // A missing sound, portrait or movie leaves silence or a blank.
        if matches!(r.kind, RefKind::Sound | RefKind::Portrait | RefKind::Movie) {
            return false;
        }
        match r.from.restype {
            ResType::GIT => r.path.field() != Some("TemplateResRef"),
            ResType::ARE | ResType::IFO | ResType::DLG | ResType::NSS => true,
            _ => false,
        }
    }
}

/// Every reference in the module.
pub fn module_references(module: &Module) -> Vec<Reference> {
    module.keys().flat_map(|k| references(*k, module.get(k).unwrap_or_default())).collect()
}

/// References that neither the module nor `resman` (the game, haks, and
/// usually the module layer too) can satisfy. Scripts need their compiled
/// form; one with only a source is reported as uncompiled. A reference
/// only the player's own content satisfies ([`Local`]: userpatch.ini's
/// haks, `override/`, a Workshop item) is reported too, as such (a
/// warning).
pub fn missing(module: &Module, resman: &ResMan) -> Vec<Missing> {
    missing_of(module, resman, module_references(module))
}

/// [`missing`] among the module's references already gathered
/// ([`module_references`]: every GFF read; [`unused_of`] takes them too).
pub fn missing_of(module: &Module, resman: &ResMan, references: Vec<Reference>) -> Vec<Missing> {
    let has = |k: &ResKey| module.contains(k) || resman.contains(k);
    // Found, but only in the player's own content (userpatch.ini's haks,
    // override/, a Workshop item: not the module's): nothing else in the
    // stack has it.
    let local = resman.layers().iter().any(|l| Local::of_priority(l.priority).is_some());
    let only_local = |k: &ResKey| if local { only_local(module, resman, k) } else { None };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for r in references {
        let candidates = r.candidates();
        // The game runs compiled scripts; a source alone does not satisfy a
        // script reference (but is reported as uncompiled, not missing).
        let satisfying: Vec<&ResKey> = match r.kind {
            RefKind::Script => {
                candidates.iter().filter(|k| k.restype == ResType::NCS && has(k)).collect()
            }
            _ => candidates.iter().filter(|k| has(k)).collect(),
        };
        // (Of several that would do, each only the player's own: the first's
        // place is said.)
        let locals: Vec<Option<Local>> = satisfying.iter().map(|k| only_local(k)).collect();
        let local_only =
            if !locals.is_empty() && locals.iter().all(Option::is_some) { locals[0] } else { None };
        if candidates.is_empty() || (!satisfying.is_empty() && local_only.is_none()) {
            continue;
        }
        // Each missing target once per referencing resource (a dialog may
        // use the same script on hundreds of nodes).
        if !seen.insert((r.from, r.kind, r.target)) {
            continue;
        }
        let uncompiled = local_only.is_none()
            && r.kind == RefKind::Script
            && candidates.iter().any(|k| k.restype == ResType::NSS && has(k));
        out.push(Missing {
            category: Category::of(&r),
            reference: r,
            uncompiled,
            only_local: local_only,
        });
    }
    out
}

/// Which of a script's two files the module has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScriptFiles {
    /// Its source (`.nss`) and its compiled script (`.ncs`).
    Both,
    /// Its compiled script alone.
    CompiledOnly,
    /// Its source alone.
    SourceOnly,
}

impl ScriptFiles {
    /// What the report says of it.
    pub fn words(self) -> &'static str {
        match self {
            ScriptFiles::Both => "source and compiled",
            ScriptFiles::CompiledOnly => "compiled only (no source)",
            ScriptFiles::SourceOnly => "source only (not compiled)",
        }
    }

    /// Its name in `mg verify --json`: `both`, `compiled`, `source`.
    pub fn id(self) -> &'static str {
        match self {
            ScriptFiles::Both => "both",
            ScriptFiles::CompiledOnly => "compiled",
            ScriptFiles::SourceOnly => "source",
        }
    }
}

/// A resource of the module that nothing uses: a blueprint, a
/// conversation, or a script, once for its source and its compiled
/// script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Unused {
    /// The resource: of a script, its source where the module has it,
    /// else its compiled script.
    pub key: ResKey,
    /// Of a script, which of its files the module has.
    pub script: Option<ScriptFiles>,
}

impl Unused {
    /// The module's files of it (a script's source and compiled script).
    pub fn files(&self) -> Vec<ResKey> {
        let r = self.key.resref;
        match self.script {
            Some(ScriptFiles::Both) => {
                vec![ResKey::new(r, ResType::NSS), ResKey::new(r, ResType::NCS)]
            }
            _ => vec![self.key],
        }
    }
}

impl std::fmt::Display for Unused {
    /// `name.nss (source and compiled)`, `name.utc`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.script {
            Some(s) => write!(f, "{} ({})", self.key, s.words()),
            None => write!(f, "{}", self.key),
        }
    }
}

/// The scripts the game's tables name in the columns the engine runs a
/// script from ([`crate::doctor::SCRIPT_COLUMNS`]: a spell's impact
/// script, an area of effect's, a state's, a trap's, a disease's and a
/// poison's), each table as the module sees it (`table`: its own and its
/// haks' over the game's): references from the table (`spells.2da`) at
/// `row 12, ImpactScript`.
pub fn table_scripts(table: &crate::doctor::TableOf) -> Vec<Reference> {
    let mut out = Vec::new();
    let mut tables: Vec<&str> = crate::doctor::SCRIPT_COLUMNS.iter().map(|(t, _)| *t).collect();
    tables.dedup();
    for name in tables {
        let (Some(t), Some(from)) = (table(name), ResKey::parse(name, ResType::TWODA)) else {
            continue;
        };
        let columns = crate::doctor::SCRIPT_COLUMNS.iter().filter(|(n, _)| *n == name);
        for (_, column) in columns {
            for row in 0..t.len() {
                let Some(cell) = t.get(row, column) else { continue };
                let Ok(target) = mg_core::ResRef::from_str(cell.trim()) else { continue };
                if target.is_empty() {
                    continue;
                }
                let path = Location::Cell { row, column: column.to_string() };
                out.push(Reference { from, path, kind: RefKind::Script, target });
            }
        }
    }
    out
}

/// [`table_scripts`] of the tables `rm` finds (the game's, under the haks
/// and the module where those are stacked on it).
pub fn table_scripts_in(rm: &ResMan) -> Vec<Reference> {
    let read = |name: &str| {
        let data = rm.get(&ResKey::parse(name, ResType::TWODA)?).ok()?;
        mg_2da::TwoDa::parse(&data, mg_core::Codepage::default()).ok().map(std::sync::Arc::new)
    };
    table_scripts(&read)
}

/// The scripts the module's own 2DAs name in the columns their
/// descriptions say hold a script's name (`kind = "script"` in
/// `name.2da.toml`: a table of the module's that a script reads with
/// `Get2DAString` and runs with `ExecuteScript`). A description's lists
/// of a script's constants read the module's script, else the one `rm`
/// finds (the game's `nwscript`).
pub fn described_scripts(module: &Module, rm: Option<&ResMan>) -> Vec<Reference> {
    use crate::table_desc::{Description, Kind, path_in_module};
    let mut out = Vec::new();
    let mut keys: Vec<ResKey> = module.keys_of(ResType::TWODA).copied().collect();
    keys.sort();
    for key in keys {
        let Some(text) = path_in_module(module, &key).and_then(|p| std::fs::read_to_string(p).ok())
        else {
            continue;
        };
        let script = |name: &str| {
            let k = ResKey::parse(name, ResType::NSS)?;
            module
                .get(&k)
                .map(<[u8]>::to_vec)
                .or_else(|| rm.and_then(|rm| rm.get(&k).ok()).map(|d| d.into_owned()))
        };
        let Ok(d) = Description::parse(&text, &script) else { continue };
        let data = module.get(&key).unwrap_or_default();
        let Ok(t) = mg_2da::TwoDa::parse(data, mg_core::Codepage::default()) else { continue };
        for (column, c) in &d.columns {
            if c.kind != Kind::Script {
                continue;
            }
            for row in 0..t.len() {
                let Some(cell) = t.get(row, column) else { continue };
                let Ok(target) = mg_core::ResRef::from_str(cell.trim()) else { continue };
                if !target.is_empty() {
                    let path = Location::Cell { row, column: column.to_string() };
                    out.push(Reference { from: key, path, kind: RefKind::Script, target });
                }
            }
        }
    }
    out
}

/// What a script's strings may name, as references (`string`): a
/// script, a conversation, a blueprint of any type.
fn string_references(from: ResKey, source: &[u8]) -> Vec<Reference> {
    let kinds = std::iter::once(RefKind::Script)
        .chain(std::iter::once(RefKind::Conversation))
        .chain(ObjectKind::ALL.iter().map(|k| RefKind::Blueprint(k.restype())));
    let kinds: Vec<RefKind> = kinds.collect();
    let mut out = Vec::new();
    for target in crate::refs::script_strings(source) {
        if target == from.resref {
            continue;
        }
        for kind in &kinds {
            out.push(Reference { from, path: Location::String, kind: *kind, target });
        }
    }
    out
}

/// What the module reaches outside it names, however far: a conversation
/// of a hak that a placed creature has, or a hak's blueprint a string in
/// a script spells (`CreateObject`), and the scripts, conversations and
/// blueprints those name in turn (`rm`: the game, the haks and the module
/// stacked). Kingmaker keeps its conversations and blueprints in its hak,
/// and they run the module's scripts.
pub fn outside_references(
    module: &Module,
    references: &[Reference],
    rm: &ResMan,
) -> Vec<Reference> {
    let mut seen: HashSet<ResKey> = HashSet::new();
    let mut queue: Vec<ResKey> = Vec::new();
    let mut reach = |refs: &[Reference], queue: &mut Vec<ResKey>| {
        for k in refs.iter().flat_map(Reference::candidates) {
            let followed = k.restype.is_gff() || k.restype == ResType::NSS;
            if followed && !module.contains(&k) && seen.insert(k) && rm.contains(&k) {
                queue.push(k);
            }
        }
    };
    reach(references, &mut queue);
    for k in module.keys_of(ResType::NSS) {
        reach(&string_references(*k, module.get(k).unwrap_or_default()), &mut queue);
    }
    let mut out = Vec::new();
    while let Some(k) = queue.pop() {
        let Ok(data) = rm.get(&k) else { continue };
        let mut found = crate::refs::references(k, &data);
        if k.restype == ResType::NSS {
            found.extend(string_references(k, &data));
        }
        reach(&found, &mut queue);
        out.extend(found);
    }
    out
}

/// What counts as a use of the module's resources beside what the module
/// names ([`unused_of`]'s `also`): the scripts the game's tables run
/// ([`table_scripts_in`]), those the module's described tables name
/// ([`described_scripts`]) and what the module reaches outside it names
/// ([`outside_references`]), by `rm` (the game, the haks and the module
/// stacked).
pub fn also_used(module: &Module, references: &[Reference], rm: &ResMan) -> Vec<Reference> {
    let mut also = table_scripts_in(rm);
    also.extend(described_scripts(module, Some(rm)));
    // (From the scripts the tables run too: the game's Dirge, x0_s0_dirge,
    // names its area of effect's scripts, x0_s0_dirgeen...)
    let from: Vec<Reference> = references.iter().chain(&also).cloned().collect();
    also.extend(outside_references(module, &from, rm));
    also
}

/// Module resources of the given kinds that nothing in the module
/// references: blueprints, conversations and scripts (a compiled script
/// and its source are one script). One is used when a field names it, a
/// script `#include`s it, a string in a script spells its name (whatever
/// the string is passed to: `ExecuteScript("x")`, `SetEventScript`,
/// `CreateObject`...; a script's own name in it doesn't count) or the
/// module's project file counts it as used (`[used]`). A name made as the
/// script runs (`"ai_" + sMode`) is not seen. (Without the game's tables:
/// [`unused_of`] takes the scripts they run.)
pub fn unused(module: &Module) -> Vec<Unused> {
    unused_of(module, &module_references(module), &[])
}

/// [`unused`] by the module's references already gathered, and with the
/// scripts the game's tables run as the module sees them
/// ([`table_scripts`]) counted as used.
pub fn unused_of(module: &Module, references: &[Reference], tables: &[Reference]) -> Vec<Unused> {
    let mut used: HashSet<(mg_core::ResRef, ResType)> = HashSet::new();
    for r in references.iter().chain(tables) {
        for k in r.candidates() {
            used.insert((k.resref, k.restype));
        }
    }
    let checked = [
        ResType::UTC,
        ResType::UTD,
        ResType::UTE,
        ResType::UTI,
        ResType::UTM,
        ResType::UTP,
        ResType::UTS,
        ResType::UTT,
        ResType::UTW,
        ResType::DLG,
        ResType::NSS,
        ResType::NCS,
    ];
    // A script's strings that spell a name: another script's
    // (`ExecuteScript("x")`; not its own, as a script naming itself runs
    // only once something else runs it), a blueprint's (`CreateObject`), a
    // conversation's.
    let names: HashSet<mg_core::ResRef> =
        module.keys().filter(|k| checked.contains(&k.restype)).map(|k| k.resref).collect();
    for k in module.keys_of(ResType::NSS) {
        let Some(src) = module.get(k) else { continue };
        for name in crate::refs::script_strings(src) {
            if !names.contains(&name) {
                continue;
            }
            for t in checked {
                if name != k.resref || !matches!(t, ResType::NSS | ResType::NCS) {
                    used.insert((name, t));
                }
            }
        }
    }
    let marked = &module.project_file.used;
    let mut out = BTreeSet::new();
    for k in module.keys().filter(|k| checked.contains(&k.restype)) {
        let script = matches!(k.restype, ResType::NSS | ResType::NCS);
        let is_used = if script {
            used.contains(&(k.resref, ResType::NCS)) || used.contains(&(k.resref, ResType::NSS))
        } else {
            used.contains(&(k.resref, k.restype))
        };
        if is_used || marked.marking(k.resref, k.restype).is_some() {
            continue;
        }
        if !script {
            out.insert(Unused { key: *k, script: None });
            continue;
        }
        // A script once: by its source where there is one.
        let source = ResKey::new(k.resref, ResType::NSS);
        let compiled = ResKey::new(k.resref, ResType::NCS);
        let files = match (module.contains(&source), module.contains(&compiled)) {
            (true, true) => ScriptFiles::Both,
            (true, false) => ScriptFiles::SourceOnly,
            _ => ScriptFiles::CompiledOnly,
        };
        let key = if files == ScriptFiles::CompiledOnly { compiled } else { source };
        out.insert(Unused { key, script: Some(files) });
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use mg_core::ResRef;
    use mg_gff::Gff;
    use mg_resman::{LayerClass, MemContainer, priority};
    use mg_schema::{StructExt, git, ifo};

    use super::*;

    #[test]
    fn a_placed_object_is_in_its_blueprints_category() {
        for k in ObjectKind::ALL {
            let r = Reference {
                from: ResKey::parse("area1", ResType::GIT).unwrap(),
                path: Location::Gff(mg_gff::GffPath::root().item(k.list(), 2).field("OnClick")),
                kind: RefKind::Script,
                target: rr("x"),
            };
            assert_eq!(Category::of(&r), Category::of_type(k.restype()), "{k:?}");
            assert_eq!(format!("{:?}", Category::from(k)), k.label(), "as Aurora's filters");
        }
        let area = Reference {
            from: ResKey::parse("area1", ResType::GIT).unwrap(),
            path: Location::Gff(mg_gff::GffPath::root().field("AreaProperties").field("MusicDay")),
            kind: RefKind::Script,
            target: rr("x"),
        };
        assert_eq!(Category::of(&area), Category::Areas);
        assert_eq!(Category::of_type(ResType::NCS), Category::Scripts);
        assert_eq!(Category::of_type(ResType::TWODA), Category::Other);
    }

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn key(s: &str, t: ResType) -> ResKey {
        ResKey::new(rr(s), t)
    }

    #[test]
    fn missing_and_unused() {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.write(&ifo::MOD_ON_MOD_LOAD, rr("base_script"));
        ifo.root.write(&ifo::MOD_ON_HEARTBEAT, rr("nowhere"));
        ifo.root.write(&ifo::MOD_ON_CLIENT_ENTR, rr("src_only"));
        m.set_info(&ifo).unwrap();
        let mut g = Gff::new(*b"GIT ");
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::TEMPLATE_RES_REF, rr("used_bp"));
        c.write(&git::creature_list::CONVERSATION, rr("gone_dlg"));
        g.root.items_mut(&git::CREATURE_LIST).push(c);
        m.set_gff(key("area1", ResType::GIT), &g).unwrap();
        for (n, t) in [
            ("used_bp", ResType::UTC),
            ("spare_bp", ResType::UTC),
            ("src_only", ResType::NSS),
            ("lonely", ResType::NCS),
        ] {
            m.set(key(n, t), Vec::new());
        }
        let mut base = MemContainer::new();
        base.insert(key("base_script", ResType::NCS), Vec::new());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, base);

        let miss = missing(&m, &rm);
        let got: Vec<(String, Category, bool)> = miss
            .iter()
            .map(|m| (m.reference.target.to_string(), m.category, m.uncompiled))
            .collect();
        assert_eq!(
            got,
            [
                ("nowhere".into(), Category::Module, false),
                ("src_only".into(), Category::Module, true),
                ("gone_dlg".into(), Category::Creatures, false),
            ]
        );
        let unused: Vec<String> = unused(&m).iter().map(|k| k.to_string()).collect();
        assert_eq!(unused, ["lonely.ncs (compiled only (no source))", "spare_bp.utc"]);
    }

    /// A script is said once, with the files the module has of it.
    #[test]
    fn a_script_is_one_line_with_its_files() {
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        for (n, t) in [
            ("both", ResType::NSS),
            ("both", ResType::NCS),
            ("compiled", ResType::NCS),
            ("source", ResType::NSS),
        ] {
            m.set(key(n, t), b"void main() {}".to_vec());
        }
        let got: Vec<(String, Option<ScriptFiles>)> =
            unused(&m).iter().map(|u| (u.key.to_string(), u.script)).collect();
        assert_eq!(
            got,
            [
                ("both.nss".into(), Some(ScriptFiles::Both)),
                ("compiled.ncs".into(), Some(ScriptFiles::CompiledOnly)),
                ("source.nss".into(), Some(ScriptFiles::SourceOnly)),
            ]
        );
        let u = unused(&m);
        assert_eq!(u[0].to_string(), "both.nss (source and compiled)");
        assert_eq!(u[0].files(), [key("both", ResType::NSS), key("both", ResType::NCS)]);
        assert_eq!(u[1].to_string(), "compiled.ncs (compiled only (no source))");
        assert_eq!(u[2].to_string(), "source.nss (source only (not compiled))");
    }

    fn table(text: &str) -> Vec<u8> {
        text.replace('\n', "\r\n").into_bytes()
    }

    /// A script the game runs from a table (a spell's, an area of
    /// effect's, a state's...) is used, by the table the module sees: the
    /// module's own over its haks' over the game's.
    #[test]
    fn a_script_a_table_runs_is_used_by_the_table_the_module_sees() {
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        for n in ["sp_fire", "sp_old", "sp_new", "aoe_in", "aoe_hb", "st_fear", "lonely"] {
            m.set(key(n, ResType::NCS), Vec::new());
        }
        // The game's: two spells and a state.
        let mut base = MemContainer::new();
        base.insert(
            key("spells", ResType::TWODA),
            table(
                "2DA V2.0\n\n   Label   ImpactScript\n0  Fire  sp_fire\n1  Old   sp_old\n2  None  ****\n",
            ),
        );
        base.insert(
            key("statescripts", ResType::TWODA),
            table("2DA V2.0\n\n   STATENAME  SCRIPTNAME\n0  Normal  ****\n1  Fear  ST_FEAR\n"),
        );
        // A hak's areas of effect.
        let mut hak = MemContainer::new();
        hak.insert(
            key("vfx_persistent", ResType::TWODA),
            table(
                "2DA V2.0\n\n   LABEL  ONENTER  ONEXIT  HEARTBEAT\n0  Cloud  aoe_in  ****  aoe_hb\n",
            ),
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, base);
        rm.add(priority::HAK, "hak:x", LayerClass::Erf, hak);
        let names = |rm: &ResMan, m: &Module| -> Vec<String> {
            let refs = module_references(m);
            let tables = table_scripts_in(rm);
            unused_of(m, &refs, &tables).iter().map(|u| u.key.resref.to_string()).collect()
        };
        assert_eq!(names(&rm, &m), ["lonely", "sp_new"]);
        // The module's own spells.2da, which the game reads in place of
        // its own: sp_old is no spell's now.
        m.set(
            key("spells", ResType::TWODA),
            table("2DA V2.0\n\n   Label   ImpactScript\n0  Fire  sp_fire\n1  New   sp_new\n"),
        );
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        assert_eq!(names(&rm, &m), ["lonely", "sp_old"]);
        // Where: the table and its cell.
        let fear = table_scripts_in(&rm).into_iter().find(|r| r.target == rr("st_fear")).unwrap();
        assert_eq!(
            (fear.from, fear.path.to_string().as_str()),
            (key("statescripts", ResType::TWODA), "row 1, SCRIPTNAME")
        );
        // Without the tables, they are unused.
        assert_eq!(unused(&m).len(), 7);
    }

    /// A string in a script that spells another script's name uses it,
    /// whatever it is passed to; one naming itself doesn't, nor a comment.
    #[test]
    fn a_script_s_string_naming_a_script_uses_it() {
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        let src =
            b"// \"in_comment\"\nvoid main() {\n  ExecuteScript(\"Run_Me\", OBJECT_SELF);\n  \
                    SetLocalString(OBJECT_SELF, \"next\", \"later\");\n  \
                    DelayCommand(1.0, ExecuteScript(\"caller\", OBJECT_SELF));\n  \
                    ExecuteScript(\"ai_\" + \"x\", OBJECT_SELF);\n  \
                    CreateObject(OBJECT_TYPE_CREATURE, \"orc_b\", GetLocation(OBJECT_SELF));\n  \
                    ExecuteScriptChunk(\"ExecuteScript(\\\"in_chunk\\\", OBJECT_SELF);\");\n}\n";
        m.set(key("caller", ResType::NSS), src.to_vec());
        for n in ["run_me", "later", "in_comment", "ai_x", "in_chunk"] {
            m.set(key(n, ResType::NCS), Vec::new());
        }
        m.set(key("orc_b", ResType::UTC), Vec::new());
        m.set(key("orc_c", ResType::UTC), Vec::new());
        let names: Vec<String> = unused(&m).iter().map(|u| u.key.resref.to_string()).collect();
        // (caller names itself; a name made as it runs isn't seen.)
        assert_eq!(names, ["ai_x", "caller", "in_comment", "orc_c"]);
    }

    /// What the module reaches in its haks names is used: a hak's
    /// conversation a placed creature has, and a hak's blueprint a string
    /// creates, and their scripts (Kingmaker keeps its conversations and
    /// blueprints in its hak).
    #[test]
    fn what_the_module_reaches_in_its_haks_names_is_used() {
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        let mut g = Gff::new(*b"GIT ");
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::CONVERSATION, rr("hak_talk"));
        g.root.items_mut(&git::CREATURE_LIST).push(c);
        m.set_gff(key("area1", ResType::GIT), &g).unwrap();
        m.set(
            key("spawner", ResType::NSS),
            b"void main() { CreateObject(OBJECT_TYPE_CREATURE, \"hak_kai\", GetLocation(OBJECT_SELF)); }"
                .to_vec(),
        );
        for n in ["talk_action", "kai_spawn", "kai_talk_action", "lonely"] {
            m.set(key(n, ResType::NCS), Vec::new());
        }
        let mut dlg = Gff::new(*b"DLG ");
        let mut entry = mg_gff::Struct::new(0);
        entry.set("Script", mg_gff::Value::resref(rr("talk_action")));
        dlg.root.set("EntryList", mg_gff::Value::List(vec![entry]));
        let mut utc = Gff::new(*b"UTC ");
        utc.root.set("ScriptSpawn", mg_gff::Value::resref(rr("kai_spawn")));
        utc.root.set("Conversation", mg_gff::Value::resref(rr("hak_kai")));
        let mut kai = Gff::new(*b"DLG ");
        let mut entry = mg_gff::Struct::new(0);
        entry.set("Script", mg_gff::Value::resref(rr("kai_talk_action")));
        kai.root.set("EntryList", mg_gff::Value::List(vec![entry]));
        let mut hak = MemContainer::new();
        hak.insert(key("hak_talk", ResType::DLG), dlg.to_bytes().unwrap());
        hak.insert(key("hak_kai", ResType::UTC), utc.to_bytes().unwrap());
        hak.insert(key("hak_kai", ResType::DLG), kai.to_bytes().unwrap());
        let mut rm = ResMan::new();
        rm.add(priority::HAK, "hak:x", LayerClass::Erf, hak);
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        let refs = module_references(&m);
        let also = also_used(&m, &refs, &rm);
        let names: Vec<String> =
            unused_of(&m, &refs, &also).iter().map(|u| u.key.resref.to_string()).collect();
        assert_eq!(names, ["lonely", "spawner"]);
        let alone: Vec<String> =
            unused_of(&m, &refs, &[]).iter().map(|u| u.key.resref.to_string()).collect();
        assert_eq!(alone.len(), 5, "{alone:?}");
    }

    /// A script the module's own table names, in a column its description
    /// says holds a script (`kind = "script"`), is used.
    #[test]
    fn a_script_a_described_table_names_is_used() {
        let dir = mg_testkit::scratch_dir("verify-described-scripts");
        let mut game = MemContainer::new();
        game.insert(key("nwscript", ResType::NSS), b"int J_ONE = 1;\n".to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, game);
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        m.set(
            key("j_modes", ResType::TWODA),
            table("2DA V2.0\n\n   Label  Script   Other\n0  Fight  j_fight  j_not\n1  None  ****  ****\n"),
        );
        for n in ["j_fight", "j_not"] {
            m.set(key(n, ResType::NCS), Vec::new());
        }
        m.save_as(&crate::ModuleLocation::Folder(dir.clone())).unwrap();
        let names = |m: &Module| -> Vec<String> {
            let also = described_scripts(m, Some(&rm));
            let refs = module_references(m);
            unused_of(m, &refs, &also).iter().map(|u| u.key.resref.to_string()).collect()
        };
        assert_eq!(names(&m), ["j_fight", "j_not"], "undescribed");
        // (Its other column a list of the game's constants, read from
        // the game's nwscript.)
        std::fs::write(
            dir.join("j_modes.2da.toml"),
            "[columns.Script]\nkind = \"script\"\n[columns.Other]\nkind = \"list\"\n\
             constants = \"nwscript\"\nprefix = \"J_\"\nother = true\n",
        )
        .unwrap();
        let m = Module::open(&dir).unwrap();
        assert_eq!(names(&m), ["j_not"]);
        let r = &described_scripts(&m, Some(&rm))[0];
        assert!(described_scripts(&m, None).is_empty(), "the game's list not found");
        assert_eq!(
            (r.from, r.path.to_string().as_str()),
            (key("j_modes", ResType::TWODA), "row 0, Script")
        );
    }

    /// The project file's `[used]` counts what it names as used.
    #[test]
    fn what_the_project_file_marks_is_used() {
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        for (n, t) in [
            ("ai_fight", ResType::NSS),
            ("ai_flee", ResType::NCS),
            ("other", ResType::NCS),
            ("spawn_orc", ResType::UTC),
            ("chat_1", ResType::DLG),
        ] {
            m.set(key(n, t), Vec::new());
        }
        m.project_file.apply(
            "moonglow.toml",
            "[used]\nscripts = [\"AI_*\"]\nblueprints = [\"spawn_*\"]\nconversations = [\"chat_?\"]\n",
            false,
        );
        let names: Vec<String> = unused(&m).iter().map(|u| u.key.to_string()).collect();
        assert_eq!(names, ["other.ncs"]);
    }

    /// What only the player's own content has (userpatch.ini's haks, the
    /// user's override, a Workshop item) is said, as a warning: a player
    /// without the same has nothing there. What the game or the module has
    /// too is not.
    #[test]
    fn a_reference_only_the_player_s_own_content_has_is_a_warning() {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.write(&ifo::MOD_ON_MOD_LOAD, rr("patch_only"));
        ifo.root.write(&ifo::MOD_ON_HEARTBEAT, rr("both"));
        ifo.root.write(&ifo::MOD_ON_CLIENT_ENTR, rr("override_only"));
        ifo.root.write(&ifo::MOD_ON_CLIENT_LEAV, rr("workshop_only"));
        ifo.root.write(&ifo::MOD_ON_PLR_DEATH, rr("in_module"));
        m.set_info(&ifo).unwrap();
        m.set(key("in_module", ResType::NCS), Vec::new());
        let mut patch = MemContainer::new();
        let mut base = MemContainer::new();
        let mut over = MemContainer::new();
        let mut workshop = MemContainer::new();
        for n in ["patch_only", "both"] {
            patch.insert(key(n, ResType::NCS), Vec::new());
        }
        base.insert(key("both", ResType::NCS), Vec::new());
        for n in ["override_only", "in_module", "workshop_only"] {
            over.insert(key(n, ResType::NCS), Vec::new());
        }
        workshop.insert(key("workshop_only", ResType::NCS), Vec::new());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, base);
        rm.add(priority::WORKSHOP, "workshop:1", LayerClass::Directory, workshop);
        rm.add(priority::OVERRIDE, "override", LayerClass::Directory, over);
        rm.add(priority::USERPATCH, "userpatch:hd", LayerClass::Erf, patch);
        let miss = missing(&m, &rm);
        let mut said: Vec<(String, Option<Local>, &str)> =
            miss.iter().map(|x| (x.reference.target.to_string(), x.only_local, x.what())).collect();
        said.sort_by(|a, b| a.0.cmp(&b.0));
        let override_ = "only in your override folder (players without the same won't have it)";
        assert_eq!(
            said,
            [
                ("override_only".to_string(), Some(Local::Override), override_),
                (
                    "patch_only".to_string(),
                    Some(Local::Userpatch),
                    "only in userpatch.ini's haks (players without them won't have it)"
                ),
                // (Its highest: the override over the Workshop.)
                ("workshop_only".to_string(), Some(Local::Override), override_),
            ],
            "{miss:?}"
        );
        assert!(miss.iter().all(|x| !x.is_error()));
        // A Workshop item alone.
        let mut alone = MemContainer::new();
        alone.insert(key("workshop_only", ResType::NCS), Vec::new());
        let mut rm = ResMan::new();
        rm.add(priority::WORKSHOP, "workshop:1", LayerClass::Directory, alone);
        let k = key("workshop_only", ResType::NCS);
        assert_eq!(only_local(&m, &rm, &k), Some(Local::Workshop));
        assert!(
            Missing { only_local: Some(Local::Workshop), ..miss[0].clone() }
                .what()
                .starts_with("only in a Steam Workshop item")
        );
    }

    #[test]
    fn an_item_blueprint_only_lying_in_an_area_is_used() {
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        let mut g = Gff::new(*b"GIT ");
        let on_the_ground = |name: &str| {
            let mut item = mg_gff::Struct::new(0);
            item.set("TemplateResRef", mg_gff::Value::resref(rr(name)));
            item
        };
        g.root.set(
            "List",
            mg_gff::Value::List(vec![on_the_ground("floor_sword"), on_the_ground("gone_sword")]),
        );
        m.set_gff(key("area1", ResType::GIT), &g).unwrap();
        m.set(key("floor_sword", ResType::UTI), Vec::new());
        m.set(key("spare_sword", ResType::UTI), Vec::new());

        let unused: Vec<String> = unused(&m).iter().map(|k| k.to_string()).collect();
        assert_eq!(unused, ["spare_sword.uti"]);
        let miss = missing(&m, &ResMan::new());
        let got: Vec<(String, Category)> =
            miss.iter().map(|m| (m.reference.target.to_string(), m.category)).collect();
        assert_eq!(got, [("gone_sword".into(), Category::Items)]);
    }
}
