//! Crash safety: while the open module has unsaved changes, a recovery copy
//! of it (edits and unsaved script text included) is written every few
//! minutes (Options › General; 5 by default) to Moonglow's own data folder,
//! never next to the module or into the game's folders. Saving the module,
//! or closing it without saving, removes the copy; a copy still there at
//! the next start (after a crash) is offered back by Recover Unsaved Work.
//!
//! A copy is the module as an archive (`<name>-<id>.mod`) and a note
//! (`.json`): where the module lives, its name, when the copy was made.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mg_module::{Module, ModuleLocation};
use serde::{Deserialize, Serialize};

use crate::{Moonglow, text};

/// Moonglow's data folder: `$XDG_DATA_HOME/moonglow` (else
/// `~/.local/share/moonglow`) on Linux, `~/Library/Application
/// Support/Moonglow` on macOS, `%APPDATA%\Moonglow` on Windows.
pub fn data_dir() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        env("APPDATA").map(|d| d.join("Moonglow"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support/Moonglow"))
    } else {
        env("XDG_DATA_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".local/share")))
            .map(|d| d.join("moonglow"))
    }
}

/// A recovery copy's note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// Where the module lives (`None`: never saved).
    pub module: Option<PathBuf>,
    /// Whether it lives in a folder (else an archive).
    pub folder: bool,
    pub name: String,
    /// When the copy was made (seconds since 1970).
    pub time: u64,
}

/// A recovery copy found in the folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Copy {
    pub archive: PathBuf,
    pub note: Note,
}

impl Copy {
    fn note_path(&self) -> PathBuf {
        self.archive.with_extension("json")
    }

    /// Removes the copy.
    pub fn remove(&self) {
        let _ = std::fs::remove_file(&self.archive);
        let _ = std::fs::remove_file(self.note_path());
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// How long ago, in words.
pub fn age(time: u64) -> String {
    let s = now().saturating_sub(time);
    match s {
        0..60 => "less than a minute ago".into(),
        60..7200 => format!("{} minutes ago", s / 60),
        7200..172_800 => format!("{} hours ago", s / 3600),
        _ => format!("{} days ago", s / 86_400),
    }
}

/// A copy's file name for a module: its name and a hash of where it lives,
/// so that two modules of the same name keep separate copies.
fn file_stem(module: Option<&Path>, name: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let place = module.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    for b in place.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    let clean: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(40)
        .collect();
    format!("{clean}-{:08x}", h as u32)
}

/// The copies in a folder, newest first.
pub fn find(dir: &Path) -> Vec<Copy> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Copy> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| {
            let note: Note = serde_json::from_slice(&std::fs::read(&p).ok()?).ok()?;
            let archive = p.with_extension("mod");
            archive.is_file().then_some(Copy { archive, note })
        })
        .collect();
    out.sort_by_key(|c| std::cmp::Reverse(c.note.time));
    out
}

/// A copy to write: the module and where it goes.
struct Job {
    module: Module,
    archive: PathBuf,
    note: Note,
}

impl Job {
    /// Writes the archive (through a temporary file), then the note.
    fn run(self) -> Result<PathBuf, String> {
        let dir = self.archive.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let bytes = self.module.to_archive_bytes().map_err(|e| e.to_string())?;
        let tmp = self.archive.with_extension("mod.tmp");
        std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.archive)
            .map_err(|e| format!("{}: {e}", self.archive.display()))?;
        let note = serde_json::to_vec_pretty(&self.note).map_err(|e| e.to_string())?;
        let path = self.archive.with_extension("json");
        std::fs::write(&path, note).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(self.archive)
    }
}

/// The autosave's state: when the last copy was made, of which revision,
/// and whether one is being written.
#[derive(Debug, Default)]
pub(crate) struct Autosave {
    last: Option<(std::time::Instant, u64)>,
    busy: Arc<AtomicBool>,
}

impl Moonglow {
    /// Where the open module's copy goes (with no folder: no copies).
    fn recovery_archive(&self) -> Option<(PathBuf, Note)> {
        let dir = self.recovery_dir.as_ref()?;
        let ws = self.ws.as_ref()?;
        let module = ws.module.location.as_ref().map(|l| l.path().to_path_buf());
        let folder = matches!(ws.module.location, Some(ModuleLocation::Folder(_)));
        let name = module
            .as_deref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| "untitled".to_string(), |s| s.to_string_lossy().into_owned());
        let stem = file_stem(module.as_deref(), &name);
        Some((dir.join(format!("{stem}.mod")), Note { module, folder, name, time: now() }))
    }

    /// The copy of the module as it is now: edits, and script editors'
    /// unsaved text.
    fn recovery_job(&mut self) -> Option<Job> {
        let (archive, note) = self.recovery_archive()?;
        let mut module = self.ws.as_mut()?.snapshot().ok()?;
        for (key, buf) in self.scripts.iter().filter(|(_, b)| b.is_dirty()) {
            module.set(*key, text::encode(&buf.text));
        }
        Some(Job { module, archive, note })
    }

    /// Writes the open module's recovery copy now; its archive.
    pub fn write_recovery(&mut self) -> Result<PathBuf, String> {
        self.recovery_job().ok_or_else(|| "no module or no recovery folder".to_string())?.run()
    }

    /// Removes the open module's recovery copy (saved, or closed without
    /// saving).
    pub(crate) fn forget_recovery(&mut self) {
        if let Some((archive, note)) = self.recovery_archive() {
            Copy { archive, note }.remove();
        }
        self.autosave.last = None;
    }

    /// Every few minutes, while there are unsaved changes, a copy in the
    /// background (Options › General).
    pub(crate) fn autosave_timer(&mut self, ui: &egui::Ui) {
        if let Some(next) = self.autosave_tick(std::time::Instant::now()) {
            ui.ctx().request_repaint_after(next);
        }
    }

    /// The autosave at `now`: a copy started in the background when one is
    /// due; when to look again.
    fn autosave_tick(&mut self, now: std::time::Instant) -> Option<Duration> {
        if self.settings.no_autosave || self.recovery_dir.is_none() {
            return None;
        }
        let ws = self.ws.as_ref()?;
        if !ws.is_modified() {
            return None;
        }
        let revision = ws.revision();
        let every =
            Duration::from_secs(60 * u64::from(self.settings.autosave_minutes.unwrap_or(5).max(1)));
        // The first change starts the clock.
        let (since, saved) = *self.autosave.last.get_or_insert((now, u64::MAX));
        if now.saturating_duration_since(since) < every
            || saved == revision
            || self.autosave.busy.load(Ordering::Acquire)
        {
            return Some(every);
        }
        let job = self.recovery_job()?;
        self.autosave.last = Some((now, revision));
        let busy = self.autosave.busy.clone();
        busy.store(true, Ordering::Release);
        std::thread::spawn(move || {
            if let Err(e) = job.run() {
                eprintln!("recovery copy: {e}");
            }
            busy.store(false, Ordering::Release);
        });
        Some(every)
    }

    /// Looks for copies left by a session that did not end (shown by
    /// Recover Unsaved Work).
    pub fn find_recoveries(&mut self) {
        self.recoveries = self.recovery_dir.as_deref().map(find).unwrap_or_default();
    }

    /// Opens a recovery copy as the module it came from: saving writes it
    /// there.
    pub fn recover(&mut self, copy: &Copy) {
        let mut m = match Module::open(&copy.archive) {
            Ok(m) => m,
            Err(e) => {
                self.log.error(format!("Could not open the recovery copy: {e}"));
                return;
            }
        };
        m.location = copy.note.module.clone().map(|p| {
            if copy.note.folder { ModuleLocation::Folder(p) } else { ModuleLocation::Archive(p) }
        });
        self.use_module(m);
        if let Some(ws) = &mut self.ws {
            ws.mark_modified();
        }
        self.log.info(format!(
            "Recovered the unsaved work on {} from {}: save to keep it",
            copy.note.name,
            age(copy.note.time)
        ));
    }
}

/// Recover Unsaved Work: the copies a session that did not end left.
pub(crate) fn window(app: &mut Moonglow, ui: &egui::Ui) {
    if app.recoveries.is_empty() {
        return;
    }
    let mut open = true;
    let (mut recover, mut discard, mut later) = (None, None, false);
    egui::Window::new("Recover Unsaved Work").open(&mut open).collapsible(false).show(
        ui.ctx(),
        |ui| {
            ui.label("Moonglow closed without saving these modules. Their unsaved work was kept:");
            egui::Grid::new("recoveries").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                for (i, c) in app.recoveries.iter().enumerate() {
                    let place = c
                        .note
                        .module
                        .as_ref()
                        .map_or_else(|| "never saved".to_string(), |p| p.display().to_string());
                    ui.strong(&c.note.name).on_hover_text(place);
                    ui.label(age(c.note.time));
                    if ui.button("Recover").on_hover_text("Open it; save to keep it").clicked() {
                        recover = Some(i);
                    }
                    if ui.button("Discard").on_hover_text("Throw the unsaved work away").clicked() {
                        discard = Some(i);
                    }
                    ui.end_row();
                }
            });
            later = ui.button("Later").on_hover_text("Ask again at the next start").clicked();
        },
    );
    if let Some(i) = recover {
        let copy = app.recoveries.remove(i);
        app.recover(&copy);
    }
    if let Some(i) = discard {
        app.recoveries.remove(i).remove();
    }
    if later || !open {
        app.recoveries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_is_made_when_one_is_due() {
        use std::time::Instant;
        let dir = mg_testkit::scratch_dir("ui-autosave");
        let mut m = Module::new();
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        m.set(key, b"void main() {}".to_vec());
        m.save_as(&ModuleLocation::Archive(dir.join("m.mod"))).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.recovery_dir = Some(dir.join("recovery"));
        app.open_module(&dir.join("m.mod"));
        let t0 = Instant::now();
        assert_eq!(app.autosave_tick(t0), None, "nothing unsaved");
        let edit = mg_edit::Edit::SetResource { key, data: Some(b"// changed".to_vec()) };
        app.ws.as_mut().unwrap().apply(mg_edit::Command::new("edit", vec![edit])).unwrap();
        // The clock starts; five minutes later a copy is written.
        assert_eq!(app.autosave_tick(t0), Some(Duration::from_secs(300)));
        assert!(find(&dir.join("recovery")).is_empty());
        app.autosave_tick(t0 + Duration::from_secs(301));
        while app.autosave.busy.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(10));
        }
        let copies = find(&dir.join("recovery"));
        assert_eq!(copies.len(), 1);
        let copy = Module::open(&copies[0].archive).unwrap();
        assert_eq!(copy.get(&key), Some(&b"// changed"[..]));
        // Not again for the same revision.
        app.autosave_tick(t0 + Duration::from_secs(700));
        assert!(!app.autosave.busy.load(Ordering::Acquire));
    }

    #[test]
    fn copies_are_named_by_module_and_place() {
        let a = file_stem(Some(Path::new("/m/a/Keep.mod")), "Keep");
        let b = file_stem(Some(Path::new("/m/b/Keep.mod")), "Keep");
        assert!(a.starts_with("Keep-") && a != b);
        assert_eq!(file_stem(None, "a b/c"), file_stem(None, "a b/c"));
        assert!(file_stem(None, "a b/c").starts_with("a_b_c-"));
        assert_eq!(age(now()), "less than a minute ago");
        assert_eq!(age(now() - 600), "10 minutes ago");
    }
}
