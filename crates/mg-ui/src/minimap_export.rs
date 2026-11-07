//! Areas' minimaps written out from the module tree (its Export menus):
//! Export Minimap… one area's to a PNG chosen, Export Minimaps… several
//! areas' into a folder chosen, each `NAME.png` after its ResRef, as the
//! areas are now (unsaved tiles and waypoints included). With Map Notes,
//! each enabled map note is marked as the game's map marks it
//! (`mg_module::minimap::draw`). Files of those names
//! in the folder are asked about once, before they are overwritten
//! ([`crate::dialog_export::ask_window`]).

use std::path::Path;

use mg_core::{ResRef, ResType};
use mg_resman::ResKey;

use crate::dialog_export::{Ask, Exported};
use crate::{Moonglow, dialogs};

impl Moonglow {
    /// An area's minimap as a PNG, as it is now, its map notes marked when
    /// asked: the PNG, and how many notes were marked.
    fn minimap_png(&mut self, area: ResRef, notes: bool) -> Result<(Vec<u8>, usize), String> {
        let (Some(ws), Some(game)) = (&mut self.ws, &self.game) else {
            return Err("no module or game data".into());
        };
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).map_err(|e| e.to_string())?.root.clone();
        let git = match notes {
            true => Some(
                ws.doc(&ResKey::new(area, ResType::GIT)).map_err(|e| e.to_string())?.root.clone(),
            ),
            false => None,
        };
        let codepage = crate::text::game_table();
        let drawn = mg_module::minimap::draw(&game.resman, &are, git.as_ref(), None, codepage)?;
        Ok((mg_module::minimap::png(&drawn.image)?, drawn.notes.len()))
    }

    /// Export Minimap…: an area's minimap written to a PNG chosen.
    pub(crate) fn export_minimap(&mut self, area: ResRef, notes: bool) {
        let (data, marked) = match self.minimap_png(area, notes) {
            Ok(d) => d,
            Err(e) => {
                self.log.error(format!("Minimap of {area}: {e}"));
                return;
            }
        };
        let suggested = std::path::PathBuf::from(format!("{area}.png"));
        let Some(path) = self.dialogs.save_file(dialogs::FileKind::Png, Some(&suggested)) else {
            return;
        };
        match std::fs::write(&path, data) {
            Ok(()) => self.log.info(format!(
                "Saved the minimap of {area} to {}{}",
                path.display(),
                marked_said(notes, marked)
            )),
            Err(e) => self.log.error(format!("{}: {e}", path.display())),
        }
    }

    /// Export Minimaps…: several areas' minimaps into a folder chosen
    /// (asked first about files of their names in it).
    pub(crate) fn export_minimaps(&mut self, areas: Vec<ResRef>, notes: bool) {
        if self.ws.is_none() || areas.is_empty() {
            return;
        }
        let start = (self.export_dir.clone())
            .or_else(|| self.module_path().and_then(|p| p.parent().map(Path::to_path_buf)));
        let Some(dir) = self.dialogs.pick_folder("Export minimaps into", start.as_deref()) else {
            return;
        };
        self.export_dir = Some(dir.clone());
        let keys: Vec<ResKey> = areas.iter().map(|a| ResKey::new(*a, ResType::ARE)).collect();
        let existing: Vec<String> = (areas.iter())
            .map(|a| format!("{a}.png"))
            .filter(|name| dir.join(name).exists())
            .collect();
        if existing.is_empty() {
            self.write_minimaps(&keys, notes, &dir);
        } else {
            let what = Exported::Minimaps { notes };
            self.ask_or_write(Ask { keys, what, dir, existing });
        }
    }

    /// Writes each area of `keys`'s minimap into `dir` (`NAME.png`); the
    /// log says how many were written and where, and names each that was
    /// not, with why.
    pub(crate) fn write_minimaps(&mut self, keys: &[ResKey], notes: bool, dir: &Path) {
        if let Err(e) = std::fs::create_dir_all(dir) {
            self.log.error(format!("{}: {e}", dir.display()));
            return;
        }
        let (mut written, mut failed, mut marked) = (Vec::new(), Vec::new(), 0);
        for key in keys {
            let name = format!("{}.png", key.resref);
            let wrote = self.minimap_png(key.resref, notes).and_then(|(data, n)| {
                let path = dir.join(&name);
                std::fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
                Ok(n)
            });
            match wrote {
                Ok(n) => {
                    marked += n;
                    written.push(name);
                }
                Err(e) => failed.push(format!("{}: {e}", key.resref)),
            }
        }
        let n = written.len();
        if n > 0 {
            let what = if n == 1 { "1 minimap".to_string() } else { format!("{n} minimaps") };
            self.log.info(format!(
                "Wrote {what} to {}{}: {}",
                dir.display(),
                marked_said(notes, marked),
                crate::text::some_of(&written, 8)
            ));
        }
        for f in failed {
            self.log.error(format!("Minimap not written: {f}"));
        }
    }
}

/// What the log adds of the map notes marked, when asked for.
fn marked_said(notes: bool, marked: usize) -> String {
    match (notes, marked) {
        (false, _) => String::new(),
        (true, 1) => " (1 map note marked)".into(),
        (true, n) => format!(" ({n} map notes marked)"),
    }
}
