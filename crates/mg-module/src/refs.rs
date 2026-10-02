//! What a module's resources refer to: scripts, conversations, blueprints,
//! areas, sounds, tilesets. The graph drives verification (missing and unused
//! resources), export (dependencies) and, later, renaming.
//!
//! References are found by walking every `CResRef` field of a GFF and
//! classifying it by its label and the list it sits in; script sources add
//! their `#include`s.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;

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
            RefKind::Blueprint(t) => match t {
                ResType::UTC => &[ResType::UTC],
                ResType::UTD => &[ResType::UTD],
                ResType::UTE => &[ResType::UTE],
                ResType::UTI => &[ResType::UTI],
                ResType::UTM => &[ResType::UTM],
                ResType::UTP => &[ResType::UTP],
                ResType::UTS => &[ResType::UTS],
                ResType::UTT => &[ResType::UTT],
                ResType::UTW => &[ResType::UTW],
                _ => &[],
            },
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

/// One reference from a resource to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub from: ResKey,
    /// Where in the source (e.g. `/Creature List[3]/ScriptSpawn`).
    pub path: String,
    pub kind: RefKind,
    pub target: ResRef,
}

impl Reference {
    pub fn candidates(&self) -> Vec<ResKey> {
        self.kind.candidates(self.target)
    }
}

/// The blueprint type of the instances or items in a list.
fn list_blueprint(list: &str) -> Option<ResType> {
    Some(match list {
        "Creature List" | "CreatureList" => ResType::UTC,
        "Door List" => ResType::UTD,
        "Encounter List" => ResType::UTE,
        "Placeable List" => ResType::UTP,
        "SoundList" => ResType::UTS,
        "StoreList" => ResType::UTM,
        "TriggerList" => ResType::UTT,
        "WaypointList" => ResType::UTW,
        "ItemList" | "Equip_ItemList" => ResType::UTI,
        _ => return None,
    })
}

fn classify(file_type: &str, list: Option<&str>, label: &str) -> Option<RefKind> {
    let script = label.starts_with("On")
        || label.starts_with("Mod_On")
        || label.starts_with("Script")
        || matches!(label, "Active" | "EndConversation" | "EndConverAbort");
    if script {
        return Some(RefKind::Script);
    }
    Some(match label {
        "Conversation" => RefKind::Conversation,
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

fn walk(
    from: ResKey,
    file_type: &str,
    s: &Struct,
    list: Option<&str>,
    path: &str,
    out: &mut Vec<Reference>,
) {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        let fpath = format!("{path}/{label}");
        match &f.value {
            Value::ResRef(_) => {
                let (Some(target), Some(kind)) =
                    (f.value.as_resref(), classify(file_type, list, &label))
                else {
                    continue;
                };
                if !target.is_empty() {
                    out.push(Reference { from, path: fpath, kind, target });
                }
            }
            Value::Struct(c) => walk(from, file_type, c, list, &fpath, out),
            Value::List(items) => {
                for (i, c) in items.iter().enumerate() {
                    walk(from, file_type, c, Some(&label), &format!("{fpath}[{i}]"), out);
                }
            }
            _ => {}
        }
    }
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
    walk(from, &gff.file_type_str(), &gff.root, None, "", &mut out);
    out
}

/// The `#include "name"` lines of a script source (comments and strings
/// are not parsed; an include inside a block comment still counts, as a
/// conservative dependency).
pub fn script_includes(from: ResKey, source: &[u8]) -> Vec<Reference> {
    let text = String::from_utf8_lossy(source);
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("#include") else { continue };
        let Some(start) = rest.find('"') else { continue };
        let rest = &rest[start + 1..];
        let Some(end) = rest.find('"') else { continue };
        if let Ok(target) = ResRef::from_str(&rest[..end])
            && !target.is_empty()
        {
            out.push(Reference {
                from,
                path: format!("line {}", n + 1),
                kind: RefKind::Include,
                target,
            });
        }
    }
    out
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
        let got: Vec<(RefKind, String, &str)> =
            refs.iter().map(|r| (r.kind, r.target.to_string(), r.path.as_str())).collect();
        assert_eq!(
            got,
            [
                (
                    RefKind::Blueprint(ResType::UTC),
                    "nw_chicken".into(),
                    "/Creature List[0]/TemplateResRef"
                ),
                (RefKind::Script, "my_spawn".into(), "/Creature List[0]/ScriptSpawn"),
                (RefKind::Conversation, "chicken_talk".into(), "/Creature List[0]/Conversation"),
            ]
        );
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
        assert_eq!(inc[1].path, "line 3");

        let p = RefKind::Portrait.candidates(rr("po_aribeth_"));
        assert_eq!(p[0], ResKey::parse("po_aribeth_m", ResType::TGA).unwrap());
    }
}
