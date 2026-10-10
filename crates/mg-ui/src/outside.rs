//! A nasher project's or a module folder's files changed outside Moonglow
//! (an editor, `git checkout`, `git pull`) while it is open: every few seconds, with
//! Options › General's reloading on, those the module has no unsaved
//! change to are read again; where it has one, Moonglow's is kept and the
//! Changed Outside Moonglow window asks which to keep.

use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use mg_module::nasher::Outside;

use crate::{Moonglow, Tab};

/// What is known of the files changed outside.
#[derive(Debug, Default)]
pub(crate) struct OutsideState {
    /// Changed outside and in Moonglow both: waiting for an answer.
    pub(crate) conflicts: Vec<Outside>,
    /// What was said already (a problem, a conflict), not to say it again
    /// every few seconds.
    said: std::collections::HashSet<u64>,
    /// The conflicts "Later" was chosen for.
    later: Vec<PathBuf>,
    /// Looks at the module's files off the UI's thread.
    watch: Option<Watch>,
    /// And at the haks and folders the game data reads.
    content: Option<Watch>,
}

/// A thread that looks at a module folder's (or a nasher project's) files
/// every few seconds and says when any was changed, added or removed, so
/// that the window's thread reads them only then. (Looked at there, a
/// large module's thousands of files stalled the window for a moment
/// every few seconds: a builder saw it.)
#[derive(Debug)]
struct Watch {
    /// The folders (with what is under them, where `deep`) and files
    /// looked at.
    paths: Vec<PathBuf>,
    changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The files `paths` name, and those in the folders they name, as one
/// number: their names, times and sizes. With `deep`, the folders under
/// a folder too (not what a version control or a build keeps there:
/// `.git`, `.nasher` and other hidden folders).
fn signature(paths: &[PathBuf], deep: bool) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut dirs = Vec::new();
    for path in paths {
        match std::fs::metadata(path) {
            Ok(meta) if meta.is_dir() => dirs.push(path.clone()),
            Ok(meta) => (path, meta.modified().ok(), meta.len()).hash(&mut h),
            Err(_) => path.hash(&mut h),
        }
    }
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut seen: Vec<(std::ffi::OsString, Option<std::time::SystemTime>, u64)> = Vec::new();
        for e in entries.flatten() {
            let name = e.file_name();
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                if deep && !name.to_string_lossy().starts_with('.') {
                    dirs.push(e.path());
                }
            } else {
                seen.push((name, meta.modified().ok(), meta.len()));
            }
        }
        seen.sort();
        (dir, seen).hash(&mut h);
    }
    h.finish()
}

impl Watch {
    fn start(paths: Vec<PathBuf>, deep: bool, every: std::time::Duration) -> Watch {
        use std::sync::atomic::{AtomicBool, Ordering};
        let changed = std::sync::Arc::new(AtomicBool::new(false));
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let (dirs, flag, done) = (paths.clone(), changed.clone(), stop.clone());
        let work = move || {
            let mut last = signature(&dirs, deep);
            // (Looked at once at the start, and only now: what changed
            // before this first look is in it, and would never be said.)
            flag.store(true, Ordering::Relaxed);
            while !done.load(Ordering::Relaxed) {
                // (In short naps, to end soon after the module closes.)
                let until = std::time::Instant::now() + every;
                while std::time::Instant::now() < until && !done.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                let now = signature(&dirs, deep);
                if now != last {
                    last = now;
                    flag.store(true, Ordering::Relaxed);
                }
            }
        };
        if std::thread::Builder::new().name("moonglow files".into()).spawn(work).is_err() {
            // (No thread: looked at every time, as before.)
            stop.store(true, Ordering::Relaxed);
            changed.store(true, Ordering::Relaxed);
        }
        Watch { paths, changed, stop }
    }
}

fn hash(parts: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// File names, a few of them by name.
fn named(changes: &[Outside]) -> String {
    let names: Vec<String> = changes
        .iter()
        .take(6)
        .map(|c| c.path.file_name().unwrap_or_default().to_string_lossy().into_owned())
        .collect();
    match changes.len() - names.len() {
        0 => names.join(", "),
        more => format!("{} and {more} more", names.join(", ")),
    }
}

impl Moonglow {
    /// Every few seconds: the module's files are read again
    /// ([`Moonglow::reload_project_files`]) if the watch saw any change
    /// among them. The watch is the module's place's: begun with it, ended
    /// when the module is closed or kept elsewhere.
    pub(crate) fn reload_changed_files(&mut self) {
        use std::sync::atomic::Ordering;
        let root = self.ws.as_ref().and_then(|ws| {
            let m = &ws.module;
            (m.project.as_ref().map(|p| p.root().to_path_buf()))
                .or_else(|| m.folder.as_ref().map(|f| f.dir().to_path_buf()))
        });
        let Some(root) = root else {
            self.outside.watch = None;
            return;
        };
        if self.outside.watch.as_ref().is_none_or(|w| w.paths[..] != [root.clone()]) {
            let every = std::time::Duration::from_secs(3);
            self.outside.watch = Some(Watch::start(vec![root], true, every));
        }
        let watch = self.outside.watch.as_ref().expect("just made");
        // (A watch without its thread says nothing: looked at each time.)
        let dead = watch.stop.load(Ordering::Relaxed);
        if watch.changed.swap(false, Ordering::Relaxed) || dead {
            self.reload_project_files();
        }
    }

    /// Every few seconds: the haks and folders the game data reads (and the
    /// custom talk table) are looked at again
    /// ([`Moonglow::reload_resources`]) if a watch of theirs saw a change:
    /// a large `override` looked at on the window's thread every time is
    /// what a module's files were.
    pub fn reload_changed_content(&mut self) {
        use std::sync::atomic::Ordering;
        let Some(game) = self.game.as_ref() else {
            self.outside.content = None;
            return;
        };
        let mut paths: Vec<PathBuf> = (game.resman.layers().iter())
            .filter(|l| crate::user_content(l))
            .filter_map(|l| l.container.watched().map(PathBuf::from))
            .collect();
        paths.extend(self.tlk_stamp.as_ref().map(|(path, _)| path.clone()));
        if self.outside.content.as_ref().is_none_or(|w| w.paths != paths) {
            let every = std::time::Duration::from_secs(3);
            self.outside.content = Some(Watch::start(paths, false, every));
        }
        // (Held by something else, a job: looked at once it is free.)
        if crate::exclusive(&mut self.game).is_none() {
            crate::trace::changed("content", || "the game data is held: not looked at".into());
            return;
        }
        crate::trace::changed("content", || "the game data is free".into());
        let watch = self.outside.content.as_ref().expect("just made");
        let dead = watch.stop.load(Ordering::Relaxed);
        if watch.changed.swap(false, Ordering::Relaxed) || dead {
            self.reload_resources(false);
        }
    }

    /// Looks at the project's or the module folder's files (see the
    /// module's notes).
    pub fn reload_project_files(&mut self) {
        let Some(ws) = self.ws.as_mut() else { return };
        let Some((changes, problems)) = ws.module.outside() else { return };
        for p in problems {
            if self.outside.said.insert(hash(&p)) {
                self.log.warn(format!("Changed outside Moonglow, and not read: {p}"));
            }
        }
        if changes.is_empty() {
            self.outside.conflicts.clear();
            return;
        }
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let (mut take, mut same, mut conflicts) = (Vec::new(), Vec::new(), Vec::new());
        for c in changes {
            let ours = ws.module.get(&c.key);
            // (A script being typed isn't in the module yet.)
            let typing = self.scripts.get(&c.key).is_some_and(|b| b.is_dirty());
            if typing {
                conflicts.push(c);
            } else if ours == c.resource.as_deref() {
                same.push(c);
            } else if ours == ws.module.as_read(&c.key) {
                take.push(c);
            } else {
                conflicts.push(c);
            }
        }
        for c in &same {
            ws.module.took(c);
        }
        for c in &conflicts {
            let id = hash((&c.path, c.resource.as_deref()));
            if self.outside.said.insert(id) {
                self.log.warn(format!(
                    "{} changed outside Moonglow and has unsaved changes here: Moonglow's is \
                     kept (Changed Outside Moonglow asks which to keep)",
                    c.path.display()
                ));
            }
        }
        self.outside.conflicts = conflicts;
        if !take.is_empty() {
            self.take_outside(&take);
        }
    }

    /// The files changed outside that wait for an answer (which to keep).
    pub fn outside_conflicts(&self) -> Vec<PathBuf> {
        self.outside.conflicts.iter().map(|c| c.path.clone()).collect()
    }

    /// Reads `changes` into the module (their files' versions), and counts
    /// the files as read.
    fn take_outside(&mut self, changes: &[Outside]) {
        let Some(ws) = self.ws.as_mut() else { return };
        let list: Vec<_> = changes.iter().map(|c| (c.key, c.resource.clone())).collect();
        let dropped = match ws.adopt(&list) {
            Ok(d) => d,
            Err(e) => {
                self.log.error(e.to_string());
                return;
            }
        };
        for c in changes {
            ws.module.took(c);
        }
        for c in changes {
            // A script's editor reads its text again.
            self.scripts.remove(&c.key);
        }
        // What is gone has no editor any more.
        let gone: Vec<_> = changes.iter().filter(|c| c.resource.is_none()).map(|c| c.key).collect();
        if !gone.is_empty() {
            self.dock.retain_tabs(|t| match t {
                Tab::Script(k) | Tab::Gff(k) | Tab::Dialog(k) | Tab::Blueprint(k) => {
                    !gone.contains(k)
                }
                Tab::Area(r) | Tab::AreaProperties(r) => {
                    !gone.iter().any(|k| k.restype == mg_core::ResType::ARE && k.resref == *r)
                }
                _ => true,
            });
        }
        // Names shown from the module are read again.
        self.area_names = Default::default();
        self.area_contents.clear();
        self.refresh_module_layer();
        self.log.info(format!(
            "Read again from the module's files (changed outside Moonglow): {}{}",
            named(changes),
            if dropped { "; the undo history, which had changes to them, was cleared" } else { "" }
        ));
        self.compile_outside(changes);
    }

    /// Options › Script Editor › Automatically Compile Scripts on Save: a
    /// script another program saved is compiled as one saved here is, so
    /// that its compiled script is not the older one, and so are the
    /// scripts that include it. (Not a great many at once, a checkout's,
    /// which would hold the window: Compile All is for those.)
    fn compile_outside(&mut self, changes: &[Outside]) {
        if !self.settings.auto_compile {
            return;
        }
        let scripts: Vec<_> = changes
            .iter()
            .filter(|c| c.resource.is_some() && c.key.restype == mg_core::ResType::NSS)
            .map(|c| c.key)
            .collect();
        if scripts.is_empty() {
            return;
        }
        if self.game.is_none() {
            self.log.info(format!(
                "{} scripts read again were not compiled (no game data)",
                scripts.len()
            ));
            return;
        }
        // (And the scripts that include them.)
        let Some(scripts) = self.with_includers(&scripts) else { return };
        let (compiled, _) = crate::script_view::compile_stale(self, &scripts);
        if !compiled.is_empty() {
            self.log.info(format!(
                "Compiled (changed outside Moonglow): {}",
                crate::transfer::listed(&compiled)
            ));
        }
        let failed: Vec<String> = scripts
            .iter()
            .filter(|k| !compiled.contains(&k.resref.to_string()))
            .filter(|k| self.script_fails(**k))
            .map(|k| k.resref.to_string())
            .collect();
        if !failed.is_empty() {
            self.log.error(format!(
                "Did not compile (Compile in the script's editor says why): {}",
                crate::transfer::listed(&failed)
            ));
        }
    }
}

/// Changed Outside Moonglow: the files changed outside that Moonglow has
/// unsaved changes to, and which to keep.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let paths: Vec<PathBuf> = app.outside.conflicts.iter().map(|c| c.path.clone()).collect();
    if paths.is_empty() || paths == app.outside.later {
        return;
    }
    enum Answer {
        Ours,
        Theirs,
        Later,
    }
    let mut answer = None;
    egui::Window::new("Changed Outside Moonglow")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                "These files of the project were changed by another program, and Moonglow has \
                 unsaved changes to them:",
            );
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for c in &app.outside.conflicts {
                    let gone = if c.resource.is_none() { " (deleted)" } else { "" };
                    ui.monospace(format!("{}{gone}", c.path.display()));
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .button("Keep Moonglow's")
                    .on_hover_text("Your changes stay; the next Save writes them over the files")
                    .clicked()
                {
                    answer = Some(Answer::Ours);
                }
                if ui
                    .button("Take the Files'")
                    .on_hover_text(
                        "The files are read again; your unsaved changes to them are lost",
                    )
                    .clicked()
                {
                    answer = Some(Answer::Theirs);
                }
                if ui
                    .button("Later")
                    .on_hover_text("Moonglow's stay for now, and Save won't write over these files")
                    .clicked()
                    || crate::widgets::escape_closes(ui.ctx(), "Changed Outside Moonglow")
                {
                    answer = Some(Answer::Later);
                }
            });
        });
    let conflicts = std::mem::take(&mut app.outside.conflicts);
    match answer {
        Some(Answer::Ours) => {
            if let Some(ws) = app.ws.as_mut() {
                for c in &conflicts {
                    ws.module.took(c);
                }
            }
            // (No longer as saved: the files have something else.)
            if let Some(ws) = app.ws.as_mut() {
                ws.mark_modified();
            }
            app.log.info(format!("Kept Moonglow's: {} (Save writes them)", named(&conflicts)));
        }
        Some(Answer::Theirs) => app.take_outside(&conflicts),
        Some(Answer::Later) => {
            app.outside.later = paths;
            app.outside.conflicts = conflicts;
        }
        None => app.outside.conflicts = conflicts,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_watch_says_when_a_file_changes_and_only_then() {
        let dir = mg_testkit::scratch_dir("ui-outside-watch");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("src/a.nss"), "void main() {}").unwrap();
        // The files as one number: other when one is longer, added or
        // removed; the same for what a hidden folder holds.
        let first = signature(std::slice::from_ref(&dir), true);
        assert_eq!(signature(std::slice::from_ref(&dir), true), first);
        std::fs::write(dir.join(".git/index"), "x").unwrap();
        assert_eq!(signature(std::slice::from_ref(&dir), true), first);
        std::fs::write(dir.join("src/a.nss"), "void main() { int a; }").unwrap();
        let longer = signature(std::slice::from_ref(&dir), true);
        assert_ne!(longer, first);
        std::fs::write(dir.join("src/b.nss"), "").unwrap();
        let added = signature(std::slice::from_ref(&dir), true);
        assert_ne!(added, longer);
        std::fs::remove_file(dir.join("src/b.nss")).unwrap();
        assert_eq!(signature(std::slice::from_ref(&dir), true), longer);

        // The watch: once at the start, then only after a change.
        let watch = Watch::start(vec![dir.clone()], true, Duration::from_millis(40));
        let seen = |watch: &Watch| {
            for _ in 0..100 {
                if watch.changed.swap(false, Ordering::Relaxed) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        assert!(seen(&watch), "looked at once, at the start");
        std::thread::sleep(Duration::from_millis(300));
        assert!(!watch.changed.load(Ordering::Relaxed), "nothing changed: nothing said");
        std::fs::write(dir.join("src/c.nss"), "void main() {}").unwrap();
        assert!(seen(&watch), "a file added is seen");
        // (The watch may have seen the file made and then written: twice.)
        std::thread::sleep(Duration::from_millis(300));
        watch.changed.store(false, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(300));
        assert!(!watch.changed.load(Ordering::Relaxed), "and nothing said after");
    }
}
