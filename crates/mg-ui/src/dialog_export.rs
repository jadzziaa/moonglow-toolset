//! Conversations written out from the module tree (its Export menu's
//! Export as Ink…, Twine…, Text… and CSV…): each one a file of its own in a
//! folder asked for, `NAME.ink` after its ResRef, as the conversation
//! editor's Export writes it (the same text: what the editor shows, its
//! changes not saved yet included). Files of those names in the folder are
//! asked about once, all of them named, before they are overwritten; one
//! that can't be written is named with why, and the rest are written.
//! (Several areas' minimaps ask the same question: [`Exported`].)

use std::path::{Path, PathBuf};

use mg_core::ResType;
use mg_gff::Gff;
use mg_module::dialog_io::Format;
use mg_resman::ResKey;

use crate::Moonglow;

/// The formats in the order the tree's menu offers them, with the word
/// its items name each by ("Export as Ink…").
pub(crate) const FORMATS: [(Format, &str); 4] =
    [(Format::Ink, "Ink"), (Format::Twine, "Twine"), (Format::Text, "Text"), (Format::Csv, "CSV")];

/// The word the menu names `format` by.
pub(crate) fn word(format: Format) -> &'static str {
    FORMATS.iter().find(|(f, _)| *f == format).map_or("", |(_, w)| w)
}

/// What the conversation editor's Export writes of the conversation `g`
/// (`key`) as `format`: its lines as the module's own talk table reads
/// them, and a CSV with a column for the language edited (Options ›
/// Language), to translate into.
pub(crate) fn text_of(g: &Gff, key: ResKey, format: Format) -> String {
    let edited = crate::text::edit_language();
    let to = if edited == mg_core::Language::ENGLISH { vec![] } else { vec![edited] };
    format.write_in(g, &key.resref.to_string(), crate::text::game_table(), &to)
}

/// The file a conversation is written to: its ResRef, with the format's
/// extension.
pub(crate) fn file_name(key: ResKey, format: Format) -> String {
    format!("{}.{}", key.resref, format.extensions()[0])
}

/// Whether each of `keys` is a conversation (and there is one): the tree's
/// Export menu offers the four then.
pub(crate) fn all_conversations(keys: &[ResKey]) -> bool {
    !keys.is_empty() && keys.iter().all(|k| k.restype == ResType::DLG)
}

/// What a question before writing over files is about: conversations
/// written as `format`, or areas' minimaps (`minimap_export`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exported {
    Conversations(Format),
    Minimaps { notes: bool },
}

/// The question asked before files in the folder are written over.
#[derive(Debug, Clone)]
pub struct Ask {
    pub keys: Vec<ResKey>,
    pub what: Exported,
    pub dir: PathBuf,
    /// The files of those names in the folder.
    pub existing: Vec<String>,
}

impl Ask {
    /// The file `key` is written as.
    fn file_of(&self, key: ResKey) -> String {
        match self.what {
            Exported::Conversations(format) => file_name(key, format),
            Exported::Minimaps { .. } => format!("{}.png", key.resref),
        }
    }

    /// The keys whose files aren't in the folder (Skip Those).
    fn new_ones(&self) -> Vec<ResKey> {
        self.keys.iter().copied().filter(|k| !self.existing.contains(&self.file_of(*k))).collect()
    }
}

impl Moonglow {
    /// Files of `ask`'s in the folder already: asked about, unless
    /// Options › General › Questions says to replace them or skip them.
    pub(crate) fn ask_or_write(&mut self, ask: Ask) {
        use crate::settings::FilesThere;
        match self.settings.files_there {
            FilesThere::Ask => self.dialog_export_ask = Some(ask),
            FilesThere::Replace => self.write_exported(&ask, &ask.keys),
            FilesThere::Skip => self.write_exported(&ask, &ask.new_ones()),
        }
    }

    /// Writes `keys` of `ask`'s export into its folder.
    fn write_exported(&mut self, ask: &Ask, keys: &[ResKey]) {
        match ask.what {
            Exported::Conversations(format) => self.write_conversations(keys, format, &ask.dir),
            Exported::Minimaps { notes } => self.write_minimaps(keys, notes, &ask.dir),
        }
    }

    /// Export as Ink… (or Twine…, Text…, CSV…) of conversations chosen in
    /// the module tree: a folder asked for, then written (asked first
    /// about files of their names in it).
    pub(crate) fn export_conversations(&mut self, keys: Vec<ResKey>, format: Format) {
        if self.ws.is_none() || keys.is_empty() {
            return;
        }
        let start = (self.export_dir.clone())
            .or_else(|| self.module_path().and_then(|p| p.parent().map(Path::to_path_buf)));
        let title = format!("Export conversations as {} into", word(format));
        let Some(dir) = self.dialogs.pick_folder(&title, start.as_deref()) else { return };
        self.export_dir = Some(dir.clone());
        let existing: Vec<String> = (keys.iter())
            .filter(|k| {
                let ending = format.extensions()[0];
                crate::transfer::named_in(&dir, &k.resref, ending).is_ok_and(|p| p.exists())
            })
            .map(|k| file_name(*k, format))
            .collect();
        if existing.is_empty() {
            self.write_conversations(&keys, format, &dir);
        } else {
            let what = Exported::Conversations(format);
            self.ask_or_write(Ask { keys, what, dir, existing });
        }
    }

    /// Writes each of `keys` into `dir` as `format`; the log says how many
    /// were written and where, and names each that was not, with why.
    pub(crate) fn write_conversations(&mut self, keys: &[ResKey], format: Format, dir: &Path) {
        if let Err(e) = std::fs::create_dir_all(dir) {
            self.log.error(format!("{}: {e}", dir.display()));
            return;
        }
        let Some(ws) = self.ws.as_mut() else { return };
        let mut written: Vec<String> = Vec::new();
        let mut failed: Vec<String> = Vec::new();
        for &key in keys {
            let name = file_name(key, format);
            let path = match crate::transfer::named_in(dir, &key.resref, format.extensions()[0]) {
                Ok(path) => path,
                Err(e) => {
                    failed.push(format!("{key}: {e}"));
                    continue;
                }
            };
            let text = match ws.doc(&key) {
                Ok(g) => text_of(g, key, format),
                Err(e) => {
                    failed.push(format!("{key}: {e}"));
                    continue;
                }
            };
            match std::fs::write(&path, text) {
                Ok(()) => written.push(name),
                Err(e) => failed.push(format!("{key}: {}: {e}", path.display())),
            }
        }
        let n = written.len();
        if n > 0 {
            let what =
                if n == 1 { "1 conversation".to_string() } else { format!("{n} conversations") };
            self.log.info(format!(
                "Wrote {what} as {} to {}: {}",
                word(format),
                dir.display(),
                crate::text::some_of(&written, 8)
            ));
        }
        for f in failed {
            self.log.error(format!("Not written: {f}"));
        }
    }
}

/// The question, while there is one: the files named, Overwrite, Skip
/// Those (the others written) or Cancel, and Always do this.
pub(crate) fn ask_window(app: &mut Moonglow, ctx: &egui::Context) {
    use crate::settings::FilesThere;
    let Some(ask) = &app.dialog_export_ask else { return };
    let mut answer: Option<Option<FilesThere>> = None;
    let always_id = egui::Id::new("dialog-export-always");
    let always = ctx.data(|d| d.get_temp::<bool>(always_id)).unwrap_or(false);
    let title = match ask.what {
        Exported::Conversations(format) => format!("Export as {}", word(format)),
        Exported::Minimaps { .. } => "Export Minimaps".to_string(),
    };
    crate::widgets::modal(ctx, title).id(egui::Id::new("dialog-export-ask")).show(ctx, |ui| {
        ui.set_max_width(460.0);
        let n = ask.existing.len();
        let these = if n == 1 { "A file of this name is" } else { "Files of these names are" };
        ui.label(format!("{these} in {} already:", ask.dir.display()));
        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
            for name in &ask.existing {
                ui.label(format!("  {name}"));
            }
        });
        ui.add_space(6.0);
        ui.label(if n == 1 { "Overwrite it?" } else { "Overwrite them?" });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Overwrite").clicked() {
                answer = Some(Some(FilesThere::Replace));
            }
            if ui
                .button("Skip Those")
                .on_hover_text("Write the others, and leave these as they are")
                .clicked()
            {
                answer = Some(Some(FilesThere::Skip));
            }
            if crate::widgets::cancel(ui) {
                answer = Some(None);
            }
        });
        crate::widgets::always_box(
            ui,
            always_id,
            "Always do this",
            &format!("{} › Files there already", crate::widgets::QUESTIONS_AT),
        );
    });
    let Some(answer) = answer else { return };
    let Some(ask) = app.dialog_export_ask.take() else { return };
    let Some(choice) = answer else { return };
    // (Always so: Options' choice, from now on.)
    if always {
        app.settings.files_there = choice;
    }
    let keys = if choice == FilesThere::Skip { ask.new_ones() } else { ask.keys.clone() };
    app.write_exported(&ask, &keys);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{ResRef, ResType};
    use mg_module::{Module, ModuleLocation};

    /// A module from elsewhere may name a resource `../x`: the exports
    /// that name a file after a resource wrote it outside the folder
    /// chosen (the review of November 2026: A2, D4, S8).
    #[test]
    fn a_resource_named_like_a_path_is_not_exported_out_of_the_folder() {
        let dir = mg_testkit::scratch_dir("ui-export-names");
        let hostile = ResKey::new(ResRef::from_str("../x").unwrap(), ResType::DLG);
        let plain = ResKey::new(ResRef::from_str("plain").unwrap(), ResType::DLG);
        let mut m = Module::new();
        for key in [hostile, plain] {
            m.set(key, mg_module::dialog::new_dialog().to_bytes().unwrap());
        }
        m.save_as(&ModuleLocation::Archive(dir.join("m.mod"))).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.open_module(&dir.join("m.mod"));
        let out = dir.join("out");
        app.write_conversations(&[hostile, plain], Format::Csv, &out);
        let files: Vec<_> =
            std::fs::read_dir(&out).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(files, ["plain.csv"]);
        assert!(!dir.join("x.csv").exists());
        let said =
            |app: &Moonglow, what: &str| app.log.entries.iter().any(|(_, l)| l.contains(what));
        assert!(said(&app, "not a name a file can have"), "{:?}", app.log.entries);

        // The names themselves.
        let name = |n: &str| ResRef::from_str(n).unwrap();
        for bad in ["../x", "a/b", "a\\b", "c:x", ""] {
            assert!(crate::transfer::named_in(&out, &name(bad), "png").is_err(), "{bad:?}");
        }
        assert_eq!(
            crate::transfer::named_in(&out, &name("area001"), "png"),
            Ok(out.join("area001.png"))
        );

        // A conversation's backup, the same.
        app.conversation_backups = dir.join("backups");
        app.actions.push(crate::Action::OpenTab(crate::Tab::Dialog(hostile)));
        app.run_actions();
        let e = mg_edit::Edit::SetResource { key: plain, data: m.get(&plain).map(<[u8]>::to_vec) };
        app.ws.as_mut().unwrap().apply(mg_edit::Command::new("Touch", vec![e])).unwrap();
        app.ws.as_mut().unwrap().mark_modified();
        assert!(app.backup_conversations().is_empty());
        assert!(!dir.join("x.bak").exists() && !dir.join("backups").join("x.bak").exists());
    }
}
