//! Edit Areas Together's sets: the areas ticked there kept under a name
//! (Save Set…) and ticked again later (Sets), for the module they were
//! chosen in. They live in Moonglow's data folder as small JSON files,
//! one a set, that name the module and its areas:
//!
//! ```json
//! { "module": "krezk", "areas": ["kz_square", "kz_inn"] }
//! ```

use std::path::{Path, PathBuf};

use mg_core::ResRef;
use mg_module::ModuleLocation;
use serde_json::{Value, json};

use crate::saved::Shelf;

const SHELF: Shelf = Shelf { folder: "area-sets", suffix: ".areas.json", what: "set of areas" };

/// Where sets of areas are kept.
pub fn dir() -> Option<PathBuf> {
    SHELF.dir()
}

/// The name a set knows its module by: a `.mod`'s name, a module
/// folder's or a nasher project's (and its target's).
pub fn module_name(location: Option<&ModuleLocation>) -> String {
    let stem = |p: &Path| p.file_stem().map(|s| s.to_string_lossy().into_owned());
    match location {
        Some(ModuleLocation::Project { root, target }) => {
            format!("{}:{target}", stem(root).unwrap_or_default())
        }
        Some(l) => stem(l.path()).unwrap_or_default(),
        None => String::new(),
    }
}

/// A set as its file has it: the module, the areas.
fn parse(text: &str) -> Result<(String, Vec<ResRef>), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let module = v.get("module").and_then(Value::as_str).unwrap_or_default().to_string();
    let areas = (v.get("areas").and_then(Value::as_array))
        .ok_or("no \"areas\" list")?
        .iter()
        .filter_map(|a| ResRef::from_str(a.as_str()?).ok())
        .collect();
    Ok((module, areas))
}

/// The sets kept for the module `module` ([`module_name`]), by name.
pub fn list_for(dir: Option<&Path>, module: &str) -> Vec<String> {
    let of = |name: &String| SHELF.read(dir, name).ok().and_then(|t| parse(&t).ok());
    SHELF.list(dir).into_iter().filter(|n| of(n).is_some_and(|(m, _)| m == module)).collect()
}

/// Keeps `areas` as the set `name` of `module` (in place of one of that
/// name).
pub fn save(
    dir: Option<&Path>,
    name: &str,
    module: &str,
    areas: impl IntoIterator<Item = ResRef>,
) -> Result<PathBuf, String> {
    let areas: Vec<String> = areas.into_iter().map(|a| a.to_lowercase().to_string()).collect();
    let text = json!({ "module": module, "areas": areas });
    let text = serde_json::to_string_pretty(&text).map_err(|e| e.to_string())?;
    SHELF.write(dir, name, &format!("{text}\n"))
}

/// The areas of the set `name`.
pub fn read(dir: Option<&Path>, name: &str) -> Result<Vec<ResRef>, String> {
    SHELF.read(dir, name).and_then(|t| parse(&t)).map(|(_, areas)| areas)
}

/// Deletes the set `name`.
pub fn delete(dir: Option<&Path>, name: &str) -> Result<(), String> {
    SHELF.delete(dir, name).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_is_kept_for_its_module_and_read_back() {
        let root = mg_testkit::scratch_dir("ui-area-sets");
        let dir = Some(root.as_path());
        let r = |s: &str| ResRef::from_str(s).unwrap();
        save(dir, "Krezk streets", "krezk", [r("kz_square"), r("KZ_Inn")]).unwrap();
        save(dir, "Caves", "barovia", [r("cave01")]).unwrap();
        assert_eq!(list_for(dir, "krezk"), ["Krezk streets"]);
        assert_eq!(read(dir, "Krezk streets").unwrap(), [r("kz_square"), r("kz_inn")]);
        // By hand: an area name that isn't one is left out.
        std::fs::write(
            root.join("By hand.areas.json"),
            r#"{ "module": "krezk", "areas": ["kz_well", "not a resref, far too long"] }"#,
        )
        .unwrap();
        assert_eq!(read(dir, "By hand").unwrap(), [r("kz_well")]);
        delete(dir, "Caves").unwrap();
        assert!(list_for(dir, "barovia").is_empty());
        let mod_path = ModuleLocation::Archive("/x/krezk.mod".into());
        assert_eq!(module_name(Some(&mod_path)), "krezk");
        let project = ModuleLocation::Project { root: "/x/tdod".into(), target: "default".into() };
        assert_eq!(module_name(Some(&project)), "tdod:default");
    }
}
