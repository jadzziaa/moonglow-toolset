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
    /// (`None`: not found in the user's `hak/` or the install's `data/hk/`).
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
    let base = ResMan::for_game(&GameInstall { user_dir: None, ..install.clone() })
        .map_err(|e| ModuleError::Archive { path: install.root.clone(), message: e.to_string() })?;
    let mut report = HakReport::default();
    for name in names {
        let file = format!("{name}.hak");
        let path = install
            .user_dir
            .as_ref()
            .map(|u| u.join("hak").join(&file))
            .filter(|p| p.is_file())
            .or_else(|| Some(install.root.join("data/hk").join(&file)).filter(|p| p.is_file()));
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
