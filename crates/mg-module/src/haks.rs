//! Hak conflict analysis (Module Properties › Custom Content › Check for
//! Conflicts): every resource the module's haks provide, the ones several
//! haks provide, and the base-game resources they override.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use mg_resman::{Container, ErfContainer, GameInstall, LayerClass, ResKey, ResMan};

use crate::ModuleError;

/// Where each hak resource comes from, in hak priority order.
#[derive(Debug, Clone, Default)]
pub struct HakReport {
    /// The haks in `Mod_HakList` order with the file found for each
    /// (`None`: not found where the game looks, [`GameInstall::hak_dirs`]).
    pub haks: Vec<(String, Option<PathBuf>)>,
    /// Every resource in any hak, with the haks that contain it (highest
    /// priority first; the first one wins in the game).
    pub resources: BTreeMap<ResKey, Vec<String>>,
    /// Hak resources that also exist in the base game.
    pub overrides: BTreeMap<ResKey, Vec<String>>,
}

impl HakReport {
    /// Resources provided by more than one hak.
    pub fn conflicts(&self) -> impl Iterator<Item = (&ResKey, &Vec<String>)> {
        self.resources.iter().filter(|(_, haks)| haks.len() > 1)
    }

    /// The text report Aurora's "Report Resources and Conflicts..." saves.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "Hak paks (highest priority first):");
        for (name, path) in &self.haks {
            match path {
                Some(p) => _ = writeln!(s, "  {name}  ({})", p.display()),
                None => _ = writeln!(s, "  {name}  (NOT FOUND)"),
            }
        }
        let section =
            |s: &mut String,
             title: &str,
             rows: &mut dyn Iterator<Item = (&ResKey, &Vec<String>)>| {
                let _ = writeln!(s, "\n{title}:");
                let mut n = 0;
                for (k, haks) in rows {
                    let _ = writeln!(s, "  {k}\t{}", haks.join(", "));
                    n += 1;
                }
                if n == 0 {
                    let _ = writeln!(s, "  (none)");
                }
            };
        section(&mut s, "Conflicting resources", &mut self.conflicts());
        section(&mut s, "Overridden standard resources", &mut self.overrides.iter());
        section(&mut s, "Complete resource list", &mut self.resources.iter());
        s
    }
}

/// Analyses the given haks (in `Mod_HakList` order) against the base game.
pub fn hak_report(install: &GameInstall, names: &[String]) -> Result<HakReport, ModuleError> {
    let base =
        ResMan::for_game(&GameInstall { user_dir: None, workshop: false, ..install.clone() })
            .map_err(|e| ModuleError::Archive {
                path: install.root.clone(),
                message: e.to_string(),
            })?;
    let mut report = HakReport::default();
    for name in names {
        let file = format!("{name}.hak");
        let path = install.hak_dirs().into_iter().map(|d| d.join(&file)).find(|p| p.is_file());
        report.haks.push((name.clone(), path.clone()));
        let Some(path) = path else { continue };
        let hak = ErfContainer::open(&path)
            .map_err(|e| ModuleError::Archive { path: path.clone(), message: e.to_string() })?;
        for k in hak.keys() {
            report.resources.entry(k).or_default().push(name.clone());
        }
    }
    let in_base = |k: &ResKey| {
        base.layers().iter().filter(|l| l.class == LayerClass::Key).any(|l| l.container.contains(k))
    };
    report.overrides =
        report.resources.iter().filter(|(k, _)| in_base(k)).map(|(k, v)| (*k, v.clone())).collect();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use mg_core::ResType;

    use super::*;

    /// A hak is found where the game finds it: in the user's folder, in a
    /// Steam Workshop item (when those are read), in the install.
    #[test]
    fn a_hak_is_found_in_a_workshop_item() {
        let lib = mg_testkit::scratch_dir("haks-workshop");
        let root = lib.join("steamapps/common/Neverwinter Nights");
        let item = lib.join("steamapps/workshop/content/704450/123");
        let user = lib.join("user");
        for d in [root.join("data/hk"), item.join("hak"), user.join("hak")] {
            std::fs::create_dir_all(d).unwrap();
        }
        let hak = |path: PathBuf, resource: &str| {
            let key = ResKey::parse(resource, ResType::TWODA).unwrap();
            let mut w = mg_erf::ErfWriter::new(*b"HAK ");
            w.add(key.resref, key.restype, b"2DA V2.0\n".to_vec()).unwrap();
            std::fs::write(path, w.to_bytes().unwrap()).unwrap();
        };
        hak(user.join("hak/mine.hak"), "mine");
        hak(item.join("hak/shared.hak"), "shared");
        hak(root.join("data/hk/stock.hak"), "stock");
        let names = ["mine", "shared", "stock", "nowhere"].map(String::from);
        let mut install = GameInstall::new(&root, Some(user), "en");
        let found = |install: &GameInstall| -> Vec<bool> {
            hak_report(install, &names).unwrap().haks.iter().map(|(_, p)| p.is_some()).collect()
        };
        assert_eq!(found(&install), [true, false, true, false]);
        install.workshop = true;
        assert_eq!(found(&install), [true, true, true, false]);
    }
}
