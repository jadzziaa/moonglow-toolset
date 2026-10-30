//! What a module's resources refer to: scripts, conversations, blueprints,
//! areas, sounds, tilesets. The graph drives verification (missing and unused
//! resources), export (dependencies) and renaming.
//!
//! References are found by walking every `CResRef` field of a GFF and
//! classifying it by its label and the list it sits in; script sources add
//! their `#include`s.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, GffPath, Struct, Value};
use mg_resman::ResKey;
use mg_rules::ObjectKind;

/// What kind of resource a reference points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RefKind {
    /// A script: satisfied by compiled `NCS` (the game needs it) or `NSS`.
    Script,
    Conversation,
    /// A blueprint of the given type (`UTC`, `UTI`, ...).
    Blueprint(ResType),
    Area,
    Sound,
    Tileset,
    Portrait,
    Movie,
    Character,
    /// An `#include`d script source.
    Include,
}

impl RefKind {
    /// What it names, for people: `script`, `blueprint (utc)`, ...
    pub fn name(self) -> String {
        match self {
            RefKind::Blueprint(t) => format!("blueprint ({})", t.extension().unwrap_or("?")),
            other => format!("{other:?}").to_ascii_lowercase(),
        }
    }

    /// Resource types that satisfy the reference, preferred first.
    pub fn types(self) -> &'static [ResType] {
        match self {
            RefKind::Script => &[ResType::NCS, ResType::NSS],
            RefKind::Conversation => &[ResType::DLG],
            RefKind::Blueprint(t) => ObjectKind::from_restype(t).map_or(&[], ObjectKind::restypes),
            RefKind::Area => &[ResType::ARE],
            RefKind::Sound => &[ResType::WAV],
            RefKind::Tileset => &[ResType::SET],
            RefKind::Portrait => &[ResType::TGA, ResType::DDS],
            // Movies live in `movies/` and `data/mov/`, outside the resource
            // manager, so they are not checked through it.
            RefKind::Movie => &[],
            RefKind::Character => &[ResType::BIC],
            RefKind::Include => &[ResType::NSS],
        }
    }

    /// The resource names that satisfy the reference: portraits name a
    /// family (`po_x` stands for `po_xh`, `po_xm`, ...), so their medium
    /// size is what must exist.
    pub fn candidates(self, target: ResRef) -> Vec<ResKey> {
        let name = match self {
            RefKind::Portrait => {
                let s = format!("{}m", target.to_string_lossy());
                match ResRef::from_str(&s) {
                    Ok(r) => r,
                    Err(_) => return Vec::new(),
                }
            }
            _ => target,
        };
        self.types().iter().map(|&t| ResKey::new(name, t)).collect()
    }
}

/// Where in a resource something is: a field of a GFF, a script's line, a
/// 2DA's cell, a sound set's entry, or a script's string. Its text
/// ([`Display`](std::fmt::Display)) is what `mg refs`, `mg verify` and the
/// window show: `/Creature List[3]/ScriptSpawn`, `line 12`, `row 12,
/// ImpactScript`, `entry 3`, `string`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Location {
    /// A field of a GFF resource.
    Gff(GffPath),
    /// A script's line (from 1).
    Line(usize),
    /// A 2DA's cell.
    Cell { row: usize, column: String },
    /// A sound set's entry (from 0).
    Entry(usize),
    /// A string in a script, which may be a resource's name.
    String,
}

impl Location {
    /// The path, of a field of a GFF.
    pub fn gff(&self) -> Option<&GffPath> {
        match self {
            Location::Gff(p) => Some(p),
            _ => None,
        }
    }

    /// The field's label, of a field of a GFF (`ScriptSpawn`).
    pub fn field(&self) -> Option<&str> {
        match self.gff()?.0.last()? {
            mg_gff::Step::Field(l) => Some(l),
            mg_gff::Step::Item(..) => None,
        }
    }
}

impl From<GffPath> for Location {
    fn from(path: GffPath) -> Location {
        Location::Gff(path)
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Gff(p) => write!(f, "{p}"),
            Location::Line(n) => write!(f, "line {n}"),
            Location::Cell { row, column } => write!(f, "row {row}, {column}"),
            Location::Entry(i) => write!(f, "entry {i}"),
            Location::String => f.write_str("string"),
        }
    }
}

/// One reference from a resource to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub from: ResKey,
    /// Where in the source (e.g. `/Creature List[3]/ScriptSpawn`).
    pub path: Location,
    pub kind: RefKind,
    pub target: ResRef,
}

impl Reference {
    pub fn candidates(&self) -> Vec<ResKey> {
        self.kind.candidates(self.target)
    }
}

/// The blueprint type of the instances or items in a list: a GIT's list
/// of placed objects but `List` (a name other files use for other
/// things: [`classify`] takes a GIT's), an encounter's creatures, an
/// inventory.
fn list_blueprint(list: &str) -> Option<ResType> {
    Some(match list {
        "CreatureList" => ResType::UTC,
        "ItemList" | "Equip_ItemList" => ResType::UTI,
        l => ObjectKind::from_list(l).filter(|k| *k != ObjectKind::Item)?.restype(),
    })
}

/// Whether a field (a resref) names a script by its label: an event's
/// (`OnEnter`, `ScriptSpawn`, `Mod_OnHeartbeat`), a conversation line's
/// (`Active`, `Script`) or a conversation's ending.
pub fn is_script_field(label: &str) -> bool {
    label.starts_with("On")
        || label.starts_with("Mod_On")
        || label.starts_with("Script")
        || matches!(label, "Active" | "EndConversation" | "EndConverAbort")
}

fn classify(file_type: &str, list: Option<&str>, label: &str) -> Option<RefKind> {
    if is_script_field(label) {
        return Some(RefKind::Script);
    }
    Some(match label {
        "Conversation" => RefKind::Conversation,
        // Items lying in an area are in its GIT's `List` (a name other
        // files use for other things).
        "TemplateResRef" if file_type == "GIT" && list == Some("List") => {
            RefKind::Blueprint(ResType::UTI)
        }
        "TemplateResRef" | "InventoryRes" | "EquippedRes" => {
            // In a blueprint, the root's TemplateResRef is its own name.
            RefKind::Blueprint(list.and_then(list_blueprint)?)
        }
        // Encounter creature lists and store instances name blueprints.
        "ResRef" if file_type == "UTE" || list == Some("CreatureList") => {
            RefKind::Blueprint(ResType::UTC)
        }
        "ResRef" if list == Some("StoreList") => RefKind::Blueprint(ResType::UTM),
        "ResRef" if list == Some("Mod_CacheNSSList") => RefKind::Script,
        "Sound" => RefKind::Sound,
        "Tileset" => RefKind::Tileset,
        "Area_Name" | "Mod_Entry_Area" => RefKind::Area,
        "Portrait" => RefKind::Portrait,
        "Mod_StartMovie" => RefKind::Movie,
        "Mod_DefaultBic" => RefKind::Character,
        _ => return None,
    })
}

fn walk(from: ResKey, file_type: &str, s: &Struct, out: &mut Vec<Reference>) {
    s.walk_fields(&mut |f| {
        if let (Value::ResRef(_), Some(target)) = (f.value, f.value.as_resref())
            && !target.is_empty()
            && let Some(kind) = classify(file_type, f.list, f.label)
        {
            out.push(Reference { from, path: Location::Gff(f.path.clone()), kind, target });
        }
    });
}

/// Points the references of the given kinds to `from` (compared ignoring
/// case, as the game does) at `to`; returns how many changed.
pub fn rewrite_references(gff: &mut Gff, kinds: &[RefKind], from: ResRef, to: ResRef) -> usize {
    fn walk_mut(
        file_type: &str,
        s: &mut Struct,
        list: Option<&str>,
        kinds: &[RefKind],
        from: ResRef,
        to: ResRef,
    ) -> usize {
        let mut n = 0;
        for f in &mut s.fields {
            let label = f.label.to_string_lossy();
            match &mut f.value {
                Value::ResRef(_) => {
                    let hit = f.value.as_resref() == Some(from)
                        && classify(file_type, list, &label).is_some_and(|k| kinds.contains(&k));
                    if hit {
                        f.value = Value::resref(to);
                        n += 1;
                    }
                }
                Value::Struct(c) => n += walk_mut(file_type, c, list, kinds, from, to),
                Value::List(items) => {
                    for c in items {
                        n += walk_mut(file_type, c, Some(&label), kinds, from, to);
                    }
                }
                _ => {}
            }
        }
        n
    }
    let file_type = gff.file_type_str();
    walk_mut(&file_type, &mut gff.root, None, kinds, from, to)
}

/// The references in a GFF resource.
pub fn gff_references(from: ResKey, gff: &Gff) -> Vec<Reference> {
    let mut out = Vec::new();
    walk(from, &gff.file_type_str(), &gff.root, &mut out);
    out
}

/// The `#include "name"` of a script source, as the compiler reads them:
/// not one in a comment (`// #include`, or a line of a `/* … */` block:
/// code commented out includes nothing, and Verify's missing include
/// would be none) or in a string ([`mg_script::lex::includes`]).
pub fn script_includes(from: ResKey, source: &[u8]) -> Vec<Reference> {
    let mut out = Vec::new();
    let mut line = 1;
    let mut counted = 0;
    for at in mg_script::lex::includes(source) {
        let quoted = String::from_utf8_lossy(&source[at.name]);
        line += source[counted..at.directive.start].iter().filter(|&&b| b == b'\n').count();
        counted = at.directive.start;
        if let Ok(target) = ResRef::from_str(&quoted)
            && !target.is_empty()
        {
            out.push(Reference {
                from,
                path: Location::Line(line),
                kind: RefKind::Include,
                target,
            });
        }
    }
    out
}

/// The plain string literals of a script source that can be a resource's
/// name (1 to 16 characters a ResRef takes), as the compiler reads them
/// (not in comments): whatever they are passed to, `ExecuteScript("x")`
/// as `SetLocalString(o, "next", "x")`; and in a longer string, the
/// strings it quotes (`ExecuteScriptChunk("ExecuteScript(\"x\", OBJECT_SELF);")`).
pub fn script_strings(source: &[u8]) -> Vec<ResRef> {
    use mg_script::lex::{TokenKind, tokenize};
    let name = |s: &[u8]| {
        if s.is_empty() || s.len() > 16 {
            return None;
        }
        ResRef::from_str(std::str::from_utf8(s).ok()?).ok()
    };
    let mut out = Vec::new();
    for t in tokenize(source) {
        if !matches!(t.kind, TokenKind::String { terminated: true }) {
            continue;
        }
        let Some(s) = t.span.end.checked_sub(1).and_then(|end| source.get(t.span.start + 1..end))
        else {
            continue;
        };
        if s.len() <= 16 {
            out.extend(name(s));
            continue;
        }
        // (A chunk of script: what it quotes, between \" and \".)
        let text = String::from_utf8_lossy(s);
        let mut parts = text.split("\\\"");
        parts.next();
        while let (Some(inner), Some(_)) = (parts.next(), parts.next()) {
            out.extend(name(inner.as_bytes()));
        }
    }
    out
}

/// The module's scripts that include any of `changed`, however
/// indirectly (they compile differently when it changes), not `changed`
/// themselves; sorted.
pub fn includers(module: &crate::Module, changed: &[ResKey]) -> Vec<ResKey> {
    use std::collections::{HashMap, HashSet};
    // Who includes each name, then outward from the changed ones: each
    // script looked at once (a scan of every script for every one found
    // took seconds of a save in a persistent world).
    let mut by: HashMap<ResRef, Vec<ResKey>> = HashMap::new();
    for k in module.keys_of(ResType::NSS) {
        let src = module.get(k).unwrap_or_default();
        for r in script_includes(*k, src) {
            by.entry(r.target).or_default().push(*k);
        }
    }
    let mut seen: HashSet<ResKey> = changed.iter().copied().collect();
    let mut next: Vec<ResRef> = changed.iter().map(|k| k.resref).collect();
    let mut found = Vec::new();
    while let Some(name) = next.pop() {
        for k in by.get(&name).into_iter().flatten() {
            if seen.insert(*k) {
                found.push(*k);
                next.push(k.resref);
            }
        }
    }
    found.sort();
    found
}

/// The references of any resource (GFF or script source); others have none.
pub fn references(key: ResKey, data: &[u8]) -> Vec<Reference> {
    if key.restype == ResType::NSS {
        return script_includes(key, data);
    }
    if !key.restype.is_gff() {
        return Vec::new();
    }
    match Gff::read(data) {
        Ok(g) => gff_references(key, &g),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use mg_schema::{StructExt, git, ifo};

    use super::*;

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    #[test]
    fn git_instances_reference_blueprints_and_scripts() {
        let mut g = Gff::new(*b"GIT ");
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::TEMPLATE_RES_REF, rr("nw_chicken"));
        c.write(&git::creature_list::SCRIPT_SPAWN, rr("my_spawn"));
        c.write(&git::creature_list::CONVERSATION, rr("chicken_talk"));
        c.set("Nothing", Value::ResRef(Vec::new()));
        g.root.items_mut(&git::CREATURE_LIST).push(c);
        let key = ResKey::parse("area1", ResType::GIT).unwrap();
        let refs = gff_references(key, &g);
        let got: Vec<(RefKind, String, String)> =
            (refs.iter()).map(|r| (r.kind, r.target.to_string(), r.path.to_string())).collect();
        assert_eq!(
            got,
            [
                (
                    RefKind::Blueprint(ResType::UTC),
                    "nw_chicken".into(),
                    "/Creature List[0]/TemplateResRef".into()
                ),
                (RefKind::Script, "my_spawn".into(), "/Creature List[0]/ScriptSpawn".into()),
                (
                    RefKind::Conversation,
                    "chicken_talk".into(),
                    "/Creature List[0]/Conversation".into()
                ),
            ]
        );
        // (Each a field of the GFF, its path typed.)
        let spawn = GffPath::root().item("Creature List", 0).field("ScriptSpawn");
        assert_eq!(refs[1].path, Location::Gff(spawn));
    }

    #[test]
    fn every_placed_object_references_its_blueprint() {
        for k in ObjectKind::ALL {
            let mut placed = Struct::new(k.struct_id());
            placed.set(k.resref_field(), Value::resref(rr("bp")));
            let mut g = Gff::new(*b"GIT ");
            g.root.set(k.list(), Value::List(vec![placed]));
            let refs = gff_references(ResKey::parse("area1", ResType::GIT).unwrap(), &g);
            let got: Vec<(RefKind, String)> =
                refs.iter().map(|r| (r.kind, r.path.to_string())).collect();
            let path = format!("/{}[0]/{}", k.list(), k.resref_field());
            assert_eq!(got, [(RefKind::Blueprint(k.restype()), path)], "{k:?}");
        }
    }

    #[test]
    fn blueprint_own_name_is_not_a_reference_but_inventory_is() {
        let mut g = Gff::new(*b"UTC ");
        g.root.set("TemplateResRef", Value::resref(rr("self")));
        let mut item = Struct::new(0);
        item.set("InventoryRes", Value::resref(rr("sword")));
        g.root.set("ItemList", Value::List(vec![item]));
        let refs = gff_references(ResKey::parse("self", ResType::UTC).unwrap(), &g);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, RefKind::Blueprint(ResType::UTI));
    }

    #[test]
    fn an_item_lying_in_an_area_references_its_blueprint() {
        let mut item = Struct::new(0);
        item.set("TemplateResRef", Value::resref(rr("my_sword")));
        let mut g = Gff::new(*b"GIT ");
        g.root.set("List", Value::List(vec![item.clone()]));
        let refs = gff_references(ResKey::parse("area1", ResType::GIT).unwrap(), &g);
        let got: Vec<(RefKind, String)> =
            refs.iter().map(|r| (r.kind, r.path.to_string())).collect();
        assert_eq!(got, [(RefKind::Blueprint(ResType::UTI), "/List[0]/TemplateResRef".into())]);
        // A `List` elsewhere is not the area's items.
        let mut other = Gff::new(*b"GIC ");
        other.root.set("List", Value::List(vec![item]));
        assert!(gff_references(ResKey::parse("area1", ResType::GIC).unwrap(), &other).is_empty());
    }

    #[test]
    fn module_info_and_includes_and_portraits() {
        let mut g = Gff::new(*b"IFO ");
        g.root.write(&ifo::MOD_ON_MOD_LOAD, rr("x_load"));
        g.root.write(&ifo::MOD_ENTRY_AREA, rr("start"));
        let refs = gff_references(ResKey::parse("module", ResType::IFO).unwrap(), &g);
        assert_eq!(
            refs.iter().map(|r| r.kind).collect::<Vec<_>>(),
            [RefKind::Script, RefKind::Area]
        );

        let src =
            b"// x\n#include \"nw_i0_generic\"\n  #include \"x0_i0_spawn\" // c\nvoid main(){}\n";
        let inc = script_includes(ResKey::parse("s", ResType::NSS).unwrap(), src);
        assert_eq!(
            inc.iter().map(|r| r.target.to_string()).collect::<Vec<_>>(),
            ["nw_i0_generic", "x0_i0_spawn"]
        );
        assert_eq!(inc[1].path.to_string(), "line 3");
        // Commented out, or in a string: none, as the compiler reads them.
        let src =
            b"// #include \"a\"\n/*\n#include \"b\"\n*/\nstring s = \"#include \\\"c\\\"\";\n\
                    #include \"d\"\n";
        let inc = script_includes(ResKey::parse("s", ResType::NSS).unwrap(), src);
        let got: Vec<_> = inc.iter().map(|r| (r.target.to_string(), r.path.to_string())).collect();
        assert_eq!(got, [("d".to_string(), "line 6".to_string())]);

        let p = RefKind::Portrait.candidates(rr("po_aribeth_"));
        assert_eq!(p[0], ResKey::parse("po_aribeth_m", ResType::TGA).unwrap());
    }

    /// Each kind of place says itself as `mg refs`, `mg verify` and the
    /// window have always said it.
    #[test]
    fn locations_are_said_as_they_were() {
        let field = GffPath::root().item("Creature List", 10).field("ScriptSpawn");
        let said = [
            Location::Gff(field.clone()),
            Location::Line(12),
            Location::Cell { row: 3, column: "ImpactScript".into() },
            Location::Entry(4),
            Location::String,
        ]
        .map(|l| l.to_string());
        assert_eq!(
            said,
            [
                "/Creature List[10]/ScriptSpawn",
                "line 12",
                "row 3, ImpactScript",
                "entry 4",
                "string"
            ]
        );
        assert_eq!(Location::from(field.clone()).gff(), Some(&field));
        assert_eq!(Location::Line(1).gff(), None);
        assert_eq!(Location::from(field).field(), Some("ScriptSpawn"));
    }

    /// A label with the characters a path is written with is written
    /// escaped, as `mg set` and `mg apply` read a path (walked as text, it
    /// was `/a/b[0]/OnSpawn`, which reads back as another path).
    #[test]
    fn a_label_with_a_path_s_characters_is_written_escaped() {
        let mut odd = Struct::new(0);
        odd.set("OnSpawn", Value::resref(rr("my_spawn")));
        let mut g = Gff::new(*b"UTC ");
        g.root.set("a/b", Value::List(vec![odd]));
        let refs = gff_references(ResKey::parse("x", ResType::UTC).unwrap(), &g);
        let text = refs[0].path.to_string();
        assert_eq!(text, "/a~1b[0]/OnSpawn");
        assert_eq!(text.parse::<GffPath>().map(Location::Gff), Ok(refs[0].path.clone()));
    }
}
