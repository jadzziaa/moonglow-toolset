//! A nasher project's files changed outside Moonglow (an editor, `git
//! checkout`, `git pull`) while it is open: every few seconds, with
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
    /// Looks at the project's files (see the module's notes).
    pub fn reload_project_files(&mut self) {
        let Some(ws) = self.ws.as_mut() else { return };
        let Some(project) = ws.module.project.as_mut() else { return };
        let (changes, problems) = project.outside();
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
        let project = ws.module.project.as_ref().expect("checked");
        let (mut take, mut same, mut conflicts) = (Vec::new(), Vec::new(), Vec::new());
        for c in changes {
            let ours = ws.module.get(&c.key);
            // (A script being typed isn't in the module yet.)
            let typing = self.scripts.get(&c.key).is_some_and(|b| b.is_dirty());
            if typing {
                conflicts.push(c);
            } else if ours == c.resource.as_deref() {
                same.push(c);
            } else if ours == project.as_read(&c.key) {
                take.push(c);
            } else {
                conflicts.push(c);
            }
        }
        for c in &same {
            ws.module.project.as_mut().expect("checked").took(c);
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
        if let Some(project) = ws.module.project.as_mut() {
            for c in changes {
                project.took(c);
            }
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
            "Read again from the project (changed outside Moonglow): {}{}",
            named(changes),
            if dropped { "; the undo history, which had changes to them, was cleared" } else { "" }
        ));
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
                {
                    answer = Some(Answer::Later);
                }
            });
        });
    let conflicts = std::mem::take(&mut app.outside.conflicts);
    match answer {
        Some(Answer::Ours) => {
            if let Some(project) = app.ws.as_mut().and_then(|ws| ws.module.project.as_mut()) {
                for c in &conflicts {
                    project.took(c);
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
